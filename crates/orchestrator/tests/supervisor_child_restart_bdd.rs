#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
//! BDD test: Supervisor child restart on failure.
//!
//! This test verifies that the supervisor correctly restarts child actors
//! when they fail, using the configured restart strategy.
//!
//! **Bead:** src-ojj9
//! **Scenario:** GIVEN supervisor WHEN child fails THEN restarted

use std::sync::atomic::{AtomicU64, Ordering};
use std::time::Duration;

use ractor::{ActorRef, ActorStatus};

use orchestrator::actors::messages::SchedulerMessage;
use orchestrator::actors::scheduler::{SchedulerActorDef, SchedulerArguments};
use orchestrator::actors::supervisor::{
    spawn_supervisor_with_name, MeltdownStatus, SupervisorArguments, SupervisorConfig,
    SupervisorMessage, SupervisorState,
};

static TEST_COUNTER: AtomicU64 = AtomicU64::new(0);

fn unique_name(prefix: &str) -> String {
    let id = TEST_COUNTER.fetch_add(1, Ordering::SeqCst);
    format!("{prefix}-{id}")
}

async fn await_scheduler_status(
    actor: &ActorRef<SchedulerMessage>,
    expected: ActorStatus,
    timeout_ms: u64,
) -> Result<(), String> {
    let deadline = tokio::time::Instant::now() + Duration::from_millis(timeout_ms);
    loop {
        if tokio::time::Instant::now() > deadline {
            return Err(format!(
                "Timeout waiting for actor status {expected:?}, got {:?}",
                actor.get_status()
            ));
        }
        if actor.get_status() == expected {
            return Ok(());
        }
        tokio::time::sleep(Duration::from_millis(10)).await;
    }
}

async fn await_supervisor_status(
    actor: &ActorRef<SupervisorMessage<SchedulerActorDef>>,
    expected: ActorStatus,
    timeout_ms: u64,
) -> Result<(), String> {
    let deadline = tokio::time::Instant::now() + Duration::from_millis(timeout_ms);
    loop {
        if tokio::time::Instant::now() > deadline {
            return Err(format!(
                "Timeout waiting for supervisor status {expected:?}, got {:?}",
                actor.get_status()
            ));
        }
        if actor.get_status() == expected {
            return Ok(());
        }
        tokio::time::sleep(Duration::from_millis(10)).await;
    }
}

async fn get_supervisor_status(
    supervisor: &ActorRef<SupervisorMessage<SchedulerActorDef>>,
) -> orchestrator::actors::supervisor::SupervisorStatus {
    let (tx, rx) = tokio::sync::oneshot::channel();
    let _ = supervisor.send_message(SupervisorMessage::GetStatus { reply: tx });
    rx.await
        .unwrap_or(orchestrator::actors::supervisor::SupervisorStatus {
            state: SupervisorState::Stopped,
            meltdown_status: MeltdownStatus::Normal,
            active_children: 0,
            total_restarts: 0,
            failures_in_window: 0,
        })
}

