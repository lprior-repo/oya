use std::time::Duration;
use std::sync::Arc;

use ractor::Actor;
use oya_events::{EventBus, InMemoryEventStore, EventStore};

use orchestrator::actors::health_check_worker::{HealthCheckConfig, HealthCheckMessage, HealthCheckWorkerDef};

/// Helper function to create a test event store and event bus
fn create_test_event_infrastructure() -> (Arc<InMemoryEventStore>, Arc<EventBus>) {
    let event_store = Arc::new(InMemoryEventStore::new());
    let event_bus = Arc::new(EventBus::new(event_store.clone()));
    (event_store, event_bus)
}

#[tokio::test]
async fn test_health_check_worker_starts_and_stops_cleanly() -> Result<(), Box<dyn std::error::Error>> {
    // Given: A health check worker with test configuration
    let (_event_store, event_bus) = create_test_event_infrastructure();
    let config = HealthCheckConfig::for_testing();
    
    // When: The worker is started
    let (worker, handle) = Actor::spawn(None, HealthCheckWorkerDef, (config, Some(event_bus))).await?;
    
    // And: A health check message is sent
    worker.send_message(HealthCheckMessage::PerformCheck)?;
    
    // And: We wait for the worker to process the check
    tokio::time::sleep(Duration::from_millis(200)).await;
    
    // And: The worker is stopped
    worker.stop(None);
    handle.await?;
    
    // Then: The worker shuts down without errors
    Ok(())
}

#[tokio::test]
async fn test_health_check_worker_emits_event_when_becoming_unhealthy() -> Result<(), Box<dyn std::error::Error>> {
    // Given: A health check worker configured to emit events
    let (event_store, event_bus) = create_test_event_infrastructure();
    
    // The worker will fail to connect (no server running), so it will emit an "unhealthy" event
    let config = HealthCheckConfig::for_testing()
        .with_check_interval(Duration::from_millis(100))
        .with_timeout(Duration::from_millis(50))
        .with_emit_events(true);
    
    // When: The worker is started
    let (worker, handle) = Actor::spawn(None, HealthCheckWorkerDef, (config, Some(event_bus))).await?;
    
    // And: We wait for health checks to occur and status to change to unhealthy
    // After max_failures (2) consecutive failures, it should emit an event
    tokio::time::sleep(Duration::from_millis(300)).await;
    
    // And: The worker is stopped
    worker.stop(None);
    handle.await?;
    
    // Then: At least one health status change event was emitted
    let events = event_store.read(None).await?;
    assert!(events.len() >= 1, 
            "Expected at least 1 health status change event, got {}", events.len());
    
    // And: The event is related to worker health
    let event_type = events[0].event_type();
    assert!(event_type.contains("worker") || event_type.contains("unhealthy"), 
            "Expected worker health event, got event type: {}", event_type);
    
    Ok(())
}