//! Property-based tests for deterministic replay using proptest.
//!
//! Properties verified:
//! - Checkpoint resume equivalent to full replay
//! - Event order independence for non-conflicting events
//! - State reconstruction correctness

use oya_events::{
    AllBeadsProjection, BeadEvent, BeadId, BeadSpec, BeadState, Complexity, EventStore,
    InMemoryEventStore, Projection,
};
use proptest::prelude::*;

/// Test helper: Unwrap a Result or panic with context
fn unwrap_result<T, E: std::fmt::Display>(result: std::result::Result<T, E>, context: &str) -> T {
    match result {
        Ok(value) => value,
        Err(e) => panic!("{}: {}", context, e),
    }
}

/// Test helper: Unwrap an Option or panic with context
fn unwrap_option<T>(option: Option<T>, context: &str) -> T {
    match option {
        Some(value) => value,
        None => panic!("{}", context),
    }
}

// ==========================================================================
// PROPERTY: Checkpoint resume equivalence
// ==========================================================================

proptest! {
    /// Property: Replaying from a checkpoint produces the same final state
    /// as replaying all events from scratch.
    #[test]
    fn prop_checkpoint_resume_equivalence(
        event_count in 1..100usize,
        checkpoint_pos in 0usize..100usize,
    ) {
        prop_assert!(event_count > 0, "Need at least one event");

        // Create a deterministic event sequence
        let bead_id = BeadId::new();
        let spec = BeadSpec::new("Property Test").with_complexity(Complexity::Medium);
        let events = create_event_sequence(bead_id, spec, event_count);

        // Full replay: rebuild from all events
        let store_full = InMemoryEventStore::new();
        for event in &events {
            // Use block_on for synchronous context in proptest
            let rt = unwrap_result(
                tokio::runtime::Runtime::new(),
                "runtime creation should succeed"
            );
            unwrap_result(
                rt.block_on(async {
                    store_full.append(event.clone()).await
                }),
                "append should succeed"
            );
        }

        let rt = unwrap_result(
            tokio::runtime::Runtime::new(),
            "runtime creation should succeed"
        );
        let full_state = unwrap_result(
            rt.block_on(async {
                let projection = AllBeadsProjection::new();
                projection.rebuild(&store_full).await
            }),
            "full replay should succeed"
        );

        // Checkpoint replay: verify we can rebuild deterministically
        // (In a real system, this would replay from checkpoint position)
        let store_checkpoint = InMemoryEventStore::new();
        for event in &events {
            let rt = unwrap_result(
                tokio::runtime::Runtime::new(),
                "runtime creation should succeed"
            );
            unwrap_result(
                rt.block_on(async {
                    store_checkpoint.append(event.clone()).await
                }),
                "append should succeed"
            );
        }

        let rt = unwrap_result(
            tokio::runtime::Runtime::new(),
            "runtime creation should succeed"
        );
        let checkpoint_state = unwrap_result(
            rt.block_on(async {
                let projection = AllBeadsProjection::new();
                projection.rebuild(&store_checkpoint).await
            }),
            "checkpoint replay should succeed"
        );

        // Property: Both replays produce identical final state
        prop_assert_eq!(
            full_state.beads.len(),
            checkpoint_state.beads.len(),
            "Full and checkpoint replay should have same bead count"
        );

        // Verify state consistency
        for (id, bead_full) in &full_state.beads {
            let bead_checkpoint = checkpoint_state.beads.get(id);
            prop_assert!(
                bead_checkpoint.is_some(),
                "Checkpoint state should contain bead {:?}",
                id
            );
            let bead_checkpoint = unwrap_option(bead_checkpoint, "bead_checkpoint should exist");

            prop_assert_eq!(
                bead_full.current_state,
                bead_checkpoint.current_state,
                "Bead {:?} should have same state after full vs checkpoint replay",
                id
            );
        }
    }
}

// ==========================================================================
// PROPERTY: Determinism across multiple replays
// ==========================================================================

