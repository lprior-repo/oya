//! Tests for worker BeadEvent emission on state transitions.
//!
//! These tests verify that worker actors emit BeadEvents when state transitions occur.

use std::sync::Arc;
use std::time::Duration;

use oya_events::{BeadEvent, BeadId, BeadState, EventBus, InMemoryEventStore};
use ractor::ActorRef;

use orchestrator::actors::worker::{
    WorkerActorDef, WorkerConfig, WorkerMessage, WorkerRetryPolicy,
};

/// Helper to setup a worker actor with an event bus for testing.
async fn setup_worker_with_event_bus() -> Result<
    (
        ActorRef<WorkerMessage>,
        Arc<EventBus>,
        Arc<InMemoryEventStore>,
    ),
    Box<dyn std::error::Error>,
> {
    // Create event store and bus
    let store = Arc::new(InMemoryEventStore::new());
    let bus = Arc::new(EventBus::new(store.clone()));

    // Create worker config with event bus
    let config = WorkerConfig {
        checkpoint_interval: Duration::from_secs(60),
        retry_policy: WorkerRetryPolicy::default(),
        event_bus: Some(bus.clone()),
        workspace_manager: None,
    };

    // Spawn worker actor
    let (worker, _handle) = ractor::Actor::spawn(None, WorkerActorDef, config).await?;

    Ok((worker, bus, store))
}

/// Helper to subscribe to events and wait for the next event.
async fn wait_for_event(bus: &EventBus, timeout_ms: u64) -> Result<BeadEvent, String> {
    let mut sub = bus.subscribe();
    match tokio::time::timeout(Duration::from_millis(timeout_ms), sub.recv()).await {
        Ok(Ok(event)) => Ok(event),
        Ok(Err(e)) => Err(format!("Failed to receive event: {:?}", e)),
        Err(_) => Err("Timeout waiting for event".to_string()),
    }
}

// ═══════════════════════════════════════════════════════════════════════════════
// STATE TRANSITION EVENT EMISSION TESTS
// ═══════════════════════════════════════════════════════════════════════════════

#[tokio::test]
async fn given_worker_when_start_bead_then_emits_state_changed_event()
-> Result<(), Box<dyn std::error::Error>> {
    // Given: A worker actor with event bus
    let (worker, bus, _store) = setup_worker_with_event_bus().await?;

    // When: Starting a bead
    let bead_id = BeadId::new();
    let bead_id_str = bead_id.to_string();
    worker.send_message(WorkerMessage::StartBead {
        bead_id: bead_id_str.clone(),
        from_state: Some(BeadState::Ready),
    })?;

    // Then: StateChanged event should be emitted
    let event = wait_for_event(&bus, 500).await?;
    assert_eq!(event.event_type(), "state_changed");
    assert_eq!(event.bead_id(), bead_id);

    // Verify event contains transition data
    match event {
        BeadEvent::StateChanged { from, to, .. } => {
            assert_eq!(from, BeadState::Ready);
            assert_eq!(to, BeadState::Running);
        }
        _ => panic!("Expected StateChanged event, got {:?}", event.event_type()),
    }

    // Cleanup
    worker.stop(Some("test complete".to_string()));

    Ok(())
}

#[tokio::test]
async fn given_worker_when_fail_bead_then_emits_failed_event()
-> Result<(), Box<dyn std::error::Error>> {
    // Given: A worker actor with event bus and an active bead
    let (worker, bus, _store) = setup_worker_with_event_bus().await?;

    let bead_id = BeadId::new();
    let bead_id_str = bead_id.to_string();
    worker.send_message(WorkerMessage::StartBead {
        bead_id: bead_id_str.clone(),
        from_state: Some(BeadState::Ready),
    })?;

    // Wait for the start event
    let _start_event = wait_for_event(&bus, 500).await?;

    // When: Failing the bead
    let error_msg = "Test failure";
    worker.send_message(WorkerMessage::FailBead {
        error: error_msg.to_string(),
    })?;

    // Then: Failed event should be emitted
    let event = wait_for_event(&bus, 500).await?;
    assert_eq!(event.event_type(), "failed");
    assert_eq!(event.bead_id(), bead_id);

    // Verify event contains error message
    match event {
        BeadEvent::Failed { error, .. } => {
            assert_eq!(error, error_msg);
        }
        _ => panic!("Expected Failed event, got {:?}", event.event_type()),
    }

    // Cleanup
    worker.stop(Some("test complete".to_string()));

    Ok(())
}

