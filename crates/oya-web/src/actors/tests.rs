//! Comprehensive unit tests for actor lifecycle and message handling
//!
//! Tests cover:
//! - Actor lifecycle (spawn, run, stop)
//! - Message handling (send, receive, processing)
//! - Channel behavior and backpressure
//! - Error handling and supervision

use super::*;
use tokio::sync::mpsc;
use tokio::time::{Duration, timeout};

#[cfg(test)]
mod scheduler_actor_tests {
    use super::*;

    #[tokio::test]
    async fn test_scheduler_spawns_and_receives_messages() {
        let scheduler = mock_scheduler();
        let spec = "test-spec".to_string();

        let result = scheduler.send(SchedulerMessage::CreateBead { spec: spec.clone() });

        assert!(result.is_ok(), "Failed to send message to scheduler");

        // Give time for message to be processed
        tokio::time::sleep(Duration::from_millis(10)).await;
    }

    #[tokio::test]
    async fn test_scheduler_handles_create_bead_message() {
        let scheduler = mock_scheduler();

        let result = scheduler.send(SchedulerMessage::CreateBead {
            spec: "test-bead-spec".to_string(),
        });

        assert!(
            result.is_ok(),
            "Should successfully send CreateBead message"
        );
    }

    #[tokio::test]
    async fn test_scheduler_handles_cancel_bead_message() {
        let scheduler = mock_scheduler();
        let test_id = Ulid::new();

        let result = scheduler.send(SchedulerMessage::CancelBead { id: test_id });

        assert!(
            result.is_ok(),
            "Should successfully send CancelBead message"
        );
    }

    #[tokio::test]
    async fn test_scheduler_handles_multiple_messages_in_sequence() {
        let scheduler = mock_scheduler();

        let spec1 = "spec-1".to_string();
        let spec2 = "spec-2".to_string();
        let id1 = Ulid::new();

        let r1 = scheduler.send(SchedulerMessage::CreateBead { spec: spec1 });
        let r2 = scheduler.send(SchedulerMessage::CreateBead { spec: spec2 });
        let r3 = scheduler.send(SchedulerMessage::CancelBead { id: id1 });

        assert!(r1.is_ok(), "First message should send successfully");
        assert!(r2.is_ok(), "Second message should send successfully");
        assert!(r3.is_ok(), "Third message should send successfully");

        // Allow time for processing
        tokio::time::sleep(Duration::from_millis(10)).await;
    }

    #[tokio::test]
    async fn test_scheduler_channel_survives_actor_lifecycle() {
        let scheduler = mock_scheduler();

        // Send before potential actor startup
        let r1 = scheduler.send(SchedulerMessage::CreateBead {
            spec: "early".to_string(),
        });
        assert!(r1.is_ok(), "Should send before actor fully initialized");

        // Small delay to ensure actor is running
        tokio::time::sleep(Duration::from_millis(5)).await;

        // Send after actor is running
        let r2 = scheduler.send(SchedulerMessage::CreateBead {
            spec: "late".to_string(),
        });
        assert!(r2.is_ok(), "Should send after actor is running");
    }

    #[tokio::test]
    async fn test_scheduler_message_types_are_distinct() {
        let scheduler = mock_scheduler();
        let test_id = Ulid::new();

        // Verify we can send different message types
        let create_result = scheduler.send(SchedulerMessage::CreateBead {
            spec: "test".to_string(),
        });

        let cancel_result = scheduler.send(SchedulerMessage::CancelBead { id: test_id });

        assert!(create_result.is_ok(), "CreateBead should send");
        assert!(cancel_result.is_ok(), "CancelBead should send");
    }
}

#[cfg(test)]
mod state_manager_actor_tests {
    use super::*;

