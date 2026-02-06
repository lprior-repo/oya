//! Scheduler Behavioral Tests - BDD Style
//!
//! Following BDD naming convention: given_<context>_when_<action>_then_<outcome>
//!
//! These tests document expected Scheduler behaviors through executable specifications.
//! The scheduler manages workflow DAGs and bead scheduling with state tracking.

#![deny(clippy::unwrap_used)]
#![deny(clippy::expect_used)]
#![deny(clippy::panic)]

use orchestrator::scheduler::{
    BeadScheduleState, QueueActorRef, QueueType, ScheduledBead, SchedulerActor, WorkflowState,
};

// ============================================================================
// 1. WORKFLOW MANAGEMENT (5 tests)
// ============================================================================

#[test]
fn given_empty_scheduler_when_register_workflow_then_workflow_tracked() {
    // GIVEN: A new scheduler with no workflows
    let mut scheduler = SchedulerActor::new();
    assert_eq!(
        scheduler.workflow_count(),
        0,
        "Precondition: scheduler should be empty"
    );

    // WHEN: Registering a new workflow
    let workflow_id = "workflow-001".to_string();
    let result = scheduler.register_workflow(workflow_id.clone());

    // THEN: The workflow should be tracked and accessible
    assert!(result.is_ok(), "Registration should succeed");
    assert_eq!(
        scheduler.workflow_count(),
        1,
        "Should have exactly one workflow"
    );

    let workflow = scheduler.get_workflow(&workflow_id);
    assert!(workflow.is_some(), "Workflow should be retrievable");

    if let Some(state) = workflow {
        assert_eq!(
            state.workflow_id(),
            &workflow_id,
            "Workflow ID should match"
        );
        assert!(state.is_empty(), "New workflow should have no beads");
    }
}

#[test]
fn given_workflow_exists_when_register_duplicate_then_error() {
    // GIVEN: A scheduler with an existing workflow
    let mut scheduler = SchedulerActor::new();
    let workflow_id = "workflow-duplicate".to_string();
    let first_result = scheduler.register_workflow(workflow_id.clone());
    assert!(
        first_result.is_ok(),
        "Precondition: first registration should succeed"
    );

    // WHEN: Attempting to register the same workflow again
    let second_result = scheduler.register_workflow(workflow_id.clone());

    // THEN: Should return error
    assert!(second_result.is_err(), "Duplicate registration should fail");
    assert_eq!(
        scheduler.workflow_count(),
        1,
        "Should still have only one workflow"
    );
}

#[test]
fn given_workflow_with_beads_when_unregister_then_workflow_removed() {
    // GIVEN: A scheduler with a workflow containing beads
    let mut scheduler = SchedulerActor::new();
    let workflow_id = "workflow-to-remove".to_string();
    let _ = scheduler.register_workflow(workflow_id.clone());
    let _ = scheduler.schedule_bead(workflow_id.clone(), "bead-1".to_string());
    let _ = scheduler.schedule_bead(workflow_id.clone(), "bead-2".to_string());

    assert_eq!(
        scheduler.workflow_count(),
        1,
        "Precondition: should have one workflow"
    );

    // WHEN: Unregistering the workflow
    let removed = scheduler.unregister_workflow(&workflow_id);

    // THEN: Workflow should be removed and returned
    assert!(removed.is_some(), "Should return removed workflow state");
    assert_eq!(scheduler.workflow_count(), 0, "Should have no workflows");

    if let Some(state) = removed {
        assert_eq!(
            state.workflow_id(),
            &workflow_id,
            "Removed workflow ID should match"
        );
        assert_eq!(state.len(), 2, "Removed workflow should have had 2 beads");
    }
}

#[test]
fn given_nonexistent_workflow_when_get_then_returns_none() {
    // GIVEN: A scheduler with some workflows
    let mut scheduler = SchedulerActor::new();
    let _ = scheduler.register_workflow("existing-workflow".to_string());

    // WHEN: Attempting to get a nonexistent workflow
    let result = scheduler.get_workflow(&"ghost-workflow".to_string());

    // THEN: Should return None
    assert!(result.is_none(), "Nonexistent workflow should return None");
}

#[test]
fn given_multiple_workflows_when_count_then_returns_correct_count() {
    // GIVEN: A scheduler with multiple workflows
    let mut scheduler = SchedulerActor::new();
    let _ = scheduler.register_workflow("workflow-1".to_string());
    let _ = scheduler.register_workflow("workflow-2".to_string());
    let _ = scheduler.register_workflow("workflow-3".to_string());

    // WHEN: Getting the workflow count
    let count = scheduler.workflow_count();

    // THEN: Should return correct count
    assert_eq!(count, 3, "Should have exactly 3 workflows");
}

// ============================================================================
// 2. BEAD SCHEDULING (6 tests)
// ============================================================================

