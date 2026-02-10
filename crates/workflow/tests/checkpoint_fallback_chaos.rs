//! Chaos tests for checkpoint corruption with fallback to previous checkpoint.
//!
//! Tests resilience to checkpoint corruption by:
//! 1. Creating multiple sequential checkpoints
//! 2. Corrupting the latest checkpoint
//! 3. Verifying fallback to previous checkpoint succeeds
//! 4. Verifying system recovers with consistent state

#![cfg(test)]
#![deny(clippy::unwrap_used)]
#![deny(clippy::expect_used)]
#![deny(clippy::panic)]
#![warn(clippy::pedantic)]
#![warn(clippy::nursery)]
#![forbid(unsafe_code)]

use thiserror::Error;
use tracing::{debug, info, warn};

use oya_workflow::checkpoint::restore::{
    restore_checkpoint, CheckpointId, RestoreError, RestoreResult,
};
use oya_workflow::checkpoint::serialize::serialize_state;
use oya_workflow::checkpoint::storage::{
    CheckpointMetadata, CheckpointStorage, InMemoryCheckpointStorage,
};

// =============================================================================
// Error Types
// =============================================================================

/// Errors that can occur during chaos testing.
#[derive(Debug, Error)]
pub enum ChaosTestError {
    #[error("Failed to create checkpoint: {reason}")]
    CheckpointCreationFailed { reason: String },

    #[error("Failed to corrupt checkpoint: {reason}")]
    CorruptionFailed { reason: String },

    #[error("Fallback to previous checkpoint failed: {reason}")]
    FallbackFailed { reason: String },

    #[error("State mismatch after recovery: {details}")]
    StateMismatch { details: String },

    #[error("Previous checkpoint not found")]
    PreviousCheckpointNotFound,

    #[error("Storage operation failed: {reason}")]
    StorageFailed { reason: String },

    #[error("Test setup failed: {reason}")]
    SetupFailed { reason: String },
}

/// Result type for chaos tests.
pub type ChaosTestResult<T> = Result<T, ChaosTestError>;

// =============================================================================
// Test State Structures
// =============================================================================

/// Test state that will be checkpointed.
#[derive(
    Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize, bincode::Encode, bincode::Decode,
)]
pub struct TestState {
    pub version: u32,
    pub counter: u64,
    pub data: String,
    pub items: Vec<String>,
}

impl TestState {
    /// Create a new test state.
    #[must_use]
    pub fn new(version: u32, counter: u64) -> Self {
        Self {
            version,
            counter,
            data: format!("state-data-v{version}"),
            items: vec![format!("item-{}", version)],
        }
    }

    /// Advance to next state version.
    #[must_use]
    pub fn advance(&self) -> Self {
        Self::new(self.version + 1, self.counter + 1)
    }
}

// =============================================================================
// Corruption Utilities
// =============================================================================

/// Corrupt a checkpoint in storage by replacing it with invalid data.
///
/// This simulates disk corruption, bit rot, or other storage failures.
fn corrupt_checkpoint_in_storage(
    storage: &mut InMemoryCheckpointStorage,
    checkpoint_id: &CheckpointId,
) -> ChaosTestResult<()> {
    info!("Corrupting checkpoint: {}", checkpoint_id);

    // Create invalid checkpoint data (not valid zstd)
    let corrupted_data = vec![0xFF, 0xFF, 0xFF, 0xFF];

    // Create metadata (we keep original metadata but corrupt the data)
    let metadata = CheckpointMetadata {
        id: *checkpoint_id,
        created_at: chrono::Utc::now(),
        version: 1,
        uncompressed_size: corrupted_data.len(),
        compressed_size: corrupted_data.len(),
        compression_ratio: 1.0,
    };

    // Store corrupted data
    storage
        .store_checkpoint(corrupted_data, metadata)
        .map_err(|e| ChaosTestError::CorruptionFailed {
            reason: e.to_string(),
        })?;

    debug!("Checkpoint corrupted successfully: {}", checkpoint_id);
    Ok(())
}

