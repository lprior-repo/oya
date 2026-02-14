#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
//! Property-based tests for checkpoint/resume cycle.
//!
//! Uses proptest to exhaustively test checkpoint operations with randomly generated data.
//!
//! # Properties Tested
//!
//! - Round-trip: Any serializable state can be checkpointed and restored exactly
//! - Compression: Compressible data achieves size reduction
//! - Storage integrity: Checkpoint ID is preserved across store/load operations

use std::collections::HashMap;

use proptest::collection::{hash_map, vec};
use proptest::prelude::*;
use proptest::string::string_regex;

use oya_workflow::checkpoint::{
    compress, compression_ratio, decompress, serialize_state, space_savings, CheckpointId,
    CheckpointManager, CheckpointMetadata, CheckpointStorage, CheckpointStrategy,
    InMemoryCheckpointStorage,
};
use oya_workflow::PhaseOutput;

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, bincode::Encode, bincode::Decode)]
struct ArbitraryWorkflowState {
    workflow_id: String,
    phase: String,
    progress: u64,
    metadata: HashMap<String, String>,
    bead_states: Vec<BeadState>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, bincode::Encode, bincode::Decode)]
struct BeadState {
    bead_id: String,
    status: String,
    attempts: u32,
    last_error: Option<String>,
}

fn workflow_id_strategy() -> impl Strategy<Value = String> {
    "(wf|workflow|job)-[a-z0-9]{1,8}"
}

fn phase_strategy() -> impl Strategy<Value = String> {
    prop_oneof![
        Just("implement".to_string()),
        Just("test".to_string()),
        Just("review".to_string()),
        Just("deploy".to_string()),
        "[a-z_]{3,12}",
    ]
}

fn metadata_strategy() -> impl Strategy<Value = HashMap<String, String>> {
    hash_map("[a-z_]{2,8}", "[a-z0-9]{2,16}", 0..5)
}

fn bead_state_strategy() -> impl Strategy<Value = BeadState> {
    (
        "bead-[a-z0-9]{4}",
        prop_oneof![
            Just("pending".to_string()),
            Just("running".to_string()),
            Just("completed".to_string()),
            Just("failed".to_string()),
        ],
        0..10u32,
        proptest::option::of("[a-z ]{5,30}"),
    )
        .prop_map(|(bead_id, status, attempts, last_error)| BeadState {
            bead_id,
            status,
            attempts,
            last_error,
        })
}

fn workflow_state_strategy() -> impl Strategy<Value = ArbitraryWorkflowState> {
    (
        workflow_id_strategy(),
        phase_strategy(),
        0..10000u64,
        metadata_strategy(),
        vec(bead_state_strategy(), 0..5),
    )
        .prop_map(|(workflow_id, phase, progress, metadata, bead_states)| {
            ArbitraryWorkflowState {
                workflow_id,
                phase,
                progress,
                metadata,
                bead_states,
            }
        })
}

fn compressible_data_strategy() -> impl Strategy<Value = Vec<u8>> {
    prop_oneof![
        vec(0u8..=5, 100..1000),
        vec(proptest::num::u8::ANY, 10..100),
        vec(Just(42u8).prop_map(|x| x), 50..500),
    ]
}

fn success_phase_output(data: Vec<u8>) -> PhaseOutput {
    PhaseOutput::success(data)
}

