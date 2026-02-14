#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
//! Integration tests for IPC Worker (Zellij plugin communication bridge).
//!
//! These tests verify the full integration of the IPC worker with:
//! - EventBus for event subscription and broadcasting
//! - OrchestratorStore for persistence
//! - Message handling (GuestMessage -> HostMessage flow)
//! - Event emission on bead operations
//!
//! # Test Patterns
//!
//! All tests follow Given-When-Then style with functional Rust patterns:
//! - Zero unwrap/panic
//! - Railway-Oriented Programming with Result<T, E>
//! - Proper error handling via match
//! - Async/await with tokio

use std::sync::Arc;
use std::time::Duration;

use oya_events::{BeadEvent, BeadId, BeadState, EventBus, InMemoryEventStore};
use ractor::{Actor, ActorRef};

use orchestrator::actors::ipc_worker::{IpcWorkerActorDef, IpcWorkerArguments, IpcWorkerMessage};
use orchestrator::ipc_messages::{GuestMessage, HostMessage};
use orchestrator::persistence::{
    BeadRecord, BeadState as PersistenceBeadState, OrchestratorStore, StoreConfig,
};

// =========================================================================
// Test Helpers
// =========================================================================

/// Setup integration test with full stack.
///
/// Creates EventBus, Store, and spawns IPC worker actor.
async fn setup_integration_test() -> Result<IntegrationTestSetup, String> {
    // Create event store and bus
    let event_store = Arc::new(InMemoryEventStore::new());
    let event_bus = Arc::new(EventBus::new(event_store.clone()));

    // Create persistence store
    let store_config = StoreConfig::in_memory();
    let store = match OrchestratorStore::connect(store_config).await {
        Ok(s) => Arc::new(s),
        Err(e) => return Err(format!("Failed to create store: {e}")),
    };
    if let Err(e) = store.initialize_schema().await {
        return Err(format!("Failed to initialize schema: {e}"));
    }

    // Create worker arguments
    let args = IpcWorkerArguments::new()
        .with_event_bus(event_bus.clone())
        .with_store(store.clone());

    // Spawn IPC worker actor
    let (worker, _handle) = Actor::spawn(None, IpcWorkerActorDef, args)
        .await
        .map_err(|e| format!("Failed to spawn worker: {e}"))?;

    Ok(IntegrationTestSetup {
        worker,
        event_bus,
        store,
    })
}

/// Integration test setup containing all components.
struct IntegrationTestSetup {
    worker: ActorRef<IpcWorkerMessage>,
    event_bus: Arc<EventBus>,
    store: Arc<OrchestratorStore>,
}

/// Create a test bead in the store.
async fn create_test_bead(
    store: &Arc<OrchestratorStore>,
    bead_id: &str,
    workflow_id: &str,
    state: PersistenceBeadState,
) -> Result<(), String> {
    let mut bead = BeadRecord::new(bead_id, workflow_id);
    bead.state = state;

    store
        .save_bead(&bead)
        .await
        .map(|_| ())
        .map_err(|e| format!("Failed to save bead: {e}"))
}

#[allow(dead_code)]
/// Wait for event with timeout and retry logic.
async fn wait_for_event(bus: &EventBus, timeout_ms: u64) -> Result<BeadEvent, String> {
    let mut sub = bus.subscribe();
    let max_attempts = 10;
    let mut attempt = 0;

    while attempt < max_attempts {
        match tokio::time::timeout(Duration::from_millis(timeout_ms), sub.recv()).await {
            Ok(Ok(event)) => return Ok(event),
            Ok(Err(e)) => {
                attempt += 1;
                if attempt >= max_attempts {
                    return Err(format!("Failed to receive event: {e:?}"));
                }
                tokio::time::sleep(Duration::from_millis(100)).await;
            }
            Err(_) => {
                attempt += 1;
                if attempt >= max_attempts {
                    return Err(format!(
                        "Timeout waiting for event after {max_attempts} attempts"
                    ));
                }
                tokio::time::sleep(Duration::from_millis(100)).await;
            }
        }
    }

    Err("Should not reach here".to_string())
}

#[allow(dead_code)]
/// Convert persistence BeadState to events BeadState.
fn persistence_to_events_state(state: PersistenceBeadState) -> BeadState {
    match state {
        PersistenceBeadState::Pending => BeadState::Pending,
        PersistenceBeadState::Ready => BeadState::Ready,
        PersistenceBeadState::Dispatched => BeadState::Scheduled,
        PersistenceBeadState::Assigned => BeadState::Ready,
        PersistenceBeadState::Running => BeadState::Running,
        PersistenceBeadState::Completed => BeadState::Completed,
        PersistenceBeadState::Failed => BeadState::Failed,
        PersistenceBeadState::Cancelled => BeadState::Cancelled,
    }
}

