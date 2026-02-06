//! Tests for BeadEvent emission in worker-actor

use oya_events::{BeadEvent, BeadState, EventBus, InMemoryEventStore};
use oya_orchestrator::actors::worker::{WorkerActorDef, WorkerConfig, WorkerMessage};
use std::sync::Arc;

// ==========================================================================
// BeadEvent Emission Tests
// ==========================================================================

#[test]
fn test_worker_config_with_event_bus() {
    let store = Arc::new(InMemoryEventStore::new());
    let bus = Arc::new(EventBus::new(store));
    let config = WorkerConfig::default().with_event_bus(bus);

    assert!(config.event_bus.is_some(), "EventBus should be configured");
}

#[tokio::test]
async fn test_start_bead_emits_claimed_event() {
    let store = Arc::new(InMemoryEventStore::new());
    let bus = Arc::new(EventBus::new(store.clone()));
    let config = WorkerConfig::default().with_event_bus(bus.clone());

    let (actor, _handle) = ractor::Actor::spawn(None, WorkerActorDef, config)
        .await
        .expect("Failed to spawn worker actor");

    // Subscribe to events
    let mut sub = bus.subscribe();

    // Start a bead
    let bead_id = "test-bead-123".to_string();
    actor
        .send_message(WorkerMessage::StartBead {
            bead_id: bead_id.clone(),
            from_state: None,
        })
        .expect("Failed to send StartBead message");

    // Give time for event to be published
    tokio::time::sleep(tokio::time::Duration::from_millis(50)).await;

    // Verify Claimed event was emitted
    let received = sub.try_recv();
    assert!(received.is_ok(), "Should receive Claimed event");

    let event = received.unwrap();
    assert_eq!(event.event_type(), "claimed");
    assert_eq!(event.bead_id().0, bead_id);

    actor.stop(None);
    _handle.await.expect("Failed to stop actor");
}

#[tokio::test]
async fn test_fail_bead_emits_failed_event() {
    let store = Arc::new(InMemoryEventStore::new());
    let bus = Arc::new(EventBus::new(store.clone()));
    let config = WorkerConfig::default().with_event_bus(bus.clone());

    let (actor, _handle) = ractor::Actor::spawn(None, WorkerActorDef, config)
        .await
        .expect("Failed to spawn worker actor");

    // Start a bead first
    let bead_id = "test-bead-456".to_string();
    actor
        .send_message(WorkerMessage::StartBead {
            bead_id: bead_id.clone(),
            from_state: None,
        })
        .expect("Failed to send StartBead message");

    tokio::time::sleep(tokio::time::Duration::from_millis(10)).await;

    // Subscribe to events
    let mut sub = bus.subscribe();

    // Fail the bead
    actor
        .send_message(WorkerMessage::FailBead {
            error: "Test failure".to_string(),
        })
        .expect("Failed to send FailBead message");

    // Give time for event to be published
    tokio::time::sleep(tokio::time::Duration::from_millis(50)).await;

    // Verify Failed event was emitted
    let received = sub.try_recv();
    assert!(received.is_ok(), "Should receive Failed event");

    let event = received.unwrap();
    assert_eq!(event.event_type(), "failed");
    assert_eq!(event.bead_id().0, bead_id);

    actor.stop(None);
    _handle.await.expect("Failed to stop actor");
}

#[tokio::test]
async fn test_stop_emits_unclaimed_event() {
    let store = Arc::new(InMemoryEventStore::new());
    let bus = Arc::new(EventBus::new(store.clone()));
    let config = WorkerConfig::default().with_event_bus(bus.clone());

    let (actor, _handle) = ractor::Actor::spawn(None, WorkerActorDef, config)
        .await
        .expect("Failed to spawn worker actor");

    // Start a bead first
    let bead_id = "test-bead-789".to_string();
    actor
        .send_message(WorkerMessage::StartBead {
            bead_id: bead_id.clone(),
            from_state: None,
        })
        .expect("Failed to send StartBead message");

    tokio::time::sleep(tokio::time::Duration::from_millis(10)).await;

    // Subscribe to events
    let mut sub = bus.subscribe();

    // Stop the worker
    actor
        .send_message(WorkerMessage::Stop {
            reason: Some("test shutdown".to_string()),
        })
        .expect("Failed to send Stop message");

    // Give time for event to be published
    tokio::time::sleep(tokio::time::Duration::from_millis(50)).await;

    // Verify Unclaimed event was emitted
    let received = sub.try_recv();
    assert!(received.is_ok(), "Should receive Unclaimed event");

    let event = received.unwrap();
    assert_eq!(event.event_type(), "unclaimed");
    assert_eq!(event.bead_id().0, bead_id);

    actor.stop(None);
    _handle.await.expect("Failed to stop actor");
}

#[tokio::test]
async fn test_event_bus_none_does_not_panic() {
    // Worker should function normally without an event bus
    let config = WorkerConfig::default(); // No event bus configured

    let (actor, _handle) = ractor::Actor::spawn(None, WorkerActorDef, config)
        .await
        .expect("Failed to spawn worker actor");

    // Send messages - should not panic
    actor
        .send_message(WorkerMessage::StartBead {
            bead_id: "test-bead-no-bus".to_string(),
            from_state: None,
        })
        .expect("Failed to send StartBead message");

    actor
        .send_message(WorkerMessage::FailBead {
            error: "Test error".to_string(),
        })
        .expect("Failed to send FailBead message");

    tokio::time::sleep(tokio::time::Duration::from_millis(10)).await;

    actor.stop(None);
    _handle.await.expect("Failed to stop actor");
}