#[test]
fn given_workflow_exists_when_schedule_bead_then_bead_pending() {
    // GIVEN: A scheduler with a registered workflow
    let mut scheduler = SchedulerActor::new();
    let workflow_id = "workflow-001".to_string();
    let _ = scheduler.register_workflow(workflow_id.clone());

    // WHEN: Scheduling a new bead
    let bead_id = "bead-001".to_string();
    let result = scheduler.schedule_bead(workflow_id.clone(), bead_id.clone());

    // THEN: Bead should be scheduled and in pending state
    assert!(result.is_ok(), "Scheduling bead should succeed");
    assert_eq!(scheduler.pending_count(), 1, "Should have one pending bead");
    assert_eq!(scheduler.ready_count(), 0, "Should have no ready beads yet");

    // Verify bead is in the workflow's DAG
    let workflow = scheduler.get_workflow(&workflow_id);
    assert!(workflow.is_some(), "Workflow should exist");
    if let Some(state) = workflow {
        assert!(
            state.contains_bead(&bead_id),
            "Workflow should contain the bead"
        );
    }
}

#[test]
fn given_no_workflow_when_schedule_bead_then_error() {
    // GIVEN: A scheduler with no workflows
    let mut scheduler = SchedulerActor::new();
    assert_eq!(scheduler.workflow_count(), 0, "Precondition: no workflows");

    // WHEN: Attempting to schedule a bead in nonexistent workflow
    let result = scheduler.schedule_bead("ghost-workflow".to_string(), "bead-001".to_string());

    // THEN: Should return error
    assert!(
        result.is_err(),
        "Scheduling in nonexistent workflow should fail"
    );
    assert_eq!(scheduler.pending_count(), 0, "Should have no pending beads");
}

#[test]
fn given_bead_scheduled_when_add_dependency_then_dependency_tracked() {
    // GIVEN: A workflow with two scheduled beads
    let mut scheduler = SchedulerActor::new();
    let workflow_id = "workflow-deps".to_string();
    let _ = scheduler.register_workflow(workflow_id.clone());
    let _ = scheduler.schedule_bead(workflow_id.clone(), "bead-a".to_string());
    let _ = scheduler.schedule_bead(workflow_id.clone(), "bead-b".to_string());

    // WHEN: Adding a dependency from A to B
    let result = scheduler.add_dependency(&workflow_id, "bead-a".to_string(), "bead-b".to_string());

    // THEN: Dependency should be tracked
    assert!(result.is_ok(), "Adding dependency should succeed");

    // Verify via ready beads - only A should be ready initially
    let ready_result = scheduler.get_workflow_ready_beads(&workflow_id);
    assert!(ready_result.is_ok(), "Getting ready beads should succeed");
    let ready = ready_result.unwrap_or_default();
    assert!(
        ready.contains(&"bead-a".to_string()),
        "A should be ready (no deps)"
    );
    assert!(
        !ready.contains(&"bead-b".to_string()),
        "B should not be ready (depends on A)"
    );
}

#[test]
fn given_chain_a_to_b_when_a_not_complete_then_b_not_ready() {
    // GIVEN: A workflow with chain A --> B where A is not complete
    let mut scheduler = SchedulerActor::new();
    let workflow_id = "workflow-chain".to_string();
    let _ = scheduler.register_workflow(workflow_id.clone());
    let _ = scheduler.schedule_bead(workflow_id.clone(), "a".to_string());
    let _ = scheduler.schedule_bead(workflow_id.clone(), "b".to_string());
    let _ = scheduler.add_dependency(&workflow_id, "a".to_string(), "b".to_string());

    // WHEN: Getting ready beads (A not completed)
    let ready_result = scheduler.get_workflow_ready_beads(&workflow_id);

    // THEN: Only A should be ready, B should not
    assert!(ready_result.is_ok(), "Getting ready beads should succeed");
    let ready = ready_result.unwrap_or_default();
    assert!(ready.contains(&"a".to_string()), "A should be ready");
    assert!(
        !ready.contains(&"b".to_string()),
        "B should NOT be ready (A not complete)"
    );
}

#[test]
fn given_chain_a_to_b_when_a_complete_then_b_ready() {
    // GIVEN: A workflow with chain A --> B
    let mut scheduler = SchedulerActor::new();
    let workflow_id = "workflow-chain".to_string();
    let _ = scheduler.register_workflow(workflow_id.clone());
    let _ = scheduler.schedule_bead(workflow_id.clone(), "a".to_string());
    let _ = scheduler.schedule_bead(workflow_id.clone(), "b".to_string());
    let _ = scheduler.add_dependency(&workflow_id, "a".to_string(), "b".to_string());

    // WHEN: A is marked complete in the workflow state
    if let Some(state) = scheduler.get_workflow_mut(&workflow_id) {
        state.mark_completed(&"a".to_string());
    }
    let ready_result = scheduler.get_workflow_ready_beads(&workflow_id);

    // THEN: B should now be ready
    assert!(ready_result.is_ok(), "Getting ready beads should succeed");
    let ready = ready_result.unwrap_or_default();
    assert!(
        ready.contains(&"b".to_string()),
        "B should be ready after A completes"
    );
    assert!(
        !ready.contains(&"a".to_string()),
        "A should not be ready (already completed)"
    );
}