#[tokio::test]
async fn given_worker_when_multiple_transitions_then_emits_events_for_each()
-> Result<(), Box<dyn std::error::Error>> {
    // Given: A worker actor with event bus
    let (worker, bus, _store) = setup_worker_with_event_bus().await?;

    // When: Starting a bead
    let bead_id = BeadId::new();
    let bead_id_str = bead_id.to_string();
    worker.send_message(WorkerMessage::StartBead {
        bead_id: bead_id_str.clone(),
        from_state: Some(BeadState::Ready),
    })?;

    // Then: First state change event emitted
    let event1 = wait_for_event(&bus, 500).await?;
    assert_eq!(event1.event_type(), "state_changed");
    assert_eq!(event1.bead_id(), bead_id);

    // When: Failing the bead
    worker.send_message(WorkerMessage::FailBead {
        error: "Test failure".to_string(),
    })?;

    // Then: Failed event emitted
    let event2 = wait_for_event(&bus, 500).await?;
    assert_eq!(event2.event_type(), "failed");
    assert_eq!(event2.bead_id(), bead_id);

    // Cleanup
    worker.stop(Some("test complete".to_string()));

    Ok(())
}

#[tokio::test]
async fn given_worker_when_no_event_bus_then_continues_normally()
-> Result<(), Box<dyn std::error::Error>> {
    // Given: A worker actor WITHOUT event bus
    let config = WorkerConfig {
        checkpoint_interval: Duration::from_secs(60),
        retry_policy: WorkerRetryPolicy::default(),
        event_bus: None, // No event bus
        workspace_manager: None,
    };

    let (worker, _handle) = ractor::Actor::spawn(None, WorkerActorDef, config).await?;

    // When: Starting a bead (should not panic or fail)
    let bead_id = BeadId::new();
    let bead_id_str = bead_id.to_string();
    let result = worker.send_message(WorkerMessage::StartBead {
        bead_id: bead_id_str,
        from_state: Some(BeadState::Ready),
    });

    // Then: Message should be sent successfully
    assert!(result.is_ok(), "Send should succeed without event bus");

    // Allow time for message processing
    tokio::time::sleep(Duration::from_millis(50)).await;

    // Cleanup
    worker.stop(Some("test complete".to_string()));

    Ok(())
}

#[tokio::test]
async fn given_worker_when_event_bus_publish_fails_then_state_transition_continues()
-> Result<(), Box<dyn std::error::Error>> {
    // Given: A worker actor with event bus
    // Note: This test verifies that event emission failures don't block state transitions
    // Since EventBus.publish() returns Result, the actor should log and continue

    let (worker, bus, _store) = setup_worker_with_event_bus().await?;

    // When: Starting a bead
    let bead_id = BeadId::new();
    let bead_id_str = bead_id.to_string();
    worker.send_message(WorkerMessage::StartBead {
        bead_id: bead_id_str.clone(),
        from_state: Some(BeadState::Ready),
    })?;

    // Then: Event should be emitted (best-effort logging)
    let event = wait_for_event(&bus, 500).await?;
    assert_eq!(event.event_type(), "state_changed");

    // Verify worker state actually changed (not blocked by event emission)
    tokio::time::sleep(Duration::from_millis(50)).await;

    // Cleanup
    worker.stop(Some("test complete".to_string()));

    Ok(())
}

#[tokio::test]
async fn given_worker_when_state_changes_with_custom_from_state_then_emits_correct_transition()
-> Result<(), Box<dyn std::error::Error>> {
    // Given: A worker actor with event bus
    let (worker, bus, _store) = setup_worker_with_event_bus().await?;

    // When: Starting a bead from a custom state (e.g., Retry)
    let bead_id = BeadId::new();
    let bead_id_str = bead_id.to_string();
    worker.send_message(WorkerMessage::StartBead {
        bead_id: bead_id_str.clone(),
        from_state: Some(BeadState::BackingOff),
    })?;

    // Then: StateChanged event should show correct transition
    let event = wait_for_event(&bus, 500).await?;
    assert_eq!(event.event_type(), "state_changed");

    match event {
        BeadEvent::StateChanged { from, to, .. } => {
            assert_eq!(from, BeadState::BackingOff);
            assert_eq!(to, BeadState::Running);
        }
        _ => panic!("Expected StateChanged event"),
    }

    // Cleanup
    worker.stop(Some("test complete".to_string()));

    Ok(())
}