/// Create multiple sequential checkpoints.
fn create_checkpoint_sequence(
    storage: &mut InMemoryCheckpointStorage,
    count: usize,
) -> ChaosTestResult<Vec<(CheckpointId, TestState)>> {
    info!("Creating {} sequential checkpoints", count);

    let mut checkpoints = Vec::new();
    let mut state = TestState::new(1, 100);

    for i in 0..count {
        // Create checkpoint
        let checkpoint_id = CheckpointId::new();

        let serialized =
            serialize_state(&state).map_err(|e| ChaosTestError::CheckpointCreationFailed {
                reason: format!("serialization failed: {e}"),
            })?;

        let metadata = CheckpointMetadata {
            id: checkpoint_id,
            created_at: chrono::Utc::now(),
            version: 1,
            uncompressed_size: std::mem::size_of_val(&state),
            compressed_size: serialized.len(),
            compression_ratio: std::mem::size_of_val(&state) as f64 / serialized.len() as f64,
        };

        storage
            .store_checkpoint(serialized, metadata)
            .map_err(|e| ChaosTestError::StorageFailed {
                reason: format!("store failed: {e}"),
            })?;

        checkpoints.push((checkpoint_id, state.clone()));
        state = state.advance();

        debug!("Created checkpoint {}/{}: {}", i + 1, count, checkpoint_id);
    }

    info!("Created {} checkpoints successfully", checkpoints.len());
    Ok(checkpoints)
}

// =============================================================================
// Fallback Logic
// =============================================================================

/// Attempt to restore checkpoint with fallback to previous version.
///
/// This implements the fallback strategy:
/// 1. Try to restore the requested checkpoint
/// 2. If corrupted, search storage for the next most recent checkpoint
/// 3. Continue searching backwards until a valid checkpoint is found
/// 4. Restore the first valid checkpoint found
fn restore_with_fallback(
    checkpoint_id: &CheckpointId,
    storage: &dyn CheckpointStorage,
    previous_checkpoints: &[(CheckpointId, TestState)],
) -> ChaosTestResult<(CheckpointId, TestState)> {
    info!("Attempting restore with fallback for: {}", checkpoint_id);

    // Try to restore the requested checkpoint first
    let restore_result = restore_checkpoint::<TestState>(checkpoint_id, storage);

    match restore_result {
        Ok(state) => {
            info!("Successfully restored checkpoint: {}", checkpoint_id);
            Ok((*checkpoint_id, state))
        }
        Err(e) => {
            warn!("Failed to restore checkpoint {}: {}", checkpoint_id, e);

            // Check if error indicates corruption (not "not found")
            if matches!(
                e,
                RestoreError::DecompressionFailed { .. }
                    | RestoreError::DeserializationFailed { .. }
                    | RestoreError::InvalidData { .. }
            ) {
                info!(
                    "Checkpoint appears corrupted, attempting fallback to previous checkpoint(s)"
                );

                // Find the position of this checkpoint in our sequence
                let checkpoint_ids: Vec<_> =
                    previous_checkpoints.iter().map(|(id, _)| *id).collect();

                let current_index = checkpoint_ids
                    .iter()
                    .position(|id| id == checkpoint_id)
                    .ok_or_else(|| ChaosTestError::FallbackFailed {
                        reason: format!("checkpoint {} not found in sequence", checkpoint_id),
                    })?;

                // Iterate backwards through previous checkpoints
                for i in (0..current_index).rev() {
                    let previous_id = checkpoint_ids[i];
                    info!(
                        "Attempting to restore previous checkpoint {}/{}: {}",
                        current_index - i,
                        current_index,
                        previous_id
                    );

                    // Try to restore this previous checkpoint
                    let previous_state = restore_checkpoint::<TestState>(&previous_id, storage);

                    match previous_state {
                        Ok(state) => {
                            info!(
                                "Successfully restored previous checkpoint: {} (version {})",
                                previous_id, state.version
                            );
                            return Ok((previous_id, state));
                        }
                        Err(prev_err) => {
                            warn!(
                                "Previous checkpoint {} also failed: {}, trying earlier...",
                                previous_id, prev_err
                            );
                            // Continue to the next previous checkpoint
                        }
                    }
                }

                // If we get here, we've exhausted all previous checkpoints
                Err(ChaosTestError::FallbackFailed {
                    reason: format!("all previous checkpoints failed for {}", checkpoint_id),
                })
            } else {
                Err(ChaosTestError::FallbackFailed {
                    reason: format!("restore failed with non-corruption error: {}", e),
                })
            }
        }
    }
}

// =============================================================================
// Test Functions
// =============================================================================

