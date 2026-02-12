//! BDD Integration Test: Persistence bead state update.
//!
//! This test validates the bead state persistence lifecycle:
//! - GIVEN a persistence backend (OrchestratorStore) with a bead
//! - WHEN the bead state is updated
//! - THEN the updated state is persisted and retrievable
//!
//! # Quality Standards
//!
//! - **Zero unwraps**: All errors use `Result` types with `?` operator
//! - **Zero panics**: No `panic!`, `unwrap()`, or `expect()` calls
//! - **BDD Style**: GIVEN-WHEN-THEN structure
//! - **Railway-Oriented Programming**: Proper error propagation throughout

#![deny(clippy::unwrap_used)]
#![deny(clippy::expect_used)]
#![deny(clippy::panic)]

use orchestrator::persistence::{BeadRecord, BeadState, OrchestratorStore, StoreConfig};

type TestResult = Result<(), String>;

async fn setup_store() -> Result<OrchestratorStore, String> {
    let config = StoreConfig::in_memory();
    let store = OrchestratorStore::connect(config)
        .await
        .map_err(|e| format!("Failed to connect to store: {}", e))?;
    store
        .initialize_schema()
        .await
        .map_err(|e| format!("Failed to initialize schema: {}", e))?;
    Ok(store)
}

/// BDD Test: GIVEN persistence WHEN bead state updated THEN state persisted.
///
/// # Test Structure
///
/// - **GIVEN**: An OrchestratorStore with a saved bead in Pending state
/// - **WHEN**: The bead state is updated to Running
/// - **THEN**: The updated state is persisted and retrievable
#[tokio::test]
async fn given_persistence_when_bead_state_updated_then_state_persisted() -> TestResult {
    let store = setup_store().await?;

    let bead = BeadRecord::new("bead-001", "workflow-001");
    store
        .save_bead(&bead)
        .await
        .map_err(|e| format!("Failed to save bead: {}", e))?;

    let updated = store
        .update_bead_state("bead-001", BeadState::Running)
        .await
        .map_err(|e| format!("Failed to update bead state: {}", e))?;

    assert_eq!(
        updated.state,
        BeadState::Running,
        "Updated bead should have Running state"
    );
    assert!(
        updated.started_at.is_some(),
        "Running bead should have started_at timestamp"
    );

    let retrieved = store
        .get_bead("bead-001")
        .await
        .map_err(|e| format!("Failed to retrieve bead: {}", e))?;

    assert_eq!(
        retrieved.state, BeadState::Running,
        "Retrieved bead should have persisted Running state"
    );
    assert_eq!(
        retrieved.started_at, updated.started_at,
        "started_at timestamp should be persisted"
    );

    Ok(())
}

/// BDD Test: GIVEN persistence WHEN bead state transitions to completed THEN timestamps persisted.
///
/// # Test Structure
///
/// - **GIVEN**: An OrchestratorStore with a Running bead
/// - **WHEN**: The bead state is updated to Completed
/// - **THEN**: The completed_at timestamp is set and persisted
#[tokio::test]
async fn given_persistence_when_bead_completed_then_timestamps_persisted() -> TestResult {
    let store = setup_store().await?;

    let bead = BeadRecord::new("bead-complete", "workflow-complete");
    store
        .save_bead(&bead)
        .await
        .map_err(|e| format!("Failed to save bead: {}", e))?;

    store
        .update_bead_state("bead-complete", BeadState::Running)
        .await
        .map_err(|e| format!("Failed to transition to Running: {}", e))?;

    let completed = store
        .update_bead_state("bead-complete", BeadState::Completed)
        .await
        .map_err(|e| format!("Failed to transition to Completed: {}", e))?;

    assert_eq!(
        completed.state,
        BeadState::Completed,
        "Bead should be in Completed state"
    );
    assert!(
        completed.completed_at.is_some(),
        "Completed bead should have completed_at timestamp"
    );

    let retrieved = store
        .get_bead("bead-complete")
        .await
        .map_err(|e| format!("Failed to retrieve bead: {}", e))?;

    assert_eq!(
        retrieved.completed_at, completed.completed_at,
        "completed_at timestamp should be persisted"
    );

    Ok(())
}

/// BDD Test: GIVEN persistence WHEN multiple bead state transitions THEN final state persisted.
///
/// # Test Structure
///
/// - **GIVEN**: An OrchestratorStore with a bead
/// - **WHEN**: The bead transitions through multiple states
/// - **THEN**: Only the final state is persisted
#[tokio::test]
async fn given_persistence_when_multiple_state_transitions_then_final_state_persisted(
) -> TestResult {
    let store = setup_store().await?;

    let bead = BeadRecord::new("bead-transitions", "workflow-transitions");
    store
        .save_bead(&bead)
        .await
        .map_err(|e| format!("Failed to save bead: {}", e))?;

    let states_to_test = vec![
        BeadState::Ready,
        BeadState::Dispatched,
        BeadState::Assigned,
        BeadState::Running,
    ];

    for state in states_to_test {
        store
            .update_bead_state("bead-transitions", state)
            .await
            .map_err(|e| format!("Failed to transition to {:?}: {}", state, e))?;

        let current = store
            .get_bead("bead-transitions")
            .await
            .map_err(|e| format!("Failed to retrieve bead: {}", e))?;

        assert_eq!(
            current.state, state,
            "Bead should be in {:?} state after transition",
            state
        );
    }

    let final_state = store
        .get_bead("bead-transitions")
        .await
        .map_err(|e| format!("Failed to retrieve final bead state: {}", e))?;

    assert_eq!(
        final_state.state,
        BeadState::Running,
        "Final persisted state should be Running"
    );

    Ok(())
}