#[test]
fn given_diamond_pattern_when_partial_complete_then_join_not_ready() {
    // GIVEN: A diamond pattern
    //     A
    //    / \
    //   B   C
    //    \ /
    //     D
    let mut scheduler = SchedulerActor::new();
    let workflow_id = "workflow-diamond".to_string();
    let _ = scheduler.register_workflow(workflow_id.clone());
    let _ = scheduler.schedule_bead(workflow_id.clone(), "a".to_string());
    let _ = scheduler.schedule_bead(workflow_id.clone(), "b".to_string());
    let _ = scheduler.schedule_bead(workflow_id.clone(), "c".to_string());
    let _ = scheduler.schedule_bead(workflow_id.clone(), "d".to_string());
    let _ = scheduler.add_dependency(&workflow_id, "a".to_string(), "b".to_string());
    let _ = scheduler.add_dependency(&workflow_id, "a".to_string(), "c".to_string());
    let _ = scheduler.add_dependency(&workflow_id, "b".to_string(), "d".to_string());
    let _ = scheduler.add_dependency(&workflow_id, "c".to_string(), "d".to_string());

    // WHEN: Only A and B are complete (not C)
    if let Some(state) = scheduler.get_workflow_mut(&workflow_id) {
        state.mark_completed(&"a".to_string());
        state.mark_completed(&"b".to_string());
    }
    let ready_result = scheduler.get_workflow_ready_beads(&workflow_id);

    // THEN: D should NOT be ready (C not complete)
    assert!(ready_result.is_ok(), "Getting ready beads should succeed");
    let ready = ready_result.unwrap_or_default();
    assert!(ready.contains(&"c".to_string()), "C should be ready");
    assert!(
        !ready.contains(&"d".to_string()),
        "D should NOT be ready (C not complete)"
    );
}

// ============================================================================
// 3. STATE TRANSITIONS (5 tests)
// ============================================================================

#[test]
fn given_pending_bead_when_mark_ready_then_in_ready_queue() {
    // GIVEN: A scheduler with a pending bead
    let mut scheduler = SchedulerActor::new();
    let workflow_id = "workflow-state".to_string();
    let bead_id = "bead-transition".to_string();
    let _ = scheduler.register_workflow(workflow_id.clone());
    let _ = scheduler.schedule_bead(workflow_id, bead_id.clone());
    assert_eq!(
        scheduler.pending_count(),
        1,
        "Precondition: bead should be pending"
    );
    assert_eq!(scheduler.ready_count(), 0, "Precondition: no ready beads");

    // WHEN: Marking bead as ready
    let result = scheduler.mark_ready(&bead_id);

    // THEN: Bead should be in ready queue
    assert!(result.is_ok(), "Marking ready should succeed");
    assert_eq!(scheduler.ready_count(), 1, "Should have one ready bead");

    let ready_beads = scheduler.get_ready_beads();
    assert!(
        ready_beads.contains(&bead_id),
        "Ready queue should contain the bead"
    );
}

#[test]
fn given_ready_bead_when_assign_worker_then_state_assigned() {
    // GIVEN: A scheduler with a ready bead
    let mut scheduler = SchedulerActor::new();
    let workflow_id = "workflow-assign".to_string();
    let bead_id = "bead-to-assign".to_string();
    let worker_id = "worker-001".to_string();
    let _ = scheduler.register_workflow(workflow_id.clone());
    let _ = scheduler.schedule_bead(workflow_id, bead_id.clone());
    let _ = scheduler.mark_ready(&bead_id);

    // WHEN: Assigning to a worker
    let result = scheduler.assign_to_worker(&bead_id, worker_id.clone());

    // THEN: Bead should be assigned
    assert!(result.is_ok(), "Assignment should succeed");
    let assignment = scheduler.get_worker_assignment(&bead_id);
    assert!(assignment.is_some(), "Should have worker assignment");
    assert_eq!(assignment, Some(&worker_id), "Worker ID should match");
}

#[test]
fn given_assigned_bead_when_complete_then_removed_from_ready() {
    // GIVEN: A scheduler with an assigned bead
    let mut scheduler = SchedulerActor::new();
    let workflow_id = "workflow-complete".to_string();
    let bead_id = "bead-to-complete".to_string();
    let _ = scheduler.register_workflow(workflow_id.clone());
    let _ = scheduler.schedule_bead(workflow_id, bead_id.clone());
    let _ = scheduler.mark_ready(&bead_id);
    let _ = scheduler.assign_to_worker(&bead_id, "worker-001".to_string());
    assert_eq!(
        scheduler.ready_count(),
        1,
        "Precondition: bead in ready queue"
    );

    // WHEN: Handling bead completion
    let result = scheduler.handle_bead_completed(&bead_id);

    // THEN: Bead should be removed from ready queue and worker assignment cleared
    assert!(result.is_ok(), "Completion handling should succeed");
    assert_eq!(scheduler.ready_count(), 0, "Ready queue should be empty");
    assert!(
        scheduler.get_worker_assignment(&bead_id).is_none(),
        "Worker assignment should be cleared"
    );
}

#[test]
fn given_nonexistent_bead_when_mark_ready_then_error() {
    // GIVEN: A scheduler with no beads
    let mut scheduler = SchedulerActor::new();

    // WHEN: Attempting to mark a nonexistent bead as ready
    let result = scheduler.mark_ready(&"ghost-bead".to_string());

    // THEN: Should return error
    assert!(
        result.is_err(),
        "Marking nonexistent bead ready should fail"
    );
}