#[tokio::test]
async fn given_corrupted_latest_checkpoint_when_restoring_then_falls_back_to_previous() {
    let test_name = "corrupted_latest_fallback";
    info!("Starting test: {}", test_name);

    // Setup: Create 3 sequential checkpoints
    let mut storage = InMemoryCheckpointStorage::new();
    let checkpoints =
        create_checkpoint_sequence(&mut storage, 3).expect("Failed to create checkpoint sequence");

    let (cp1_id, state1) = &checkpoints[0];
    let (cp2_id, state2) = &checkpoints[1];
    let (cp3_id, state3) = &checkpoints[2];

    info!("Created checkpoints: {}, {}, {}", cp1_id, cp2_id, cp3_id);

    // Corrupt the latest checkpoint (cp3)
    corrupt_checkpoint_in_storage(&mut storage, cp3_id).expect("Failed to corrupt checkpoint");

    // Attempt to restore with fallback
    let (restored_id, restored_state) = restore_with_fallback(cp3_id, &storage, &checkpoints)
        .expect("Fallback to previous checkpoint should succeed");

    // Verify fallback to cp2
    assert_eq!(
        restored_id, *cp2_id,
        "Should fall back to previous checkpoint (cp2)"
    );

    assert_eq!(
        restored_state, *state2,
        "Restored state should match previous checkpoint state"
    );

    // Verify we did NOT get cp3 (corrupted) or cp1 (too old)
    assert_ne!(
        restored_id, *cp3_id,
        "Should not restore corrupted checkpoint"
    );
    assert_ne!(restored_id, *cp1_id, "Should not skip to oldest checkpoint");

    info!("Test passed: {}", test_name);
}

#[tokio::test]
async fn given_corrupted_middle_checkpoint_when_restoring_then_falls_back_to_previous() {
    let test_name = "corrupted_middle_fallback";
    info!("Starting test: {}", test_name);

    // Setup: Create 3 sequential checkpoints
    let mut storage = InMemoryCheckpointStorage::new();
    let checkpoints =
        create_checkpoint_sequence(&mut storage, 3).expect("Failed to create checkpoint sequence");

    let (cp1_id, state1) = &checkpoints[0];
    let (cp2_id, _state2) = &checkpoints[1];
    let (cp3_id, _state3) = &checkpoints[2];

    info!("Created checkpoints: {}, {}, {}", cp1_id, cp2_id, cp3_id);

    // Corrupt the middle checkpoint (cp2)
    corrupt_checkpoint_in_storage(&mut storage, cp2_id).expect("Failed to corrupt checkpoint");

    // Attempt to restore cp2 with fallback
    let (restored_id, restored_state) = restore_with_fallback(cp2_id, &storage, &checkpoints)
        .expect("Fallback to previous checkpoint should succeed");

    // Verify fallback to cp1
    assert_eq!(
        restored_id, *cp1_id,
        "Should fall back to previous checkpoint (cp1)"
    );

    assert_eq!(
        restored_state, *state1,
        "Restored state should match cp1 state"
    );

    assert_ne!(
        restored_id, *cp2_id,
        "Should not restore corrupted checkpoint"
    );

    info!("Test passed: {}", test_name);
}

#[tokio::test]
async fn given_corrupted_first_checkpoint_when_restoring_then_fails_no_fallback() {
    let test_name = "corrupted_first_no_fallback";
    info!("Starting test: {}", test_name);

    // Setup: Create 3 sequential checkpoints
    let mut storage = InMemoryCheckpointStorage::new();
    let checkpoints =
        create_checkpoint_sequence(&mut storage, 3).expect("Failed to create checkpoint sequence");

    let (cp1_id, _state1) = &checkpoints[0];

    info!(
        "Created checkpoints: {}, {}, {}",
        checkpoints[0].0, checkpoints[1].0, checkpoints[2].0
    );

    // Corrupt the first checkpoint (cp1)
    corrupt_checkpoint_in_storage(&mut storage, cp1_id).expect("Failed to corrupt checkpoint");

    // Attempt to restore cp1 with fallback (should fail - no previous)
    let result = restore_with_fallback(cp1_id, &storage, &checkpoints);

    assert!(
        result.is_err(),
        "Should fail when no previous checkpoint available"
    );

    match result {
        Err(ChaosTestError::FallbackFailed { .. }) => {
            info!("Correctly returned FallbackFailed error when no previous checkpoints exist");
        }
        Err(e) => {
            panic!("Unexpected error type: {}", e);
        }
        Ok(_) => {
            panic!("Should have failed when first checkpoint is corrupted");
        }
    }

    info!("Test passed: {}", test_name);
}

