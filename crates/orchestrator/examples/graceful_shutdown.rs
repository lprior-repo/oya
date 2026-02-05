//! Example demonstrating graceful shutdown with checkpoint coordination.
//!
//! Run with: cargo run --example graceful_shutdown
//!
//! Press Ctrl+C to trigger graceful shutdown.

use std::sync::Arc;
use std::time::Duration;

use orchestrator::scheduler::{QueueActorRef, QueueType, SchedulerActor};
use orchestrator::shutdown::{
    CheckpointResult, ShutdownCoordinator, ShutdownSignal, install_signal_handlers,
};
use tracing::{Level, info};
use tracing_subscriber::FmtSubscriber;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize tracing
    let subscriber = FmtSubscriber::builder()
        .with_max_level(Level::INFO)
        .finish();
    tracing::subscriber::set_global_default(subscriber)?;

    info!("Starting orchestrator with graceful shutdown support");

    // Create shutdown coordinator
    let coordinator = Arc::new(ShutdownCoordinator::new());

    // Install signal handlers
    let signal_handle = install_signal_handlers(Arc::clone(&coordinator)).await?;

    // Create scheduler actor
    let mut scheduler = SchedulerActor::new();

    // Register some test workflows
    scheduler.register_workflow("workflow-1".to_string())?;
    scheduler.register_workflow("workflow-2".to_string())?;

    // Add some queue references
    scheduler.add_queue_ref(QueueActorRef::new("queue-1".to_string(), QueueType::FIFO));
    scheduler.add_queue_ref(QueueActorRef::new(
        "queue-2".to_string(),
        QueueType::Priority,
    ));

    info!(
        workflows = scheduler.workflow_count(),
        queues = scheduler.get_queue_refs().len(),
        "Scheduler initialized"
    );

    // Subscribe to shutdown notifications
    let mut shutdown_rx = coordinator.subscribe();
    let checkpoint_tx = coordinator.checkpoint_sender();

    // Spawn a task that listens for shutdown and saves checkpoints
    let scheduler_clone = scheduler.clone();
    let checkpoint_handler = tokio::spawn(async move {
        match shutdown_rx.recv().await {
            Ok(signal) => {
                info!(signal = %signal, "Checkpoint handler received shutdown signal");

                // Simulate checkpoint saving
                let start = std::time::Instant::now();

                // Save scheduler state checkpoint
                let stats = scheduler_clone.stats();
                info!(
                    workflows = stats.workflow_count,
                    pending = stats.pending_count,
                    ready = stats.ready_count,
                    "Saving scheduler checkpoint"
                );

                // Simulate some work
                tokio::time::sleep(Duration::from_millis(500)).await;

                let duration_ms = start.elapsed().as_millis() as u64;
                let result = CheckpointResult::success("scheduler", duration_ms);

                if let Err(e) = checkpoint_tx.send(result).await {
                    eprintln!("Failed to send checkpoint result: {}", e);
                }

                info!(duration_ms = duration_ms, "Scheduler checkpoint saved");
            }
            Err(e) => {
                eprintln!("Shutdown receiver error: {}", e);
            }
        }
    });

    // Simulate some work
    info!("Running orchestrator... (Press Ctrl+C to shutdown)");

    // Wait for shutdown signal
    tokio::select! {
        _ = tokio::signal::ctrl_c() => {
            info!("Received Ctrl+C, initiating shutdown");
            coordinator.initiate_shutdown(ShutdownSignal::Programmatic).await?;
        }
        _ = signal_handle => {
            info!("Signal handler completed");
        }
    }

    // Execute graceful shutdown
    info!("Executing graceful shutdown sequence");
    let stats = coordinator.wait_with_timeout().await?;

    info!(
        checkpoints_saved = stats.checkpoints_saved,
        checkpoints_failed = stats.checkpoints_failed,
        duration_ms = stats.total_duration_ms,
        "Graceful shutdown complete"
    );

    // Wait for checkpoint handler to complete
    checkpoint_handler.await?;

    Ok(())
}