// =========================================================================
// Integration Tests: Start Bead
// =========================================================================

#[tokio::test]
async fn given_ipc_worker_when_start_bead_then_emits_state_changed_event(
) -> Result<(), Box<dyn std::error::Error>> {
    let setup = setup_integration_test()
        .await
        .map_err(|e| format!("Setup failed: {e}"))?;

    let bead_id = "test-start-integration";
    create_test_bead(
        &setup.store,
        bead_id,
        "test-workflow",
        PersistenceBeadState::Ready,
    )
    .await
    .map_err(|e| format!("Failed to create bead: {e}"))?;

    // Subscribe to events before sending message
    let mut sub = setup.event_bus.subscribe();

    // Send StartBead message via RPC using ractor::call_t!
    let response = ractor::call_t!(
        &setup.worker,
        |reply| IpcWorkerMessage::HandleGuestMessage {
            message: GuestMessage::StartBead {
                bead_id: bead_id.to_string(),
            },
            reply,
        },
        5000
    )
    .map_err(|e| format!("RPC call failed: {e}"))?;

    // Wait for StateChanged event
    let event = tokio::time::timeout(Duration::from_millis(1000), sub.recv())
        .await
        .map_err(|_| "Timeout waiting for event")?
        .map_err(|e| format!("Failed to receive event: {e:?}"))?;

    match &event {
        BeadEvent::StateChanged {
            bead_id: id,
            from,
            to,
            ..
        } => {
            assert_eq!(format!("{id}"), bead_id);
            assert_eq!(*from, BeadState::Ready);
            assert_eq!(*to, BeadState::Running);
        }
        other => {
            return Err(
                format!("Expected StateChanged event, got {:?}", other.event_type()).into(),
            );
        }
    }

    // Verify response
    match response {
        Ok(HostMessage::Ack { command, message }) => {
            assert_eq!(command, "StartBead");
            assert!(message.contains("started successfully"));
        }
        other => {
            return Err(format!("Expected Ack response, got {other:?}").into());
        }
    }

    // Cleanup
    setup.worker.stop(Some("test complete".to_string()));

    Ok(())
}

#[tokio::test]
async fn given_ipc_worker_when_cancel_bead_then_emits_state_changed_event(
) -> Result<(), Box<dyn std::error::Error>> {
    let setup = setup_integration_test()
        .await
        .map_err(|e| format!("Setup failed: {e}"))?;

    let bead_id = "test-cancel-integration";
    create_test_bead(
        &setup.store,
        bead_id,
        "test-workflow",
        PersistenceBeadState::Running,
    )
    .await
    .map_err(|e| format!("Failed to create bead: {e}"))?;

    let mut sub = setup.event_bus.subscribe();

    let response = ractor::call_t!(
        &setup.worker,
        |reply| IpcWorkerMessage::HandleGuestMessage {
            message: GuestMessage::CancelBead {
                bead_id: bead_id.to_string(),
            },
            reply,
        },
        5000
    )
    .map_err(|e| format!("RPC call failed: {e}"))?;

    let event = tokio::time::timeout(Duration::from_millis(1000), sub.recv())
        .await
        .map_err(|_| "Timeout waiting for event")?
        .map_err(|e| format!("Failed to receive event: {e:?}"))?;

    match &event {
        BeadEvent::StateChanged {
            bead_id: id,
            from,
            to,
            ..
        } => {
            assert_eq!(format!("{id}"), bead_id);
            assert_eq!(*from, BeadState::Running);
            assert_eq!(*to, BeadState::Cancelled);
        }
        other => {
            return Err(
                format!("Expected StateChanged event, got {:?}", other.event_type()).into(),
            );
        }
    }

    match response {
        Ok(HostMessage::Ack { command, message }) => {
            assert_eq!(command, "CancelBead");
            assert!(message.contains("cancelled"));
        }
        other => {
            return Err(format!("Expected Ack response, got {other:?}").into());
        }
    }

    setup.worker.stop(Some("test complete".to_string()));

    Ok(())
}