#[tokio::test]
async fn given_all_checkpoints_corrupted_when_restoring_then_fails_completely() {
    let test_name = "all_corrupted_fail";
    info!("Starting test: {}", test_name);

    // Setup: Create 3 sequential checkpoints
    let mut storage = InMemoryCheckpointStorage::new();
    let checkpoints =
        create_checkpoint_sequence(&mut storage, 3).expect("Failed to create checkpoint sequence");

    let (cp1_id, _) = checkpoints[0];
    let (cp2_id, _) = checkpoints[1];
    let (cp3_id, _) = checkpoints[2];

    info!("Created checkpoints: {}, {}, {}", cp1_id, cp2_id, cp3_id);

    // Corrupt all checkpoints
    for (id, _) in &checkpoints {
        corrupt_checkpoint_in_storage(&mut storage, id).expect("Failed to corrupt checkpoint");
    }

    // Attempt to restore cp3 with fallback (should fail - all corrupted)
    let result = restore_with_fallback(&cp3_id, &storage, &checkpoints);

    assert!(
        result.is_err(),
        "Should fail when all checkpoints are corrupted"
    );

    match result {
        Err(ChaosTestError::FallbackFailed { .. }) => {
            info!("Correctly returned FallbackFailed error");
        }
        Err(e) => {
            panic!("Unexpected error type: {}", e);
        }
        Ok(_) => {
            panic!("Should have failed when all checkpoints are corrupted");
        }
    }

    info!("Test passed: {}", test_name);
}

#[tokio::test]
async fn given_uncorrupted_checkpoint_when_restoring_then_succeeds_without_fallback() {
    let test_name = "uncorrupted_no_fallback_needed";
    info!("Starting test: {}", test_name);

    // Setup: Create 3 sequential checkpoints
    let mut storage = InMemoryCheckpointStorage::new();
    let checkpoints =
        create_checkpoint_sequence(&mut storage, 3).expect("Failed to create checkpoint sequence");

    let (cp3_id, state3) = &checkpoints[2];

    info!(
        "Created checkpoints: {}, {}, {}",
        checkpoints[0].0, checkpoints[1].0, cp3_id
    );

    // Do NOT corrupt any checkpoints

    // Attempt to restore cp3 (should succeed without fallback)
    let (restored_id, restored_state) = restore_with_fallback(cp3_id, &storage, &checkpoints)
        .expect("Restore should succeed without fallback");

    // Verify we got cp3 (not a fallback)
    assert_eq!(
        restored_id, *cp3_id,
        "Should restore requested checkpoint without fallback"
    );

    assert_eq!(
        restored_state, *state3,
        "Restored state should match cp3 state"
    );

    info!("Test passed: {}", test_name);
}

#[tokio::test]
async fn test_invariant_state_consistency_after_fallback() {
    let test_name = "state_consistency_invariant";
    info!("Starting test: {}", test_name);

    // Setup: Create 5 sequential checkpoints
    let mut storage = InMemoryCheckpointStorage::new();
    let checkpoints =
        create_checkpoint_sequence(&mut storage, 5).expect("Failed to create checkpoint sequence");

    // Corrupt checkpoints 2, 3, and 4
    let corrupted_ids = vec![checkpoints[1].0, checkpoints[2].0, checkpoints[3].0];
    for id in &corrupted_ids {
        corrupt_checkpoint_in_storage(&mut storage, id).expect("Failed to corrupt checkpoint");
    }

    info!("Corrupted checkpoints: {:?}", corrupted_ids);

    // Attempt to restore cp4 (should fall back to cp1)
    let cp4_id = checkpoints[3].0;
    let (restored_id, restored_state) =
        restore_with_fallback(&cp4_id, &storage, &checkpoints).expect("Fallback should succeed");

    // Invariant: Restored state should be valid TestState
    assert_eq!(
        restored_state.version, 1,
        "Version should match cp1 (version 1)"
    );

    assert_eq!(
        restored_state.counter, 100,
        "Counter should match cp1 (counter 100)"
    );

    assert_eq!(
        restored_state.items.len(),
        1,
        "Items should match cp1 (1 item)"
    );

    // Invariant: Restored ID should be cp1
    let cp1_id = checkpoints[0].0;
    assert_eq!(
        restored_id, cp1_id,
        "Should fall back through all corrupted checkpoints to cp1"
    );

    info!("Test passed: {}", test_name);
}

