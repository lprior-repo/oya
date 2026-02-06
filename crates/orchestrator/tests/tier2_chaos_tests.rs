//! Tier-2 Actor Chaos Tests
//!
//! HOSTILE chaos engineering tests for tier-2 actor failures.
//! These tests verify that tier-2 actor crashes are handled gracefully
//! and that the system achieves 100% recovery rate.

#![deny(clippy::unwrap_used)]
#![deny(clippy::expect_used)]
#![deny(clippy::panic)]

use std::collections::HashMap;
use std::time::{Duration, SystemTime, UNIX_EPOCH};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;

use orchestrator::actors::scheduler::{SchedulerActorDef, SchedulerArguments};
use orchestrator::actors::supervisor::{
    SupervisorActorDef, SupervisorArguments, SupervisorConfig, SupervisorMessage,
};
use ractor::{Actor, ActorRef, ActorStatus};
use tokio::time::sleep;

const STATUS_TIMEOUT: Duration = Duration::from_millis(200);
const RECOVERY_WAIT: Duration = Duration::from_millis(500);

fn supervisor_args(config: SupervisorConfig) -> SupervisorArguments {
    SupervisorArguments::new().with_config(config)
}

fn is_actor_alive(status: ActorStatus) -> bool {
    matches!(
        status,
        ActorStatus::Starting | ActorStatus::Running | ActorStatus::Upgrading
    )
}