#[tokio::test]
async fn given_ipc_worker_when_retry_bead_then_transitions_to_ready(
) -> Result<(), Box<dyn std::error::Error>> {
    let setup = setup_integration_test()
        .await
        .map_err(|e| format!("Setup failed: {e}"))?;

    let bead_id = "test-retry-integration";
    create_test_bead(
        &setup.store,
        bead_id,
        "test-workflow",
        PersistenceBeadState::Failed,
    )
    .await
    .map_err(|e| format!("Failed to create bead: {e}"))?;

    let mut sub = setup.event_bus.subscribe();

    let response = ractor::call_t!(
        &setup.worker,
        |reply| IpcWorkerMessage::HandleGuestMessage {
            message: GuestMessage::RetryBead {
                bead_id: bead_id.to_string(),
            },
            reply,
        },
        5000
    )
    .map_err(|e| format!("RPC call failed: {e}"))?;

    let event = tokio::time::timeout(Duration::from_millis(1000), sub.recv())
        .await
        .map_err(|_| "Timeout waiting for event")?
        .map_err(|e| format!("Failed to receive event: {e:?}"))?;

    match &event {
        BeadEvent::StateChanged {
            bead_id: id,
            from,
            to,
            ..
        } => {
            assert_eq!(format!("{id}"), bead_id);
            assert_eq!(*from, BeadState::Failed);
            assert_eq!(*to, BeadState::Ready);
        }
        other => {
            return Err(
                format!("Expected StateChanged event, got {:?}", other.event_type()).into(),
            );
        }
    }

    match response {
        Ok(HostMessage::Ack { command, message }) => {
            assert_eq!(command, "RetryBead");
            assert!(message.contains("reset for retry"));
        }
        other => {
            return Err(format!("Expected Ack response, got {other:?}").into());
        }
    }

    setup.worker.stop(Some("test complete".to_string()));

    Ok(())
}

// =========================================================================
// Integration Tests: Error Handling
// =========================================================================

#[tokio::test]
async fn given_ipc_worker_when_start_nonexistent_bead_then_returns_error(
) -> Result<(), Box<dyn std::error::Error>> {
    let setup = setup_integration_test()
        .await
        .map_err(|e| format!("Setup failed: {e}"))?;

    let bead_id = "nonexistent-bead";

    let response = ractor::call_t!(
        &setup.worker,
        |reply| IpcWorkerMessage::HandleGuestMessage {
            message: GuestMessage::StartBead {
                bead_id: bead_id.to_string(),
            },
            reply,
        },
        5000
    )
    .map_err(|e| format!("RPC call failed: {e}"))?;

    match response {
        Err(orchestrator::actors::errors::ActorError::BeadNotFound(id)) => {
            assert_eq!(id, bead_id);
        }
        other => {
            return Err(format!("Expected BeadNotFound error, got {other:?}").into());
        }
    }

    setup.worker.stop(Some("test complete".to_string()));

    Ok(())
}

#[tokio::test]
async fn given_ipc_worker_when_start_completed_bead_then_returns_invalid_state_error(
) -> Result<(), Box<dyn std::error::Error>> {
    let setup = setup_integration_test()
        .await
        .map_err(|e| format!("Setup failed: {e}"))?;

    let bead_id = "test-completed-error";
    create_test_bead(
        &setup.store,
        bead_id,
        "test-workflow",
        PersistenceBeadState::Completed,
    )
    .await
    .map_err(|e| format!("Failed to create bead: {e}"))?;

    let response = ractor::call_t!(
        &setup.worker,
        |reply| IpcWorkerMessage::HandleGuestMessage {
            message: GuestMessage::StartBead {
                bead_id: bead_id.to_string(),
            },
            reply,
        },
        5000
    )
    .map_err(|e| format!("RPC call failed: {e}"))?;

    match response {
        Err(orchestrator::actors::errors::ActorError::InvalidStateTransition(_)) => {
            // Expected error
        }
        other => {
            return Err(format!("Expected InvalidStateTransition error, got {other:?}").into());
        }
    }

    // Verify state didn't change
    let bead = setup
        .store
        .get_bead(bead_id)
        .await
        .map_err(|e| format!("Failed to get bead: {e}"))?;
    assert_eq!(bead.state, PersistenceBeadState::Completed);

    setup.worker.stop(Some("test complete".to_string()));

    Ok(())
}

// =========================================================================
// Integration Tests: Multiple Operations
// =========================================================================