#[tokio::test]
async fn given_supervisor_with_child_when_child_fails_then_restarted(
) -> Result<(), Box<dyn std::error::Error>> {
    let test_name = unique_name("sup-child-restart");

    let args = SupervisorArguments::new().with_config(SupervisorConfig::for_testing());
    let supervisor = spawn_supervisor_with_name::<SchedulerActorDef>(args, &test_name)
        .await
        .map_err(|e| format!("Failed to spawn supervisor: {e}"))?;

    await_supervisor_status(&supervisor, ActorStatus::Running, 1000)
        .await
        .map_err(|e| format!("Supervisor not running: {e}"))?;

    let child_name = unique_name("child");
    let (spawn_tx, spawn_rx) = tokio::sync::oneshot::channel();
    supervisor.send_message(SupervisorMessage::SpawnChild {
        name: child_name.clone(),
        args: SchedulerArguments::default(),
        reply: spawn_tx,
    })?;

    spawn_rx.await??;

    let (get_tx, get_rx) = tokio::sync::oneshot::channel();
    supervisor.send_message(SupervisorMessage::GetChild {
        name: child_name.clone(),
        reply: get_tx,
    })?;

    let child_ref = get_rx
        .await?
        .ok_or_else(|| format!("Child {child_name} not found"))?;

    await_scheduler_status(&child_ref, ActorStatus::Running, 1000)
        .await
        .map_err(|e| format!("Child not running: {e}"))?;

    let initial_status = get_supervisor_status(&supervisor).await;
    assert_eq!(initial_status.active_children, 1);

    child_ref.stop(Some("Simulated failure".to_string()));

    tokio::time::sleep(Duration::from_millis(200)).await;

    let (get_tx2, get_rx2) = tokio::sync::oneshot::channel();
    supervisor.send_message(SupervisorMessage::GetChild {
        name: child_name.clone(),
        reply: get_tx2,
    })?;

    let restarted_ref = get_rx2.await?;

    let restarted_ref = restarted_ref.ok_or("Child should be restarted after failure")?;

    await_scheduler_status(&restarted_ref, ActorStatus::Running, 2000)
        .await
        .map_err(|e| format!("Restarted child not running: {e}"))?;

    let final_status = get_supervisor_status(&supervisor).await;
    assert!(
        final_status.total_restarts >= 1,
        "Expected at least 1 restart, got {}",
        final_status.total_restarts
    );

    supervisor.stop(None);
    Ok(())
}

#[tokio::test]
async fn given_supervisor_when_child_fails_multiple_times_then_restarts_each_time(
) -> Result<(), Box<dyn std::error::Error>> {
    let test_name = unique_name("sup-multi-restart");

    let args = SupervisorArguments::new().with_config(SupervisorConfig::for_testing());
    let supervisor = spawn_supervisor_with_name::<SchedulerActorDef>(args, &test_name)
        .await
        .map_err(|e| format!("Failed to spawn supervisor: {e}"))?;

    await_supervisor_status(&supervisor, ActorStatus::Running, 1000).await?;

    let child_name = unique_name("child");
    let (spawn_tx, spawn_rx) = tokio::sync::oneshot::channel();
    supervisor.send_message(SupervisorMessage::SpawnChild {
        name: child_name.clone(),
        args: SchedulerArguments::default(),
        reply: spawn_tx,
    })?;

    spawn_rx.await??;

    for i in 0..2 {
        let (get_tx, get_rx) = tokio::sync::oneshot::channel();
        supervisor.send_message(SupervisorMessage::GetChild {
            name: child_name.clone(),
            reply: get_tx,
        })?;

        let child_ref = get_rx
            .await?
            .ok_or_else(|| format!("Child {child_name} not found on iteration {i}"))?;

        await_scheduler_status(&child_ref, ActorStatus::Running, 2000)
            .await
            .map_err(|e| format!("Child not running on iteration {i}: {e}"))?;

        child_ref.stop(Some(format!("Simulated failure {i}")));

        tokio::time::sleep(Duration::from_millis(300)).await;
    }

    let (get_tx, get_rx) = tokio::sync::oneshot::channel();
    supervisor.send_message(SupervisorMessage::GetChild {
        name: child_name.clone(),
        reply: get_tx,
    })?;

    let final_ref = get_rx.await?;
    assert!(
        final_ref.is_some(),
        "Child should still be restarted after multiple failures"
    );

    let final_status = get_supervisor_status(&supervisor).await;
    assert!(
        final_status.total_restarts >= 2,
        "Expected at least 2 restarts, got {}",
        final_status.total_restarts
    );

    supervisor.stop(None);
    Ok(())
}