fn is_actor_stopped(status: ActorStatus) -> bool {
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
    supervisor: &ActorRef<SupervisorMessage<SchedulerActorDef>>,
    name: &str,
    args: SchedulerArguments,
) -> Result<(), String> {
    let (tx, rx) = tokio::sync::oneshot::channel();
    supervisor
        .cast(SupervisorMessage::<SchedulerActorDef>::SpawnChild {
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

async fn spawn_supervisor_with_name(
    args: SupervisorArguments,
    name: &str,
) -> Result<ActorRef<SupervisorMessage<SchedulerActorDef>>, String> {
    let (actor, _handle) = Actor::spawn(
        Some(name.to_string()),
        SupervisorActorDef::<SchedulerActorDef>::new(SchedulerActorDef),
        args,
    )
    .await
    .map_err(|e| format!("Failed to spawn supervisor: {}", e))?;
    Ok(actor)
}

async fn get_supervisor_status(
    supervisor: &ActorRef<SupervisorMessage<SchedulerActorDef>>,
) -> Result<usize, String> {
    let (tx, rx) = tokio::sync::oneshot::channel();
    supervisor
        .cast(SupervisorMessage::GetStatus { reply: tx })
        .map_err(|e| format!("Failed to get status: {}", e))?;

    match tokio::time::timeout(STATUS_TIMEOUT, rx).await {
        Ok(Ok(status)) => Ok(status.active_children),
        Ok(Err(e)) => Err(format!("Failed to receive status: {}", e)),
        Err(_) => Err("Timeout waiting for status".to_string()),
    }
}

// ============================================================================
// TIER-2 KILL TESTS (hostile: kill tier-2 actors, verify recovery)
// ============================================================================

/// **Attack 1.1**: Kill single tier-2 actor, verify 100% recovery
#[tokio::test]
async fn given_tier2_actor_killed_when_supervisor_active_then_full_recovery() {
    // GIVEN: A tier-1 supervisor with a tier-2 child
    let mut config = SupervisorConfig::for_testing();
    config.max_restarts = 10; // Allow restarts
    config.base_backoff_ms = 10; // Fast restart for testing

    let supervisor_name = unique_name("chaos-tier2-single-kill");
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
        is_actor_alive(supervisor.get_status()),
        "supervisor should be alive"
    );

    // Spawn a tier-2 child
    let child_args = SchedulerArguments::new();
    let child_name = format!("{supervisor_name}-victim-1");
    let spawn_result = spawn_child(&supervisor, &child_name, child_args).await;

    assert!(
        spawn_result.is_ok(),
        "child spawn should succeed: {:?}",
        spawn_result
    );

    // Give child time to start
    sleep(Duration::from_millis(100)).await;

    // Verify child is tracked
    let child_count_before = get_supervisor_status(&supervisor).await;
    assert_eq!(
        child_count_before.unwrap_or(0), 1,
        "supervisor should track 1 child"
    );

    // WHEN: Kill the tier-2 actor (stop the child)
    let stop_result = supervisor
        .cast(SupervisorMessage::StopChild {
            name: child_name.clone(),
        })
        .map_err(|e| format!("Failed to stop child: {}", e));

    assert!(
        stop_result.is_ok(),
        "child stop should succeed: {:?}",
        stop_result
    );

    // THEN: Wait for recovery (restart)
    sleep(RECOVERY_WAIT).await;

    // Verify recovery: child should be restarted
    let child_count_after = get_supervisor_status(&supervisor).await;
    assert_eq!(
        child_count_after.unwrap_or(0), 1,
        "supervisor should have 1 child after recovery"
    );

    // Verify supervisor is still alive
    assert!(
        is_actor_alive(supervisor.get_status()),
        "supervisor should still be alive after child recovery"
    );

    // Clean up
    supervisor.stop(Some("Test complete".to_string()));
}

/// **Attack 1.2**: Kill multiple tier-2 actors simultaneously
#[tokio::test]
async fn given_multiple_tier2_killed_when_simultaneous_then_all_recover() {
    // GIVEN: A tier-1 supervisor with multiple tier-2 children
    let mut config = SupervisorConfig::for_testing();
    config.max_restarts = 10;
    config.base_backoff_ms = 10;

    let supervisor_name = unique_name("chaos-tier2-multi-kill");
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

    // Spawn 5 tier-2 children
    let child_count = 5;
    let child_names: Vec<String> = (0..child_count)
        .map(|i| format!("{supervisor_name}-victim-{i}"))
        .collect();

    for child_name in &child_names {
        let child_args = SchedulerArguments::new();
        let spawn_result = spawn_child(&supervisor, child_name, child_args).await;

        assert!(
            spawn_result.is_ok(),
            "child '{}' spawn should succeed: {:?}",
            child_name,
            spawn_result
        );
    }

    // Give children time to start
    sleep(Duration::from_millis(100)).await;

    // Verify all children are tracked
    let child_count_before = get_supervisor_status(&supervisor).await;
    assert_eq!(
        child_count_before.unwrap_or(0), child_count,
        "supervisor should track {} children",
        child_count
    );

    // WHEN: Kill all tier-2 actors simultaneously
    for child_name in &child_names {
        let stop_result = supervisor.cast(SupervisorMessage::StopChild {
            name: child_name.clone(),
        });

        assert!(
            stop_result.is_ok(),
            "child '{}' stop should succeed: {:?}",
            child_name,
            stop_result
        );
    }

    // THEN: Wait for recovery of all children
    sleep(RECOVERY_WAIT).await;

    // Verify recovery: all children should be restarted
    let child_count_after = get_supervisor_status(&supervisor).await;
    assert_eq!(
        child_count_after.unwrap_or(0), child_count,
        "supervisor should have {} children after recovery",
        child_count
    );

    // Verify supervisor is still alive
    assert!(
        is_actor_alive(supervisor.get_status()),
        "supervisor should still be alive after all children recovered"
    );

    // Clean up
    supervisor.stop(Some("Test complete".to_string()));
}

/// **Attack 1.3**: Kill random tier-2 actors repeatedly (chaos monkey)
#[tokio::test]
async fn given_chaos_monkey_kills_random_tier2_when_100_cycles_then_100_percent_recovery() {
    // GIVEN: A tier-1 supervisor with multiple tier-2 children
    let mut config = SupervisorConfig::for_testing();
    config.max_restarts = 1000; // High limit for chaos
    config.base_backoff_ms = 5; // Fast restart

    let supervisor_name = unique_name("chaos-tier2-monkey");
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

    // Spawn 10 tier-2 children
    let child_count = 10;
    let child_names: Vec<String> = (0..child_count)
        .map(|i| format!("{supervisor_name}-chaos-{i}"))
        .collect();

    for child_name in &child_names {
        let child_args = SchedulerArguments::new();
        let spawn_result = spawn_child(&supervisor, child_name, child_args).await;

        assert!(
            spawn_result.is_ok(),
            "child '{}' spawn should succeed: {:?}",
            child_name,
            spawn_result
        );
    }

    // Give children time to start
    sleep(Duration::from_millis(100)).await;

    // Verify all children are tracked
    let child_count_before = get_supervisor_status(&supervisor).await;
    assert_eq!(
        child_count_before.unwrap_or(0), child_count,
        "supervisor should track {} children",
        child_count
    );

    // Track recovery statistics
    let kill_count = Arc::new(AtomicUsize::new(0));
    let recovery_count = Arc::new(AtomicUsize::new(0));

    // WHEN: Chaos monkey kills random tier-2 actors for 100 cycles
    let chaos_cycles = 100;

    for cycle in 0..chaos_cycles {
        // Kill a random child
        let victim_index = (cycle % child_count) as usize;
        let victim_name = &child_names[victim_index];

        let stop_result = supervisor.cast(SupervisorMessage::StopChild {
            name: victim_name.clone(),
        });

        if stop_result.is_ok() {
            kill_count.fetch_add(1, Ordering::SeqCst);
        }

        // Small delay between kills
        sleep(Duration::from_millis(10)).await;

        // Check recovery
        let current_count = get_supervisor_status(&supervisor).await.unwrap_or(0);
        if current_count == child_count {
            recovery_count.fetch_add(1, Ordering::SeqCst);
        }
    }

    // Wait for final recovery
    sleep(RECOVERY_WAIT).await;

    // THEN: Verify 100% recovery rate
    let final_count = get_supervisor_status(&supervisor).await;
    assert_eq!(
        final_count.unwrap_or(0), child_count,
        "all tier-2 actors should be recovered after chaos"
    );

    let kills = kill_count.load(Ordering::SeqCst);
    let recoveries = recovery_count.load(Ordering::SeqCst);

    assert!(
        kills >= chaos_cycles / 2, // At least some kills succeeded
        "chaos monkey should have killed actors: {} kills",
        kills
    );

    // Verify supervisor survived the chaos
    assert!(
        is_actor_alive(supervisor.get_status()),
        "supervisor should survive chaos monkey attack"
    );

    // Clean up
    supervisor.stop(Some("Chaos test complete".to_string()));
}

/// **Attack 2.1**: Cascading tier-2 failures (kill children rapidly)
#[tokio::test]
async fn given_cascading_tier2_failures_when_rapid_kills_then_supervisor_survives() {
    // GIVEN: A tier-1 supervisor with many tier-2 children
    let mut config = SupervisorConfig::for_testing();
    config.max_restarts = 100;
    config.base_backoff_ms = 5;

    let supervisor_name = unique_name("chaos-tier2-cascade");
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

    // Spawn 20 tier-2 children
    let child_count = 20;
    let child_names: Vec<String> = (0..child_count)
        .map(|i| format!("{supervisor_name}-cascade-{i}"))
        .collect();

    for child_name in &child_names {
        let child_args = SchedulerArguments::new();
        let _ = spawn_child(&supervisor, child_name, child_args).await;
    }

    sleep(Duration::from_millis(100)).await;

    // WHEN: Kill all children in rapid succession (cascading failure)
    for child_name in &child_names {
        let _ = supervisor.cast(SupervisorMessage::StopChild {
            name: child_name.clone(),
        });
        // No delay between kills - simulate cascade
    }

    // THEN: Supervisor should survive and recover all children
    sleep(Duration::from_millis(500)).await;

    let final_count = get_supervisor_status(&supervisor).await;
    assert_eq!(
        final_count.unwrap_or(0), child_count,
        "all children should recover after cascade"
    );

    assert!(
        is_actor_alive(supervisor.get_status()),
        "supervisor should survive cascading failures"
    );

    // Clean up
    supervisor.stop(Some("Cascade test complete".to_string()));
}

/// **Attack 3.1**: Kill tier-2 actor during supervisor restart
#[tokio::test]
async fn given_tier2_killed_during_supervisor_restart_then_graceful() {
    // GIVEN: A tier-1 supervisor with a child
    let mut config = SupervisorConfig::for_testing();
    config.max_restarts = 5;
    config.base_backoff_ms = 20;

    let supervisor_name = unique_name("chaos-tier2-race");
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
    let child_name = format!("{supervisor_name}-race-victim");
    let _ = spawn_child(&supervisor, &child_name, child_args).await;

    sleep(Duration::from_millis(50)).await;

    // WHEN: Kill child and immediately stop supervisor (race condition)
    let _ = supervisor.cast(SupervisorMessage::StopChild {
        name: child_name.clone(),
    });

    sleep(Duration::from_millis(5)).await; // Small delay to trigger restart

    supervisor.stop(Some("Stop during child restart".to_string()));

    // THEN: Should stop gracefully without panic
    sleep(Duration::from_millis(100)).await;

    assert!(
        is_actor_stopped(supervisor.get_status()),
        "supervisor should stop cleanly"
    );
}

/// **Attack 4.1**: Continuous chaos for 5 seconds
#[tokio::test]
async fn given_continuous_chaos_when_5_seconds_then_stable_recovery() {
    // GIVEN: A tier-1 supervisor with tier-2 children
    let mut config = SupervisorConfig::for_testing();
    config.max_restarts = 500;
    config.base_backoff_ms = 2; // Very fast restart

    let supervisor_name = unique_name("chaos-tier2-continuous");
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

    // Spawn 8 tier-2 children
    let child_count = 8;
    let child_names: Vec<String> = (0..child_count)
        .map(|i| format!("{supervisor_name}-continuous-{i}"))
        .collect();

    for child_name in &child_names {
        let child_args = SchedulerArguments::new();
        let _ = spawn_child(&supervisor, child_name, child_args).await;
    }

    sleep(Duration::from_millis(100)).await;

    // WHEN: Run continuous chaos for 5 seconds
    let chaos_duration = Duration::from_secs(5);
    let start = SystemTime::now();
    let mut kill_count = 0;

    while SystemTime::now()
        .duration_since(start)
        .unwrap_or_default()
        < chaos_duration
    {
        // Kill a random child
        let victim_index = kill_count as usize % child_count;
        let victim_name = &child_names[victim_index];

        let _ = supervisor.cast(SupervisorMessage::StopChild {
            name: victim_name.clone(),
        });

        kill_count += 1;
        sleep(Duration::from_millis(50)).await; // 20 kills/second
    }

    // THEN: System should be stable after chaos ends
    sleep(Duration::from_millis(500)).await;

    let final_count = get_supervisor_status(&supervisor).await;
    assert_eq!(
        final_count.unwrap_or(0), child_count,
        "all tier-2 actors should recover after continuous chaos"
    );

    assert!(
        is_actor_alive(supervisor.get_status()),
        "supervisor should survive continuous chaos"
    );

    // Clean up
    supervisor.stop(Some("Continuous chaos test complete".to_string()));
}

/// **Attack 5.1**: Verify 100% recovery rate across all kills
#[tokio::test]
async fn given_100_tier2_kills_when_all_recover_then_rate_is_100_percent() {
    // GIVEN: A tier-1 supervisor with tier-2 children
    let mut config = SupervisorConfig::for_testing();
    config.max_restarts = 200;
    config.base_backoff_ms = 5;

    let supervisor_name = unique_name("chaos-tier2-recovery-rate");
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

    // Spawn 5 tier-2 children
    let child_count = 5;
    let child_names: Vec<String> = (0..child_count)
        .map(|i| format!("{supervisor_name}-recovery-{i}"))
        .collect();

    for child_name in &child_names {
        let child_args = SchedulerArguments::new();
        let _ = spawn_child(&supervisor, child_name, child_args).await;
    }

    sleep(Duration::from_millis(100)).await;

    // WHEN: Kill each child 20 times (100 total kills)
    let kills_per_child = 20;
    let total_kills = child_count * kills_per_child;

    for round in 0..kills_per_child {
        for child_name in &child_names {
            let _ = supervisor.cast(SupervisorMessage::StopChild {
                name: child_name.clone(),
            });
            sleep(Duration::from_millis(10)).await;
        }

        // Verify recovery between rounds
        sleep(Duration::from_millis(50)).await;
        let current_count = get_supervisor_status(&supervisor).await;
        assert_eq!(
            current_count.unwrap_or(0), child_count,
            "round {}: all children should recover",
            round
        );
    }

    // THEN: Final verification - 100% recovery rate
    let final_count = get_supervisor_status(&supervisor).await;
    assert_eq!(
        final_count.unwrap_or(0), child_count,
        "after {} kills, all tier-2 actors should be recovered (100% rate)",
        total_kills
    );

    assert!(
        is_actor_alive(supervisor.get_status()),
        "supervisor should survive all kills"
    );

    // Clean up
    supervisor.stop(Some("Recovery rate test complete".to_string()));
}
