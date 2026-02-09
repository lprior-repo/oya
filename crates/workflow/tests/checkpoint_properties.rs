//! Property-based tests for checkpoint compression and round-trip serialization.
//!
//! These tests use proptest to verify:
//! - Arbitrary serializable state round-trips successfully (serialize → deserialize)
//! - Compress → decompress preserves data for any input
//! - Compression achieves 50%+ size reduction for typical workflow state
//! - Full checkpoint → restore cycle preserves exact state
//! - **Checkpoint + events_since_checkpoint = current_state** (event sourcing property)

use oya_workflow::{compress, compression_ratio, decompress, space_savings};
use proptest::prelude::*;
use std::collections::HashMap;

// Test: Compress → decompress round-trip preserves data for any input.
// This property test verifies that for any byte vector, compressing and
// then decompressing returns the original data exactly.
proptest! {
    #[test]
    fn prop_compress_decompress_roundtrip(data in prop::collection::vec(any::<u8>(), 0..10000)) {
        // GIVEN: Any arbitrary byte vector
        // WHEN: Compressed then decompressed
        let compressed = compress(&data);
        prop_assert!(compressed.is_ok(), "Compression should succeed for any input");

        if let Ok(compressed_value) = compressed {
            let decompressed = decompress(&compressed_value, data.len());
            prop_assert!(decompressed.is_ok(), "Decompression should succeed");

            // THEN: Original data is preserved exactly
            if let Ok(decompressed_value) = decompressed {
                prop_assert_eq!(decompressed_value, data, "Round-trip should preserve data exactly");
            }
        }
    }
}

// Test: Compression ratio is calculated correctly.
// Verifies the compression_ratio function returns accurate values:
// - ratio = uncompressed_size / compressed_size
// - Returns 1.0 when compressed_size is 0 (edge case)
proptest! {
    #[test]
    fn prop_compression_ratio_calculation(
        uncompressed in 1u64..10000,
        compressed in 1u64..10000
    ) {
        // GIVEN: Valid uncompressed and compressed sizes
        // WHEN: Calculate compression ratio
        let ratio = compression_ratio(uncompressed, compressed);

        // THEN: Ratio should be uncompressed / compressed
        let expected = uncompressed as f64 / compressed as f64;
        prop_assert!((ratio - expected).abs() < 0.01, "Ratio calculation should be accurate");

        // WHEN: Compressed size is 0 (edge case)
        let edge_ratio = compression_ratio(uncompressed, 0);

        // THEN: Should return 1.0 to avoid division by zero
        prop_assert_eq!(edge_ratio, 1.0, "Should handle zero compressed size");
    }
}

// Test: Space savings is calculated correctly.
// Verifies space_savings returns the correct bytes saved:
// - savings = uncompressed_size - compressed_size
// - Saturates at 0 (never negative)
proptest! {
    #[test]
    fn prop_space_savings_calculation(
        uncompressed in 1u64..10000,
        compressed in 0u64..10000
    ) {
        // GIVEN: Valid uncompressed and compressed sizes
        // WHEN: Calculate space savings
        let savings = space_savings(uncompressed, compressed);

        // THEN: Savings should be uncompressed - compressed (saturated)
        let expected = uncompressed.saturating_sub(compressed);
        prop_assert_eq!(savings, expected, "Space savings should be accurate");

        // WHEN: Compressed is larger than uncompressed
        let negative_savings = space_savings(100, 200);

        // THEN: Should return 0 (saturating_sub)
        prop_assert_eq!(negative_savings, 0, "Should not return negative savings");
    }
}

/// Test: Compression achieves target ratio for repetitive data.
///
/// This test verifies that typical workflow state (which often contains
/// repetitive strings, IDs, and patterns) achieves at least 50% compression.
#[test]
fn test_compression_achieves_50_percent_target() -> Result<(), Box<dyn std::error::Error>> {
    // GIVEN: Typical workflow state with repetitive patterns
    let repetitive_data = {
        let mut data = Vec::new();
        for i in 0..1000 {
            data.extend_from_slice(format!("workflow-phase-{}-checkpoint-state\n", i).as_bytes());
        }
        data
    };

    let original_size = repetitive_data.len();

    // WHEN: Compressed
    let compressed = compress(&repetitive_data);
    assert!(compressed.is_ok(), "Compression should succeed");

    let compressed = compressed.map_err(|e| format!("Compression failed: {}", e))?;
    let compressed_size = compressed.len();

    // THEN: Should achieve at least 50% size reduction
    let ratio = compression_ratio(original_size as u64, compressed_size as u64);
    let savings_pct = ((original_size - compressed_size) as f64 / original_size as f64) * 100.0;

    assert!(
        ratio >= 2.0,
        "Compression ratio should be at least 2.0 (50% reduction), got {:.2}",
        ratio
    );
    assert!(
        savings_pct >= 50.0,
        "Space savings should be at least 50%, got {:.1}%",
        savings_pct
    );

    println!("Compression stats:");
    println!("  Original size: {} bytes", original_size);
    println!("  Compressed size: {} bytes", compressed_size);
    println!("  Compression ratio: {:.2}", ratio);
    println!("  Space savings: {:.1}%", savings_pct);
    Ok(())
}

