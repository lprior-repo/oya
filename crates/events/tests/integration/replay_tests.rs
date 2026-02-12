//! BDD Integration Test: Event Replay Restores State
//!
//! Tests verify that:
//! - GIVEN: A sequence of recorded events
//! - WHEN: Events are replayed
//! - THEN: State matches what it was before
//!
//! This is the core property of event sourcing - replaying events
//! deterministically produces the same state.

use oya_events::{
    AllBeadsProjection, BeadEvent, BeadId, BeadSpec, BeadState, Complexity, EventStore,
    InMemoryEventStore, PhaseId, PhaseOutput, Projection,
};

type TestResult<T> = std::result::Result<T, String>;

fn check_result<T, E: std::fmt::Display>(result: std::result::Result<T, E>, context: &str) -> TestResult<T> {
    result.map_err(|e| format!("{}: {}", context, e))
}

#[tokio::test]
async fn bdd_event_replay_restores_single_bead_state() -> TestResult<()> {
    let store = InMemoryEventStore::new();
    let bead_id = BeadId::new();
    let spec = BeadSpec::new("Test Bead").with_complexity(Complexity::Medium);

    check_result(store.append(BeadEvent::created(bead_id, spec.clone())).await, "append created")?;
    check_result(
        store
            .append(BeadEvent::state_changed(bead_id, BeadState::Pending, BeadState::Scheduled))
            .await,
        "append state change 1",
    )?;
    check_result(
        store
            .append(BeadEvent::state_changed(bead_id, BeadState::Scheduled, BeadState::Ready))
            .await,
        "append state change 2",
    )?;
    check_result(
        store.append(BeadEvent::claimed(bead_id, "agent-1")).await,
        "append claimed",
    )?;

    let projection = AllBeadsProjection::new();
    let original_state = check_result(projection.rebuild(&store).await, "rebuild original")?;

    let replayed_state = check_result(projection.rebuild(&store).await, "rebuild replay")?;

    assert_eq!(original_state.beads.len(), replayed_state.beads.len());
    assert!(replayed_state.beads.contains_key(&bead_id));

    let original_bead = original_state
        .beads
        .get(&bead_id)
        .ok_or_else(|| "Original bead not found".to_string())?;
    let replayed_bead = replayed_state
        .beads
        .get(&bead_id)
        .ok_or_else(|| "Replayed bead not found".to_string())?;

    assert_eq!(
        original_bead.current_state, replayed_bead.current_state,
        "State should match after replay"
    );
    assert_eq!(
        original_bead.history.len(), replayed_bead.history.len(),
        "History length should match after replay"
    );
    assert_eq!(
        original_bead.claimed_by, replayed_bead.claimed_by,
        "Claimed agent should match after replay"
    );

    Ok(())
}

#[tokio::test]
async fn bdd_event_replay_restores_multiple_beads_state() -> TestResult<()> {
    let store = InMemoryEventStore::new();

    let bead_ids: Vec<BeadId> = (0..5).map(|_| BeadId::new()).collect();

    for (i, bead_id) in bead_ids.iter().enumerate() {
        let spec = BeadSpec::new(format!("Bead {}", i)).with_complexity(Complexity::Simple);
        check_result(
            store.append(BeadEvent::created(*bead_id, spec)).await,
            &format!("append created for bead {}", i),
        )?;
    }

    for (i, bead_id) in bead_ids.iter().enumerate() {
        check_result(
            store
                .append(BeadEvent::state_changed(*bead_id, BeadState::Pending, BeadState::Scheduled))
                .await,
            &format!("append scheduled for bead {}", i),
        )?;
    }

    for bead_id in &bead_ids[0..3] {
        check_result(
            store
                .append(BeadEvent::state_changed(*bead_id, BeadState::Scheduled, BeadState::Ready))
                .await,
            "append ready",
        )?;
    }

    for (i, bead_id) in bead_ids.iter().enumerate() {
        check_result(
            store.append(BeadEvent::claimed(*bead_id, &format!("agent-{}", i))).await,
            &format!("append claimed for bead {}", i),
        )?;
    }

    let projection = AllBeadsProjection::new();
    let original_state = check_result(projection.rebuild(&store).await, "rebuild original")?;
    let replayed_state = check_result(projection.rebuild(&store).await, "rebuild replay")?;

    assert_eq!(
        original_state.beads.len(),
        replayed_state.beads.len(),
        "Bead count should match"
    );

    for bead_id in &bead_ids {
        let original = original_state
            .beads
            .get(bead_id)
            .ok_or_else(|| format!("Original bead {:?} not found", bead_id))?;
        let replayed = replayed_state
            .beads
            .get(bead_id)
            .ok_or_else(|| format!("Replayed bead {:?} not found", bead_id))?;

        assert_eq!(
            original.current_state, replayed.current_state,
            "State mismatch for bead {:?}",
            bead_id
        );
        assert_eq!(
            original.history.len(),
            replayed.history.len(),
            "History mismatch for bead {:?}",
            bead_id
        );
        assert_eq!(
            original.claimed_by, replayed.claimed_by,
            "Claim mismatch for bead {:?}",
            bead_id
        );
    }

    assert_eq!(
        original_state.count_in_state(BeadState::Ready),
        replayed_state.count_in_state(BeadState::Ready),
        "Ready count should match"
    );
    assert_eq!(
        original_state.count_in_state(BeadState::Scheduled),
        replayed_state.count_in_state(BeadState::Scheduled),
        "Scheduled count should match"
    );

    Ok(())
}

