//! Behavioral tests for HeartbeatMonitor
//!
//! These tests document WHAT the HeartbeatMonitor does, not HOW it does it.
//! Following Martin Fowler's testing principles:
//! - Test behavior, not implementation
//! - Tests should survive refactoring
//! - Test names describe behavior
//! - Focus on: given input X → expect output Y
//!
//! The HeartbeatMonitor is responsible for:
//! - Tracking health status of worker processes
//! - Degrading health after missed heartbeats
//! - Marking workers unhealthy after repeated failures
//! - Restoring health when heartbeats resume

#![deny(clippy::unwrap_used)]
#![deny(clippy::expect_used)]
#![deny(clippy::panic)]

use pipeline::{HeartbeatMonitor, HealthStatus, ProcessId};

// ============================================================================
// BEHAVIOR: Monitoring Configuration
// ============================================================================

#[test]
fn should_schedule_health_checks_at_configured_interval() {
    // Given: A monitor configured with 60-second interval
    let result = HeartbeatMonitor::with_interval(60);

    // Then: The monitor should report that interval
    assert!(result.is_ok());
    let monitor = result.ok().filter(|m| m.check_interval() == 60);
    assert!(monitor.is_some(), "Monitor should use configured interval");
}

#[test]
fn should_use_default_interval_when_not_specified() {
    // Given: A monitor created with default settings
    let monitor = HeartbeatMonitor::new();

    // Then: It should use 30-second default interval
    assert_eq!(
        monitor.check_interval(),
        30,
        "Default interval should be 30 seconds"
    );
}

#[test]
fn should_reject_zero_second_check_interval() {
    // Given: An attempt to create monitor with zero-second interval
    let result = HeartbeatMonitor::with_interval(0);

    // Then: The creation should fail
    assert!(result.is_err(), "Zero-second interval should be rejected");
}

#[test]
fn should_reject_excessively_long_check_interval() {
    // Given: An attempt to create monitor with >1 hour interval
    let result = HeartbeatMonitor::with_interval(3601);

    // Then: The creation should fail
    assert!(
        result.is_err(),
        "Intervals exceeding 3600 seconds should be rejected"
    );
}

// ============================================================================
// BEHAVIOR: Worker Registration
// ============================================================================

#[test]
fn should_track_newly_registered_workers() {
    // Given: A monitor with no workers
    let mut monitor = HeartbeatMonitor::new();
    assert_eq!(monitor.worker_count(), 0);

    // When: A worker is registered
    let worker_id = ProcessId::new(1);
    let result = monitor.register_worker(worker_id);

    // Then: The worker should be tracked
    assert!(result.is_ok(), "Worker registration should succeed");
    assert_eq!(monitor.worker_count(), 1, "Worker count should increase");

    let workers = monitor.registered_workers();
    assert!(
        workers.contains(&worker_id),
        "Worker should appear in registered list"
    );
}

#[test]
fn should_prevent_duplicate_worker_registration() {
    // Given: A monitor with one registered worker
    let mut monitor = HeartbeatMonitor::new();
    let worker_id = ProcessId::new(1);
    let first_result = monitor.register_worker(worker_id);
    assert!(first_result.is_ok());

    // When: The same worker is registered again
    let second_result = monitor.register_worker(worker_id);

    // Then: The duplicate registration should fail
    assert!(
        second_result.is_err(),
        "Duplicate registration should be rejected"
    );
    assert_eq!(
        monitor.worker_count(),
        1,
        "Worker count should remain unchanged"
    );
}

#[test]
fn should_remove_worker_from_monitoring() {
    // Given: A monitor with a registered worker
    let mut monitor = HeartbeatMonitor::new();
    let worker_id = ProcessId::new(1);
    let register_result = monitor.register_worker(worker_id);
    assert!(register_result.is_ok());

    // When: The worker is unregistered
    let removed = monitor.unregister_worker(&worker_id);

    // Then: The worker should no longer be tracked
    assert!(removed.is_some(), "Unregistration should return the health check");
    assert_eq!(monitor.worker_count(), 0, "Worker count should decrease");
    assert!(
        monitor.get_health_status(&worker_id).is_none(),
        "Worker should not have health status after removal"
    );
}