// Test: Compression achieves target for workflow-like data.
// Verifies that realistic workflow state compresses by at least 50%.
// Workflow state typically contains:
// - Repeated phase names (build, test, deploy)
// - Repeated field names (phase_id, timestamp, state)
// - UUID patterns
// - Timestamp structures
#[test]
fn test_workflow_compression_achieves_target() -> Result<(), Box<dyn std::error::Error>> {
    // GIVEN: Realistic workflow-like data with multiple phases
    // For this test, verify compression doesn't explode size
    // and achieves reasonable ratio for typical data
    let test_data = {
        let mut data = Vec::new();
        for _ in 0..100 {
            data.extend_from_slice(b"workflow-state-build-phase-completed");
            data.extend_from_slice(b"workflow-state-test-phase-completed");
            data.extend_from_slice(b"workflow-state-deploy-phase-completed");
        }
        data
    };

    let compressed = compress(&test_data).map_err(|e| format!("Compression failed: {}", e))?;
    let ratio = compression_ratio(test_data.len() as u64, compressed.len() as u64);

    assert!(
        ratio >= 2.0,
        "Typical workflow data should compress at least 2:1, got {:.2}",
        ratio
    );

    println!("Workflow compression test:");
    println!("  Test data size: {} bytes", test_data.len());
    println!("  Compressed size: {} bytes", compressed.len());
    println!("  Compression ratio: {:.2}", ratio);
    Ok(())
}

// Test: Property-based round-trip for complex data patterns.
// Generates arbitrary repetitive byte patterns and verifies they
// can be compressed and decompressed without errors.
proptest! {
    #[test]
    fn prop_complex_data_compression(
        pattern in prop::collection::vec(1u8..255u8, 1..100),
        repeat_count in 1usize..100
    ) {
        // GIVEN: Create data with repeated patterns
        let mut data = Vec::new();
        for _ in 0..repeat_count {
            data.extend_from_slice(&pattern);
        }

        // WHEN: Compressed
        let result = compress(&data);

        // THEN: Should succeed
        prop_assert!(result.is_ok(), "Should compress any data");

        if let Ok(compressed) = result {
            prop_assert!(!compressed.is_empty(), "Compressed data should not be empty");

            // WHEN: Decompressed
            let decompressed = decompress(&compressed, data.len());

            // THEN: Should match original
            prop_assert!(decompressed.is_ok(), "Should decompress successfully");
            if let Ok(decompressed_value) = decompressed {
                prop_assert_eq!(decompressed_value, data, "Round-trip should preserve data");
            }
        }
    }
}

// Test: Empty and minimal data round-trip correctly.
///
/// Edge case testing for empty/small inputs.
#[test]
fn test_edge_cases_compress_decompress() -> Result<(), Box<dyn std::error::Error>> {
    // Empty data
    let empty = vec![];
    let compressed = compress(&empty);
    assert!(compressed.is_ok(), "Empty data should compress");
    let compressed_inner = compressed.map_err(|e| format!("Compression failed: {}", e))?;
    let decompressed = decompress(&compressed_inner, 0);
    assert!(decompressed.is_ok(), "Empty data should decompress");
    assert_eq!(
        decompressed.map_err(|e| format!("Decompression failed: {}", e))?,
        empty,
        "Empty round-trip should work"
    );

    // Single byte
    let single = vec![42u8];
    let compressed = compress(&single).map_err(|e| format!("Compression failed: {}", e))?;
    let decompressed =
        decompress(&compressed, 1).map_err(|e| format!("Decompression failed: {}", e))?;
    assert_eq!(decompressed, single, "Single byte round-trip should work");

    // Highly repetitive data (best case for compression)
    let repetitive = vec![0xFFu8; 10000];
    let compressed = compress(&repetitive).map_err(|e| format!("Compression failed: {}", e))?;
    let ratio = compression_ratio(10000, compressed.len() as u64);
    assert!(
        ratio > 100.0,
        "Highly repetitive data should compress >100:1, got {:.2}",
        ratio
    );
    let decompressed =
        decompress(&compressed, 10000).map_err(|e| format!("Decompression failed: {}", e))?;
    assert_eq!(
        decompressed, repetitive,
        "Repetitive round-trip should work"
    );

    println!("Edge case compression ratio: {:.2}", ratio);
    Ok(())
}

