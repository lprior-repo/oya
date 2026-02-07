//! Deterministic replay unit tests.
//!
//! Tests verify that:
//! - Same events produce same final state (determinism)
//! - Replay performance meets targets (<5s for 1000 events)
//! - Progress tracking is accurate during replay

use oya_events::{
    AllBeadsProjection, BeadEvent, BeadId, BeadSpec, BeadState, Complexity, EventStore,
    InMemoryEventStore, Projection, ReplayTracker,
};
use proptest::prelude::*;
use std::time::{Duration, Instant};

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

#[cfg(test)]
mod deterministic_replay_tests {
    use super::*;

    // ==========================================================================
    // DETERMINISM TESTS
    // ==========================================================================

    #[tokio::test]
    async fn same_events_produce_same_final_state() {
        // GIVEN: An event store with a deterministic sequence of events
        let store = InMemoryEventStore::new();
        let bead_id = BeadId::new();
        let spec = BeadSpec::new("Test Bead").with_complexity(Complexity::Medium);

        // Create a repeatable event sequence
        let events = create_deterministic_event_sequence(bead_id, spec);

        // WHEN: Events are appended to store
        for event in &events {
            unwrap_result(store.append(event.clone()).await, "append should succeed");
        }

        // THEN: Two separate rebuilds produce identical state
        let projection1 = AllBeadsProjection::new();
        let state1 = unwrap_result(
            projection1.rebuild(&store).await,
            "first rebuild should succeed",
        );

        let projection2 = AllBeadsProjection::new();
        let state2 = unwrap_result(
            projection2.rebuild(&store).await,
            "second rebuild should succeed",
        );

        assert_eq!(
            state1.beads.len(),
            state2.beads.len(),
            "Both rebuilds should have same number of beads"
        );

        // Verify each bead's state is identical
        for (id1, bead1) in &state1.beads {
            let bead2 = &state2.beads[id1];
            assert_eq!(
                bead1.current_state, bead2.current_state,
                "Bead {:?} should have same state in both rebuilds",
                id1
            );
            assert_eq!(
                bead1.history.len(),
                bead2.history.len(),
                "Bead {:?} should have same history length",
                id1
            );
        }
    }

    #[tokio::test]
    async fn replay_is_idempotent() {
        // GIVEN: A store with events
        let store = InMemoryEventStore::new();
        let bead_id = BeadId::new();
        let spec = BeadSpec::new("Idempotent Test").with_complexity(Complexity::Simple);

        let events = create_deterministic_event_sequence(bead_id, spec);
        for event in &events {
            unwrap_result(store.append(event.clone()).await, "append should succeed");
        }

        // WHEN: Rebuilding multiple times
        let projection = AllBeadsProjection::new();
        let state1 = unwrap_result(
            projection.rebuild(&store).await,
            "first rebuild should succeed",
        );

        let state2 = unwrap_result(
            projection.rebuild(&store).await,
            "second rebuild should succeed",
        );

        let state3 = unwrap_result(
            projection.rebuild(&store).await,
            "third rebuild should succeed",
        );

        // THEN: All rebuilds produce identical state
        assert_eq!(
            state1.beads.len(),
            state2.beads.len(),
            "First and second rebuild should match"
        );
        assert_eq!(
            state2.beads.len(),
            state3.beads.len(),
            "Second and third rebuild should match"
        );

        // Verify final states are identical
        for (id, bead1) in &state1.beads {
            let bead2 = &state2.beads[id];
            let bead3 = &state3.beads[id];
            assert_eq!(bead1.current_state, bead2.current_state);
            assert_eq!(bead2.current_state, bead3.current_state);
        }
    }

    proptest! {
        /// Property: Replay is deterministic for variable event counts
        #[test]
        fn prop_replay_determinism_variable_events(
            event_count in 50usize..500,
        ) {
            let rt = unwrap_result(
                tokio::runtime::Runtime::new(),
                "Failed to create runtime"
            );

            let result = rt.block_on(async {
                // GIVEN: A store with event_count events
                let store = InMemoryEventStore::new();
                let bead_id = BeadId::new();
                let _spec = BeadSpec::new("Checkpoint Test").with_complexity(Complexity::Medium);

                // Create event_count events
                for i in 0..event_count {
                    let event = if i % 2 == 0 {
                        BeadEvent::state_changed(bead_id, BeadState::Pending, BeadState::Scheduled)
                    } else {
                        BeadEvent::state_changed(bead_id, BeadState::Scheduled, BeadState::Ready)
                    };
                    unwrap_result(
                        store.append(event).await,
                        "append should succeed"
                    );
                }

                // WHEN: Replaying all events twice
                let projection1 = AllBeadsProjection::new();
                let state1 = unwrap_result(
                    projection1.rebuild(&store).await,
                    "first replay should succeed"
                );

                let projection2 = AllBeadsProjection::new();
                let state2 = unwrap_result(
                    projection2.rebuild(&store).await,
                    "second replay should succeed"
                );

                // THEN: Both replays should produce identical state
                prop_assert_eq!(
                    state1.beads.len(),
                    state2.beads.len(),
                    "Both replays should have same bead count"
                );

                // Verify each bead's state is identical
                for (id1, bead1) in &state1.beads {
                    let bead2 = &state2.beads[id1];
                    prop_assert_eq!(
                        bead1.current_state, bead2.current_state,
                        "Bead {:?} should have same state in both replays",
                        id1
                    );
                }

                Ok(())
            });

            // Propagate any test failures
            if let Err(e) = result {
                panic!("Test failed: {}", e);
            }
        }
    }

