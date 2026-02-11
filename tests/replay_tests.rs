//! BDD-style integration tests for event replay.
//!
//! These tests verify that event replay correctly restores state.
//!
//! ## Test Scenario
//!
//! GIVEN: A sequence of recorded events
//! WHEN: Events are replayed
//! THEN: State matches what it was before
//!
//! ## Testing Philosophy
//!
//! Uses TDD15 workflow: RED → GREEN → REFACTOR
//! - Zero unwraps, zero panics
//! - Functional patterns throughout
//! - Railway-Oriented Programming for error handling

#![deny(clippy::unwrap_used)]
#![deny(clippy::expect_used)]
#![deny(clippy::panic)]

use oya_events::{
    BeadEvent, BeadId, BeadResult, BeadSpec, BeadState, Complexity, ConnectionConfig,
    DurableEventStore, PhaseId, PhaseOutput, connect,
    replay::{ApplyContext, EventSourcedState, apply_event, apply_events},
};
use std::collections::HashMap;
use std::sync::Arc;
use tempfile::TempDir;

/// Simple in-memory state that tracks bead states from events.
#[derive(Debug, Clone, Default)]
struct InMemoryState {
    /// Current state of each bead.
    bead_states: HashMap<BeadId, BeadState>,
}

impl InMemoryState {
    /// Create a new empty state.
    fn new() -> Self {
        Self::default()
    }

    /// Get the current state of a bead.
    fn get_state(&self, bead_id: BeadId) -> Option<BeadState> {
        self.bead_states.get(&bead_id).copied()
    }

    /// Set the state of a bead.
    fn set_state(&mut self, bead_id: BeadId, state: BeadState) {
        self.bead_states.insert(bead_id, state);
    }
}

impl EventSourcedState for InMemoryState {
    fn validate_transition(
        &self,
        bead_id: BeadId,
        from: BeadState,
        to: BeadState,
    ) -> Result<(), oya_events::replay::apply::ApplyError> {
        let current_state = self.get_state(bead_id);

        match current_state {
            None => {
                // Bead doesn't exist yet - first state is valid
                Ok(())
            }
            Some(current) => {
                if current == from || current.can_transition_to(to) {
                    Ok(())
                } else {
                    Err(oya_events::replay::apply::ApplyError::InvalidTransition {
                        bead_id,
                        from: current,
                        to,
                    })
                }
            }
        }
    }

    fn apply_event(
        &mut self,
        event: &BeadEvent,
    ) -> Result<(), oya_events::replay::apply::ApplyError> {
        let bead_id = event.bead_id();

        match event {
            BeadEvent::Created { spec, .. } => {
                self.bead_states.insert(bead_id, BeadState::Pending);
            }
            BeadEvent::StateChanged { from: _, to, .. } => {
                self.bead_states.insert(bead_id, *to);
            }
            BeadEvent::Failed { .. } => {
                self.bead_states.insert(bead_id, BeadState::Completed);
            }
            BeadEvent::Completed { .. } => {
                self.bead_states.insert(bead_id, BeadState::Completed);
            }
            _ => {
                // For other event types, don't change state
            }
        }

        Ok(())
    }
}

/// Test context for replay tests.
struct ReplayTestContext {
    _temp_dir: TempDir,
    store: DurableEventStore,
}

impl ReplayTestContext {
    /// Create a new test context with a temporary database.
    async fn new() -> Result<Self, Box<dyn std::error::Error>> {
        let temp_dir = TempDir::new()?;
        let storage_path = temp_dir.path().join("events");

        let config = ConnectionConfig::new(storage_path)
            .with_namespace("replay_test")
            .with_database("events");

        let db = connect(config).await?;
        let store = DurableEventStore::new(db).await?;

        Ok(Self {
            _temp_dir: temp_dir,
            store,
        })
    }
}