// ============================================================================
// BEHAVIOR: Initial Health Status
// ============================================================================

#[test]
fn should_mark_worker_healthy_when_first_registered() {
    // Given: A newly registered worker
    let mut monitor = HeartbeatMonitor::new();
    let worker_id = ProcessId::new(1);
    let result = monitor.register_worker(worker_id);
    assert!(result.is_ok());

    // Then: The worker should start in healthy state
    let status = monitor.get_health_status(&worker_id);
    assert_eq!(
        status,
        Some(HealthStatus::Healthy),
        "New workers should start healthy"
    );

    let health_check = monitor.get_health_check(&worker_id);
    assert!(health_check.is_some());
    let check = health_check.filter(|c| c.consecutive_failures() == 0);
    assert!(
        check.is_some(),
        "New workers should have zero consecutive failures"
    );
}

// ============================================================================
// BEHAVIOR: Heartbeat Reception (Success)
// ============================================================================

#[test]
fn should_mark_worker_healthy_when_heartbeat_received() {
    // Given: A registered worker
    let mut monitor = HeartbeatMonitor::new();
    let worker_id = ProcessId::new(1);
    let register_result = monitor.register_worker(worker_id);
    assert!(register_result.is_ok());

    // When: A successful heartbeat is received
    let result = monitor.record_success(&worker_id);

    // Then: The worker should remain healthy
    assert!(result.is_ok(), "Heartbeat recording should succeed");
    let status = monitor.get_health_status(&worker_id);
    assert_eq!(
        status,
        Some(HealthStatus::Healthy),
        "Worker should be healthy after heartbeat"
    );
}

#[test]
fn should_reset_failure_count_when_heartbeat_received() {
    // Given: A worker with previous failures
    let mut monitor = HeartbeatMonitor::new();
    let worker_id = ProcessId::new(1);
    let register_result = monitor.register_worker(worker_id);
    assert!(register_result.is_ok());

    let fail1 = monitor.record_failure(&worker_id);
    assert!(fail1.is_ok());
    let fail2 = monitor.record_failure(&worker_id);
    assert!(fail2.is_ok());

    let check_before = monitor.get_health_check(&worker_id);
    assert!(check_before.is_some());
    assert!(
        check_before.filter(|c| c.consecutive_failures() == 2).is_some(),
        "Should have 2 consecutive failures"
    );

    // When: A successful heartbeat is received
    let result = monitor.record_success(&worker_id);
    assert!(result.is_ok());

    // Then: The failure count should reset to zero
    let check_after = monitor.get_health_check(&worker_id);
    assert!(check_after.is_some());
    let reset_check = check_after.filter(|c| c.consecutive_failures() == 0);
    assert!(
        reset_check.is_some(),
        "Failure count should reset after successful heartbeat"
    );
}

#[test]
fn should_reject_heartbeat_from_unregistered_worker() {
    // Given: A monitor with no registered workers
    let mut monitor = HeartbeatMonitor::new();
    let worker_id = ProcessId::new(999);

    // When: A heartbeat is received from an unknown worker
    let result = monitor.record_success(&worker_id);

    // Then: The heartbeat should be rejected
    assert!(
        result.is_err(),
        "Heartbeats from unregistered workers should be rejected"
    );
}

// ============================================================================
// BEHAVIOR: Health Degradation (Single Failure)
// ============================================================================