#[tokio::test]
async fn bdd_event_replay_restores_phase_completion() -> TestResult<()> {
    let store = InMemoryEventStore::new();
    let bead_id = BeadId::new();
    let spec = BeadSpec::new("Phase Test").with_complexity(Complexity::Complex);
    let phase_id = PhaseId::new();

    check_result(store.append(BeadEvent::created(bead_id, spec)).await, "append created")?;
    check_result(
        store
            .append(BeadEvent::state_changed(bead_id, BeadState::Pending, BeadState::Scheduled))
            .await,
        "append state change",
    )?;
    check_result(
        store
            .append(BeadEvent::phase_completed(
                bead_id,
                phase_id,
                "implement",
                PhaseOutput::success(b"implementation complete".to_vec()),
            ))
            .await,
        "append phase completed",
    )?;

    let projection = AllBeadsProjection::new();
    let original_state = check_result(projection.rebuild(&store).await, "rebuild original")?;
    let replayed_state = check_result(projection.rebuild(&store).await, "rebuild replay")?;

    let original_bead = original_state
        .beads
        .get(&bead_id)
        .ok_or_else(|| "Original bead not found".to_string())?;
    let replayed_bead = replayed_state
        .beads
        .get(&bead_id)
        .ok_or_else(|| "Replayed bead not found".to_string())?;

    assert_eq!(
        original_bead.current_phase, replayed_bead.current_phase,
        "Phase should match after replay"
    );

    Ok(())
}

#[tokio::test]
async fn bdd_event_replay_restores_dependencies() -> TestResult<()> {
    let store = InMemoryEventStore::new();
    let parent_id = BeadId::new();
    let child_id = BeadId::new();

    let parent_spec = BeadSpec::new("Parent").with_complexity(Complexity::Simple);
    check_result(
        store.append(BeadEvent::created(parent_id, parent_spec)).await,
        "append parent",
    )?;

    let child_spec = BeadSpec::new("Child")
        .with_complexity(Complexity::Simple)
        .with_dependency(parent_id);
    check_result(
        store.append(BeadEvent::created(child_id, child_spec)).await,
        "append child",
    )?;

    check_result(
        store
            .append(BeadEvent::dependency_resolved(child_id, parent_id))
            .await,
        "append dependency resolved",
    )?;

    let projection = AllBeadsProjection::new();
    let original_state = check_result(projection.rebuild(&store).await, "rebuild original")?;
    let replayed_state = check_result(projection.rebuild(&store).await, "rebuild replay")?;

    let original_child = original_state
        .beads
        .get(&child_id)
        .ok_or_else(|| "Original child not found".to_string())?;
    let replayed_child = replayed_state
        .beads
        .get(&child_id)
        .ok_or_else(|| "Replayed child not found".to_string())?;

    assert_eq!(
        original_child.dependencies, replayed_child.dependencies,
        "Dependencies should match"
    );
    assert_eq!(
        original_child.blocked_by, replayed_child.blocked_by,
        "Blocked status should match"
    );

    Ok(())
}