#[tokio::test]
async fn given_recorded_events_when_replayed_then_state_matches_original()
-> Result<(), Box<dyn std::error::Error>> {
    // ==========================================================================
    // GIVEN: A sequence of recorded events
    // ==========================================================================

    let context = ReplayTestContext::new().await?;

    // Create a bead and record its lifecycle events
    let bead_id = BeadId::new();
    let spec = BeadSpec::new("Test Bead").with_complexity(Complexity::Simple);

    // Event 1: Create the bead
    let event1 = BeadEvent::created(bead_id, spec.clone());
    context.store.append_event(&event1).await?;

    // Event 2: Transition from Pending to Scheduled
    let event2 = BeadEvent::state_changed(bead_id, BeadState::Pending, BeadState::Scheduled);
    context.store.append_event(&event2).await?;

    // Event 3: Transition from Scheduled to Ready
    let event3 = BeadEvent::state_changed(bead_id, BeadState::Scheduled, BeadState::Ready);
    context.store.append_event(&event3).await?;

    // Event 4: Claim the bead
    let event4 = BeadEvent::claimed(bead_id, "agent-1");
    context.store.append_event(&event4).await?;

    // Event 5: Transition from Ready to Running
    let event5 = BeadEvent::state_changed(bead_id, BeadState::Ready, BeadState::Running);
    context.store.append_event(&event5).await?;

    // Event 6: Complete a phase
    let phase_id = PhaseId::new();
    let event6 = BeadEvent::phase_completed(
        bead_id,
        phase_id,
        "implement",
        PhaseOutput::success(vec![1, 2, 3]),
    );
    context.store.append_event(&event6).await?;

    // Event 7: Transition to Completed
    let event7 = BeadEvent::completed(bead_id, BeadResult::success(vec![1, 2, 3], 1000));
    context.store.append_event(&event7).await?;

    // Build original state by applying all events
    let mut original_state = InMemoryState::new();
    let mut original_context = ApplyContext::new();
    let original_events = [event1, event2, event3, event4, event5, event6, event7];

    apply_events(&mut original_state, &original_events, &mut original_context)?;

    let original_bead_state = original_state
        .get_state(bead_id)
        .expect("Original state should have bead state");

    // ==========================================================================
    // WHEN: Events are replayed
    // ==========================================================================

    // Read events from the store
    let replayed_events = context.store.read_events(&bead_id).await?;

    // Replay events to rebuild state
    let mut replayed_state = InMemoryState::new();
    let mut replay_context = ApplyContext::new();

    apply_events(&mut replayed_state, &replayed_events, &mut replay_context)?;

    // ==========================================================================
    // THEN: State matches what it was before
    // ==========================================================================

    let replayed_bead_state = replayed_state
        .get_state(bead_id)
        .expect("Replayed state should have bead state");

    assert_eq!(
        original_bead_state, replayed_bead_state,
        "Replayed state should match original state: expected {:?}, got {:?}",
        original_bead_state, replayed_bead_state
    );

    assert_eq!(
        original_events.len(),
        replayed_events.len(),
        "Should replay all events: expected {}, got {}",
        original_events.len(),
        replayed_events.len()
    );

    // Verify the final state is Completed
    assert_eq!(
        replayed_bead_state,
        BeadState::Completed,
        "Final state should be Completed"
    );

    Ok(())
}