// Test: Verify compression doesn't expand data significantly.
// While zstd can expand incompressible data slightly, it should not
// explode the size (e.g., should stay within 110% of original).
proptest! {
    #[test]
    fn prop_compression_does_not_expand_excessively(data in prop::collection::vec(any::<u8>(), 100..10000)) {
        // GIVEN: Any data
        let original_size = data.len();

        // WHEN: Compressed
        let compressed = compress(&data);
        prop_assert!(compressed.is_ok(), "Compression should succeed");

        if let Ok(compressed_data) = compressed {
            let compressed_size = compressed_data.len();

            // THEN: Compressed size should be reasonable
            // Allow up to 110% expansion for incompressible data
            let max_allowed = (original_size as f64 * 1.1) as usize;
            prop_assert!(
                compressed_size <= max_allowed,
                "Compression should not exceed 110% of original: {} > {}",
                compressed_size,
                max_allowed
            );
        }
    }
}

// ==========================================================================
// PROPERTY: Checkpoint + Events Since = Current State
// ==========================================================================

/// Test state for event sourcing property tests.
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
struct TestEventSourcedState {
    /// Map of bead ID to current state.
    bead_states: HashMap<String, BeadStateEntry>,
    /// Total number of events applied.
    events_applied: u64,
    /// Timestamp of last event.
    last_event_timestamp: Option<chrono::DateTime<chrono::Utc>>,
}

/// Entry in the bead state map.
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
struct BeadStateEntry {
    /// Current bead state.
    state: String,
    /// Event count for this bead.
    event_count: u64,
    /// Last update timestamp.
    updated_at: chrono::DateTime<chrono::Utc>,
}

impl TestEventSourcedState {
    /// Create a new empty state.
    fn new() -> Self {
        Self {
            bead_states: HashMap::new(),
            events_applied: 0,
            last_event_timestamp: None,
        }
    }

    /// Apply an event to the state.
    fn apply(&mut self, bead_id: &str, event_type: &str, timestamp: chrono::DateTime<chrono::Utc>) {
        let entry = self
            .bead_states
            .entry(bead_id.to_string())
            .or_insert_with(|| BeadStateEntry {
                state: "pending".to_string(),
                event_count: 0,
                updated_at: timestamp,
            });

        entry.state = event_type.to_string();
        entry.event_count += 1;
        entry.updated_at = timestamp;
        self.events_applied += 1;
        self.last_event_timestamp = Some(timestamp);
    }
}

impl Default for TestEventSourcedState {
    fn default() -> Self {
        Self::new()
    }
}

/// Test event for property testing.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
struct TestEvent {
    bead_id: String,
    event_type: String,
    timestamp: chrono::DateTime<chrono::Utc>,
}

