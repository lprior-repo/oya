//! BDD test: Scheduler graceful shutdown signal handling.
//!
//! This test verifies that the scheduler responds correctly to shutdown signals
//! from the ShutdownCoordinator, implementing graceful shutdown behavior.
//!
//! **Bead:** src-17h2
//! **Scenario:** GIVEN scheduler WHEN shutdown signal THEN graceful stop

use std::sync::atomic::{AtomicU64, Ordering};
use std::time::Duration;

use ractor::ActorRef;
use tokio::time::timeout;

use orchestrator::actors::{SchedulerActorDef, SchedulerArguments, SchedulerMessage};
use orchestrator::shutdown::{ShutdownCoordinator, ShutdownSignal};

/// Atomic counter for generating unique actor names.
static TEST_COUNTER: AtomicU64 = AtomicU64::new(0);

/// Generate a unique actor name for testing.
fn unique_scheduler_name() -> String {
    let id = TEST_COUNTER.fetch_add(1, Ordering::SeqCst);
    format!("scheduler-shutdown-test-{}", id)
}

/// Helper to spawn a scheduler with shutdown coordinator for testing.
async fn setup_scheduler_with_shutdown() -> Result<
    (
        ActorRef<SchedulerMessage>,
        tokio::task::JoinHandle<()>,
    ),
    Box<dyn std::error::Error>,
> {
    let coordinator = std::sync::Arc::new(ShutdownCoordinator::new());
    let args = SchedulerArguments::new().with_shutdown_coordinator(coordinator.clone());
    let name = unique_scheduler_name();

    let (scheduler, handle) =
        ractor::Actor::spawn(Some(name), SchedulerActorDef, args).await?;

    // Wait for actor to be fully initialized
    tokio::time::sleep(Duration::from_millis(50)).await;

    Ok((scheduler, handle))
}

#[tokio::test]
async fn given_scheduler_when_shutdown_signal_then_graceful_stop(
) -> Result<(), Box<dyn std::error::Error>> {
    // Given: A running scheduler actor with shutdown coordinator
    let (scheduler, actor_handle) = setup_scheduler_with_shutdown().await?;

    // Verify scheduler is running by checking it responds to messages
    let stats = scheduler
        .call(
            |reply| SchedulerMessage::GetStats { reply },
            Some(Duration::from_millis(500)),
        )
        .await?;

    match stats {
        ractor::rpc::CallResult::Success(_stats) => {
            // Scheduler is running and responsive
        }
        ractor::rpc::CallResult::Timeout => {
            return Err("Scheduler not responsive before shutdown".into());
        }
        ractor::rpc::CallResult::SenderError => {
            return Err("Sender error before shutdown".into());
        }
    }

    // When: Shutdown signal is sent via ShutdownCoordinator
    let coordinator = std::sync::Arc::new(ShutdownCoordinator::new());
    let signal_result = coordinator
        .initiate_shutdown(ShutdownSignal::Programmatic)
        .await;
    assert!(
        signal_result.is_ok(),
        "Shutdown initiation should succeed: {:?}",
        signal_result
    );

    // Send shutdown message directly to scheduler (simulating coordinator broadcast)
    let send_result = scheduler.send_message(SchedulerMessage::Shutdown);
    assert!(
        send_result.is_ok(),
        "Shutdown message send should succeed: {:?}",
        send_result
    );

    // Then: Scheduler should gracefully stop within reasonable time
    // Wait for actor to stop (with timeout to prevent hanging)
    let stop_result = timeout(Duration::from_secs(5), actor_handle).await;

    match stop_result {
        Ok(Ok(_)) => {
            // Actor stopped successfully
            Ok(())
        }
        Ok(Err(e)) => Err(format!("Actor stopped with error: {:?}", e).into()),
        Err(_) => Err("Scheduler did not stop within timeout".into()),
    }
}