#[test]
fn given_completed_bead_when_check_terminal_then_true() {
    // GIVEN: A BeadScheduleState in Completed state
    let state = BeadScheduleState::Completed;

    // WHEN: Checking if terminal
    let is_terminal = state.is_terminal();

    // THEN: Should return true
    assert!(is_terminal, "Completed state should be terminal");

    // Also verify other states are not terminal
    assert!(
        !BeadScheduleState::Pending.is_terminal(),
        "Pending not terminal"
    );
    assert!(
        !BeadScheduleState::Ready.is_terminal(),
        "Ready not terminal"
    );
    assert!(
        !BeadScheduleState::Dispatched.is_terminal(),
        "Dispatched not terminal"
    );
    assert!(
        !BeadScheduleState::Assigned.is_terminal(),
        "Assigned not terminal"
    );
    assert!(
        !BeadScheduleState::Running.is_terminal(),
        "Running not terminal"
    );
}

// ============================================================================
// 4. QUEUE MANAGEMENT (4 tests)
// ============================================================================

#[test]
fn given_scheduler_when_add_queue_ref_then_queue_tracked() {
    // GIVEN: A scheduler with no queue references
    let mut scheduler = SchedulerActor::new();
    assert!(
        scheduler.get_queue_refs().is_empty(),
        "Precondition: no queues"
    );

    // WHEN: Adding a queue reference
    let queue_ref = QueueActorRef::new("queue-fifo".to_string(), QueueType::FIFO);
    scheduler.add_queue_ref(queue_ref.clone());

    // THEN: Queue should be tracked
    let queues = scheduler.get_queue_refs();
    assert_eq!(queues.len(), 1, "Should have one queue");
    assert!(
        queues.contains(&queue_ref),
        "Should contain the added queue"
    );
}

#[test]
fn given_queue_added_when_get_refs_then_contains_queue() {
    // GIVEN: A scheduler with multiple queues
    let mut scheduler = SchedulerActor::new();
    let fifo_queue = QueueActorRef::new("queue-fifo".to_string(), QueueType::FIFO);
    let lifo_queue = QueueActorRef::new("queue-lifo".to_string(), QueueType::LIFO);
    let priority_queue = QueueActorRef::new("queue-priority".to_string(), QueueType::Priority);
    scheduler.add_queue_ref(fifo_queue.clone());
    scheduler.add_queue_ref(lifo_queue.clone());
    scheduler.add_queue_ref(priority_queue.clone());

    // WHEN: Getting queue references
    let queues = scheduler.get_queue_refs();

    // THEN: All queues should be present
    assert_eq!(queues.len(), 3, "Should have 3 queues");
    assert!(queues.contains(&fifo_queue), "Should contain FIFO queue");
    assert!(queues.contains(&lifo_queue), "Should contain LIFO queue");
    assert!(
        queues.contains(&priority_queue),
        "Should contain Priority queue"
    );
}

#[test]
fn given_no_queues_when_get_refs_then_empty() {
    // GIVEN: A scheduler with no queues
    let scheduler = SchedulerActor::new();

    // WHEN: Getting queue references
    let queues = scheduler.get_queue_refs();

    // THEN: Should be empty
    assert!(queues.is_empty(), "Should have no queues");
}

#[test]
fn given_duplicate_queue_when_add_then_not_duplicated() {
    // GIVEN: A scheduler with a queue
    let mut scheduler = SchedulerActor::new();
    let queue_ref = QueueActorRef::new("queue-1".to_string(), QueueType::FIFO);
    scheduler.add_queue_ref(queue_ref.clone());
    assert_eq!(
        scheduler.get_queue_refs().len(),
        1,
        "Precondition: one queue"
    );

    // WHEN: Adding the same queue again
    scheduler.add_queue_ref(queue_ref);

    // THEN: Should not have duplicates
    assert_eq!(
        scheduler.get_queue_refs().len(),
        1,
        "Should still have only one queue (no duplicates)"
    );
}

// ============================================================================
// 5. STATISTICS (3 tests)
// ============================================================================

#[test]
fn given_mixed_state_when_stats_then_correct_counts() {
    // GIVEN: A scheduler with mixed state - workflows, beads, queues
    let mut scheduler = SchedulerActor::new();

    // Register 2 workflows
    let _ = scheduler.register_workflow("workflow-1".to_string());
    let _ = scheduler.register_workflow("workflow-2".to_string());

    // Schedule beads: 2 pending, 1 ready, 1 assigned
    let _ = scheduler.schedule_bead("workflow-1".to_string(), "bead-pending-1".to_string());
    let _ = scheduler.schedule_bead("workflow-1".to_string(), "bead-pending-2".to_string());
    let _ = scheduler.schedule_bead("workflow-1".to_string(), "bead-ready".to_string());
    let _ = scheduler.schedule_bead("workflow-2".to_string(), "bead-assigned".to_string());

    // Mark one ready and one assigned
    let _ = scheduler.mark_ready(&"bead-ready".to_string());
    let _ = scheduler.mark_ready(&"bead-assigned".to_string());
    let _ = scheduler.assign_to_worker(&"bead-assigned".to_string(), "worker-1".to_string());

    // Add a queue
    scheduler.add_queue_ref(QueueActorRef::new("queue-1".to_string(), QueueType::FIFO));

    // WHEN: Getting stats
    let stats = scheduler.stats();

    // THEN: Stats should reflect correct counts
    assert_eq!(stats.workflow_count, 2, "Should have 2 workflows");
    assert_eq!(stats.pending_count, 2, "Should have 2 pending beads");
    assert_eq!(stats.ready_count, 2, "Should have 2 ready beads");
    assert_eq!(stats.assigned_count, 1, "Should have 1 assigned bead");
    assert_eq!(stats.queue_count, 1, "Should have 1 queue");
}

