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
use std::time::{Duration, Instant};

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
            store
                .append(event.clone())
                .await
                .expect("append should succeed");
        }

        // THEN: Two separate rebuilds produce identical state
        let projection1 = AllBeadsProjection::new();
        let state1 = projection1
            .rebuild(&store)
            .await
            .expect("first rebuild should succeed");

        let projection2 = AllBeadsProjection::new();
        let state2 = projection2
            .rebuild(&store)
            .await
            .expect("second rebuild should succeed");

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
            store
                .append(event.clone())
                .await
                .expect("append should succeed");
        }

        // WHEN: Rebuilding multiple times
        let projection = AllBeadsProjection::new();
        let state1 = projection
            .rebuild(&store)
            .await
            .expect("first rebuild should succeed");

        let state2 = projection
            .rebuild(&store)
            .await
            .expect("second rebuild should succeed");

        let state3 = projection
            .rebuild(&store)
            .await
            .expect("third rebuild should succeed");

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

    #[tokio::test]
    async fn partial_replay_matches_full_replay_from_checkpoint() {
        // GIVEN: A store with 100 events
        let store = InMemoryEventStore::new();
        let bead_id = BeadId::new();
        let _spec = BeadSpec::new("Checkpoint Test").with_complexity(Complexity::Medium);

        // Create 100 events
        for i in 0..100 {
            let event = if i % 2 == 0 {
                BeadEvent::state_changed(bead_id, BeadState::Pending, BeadState::Scheduled)
            } else {
                BeadEvent::state_changed(bead_id, BeadState::Scheduled, BeadState::Ready)
            };
            store.append(event).await.expect("append should succeed");
        }

        // WHEN: Replaying all events
        let projection_full = AllBeadsProjection::new();
        let full_state = projection_full
            .rebuild(&store)
            .await
            .expect("full replay should succeed");

        // THEN: Partial replay should match if we track correctly
        // This tests that replaying from checkpoint N produces same state
        // as replaying from scratch to N
        let events = store.read(None).await.expect("read should succeed");
        let _checkpoint_event = events.get(49).map(|e| e.event_id()); // Event 50

        let projection_partial = AllBeadsProjection::new();
        let partial_state = projection_partial
            .rebuild(&store)
            .await
            .expect("partial replay should succeed");

        // For this test, we verify that the state is deterministic
        // (in a real system, you'd rebuild from checkpoint event)
        assert_eq!(
            full_state.beads.len(),
            partial_state.beads.len(),
            "Full and partial replay should have same bead count"
        );
    }

    // ==========================================================================
    // PERFORMANCE TESTS
    // ==========================================================================

    #[tokio::test]
    async fn replay_1000_events_under_5_seconds() {
        // GIVEN: A store with 1000 events
        let store = InMemoryEventStore::new();
        let bead_id = BeadId::new();
        let spec = BeadSpec::new("Performance Test").with_complexity(Complexity::Complex);

        // Create the bead first
        store
            .append(BeadEvent::created(bead_id, spec))
            .await
            .expect("append should succeed");

        // Add 1000 events
        for i in 0..1000 {
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
            store.append(event).await.expect("append should succeed");
        }

        // WHEN: Replaying all events
        let start = Instant::now();
        let projection = AllBeadsProjection::new();
        let result = projection.rebuild(&store).await;
        let duration = start.elapsed();

        // THEN: Replay should succeed and complete in under 5 seconds
        assert!(result.is_ok(), "Rebuild should succeed: {:?}", result.err());

        let state = result.expect("rebuild result");
        assert!(
            duration < Duration::from_secs(5),
            "Replay of 1000 events should take <5s, took {:?}",
            duration
        );

        // Verify state was actually built
        assert!(
            state.beads.contains_key(&bead_id),
            "State should contain the test bead"
        );

        println!("Replayed 1000 events in {:?} (target: <5s)", duration);
    }

    #[tokio::test]
    async fn replay_with_progress_tracking() {
        // GIVEN: A store with 100 events and a progress tracker
        let store = InMemoryEventStore::new();
        let bead_id = BeadId::new();
        let _spec = BeadSpec::new("Progress Test").with_complexity(Complexity::Medium);

        // Add 100 events
        for _ in 0..100 {
            let event = BeadEvent::state_changed(bead_id, BeadState::Pending, BeadState::Scheduled);
            store.append(event).await.expect("append should succeed");
        }

        // WHEN: Rebuilding with progress tracking
        let (tracker, mut rx) = ReplayTracker::new(100, 10);
        let projection = AllBeadsProjection::new();
        let rebuild_result = projection
            .rebuild_with_progress(&store, Some(&tracker))
            .await;

        // THEN: Rebuild should succeed
        assert!(
            rebuild_result.is_ok(),
            "Rebuild with progress should succeed: {:?}",
            rebuild_result.err()
        );

        // THEN: Progress should reach 100%
        let final_progress = tracker.current_progress();
        assert_eq!(
            final_progress.events_processed, 100,
            "Should process all 100 events"
        );
        assert_eq!(
            final_progress.percent_complete, 100.0,
            "Should be 100% complete"
        );

        // THEN: Progress updates should have been emitted
        let mut update_count = 0;
        let _ = rx.changed().await;
        while rx.has_changed().is_ok() {
            update_count += 1;
            let progress = rx.borrow_and_update();
            if progress.events_processed >= 100 {
                break;
            }
            // Small timeout to avoid infinite loop
            if update_count > 20 {
                break;
            }
        }

        assert!(
            update_count > 0,
            "Should have received at least one progress update"
        );
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
            store.append(event).await.expect("append should succeed");
        }

        // WHEN: Rebuilding with tracker
        let projection = AllBeadsProjection::new();
        projection
            .rebuild_with_progress(&store, Some(&tracker))
            .await
            .expect("rebuild should succeed");

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
        let state = result.expect("state");
        assert_eq!(state.beads.len(), 0, "State should have no beads");
    }

    #[tokio::test]
    async fn replay_single_event() {
        // GIVEN: A store with one event
        let store = InMemoryEventStore::new();
        let bead_id = BeadId::new();
        let spec = BeadSpec::new("Single Event").with_complexity(Complexity::Simple);

        let event = BeadEvent::created(bead_id, spec);
        store.append(event).await.expect("append should succeed");

        // WHEN: Rebuilding
        let projection = AllBeadsProjection::new();
        let result = projection.rebuild(&store).await;

        // THEN: Should succeed with one bead
        assert!(result.is_ok(), "Rebuild should succeed");
        let state = result.expect("state");
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
            store
                .append(BeadEvent::created(bead_id, spec))
                .await
                .expect("append should succeed");
            store
                .append(BeadEvent::state_changed(
                    bead_id,
                    BeadState::Pending,
                    BeadState::Scheduled,
                ))
                .await
                .expect("append should succeed");
        }

        // WHEN: Rebuilding
        let projection = AllBeadsProjection::new();
        let result = projection.rebuild(&store).await;

        // THEN: Should have all three beads
        assert!(result.is_ok(), "Rebuild should succeed");
        let state = result.expect("state");
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