/// Property: Checkpoint + events since checkpoint = current state.
///
/// This is the fundamental property of event sourcing systems:
/// - Take a checkpoint at some point in the event stream
/// - Restore the checkpoint
/// - Apply all events that occurred after the checkpoint
/// - Result should exactly match the current state
///
/// Property test:
/// - Generate arbitrary event sequences (1-100 events)
/// - Pick random checkpoint position (25%, 50%, 75%)
/// - Verify: restore(checkpoint) + apply(events_since) == current_state
proptest! {
    #[test]
    fn prop_checkpoint_plus_events_yields_current(
        event_count in 1usize..100,
        checkpoint_pct in 25usize..75
    ) {
        // GIVEN: A sequence of events and a checkpoint position
        let mut current_state = TestEventSourcedState::new();
        let mut events = Vec::with_capacity(event_count);

        // Generate events
        for i in 0..event_count {
            let bead_id = format!("bead-{}", i % 5); // 5 different beads
            let event_type = format!("event-{}", i);
            let timestamp = chrono::Utc::now() + chrono::Duration::milliseconds(i as i64);

            let event = TestEvent {
                bead_id: bead_id.clone(),
                event_type: event_type.clone(),
                timestamp,
            };

            current_state.apply(&bead_id, &event_type, timestamp);
            events.push(event);
        }

        // Calculate checkpoint position
        let checkpoint_idx = (event_count * checkpoint_pct / 100).min(event_count - 1);

        // WHEN: Create checkpoint at checkpoint_idx
        let checkpoint_state = {
            let mut state = TestEventSourcedState::new();
            for event in events.iter().take(checkpoint_idx + 1) {
                state.apply(&event.bead_id, &event.event_type, event.timestamp);
            }
            state
        };

        // Serialize and compress checkpoint
        let serialized = serde_json::to_vec(&checkpoint_state);
        prop_assert!(serialized.is_ok(), "Serialization should succeed");
        let serialized = serialized.unwrap_or_default();

        let compressed = compress(&serialized);
        prop_assert!(compressed.is_ok(), "Compression should succeed");
        let compressed = compressed.unwrap_or_default();

        // Decompress and restore checkpoint
        let decompressed = decompress(&compressed, serialized.len());
        prop_assert!(decompressed.is_ok(), "Decompression should succeed");
        let decompressed = decompressed.unwrap_or_default();

        let restored_state_result: Result<TestEventSourcedState, _> = serde_json::from_slice(&decompressed);
        prop_assert!(restored_state_result.is_ok(), "Deserialization should succeed");
        let restored_state = restored_state_result.unwrap_or_default();

        // THEN: Apply events since checkpoint
        let mut final_state = restored_state;
        for event in events.iter().skip(checkpoint_idx + 1) {
            final_state.apply(&event.bead_id, &event.event_type, event.timestamp);
        }

        // THEN: Final state should match current state
        prop_assert_eq!(
            final_state.events_applied,
            current_state.events_applied,
            "Event count should match"
        );

        prop_assert_eq!(
            final_state.bead_states.len(),
            current_state.bead_states.len(),
            "Bead count should match"
        );

        // Verify each bead state matches
        for (bead_id, current_entry) in &current_state.bead_states {
            let final_entry = final_state.bead_states.get(bead_id);
            prop_assert!(
                final_entry.is_some(),
                "Missing bead: {}",
                bead_id
            );
            let final_entry = final_entry.unwrap();

            prop_assert_eq!(
                &final_entry.state,
                &current_entry.state,
                "State for bead {} should match",
                bead_id
            );

            prop_assert_eq!(
                final_entry.event_count,
                current_entry.event_count,
                "Event count for bead {} should match",
                bead_id
            );
        }

        // Verify last timestamp matches
        match (final_state.last_event_timestamp, current_state.last_event_timestamp) {
            (Some(final_ts), Some(current_ts)) => {
                let diff = if final_ts > current_ts {
                    final_ts - current_ts
                } else {
                    current_ts - final_ts
                };
                prop_assert!(
                    diff < chrono::Duration::seconds(1),
                    "Last timestamp should match: got {}, expected {}",
                    final_ts,
                    current_ts
                );
            }
            (None, None) => {}, // Both None - OK
            _ => {
                prop_assert!(false, "Last timestamp mismatch: one is None, one is Some");
            }
        }
    }
}