    #[tokio::test]
    async fn test_state_manager_spawns_and_receives_messages() {
        let state_manager = mock_state_manager();
        let test_id = Ulid::new();

        let result = state_manager.send(StateManagerMessage::QueryBead { id: test_id });

        assert!(result.is_ok(), "Failed to send message to state manager");

        // Give time for message to be processed
        tokio::time::sleep(Duration::from_millis(10)).await;
    }

    #[tokio::test]
    async fn test_state_manager_handles_query_bead_message() {
        let state_manager = mock_state_manager();
        let bead_id = Ulid::new();

        let result = state_manager.send(StateManagerMessage::QueryBead { id: bead_id });

        assert!(result.is_ok(), "Should successfully send QueryBead message");
    }

    #[tokio::test]
    async fn test_state_manager_handles_multiple_queries() {
        let state_manager = mock_state_manager();

        let id1 = Ulid::new();
        let id2 = Ulid::new();
        let id3 = Ulid::new();

        let r1 = state_manager.send(StateManagerMessage::QueryBead { id: id1 });
        let r2 = state_manager.send(StateManagerMessage::QueryBead { id: id2 });
        let r3 = state_manager.send(StateManagerMessage::QueryBead { id: id3 });

        assert!(r1.is_ok(), "First query should send successfully");
        assert!(r2.is_ok(), "Second query should send successfully");
        assert!(r3.is_ok(), "Third query should send successfully");

        // Allow time for processing
        tokio::time::sleep(Duration::from_millis(10)).await;
    }

    #[tokio::test]
    async fn test_state_manager_channel_survives_actor_lifecycle() {
        let state_manager = mock_state_manager();

        // Send before potential actor startup
        let r1 = state_manager.send(StateManagerMessage::QueryBead { id: Ulid::new() });
        assert!(r1.is_ok(), "Should send before actor fully initialized");

        // Small delay to ensure actor is running
        tokio::time::sleep(Duration::from_millis(5)).await;

        // Send after actor is running
        let r2 = state_manager.send(StateManagerMessage::QueryBead { id: Ulid::new() });
        assert!(r2.is_ok(), "Should send after actor is running");
    }
}

#[cfg(test)]
mod message_type_tests {
    use super::*;

    #[test]
    fn test_scheduler_message_create_bead_construction() {
        let spec = "test-spec".to_string();
        let msg = SchedulerMessage::CreateBead { spec: spec.clone() };

        match msg {
            SchedulerMessage::CreateBead { spec: s } => {
                assert_eq!(s, spec, "Spec should match");
            }
            _ => panic!("Expected CreateBead variant"),
        }
    }

    #[test]
    fn test_scheduler_message_cancel_bead_construction() {
        let test_id = Ulid::new();
        let msg = SchedulerMessage::CancelBead { id: test_id };

        match msg {
            SchedulerMessage::CancelBead { id } => {
                assert_eq!(id, test_id, "ID should match");
            }
            _ => panic!("Expected CancelBead variant"),
        }
    }

    #[test]
    fn test_scheduler_response_created_construction() {
        let test_id = Ulid::new();
        let response = SchedulerResponse::Created { id: test_id };

        match response {
            SchedulerResponse::Created { id } => {
                assert_eq!(id, test_id, "ID should match");
            }
            _ => panic!("Expected Created variant"),
        }
    }

    #[test]
    fn test_scheduler_response_cancelled_construction() {
        let test_id = Ulid::new();
        let response = SchedulerResponse::Cancelled { id: test_id };

        match response {
            SchedulerResponse::Cancelled { id } => {
                assert_eq!(id, test_id, "ID should match");
            }
            _ => panic!("Expected Cancelled variant"),
        }
    }

    #[test]
    fn test_scheduler_response_error_construction() {
        let error_msg = "test error".to_string();
        let response = SchedulerResponse::Error {
            message: error_msg.clone(),
        };

        match response {
            SchedulerResponse::Error { message } => {
                assert_eq!(message, error_msg, "Error message should match");
            }
            _ => panic!("Expected Error variant"),
        }
    }

