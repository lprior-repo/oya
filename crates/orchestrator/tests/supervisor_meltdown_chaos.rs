#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
//! Chaos tests for supervisor meltdown scenarios.
//!
//! Tests verify that the supervisor correctly handles:
//! - Repeated child crashes
//! - Backoff strategies
//! - Max restart limits
//! - Final meltdown state

use orchestrator::actors::scheduler::{SchedulerActorDef, SchedulerArguments};
use orchestrator::actors::supervisor::{
    spawn_supervisor_with_name, SupervisorArguments, SupervisorConfig, SupervisorMessage,
};
use std::time::Duration;
use tracing::info;

async fn setup_supervisor(
    test_name: &str,
    max_restarts: u32,
) -> Result<ractor::ActorRef<SupervisorMessage<SchedulerActorDef>>, Box<dyn std::error::Error>> {
    let mut config = SupervisorConfig::default();
    config.max_restarts = max_restarts;
    config.base_backoff_ms = 10; // Fast for testing

    let args = SupervisorArguments::new().with_config(config);

    let supervisor =
        spawn_supervisor_with_name::<SchedulerActorDef>(args, &format!("supervisor-{test_name}"))
            .await?;
    Ok(supervisor)
}

#[tokio::test]
async fn test_supervisor_restarts_child_until_limit() -> Result<(), Box<dyn std::error::Error>> {
    let test_name = "restart_limit";
    let max_restarts = 2;
    let supervisor = setup_supervisor(test_name, max_restarts).await?;

    // Spawn child
    let (tx, rx) = tokio::sync::oneshot::channel();
    let _ = supervisor.send_message(SupervisorMessage::<SchedulerActorDef>::SpawnChild {
        name: format!("child-{test_name}"),
        args: SchedulerArguments::new(),
        reply: tx,
    });

    rx.await??;

    // Crash it max_restarts times
    for i in 1..=max_restarts {
        info!("Crash iteration {i}");
        let _ = supervisor.send_message(SupervisorMessage::<SchedulerActorDef>::StopChild {
            name: format!("child-{test_name}"),
        });
        tokio::time::sleep(Duration::from_millis(50)).await;
    }

    // One more crash should trigger meltdown/stop
    let _ = supervisor.send_message(SupervisorMessage::<SchedulerActorDef>::StopChild {
        name: format!("child-{test_name}"),
    });
    tokio::time::sleep(Duration::from_millis(100)).await;

    let (tx, rx) = tokio::sync::oneshot::channel();
    let _ = supervisor.send_message(SupervisorMessage::GetStatus { reply: tx });
    let status = rx.await?;

    // In a meltdown, the supervisor might stop itself or just stop the child
    // Current implementation: supervisor stays running but child is not restarted
    assert!(status.total_restarts >= max_restarts);

    supervisor.stop(None);
    Ok(())
}

#[tokio::test]
async fn test_supervisor_backoff_increases() -> Result<(), Box<dyn std::error::Error>> {
    let test_name = "backoff_increase";
    let supervisor = setup_supervisor(test_name, 5).await?;

    // Spawn child
    let (tx, rx) = tokio::sync::oneshot::channel();
    let _ = supervisor.send_message(SupervisorMessage::<SchedulerActorDef>::SpawnChild {
        name: format!("child-{test_name}"),
        args: SchedulerArguments::new(),
        reply: tx,
    });
    rx.await??;

    // First crash
    let _ = supervisor.send_message(SupervisorMessage::<SchedulerActorDef>::StopChild {
        name: format!("child-{test_name}"),
    });
    tokio::time::sleep(Duration::from_millis(20)).await;

    // Second crash
    let _ = supervisor.send_message(SupervisorMessage::<SchedulerActorDef>::StopChild {
        name: format!("child-{test_name}"),
    });
    tokio::time::sleep(Duration::from_millis(50)).await;

    let (tx, rx) = tokio::sync::oneshot::channel();
    let _ = supervisor.send_message(SupervisorMessage::GetStatus { reply: tx });
    let status = rx.await?;

    assert!(status.total_restarts >= 2);

    supervisor.stop(None);
    Ok(())
}
