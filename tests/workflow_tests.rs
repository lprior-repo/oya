//! BDD-style integration tests for workflow registration and tracking.
//!
//! These tests verify the Given-When-Then behavior:
//! GIVEN empty scheduler
//! WHEN register workflow
//! THEN tracked correctly

#![deny(clippy::unwrap_used)]
#![deny(clippy::expect_used)]
#![deny(clippy::panic)]

use orchestrator::scheduler::{SchedulerActor, WorkflowId};

/// GIVEN a scheduler with no workflows
/// WHEN RegisterWorkflow is called with a new workflow ID
/// THEN the workflow is tracked and retrievable
#[test]
fn given_empty_scheduler_when_register_workflow_then_tracked_correctly() {
    // GIVEN: A new scheduler with no workflows
    let mut scheduler = SchedulerActor::new();
    let workflow_id: WorkflowId = "workflow-test-001".to_string();

    // Verify initial state
    assert_eq!(
        scheduler.workflow_count(),
        0,
        "scheduler should start with no workflows"
    );

    // WHEN: Registering a new workflow
    let result = scheduler.register_workflow(workflow_id.clone());

    // THEN: The registration should succeed
    assert!(result.is_ok(), "workflow registration should succeed");

    // AND: The workflow count should be 1
    assert_eq!(
        scheduler.workflow_count(),
        1,
        "scheduler should track exactly one workflow"
    );

    // AND: The workflow should be retrievable
    let workflow = scheduler.get_workflow(&workflow_id);
    assert!(
        workflow.is_some(),
        "registered workflow should be retrievable"
    );

    // AND: The workflow should have the correct ID
    if let Some(state) = workflow {
        assert_eq!(
            state.workflow_id(),
            &workflow_id,
            "workflow should have correct ID"
        );
        assert!(
            state.is_empty(),
            "new workflow should have no beads initially"
        );
    }
}

/// GIVEN a scheduler with no workflows
/// WHEN RegisterWorkflow is called with multiple workflow IDs
/// THEN all workflows are tracked independently
#[test]
fn given_empty_scheduler_when_register_multiple_workflows_then_all_tracked() {
    // GIVEN: A new scheduler with no workflows
    let mut scheduler = SchedulerActor::new();

    let workflow_id_1: WorkflowId = "workflow-alpha".to_string();
    let workflow_id_2: WorkflowId = "workflow-beta".to_string();
    let workflow_id_3: WorkflowId = "workflow-gamma".to_string();

    // WHEN: Registering multiple workflows
    let result_1 = scheduler.register_workflow(workflow_id_1.clone());
    let result_2 = scheduler.register_workflow(workflow_id_2.clone());
    let result_3 = scheduler.register_workflow(workflow_id_3.clone());

    // THEN: All registrations should succeed
    assert!(
        result_1.is_ok(),
        "first workflow registration should succeed"
    );
    assert!(
        result_2.is_ok(),
        "second workflow registration should succeed"
    );
    assert!(
        result_3.is_ok(),
        "third workflow registration should succeed"
    );

    // AND: All workflows should be tracked
    assert_eq!(
        scheduler.workflow_count(),
        3,
        "scheduler should track all three workflows"
    );

    // AND: Each workflow should be independently retrievable
    assert!(
        scheduler.get_workflow(&workflow_id_1).is_some(),
        "workflow-alpha should be retrievable"
    );
    assert!(
        scheduler.get_workflow(&workflow_id_2).is_some(),
        "workflow-beta should be retrievable"
    );
    assert!(
        scheduler.get_workflow(&workflow_id_3).is_some(),
        "workflow-gamma should be retrievable"
    );
}

/// GIVEN a scheduler with an existing workflow
/// WHEN RegisterWorkflow is called with the same workflow ID
/// THEN the registration fails and the original workflow is preserved
#[test]
fn given_existing_workflow_when_register_duplicate_then_fails() {
    // GIVEN: A scheduler with an existing workflow
    let mut scheduler = SchedulerActor::new();
    let workflow_id: WorkflowId = "workflow-unique".to_string();

    let first_result = scheduler.register_workflow(workflow_id.clone());
    assert!(first_result.is_ok(), "first registration should succeed");
    assert_eq!(scheduler.workflow_count(), 1, "should have one workflow");

    // WHEN: Attempting to register the same workflow ID again
    let second_result = scheduler.register_workflow(workflow_id.clone());

    // THEN: The duplicate registration should fail
    assert!(
        second_result.is_err(),
        "duplicate workflow registration should fail"
    );

    // AND: The workflow count should remain unchanged
    assert_eq!(
        scheduler.workflow_count(),
        1,
        "workflow count should remain at 1"
    );

    // AND: The original workflow should still be accessible
    let workflow = scheduler.get_workflow(&workflow_id);
    assert!(
        workflow.is_some(),
        "original workflow should still be retrievable"
    );
}

/// GIVEN a scheduler with multiple workflows
/// WHEN one workflow is unregistered
/// THEN only that workflow is removed and others remain
#[test]
fn given_multiple_workflows_when_unregister_one_then_others_remain() {
    // GIVEN: A scheduler with multiple workflows
    let mut scheduler = SchedulerActor::new();
    let workflow_id_1: WorkflowId = "workflow-one".to_string();
    let workflow_id_2: WorkflowId = "workflow-two".to_string();

    scheduler.register_workflow(workflow_id_1.clone()).ok();
    scheduler.register_workflow(workflow_id_2.clone()).ok();

    assert_eq!(scheduler.workflow_count(), 2, "should have two workflows");

    // WHEN: Unregistering one workflow
    let removed = scheduler.unregister_workflow(&workflow_id_1);

    // THEN: The removed workflow should be returned
    assert!(
        removed.is_some(),
        "unregister should return the removed workflow"
    );

    // AND: The workflow count should decrease
    assert_eq!(
        scheduler.workflow_count(),
        1,
        "should have one workflow remaining"
    );

    // AND: The removed workflow should no longer be retrievable
    assert!(
        scheduler.get_workflow(&workflow_id_1).is_none(),
        "unregistered workflow should not be retrievable"
    );

    // AND: Other workflows should remain intact
    assert!(
        scheduler.get_workflow(&workflow_id_2).is_some(),
        "other workflow should still be retrievable"
    );
}

/// GIVEN a scheduler with a registered workflow
/// WHEN scheduler statistics are queried
/// THEN statistics accurately reflect the workflow state
#[test]
fn given_registered_workflow_when_query_stats_then_accurate() {
    // GIVEN: A scheduler with a registered workflow
    let mut scheduler = SchedulerActor::new();
    let workflow_id: WorkflowId = "workflow-stats".to_string();

    scheduler.register_workflow(workflow_id).ok();

    // WHEN: Querying scheduler statistics
    let stats = scheduler.stats();

    // THEN: Statistics should reflect the workflow
    assert_eq!(stats.workflow_count, 1, "stats should report one workflow");
    assert_eq!(
        stats.pending_count, 0,
        "stats should report zero pending beads"
    );
    assert_eq!(stats.ready_count, 0, "stats should report zero ready beads");
}