#[tokio::test]
async fn given_multiple_beads_when_replayed_then_all_states_match()
-> Result<(), Box<dyn std::error::Error>> {
    // GIVEN: Multiple beads with recorded events
    let context = ReplayTestContext::new().await?;

    let bead1_id = BeadId::new();
    let bead2_id = BeadId::new();
    let bead3_id = BeadId::new();

    // Bead 1: Simple lifecycle
    let bead1_events = vec![
        BeadEvent::created(
            bead1_id,
            BeadSpec::new("Bead 1").with_complexity(Complexity::Simple),
        ),
        BeadEvent::state_changed(bead1_id, BeadState::Pending, BeadState::Scheduled),
        BeadEvent::completed(bead1_id, BeadResult::success(vec![1], 100)),
    ];

    // Bead 2: Full lifecycle
    let bead2_events = vec![
        BeadEvent::created(
            bead2_id,
            BeadSpec::new("Bead 2").with_complexity(Complexity::Medium),
        ),
        BeadEvent::state_changed(bead2_id, BeadState::Pending, BeadState::Scheduled),
        BeadEvent::state_changed(bead2_id, BeadState::Scheduled, BeadState::Ready),
        BeadEvent::claimed(bead2_id, "agent-1"),
        BeadEvent::state_changed(bead2_id, BeadState::Ready, BeadState::Running),
        BeadEvent::completed(bead2_id, BeadResult::success(vec![1, 2], 200)),
    ];

    // Bead 3: Failed lifecycle
    let bead3_events = vec![
        BeadEvent::created(
            bead3_id,
            BeadSpec::new("Bead 3").with_complexity(Complexity::Complex),
        ),
        BeadEvent::state_changed(bead3_id, BeadState::Pending, BeadState::Scheduled),
        BeadEvent::state_changed(bead3_id, BeadState::Scheduled, BeadState::Ready),
        BeadEvent::claimed(bead3_id, "agent-2"),
        BeadEvent::failed(bead3_id, "execution error"),
    ];

    // Store all events
    for event in &bead1_events {
        context.store.append_event(event).await?;
    }
    for event in &bead2_events {
        context.store.append_event(event).await?;
    }
    for event in &bead3_events {
        context.store.append_event(event).await?;
    }

    // Build original state
    let mut original_state = InMemoryState::new();
    let mut original_context = ApplyContext::new();

    for event in bead1_events
        .iter()
        .chain(bead2_events.iter())
        .chain(bead3_events.iter())
    {
        apply_event(&mut original_state, event, &mut original_context)?;
    }

    // WHEN: Events are replayed
    let mut replayed_state = InMemoryState::new();
    let mut replay_context = ApplyContext::new();

    for bead_id in [bead1_id, bead2_id, bead3_id] {
        let events = context.store.read_events(&bead_id).await?;
        apply_events(&mut replayed_state, &events, &mut replay_context)?;
    }

    // THEN: All states match
    assert_eq!(
        original_state.get_state(bead1_id),
        replayed_state.get_state(bead1_id),
        "Bead 1 state should match"
    );

    assert_eq!(
        original_state.get_state(bead2_id),
        replayed_state.get_state(bead2_id),
        "Bead 2 state should match"
    );

    assert_eq!(
        original_state.get_state(bead3_id),
        replayed_state.get_state(bead3_id),
        "Bead 3 state should match"
    );

    // Verify final states
    assert_eq!(
        replayed_state.get_state(bead1_id),
        Some(BeadState::Completed),
        "Bead 1 should be Completed"
    );

    assert_eq!(
        replayed_state.get_state(bead2_id),
        Some(BeadState::Completed),
        "Bead 2 should be Completed"
    );

    assert_eq!(
        replayed_state.get_state(bead3_id),
        Some(BeadState::Completed),
        "Bead 3 should be Completed (failed)"
    );

    Ok(())
}

#[tokio::test]
async fn given_empty_event_log_when_replayed_then_state_remains_empty()
-> Result<(), Box<dyn std::error::Error>> {
    // GIVEN: Empty event log
    let context = ReplayTestContext::new().await?;
    let bead_id = BeadId::new();

    // WHEN: Events are replayed
    let events = context.store.read_events(&bead_id).await?;

    let mut replayed_state = InMemoryState::new();
    let mut replay_context = ApplyContext::new();

    apply_events(&mut replayed_state, &events, &mut replay_context)?;

    // THEN: State remains empty
    assert!(
        replayed_state.get_state(bead_id).is_none(),
        "State should be empty for bead with no events"
    );

    Ok(())
}

#[tokio::test]
async fn given_partial_event_sequence_when_replayed_then_state_is_partially_restored()
-> Result<(), Box<dyn std::error::Error>> {
    // GIVEN: Partial event sequence (not completed)
    let context = ReplayTestContext::new().await?;

    let bead_id = BeadId::new();

    let events = vec![
        BeadEvent::created(
            bead_id,
            BeadSpec::new("Partial Bead").with_complexity(Complexity::Medium),
        ),
        BeadEvent::state_changed(bead_id, BeadState::Pending, BeadState::Scheduled),
        BeadEvent::state_changed(bead_id, BeadState::Scheduled, BeadState::Ready),
    ];

    for event in &events {
        context.store.append_event(event).await?;
    }

    // Build original state
    let mut original_state = InMemoryState::new();
    let mut original_context = ApplyContext::new();
    apply_events(&mut original_state, &events, &mut original_context)?;

    // WHEN: Events are replayed
    let replayed_events = context.store.read_events(&bead_id).await?;

    let mut replayed_state = InMemoryState::new();
    let mut replay_context = ApplyContext::new();
    apply_events(&mut replayed_state, &replayed_events, &mut replay_context)?;

    // THEN: State is partially restored (not Completed)
    assert_eq!(
        original_state.get_state(bead_id),
        replayed_state.get_state(bead_id),
        "Replayed state should match original state"
    );

    assert_eq!(
        replayed_state.get_state(bead_id),
        Some(BeadState::Ready),
        "Bead should be in Ready state (not completed)"
    );

    Ok(())
}