#[test]
fn should_mark_worker_degraded_after_one_missed_check() {
    // Given: A healthy worker
    let mut monitor = HeartbeatMonitor::new();
    let worker_id = ProcessId::new(1);
    let register_result = monitor.register_worker(worker_id);
    assert!(register_result.is_ok());

    // When: One health check fails
    let result = monitor.record_failure(&worker_id);

    // Then: The worker should be marked as degraded
    assert!(result.is_ok(), "Failure recording should succeed");
    let status = monitor.get_health_status(&worker_id);
    assert_eq!(
        status,
        Some(HealthStatus::Degraded),
        "Worker should be degraded after one failure"
    );

    let check = monitor.get_health_check(&worker_id);
    assert!(check.is_some());
    let failure_check = check.filter(|c| c.consecutive_failures() == 1);
    assert!(
        failure_check.is_some(),
        "Should have 1 consecutive failure"
    );
}

#[test]
fn should_remain_degraded_after_two_consecutive_failures() {
    // Given: A healthy worker
    let mut monitor = HeartbeatMonitor::new();
    let worker_id = ProcessId::new(1);
    let register_result = monitor.register_worker(worker_id);
    assert!(register_result.is_ok());

    // When: Two health checks fail
    let result1 = monitor.record_failure(&worker_id);
    assert!(result1.is_ok());
    let result2 = monitor.record_failure(&worker_id);
    assert!(result2.is_ok());

    // Then: The worker should still be degraded (not yet unhealthy)
    let status = monitor.get_health_status(&worker_id);
    assert_eq!(
        status,
        Some(HealthStatus::Degraded),
        "Worker should be degraded after two failures"
    );

    let check = monitor.get_health_check(&worker_id);
    assert!(check.is_some());
    let failure_check = check.filter(|c| c.consecutive_failures() == 2);
    assert!(
        failure_check.is_some(),
        "Should have 2 consecutive failures"
    );
}

// ============================================================================
// BEHAVIOR: Unhealthy State (Three Failures)
// ============================================================================

#[test]
fn should_mark_worker_unhealthy_after_three_consecutive_failures() {
    // Given: A healthy worker
    let mut monitor = HeartbeatMonitor::new();
    let worker_id = ProcessId::new(1);
    let register_result = monitor.register_worker(worker_id);
    assert!(register_result.is_ok());

    // When: Three consecutive health checks fail
    let result1 = monitor.record_failure(&worker_id);
    assert!(result1.is_ok());
    let result2 = monitor.record_failure(&worker_id);
    assert!(result2.is_ok());
    let result3 = monitor.record_failure(&worker_id);
    assert!(result3.is_ok());

    // Then: The worker should be marked as unhealthy
    let status = monitor.get_health_status(&worker_id);
    assert_eq!(
        status,
        Some(HealthStatus::Unhealthy),
        "Worker should be unhealthy after three failures"
    );

    let check = monitor.get_health_check(&worker_id);
    assert!(check.is_some());
    let failure_check = check.filter(|c| c.consecutive_failures() == 3);
    assert!(
        failure_check.is_some(),
        "Should have 3 consecutive failures"
    );
}

#[test]
fn should_remain_unhealthy_after_additional_failures() {
    // Given: A worker that is already unhealthy
    let mut monitor = HeartbeatMonitor::new();
    let worker_id = ProcessId::new(1);
    let register_result = monitor.register_worker(worker_id);
    assert!(register_result.is_ok());

    let f1 = monitor.record_failure(&worker_id);
    assert!(f1.is_ok());
    let f2 = monitor.record_failure(&worker_id);
    assert!(f2.is_ok());
    let f3 = monitor.record_failure(&worker_id);
    assert!(f3.is_ok());

    // When: Additional failures occur
    let result4 = monitor.record_failure(&worker_id);
    assert!(result4.is_ok());

    // Then: The worker should remain unhealthy
    let status = monitor.get_health_status(&worker_id);
    assert_eq!(
        status,
        Some(HealthStatus::Unhealthy),
        "Worker should remain unhealthy"
    );

    let check = monitor.get_health_check(&worker_id);
    assert!(check.is_some());
    let failure_check = check.filter(|c| c.consecutive_failures() == 4);
    assert!(
        failure_check.is_some(),
        "Failure count should continue to increment"
    );
}