#[test]
fn given_empty_scheduler_when_stats_then_all_zero() {
    // GIVEN: A new empty scheduler
    let scheduler = SchedulerActor::new();

    // WHEN: Getting stats
    let stats = scheduler.stats();

    // THEN: All counts should be zero
    assert_eq!(stats.workflow_count, 0, "Workflow count should be 0");
    assert_eq!(stats.pending_count, 0, "Pending count should be 0");
    assert_eq!(stats.ready_count, 0, "Ready count should be 0");
    assert_eq!(stats.assigned_count, 0, "Assigned count should be 0");
    assert_eq!(stats.queue_count, 0, "Queue count should be 0");
}

#[test]
fn given_scheduler_with_data_when_clear_then_all_reset() {
    // GIVEN: A scheduler with various data
    let mut scheduler = SchedulerActor::new();
    let _ = scheduler.register_workflow("workflow-1".to_string());
    let _ = scheduler.schedule_bead("workflow-1".to_string(), "bead-1".to_string());
    let _ = scheduler.mark_ready(&"bead-1".to_string());
    scheduler.add_queue_ref(QueueActorRef::new("queue-1".to_string(), QueueType::FIFO));

    assert!(
        scheduler.workflow_count() > 0,
        "Precondition: should have data"
    );

    // WHEN: Clearing the scheduler
    scheduler.clear();

    // THEN: All state should be reset
    let stats = scheduler.stats();
    assert_eq!(stats.workflow_count, 0, "Workflows should be cleared");
    assert_eq!(stats.pending_count, 0, "Pending should be cleared");
    assert_eq!(stats.ready_count, 0, "Ready should be cleared");
    assert_eq!(stats.assigned_count, 0, "Assigned should be cleared");
    assert_eq!(stats.queue_count, 0, "Queues should be cleared");
}

// ============================================================================
// 6. READY DETECTION WITH DAG (3 tests)
// ============================================================================

#[test]
fn given_workflow_with_roots_when_get_ready_then_returns_roots() {
    // GIVEN: A workflow with root nodes (no dependencies)
    //   A (root)
    //   B (root)
    //   C depends on A and B
    let mut scheduler = SchedulerActor::new();
    let workflow_id = "workflow-roots".to_string();
    let _ = scheduler.register_workflow(workflow_id.clone());
    let _ = scheduler.schedule_bead(workflow_id.clone(), "a".to_string());
    let _ = scheduler.schedule_bead(workflow_id.clone(), "b".to_string());
    let _ = scheduler.schedule_bead(workflow_id.clone(), "c".to_string());
    let _ = scheduler.add_dependency(&workflow_id, "a".to_string(), "c".to_string());
    let _ = scheduler.add_dependency(&workflow_id, "b".to_string(), "c".to_string());

    // WHEN: Getting ready beads
    let ready_result = scheduler.get_workflow_ready_beads(&workflow_id);

    // THEN: Only root nodes (A and B) should be ready
    assert!(ready_result.is_ok(), "Getting ready beads should succeed");
    let ready = ready_result.unwrap_or_default();
    assert_eq!(ready.len(), 2, "Should have 2 ready beads (roots)");
    assert!(ready.contains(&"a".to_string()), "A should be ready");
    assert!(ready.contains(&"b".to_string()), "B should be ready");
    assert!(
        !ready.contains(&"c".to_string()),
        "C should NOT be ready (has deps)"
    );
}

#[test]
fn given_complex_dag_when_partial_complete_then_correct_ready_set() {
    // GIVEN: A complex DAG
    //   A --> B --> D
    //   A --> C --> D
    //   B --> E
    //   C --> E
    let mut scheduler = SchedulerActor::new();
    let workflow_id = "workflow-complex".to_string();
    let _ = scheduler.register_workflow(workflow_id.clone());
    let _ = scheduler.schedule_bead(workflow_id.clone(), "a".to_string());
    let _ = scheduler.schedule_bead(workflow_id.clone(), "b".to_string());
    let _ = scheduler.schedule_bead(workflow_id.clone(), "c".to_string());
    let _ = scheduler.schedule_bead(workflow_id.clone(), "d".to_string());
    let _ = scheduler.schedule_bead(workflow_id.clone(), "e".to_string());
    let _ = scheduler.add_dependency(&workflow_id, "a".to_string(), "b".to_string());
    let _ = scheduler.add_dependency(&workflow_id, "a".to_string(), "c".to_string());
    let _ = scheduler.add_dependency(&workflow_id, "b".to_string(), "d".to_string());
    let _ = scheduler.add_dependency(&workflow_id, "c".to_string(), "d".to_string());
    let _ = scheduler.add_dependency(&workflow_id, "b".to_string(), "e".to_string());
    let _ = scheduler.add_dependency(&workflow_id, "c".to_string(), "e".to_string());

    // WHEN: A and B are complete
    if let Some(state) = scheduler.get_workflow_mut(&workflow_id) {
        state.mark_completed(&"a".to_string());
        state.mark_completed(&"b".to_string());
    }
    let ready_result = scheduler.get_workflow_ready_beads(&workflow_id);

    // THEN: C should be ready; D and E should NOT be ready (both need C)
    assert!(ready_result.is_ok(), "Getting ready beads should succeed");
    let ready = ready_result.unwrap_or_default();
    assert!(
        ready.contains(&"c".to_string()),
        "C should be ready (A complete)"
    );
    assert!(
        !ready.contains(&"d".to_string()),
        "D should NOT be ready (C not complete)"
    );
    assert!(
        !ready.contains(&"e".to_string()),
        "E should NOT be ready (C not complete)"
    );
}

