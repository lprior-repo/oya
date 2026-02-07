//! Tier-2 Random Kill Chaos Test
//!
//! Test that kills random tier-2 actors and verifies 100% recovery.
//! This is the implementation for bead src-23mm.

#![deny(clippy::unwrap_used)]
#![deny(clippy::expect_used)]
#![deny(clippy::panic)]

use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

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

/// **Test for bead src-23mm**: Kill random tier-2 actors, verify 100% recovery
///
/// This test:
/// 1. Spawns a supervisor with multiple tier-2 children
/// 2. Randomly kills tier-2 actors (chaos monkey pattern)
/// 3. Verifies 100% recovery rate (all actors restarted)
/// 4. Ensures supervisor survives the chaos
#[tokio::test]
async fn given_random_tier2_kills_when_100_percent_recovery_required_then_all_recover() {
    // GIVEN: A tier-1 supervisor with multiple tier-2 children
    let mut config = SupervisorConfig::for_testing();
    config.max_restarts = 1000; // High limit for chaos
    config.base_backoff_ms = 5; // Fast restart

    let supervisor_name = unique_name("chaos-tier2-random-kill");
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
        .map(|i| format!("{supervisor_name}-random-{i}"))
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
        child_count_before.map_or(0, |v| v),
        child_count,
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
        let current_count = get_supervisor_status(&supervisor).await.map_or(0, |v| v);
        if current_count == child_count {
            recovery_count.fetch_add(1, Ordering::SeqCst);
        }
    }

    // Wait for final recovery (give time for all ChildExited messages to be processed)
    sleep(Duration::from_secs(2)).await;

    // THEN: Verify 100% recovery rate
    let final_count = get_supervisor_status(&supervisor).await;
    assert_eq!(
        final_count.map_or(0, |v| v),
        child_count,
        "all tier-2 actors should be recovered after chaos (100% recovery rate)"
    );

    let kills = kill_count.load(Ordering::SeqCst);
    let _recoveries = recovery_count.load(Ordering::SeqCst);

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
    supervisor.stop(Some("Test complete".to_string()));
}

/// **Extended test**: Verify 100% recovery with metrics
#[tokio::test]
async fn given_extended_chaos_when_metrics_tracked_then_recovery_is_100_percent() {
    // GIVEN: A tier-1 supervisor with tier-2 children
    let mut config = SupervisorConfig::for_testing();
    config.max_restarts = 500;
    config.base_backoff_ms = 5;

    let supervisor_name = unique_name("chaos-tier2-metrics");
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
        .map(|i| format!("{supervisor_name}-metrics-{i}"))
        .collect();

    for child_name in &child_names {
        let child_args = SchedulerArguments::new();
        let _ = spawn_child(&supervisor, child_name, child_args).await;
    }

    sleep(Duration::from_millis(100)).await;

    // WHEN: Run chaos with detailed metrics
    let kill_rounds = 25; // Kill each child 25 times
    let _total_expected_kills = child_count * kill_rounds;
    let mut successful_kills = 0;

    for round in 0..kill_rounds {
        for child_name in &child_names {
            let stop_result = supervisor.cast(SupervisorMessage::StopChild {
                name: child_name.clone(),
            });

            if stop_result.is_ok() {
                successful_kills += 1;
            }

            sleep(Duration::from_millis(5)).await;
        }

        // Verify recovery between rounds
        sleep(Duration::from_millis(50)).await;
        let current_count = get_supervisor_status(&supervisor).await;
        assert_eq!(
            current_count.map_or(0, |v| v),
            child_count,
            "round {}: all children should recover (100% recovery)",
            round
        );
    }

    // THEN: Final verification - 100% recovery rate
    sleep(RECOVERY_WAIT).await;
    let final_count = get_supervisor_status(&supervisor).await;

    assert_eq!(
        final_count.map_or(0, |v| v),
        child_count,
        "after {} successful kills, all tier-2 actors should be recovered (100% rate)",
        successful_kills
    );

    assert!(
        is_actor_alive(supervisor.get_status()),
        "supervisor should survive all chaos rounds"
    );

    // Clean up
    supervisor.stop(Some("Metrics test complete".to_string()));
}