/// BDD Test: GIVEN persistence WHEN bead state is terminal THEN is_terminal returns true.
///
/// # Test Structure
///
/// - **GIVEN**: Bead states that are terminal (Completed, Failed, Cancelled)
/// - **WHEN**: is_terminal is called
/// - **THEN**: It returns true for terminal states and false for others
#[tokio::test]
async fn given_persistence_when_bead_terminal_state_then_is_terminal_true() -> TestResult {
    let store = setup_store().await?;

    let terminal_states = vec![
        (BeadState::Completed, "bead-terminal-completed"),
        (BeadState::Failed, "bead-terminal-failed"),
        (BeadState::Cancelled, "bead-terminal-cancelled"),
    ];

    for (state, bead_id) in &terminal_states {
        assert!(
            state.is_terminal(),
            "{:?} should be a terminal state",
            state
        );

        let bead = BeadRecord::new(*bead_id, "workflow-terminal");
        store
            .save_bead(&bead)
            .await
            .map_err(|e| format!("Failed to save bead {}: {}", bead_id, e))?;

        store
            .update_bead_state(bead_id, *state)
            .await
            .map_err(|e| format!("Failed to update bead {} to {:?}: {}", bead_id, state, e))?;

        let retrieved = store
            .get_bead(bead_id)
            .await
            .map_err(|e| format!("Failed to retrieve bead {}: {}", bead_id, e))?;

        assert_eq!(
            retrieved.state, *state,
            "Terminal state {:?} should be persisted for bead {}",
            state, bead_id
        );
    }

    let non_terminal_states = vec![
        BeadState::Pending,
        BeadState::Ready,
        BeadState::Dispatched,
        BeadState::Assigned,
        BeadState::Running,
    ];

    for state in non_terminal_states {
        assert!(
            !state.is_terminal(),
            "{:?} should NOT be a terminal state",
            state
        );
    }

    Ok(())
}

/// BDD Test: GIVEN persistence WHEN bead failed THEN error message persisted.
///
/// # Test Structure
///
/// - **GIVEN**: An OrchestratorStore with a bead
/// - **WHEN**: The bead is marked as failed with an error message
/// - **THEN**: The error message and failed state are persisted
#[tokio::test]
async fn given_persistence_when_bead_failed_then_error_message_persisted() -> TestResult {
    let store = setup_store().await?;

    let bead = BeadRecord::new("bead-failed", "workflow-failed");
    store
        .save_bead(&bead)
        .await
        .map_err(|e| format!("Failed to save bead: {}", e))?;

    let failed = store
        .mark_bead_failed("bead-failed", "Task exceeded timeout")
        .await
        .map_err(|e| format!("Failed to mark bead as failed: {}", e))?;

    assert_eq!(
        failed.state,
        BeadState::Failed,
        "Bead should be in Failed state"
    );
    assert_eq!(
        failed.error_message,
        Some("Task exceeded timeout".to_string()),
        "Error message should be persisted"
    );
    assert!(
        failed.completed_at.is_some(),
        "Failed bead should have completed_at timestamp"
    );

    let retrieved = store
        .get_bead("bead-failed")
        .await
        .map_err(|e| format!("Failed to retrieve bead: {}", e))?;

    assert_eq!(
        retrieved.error_message,
        Some("Task exceeded timeout".to_string()),
        "Error message should be persisted"
    );

    Ok(())
}

/// BDD Test: GIVEN persistence WHEN bead assigned to worker THEN assignment persisted.
///
/// # Test Structure
///
/// - **GIVEN**: An OrchestratorStore with a bead
/// - **WHEN**: The bead is assigned to a worker
/// - **THEN**: The worker assignment and Assigned state are persisted
#[tokio::test]
async fn given_persistence_when_bead_assigned_to_worker_then_assignment_persisted() -> TestResult {
    let store = setup_store().await?;

    let bead = BeadRecord::new("bead-assign", "workflow-assign");
    store
        .save_bead(&bead)
        .await
        .map_err(|e| format!("Failed to save bead: {}", e))?;

    let assigned = store
        .assign_bead_to_worker("bead-assign", "worker-001")
        .await
        .map_err(|e| format!("Failed to assign bead to worker: {}", e))?;

    assert_eq!(
        assigned.state,
        BeadState::Assigned,
        "Bead should be in Assigned state"
    );
    assert_eq!(
        assigned.assigned_worker,
        Some("worker-001".to_string()),
        "Worker assignment should be persisted"
    );

    let retrieved = store
        .get_bead("bead-assign")
        .await
        .map_err(|e| format!("Failed to retrieve bead: {}", e))?;

    assert_eq!(
        retrieved.assigned_worker,
        Some("worker-001".to_string()),
        "Worker assignment should be persisted"
    );

    Ok(())
}