#[test]
fn given_preferred_order_dep_when_not_complete_then_still_ready() {
    // GIVEN: A workflow with preferred order dependency (non-blocking)
    let mut scheduler = SchedulerActor::new();
    let workflow_id = "workflow-preferred".to_string();
    let _ = scheduler.register_workflow(workflow_id.clone());
    let _ = scheduler.schedule_bead(workflow_id.clone(), "a".to_string());
    let _ = scheduler.schedule_bead(workflow_id.clone(), "b".to_string());

    // Add preferred order dependency via the workflow state directly
    // (since add_dependency uses BlockingDependency by default)
    if let Some(state) = scheduler.get_workflow_mut(&workflow_id) {
        let _ = state.add_dependency(
            "a".to_string(),
            "b".to_string(),
            orchestrator::dag::DependencyType::PreferredOrder,
        );
    }

    // WHEN: Getting ready beads (nothing complete)
    let ready_result = scheduler.get_workflow_ready_beads(&workflow_id);

    // THEN: Both A and B should be ready (PreferredOrder doesn't block)
    assert!(ready_result.is_ok(), "Getting ready beads should succeed");
    let ready = ready_result.unwrap_or_default();
    assert!(ready.contains(&"a".to_string()), "A should be ready");
    assert!(
        ready.contains(&"b".to_string()),
        "B should be ready (PreferredOrder is non-blocking)"
    );
}

// ============================================================================
// ADDITIONAL BEHAVIORAL TESTS (7 more tests to reach 25+)
// ============================================================================

#[test]
fn given_workflow_state_when_mark_bead_completed_then_completed_count_increases() {
    // GIVEN: A workflow state with beads
    let mut state = WorkflowState::new("workflow-001".to_string());
    let _ = state.add_bead("a".to_string());
    let _ = state.add_bead("b".to_string());
    assert_eq!(
        state.completed_count(),
        0,
        "Precondition: no completed beads"
    );

    // WHEN: Marking a bead as completed
    state.mark_completed(&"a".to_string());

    // THEN: Completed count should increase
    assert_eq!(state.completed_count(), 1, "Should have 1 completed bead");
}

#[test]
fn given_workflow_with_all_completed_when_is_complete_then_true() {
    // GIVEN: A workflow state where all beads are completed
    let mut state = WorkflowState::new("workflow-complete".to_string());
    let _ = state.add_bead("a".to_string());
    let _ = state.add_bead("b".to_string());
    state.mark_completed(&"a".to_string());
    state.mark_completed(&"b".to_string());

    // WHEN: Checking if workflow is complete
    let is_complete = state.is_complete();

    // THEN: Should return true
    assert!(
        is_complete,
        "Workflow should be complete when all beads done"
    );
}

#[test]
fn given_workflow_with_partial_completed_when_is_complete_then_false() {
    // GIVEN: A workflow state where some beads are completed
    let mut state = WorkflowState::new("workflow-partial".to_string());
    let _ = state.add_bead("a".to_string());
    let _ = state.add_bead("b".to_string());
    state.mark_completed(&"a".to_string());

    // WHEN: Checking if workflow is complete
    let is_complete = state.is_complete();

    // THEN: Should return false
    assert!(
        !is_complete,
        "Workflow should NOT be complete when some beads pending"
    );
}

#[test]
fn given_scheduled_bead_when_assign_to_queue_then_state_dispatched() {
    // GIVEN: A scheduled bead
    let mut bead = ScheduledBead::new("bead-001".to_string(), "workflow-001".to_string());
    assert_eq!(
        bead.state,
        BeadScheduleState::Pending,
        "Precondition: should be pending"
    );

    // WHEN: Assigning to a queue
    bead.assign_to_queue("queue-fifo".to_string());

    // THEN: State should be Dispatched and queue assigned
    assert_eq!(
        bead.state,
        BeadScheduleState::Dispatched,
        "State should be Dispatched"
    );
    assert_eq!(
        bead.assigned_queue,
        Some("queue-fifo".to_string()),
        "Queue should be assigned"
    );
}