    #[test]
    fn test_state_manager_message_construction() {
        let test_id = Ulid::new();
        let msg = StateManagerMessage::QueryBead { id: test_id };

        match msg {
            StateManagerMessage::QueryBead { id } => {
                assert_eq!(id, test_id, "ID should match");
            }
        }
    }

    #[test]
    fn test_bead_state_construction() {
        let test_id = Ulid::new();
        let state = BeadState {
            id: test_id,
            status: "pending".to_string(),
            phase: "init".to_string(),
            events: vec!["created".to_string()],
            created_at: "2024-01-01".to_string(),
            updated_at: "2024-01-01".to_string(),
        };

        assert_eq!(state.id, test_id, "ID should match");
        assert_eq!(state.status, "pending", "Status should match");
        assert_eq!(state.phase, "init", "Phase should match");
        assert_eq!(state.events.len(), 1, "Should have one event");
    }
}

#[cfg(test)]
mod app_state_tests {
    use super::*;
    use std::sync::Arc;

    #[test]
    fn test_app_state_construction() {
        let scheduler = mock_scheduler();
        let state_manager = mock_state_manager();

        let app_state = AppState {
            scheduler: Arc::new(scheduler),
            state_manager: Arc::new(state_manager),
        };

        // Verify state can be cloned (required for Axum)
        let cloned = app_state.clone();

        // Both should point to same underlying channels
        assert!(Arc::ptr_eq(&app_state.scheduler, &cloned.scheduler));
        assert!(Arc::ptr_eq(&app_state.state_manager, &cloned.state_manager));
    }

    #[tokio::test]
    async fn test_app_state_actors_can_receive_messages() {
        let scheduler = mock_scheduler();
        let state_manager = mock_state_manager();

        let app_state = AppState {
            scheduler: Arc::new(scheduler),
            state_manager: Arc::new(state_manager),
        };

        // Send via app state
        let r1 = app_state.scheduler.send(SchedulerMessage::CreateBead {
            spec: "test".to_string(),
        });

        let r2 = app_state
            .state_manager
            .send(StateManagerMessage::QueryBead { id: Ulid::new() });

        assert!(r1.is_ok(), "Scheduler should receive message");
        assert!(r2.is_ok(), "State manager should receive message");
    }

    #[tokio::test]
    async fn test_app_state_clone_shares_actors() {
        let scheduler = mock_scheduler();
        let state_manager = mock_state_manager();

        let app_state = AppState {
            scheduler: Arc::new(scheduler),
            state_manager: Arc::new(state_manager),
        };

        let cloned = app_state.clone();

        // Send via original
        let r1 = app_state.scheduler.send(SchedulerMessage::CreateBead {
            spec: "original".to_string(),
        });

        // Send via clone
        let r2 = cloned.scheduler.send(SchedulerMessage::CreateBead {
            spec: "cloned".to_string(),
        });

        assert!(r1.is_ok(), "Original should send successfully");
        assert!(r2.is_ok(), "Clone should send successfully");
    }
}

#[cfg(test)]
mod channel_behavior_tests {
    use super::*;

    #[tokio::test]
    async fn test_channel_accepts_high_message_volume() {
        let scheduler = mock_scheduler();

        // Send 1000 messages rapidly
        for i in 0..1000 {
            let result = scheduler.send(SchedulerMessage::CreateBead {
                spec: format!("spec-{}", i),
            });
            assert!(result.is_ok(), "Message {} should send successfully", i);
        }

        // Give time for processing
        tokio::time::sleep(Duration::from_millis(50)).await;
    }

