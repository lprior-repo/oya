//! Tier-1 supervisor startup tests.

#![deny(clippy::unwrap_used)]
#![deny(clippy::expect_used)]
#![deny(clippy::panic)]

use orchestrator::actors::supervisor::SupervisorConfig;
use orchestrator::supervision::{Tier1SupervisorKind, Tier1Supervisors, spawn_tier1_supervisors};
use ractor::ActorStatus;

fn build_prefix() -> String {
    let nanos = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_or(0u128, |duration| duration.as_nanos());
    format!("tier1-{}-{}", std::process::id(), nanos)
}

fn shutdown_all(supervisors: &Tier1Supervisors) {
    supervisors
        .storage
        .actor
        .stop(Some("test shutdown".to_string()));
    supervisors
        .workflow
        .actor
        .stop(Some("test shutdown".to_string()));
    supervisors
        .queue
        .actor
        .stop(Some("test shutdown".to_string()));
    supervisors
        .reconciler
        .actor
        .stop(Some("test shutdown".to_string()));
}

#[tokio::test]
async fn given_tier1_supervisors_when_spawned_then_all_running() {
    let prefix = build_prefix();
    let config = SupervisorConfig::for_testing();

    let spawn_result = spawn_tier1_supervisors(&prefix, config).await;
    assert!(spawn_result.is_ok(), "tier-1 supervisors should spawn");

    if let Ok(supervisors) = spawn_result {
        let storage_status = supervisors.storage.actor.get_status();
        let workflow_status = supervisors.workflow.actor.get_status();
        let queue_status = supervisors.queue.actor.get_status();
        let reconciler_status = supervisors.reconciler.actor.get_status();

        assert!(
            matches!(
                storage_status,
                ActorStatus::Starting | ActorStatus::Running | ActorStatus::Upgrading
            ),
            "storage supervisor should be running"
        );
        assert!(
            matches!(
                workflow_status,
                ActorStatus::Starting | ActorStatus::Running | ActorStatus::Upgrading
            ),
            "workflow supervisor should be running"
        );
        assert!(
            matches!(
                queue_status,
                ActorStatus::Starting | ActorStatus::Running | ActorStatus::Upgrading
            ),
            "queue supervisor should be running"
        );
        assert!(
            matches!(
                reconciler_status,
                ActorStatus::Starting | ActorStatus::Running | ActorStatus::Upgrading
            ),
            "reconciler supervisor should be running"
        );

        shutdown_all(&supervisors);
    }
}

#[tokio::test]
async fn given_tier1_supervisors_when_spawned_then_names_unique() {
    let prefix = build_prefix();
    let config = SupervisorConfig::for_testing();

    let spawn_result = spawn_tier1_supervisors(&prefix, config).await;
    assert!(spawn_result.is_ok(), "tier-1 supervisors should spawn");

    if let Ok(supervisors) = spawn_result {
        let names = [
            supervisors.storage.name.clone(),
            supervisors.workflow.name.clone(),
            supervisors.queue.name.clone(),
            supervisors.reconciler.name.clone(),
        ];

        let unique_count = names
            .iter()
            .fold(Vec::<String>::new(), |mut acc, name| {
                if !acc.iter().any(|existing: &String| existing == name) {
                    acc.push(name.clone());
                }
                acc
            })
            .len();

        assert_eq!(unique_count, 4, "supervisor names should be unique");

        shutdown_all(&supervisors);
    }
}

#[test]
fn given_tier1_kind_when_as_str_then_matches_expected() {
    let storage = Tier1SupervisorKind::Storage.as_str();
    let workflow = Tier1SupervisorKind::Workflow.as_str();
    let queue = Tier1SupervisorKind::Queue.as_str();
    let reconciler = Tier1SupervisorKind::Reconciler.as_str();

    assert_eq!(storage, "storage");
    assert_eq!(workflow, "workflow");
    assert_eq!(queue, "queue");
    assert_eq!(reconciler, "reconciler");
}