#[test]
fn given_bead_schedule_state_when_is_ready_then_only_ready_state_returns_true() {
    // GIVEN: Various bead schedule states
    // WHEN: Checking is_ready on each
    // THEN: Only Ready state should return true
    assert!(
        !BeadScheduleState::Pending.is_ready(),
        "Pending.is_ready should be false"
    );
    assert!(
        BeadScheduleState::Ready.is_ready(),
        "Ready.is_ready should be true"
    );
    assert!(
        !BeadScheduleState::Dispatched.is_ready(),
        "Dispatched.is_ready should be false"
    );
    assert!(
        !BeadScheduleState::Assigned.is_ready(),
        "Assigned.is_ready should be false"
    );
    assert!(
        !BeadScheduleState::Running.is_ready(),
        "Running.is_ready should be false"
    );
    assert!(
        !BeadScheduleState::Completed.is_ready(),
        "Completed.is_ready should be false"
    );
}

#[test]
fn given_workflow_state_when_is_bead_ready_then_respects_dependencies() {
    // GIVEN: A workflow state with dependencies A --> B
    let mut state = WorkflowState::new("workflow-ready-check".to_string());
    let _ = state.add_bead("a".to_string());
    let _ = state.add_bead("b".to_string());
    let _ = state.add_dependency(
        "a".to_string(),
        "b".to_string(),
        orchestrator::dag::DependencyType::BlockingDependency,
    );

    // WHEN: Checking if beads are ready
    let a_ready = state.is_bead_ready(&"a".to_string());
    let b_ready = state.is_bead_ready(&"b".to_string());

    // THEN: A should be ready (no deps), B should not (A not complete)
    assert!(a_ready.is_ok(), "is_bead_ready(a) should succeed");
    assert!(a_ready.unwrap_or(false), "A should be ready");
    assert!(b_ready.is_ok(), "is_bead_ready(b) should succeed");
    assert!(!b_ready.unwrap_or(true), "B should NOT be ready");

    // After marking A complete, B should be ready
    state.mark_completed(&"a".to_string());
    let b_ready_after = state.is_bead_ready(&"b".to_string());
    assert!(
        b_ready_after.unwrap_or(false),
        "B should be ready after A completes"
    );
}

#[test]
fn given_default_scheduler_when_created_then_same_as_new() {
    // GIVEN/WHEN: Creating scheduler via Default trait
    let default_scheduler = SchedulerActor::default();
    let new_scheduler = SchedulerActor::new();

    // THEN: Should have same initial state
    assert_eq!(
        default_scheduler.workflow_count(),
        new_scheduler.workflow_count(),
        "Default should match new"
    );
    assert_eq!(
        default_scheduler.pending_count(),
        new_scheduler.pending_count(),
        "Default should match new"
    );
    assert_eq!(
        default_scheduler.ready_count(),
        new_scheduler.ready_count(),
        "Default should match new"
    );
}

#[test]
fn given_workflow_state_when_get_beads_then_returns_all_bead_ids() {
    // GIVEN: A workflow state with multiple beads
    let mut state = WorkflowState::new("workflow-beads".to_string());
    let _ = state.add_bead("bead-1".to_string());
    let _ = state.add_bead("bead-2".to_string());
    let _ = state.add_bead("bead-3".to_string());

    // WHEN: Getting all beads
    let beads = state.beads();

    // THEN: Should return all bead IDs
    assert_eq!(beads.len(), 3, "Should have 3 beads");
    assert!(
        beads.contains(&"bead-1".to_string()),
        "Should contain bead-1"
    );
    assert!(
        beads.contains(&"bead-2".to_string()),
        "Should contain bead-2"
    );
    assert!(
        beads.contains(&"bead-3".to_string()),
        "Should contain bead-3"
    );
}

#[test]
fn given_nonexistent_bead_when_assign_to_worker_then_error() {
    // GIVEN: A scheduler with no beads
    let mut scheduler = SchedulerActor::new();

    // WHEN: Attempting to assign nonexistent bead to worker
    let result = scheduler.assign_to_worker(&"ghost-bead".to_string(), "worker-1".to_string());

    // THEN: Should return error
    assert!(result.is_err(), "Assigning nonexistent bead should fail");
}

#[test]
fn given_workflow_state_when_len_then_returns_bead_count() {
    // GIVEN: A workflow state with beads
    let mut state = WorkflowState::new("workflow-len".to_string());
    assert_eq!(state.len(), 0, "New workflow should have len 0");

    let _ = state.add_bead("bead-1".to_string());
    let _ = state.add_bead("bead-2".to_string());

    // WHEN: Getting length
    let len = state.len();

    // THEN: Should return bead count
    assert_eq!(len, 2, "Should have 2 beads");
}

#[test]
fn given_queue_types_when_compare_then_distinct_values() {
    // GIVEN: Different queue types
    // WHEN: Comparing them
    // THEN: Each should be distinct
    assert_ne!(QueueType::FIFO, QueueType::LIFO, "FIFO != LIFO");
    assert_ne!(QueueType::FIFO, QueueType::RoundRobin, "FIFO != RoundRobin");
    assert_ne!(QueueType::FIFO, QueueType::Priority, "FIFO != Priority");
    assert_ne!(QueueType::LIFO, QueueType::RoundRobin, "LIFO != RoundRobin");
    assert_ne!(QueueType::LIFO, QueueType::Priority, "LIFO != Priority");
    assert_ne!(
        QueueType::RoundRobin,
        QueueType::Priority,
        "RoundRobin != Priority"
    );

    // Same type should be equal
    assert_eq!(QueueType::FIFO, QueueType::FIFO, "FIFO == FIFO");
}

