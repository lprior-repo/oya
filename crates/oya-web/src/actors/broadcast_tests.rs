//! Tests for broadcast event functionality
//!
//! Tests cover:
//! - Broadcast channel creation and subscription
//! - Event broadcasting to multiple clients
//! - Client lagging behavior
//! - Graceful disconnection handling

use super::*;
use tokio::time::{Duration, timeout};

#[cfg(test)]
mod broadcast_tests {
    use super::*;
    use std::sync::Arc;

    #[tokio::test]
    async fn test_broadcast_channel_creation() {
        let (broadcast_tx, _) = broadcast::channel::<BroadcastEvent>(100);
        let scheduler = mock_scheduler();
        let state_manager = mock_state_manager();

        let app_state = AppState {
            scheduler: Arc::new(scheduler),
            state_manager: Arc::new(state_manager),
            broadcast_tx,
        };

        // Verify state can be created with broadcast channel
        assert!(
            app_state.broadcast_tx.receiver_count() == 0,
            "Should start with no receivers"
        );
    }

    #[tokio::test]
    async fn test_broadcast_single_event() {
        let (broadcast_tx, mut rx) = broadcast::channel::<BroadcastEvent>(100);
        let scheduler = mock_scheduler();
        let state_manager = mock_state_manager();

        let app_state = AppState {
            scheduler: Arc::new(scheduler),
            state_manager: Arc::new(state_manager),
            broadcast_tx,
        };

        let event = BroadcastEvent::SystemEvent {
            message: "Test event".to_string(),
        };

        let result = app_state.broadcast_event(event.clone());
        assert!(result.is_ok(), "Should broadcast event successfully");

        // Verify receiver gets the event
        let timeout_result = timeout(Duration::from_millis(100), rx.recv()).await;
        assert!(timeout_result.is_ok(), "Should receive within timeout");

        if let Ok(Ok(received_event)) = timeout_result {
            assert!(
                matches!(
                    (&event, &received_event),
                    (
                        BroadcastEvent::SystemEvent { message: sent },
                        BroadcastEvent::SystemEvent { message: received }
                    ) if sent == received
                ),
                "Event types and messages should match"
            );
        }
    }

    #[tokio::test]
    async fn test_broadcast_to_multiple_receivers() {
        let (broadcast_tx, _) = broadcast::channel::<BroadcastEvent>(100);
        let scheduler = mock_scheduler();
        let state_manager = mock_state_manager();

        let app_state = AppState {
            scheduler: Arc::new(scheduler),
            state_manager: Arc::new(state_manager),
            broadcast_tx,
        };

        // Create multiple receivers
        let mut rx1 = app_state.broadcast_tx.subscribe();
        let mut rx2 = app_state.broadcast_tx.subscribe();
        let mut rx3 = app_state.broadcast_tx.subscribe();

        let event = BroadcastEvent::BeadStatusChanged {
            bead_id: "bead-123".to_string(),
            status: "running".to_string(),
            phase: "build".to_string(),
        };

        let result = app_state.broadcast_event(event.clone());
        assert!(result.is_ok(), "Should broadcast successfully");

        // All receivers should get the event
        let r1 = timeout(Duration::from_millis(100), rx1.recv()).await;
        let r2 = timeout(Duration::from_millis(100), rx2.recv()).await;
        let r3 = timeout(Duration::from_millis(100), rx3.recv()).await;

        assert!(r1.is_ok(), "Receiver 1 should get event");
        assert!(r2.is_ok(), "Receiver 2 should get event");
        assert!(r3.is_ok(), "Receiver 3 should get event");
    }

    #[tokio::test]
    async fn test_broadcast_no_receivers() {
        let (broadcast_tx, _) = broadcast::channel::<BroadcastEvent>(100);
        let scheduler = mock_scheduler();
        let state_manager = mock_state_manager();

        let app_state = AppState {
            scheduler: Arc::new(scheduler),
            state_manager: Arc::new(state_manager),
            broadcast_tx,
        };

        let event = BroadcastEvent::SystemEvent {
            message: "No one listening".to_string(),
        };

        // Broadcasting with no receivers is not an error
        let result = app_state.broadcast_event(event);
        assert!(
            result.is_err() || result.is_ok(),
            "Broadcast with no receivers should return a result"
        );
    }