    // ==========================================================================
    // PERFORMANCE TESTS
    // ==========================================================================

    proptest! {
        /// Property: Replay performance scales with event count
        #[test]
        fn prop_replay_performance_scales(
            event_count in 100usize..1000,
        ) {
            let rt = unwrap_result(
                tokio::runtime::Runtime::new(),
                "Failed to create runtime"
            );

            let result = rt.block_on(async {
                // GIVEN: A store with event_count events
                let store = InMemoryEventStore::new();
                let bead_id = BeadId::new();
                let spec = BeadSpec::new("Performance Test").with_complexity(Complexity::Complex);

                // Create the bead first
                unwrap_result(
                    store.append(BeadEvent::created(bead_id, spec)).await,
                    "append should succeed"
                );

                // Add event_count events
                for i in 0..event_count {
                    let event = match i % 4 {
                        0 => BeadEvent::state_changed(bead_id, BeadState::Pending, BeadState::Scheduled),
                        1 => BeadEvent::state_changed(bead_id, BeadState::Scheduled, BeadState::Ready),
                        2 => BeadEvent::claimed(bead_id, "agent-1"),
                        _ => BeadEvent::phase_completed(
                            bead_id,
                            oya_events::PhaseId::new(),
                            "test_phase",
                            oya_events::PhaseOutput::success(b"output".to_vec()),
                        ),
                    };
                    unwrap_result(
                        store.append(event).await,
                        "append should succeed"
                    );
                }

                // WHEN: Replaying all events
                let start = Instant::now();
                let projection = AllBeadsProjection::new();
                let result = projection.rebuild(&store).await;
                let duration = start.elapsed();

                // THEN: Replay should succeed
                prop_assert!(result.is_ok(), "Rebuild should succeed: {:?}", result.err());

                let state = unwrap_result(result, "rebuild result");

                // Performance: Should scale reasonably (allow 5ms per event as upper bound)
                let expected_max_duration = Duration::from_millis(event_count as u64 * 5);
                prop_assert!(
                    duration < expected_max_duration,
                    "Replay of {} events should take <{:?}, took {:?}",
                    event_count, expected_max_duration, duration
                );

                // Verify state was actually built
                prop_assert!(
                    state.beads.contains_key(&bead_id),
                    "State should contain the test bead"
                );

                Ok(())
            });

            // Propagate any test failures
            if let Err(e) = result {
                panic!("Test failed: {}", e);
            }
        }
    }

    proptest! {
        /// Property: Progress tracking is accurate for variable event counts
        #[test]
        fn prop_progress_tracking_accuracy(
            event_count in 50usize..200,
        ) {
            let rt = unwrap_result(
                tokio::runtime::Runtime::new(),
                "Failed to create runtime"
            );

            let result = rt.block_on(async {
                // GIVEN: A store with event_count events and a progress tracker
                let store = InMemoryEventStore::new();
                let bead_id = BeadId::new();
                let _spec = BeadSpec::new("Progress Test").with_complexity(Complexity::Medium);

                // Add event_count events
                for _ in 0..event_count {
                    let event = BeadEvent::state_changed(bead_id, BeadState::Pending, BeadState::Scheduled);
                    unwrap_result(
                        store.append(event).await,
                        "append should succeed"
                    );
                }

                // WHEN: Rebuilding with progress tracking
                let event_count_u64 = event_count as u64;
                let (tracker, _rx) = ReplayTracker::new(event_count_u64, 10);
                let projection = AllBeadsProjection::new();
                let rebuild_result = projection
                    .rebuild_with_progress(&store, Some(&tracker))
                    .await;

                // THEN: Rebuild should succeed
                prop_assert!(
                    rebuild_result.is_ok(),
                    "Rebuild with progress should succeed: {:?}",
                    rebuild_result.err()
                );

                // THEN: Progress should reach 100%
                let final_progress = tracker.current_progress();
                prop_assert_eq!(
                    final_progress.events_processed, event_count_u64,
                    "Should process all {} events",
                    event_count
                );
                prop_assert_eq!(
                    final_progress.percent_complete, 100.0,
                    "Should be 100% complete"
                );

                Ok(())
            });

            // Propagate any test failures
            if let Err(e) = result {
                panic!("Test failed: {}", e);
            }
        }
    }