#[tokio::test]
async fn test_postcondition_fallback_never_skips_valid_checkpoint() {
    let test_name = "fallback_no_skip_invariant";
    info!("Starting test: {}", test_name);

    // Setup: Create 4 sequential checkpoints
    let mut storage = InMemoryCheckpointStorage::new();
    let checkpoints =
        create_checkpoint_sequence(&mut storage, 4).expect("Failed to create checkpoint sequence");

    // Corrupt only cp3
    let cp3_id = checkpoints[2].0;
    corrupt_checkpoint_in_storage(&mut storage, &cp3_id).expect("Failed to corrupt checkpoint");

    info!("Corrupted checkpoint: {}", cp3_id);

    // Attempt to restore cp3 (should fall back to cp2, NOT cp1)
    let (restored_id, restored_state) =
        restore_with_fallback(&cp3_id, &storage, &checkpoints).expect("Fallback should succeed");

    // Postcondition: Should fall back to immediate previous, not skip
    let cp2_id = checkpoints[1].0;
    let cp2_state = &checkpoints[1].1;

    assert_eq!(
        restored_id, cp2_id,
        "Should fall back to immediate previous checkpoint (cp2), not skip to cp1"
    );

    assert_eq!(restored_state, *cp2_state, "State should match cp2");

    let cp1_id = checkpoints[0].0;
    assert_ne!(
        restored_id, cp1_id,
        "Should not skip valid checkpoint cp2 to reach cp1"
    );

    info!("Test passed: {}", test_name);
}

// =============================================================================
// Integration Test
// =============================================================================

#[tokio::test]
async fn test_scenario_workflow_checkpoint_recovery_after_corruption() {
    let test_name = "workflow_recovery_scenario";
    info!("Starting integration test: {}", test_name);

    // Scenario: Workflow execution creates checkpoints at each stage
    let mut storage = InMemoryCheckpointStorage::new();

    // Simulate workflow stages
    let stages = vec!["initialize", "process", "validate", "finalize"];
    let mut checkpoints = Vec::new();

    for (i, stage) in stages.iter().enumerate() {
        let state = TestState::new(i as u32 + 1, (i as u64 + 1) * 1000);
        let checkpoint_id = CheckpointId::new();

        let serialized = serialize_state(&state).expect("serialization should succeed");

        let metadata = CheckpointMetadata {
            id: checkpoint_id,
            created_at: chrono::Utc::now(),
            version: 1,
            uncompressed_size: std::mem::size_of_val(&state),
            compressed_size: serialized.len(),
            compression_ratio: std::mem::size_of_val(&state) as f64 / serialized.len() as f64,
        };

        storage
            .store_checkpoint(serialized, metadata)
            .expect("store should succeed");

        checkpoints.push((checkpoint_id, state));
        info!(
            "Checkpoint created for stage '{}': {}",
            stage, checkpoint_id
        );
    }

    // Simulate corruption of "finalize" checkpoint
    let finalize_id = checkpoints[3].0;
    corrupt_checkpoint_in_storage(&mut storage, &finalize_id)
        .expect("Failed to corrupt checkpoint");

    info!(
        "Simulated corruption of finalize checkpoint: {}",
        finalize_id
    );

    // Attempt to recover workflow from finalize stage
    let (recovered_id, recovered_state) =
        restore_with_fallback(&finalize_id, &storage, &checkpoints)
            .expect("Workflow recovery should succeed with fallback");

    // Verify recovery to "validate" stage
    let validate_id = checkpoints[2].0;
    let validate_state = &checkpoints[2].1;

    assert_eq!(
        recovered_id, validate_id,
        "Workflow should recover to 'validate' stage"
    );

    assert_eq!(
        recovered_state.version, 3,
        "Recovered state should be at version 3 (validate stage)"
    );

    assert_eq!(
        recovered_state.counter, 3000,
        "Recovered counter should match validate stage"
    );

    info!(
        "Workflow successfully recovered from corruption: {} -> {} (version {})",
        finalize_id, recovered_id, recovered_state.version
    );

    info!("Integration test passed: {}", test_name);
}
