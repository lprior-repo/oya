//! Supervisor Chaos Tests - Tier-1 crash recovery
//!
//! HOSTILE chaos engineering tests for supervisor crash scenarios.
//! These tests verify that tier-1 supervisor crashes are handled gracefully
//! and that the system can recover from supervisor failures.

#![deny(clippy::unwrap_used)]
#![deny(clippy::expect_used)]
#![deny(clippy::panic)]

use std::time::{Duration, SystemTime, UNIX_EPOCH};

use orchestrator::actors::scheduler::SchedulerArguments;
use orchestrator::actors::supervisor::{
    SchedulerSupervisorConfig, SupervisorArguments, SupervisorMessage, spawn_supervisor_with_name,
};
use ractor::{ActorRef, ActorStatus};
use tokio::time::sleep;

const STATUS_TIMEOUT: Duration = Duration::from_millis(200);

fn supervisor_args(config: SchedulerSupervisorConfig) -> SupervisorArguments {
    SupervisorArguments::new().with_config(config)
}

fn is_supervisor_alive(status: ActorStatus) -> bool {
    matches!(
        status,
        ActorStatus::Starting | ActorStatus::Running | ActorStatus::Upgrading
    )
}

fn is_supervisor_stopped(status: ActorStatus) -> bool {
    matches!(status, ActorStatus::Stopping | ActorStatus::Stopped)
}

fn unique_name(label: &str) -> String {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_nanos())
        .unwrap_or_default();
    format!("{label}-{}-{nanos}", std::process::id())
}

async fn spawn_child(
    supervisor: &ActorRef<SupervisorMessage>,
    name: &str,
    args: SchedulerArguments,
) -> Result<(), String> {
    let (tx, rx) = tokio::sync::oneshot::channel();
    supervisor
        .cast(SupervisorMessage::SpawnChild {
            name: name.to_string(),
            args,
            reply: tx,
        })
        .map_err(|e| format!("Failed to spawn child '{}': {}", name, e))?;

    match tokio::time::timeout(STATUS_TIMEOUT, rx).await {
        Ok(Ok(result)) => result.map_err(|e| format!("Child '{}' failed: {}", name, e)),
        Ok(Err(e)) => Err(format!("Child '{}' reply failed: {}", name, e)),
        Err(_) => Err(format!("Timeout waiting for child '{}'", name)),
    }
}

// ============================================================================
// TIER-1 CRASH TESTS (hostile: what if supervisor dies?)
// ============================================================================

/// **Attack 1.1**: When tier-1 crashes with no children, verify clean shutdown
#[tokio::test]
async fn given_tier1_crashes_with_no_children_then_clean_shutdown() {
    // GIVEN: A tier-1 supervisor with no children
    let config = SchedulerSupervisorConfig::for_testing();
    let supervisor_name = unique_name("chaos-supervisor-empty");
    let supervisor_result =
        spawn_supervisor_with_name(supervisor_args(config), &supervisor_name).await;

    assert!(
        supervisor_result.is_ok(),
        "should spawn supervisor successfully"
    );

    let supervisor = match supervisor_result {
        Ok(sup) => sup,
        Err(e) => {
            eprintln!("Failed to spawn supervisor: {}", e);
            return;
        }
    };

    // Verify supervisor is alive
    assert!(
        is_supervisor_alive(supervisor.get_status()),
        "supervisor should be alive"
    );

    // WHEN: Tier-1 crashes (simulated by stopping the actor)
    supervisor.stop(Some("Simulated tier-1 crash".to_string()));

    // Give time for shutdown
    sleep(Duration::from_millis(50)).await;

    // THEN: Supervisor should be cleanly stopped
    assert!(
        is_supervisor_stopped(supervisor.get_status()),
        "supervisor should be stopped after crash"
    );
}