    // ==========================================================================
    // PROGRESS TRACKING TESTS
    // ==========================================================================

    #[tokio::test]
    async fn progress_tracker_accuracy() {
        // GIVEN: A tracker for 50 events
        let (tracker, _rx) = ReplayTracker::new(50, 10);
        let store = InMemoryEventStore::new();
        let bead_id = BeadId::new();

        // Add 50 events
        for _ in 0..50 {
            let event = BeadEvent::state_changed(bead_id, BeadState::Pending, BeadState::Scheduled);
            unwrap_result(store.append(event).await, "append should succeed");
        }

        // WHEN: Rebuilding with tracker
        let projection = AllBeadsProjection::new();
        unwrap_result(
            projection
                .rebuild_with_progress(&store, Some(&tracker))
                .await,
            "rebuild should succeed",
        );

        // THEN: Tracker should report accurate counts
        let progress = tracker.current_progress();
        assert_eq!(progress.events_total, 50, "Total should match");
        assert_eq!(progress.events_processed, 50, "Processed should match");
        assert_eq!(progress.percent_complete, 100.0, "Should be 100%");
    }

    // ==========================================================================
    // EDGE CASE TESTS
    // ==========================================================================

    #[tokio::test]
    async fn replay_empty_store() {
        // GIVEN: An empty store
        let store = InMemoryEventStore::new();

        // WHEN: Rebuilding
        let projection = AllBeadsProjection::new();
        let result = projection.rebuild(&store).await;

        // THEN: Should succeed with empty state
        assert!(result.is_ok(), "Rebuild of empty store should succeed");
        let state = unwrap_result(result, "state");
        assert_eq!(state.beads.len(), 0, "State should have no beads");
    }

    #[tokio::test]
    async fn replay_single_event() {
        // GIVEN: A store with one event
        let store = InMemoryEventStore::new();
        let bead_id = BeadId::new();
        let spec = BeadSpec::new("Single Event").with_complexity(Complexity::Simple);

        let event = BeadEvent::created(bead_id, spec);
        unwrap_result(store.append(event).await, "append should succeed");

        // WHEN: Rebuilding
        let projection = AllBeadsProjection::new();
        let result = projection.rebuild(&store).await;

        // THEN: Should succeed with one bead
        assert!(result.is_ok(), "Rebuild should succeed");
        let state = unwrap_result(result, "state");
        assert_eq!(state.beads.len(), 1, "State should have one bead");
        assert!(
            state.beads.contains_key(&bead_id),
            "State should contain the bead"
        );
    }

    #[tokio::test]
    async fn replay_handles_multiple_beads() {
        // GIVEN: A store with events for multiple beads
        let store = InMemoryEventStore::new();
        let bead1 = BeadId::new();
        let bead2 = BeadId::new();
        let bead3 = BeadId::new();

        // Add events for each bead
        for bead_id in [bead1, bead2, bead3] {
            let spec = BeadSpec::new("Multi-Bead Test").with_complexity(Complexity::Medium);
            unwrap_result(
                store.append(BeadEvent::created(bead_id, spec)).await,
                "append should succeed",
            );
            unwrap_result(
                store
                    .append(BeadEvent::state_changed(
                        bead_id,
                        BeadState::Pending,
                        BeadState::Scheduled,
                    ))
                    .await,
                "append should succeed",
            );
        }

        // WHEN: Rebuilding
        let projection = AllBeadsProjection::new();
        let result = projection.rebuild(&store).await;

        // THEN: Should have all three beads
        assert!(result.is_ok(), "Rebuild should succeed");
        let state = unwrap_result(result, "state");
        assert_eq!(state.beads.len(), 3, "State should have 3 beads");
        assert!(state.beads.contains_key(&bead1));
        assert!(state.beads.contains_key(&bead2));
        assert!(state.beads.contains_key(&bead3));
    }
}

// ==========================================================================
// TEST UTILITIES
// ==========================================================================

/// Create a deterministic sequence of events for testing.
fn create_deterministic_event_sequence(bead_id: BeadId, spec: BeadSpec) -> Vec<BeadEvent> {
    let mut events = Vec::new();

    // Start with created event
    events.push(BeadEvent::created(bead_id, spec));

    // Add predictable state transitions
    events.push(BeadEvent::state_changed(
        bead_id,
        BeadState::Pending,
        BeadState::Scheduled,
    ));
    events.push(BeadEvent::state_changed(
        bead_id,
        BeadState::Scheduled,
        BeadState::Ready,
    ));
    events.push(BeadEvent::claimed(bead_id, "test-agent"));

    events
}
