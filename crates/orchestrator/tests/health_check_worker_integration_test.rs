#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
use std::sync::Arc;
use std::time::Duration;

use ractor::{Actor, ActorRef};

use oya_events::{EventBus, InMemoryEventStore};

use orchestrator::actors::health_check_worker::{
    HealthCheckConfig, HealthCheckMessage, HealthCheckWorkerDef,
};

#[tokio::test]
async fn test_health_check_worker_integration() -> Result<(), Box<dyn std::error::Error>> {
    // Setup test environment
    let config = HealthCheckConfig::for_testing();
    let event_bus = Some(Arc::new(EventBus::new(Arc::new(InMemoryEventStore::new()))));

    // Start the worker
    let (worker, handle): (ActorRef<HealthCheckMessage>, _) =
        Actor::spawn(None, HealthCheckWorkerDef, (config, event_bus)).await?;

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