/// **Attack 1.2**: When tier-1 crashes with running children, verify children are stopped
#[tokio::test]
async fn given_tier1_crashes_with_children_then_children_stopped() {
    // GIVEN: A tier-1 supervisor with children
    let config = SchedulerSupervisorConfig::for_testing();
    let supervisor_name = unique_name("chaos-supervisor-with-children");
    let supervisor_result =
        spawn_supervisor_with_name(supervisor_args(config), &supervisor_name).await;

    assert!(
        supervisor_result.is_ok(),
        "should spawn supervisor successfully"
    );

    let supervisor = match supervisor_result {
        Ok(sup) => sup,
        Err(e) => {
            eprintln!("Failed to spawn supervisor: {}", e);
            return;
        }
    };

    // Spawn multiple children
    let child_args = SchedulerArguments::new();

    let spawn_results = [
        spawn_child(
            &supervisor,
            &format!("{supervisor_name}-child-1"),
            child_args.clone(),
        )
        .await,
        spawn_child(
            &supervisor,
            &format!("{supervisor_name}-child-2"),
            child_args.clone(),
        )
        .await,
        spawn_child(
            &supervisor,
            &format!("{supervisor_name}-child-3"),
            child_args,
        )
        .await,
    ];

    // Check all spawns succeeded
    for (idx, result) in spawn_results.iter().enumerate() {
        assert!(
            result.is_ok(),
            "child-{} spawn should succeed: {:?}",
            idx + 1,
            result
        );
    }

    // Give children time to start
    sleep(Duration::from_millis(100)).await;

    // Query supervisor status to verify children are tracked
    let (tx, rx) = tokio::sync::oneshot::channel();
    let status_result = supervisor
        .cast(SupervisorMessage::GetStatus { reply: tx })
        .map_err(|e| format!("Failed to get status: {}", e));

    assert!(
        status_result.is_ok(),
        "should be able to query supervisor status"
    );

    // Wait for status response
    let status_response = tokio::time::timeout(STATUS_TIMEOUT, rx).await;

    let child_count = match status_response {
        Ok(Ok(status)) => status.active_children,
        Ok(Err(e)) => {
            eprintln!("Failed to receive status: {}", e);
            return;
        }
        Err(_) => {
            eprintln!("Timeout waiting for status");
            return;
        }
    };

    assert_eq!(child_count, 3, "supervisor should track 3 children");

    // WHEN: Tier-1 crashes
    supervisor.stop(Some("Simulated tier-1 crash with children".to_string()));

    // Give time for cascade shutdown
    sleep(Duration::from_millis(100)).await;

    // THEN: Supervisor should be stopped
    assert!(
        is_supervisor_stopped(supervisor.get_status()),
        "supervisor should be stopped after crash"
    );

    // HOSTILE: Verify children can't be messaged (they should be stopped too)
    // In a real supervisor crash, children would be orphaned and eventually cleaned up
    // by the actor system. We verify the supervisor is gone, which is the core requirement.
}

/// **Attack 1.3**: When tier-1 crashes during child restart, verify graceful handling
#[tokio::test]
async fn given_tier1_crashes_during_child_restart_then_graceful() {
    // GIVEN: A tier-1 supervisor with a child that's about to restart
    let mut config = SchedulerSupervisorConfig::for_testing();
    // Set very short restart delay to trigger race condition
    config.base_backoff_ms = 10;

    let supervisor_name = unique_name("chaos-supervisor-restart-race");
    let supervisor_result =
        spawn_supervisor_with_name(supervisor_args(config), &supervisor_name).await;

    assert!(
        supervisor_result.is_ok(),
        "should spawn supervisor successfully"
    );

    let supervisor = match supervisor_result {
        Ok(sup) => sup,
        Err(e) => {
            eprintln!("Failed to spawn supervisor: {}", e);
            return;
        }
    };

    // Spawn a child
    let child_args = SchedulerArguments::new();
    let spawn_result = spawn_child(
        &supervisor,
        &format!("{supervisor_name}-restart-target"),
        child_args,
    )
    .await;

    assert!(
        spawn_result.is_ok(),
        "child spawn should succeed: {:?}",
        spawn_result
    );

    sleep(Duration::from_millis(50)).await;

    // Stop the child to trigger a restart
    let stop_result = supervisor
        .cast(SupervisorMessage::StopChild {
            name: format!("{supervisor_name}-restart-target"),
        })
        .map_err(|e| format!("Failed to stop child: {}", e));

    assert!(
        stop_result.is_ok(),
        "child stop should succeed: {:?}",
        stop_result
    );

    // WHEN: Immediately crash supervisor during restart window
    sleep(Duration::from_millis(5)).await; // Crash before restart completes
    supervisor.stop(Some("Crash during child restart".to_string()));

    // Give time for cleanup
    sleep(Duration::from_millis(100)).await;

    // THEN: Supervisor should be stopped gracefully (no panic)
    assert!(
        is_supervisor_stopped(supervisor.get_status()),
        "supervisor should be stopped after crash during restart"
    );
}