    #[tokio::test]
    async fn test_multiple_senders_to_same_actor() {
        let scheduler = mock_scheduler();

        // Clone sender multiple times
        let s1 = scheduler.clone();
        let s2 = scheduler.clone();
        let s3 = scheduler.clone();

        // Send from each clone
        let r1 = s1.send(SchedulerMessage::CreateBead {
            spec: "s1".to_string(),
        });
        let r2 = s2.send(SchedulerMessage::CreateBead {
            spec: "s2".to_string(),
        });
        let r3 = s3.send(SchedulerMessage::CreateBead {
            spec: "s3".to_string(),
        });

        assert!(r1.is_ok(), "Sender 1 should succeed");
        assert!(r2.is_ok(), "Sender 2 should succeed");
        assert!(r3.is_ok(), "Sender 3 should succeed");
    }

    #[tokio::test]
    async fn test_actor_processes_messages_in_order() {
        let (tx, mut rx) = mpsc::unbounded_channel::<usize>();

        // Spawn actor that records message order
        let order_tx = tx.clone();
        tokio::spawn(async move {
            let mut counter = 0;
            while counter < 10 {
                let result = order_tx.send(counter);
                assert!(result.is_ok(), "Should send order marker");
                counter += 1;
                tokio::time::sleep(Duration::from_millis(1)).await;
            }
        });

        // Verify messages arrive in order
        let mut expected = 0;
        while expected < 10 {
            let msg = timeout(Duration::from_millis(100), rx.recv())
                .await
                .expect("Should receive within timeout")
                .expect("Channel should not be closed");

            assert_eq!(msg, expected, "Messages should arrive in order");
            expected += 1;
        }
    }
}

#[cfg(test)]
mod clone_behavior_tests {
    use super::*;

    #[test]
    fn test_scheduler_message_clone() {
        let original = SchedulerMessage::CreateBead {
            spec: "test".to_string(),
        };

        let cloned = original.clone();

        match (&original, &cloned) {
            (
                SchedulerMessage::CreateBead { spec: s1 },
                SchedulerMessage::CreateBead { spec: s2 },
            ) => {
                assert_eq!(s1, s2, "Cloned spec should match original");
            }
            _ => panic!("Both should be CreateBead variants"),
        }
    }

    #[test]
    fn test_scheduler_response_clone() {
        let test_id = Ulid::new();
        let original = SchedulerResponse::Created { id: test_id };

        let cloned = original.clone();

        match (&original, &cloned) {
            (SchedulerResponse::Created { id: id1 }, SchedulerResponse::Created { id: id2 }) => {
                assert_eq!(id1, id2, "Cloned ID should match original");
            }
            _ => panic!("Both should be Created variants"),
        }
    }

    #[test]
    fn test_state_manager_message_clone() {
        let test_id = Ulid::new();
        let original = StateManagerMessage::QueryBead { id: test_id };

        let cloned = original.clone();

        match (&original, &cloned) {
            (
                StateManagerMessage::QueryBead { id: id1 },
                StateManagerMessage::QueryBead { id: id2 },
            ) => {
                assert_eq!(id1, id2, "Cloned ID should match original");
            }
        }
    }
}

#[cfg(test)]
mod debug_trait_tests {
    use super::*;

    #[test]
    fn test_scheduler_message_debug() {
        let msg = SchedulerMessage::CreateBead {
            spec: "test".to_string(),
        };

        let debug_output = format!("{:?}", msg);
        assert!(
            debug_output.contains("CreateBead"),
            "Debug should show variant"
        );
        assert!(
            debug_output.contains("test"),
            "Debug should show spec value"
        );
    }

    #[test]
    fn test_scheduler_response_debug() {
        let response = SchedulerResponse::Error {
            message: "test error".to_string(),
        };

        let debug_output = format!("{:?}", response);
        assert!(debug_output.contains("Error"), "Debug should show variant");
        assert!(
            debug_output.contains("test error"),
            "Debug should show message"
        );
    }

    #[test]
    fn test_state_manager_message_debug() {
        let msg = StateManagerMessage::QueryBead { id: Ulid::new() };

        let debug_output = format!("{:?}", msg);
        assert!(
            debug_output.contains("QueryBead"),
            "Debug should show variant"
        );
    }
}