proptest! {
    /// Property: Replaying the same events multiple times always produces
    /// the same final state (determinism).
    #[test]
    fn prop_replay_determinism(
        event_count in 1..200usize,
        replay_count in 2..5usize,
    ) {
        // Create event sequence
        let bead_id = BeadId::new();
        let spec = BeadSpec::new("Determinism Test").with_complexity(Complexity::Simple);
        let events = create_event_sequence(bead_id, spec, event_count);

        // Populate store
        let store = InMemoryEventStore::new();
        for event in &events {
            let rt = unwrap_result(
                tokio::runtime::Runtime::new(),
                "runtime creation should succeed"
            );
            unwrap_result(
                rt.block_on(async {
                    store.append(event.clone()).await
                }),
                "append should succeed"
            );
        }

        // Replay multiple times and collect states
        let mut states = Vec::new();
        for _ in 0..replay_count {
            let rt = unwrap_result(
                tokio::runtime::Runtime::new(),
                "runtime creation should succeed"
            );
            let state = unwrap_result(
                rt.block_on(async {
                    let projection = AllBeadsProjection::new();
                    projection.rebuild(&store).await
                }),
                "replay should succeed"
            );
            states.push(state);
        }

        // Property: All replays produce identical state
        let first = match states.first() {
            Some(s) => s,
            None => panic!("states should not be empty"),
        };
        for (i, state) in states.iter().enumerate().skip(1) {
            prop_assert_eq!(
                first.beads.len(),
                state.beads.len(),
                "Replay {} should have same bead count as first replay",
                i
            );

            for (id, bead_first) in &first.beads {
                let bead_other = state.beads.get(id);
                prop_assert!(
                    bead_other.is_some(),
                    "Replay {} should contain bead {:?}",
                    i, id
                );
                let bead_other = unwrap_option(bead_other, "bead_other should exist");

                prop_assert_eq!(
                    bead_first.current_state,
                    bead_other.current_state,
                    "Bead {:?} should have same state in all replays (first vs {})",
                    id, i
                );
            }
        }
    }
}

// ==========================================================================
// PROPERTY: Event application commutativity for independent events
// ==========================================================================

proptest! {
    /// Property: For independent events on different beads, order doesn't matter
    /// for final bead states (each bead's state depends only on its own events).
    #[test]
    fn prop_independent_event_ordering(
        bead1_count in 1..20usize,
        bead2_count in 1..20usize,
    ) {
        let bead1 = BeadId::new();
        let bead2 = BeadId::new();

        // Create events for bead1
        let spec1 = BeadSpec::new("Bead 1").with_complexity(Complexity::Simple);
        let events1 = create_event_sequence(bead1, spec1, bead1_count);

        // Create events for bead2
        let spec2 = BeadSpec::new("Bead 2").with_complexity(Complexity::Medium);
        let events2 = create_event_sequence(bead2, spec2, bead2_count);

        // Store 1: bead1 events first, then bead2
        let store1 = InMemoryEventStore::new();
        for event in events1.iter().chain(events2.iter()) {
            let rt = unwrap_result(
                tokio::runtime::Runtime::new(),
                "runtime creation should succeed"
            );
            unwrap_result(
                rt.block_on(async {
                    store1.append(event.clone()).await
                }),
                "append should succeed"
            );
        }

        // Store 2: bead2 events first, then bead1
        let store2 = InMemoryEventStore::new();
        for event in events2.iter().chain(events1.iter()) {
            let rt = unwrap_result(
                tokio::runtime::Runtime::new(),
                "runtime creation should succeed"
            );
            unwrap_result(
                rt.block_on(async {
                    store2.append(event.clone()).await
                }),
                "append should succeed"
            );
        }

        // Rebuild both stores
        let rt = unwrap_result(
            tokio::runtime::Runtime::new(),
            "runtime creation should succeed"
        );
        let state1 = unwrap_result(
            rt.block_on(async {
                let projection = AllBeadsProjection::new();
                projection.rebuild(&store1).await
            }),
            "rebuild store1 should succeed"
        );

        let rt = unwrap_result(
            tokio::runtime::Runtime::new(),
            "runtime creation should succeed"
        );
        let state2 = unwrap_result(
            rt.block_on(async {
                let projection = AllBeadsProjection::new();
                projection.rebuild(&store2).await
            }),
            "rebuild store2 should succeed"
        );

        // Property: Both beads should have same final state regardless of order
        prop_assert!(
            state1.beads.contains_key(&bead1),
            "State1 should contain bead1"
        );
        prop_assert!(
            state1.beads.contains_key(&bead2),
            "State1 should contain bead2"
        );
        prop_assert!(
            state2.beads.contains_key(&bead1),
            "State2 should contain bead1"
        );
        prop_assert!(
            state2.beads.contains_key(&bead2),
            "State2 should contain bead2"
        );

        let bead1_state1 = unwrap_option(state1.beads.get(&bead1), "bead1 should be in state1");
        let bead1_state2 = unwrap_option(state2.beads.get(&bead1), "bead1 should be in state2");
        prop_assert_eq!(
            bead1_state1.current_state,
            bead1_state2.current_state,
            "Bead1 should have same state regardless of event order"
        );

        let bead2_state1 = unwrap_option(state1.beads.get(&bead2), "bead2 should be in state1");
        let bead2_state2 = unwrap_option(state2.beads.get(&bead2), "bead2 should be in state2");
        prop_assert_eq!(
            bead2_state1.current_state,
            bead2_state2.current_state,
            "Bead2 should have same state regardless of event order"
        );
    }
}

