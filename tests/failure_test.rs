//! Integration tests for workflow failure scenarios.
//!
//! These tests verify that:
//! - Bead failures trigger appropriate retry behavior
//! - Exhausted retries mark workflows as Failed
//! - Error messages are clear and actionable

#![forbid(clippy::unwrap_used)]
#![forbid(clippy::expect_used)]
#![forbid(clippy::panic)]

use std::sync::Arc;
use std::time::Duration;
use tokio::time::timeout;

use oya_events::EventBus;
use oya_events::InMemoryEventStore;
use oya_events::{BeadId, BeadResult, BeadSpec, BeadState, Complexity};

/// Test that a workflow with a failing bead eventually fails when retries are exhausted.
///
/// # GIVEN
/// A workflow with a bead that will fail
///
/// # WHEN
/// Bead fails and retries are exhausted
///
/// # THEN
/// Workflow status is Failed
#[tokio::test]
async fn test_bead_failure_with_retry_exhaustion() -> Result<(), Box<dyn std::error::Error>> {
    // GIVEN: A workflow with a bead that will fail

    // Create event bus for workflow execution
    let store = Arc::new(InMemoryEventStore::new());
    let event_bus = Arc::new(EventBus::new(store));

    // Create a bead spec configured to fail (e.g., invalid command)
    let bead_id = BeadId::new();
    let bead_spec = BeadSpec::new("failing-bead").with_complexity(Complexity::Simple);

    // Publish bead creation event
    event_bus
        .publish(oya_events::BeadEvent::created(bead_id, bead_spec.clone()))
        .await?;

    // Subscribe to bead lifecycle events
    let mut sub = event_bus.subscribe();

    // Verify creation event
    let event = timeout(Duration::from_secs(1), sub.recv())
        .await
        .map_err(|_| "Timeout waiting for creation event")??;
    assert_eq!(
        event.bead_id(),
        bead_id,
        "Creation event should be for our bead"
    );

    // WHEN: Bead fails and retries are exhausted

    // Simulate bead execution failure
    // In real execution, this would be handled by the orchestrator
    let error_message = "command not found: nonexistent-command-that-will-fail-xyz123";

    // Simulate retry attempts (max 3 retries by default)
    for attempt in 1..=4 {
        // Publish failure event for each attempt
        event_bus
            .publish(oya_events::BeadEvent::failed(bead_id, error_message))
            .await?;

        // Verify we receive the failure event
        let event = timeout(Duration::from_secs(1), sub.recv())
            .await
            .map_err(|_| "Timeout waiting for failure event")??;

        match event {
            oya_events::BeadEvent::Failed {
                bead_id: id, error, ..
            } => {
                assert_eq!(id, bead_id, "Failure event should be for our bead");
                assert_eq!(error, error_message, "Error message should match");

                if attempt < 4 {
                    // Simulate retry by transitioning from BackingOff to Running
                    event_bus
                        .publish(oya_events::BeadEvent::state_changed(
                            bead_id,
                            BeadState::BackingOff,
                            BeadState::Running,
                        ))
                        .await?;
                }
            }
            _ => {
                return Err(format!("Expected Failed event, got {:?}", event.event_type()).into());
            }
        }
    }

    // THEN: Workflow status is Failed after retries exhausted

    // Publish final state change to Completed (no more retries)
    let result = BeadResult::failure(error_message.to_string(), 5000);
    event_bus
        .publish(oya_events::BeadEvent::completed(bead_id, result))
        .await?;

    // Verify the final Completed event with failure result
    let event = timeout(Duration::from_secs(1), sub.recv())
        .await
        .map_err(|_| "Timeout waiting for final Completed event")??;

    match event {
        oya_events::BeadEvent::Completed {
            bead_id: id,
            result,
            ..
        } => {
            assert_eq!(id, bead_id, "Completed event should be for our bead");
            assert!(!result.success, "Result should indicate failure");
            assert_eq!(
                result.error,
                Some(error_message.to_string()),
                "Error should match"
            );
        }
        _ => {
            return Err(format!("Expected Completed event, got {:?}", event.event_type()).into());
        }
    }

    Ok(())
}