#[test]
fn should_recover_unhealthy_worker_when_heartbeat_resumes() {
    // Given: A worker that became unhealthy
    let mut monitor = HeartbeatMonitor::new();
    let worker_id = ProcessId::new(1);
    let register_result = monitor.register_worker(worker_id);
    assert!(register_result.is_ok());

    let f1 = monitor.record_failure(&worker_id);
    assert!(f1.is_ok());
    let f2 = monitor.record_failure(&worker_id);
    assert!(f2.is_ok());
    let f3 = monitor.record_failure(&worker_id);
    assert!(f3.is_ok());

    let status_before = monitor.get_health_status(&worker_id);
    assert_eq!(status_before, Some(HealthStatus::Unhealthy));

    // When: The worker sends a successful heartbeat
    let result = monitor.record_success(&worker_id);
    assert!(result.is_ok());

    // Then: The worker should recover to healthy state
    let status = monitor.get_health_status(&worker_id);
    assert_eq!(
        status,
        Some(HealthStatus::Healthy),
        "Worker should recover to healthy after successful heartbeat"
    );

    let check = monitor.get_health_check(&worker_id);
    assert!(check.is_some());
    let recovery_check = check.filter(|c| c.consecutive_failures() == 0);
    assert!(
        recovery_check.is_some(),
        "Failure count should reset to zero"
    );
}

// ============================================================================
// BEHAVIOR: Worker Queries
// ============================================================================

#[test]
fn should_list_all_healthy_workers() {
    // Given: Multiple workers in different health states
    let mut monitor = HeartbeatMonitor::new();
    let healthy1 = ProcessId::new(1);
    let degraded = ProcessId::new(2);
    let unhealthy = ProcessId::new(3);
    let healthy2 = ProcessId::new(4);

    let r1 = monitor.register_worker(healthy1);
    assert!(r1.is_ok());
    let r2 = monitor.register_worker(degraded);
    assert!(r2.is_ok());
    let r3 = monitor.register_worker(unhealthy);
    assert!(r3.is_ok());
    let r4 = monitor.register_worker(healthy2);
    assert!(r4.is_ok());

    // Make degraded worker degraded (1 failure)
    let d1 = monitor.record_failure(&degraded);
    assert!(d1.is_ok());

    // Make unhealthy worker unhealthy (3 failures)
    let u1 = monitor.record_failure(&unhealthy);
    assert!(u1.is_ok());
    let u2 = monitor.record_failure(&unhealthy);
    assert!(u2.is_ok());
    let u3 = monitor.record_failure(&unhealthy);
    assert!(u3.is_ok());

    // When: Querying for healthy workers
    let healthy_list = monitor.healthy_workers();

    // Then: Only healthy workers should be returned
    assert_eq!(healthy_list.len(), 2, "Should have 2 healthy workers");
    assert!(
        healthy_list.contains(&healthy1),
        "Should include first healthy worker"
    );
    assert!(
        healthy_list.contains(&healthy2),
        "Should include second healthy worker"
    );
    assert!(
        !healthy_list.contains(&degraded),
        "Should not include degraded worker"
    );
    assert!(
        !healthy_list.contains(&unhealthy),
        "Should not include unhealthy worker"
    );
}