/// Property: Multiple checkpoint positions all yield correct current state.
///
/// Verify that for any checkpoint position, the property holds.
proptest! {
    #[test]
    fn prop_multiple_checkpoint_positions_yield_correct_state(
        event_count in 10usize..100
    ) {
        // GIVEN: A sequence of events
        let mut current_state = TestEventSourcedState::new();
        let mut events = Vec::with_capacity(event_count);

        for i in 0..event_count {
            let bead_id = format!("bead-{}", i % 3); // 3 different beads
            let event_type = format!("event-{}", i);
            let timestamp = chrono::Utc::now() + chrono::Duration::milliseconds(i as i64);

            let event = TestEvent {
                bead_id: bead_id.clone(),
                event_type: event_type.clone(),
                timestamp,
            };

            current_state.apply(&bead_id, &event_type, timestamp);
            events.push(event);
        }

        // WHEN: Test multiple checkpoint positions
        let checkpoint_positions = vec![0, event_count / 4, event_count / 2, event_count - 1];

        for checkpoint_idx in checkpoint_positions {
            // Create checkpoint
            let checkpoint_state = {
                let mut state = TestEventSourcedState::new();
                for event in events.iter().take(checkpoint_idx + 1) {
                    state.apply(&event.bead_id, &event.event_type, event.timestamp);
                }
                state
            };

            // Serialize/deserialize round-trip
            let serialized = serde_json::to_vec(&checkpoint_state);
            prop_assert!(
                serialized.is_ok(),
                "Serialization should succeed at checkpoint {}",
                checkpoint_idx
            );
            let serialized = serialized.unwrap_or_default();

            let compressed = compress(&serialized);
            prop_assert!(
                compressed.is_ok(),
                "Compression should succeed at checkpoint {}",
                checkpoint_idx
            );
            let compressed = compressed.unwrap_or_default();

            let decompressed = decompress(&compressed, serialized.len());
            prop_assert!(
                decompressed.is_ok(),
                "Decompression should succeed at checkpoint {}",
                checkpoint_idx
            );
            let decompressed = decompressed.unwrap_or_default();

            let restored_state_result: Result<TestEventSourcedState, _> = serde_json::from_slice(&decompressed);
            prop_assert!(
                restored_state_result.is_ok(),
                "Deserialization should succeed at checkpoint {}",
                checkpoint_idx
            );
            let restored_state = restored_state_result.unwrap_or_default();

            // Apply events since checkpoint
            let mut final_state = restored_state;
            for event in events.iter().skip(checkpoint_idx + 1) {
                final_state.apply(&event.bead_id, &event.event_type, event.timestamp);
            }

            // THEN: Verify final state matches current state
            prop_assert_eq!(
                final_state.events_applied,
                current_state.events_applied,
                "Event count should match for checkpoint at {}",
                checkpoint_idx
            );

            prop_assert_eq!(
                final_state.bead_states.len(),
                current_state.bead_states.len(),
                "Bead count should match for checkpoint at {}",
                checkpoint_idx
            );
        }
    }
}

/// Property: Empty event stream with checkpoint at start.
///
/// Edge case: Checkpoint at position 0, then apply all events.
/// Should yield same state as applying all events from scratch.
#[test]
fn test_checkpoint_at_start_plus_all_events() -> Result<(), Box<dyn std::error::Error>> {
    // GIVEN: Empty checkpoint + event sequence
    let checkpoint_state = TestEventSourcedState::new();

    let mut events = Vec::new();
    for i in 0..10 {
        let bead_id = format!("bead-{}", i);
        let event_type = format!("event-{}", i);
        let timestamp = chrono::Utc::now() + chrono::Duration::milliseconds(i as i64);

        events.push(TestEvent {
            bead_id,
            event_type,
            timestamp,
        });
    }

    // WHEN: Build current state by applying all events
    let mut current_state = TestEventSourcedState::new();
    for event in &events {
        current_state.apply(&event.bead_id, &event.event_type, event.timestamp);
    }

    // WHEN: Restore empty checkpoint and apply all events
    let serialized = serde_json::to_vec(&checkpoint_state)?;
    let compressed = compress(&serialized)?;
    let decompressed = decompress(&compressed, serialized.len())?;
    let mut final_state: TestEventSourcedState = serde_json::from_slice(&decompressed)?;

    for event in &events {
        final_state.apply(&event.bead_id, &event.event_type, event.timestamp);
    }

    // THEN: States should match
    assert_eq!(final_state, current_state, "Empty checkpoint + all events should match current state");

    Ok(())
}

/// Property: Checkpoint at end + zero events = current state.
///
/// Edge case: Checkpoint at final event, no events to apply.
/// Should yield same state as checkpoint state.
#[test]
fn test_checkpoint_at_end_plus_zero_events() -> Result<(), Box<dyn std::error::Error>> {
    // GIVEN: Event sequence
    let mut current_state = TestEventSourcedState::new();
    let mut events = Vec::new();

    for i in 0..10 {
        let bead_id = format!("bead-{}", i);
        let event_type = format!("event-{}", i);
        let timestamp = chrono::Utc::now() + chrono::Duration::milliseconds(i as i64);

        let event = TestEvent {
            bead_id: bead_id.clone(),
            event_type: event_type.clone(),
            timestamp,
        };

        current_state.apply(&bead_id, &event_type, timestamp);
        events.push(event);
    }

    // WHEN: Checkpoint at end (after last event)
    let checkpoint_state = current_state.clone();

    // Serialize/deserialize round-trip
    let serialized = serde_json::to_vec(&checkpoint_state)?;
    let compressed = compress(&serialized)?;
    let decompressed = decompress(&compressed, serialized.len())?;
    let restored_state: TestEventSourcedState = serde_json::from_slice(&decompressed)?;

    // No events to apply (checkpoint at end)

    // THEN: Restored state should match current state
    assert_eq!(restored_state, current_state, "Checkpoint at end should match current state");

    Ok(())
}
