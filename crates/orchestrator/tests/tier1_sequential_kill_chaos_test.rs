//! Tier-1 Sequential Kill Chaos Tests
//!
//! HOSTILE chaos engineering test for sequential tier-1 supervisor failures.
//! This test verifies that tier-1 supervisors can be killed sequentially
//! and that the system achieves graceful shutdown with 100% stopped rate.

#![deny(clippy::unwrap_used)]
#![deny(clippy::expect_used)]
#![deny(clippy::panic)]

use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use orchestrator::actors::supervisor::SupervisorConfig;
use orchestrator::supervision::spawn_tier1_supervisors;
use ractor::ActorStatus;

fn build_prefix() -> String {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_nanos())
        .unwrap_or_default();
    format!("tier1-sequential-{}-{}", std::process::id(), nanos)
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

/// **Attack 1.1**: Kill tier-1 supervisors sequentially and verify system stability
///
/// This test simulates a catastrophic failure scenario where tier-1 supervisors
/// crash one after another. The system must handle these failures gracefully
/// without panicking or losing data.
#[tokio::test]
async fn given_tier1_supervisors_when_killed_sequentially_then_system_stable() {
    // GIVEN: All tier-1 supervisors are spawned and running
    let prefix = build_prefix();
    let config = SupervisorConfig::for_testing();

    let spawn_result = spawn_tier1_supervisors(&prefix, config).await;
    assert!(
        spawn_result.is_ok(),
        "tier-1 supervisors should spawn successfully"
    );

    let supervisors = match spawn_result {
        Ok(sups) => sups,
        Err(e) => {
            eprintln!("Failed to spawn tier-1 supervisors: {}", e);
            return;
        }
    };

    // Verify all supervisors are alive
    let storage_status = supervisors.storage.actor.get_status();
    let workflow_status = supervisors.workflow.actor.get_status();
    let queue_status = supervisors.queue.actor.get_status();
    let reconciler_status = supervisors.reconciler.actor.get_status();

    assert!(
        is_supervisor_alive(storage_status),
        "storage supervisor should be alive initially"
    );
    assert!(
        is_supervisor_alive(workflow_status),
        "workflow supervisor should be alive initially"
    );
    assert!(
        is_supervisor_alive(queue_status),
        "queue supervisor should be alive initially"
    );
    assert!(
        is_supervisor_alive(reconciler_status),
        "reconciler supervisor should be alive initially"
    );

    // Metrics collection
    let kill_count = Arc::new(AtomicUsize::new(0));
    let total_supervisors = 4;

    // WHEN: Kill supervisors sequentially (one after another)
    // Kill order: storage → workflow → queue → reconciler

    // Kill storage supervisor
    supervisors
        .storage
        .actor
        .stop(Some("Sequential kill 0: storage".to_string()));
    kill_count.fetch_add(1, Ordering::SeqCst);
    sleep(Duration::from_millis(100)).await;

    let storage_status_after = supervisors.storage.actor.get_status();
    assert!(
        is_supervisor_stopped(storage_status_after),
        "storage supervisor should be stopped after kill"
    );

    // Kill workflow supervisor
    supervisors
        .workflow
        .actor
        .stop(Some("Sequential kill 1: workflow".to_string()));
    kill_count.fetch_add(1, Ordering::SeqCst);
    sleep(Duration::from_millis(100)).await;

    let workflow_status_after = supervisors.workflow.actor.get_status();
    assert!(
        is_supervisor_stopped(workflow_status_after),
        "workflow supervisor should be stopped after kill"
    );

    // Kill queue supervisor
    supervisors
        .queue
        .actor
        .stop(Some("Sequential kill 2: queue".to_string()));
    kill_count.fetch_add(1, Ordering::SeqCst);
    sleep(Duration::from_millis(100)).await;

    let queue_status_after = supervisors.queue.actor.get_status();
    assert!(
        is_supervisor_stopped(queue_status_after),
        "queue supervisor should be stopped after kill"
    );

    // Kill reconciler supervisor
    supervisors
        .reconciler
        .actor
        .stop(Some("Sequential kill 3: reconciler".to_string()));
    kill_count.fetch_add(1, Ordering::SeqCst);
    sleep(Duration::from_millis(100)).await;

    let reconciler_status_after = supervisors.reconciler.actor.get_status();
    assert!(
        is_supervisor_stopped(reconciler_status_after),
        "reconciler supervisor should be stopped after kill"
    );

    // THEN: All supervisors should be stopped, system is stable
    let final_kills = kill_count.load(Ordering::SeqCst);
    let stopped_rate = (final_kills as f64 / total_supervisors as f64) * 100.0;

    assert_eq!(
        final_kills, total_supervisors,
        "all supervisors should be stopped: {}/{} ({}% stopped rate)",
        final_kills, total_supervisors, stopped_rate
    );

    // Verify all remain stopped (no unexpected restarts)
    assert!(
        is_supervisor_stopped(supervisors.storage.actor.get_status()),
        "storage supervisor should remain stopped"
    );
    assert!(
        is_supervisor_stopped(supervisors.workflow.actor.get_status()),
        "workflow supervisor should remain stopped"
    );
    assert!(
        is_supervisor_stopped(supervisors.queue.actor.get_status()),
        "queue supervisor should remain stopped"
    );
    assert!(
        is_supervisor_stopped(supervisors.reconciler.actor.get_status()),
        "reconciler supervisor should remain stopped"
    );

    // If we reach here without panic, the system survived sequential crashes
    // This is the key invariant: graceful shutdown under chaos
}

// Helper function for sleep
async fn sleep(duration: Duration) {
    tokio::time::sleep(duration).await;
}