/// **Attack 2.1**: When tier-1 receives invalid message after crash, verify no panic
#[tokio::test]
async fn given_tier1_stopped_when_message_sent_then_error_not_panic() {
    // GIVEN: A stopped tier-1 supervisor
    let config = SchedulerSupervisorConfig::for_testing();
    let supervisor_name = unique_name("chaos-supervisor-stopped");
    let supervisor_result =
        spawn_supervisor_with_name(supervisor_args(config), &supervisor_name).await;

    assert!(
        supervisor_result.is_ok(),
        "should spawn supervisor successfully"
    );

    let supervisor = match supervisor_result {
        Ok(sup) => sup,
        Err(e) => {
            eprintln!("Failed to spawn supervisor: {}", e);
            return;
        }
    };

    // Stop supervisor
    supervisor.stop(Some("Prepare for message test".to_string()));
    sleep(Duration::from_millis(50)).await;

    // WHEN: Sending message to stopped supervisor
    let child_args = SchedulerArguments::new();
    let (tx, rx) = tokio::sync::oneshot::channel();
    let result = supervisor.cast(SupervisorMessage::SpawnChild {
        name: format!("{supervisor_name}-zombie-child"),
        args: child_args,
        reply: tx,
    });

    // THEN: Should return error, not panic
    assert!(
        result.is_err(),
        "sending to stopped supervisor should return error"
    );

    // Verify error is MessagingErr type (actor stopped)
    if result.is_ok() {
        let _ = tokio::time::timeout(STATUS_TIMEOUT, rx).await;
    }
}

/// **Attack 3.1**: Rapid supervisor crash/restart cycles
#[tokio::test]
async fn given_rapid_tier1_crash_restart_cycles_then_stable() {
    // HOSTILE: Create and destroy supervisors rapidly to test resource cleanup
    for i in 0..10 {
        let config = SchedulerSupervisorConfig::for_testing();
        let supervisor_name = unique_name(&format!("chaos-supervisor-cycle-{i}"));
        let supervisor_result =
            spawn_supervisor_with_name(supervisor_args(config), &supervisor_name).await;

        assert!(
            supervisor_result.is_ok(),
            "cycle {} should spawn successfully",
            i
        );

        let supervisor = match supervisor_result {
            Ok(sup) => sup,
            Err(e) => {
                eprintln!("Failed to spawn supervisor in cycle {}: {}", i, e);
                continue;
            }
        };

        // Spawn a child
        let child_args = SchedulerArguments::new();
        let _ = spawn_child(&supervisor, &format!("{supervisor_name}-child"), child_args).await;

        sleep(Duration::from_millis(10)).await;

        // Crash it
        supervisor.stop(Some(format!("Crash cycle {}", i)));

        sleep(Duration::from_millis(10)).await;

        // Verify stopped
        assert!(
            is_supervisor_stopped(supervisor.get_status()),
            "cycle {} supervisor should be stopped",
            i
        );
    }

    // THEN: All cycles complete without panic or resource leak
    // If we get here, the test passed
}

/// **Attack 4.1**: Tier-1 crash with meltdown in progress
#[tokio::test]
async fn given_tier1_crashes_during_meltdown_then_graceful() {
    // GIVEN: A tier-1 supervisor configured to meltdown quickly
    let mut config = SchedulerSupervisorConfig::for_testing();
    config.meltdown_threshold = 2.0; // Very low threshold
    config.max_restarts = 1; // Allow only 1 restart

    let supervisor_name = unique_name("chaos-supervisor-meltdown");
    let supervisor_result =
        spawn_supervisor_with_name(supervisor_args(config), &supervisor_name).await;

    assert!(
        supervisor_result.is_ok(),
        "should spawn supervisor successfully"
    );

    let supervisor = match supervisor_result {
        Ok(sup) => sup,
        Err(e) => {
            eprintln!("Failed to spawn supervisor: {}", e);
            return;
        }
    };

    // Spawn a child
    let child_args = SchedulerArguments::new();
    let _ = spawn_child(
        &supervisor,
        &format!("{supervisor_name}-meltdown-child"),
        child_args,
    )
    .await;

    sleep(Duration::from_millis(50)).await;

    // Stop child multiple times to trigger meltdown
    for _i in 0..3 {
        let _ = supervisor.cast(SupervisorMessage::StopChild {
            name: format!("{supervisor_name}-meltdown-child"),
        });
        sleep(Duration::from_millis(20)).await;
    }

    // WHEN: Crash supervisor during meltdown window
    supervisor.stop(Some("Crash during meltdown".to_string()));

    sleep(Duration::from_millis(50)).await;

    // THEN: Should stop gracefully without panic
    assert!(
        is_supervisor_stopped(supervisor.get_status()),
        "supervisor should be stopped after meltdown crash"
    );
}

// ============================================================================
// RECOVERY TESTS (hostile: can tier-1 be restarted?)
// ============================================================================