/// **Chaos Test**: Kill tier-1 supervisors sequentially and verify system stability
///
/// This test verifies that the system can handle sequential crashes of all
/// tier-1 supervisors (storage, workflow, queue, reconciler) without panicking.
/// Each supervisor is killed one at a time, with verification after each kill.
#[tokio::test]
async fn given_tier1_supervisors_when_killed_sequentially_then_system_stable() {
    // GIVEN: Spawn all tier-1 supervisors
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
            eprintln!("Failed to spawn tier-1 supervisors: {:?}", e);
            return;
        }
    };

    // Verify all supervisors are running
    let storage_status = supervisors.storage.actor.get_status();
    let workflow_status = supervisors.workflow.actor.get_status();
    let queue_status = supervisors.queue.actor.get_status();
    let reconciler_status = supervisors.reconciler.actor.get_status();

    assert!(
        matches!(
            storage_status,
            ActorStatus::Starting | ActorStatus::Running | ActorStatus::Upgrading
        ),
        "storage supervisor should be running initially"
    );
    assert!(
        matches!(
            workflow_status,
            ActorStatus::Starting | ActorStatus::Running | ActorStatus::Upgrading
        ),
        "workflow supervisor should be running initially"
    );
    assert!(
        matches!(
            queue_status,
            ActorStatus::Starting | ActorStatus::Running | ActorStatus::Upgrading
        ),
        "queue supervisor should be running initially"
    );
    assert!(
        matches!(
            reconciler_status,
            ActorStatus::Starting | ActorStatus::Running | ActorStatus::Upgrading
        ),
        "reconciler supervisor should be running initially"
    );

    // WHEN: Kill storage supervisor first
    supervisors
        .storage
        .actor
        .stop(Some("Sequential kill: storage".to_string()));
    tokio::time::sleep(std::time::Duration::from_millis(100)).await;

    // THEN: Verify storage is stopped
    let storage_status_after = supervisors.storage.actor.get_status();
    assert!(
        matches!(
            storage_status_after,
            ActorStatus::Stopping | ActorStatus::Stopped
        ),
        "storage supervisor should be stopped after kill"
    );

    // WHEN: Kill workflow supervisor second
    supervisors
        .workflow
        .actor
        .stop(Some("Sequential kill: workflow".to_string()));
    tokio::time::sleep(std::time::Duration::from_millis(100)).await;

    // THEN: Verify workflow is stopped
    let workflow_status_after = supervisors.workflow.actor.get_status();
    assert!(
        matches!(
            workflow_status_after,
            ActorStatus::Stopping | ActorStatus::Stopped
        ),
        "workflow supervisor should be stopped after kill"
    );

    // WHEN: Kill queue supervisor third
    supervisors
        .queue
        .actor
        .stop(Some("Sequential kill: queue".to_string()));
    tokio::time::sleep(std::time::Duration::from_millis(100)).await;

    // THEN: Verify queue is stopped
    let queue_status_after = supervisors.queue.actor.get_status();
    assert!(
        matches!(
            queue_status_after,
            ActorStatus::Stopping | ActorStatus::Stopped
        ),
        "queue supervisor should be stopped after kill"
    );

    // WHEN: Kill reconciler supervisor fourth
    supervisors
        .reconciler
        .actor
        .stop(Some("Sequential kill: reconciler".to_string()));
    tokio::time::sleep(std::time::Duration::from_millis(100)).await;

    // THEN: Verify reconciler is stopped
    let reconciler_status_after = supervisors.reconciler.actor.get_status();
    assert!(
        matches!(
            reconciler_status_after,
            ActorStatus::Stopping | ActorStatus::Stopped
        ),
        "reconciler supervisor should be stopped after kill"
    );

    // FINAL VERIFICATION: All supervisors should be stopped
    assert!(
        matches!(
            supervisors.storage.actor.get_status(),
            ActorStatus::Stopping | ActorStatus::Stopped
        ),
        "storage should remain stopped"
    );
    assert!(
        matches!(
            supervisors.workflow.actor.get_status(),
            ActorStatus::Stopping | ActorStatus::Stopped
        ),
        "workflow should remain stopped"
    );
    assert!(
        matches!(
            supervisors.queue.actor.get_status(),
            ActorStatus::Stopping | ActorStatus::Stopped
        ),
        "queue should remain stopped"
    );
    assert!(
        matches!(
            supervisors.reconciler.actor.get_status(),
            ActorStatus::Stopping | ActorStatus::Stopped
        ),
        "reconciler should remain stopped"
    );

    // If we reach here without panic, the system survived sequential chaos
}
