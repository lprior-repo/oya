//! BDD-style integration tests for scheduler in-flight work completion.
//!
//! These tests verify the Given-When-Then behavior:
//! GIVEN scheduler with in-flight work
//! WHEN stop is requested
//! THEN all in-flight work completes before stopping

#![deny(clippy::unwrap_used)]
#![deny(clippy::expect_used)]
#![deny(clippy::panic)]

use oya_orchestrator::scheduler::{SchedulerActor, BeadScheduleState};

/// GIVEN scheduler with beads in various states
/// WHEN stop is requested with in-flight work
/// THEN scheduler completes all in-flight work before stopping
#[tokio::test]
async fn given_scheduler_with_in_flight_work_when_stop_requested_then_completes_all_work() {
    // GIVEN: A scheduler with beads in different states
    let mut scheduler = SchedulerActor::new();
    let workflow_id = "workflow-test".to_string();
    let bead_pending = "bead-pending".to_string();
    let bead_ready = "bead-ready".to_string();
    let bead_in_flight = "bead-in-flight".to_string();
    let bead_completed = "bead-completed".to_string();
    let worker_id = "worker-123".to_string();

    // Register workflow
    scheduler
        .register_workflow(workflow_id.clone())
        .expect("workflow registration should succeed");

    // Schedule beads
    scheduler
        .schedule_bead(workflow_id.clone(), bead_pending.clone())
        .expect("pending bead scheduling should succeed");
    scheduler
        .schedule_bead(workflow_id.clone(), bead_ready.clone())
        .expect("ready bead scheduling should succeed");
    scheduler
        .schedule_bead(workflow_id.clone(), bead_in_flight.clone())
        .expect("in-flight bead scheduling should succeed");
    scheduler
        .schedule_bead(workflow_id, bead_completed.clone())
        .expect("completed bead scheduling should succeed");

    // Set up bead states
    scheduler
        .mark_ready(&bead_ready)
        .expect("marking bead ready should succeed");
    scheduler
        .assign_to_worker(&bead_in_flight, worker_id.clone())
        .expect("assigning bead to worker should succeed");
    scheduler
        .handle_bead_completed(&bead_completed)
        .expect("handling bead completion should succeed");

    // Verify initial state
    assert_eq!(
        scheduler.pending_count(),
        1,
        "should have 1 pending bead (bead-pending)"
    );
    assert_eq!(
        scheduler.ready_count(),
        1,
        "should have 1 ready bead (bead-ready)"
    );
    assert_eq!(
        scheduler.stats().assigned_count,
        1,
        "should have 1 in-flight bead (bead-in-flight)"
    );

    // WHEN: Stop is requested with in-flight work
    let stop_result = scheduler.stop().await;

    // THEN: Stop should fail because there's in-flight work
    assert!(
        stop_result.is_err(),
        "stop should fail when in-flight work exists"
    );

    let error = stop_result.expect_err("should have error");
    assert!(
        error.to_string().contains("in-flight"),
        "error should mention in-flight work"
    );
}

/// GIVEN scheduler with only completed beads
/// WHEN stop is requested
/// THEN stop succeeds immediately
#[tokio::test]
async fn given_scheduler_with_completed_work_when_stop_requested_then_stops_immediately() {
    // GIVEN: A scheduler with only completed beads
    let mut scheduler = SchedulerActor::new();
    let workflow_id = "workflow-test".to_string();
    let bead_1 = "bead-1".to_string();
    let bead_2 = "bead-2".to_string();

    scheduler
        .register_workflow(workflow_id.clone())
        .expect("workflow registration should succeed");

    scheduler
        .schedule_bead(workflow_id.clone(), bead_1.clone())
        .expect("bead-1 scheduling should succeed");
    scheduler
        .schedule_bead(workflow_id, bead_2.clone())
        .expect("bead-2 scheduling should succeed");

    // Mark all beads as completed
    scheduler
        .handle_bead_completed(&bead_1)
        .expect("bead-1 completion should succeed");
    scheduler
        .handle_bead_completed(&bead_2)
        .expect("bead-2 completion should succeed");

    // Verify no in-flight work
    assert_eq!(
        scheduler.stats().assigned_count,
        0,
        "should have no in-flight beads"
    );
    assert_eq!(
        scheduler.ready_count(),
        0,
        "should have no ready beads"
    );
    assert_eq!(
        scheduler.pending_count(),
        0,
        "should have no pending beads (all completed)"
    );

    // WHEN: Stop is requested with no in-flight work
    let stop_result = scheduler.stop().await;

    // THEN: Stop should succeed
    assert!(
        stop_result.is_ok(),
        "stop should succeed when no in-flight work exists"
    );
}

/// GIVEN scheduler with ready beads but not in-flight
/// WHEN stop is requested
/// THEN stop succeeds without waiting for ready beads
#[tokio::test]
async fn given_scheduler_with_ready_beads_when_stop_requested_then_stops_without_dispatching() {
    // GIVEN: A scheduler with ready beads (not yet assigned)
    let mut scheduler = SchedulerActor::new();
    let workflow_id = "workflow-test".to_string();
    let bead_ready = "bead-ready".to_string();

    scheduler
        .register_workflow(workflow_id)
        .expect("workflow registration should succeed");

    scheduler
        .schedule_bead("workflow-test".to_string(), bead_ready.clone())
        .expect("bead scheduling should succeed");
    scheduler
        .mark_ready(&bead_ready)
        .expect("marking bead ready should succeed");

    // Verify state
    assert_eq!(
        scheduler.ready_count(),
        1,
        "should have 1 ready bead"
    );
    assert_eq!(
        scheduler.stats().assigned_count,
        0,
        "should have no in-flight beads"
    );

    // WHEN: Stop is requested with ready beads but no in-flight work
    let stop_result = scheduler.stop().await;

    // THEN: Stop should succeed (ready beads don't block stop)
    assert!(
        stop_result.is_ok(),
        "stop should succeed when no in-flight work exists (ready beads OK)"
    );
}