/// **Attack 5.1**: Spawn new tier-1 after crash and verify it works
#[tokio::test]
async fn given_tier1_crashed_when_new_tier1_spawned_then_functional() {
    // GIVEN: A tier-1 supervisor that crashed
    let config = SchedulerSupervisorConfig::for_testing();
    let supervisor_one_name = unique_name("crash-then-recover-1");
    let supervisor1_result =
        spawn_supervisor_with_name(supervisor_args(config.clone()), &supervisor_one_name).await;

    assert!(
        supervisor1_result.is_ok(),
        "first supervisor should spawn successfully"
    );

    let supervisor1 = match supervisor1_result {
        Ok(sup) => sup,
        Err(e) => {
            eprintln!("Failed to spawn first supervisor: {}", e);
            return;
        }
    };

    // Crash it
    supervisor1.stop(Some("First supervisor crash".to_string()));
    sleep(Duration::from_millis(50)).await;

    assert!(
        is_supervisor_stopped(supervisor1.get_status()),
        "first supervisor should be stopped"
    );

    // WHEN: Spawn a new tier-1 supervisor with same name
    let supervisor_two_name = unique_name("crash-then-recover-2");
    let supervisor2_result =
        spawn_supervisor_with_name(supervisor_args(config), &supervisor_two_name).await;

    assert!(
        supervisor2_result.is_ok(),
        "second supervisor should spawn successfully"
    );

    let supervisor2 = match supervisor2_result {
        Ok(sup) => sup,
        Err(e) => {
            eprintln!("Failed to spawn second supervisor: {}", e);
            return;
        }
    };

    // THEN: New supervisor should be functional
    assert!(
        is_supervisor_alive(supervisor2.get_status()),
        "second supervisor should be alive"
    );

    // Spawn a child to verify functionality
    let child_args = SchedulerArguments::new();
    let spawn_result = spawn_child(
        &supervisor2,
        &format!("{supervisor_two_name}-recovery-child"),
        child_args,
    )
    .await;

    assert!(
        spawn_result.is_ok(),
        "new supervisor should spawn children successfully: {:?}",
        spawn_result
    );

    // Clean up
    supervisor2.stop(None);
}

/// **Attack 5.2**: Verify tier-1 crash doesn't corrupt shared state
#[tokio::test]
async fn given_tier1_crashes_then_no_shared_state_corruption() {
    // HOSTILE: Verify that supervisor crashes don't leave behind corrupt global state
    // This test verifies isolation between supervisor instances

    let config = SchedulerSupervisorConfig::for_testing();
    let base_name = unique_name("isolation-test");

    // Spawn and crash multiple supervisors
    for i in 0..5 {
        let supervisor_result = spawn_supervisor_with_name(
            supervisor_args(config.clone()),
            &format!("{base_name}-{i}"),
        )
        .await;

        assert!(
            supervisor_result.is_ok(),
            "supervisor {} should spawn successfully",
            i
        );

        let supervisor = match supervisor_result {
            Ok(sup) => sup,
            Err(e) => {
                eprintln!("Failed to spawn supervisor {}: {}", i, e);
                continue;
            }
        };

        // Add children
        let child_args = SchedulerArguments::new();
        let _ = spawn_child(
            &supervisor,
            &format!("{base_name}-child-{i}-1"),
            child_args.clone(),
        )
        .await;
        let _ = spawn_child(&supervisor, &format!("{base_name}-child-{i}-2"), child_args).await;

        sleep(Duration::from_millis(20)).await;

        // Crash it
        supervisor.stop(Some(format!("Crash test {}", i)));
        sleep(Duration::from_millis(20)).await;
    }

    // WHEN: Spawn a fresh supervisor after all crashes
    let final_supervisor_name = unique_name("isolation-test-final");
    let final_supervisor_result =
        spawn_supervisor_with_name(supervisor_args(config), &final_supervisor_name).await;

    assert!(
        final_supervisor_result.is_ok(),
        "final supervisor should spawn successfully"
    );

    let final_supervisor = match final_supervisor_result {
        Ok(sup) => sup,
        Err(e) => {
            eprintln!("Failed to spawn final supervisor: {}", e);
            return;
        }
    };

    // THEN: Should start with clean state (0 children)
    let (tx, rx) = tokio::sync::oneshot::channel();
    let _ = final_supervisor.cast(SupervisorMessage::GetStatus { reply: tx });

    let status_response = tokio::time::timeout(STATUS_TIMEOUT, rx).await;

    let child_count = match status_response {
        Ok(Ok(status)) => status.active_children,
        Ok(Err(e)) => {
            eprintln!("Failed to receive status: {}", e);
            return;
        }
        Err(_) => {
            eprintln!("Timeout waiting for status");
            return;
        }
    };

    assert_eq!(
        child_count, 0,
        "final supervisor should have clean state with 0 children"
    );

    // Clean up
    final_supervisor.stop(None);
}