proptest! {
    #[test]
    fn prop_state_round_trip_exact(state in workflow_state_strategy()) {
        let serialized_result = serialize_state(&state);
        prop_assert!(serialized_result.is_ok(), "serialization should succeed");

        let serialized = serialized_result.map_err(|e| TestCaseError::fail(format!("{e}")))?;
        prop_assert!(!serialized.is_empty(), "serialized data should not be empty");

        // Use decompress_auto since we don't know the exact uncompressed size here
        let decompressed_result = oya_workflow::checkpoint::decompress_auto(&serialized);
        prop_assert!(decompressed_result.is_ok(), "decompression should succeed");
        let decompressed = decompressed_result.map_err(|e| TestCaseError::fail(format!("{e}")))?;

        let deserialized: Result<ArbitraryWorkflowState, _> = bincode::decode_from_slice(
            &decompressed[12..],
            bincode::config::standard()
        ).map(|(s, _)| s);

        prop_assert!(deserialized.is_ok(), "deserialization should succeed");
        let restored = deserialized.map_err(|e| TestCaseError::fail(format!("{e}")))?;
        prop_assert_eq!(restored, state, "restored state should match original");
    }

    #[test]
    fn prop_compress_decompress_round_trip(data in compressible_data_strategy()) {
        let original_size = data.len();

        let compressed_result = compress(&data);
        prop_assert!(compressed_result.is_ok(), "compression should succeed");

        let compressed = compressed_result.map_err(|e| TestCaseError::fail(format!("{e}")))?;

        let decompressed_result = decompress(&compressed, original_size);
        prop_assert!(decompressed_result.is_ok(), "decompression should succeed");

        let decompressed = decompressed_result.map_err(|e| TestCaseError::fail(format!("{e}")))?;
        prop_assert_eq!(decompressed, data, "decompressed data should match original");
    }

    #[test]
    fn prop_compression_ratio_consistency(
        original_size in 100u64..100000u64,
        compressed_size in 10u64..50000u64
    ) {
        let ratio = compression_ratio(original_size, compressed_size);

        if compressed_size < original_size {
            prop_assert!(ratio > 1.0, "ratio > 1 when compressed < original");
        } else if compressed_size > original_size {
            prop_assert!(ratio < 1.0, "ratio < 1 when compressed > original");
        } else {
            prop_assert!((ratio - 1.0).abs() < 0.0001, "ratio = 1 when sizes equal");
        }

        let space_saved = space_savings(original_size, compressed_size);
        if compressed_size < original_size {
            prop_assert!(space_saved > 0, "space saved should be positive");
        }
    }

    #[test]
    fn prop_checkpoint_id_preserved(data in vec(proptest::num::u8::ANY, 0..1000)) {
        let mut storage = InMemoryCheckpointStorage::new();

        let checkpoint_id = CheckpointId::new();
        let metadata = CheckpointMetadata {
            id: checkpoint_id,
            created_at: chrono::Utc::now(),
            version: 1,
            uncompressed_size: data.len(),
            compressed_size: data.len() / 2.max(1),
            compression_ratio: 2.0,
        };

        let store_result = storage.store_checkpoint(data.clone(), metadata.clone());
        prop_assert!(store_result.is_ok(), "store should succeed");

        let load_result = storage.load_checkpoint(&checkpoint_id);
        prop_assert!(load_result.is_ok(), "load should succeed");

        let (loaded_data, loaded_metadata) = load_result.map_err(|e| TestCaseError::fail(format!("{e}")))?;
        prop_assert_eq!(loaded_data, data, "loaded data should match");
        prop_assert_eq!(loaded_metadata.id, checkpoint_id, "ID should be preserved");
    }

    #[test]
    fn prop_multiple_checkpoints_independent(
        datasets in vec(vec(proptest::num::u8::ANY, 10..100), 1..10)
    ) {
        let mut storage = InMemoryCheckpointStorage::new();
        let mut ids: Vec<CheckpointId> = Vec::new();

        for data in &datasets {
            let id = CheckpointId::new();
            let metadata = CheckpointMetadata {
                id,
                created_at: chrono::Utc::now(),
                version: 1,
                uncompressed_size: data.len(),
                compressed_size: data.len(),
                compression_ratio: 1.0,
            };

            let store_result = storage.store_checkpoint(data.clone(), metadata);
            prop_assert!(store_result.is_ok(), "store should succeed for each dataset");
            ids.push(id);
        }

        for (i, id) in ids.iter().enumerate() {
            let load_result = storage.load_checkpoint(id);
            prop_assert!(load_result.is_ok(), "load should succeed for ID {}", i);

            let (loaded_data, _) = load_result.map_err(|e| TestCaseError::fail(format!("{e}")))?;
            prop_assert_eq!(loaded_data, datasets[i].clone(), "data {} should match", i);
        }
    }

    #[test]
    fn prop_delete_removes_checkpoint(data in vec(proptest::num::u8::ANY, 10..200)) {
        let mut storage = InMemoryCheckpointStorage::new();

        let id = CheckpointId::new();
        let metadata = CheckpointMetadata {
            id,
            created_at: chrono::Utc::now(),
            version: 1,
            uncompressed_size: data.len(),
            compressed_size: data.len(),
            compression_ratio: 1.0,
        };

        let _ = storage.store_checkpoint(data, metadata);

        let delete_result = storage.delete_checkpoint(&id);
        prop_assert!(delete_result.is_ok(), "delete should succeed");

        let load_result = storage.load_checkpoint(&id);
        prop_assert!(load_result.is_err(), "load after delete should fail");
    }

    #[test]
    fn prop_always_strategy_checkpoints_every_phase(outputs in vec(vec(proptest::num::u8::ANY, 0..50), 1..20)) {
        let mut manager = CheckpointManager::new(CheckpointStrategy::Always);

        for output in &outputs {
            let decision = manager.update(&success_phase_output(output.clone()));
            prop_assert!(
                matches!(decision, oya_workflow::checkpoint::CheckpointDecision::Checkpoint),
                "Always strategy should checkpoint after every phase"
            );
        }
    }

    #[test]
    fn prop_interval_strategy_periodic(interval in 1u8..10u8, phases in 20u8..50u8) {
        let mut manager = CheckpointManager::new(CheckpointStrategy::Interval(interval as usize));

        let mut checkpoint_count = 0u32;
        for _ in 0..phases {
            let decision = manager.update(&success_phase_output(vec![1, 2, 3]));
            if matches!(decision, oya_workflow::checkpoint::CheckpointDecision::Checkpoint) {
                checkpoint_count += 1;
            }
        }

        let expected_min = (phases as u64) / (interval as u64 + 1);
        prop_assert!(
            checkpoint_count >= expected_min as u32,
            "Interval({}) should checkpoint at least {} times in {} phases, got {}",
            interval, expected_min, phases, checkpoint_count
        );
    }

    #[test]
    fn prop_repetitive_data_compresses_well(byte in 0u8..=255, count in 500usize..5000) {
        let data = vec![byte; count];

        let compressed_result = compress(&data);
        prop_assert!(compressed_result.is_ok(), "compression should succeed");

        let compressed = compressed_result.map_err(|e| TestCaseError::fail(format!("{e}")))?;
        let ratio = compression_ratio(count as u64, compressed.len() as u64);

        prop_assert!(
            ratio > 10.0,
            "repetitive data ({} x {:02X}) should achieve >10x compression, got {:.2}x",
            count, byte, ratio
        );
    }

    #[test]
    fn prop_compressible_data_achieves_target_ratio(data in vec(0u8..=3, 1000..10000)) {
        let original_size = data.len();

        let compressed_result = compress(&data);
        prop_assert!(compressed_result.is_ok(), "compression should succeed");

        let compressed = compressed_result.map_err(|e| TestCaseError::fail(format!("{e}")))?;
        let space_saved = space_savings(original_size as u64, compressed.len() as u64);

        let reduction_percent = (space_saved as f64 / original_size as f64) * 100.0;
        prop_assert!(
            reduction_percent >= 50.0,
            "should achieve at least 50% size reduction, got {:.1}%",
            reduction_percent
        );
    }

    #[test]
    fn prop_full_cycle_preserves_state(state in workflow_state_strategy()) {
        let mut storage = InMemoryCheckpointStorage::new();

        let serialized_result = serialize_state(&state);
        prop_assert!(serialized_result.is_ok(), "serialization should succeed");
        let compressed = serialized_result.map_err(|e| TestCaseError::fail(format!("{e}")))?;

        let checkpoint_id = CheckpointId::new();
        let metadata = CheckpointMetadata {
            id: checkpoint_id,
            created_at: chrono::Utc::now(),
            version: 1,
            uncompressed_size: compressed.len() * 2,
            compressed_size: compressed.len(),
            compression_ratio: 2.0,
        };

        let store_result = storage.store_checkpoint(compressed.clone(), metadata);
        prop_assert!(store_result.is_ok(), "storage should succeed");

        let load_result = storage.load_checkpoint(&checkpoint_id);
        prop_assert!(load_result.is_ok(), "load should succeed");
        let (loaded_compressed, _) = load_result.map_err(|e| TestCaseError::fail(format!("{e}")))?;

        let decompressed_result = oya_workflow::checkpoint::decompress_auto(&loaded_compressed);
        prop_assert!(decompressed_result.is_ok(), "decompression should succeed");
        let decompressed = decompressed_result.map_err(|e| TestCaseError::fail(format!("{e}")))?;

        let restored: Result<ArbitraryWorkflowState, _> = bincode::decode_from_slice(
            &decompressed[12..],
            bincode::config::standard()
        ).map(|(s, _)| s);

        prop_assert!(restored.is_ok(), "deserialization should succeed");
        let restored = restored.map_err(|e| TestCaseError::fail(format!("{e}")))?;
        prop_assert_eq!(restored, state, "full cycle should preserve state exactly");
    }
}