#[test]
fn should_list_all_unhealthy_workers_requiring_intervention() {
    // Given: Multiple workers in different health states
    let mut monitor = HeartbeatMonitor::new();
    let healthy = ProcessId::new(1);
    let degraded = ProcessId::new(2);
    let unhealthy1 = ProcessId::new(3);
    let unhealthy2 = ProcessId::new(4);

    let r1 = monitor.register_worker(healthy);
    assert!(r1.is_ok());
    let r2 = monitor.register_worker(degraded);
    assert!(r2.is_ok());
    let r3 = monitor.register_worker(unhealthy1);
    assert!(r3.is_ok());
    let r4 = monitor.register_worker(unhealthy2);
    assert!(r4.is_ok());

    // Make degraded worker degraded (1 failure)
    let d1 = monitor.record_failure(&degraded);
    assert!(d1.is_ok());

    // Make unhealthy workers unhealthy (3 failures each)
    for _i in 0..3 {
        let u1 = monitor.record_failure(&unhealthy1);
        assert!(u1.is_ok());
        let u2 = monitor.record_failure(&unhealthy2);
        assert!(u2.is_ok());
    }

    // When: Querying for unhealthy workers
    let unhealthy_list = monitor.unhealthy_workers();

    // Then: Only unhealthy workers should be returned
    assert_eq!(unhealthy_list.len(), 2, "Should have 2 unhealthy workers");
    assert!(
        unhealthy_list.contains(&unhealthy1),
        "Should include first unhealthy worker"
    );
    assert!(
        unhealthy_list.contains(&unhealthy2),
        "Should include second unhealthy worker"
    );
    assert!(
        !unhealthy_list.contains(&healthy),
        "Should not include healthy worker"
    );
    assert!(
        !unhealthy_list.contains(&degraded),
        "Should not include degraded worker (not yet requiring intervention)"
    );
}

#[test]
fn should_count_workers_by_health_status() {
    // Given: Multiple workers in different health states
    let mut monitor = HeartbeatMonitor::new();

    // Register 5 workers
    for i in 1..=5 {
        let result = monitor.register_worker(ProcessId::new(i));
        assert!(result.is_ok());
    }

    // Worker 2: Degraded (1 failure)
    let d1 = monitor.record_failure(&ProcessId::new(2));
    assert!(d1.is_ok());

    // Worker 3: Degraded (2 failures)
    let d2a = monitor.record_failure(&ProcessId::new(3));
    assert!(d2a.is_ok());
    let d2b = monitor.record_failure(&ProcessId::new(3));
    assert!(d2b.is_ok());

    // Worker 4: Unhealthy (3 failures)
    for _i in 0..3 {
        let u1 = monitor.record_failure(&ProcessId::new(4));
        assert!(u1.is_ok());
    }

    // Worker 5: Unhealthy (4 failures)
    for _i in 0..4 {
        let u2 = monitor.record_failure(&ProcessId::new(5));
        assert!(u2.is_ok());
    }

    // When: Counting workers by status
    let healthy_count = monitor.count_by_status(HealthStatus::Healthy);
    let degraded_count = monitor.count_by_status(HealthStatus::Degraded);
    let unhealthy_count = monitor.count_by_status(HealthStatus::Unhealthy);

    // Then: Counts should match expected distribution
    assert_eq!(healthy_count, 1, "Should have 1 healthy worker");
    assert_eq!(degraded_count, 2, "Should have 2 degraded workers");
    assert_eq!(unhealthy_count, 2, "Should have 2 unhealthy workers");
}

// ============================================================================
// BEHAVIOR: Error Handling
// ============================================================================

#[test]
fn should_reject_failure_recording_for_unregistered_worker() {
    // Given: A monitor with no registered workers
    let mut monitor = HeartbeatMonitor::new();
    let worker_id = ProcessId::new(999);

    // When: Recording a failure for an unknown worker
    let result = monitor.record_failure(&worker_id);

    // Then: The operation should fail
    assert!(
        result.is_err(),
        "Recording failures for unregistered workers should be rejected"
    );
}

#[test]
fn should_return_none_for_unregistered_worker_health_status() {
    // Given: A monitor with no registered workers
    let monitor = HeartbeatMonitor::new();
    let worker_id = ProcessId::new(999);

    // When: Querying health status of unknown worker
    let status = monitor.get_health_status(&worker_id);

    // Then: No status should be returned
    assert!(
        status.is_none(),
        "Unregistered workers should not have health status"
    );
}