#[tokio::test]
async fn given_ipc_worker_when_multiple_bead_operations_then_all_events_emitted(
) -> Result<(), Box<dyn std::error::Error>> {
    let setup = setup_integration_test()
        .await
        .map_err(|e| format!("Setup failed: {e}"))?;

    let bead_id_1 = "test-multi-1";
    let bead_id_2 = "test-multi-2";

    create_test_bead(
        &setup.store,
        bead_id_1,
        "test-workflow",
        PersistenceBeadState::Ready,
    )
    .await
    .map_err(|e| format!("Failed to create bead 1: {e}"))?;

    create_test_bead(
        &setup.store,
        bead_id_2,
        "test-workflow",
        PersistenceBeadState::Running,
    )
    .await
    .map_err(|e| format!("Failed to create bead 2: {e}"))?;

    let mut sub = setup.event_bus.subscribe();

    // Start bead 1
    let _ = ractor::call_t!(
        &setup.worker,
        |reply| IpcWorkerMessage::HandleGuestMessage {
            message: GuestMessage::StartBead {
                bead_id: bead_id_1.to_string(),
            },
            reply,
        },
        5000
    );

    // Cancel bead 2
    let _ = ractor::call_t!(
        &setup.worker,
        |reply| IpcWorkerMessage::HandleGuestMessage {
            message: GuestMessage::CancelBead {
                bead_id: bead_id_2.to_string(),
            },
            reply,
        },
        5000
    );

    // Wait for both events
    let event1 = tokio::time::timeout(Duration::from_millis(1000), sub.recv())
        .await
        .map_err(|_| "Timeout waiting for event 1")?
        .map_err(|e| format!("Failed to receive event 1: {e:?}"))?;

    let event2 = tokio::time::timeout(Duration::from_millis(1000), sub.recv())
        .await
        .map_err(|_| "Timeout waiting for event 2")?
        .map_err(|e| format!("Failed to receive event 2: {e:?}"))?;

    // Verify we got StateChanged events for both beads
    let ids = [event1.bead_id(), event2.bead_id()];
    assert!(ids.contains(&BeadId::try_from(bead_id_1.to_string()).map_err(|e| format!("{e:?}"))?));
    assert!(ids.contains(&BeadId::try_from(bead_id_2.to_string()).map_err(|e| format!("{e:?}"))?));

    setup.worker.stop(Some("test complete".to_string()));

    Ok(())
}

// =========================================================================
// Integration Tests: Idempotency
// =========================================================================

#[tokio::test]
async fn given_ipc_worker_when_start_running_bead_twice_then_succeeds_idempotently(
) -> Result<(), Box<dyn std::error::Error>> {
    let setup = setup_integration_test()
        .await
        .map_err(|e| format!("Setup failed: {e}"))?;

    let bead_id = "test-idempotent-start";
    create_test_bead(
        &setup.store,
        bead_id,
        "test-workflow",
        PersistenceBeadState::Running,
    )
    .await
    .map_err(|e| format!("Failed to create bead: {e}"))?;

    // First start
    let response1 = ractor::call_t!(
        &setup.worker,
        |reply| IpcWorkerMessage::HandleGuestMessage {
            message: GuestMessage::StartBead {
                bead_id: bead_id.to_string(),
            },
            reply,
        },
        5000
    )
    .map_err(|e| format!("First RPC call failed: {e}"))?;

    match response1 {
        Ok(HostMessage::Ack { .. }) => {
            // First call succeeds
        }
        other => {
            return Err(format!("Expected first Ack, got {other:?}").into());
        }
    }

    // Second start (should also succeed)
    let response2 = ractor::call_t!(
        &setup.worker,
        |reply| IpcWorkerMessage::HandleGuestMessage {
            message: GuestMessage::StartBead {
                bead_id: bead_id.to_string(),
            },
            reply,
        },
        5000
    )
    .map_err(|e| format!("Second RPC call failed: {e}"))?;

    match response2 {
        Ok(HostMessage::Ack { command, message }) => {
            assert_eq!(command, "StartBead");
            assert!(message.contains("already running"));
        }
        other => {
            return Err(format!("Expected second Ack, got {other:?}").into());
        }
    }

    // Verify state is still Running
    let bead = setup
        .store
        .get_bead(bead_id)
        .await
        .map_err(|e| format!("Failed to get bead: {e}"))?;
    assert_eq!(bead.state, PersistenceBeadState::Running);

    setup.worker.stop(Some("test complete".to_string()));

    Ok(())
}

