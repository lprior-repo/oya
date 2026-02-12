use std::sync::Arc;
use std::time::Duration;

use ractor::{Actor, ActorProcessingErr, ActorRef};
use tokio::sync::watch;
use tracing::debug;

use oya_events::{BeadEvent, EventBus};

use crate::actors::health_check_worker::{
    HealthCheckConfig, HealthCheckMessage, HealthCheckWorkerDef, HealthCheckWorkerState,
    HealthStatus,
};

#[tokio::test]
async fn test_health_check_worker_integration() -> Result<(), Box<dyn std::error::Error>> {
    // Setup test environment
    let config = HealthCheckConfig::for_testing();
    let event_bus = Some(Arc::new(EventBus::new()));

    // Start the worker
    let (worker, handle) = Actor::spawn(None, HealthCheckWorkerDef, (config, event_bus)).await?;

    // Send a health check message
    worker.send_message(HealthCheckMessage::PerformCheck)?;

    // Wait for the worker to process the check
    tokio::time::sleep(Duration::from_millis(200)).await;

    // Stop the worker
    worker.stop(None);
    handle.await?;

    // Test passes if we reach this point without errors
    Ok(())
}