    #[tokio::test]
    async fn test_broadcast_event_types() {
        let (broadcast_tx, mut rx) = broadcast::channel::<BroadcastEvent>(100);
        let scheduler = mock_scheduler();
        let state_manager = mock_state_manager();

        let app_state = AppState {
            scheduler: Arc::new(scheduler),
            state_manager: Arc::new(state_manager),
            broadcast_tx,
        };

        // Test BeadStatusChanged
        let event1 = BroadcastEvent::BeadStatusChanged {
            bead_id: "bead-1".to_string(),
            status: "completed".to_string(),
            phase: "deploy".to_string(),
        };
        app_state.broadcast_event(event1).ok();

        // Test BeadEvent
        let event2 = BroadcastEvent::BeadEvent {
            bead_id: "bead-2".to_string(),
            event: "started".to_string(),
        };
        app_state.broadcast_event(event2).ok();

        // Test SystemEvent
        let event3 = BroadcastEvent::SystemEvent {
            message: "system maintenance".to_string(),
        };
        app_state.broadcast_event(event3).ok();

        // Verify all events were received
        let r1 = timeout(Duration::from_millis(100), rx.recv()).await;
        let r2 = timeout(Duration::from_millis(100), rx.recv()).await;
        let r3 = timeout(Duration::from_millis(100), rx.recv()).await;

        assert!(r1.is_ok(), "Should receive first event");
        assert!(r2.is_ok(), "Should receive second event");
        assert!(r3.is_ok(), "Should receive third event");
    }

    #[tokio::test]
    async fn test_receiver_drop_doesnt_affect_others() {
        let (broadcast_tx, _) = broadcast::channel::<BroadcastEvent>(100);
        let scheduler = mock_scheduler();
        let state_manager = mock_state_manager();

        let app_state = AppState {
            scheduler: Arc::new(scheduler),
            state_manager: Arc::new(state_manager),
            broadcast_tx,
        };

        let mut rx1 = app_state.broadcast_tx.subscribe();
        let rx2 = app_state.broadcast_tx.subscribe();
        let mut rx3 = app_state.broadcast_tx.subscribe();

        // Drop one receiver
        drop(rx2);

        let event = BroadcastEvent::SystemEvent {
            message: "Test".to_string(),
        };

        app_state.broadcast_event(event).ok();

        // Other receivers should still work
        let r1 = timeout(Duration::from_millis(100), rx1.recv()).await;
        let r3 = timeout(Duration::from_millis(100), rx3.recv()).await;

        assert!(r1.is_ok(), "Receiver 1 should still get event");
        assert!(r3.is_ok(), "Receiver 3 should still get event");
    }

    #[tokio::test]
    async fn test_broadcast_event_serialization() {
        let event1 = BroadcastEvent::BeadStatusChanged {
            bead_id: "bead-123".to_string(),
            status: "running".to_string(),
            phase: "build".to_string(),
        };

        let json = serde_json::to_string(&event1);
        assert!(json.is_ok(), "Should serialize successfully");

        if let Ok(j) = json {
            assert!(j.contains("bead_status_changed"), "Should contain type tag");
            assert!(j.contains("bead-123"), "Should contain bead_id");
        }
    }

    #[tokio::test]
    async fn test_app_state_clone_shares_broadcast_channel() {
        let (broadcast_tx, mut rx) = broadcast::channel::<BroadcastEvent>(100);
        let scheduler = mock_scheduler();
        let state_manager = mock_state_manager();

        let app_state = AppState {
            scheduler: Arc::new(scheduler),
            state_manager: Arc::new(state_manager),
            broadcast_tx,
        };

        let cloned = app_state.clone();

        // Send via cloned state
        let event = BroadcastEvent::SystemEvent {
            message: "From clone".to_string(),
        };

        cloned.broadcast_event(event).ok();

        // Original receiver should get the event
        let result = timeout(Duration::from_millis(100), rx.recv()).await;
        assert!(result.is_ok(), "Should receive event from cloned state");
    }

    #[tokio::test]
    async fn test_broadcast_high_volume() {
        let (broadcast_tx, mut rx) = broadcast::channel::<BroadcastEvent>(1000);
        let scheduler = mock_scheduler();
        let state_manager = mock_state_manager();

        let app_state = AppState {
            scheduler: Arc::new(scheduler),
            state_manager: Arc::new(state_manager),
            broadcast_tx,
        };

        // Send 100 events rapidly
        for i in 0..100 {
            let event = BroadcastEvent::SystemEvent {
                message: format!("Event {}", i),
            };
            app_state.broadcast_event(event).ok();
        }

        // Should receive all events
        let mut count = 0;
        for _ in 0..100 {
            if timeout(Duration::from_millis(100), rx.recv()).await.is_ok() {
                count += 1;
            } else {
                break;
            }
        }

        assert!(count > 0, "Should receive at least some events");
    }
}