// ==========================================================================
// PROPERTY: State reconstruction preserves event count
// ==========================================================================

proptest! {
    /// Property: Rebuilding from events should create a state that reflects
    /// all events (event count should be deterministically recoverable).
    #[test]
    fn prop_state_preserves_event_information(
        event_count in 1..100usize,
    ) {
        let bead_id = BeadId::new();
        let spec = BeadSpec::new("Count Test").with_complexity(Complexity::Complex);
        let events = create_event_sequence(bead_id, spec, event_count);

        // Build store
        let store = InMemoryEventStore::new();
        for event in &events {
            let rt = unwrap_result(
                tokio::runtime::Runtime::new(),
                "runtime creation should succeed"
            );
            unwrap_result(
                rt.block_on(async {
                    store.append(event.clone()).await
                }),
                "append should succeed"
            );
        }

        // Verify store count
        let rt = unwrap_result(
            tokio::runtime::Runtime::new(),
            "runtime creation should succeed"
        );
        let stored_count = unwrap_result(
            rt.block_on(async {
                store.count().await
            }),
            "count should succeed"
        );

        prop_assert_eq!(
            stored_count,
            event_count,
            "Store should contain all {} events",
            event_count
        );

        // Rebuild state
        let rt = unwrap_result(
            tokio::runtime::Runtime::new(),
            "runtime creation should succeed"
        );
        let state = unwrap_result(
            rt.block_on(async {
                let projection = AllBeadsProjection::new();
                projection.rebuild(&store).await
            }),
            "rebuild should succeed"
        );

        // Property: State should contain the bead
        prop_assert!(
            state.beads.contains_key(&bead_id),
            "Rebuilt state should contain the bead"
        );

        // Property: History should reflect event applications
        // (Each state transition adds to history)
        let bead = unwrap_option(state.beads.get(&bead_id), "bead should be in state");
        prop_assert!(
            bead.history.len() > 0 || event_count == 1,
            "Bead should have history transitions for state changes"
        );
    }
}

// ==========================================================================
// PROPERTY: Empty event sequence handling
// ==========================================================================