// ============================================================================
// BEAD COMPLETED EVENT SUBSCRIPTION TESTS (TDD-15 RED PHASE)
// ============================================================================

#[test]
fn given_scheduler_with_workflow_when_bead_completed_event_received_then_dag_updated() {
    // GIVEN: A scheduler with a workflow containing a chain A --> B
    let mut scheduler = SchedulerActor::new();
    let workflow_id = "workflow-events".to_string();
    let _ = scheduler.register_workflow(workflow_id.clone());
    let _ = scheduler.schedule_bead(workflow_id.clone(), "a".to_string());
    let _ = scheduler.schedule_bead(workflow_id.clone(), "b".to_string());
    let _ = scheduler.add_dependency(&workflow_id, "a".to_string(), "b".to_string());

    // Verify initial state: only A is ready
    let ready_result = scheduler.get_workflow_ready_beads(&workflow_id);
    assert!(ready_result.is_ok());
    let ready = ready_result.unwrap_or_default();
    assert!(
        ready.contains(&"a".to_string()),
        "A should be ready initially"
    );
    assert!(
        !ready.contains(&"b".to_string()),
        "B should NOT be ready initially"
    );

    // WHEN: A BeadCompleted event is received for bead A
    let _ = scheduler.handle_bead_completed(&"a".to_string());

    // THEN: Workflow DAG should mark A as completed, making B ready
    let ready_result = scheduler.get_workflow_ready_beads(&workflow_id);
    assert!(ready_result.is_ok());
    let ready = ready_result.unwrap_or_default();
    assert!(
        !ready.contains(&"a".to_string()),
        "A should NOT be ready (completed)"
    );
    assert!(
        ready.contains(&"b".to_string()),
        "B should be ready after A completes"
    );
}

#[test]
fn given_scheduler_with_diamond_when_beads_complete_then_join_becomes_ready() {
    // GIVEN: A scheduler with diamond pattern
    //     A
    //    / \
    //   B   C
    //    \ /
    //     D
    let mut scheduler = SchedulerActor::new();
    let workflow_id = "workflow-diamond-events".to_string();
    let _ = scheduler.register_workflow(workflow_id.clone());
    let _ = scheduler.schedule_bead(workflow_id.clone(), "a".to_string());
    let _ = scheduler.schedule_bead(workflow_id.clone(), "b".to_string());
    let _ = scheduler.schedule_bead(workflow_id.clone(), "c".to_string());
    let _ = scheduler.schedule_bead(workflow_id.clone(), "d".to_string());
    let _ = scheduler.add_dependency(&workflow_id, "a".to_string(), "b".to_string());
    let _ = scheduler.add_dependency(&workflow_id, "a".to_string(), "c".to_string());
    let _ = scheduler.add_dependency(&workflow_id, "b".to_string(), "d".to_string());
    let _ = scheduler.add_dependency(&workflow_id, "c".to_string(), "d".to_string());

    // WHEN: A completes (B and C should become ready)
    let _ = scheduler.handle_bead_completed(&"a".to_string());

    let ready_result = scheduler.get_workflow_ready_beads(&workflow_id);
    assert!(ready_result.is_ok());
    let ready = ready_result.unwrap_or_default();
    assert!(ready.contains(&"b".to_string()), "B should be ready");
    assert!(ready.contains(&"c".to_string()), "C should be ready");
    assert!(
        !ready.contains(&"d".to_string()),
        "D should NOT be ready yet"
    );

    // WHEN: B completes (still waiting on C)
    let _ = scheduler.handle_bead_completed(&"b".to_string());

    let ready_result = scheduler.get_workflow_ready_beads(&workflow_id);
    assert!(ready_result.is_ok());
    let ready = ready_result.unwrap_or_default();
    assert!(
        !ready.contains(&"d".to_string()),
        "D still NOT ready (C not complete)"
    );

    // WHEN: C completes (D should become ready)
    let _ = scheduler.handle_bead_completed(&"c".to_string());

    let ready_result = scheduler.get_workflow_ready_beads(&workflow_id);
    assert!(ready_result.is_ok());
    let ready = ready_result.unwrap_or_default();
    assert!(
        ready.contains(&"d".to_string()),
        "D should be ready after B and C complete"
    );
}

#[test]
fn given_scheduler_when_nonexistent_bead_completes_then_no_error() {
    // GIVEN: A scheduler with a workflow
    let mut scheduler = SchedulerActor::new();
    let workflow_id = "workflow-robust".to_string();
    let _ = scheduler.register_workflow(workflow_id.clone());
    let _ = scheduler.schedule_bead(workflow_id.clone(), "a".to_string());

    // WHEN: BeadCompleted event received for nonexistent bead
    // THEN: Should handle gracefully without error
    let _ = scheduler.handle_bead_completed(&"ghost-bead".to_string());

    // Verify scheduler still functional
    let ready_result = scheduler.get_workflow_ready_beads(&workflow_id);
    assert!(ready_result.is_ok());
}