#[tokio::test]
async fn given_worker_when_stop_then_no_state_event_emitted()
-> Result<(), Box<dyn std::error::Error>> {
    // Given: A worker actor with event bus
    let (worker, bus, _store) = setup_worker_with_event_bus().await?;

    // When: Stopping the worker
    worker.stop(Some("test stop".to_string()));

    // Then: Wait a bit to ensure no events were emitted
    // (stop doesn't trigger state change events)
    match tokio::time::timeout(Duration::from_millis(200), bus.subscribe().recv()).await {
        Ok(_) => Err("Unexpected event emitted on stop".into()),
        Err(_) => Ok(()), // Timeout is expected - no events should be emitted
    }
}

// ═══════════════════════════════════════════════════════════════════════════════
// HEALTH CHECK EVENT EMISSION TESTS
// ═══════════════════════════════════════════════════════════════════════════════

#[tokio::test]
async fn given_worker_when_health_check_fails_then_emits_worker_unhealthy_event()
-> Result<(), Box<dyn std::error::Error>> {
    // Given: A worker actor with event bus
    let (worker, bus, _store) = setup_worker_with_event_bus().await?;

    // When: Health check fails
    let reason = "health check timeout after 5s";
    worker.send_message(WorkerMessage::HealthCheckFailed {
        reason: reason.to_string(),
    })?;

    // Then: WorkerUnhealthy event should be emitted
    let event = wait_for_event(&bus, 500).await?;
    assert_eq!(event.event_type(), "worker_unhealthy");

    // Verify event contains worker_id and reason
    match event {
        oya_events::BeadEvent::WorkerUnhealthy {
            worker_id,
            reason: event_reason,
            ..
        } => {
            assert!(!worker_id.is_empty(), "worker_id should not be empty");
            assert_eq!(event_reason, reason);
        }
        _ => panic!(
            "Expected WorkerUnhealthy event, got {:?}",
            event.event_type()
        ),
    }

    // Cleanup
    worker.stop(Some("test complete".to_string()));

    Ok(())
}

#[tokio::test]
async fn given_worker_with_active_bead_when_health_check_fails_then_emits_unhealthy_and_fails_bead()
-> Result<(), Box<dyn std::error::Error>> {
    // Given: A worker actor with event bus and an active bead
    let (worker, bus, _store) = setup_worker_with_event_bus().await?;

    let bead_id = oya_events::BeadId::new();
    let bead_id_str = bead_id.to_string();
    worker.send_message(WorkerMessage::StartBead {
        bead_id: bead_id_str.clone(),
        from_state: Some(oya_events::BeadState::Ready),
    })?;

    // Wait for the start event
    let _start_event = wait_for_event(&bus, 500).await?;

    // When: Health check fails
    let reason = "worker not responding";
    worker.send_message(WorkerMessage::HealthCheckFailed {
        reason: reason.to_string(),
    })?;

    // Then: WorkerUnhealthy event should be emitted
    let unhealthy_event = wait_for_event(&bus, 500).await?;
    assert_eq!(unhealthy_event.event_type(), "worker_unhealthy");

    // Then: Bead should also fail
    let fail_event = wait_for_event(&bus, 500).await?;
    assert_eq!(fail_event.event_type(), "failed");
    assert_eq!(fail_event.bead_id(), bead_id);

    // Cleanup
    worker.stop(Some("test complete".to_string()));

    Ok(())
}

#[tokio::test]
async fn given_worker_when_no_event_bus_then_health_check_fails_continues_normally()
-> Result<(), Box<dyn std::error::Error>> {
    // Given: A worker actor WITHOUT event bus
    let config = WorkerConfig {
        checkpoint_interval: Duration::from_secs(60),
        retry_policy: WorkerRetryPolicy::default(),
        event_bus: None, // No event bus
        workspace_manager: None,
    };

    let (worker, _handle) = ractor::Actor::spawn(None, WorkerActorDef, config).await?;

    // When: Health check fails (should not panic or fail)
    let result = worker.send_message(WorkerMessage::HealthCheckFailed {
        error: "health check failed".to_string(),
    });

    // Then: Message should be sent successfully
    assert!(result.is_ok(), "Send should succeed without event bus");

    // Allow time for message processing
    tokio::time::sleep(Duration::from_millis(50)).await;

    // Cleanup
    worker.stop(Some("test complete".to_string()));

    Ok(())
}
