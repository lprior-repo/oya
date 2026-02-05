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
        .map(|duration| duration.as_nanos())
        .unwrap_or_default();
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
