use std::time::Duration;
use std::sync::Arc;

use ractor::Actor;
use oya_events::{EventBus, InMemoryEventStore, EventStore};

use orchestrator::actors::health_check_worker::{HealthCheckConfig, HealthCheckMessage, HealthCheckWorkerDef};

#[tokio::test]
async fn test_health_check_worker_starts_and_stops() -> Result<(), Box<dyn std::error::Error>> {
    // Setup test environment with in-memory event store
    let event_store = Arc::new(InMemoryEventStore::new());
    let event_bus = Arc::new(EventBus::new(event_store));
    
    let config = HealthCheckConfig::for_testing();
    
    // Start the worker
    let (worker, handle) = Actor::spawn(None, HealthCheckWorkerDef, (config, Some(event_bus))).await?;
    
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

#[tokio::test]
async fn test_health_check_worker_emits_event_on_status_change() -> Result<(), Box<dyn std::error::Error>> {
    // Setup test environment with in-memory event store
    let event_store = Arc::new(InMemoryEventStore::new());
    let event_bus = Arc::new(EventBus::new(event_store.clone()));
    
    // Configure worker to check every 100ms and emit events
    // The worker will fail to connect (no server running), so it will emit an "unhealthy" event
    let config = HealthCheckConfig::for_testing()
        .with_check_interval(Duration::from_millis(100))
        .with_timeout(Duration::from_millis(50))
        .with_emit_events(true);
    
    // Start the worker
    let (worker, handle) = Actor::spawn(None, HealthCheckWorkerDef, (config, Some(event_bus))).await?;
    
    // Wait for health checks to occur and status to change to unhealthy
    // After max_failures (2) consecutive failures, it should emit an event
    tokio::time::sleep(Duration::from_millis(300)).await;
    
    // Stop the worker
    worker.stop(None);
    handle.await?;
    
    // Verify that at least one status change event was emitted
    let events = event_store.read(None).await?;
    
    // We expect at least 1 health status change event (Unknown -> Unhealthy)
    assert!(events.len() >= 1, "Expected at least 1 health status change event, got {}", events.len());
    
    // Verify that the event is related to worker health
    let event_type = events[0].event_type();
    assert!(event_type.contains("worker") || event_type.contains("health"), 
            "Expected worker health event, got event type: {}", event_type);
    
    Ok(())
}