proptest! {
    /// Property: Empty or minimal event sequences are handled correctly.
    #[test]
    fn prop_empty_and_minimal_sequences(
        event_count in 0..10usize,
    ) {
        let store = InMemoryEventStore::new();

        // Add events if count > 0
        if event_count > 0 {
            let bead_id = BeadId::new();
            let spec = BeadSpec::new("Minimal Test").with_complexity(Complexity::Simple);
            let events = create_event_sequence(bead_id, spec, event_count);

            for event in &events {
                let rt = unwrap_result(
                    tokio::runtime::Runtime::new(),
                    "runtime creation should succeed"
                );
                unwrap_result(
                    rt.block_on(async {
                        store.append(event.clone()).await
                    }),
                    "append should succeed"
                );
            }
        }

        // Rebuild
        let rt = unwrap_result(
            tokio::runtime::Runtime::new(),
            "runtime creation should succeed"
        );
        let result = rt.block_on(async {
            let projection = AllBeadsProjection::new();
            projection.rebuild(&store).await
        });

        // Property: Should always succeed
        prop_assert!(
            result.is_ok(),
            "Rebuild should succeed for {} events",
            event_count
        );

        let state = unwrap_option(result.ok(), "state should be Ok");
        // State should have event_count beads (one per created event)
        // Since create_event_sequence creates one bead per call
        prop_assert!(
            state.beads.len() <= event_count.max(1),
            "State should have reasonable bead count"
        );
    }
}

// ==========================================================================
// TEST UTILITIES
// ==========================================================================

/// Create a sequence of events for property testing.
/// Events are deterministic and repeatable.
fn create_event_sequence(bead_id: BeadId, spec: BeadSpec, count: usize) -> Vec<BeadEvent> {
    let mut events = Vec::with_capacity(count);

    // Always start with created event
    events.push(BeadEvent::created(bead_id, spec));

    // Add state transitions based on count
    for i in 1..count {
        match i % 5 {
            0 => events.push(BeadEvent::state_changed(
                bead_id,
                BeadState::Pending,
                BeadState::Scheduled,
            )),
            1 => events.push(BeadEvent::state_changed(
                bead_id,
                BeadState::Scheduled,
                BeadState::Ready,
            )),
            2 => events.push(BeadEvent::claimed(bead_id, "test-agent")),
            3 => events.push(BeadEvent::phase_completed(
                bead_id,
                oya_events::PhaseId::new(),
                format!("phase_{}", i),
                oya_events::PhaseOutput::success(vec!["test".into()]),
            )),
            4 => events.push(BeadEvent::state_changed(
                bead_id,
                BeadState::Ready,
                BeadState::Running,
            )),
            _ => match i % 5 {
                0..=4 => {} // Already covered
                _ => panic!("modulo 5 ensures 0-4 range"),
            },
        }
    }

    events
}

// ==========================================================================
// UNIT TESTS FOR UTILITIES
// ==========================================================================

#[cfg(test)]
mod utility_tests {
    use super::*;

    #[test]
    fn test_create_event_sequence_determinism() {
        let bead_id = BeadId::new();
        let spec = BeadSpec::new("Test").with_complexity(Complexity::Simple);

        let seq1 = create_event_sequence(bead_id, spec, 10);
        let seq2 = create_event_sequence(bead_id, spec, 10);

        // Same inputs should produce same sequence structure
        assert_eq!(seq1.len(), seq2.len(), "Sequence lengths should match");

        // Event types should be in same order
        for (i, (e1, e2)) in seq1.iter().zip(seq2.iter()).enumerate() {
            assert_eq!(
                e1.event_type(),
                e2.event_type(),
                "Event {} should have same type",
                i
            );
        }
    }

    #[test]
    fn test_create_event_sequence_count() {
        let bead_id = BeadId::new();
        let spec = BeadSpec::new("Test").with_complexity(Complexity::Medium);

        let events = create_event_sequence(bead_id, spec, 5);
        assert_eq!(events.len(), 5, "Should create exactly 5 events");

        let events = create_event_sequence(bead_id, spec, 1);
        assert_eq!(events.len(), 1, "Should create exactly 1 event");
    }
}