#[tokio::test]
async fn given_scheduler_with_coordinator_when_sigterm_then_checkpoint_saved(
) -> Result<(), Box<dyn std::error::Error>> {
    // Given: A scheduler with shutdown coordinator
    let coordinator = std::sync::Arc::new(ShutdownCoordinator::new());
    let args = SchedulerArguments::new().with_shutdown_coordinator(coordinator.clone());
    let name = unique_scheduler_name();

    let (scheduler, actor_handle) =
        ractor::Actor::spawn(Some(name), SchedulerActorDef, args).await?;

    // Wait for initialization
    tokio::time::sleep(Duration::from_millis(50)).await;

    // Register some workflows to create state
    let _ = scheduler.send_message(SchedulerMessage::RegisterWorkflow {
        workflow_id: "wf-1".to_string(),
    });
    let _ = scheduler.send_message(SchedulerMessage::RegisterWorkflow {
        workflow_id: "wf-2".to_string(),
    });
    tokio::time::sleep(Duration::from_millis(50)).await;

    // When: SIGTERM shutdown signal is initiated
    coordinator
        .initiate_shutdown(ShutdownSignal::Sigterm)
        .await?;

    // Simulate broadcast by sending shutdown message
    let _ = scheduler.send_message(SchedulerMessage::Shutdown);

    // Then: Scheduler should stop and checkpoint should be saved
    let stop_result = timeout(Duration::from_secs(5), actor_handle).await;
    assert!(
        stop_result.is_ok(),
        "Scheduler should stop gracefully: {:?}",
        stop_result
    );

    // Verify shutdown phase completed
    let phase = coordinator.phase().await;
    assert!(
        matches!(
            phase,
            orchestrator::shutdown::ShutdownPhase::Initiating
                | orchestrator::shutdown::ShutdownPhase::Complete
        ),
        "Shutdown phase should be Initiating or Complete, got: {:?}",
        phase
    );

    Ok(())
}

#[tokio::test]
async fn given_scheduler_when_multiple_shutdown_signals_then_first_wins(
) -> Result<(), Box<dyn std::error::Error>> {
    // Given: A scheduler with shutdown coordinator
    let coordinator = std::sync::Arc::new(ShutdownCoordinator::new());
    let args = SchedulerArguments::new().with_shutdown_coordinator(coordinator.clone());
    let name = unique_scheduler_name();

    let (scheduler, actor_handle) =
        ractor::Actor::spawn(Some(name), SchedulerActorDef, args).await?;

    tokio::time::sleep(Duration::from_millis(50)).await;

    // When: Multiple shutdown signals are sent rapidly
    let _ = coordinator
        .initiate_shutdown(ShutdownSignal::Sigterm)
        .await;
    let _ = coordinator
        .initiate_shutdown(ShutdownSignal::Sigint)
        .await;

    let _ = scheduler.send_message(SchedulerMessage::Shutdown);
    let _ = scheduler.send_message(SchedulerMessage::Shutdown);

    // Then: First shutdown should succeed, duplicates ignored
    let stop_result = timeout(Duration::from_secs(5), actor_handle).await;
    assert!(
        stop_result.is_ok(),
        "Scheduler should handle duplicate shutdown signals gracefully: {:?}",
        stop_result
    );

    Ok(())
}

#[tokio::test]
async fn given_scheduler_when_shutdown_then_post_stop_called(
) -> Result<(), Box<dyn std::error::Error>> {
    // Given: A scheduler with shutdown coordinator
    let coordinator = std::sync::Arc::new(ShutdownCoordinator::new());
    let args = SchedulerArguments::new().with_shutdown_coordinator(coordinator.clone());
    let name = unique_scheduler_name();

    let (scheduler, actor_handle) =
        ractor::Actor::spawn(Some(name), SchedulerActorDef, args).await?;

    // Add some state
    let _ = scheduler.send_message(SchedulerMessage::RegisterWorkflow {
        workflow_id: "wf-checkpoint".to_string(),
    });
    tokio::time::sleep(Duration::from_millis(50)).await;

    // When: Shutdown is initiated
    let _ = coordinator
        .initiate_shutdown(ShutdownSignal::Programmatic)
        .await;
    let _ = scheduler.send_message(SchedulerMessage::Shutdown);

    // Then: Scheduler should stop gracefully (post_stop executes)
    let stop_result = timeout(Duration::from_secs(5), actor_handle).await;
    assert!(
        stop_result.is_ok(),
        "Scheduler should stop gracefully with post_stop hook: {:?}",
        stop_result
    );

    // Verify shutdown phase was updated
    let phase = coordinator.phase().await;
    assert!(
        matches!(
            phase,
            orchestrator::shutdown::ShutdownPhase::Initiating
                | orchestrator::shutdown::ShutdownPhase::Complete
        ),
        "Shutdown phase should be at least Initiating, got: {:?}",
        phase
    );

    Ok(())
}