#[tokio::test]
async fn given_ipc_worker_when_cancel_cancelled_bead_then_succeeds_idempotently(
) -> Result<(), Box<dyn std::error::Error>> {
    let setup = setup_integration_test()
        .await
        .map_err(|e| format!("Setup failed: {e}"))?;

    let bead_id = "test-idempotent-cancel";
    create_test_bead(
        &setup.store,
        bead_id,
        "test-workflow",
        PersistenceBeadState::Cancelled,
    )
    .await
    .map_err(|e| format!("Failed to create bead: {e}"))?;

    // First cancel
    let response1 = ractor::call_t!(
        &setup.worker,
        |reply| IpcWorkerMessage::HandleGuestMessage {
            message: GuestMessage::CancelBead {
                bead_id: bead_id.to_string(),
            },
            reply,
        },
        5000
    )
    .map_err(|e| format!("First RPC call failed: {e}"))?;

    match response1 {
        Ok(HostMessage::Ack { .. }) => {
            // First call succeeds
        }
        other => {
            return Err(format!("Expected first Ack, got {other:?}").into());
        }
    }

    // Second cancel (should also succeed)
    let response2 = ractor::call_t!(
        &setup.worker,
        |reply| IpcWorkerMessage::HandleGuestMessage {
            message: GuestMessage::CancelBead {
                bead_id: bead_id.to_string(),
            },
            reply,
        },
        5000
    )
    .map_err(|e| format!("Second RPC call failed: {e}"))?;

    match response2 {
        Ok(HostMessage::Ack { command, message }) => {
            assert_eq!(command, "CancelBead");
            assert!(message.contains("already cancelled"));
        }
        other => {
            return Err(format!("Expected second Ack, got {other:?}").into());
        }
    }

    // Verify state is still Cancelled
    let bead = setup
        .store
        .get_bead(bead_id)
        .await
        .map_err(|e| format!("Failed to get bead: {e}"))?;
    assert_eq!(bead.state, PersistenceBeadState::Cancelled);

    setup.worker.stop(Some("test complete".to_string()));

    Ok(())
}

// =========================================================================
// Integration Tests: Event Bus Integration
// =========================================================================

#[tokio::test]
async fn given_ipc_worker_when_event_bus_available_then_subscribes_on_startup(
) -> Result<(), Box<dyn std::error::Error>> {
    let setup = setup_integration_test()
        .await
        .map_err(|e| format!("Setup failed: {e}"))?;

    // Give worker time to subscribe
    tokio::time::sleep(Duration::from_millis(100)).await;

    // Publish a test event
    let test_bead_id = BeadId::new();
    let event = BeadEvent::state_changed(test_bead_id, BeadState::Ready, BeadState::Running);

    setup.event_bus.publish(event).await?;

    // Verify worker is still running (no crash on event)
    tokio::time::sleep(Duration::from_millis(100)).await;

    setup.worker.stop(Some("test complete".to_string()));

    Ok(())
}

// =========================================================================
// Integration Tests: Persistence Integration
// =========================================================================

#[tokio::test]
async fn given_ipc_worker_when_bead_operation_then_persists_state_change(
) -> Result<(), Box<dyn std::error::Error>> {
    let setup = setup_integration_test()
        .await
        .map_err(|e| format!("Setup failed: {e}"))?;

    let bead_id = "test-persistence-integration";
    create_test_bead(
        &setup.store,
        bead_id,
        "test-workflow",
        PersistenceBeadState::Ready,
    )
    .await
    .map_err(|e| format!("Failed to create bead: {e}"))?;

    // Verify initial state
    let bead = setup
        .store
        .get_bead(bead_id)
        .await
        .map_err(|e| format!("Failed to get bead: {e}"))?;
    assert_eq!(bead.state, PersistenceBeadState::Ready);

    // Start bead
    let _ = ractor::call_t!(
        &setup.worker,
        |reply| IpcWorkerMessage::HandleGuestMessage {
            message: GuestMessage::StartBead {
                bead_id: bead_id.to_string(),
            },
            reply,
        },
        5000
    )
    .map_err(|e| format!("RPC call failed: {e}"))?;

    // Wait for operation to complete
    tokio::time::sleep(Duration::from_millis(200)).await;

    // Verify state was persisted
    let bead = setup
        .store
        .get_bead(bead_id)
        .await
        .map_err(|e| format!("Failed to get bead after operation: {e}"))?;
    assert_eq!(bead.state, PersistenceBeadState::Running);

    setup.worker.stop(Some("test complete".to_string()));

    Ok(())
}