#[tokio::test]
async fn bdd_event_replay_is_idempotent() -> TestResult<()> {
    let store = InMemoryEventStore::new();

    for i in 0..10 {
        let bead_id = BeadId::new();
        let spec = BeadSpec::new(format!("Bead {}", i)).with_complexity(Complexity::Medium);
        check_result(
            store.append(BeadEvent::created(bead_id, spec)).await,
            "append created",
        )?;
        check_result(
            store
                .append(BeadEvent::state_changed(bead_id, BeadState::Pending, BeadState::Scheduled))
                .await,
            "append state change",
        )?;
    }

    let projection = AllBeadsProjection::new();

    let states: Vec<_> = futures::future::join_all((0..5).map(|_| projection.rebuild(&store)))
        .await
        .into_iter()
        .map(|r| check_result(r, "rebuild"))
        .collect::<TestResult<Vec<_>>>()?;

    for (i, state) in states.iter().enumerate() {
        assert_eq!(state.beads.len(), 10, "Replay {} should have 10 beads", i);
    }

    let first = &states[0];
    for (i, state) in states.iter().enumerate().skip(1) {
        for (bead_id, bead) in &first.beads {
            let other = state
                .beads
                .get(bead_id)
                .ok_or_else(|| format!("Bead {:?} missing in replay {}", bead_id, i))?;
            assert_eq!(
                bead.current_state, other.current_state,
                "State mismatch in replay {} for bead {:?}",
                i, bead_id
            );
            assert_eq!(
                bead.history.len(), other.history.len(),
                "History mismatch in replay {} for bead {:?}",
                i, bead_id
            );
        }
    }

    Ok(())
}

#[tokio::test]
async fn bdd_event_replay_preserves_event_order() -> TestResult<()> {
    let store = InMemoryEventStore::new();
    let bead_id = BeadId::new();
    let spec = BeadSpec::new("Order Test").with_complexity(Complexity::Simple);

    check_result(store.append(BeadEvent::created(bead_id, spec)).await, "append created")?;

    let transitions = [
        (BeadState::Pending, BeadState::Scheduled),
        (BeadState::Scheduled, BeadState::Ready),
        (BeadState::Ready, BeadState::InProgress),
        (BeadState::InProgress, BeadState::Review),
        (BeadState::Review, BeadState::Completed),
    ];

    for (from, to) in transitions {
        check_result(
            store.append(BeadEvent::state_changed(bead_id, from, to)).await,
            &format!("append {:?} -> {:?}", from, to),
        )?;
    }

    let projection = AllBeadsProjection::new();
    let state = check_result(projection.rebuild(&store).await, "rebuild")?;

    let bead = state
        .beads
        .get(&bead_id)
        .ok_or_else(|| "Bead not found".to_string())?;

    assert_eq!(bead.current_state, BeadState::Completed);
    assert_eq!(bead.history.len(), 5);

    let expected_transitions: Vec<_> = transitions.to_vec();
    for (i, (expected_from, expected_to)) in expected_transitions.iter().enumerate() {
        let transition = bead
            .history
            .get(i)
            .ok_or_else(|| format!("History entry {} missing", i))?;
        assert_eq!(
            transition.from, *expected_from,
            "History entry {} has wrong 'from' state",
            i
        );
        assert_eq!(
            transition.to, *expected_to,
            "History entry {} has wrong 'to' state",
            i
        );
    }

    Ok(())
}

#[tokio::test]
async fn bdd_event_replay_handles_empty_store() -> TestResult<()> {
    let store = InMemoryEventStore::new();

    let projection = AllBeadsProjection::new();
    let state = check_result(projection.rebuild(&store).await, "rebuild empty store")?;

    assert_eq!(state.beads.len(), 0, "Empty store should produce empty state");
    assert!(
        state.state_counts.is_empty(),
        "Empty store should have no state counts"
    );

    Ok(())
}

#[tokio::test]
async fn bdd_event_replay_state_counts_match() -> TestResult<()> {
    let store = InMemoryEventStore::new();

    let beads: Vec<(BeadId, BeadSpec)> = (0..20)
        .map(|i| {
            let id = BeadId::new();
            let spec = BeadSpec::new(format!("Bead {}", i)).with_complexity(Complexity::Simple);
            (id, spec)
        })
        .collect();

    for (id, spec) in &beads {
        check_result(store.append(BeadEvent::created(*id, spec.clone())).await, "append created")?;
    }

    for (i, (id, _)) in beads.iter().enumerate() {
        let target_state = match i % 4 {
            0 => BeadState::Scheduled,
            1 => BeadState::Ready,
            2 => BeadState::InProgress,
            _ => BeadState::Review,
        };
        check_result(
            store
                .append(BeadEvent::state_changed(*id, BeadState::Pending, target_state))
                .await,
            "append state change",
        )?;
    }

    let projection = AllBeadsProjection::new();
    let original = check_result(projection.rebuild(&store).await, "rebuild original")?;
    let replayed = check_result(projection.rebuild(&store).await, "rebuild replayed")?;

    for state in [
        BeadState::Pending,
        BeadState::Scheduled,
        BeadState::Ready,
        BeadState::InProgress,
        BeadState::Review,
    ] {
        assert_eq!(
            original.count_in_state(state),
            replayed.count_in_state(state),
            "Count mismatch for state {:?}",
            state
        );
    }

    Ok(())
}