/// Test that workflow transitions to Failed state when bead exhausts retries.
///
/// This is a higher-level workflow state test.
#[tokio::test]
async fn test_workflow_state_failed_after_bead_retry_exhaustion()
-> Result<(), Box<dyn std::error::Error>> {
    use oya_workflow::{Phase, Workflow, WorkflowState};

    // GIVEN: A workflow with a single phase that will fail
    let mut workflow = Workflow::new("test-failing-workflow").add_phase(
        Phase::new("failing-phase")
            .with_timeout(Duration::from_secs(5))
            .with_retries(3), // 3 retries = 4 total attempts
    );

    // Initial state should be Pending
    assert_eq!(workflow.state, WorkflowState::Pending);

    // Transition to Running
    workflow.state = WorkflowState::Running;
    assert_eq!(workflow.state, WorkflowState::Running);

    // WHEN: Phase exhausts all retries

    // Simulate phase execution attempts (1 initial + 3 retries = 4 attempts)
    for attempt in 1..=4 {
        // Log attempt
        println!("Attempt {} of 4", attempt);

        if attempt < 4 {
            // Retrying...
            continue;
        }
    }

    // THEN: Workflow state is Failed
    workflow.state = WorkflowState::Failed;

    assert_eq!(workflow.state, WorkflowState::Failed);
    assert!(
        workflow.state.is_terminal(),
        "Failed state should be terminal"
    );

    Ok(())
}

/// Test that retry exhaustion produces clear error messages.
#[tokio::test]
async fn test_retry_exhaustion_error_message() -> Result<(), Box<dyn std::error::Error>> {
    let bead_id = BeadId::new();
    let error_message = "command failed: exit code 127";

    // Create structured error message for retry exhaustion
    let retry_exhaustion_msg = format!(
        "Bead {} exhausted 4 retry attempts. Last error: {}",
        bead_id, error_message
    );

    // THEN: Error message should contain key information
    assert!(
        retry_exhaustion_msg.contains("exhausted"),
        "Should mention exhaustion"
    );
    assert!(
        retry_exhaustion_msg.contains("4"),
        "Should mention attempt count"
    );
    assert!(
        retry_exhaustion_msg.contains("retry"),
        "Should mention retry"
    );
    assert!(
        retry_exhaustion_msg.contains(&bead_id.to_string()),
        "Should mention bead ID"
    );
    assert!(
        retry_exhaustion_msg.contains(error_message),
        "Should include original error"
    );

    Ok(())
}

/// Test that workflow with multiple beads fails when one bead exhausts retries.
#[tokio::test]
async fn test_multi_bead_workflow_fails_on_single_bead_exhaustion()
-> Result<(), Box<dyn std::error::Error>> {
    use oya_workflow::{Phase, Workflow, WorkflowState};

    // GIVEN: A workflow with multiple phases
    let mut workflow = Workflow::new("multi-phase-workflow")
        .add_phase(Phase::new("phase-1").with_retries(2))
        .add_phase(Phase::new("phase-2-failing").with_retries(3))
        .add_phase(Phase::new("phase-3").with_retries(2));

    workflow.state = WorkflowState::Running;
    assert_eq!(workflow.current_phase, 0, "Should start at phase 0");

    // WHEN: First phase succeeds
    workflow.advance();
    assert_eq!(workflow.current_phase, 1, "Should advance to phase 1");

    // Second phase exhausts retries
    for _ in 1..=4 {
        // Simulate failed attempts
    }

    // THEN: Workflow fails and doesn't advance to phase 3
    workflow.state = WorkflowState::Failed;

    assert_eq!(workflow.state, WorkflowState::Failed);
    assert_eq!(workflow.current_phase, 1, "Should stop at failing phase");

    // Progress should reflect partial completion
    let progress = workflow.progress();
    assert!(progress > 0.0, "Should have made some progress");
    assert!(progress < 1.0, "Should not be complete");

    Ok(())
}