#[tokio::test]
async fn given_supervisor_with_max_restarts_when_exceeded_then_no_restart(
) -> Result<(), Box<dyn std::error::Error>> {
    let test_name = unique_name("sup-max-restarts");

    let config = SupervisorConfig {
        max_restarts: 1,
        ..SupervisorConfig::for_testing()
    };
    let args = SupervisorArguments::new().with_config(config);
    let supervisor = spawn_supervisor_with_name::<SchedulerActorDef>(args, &test_name)
        .await
        .map_err(|e| format!("Failed to spawn supervisor: {e}"))?;

    await_supervisor_status(&supervisor, ActorStatus::Running, 1000).await?;

    let child_name = unique_name("child");
    let (spawn_tx, spawn_rx) = tokio::sync::oneshot::channel();
    supervisor.send_message(SupervisorMessage::SpawnChild {
        name: child_name.clone(),
        args: SchedulerArguments::default(),
        reply: spawn_tx,
    })?;

    spawn_rx.await??;

    let (get_tx1, get_rx1) = tokio::sync::oneshot::channel();
    supervisor.send_message(SupervisorMessage::GetChild {
        name: child_name.clone(),
        reply: get_tx1,
    })?;
    let child_ref = get_rx1.await?.ok_or("Child not found 1")?;
    child_ref.stop(Some("Failure 1".to_string()));

    tokio::time::sleep(Duration::from_millis(300)).await;

    let (get_tx2, get_rx2) = tokio::sync::oneshot::channel();
    supervisor.send_message(SupervisorMessage::GetChild {
        name: child_name.clone(),
        reply: get_tx2,
    })?;
    let restarted_ref = get_rx2.await?.ok_or("Child not found 2")?;
    restarted_ref.stop(Some("Failure 2".to_string()));

    tokio::time::sleep(Duration::from_millis(300)).await;

    let (get_tx3, get_rx3) = tokio::sync::oneshot::channel();
    supervisor.send_message(SupervisorMessage::GetChild {
        name: child_name.clone(),
        reply: get_tx3,
    })?;
    let final_ref = get_rx3.await?;

    assert!(
        final_ref.is_none(),
        "Child should NOT be restarted after max_restarts exceeded"
    );

    let status = get_supervisor_status(&supervisor).await;
    assert_eq!(
        status.active_children, 0,
        "Should have no active children after max restarts"
    );

    supervisor.stop(None);
    Ok(())
}

#[tokio::test]
async fn given_supervisor_when_child_restarted_then_supervisor_remains_running(
) -> Result<(), Box<dyn std::error::Error>> {
    let test_name = unique_name("sup-invariant");

    let args = SupervisorArguments::new().with_config(SupervisorConfig::for_testing());
    let supervisor = spawn_supervisor_with_name::<SchedulerActorDef>(args, &test_name)
        .await
        .map_err(|e| format!("Failed to spawn supervisor: {e}"))?;

    await_supervisor_status(&supervisor, ActorStatus::Running, 1000).await?;

    let child_name = unique_name("child");
    let (spawn_tx, spawn_rx) = tokio::sync::oneshot::channel();
    supervisor.send_message(SupervisorMessage::SpawnChild {
        name: child_name.clone(),
        args: SchedulerArguments::default(),
        reply: spawn_tx,
    })?;

    spawn_rx.await??;

    let (get_tx, get_rx) = tokio::sync::oneshot::channel();
    supervisor.send_message(SupervisorMessage::GetChild {
        name: child_name.clone(),
        reply: get_tx,
    })?;
    let child_ref = get_rx.await?.ok_or("Child not found")?;

    child_ref.stop(Some("Failure".to_string()));

    tokio::time::sleep(Duration::from_millis(200)).await;

    assert_eq!(
        supervisor.get_status(),
        ActorStatus::Running,
        "Supervisor should remain running after child failure"
    );

    let status = get_supervisor_status(&supervisor).await;
    assert_eq!(
        status.state,
        SupervisorState::Running,
        "Supervisor state should be Running"
    );

    supervisor.stop(None);
    Ok(())
}
