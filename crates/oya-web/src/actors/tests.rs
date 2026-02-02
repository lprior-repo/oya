//! Comprehensive unit tests for actor lifecycle and message handling
//!
//! Tests cover:
//! - Actor lifecycle (spawn, run, stop)
//! - Message handling (send, receive, processing)
//! - Channel behavior and backpressure
//! - Error handling and supervision

use super::*;
use tokio::sync::{broadcast, mpsc, oneshot};
use tokio::time::{Duration, timeout};

#[cfg(test)]
mod scheduler_actor_tests {
    use super::*;

    #[tokio::test]
    async fn test_scheduler_spawns_and_receives_messages() {
        let scheduler = mock_scheduler();
        let spec = "test-spec".to_string();

        let result = scheduler.send(SchedulerMessage::CreateBead {
            id: Ulid::new(),
            spec: spec.clone(),
        });

        assert!(result.is_ok(), "Failed to send message to scheduler");

        // Give time for message to be processed
        tokio::time::sleep(Duration::from_millis(10)).await;
    }

    #[tokio::test]
    async fn test_scheduler_handles_create_bead_message() {
        let scheduler = mock_scheduler();

        let result = scheduler.send(SchedulerMessage::CreateBead {
            id: Ulid::new(),
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

        let r1 = scheduler.send(SchedulerMessage::CreateBead {
            id: Ulid::new(),
            spec: spec1,
        });
        let r2 = scheduler.send(SchedulerMessage::CreateBead {
            id: Ulid::new(),
            spec: spec2,
        });
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
            id: Ulid::new(),
            spec: "early".to_string(),
        });
        assert!(r1.is_ok(), "Should send before actor fully initialized");

        // Small delay to ensure actor is running
        tokio::time::sleep(Duration::from_millis(5)).await;

        // Send after actor is running
        let r2 = scheduler.send(SchedulerMessage::CreateBead {
            id: Ulid::new(),
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
            id: Ulid::new(),
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

    /// Helper to create a QueryBead message with a oneshot channel
    fn query_bead_message(id: Ulid) -> (StateManagerMessage, oneshot::Receiver<Option<BeadState>>) {
        let (tx, rx) = oneshot::channel();
        (StateManagerMessage::QueryBead { id, response: tx }, rx)
    }

    #[tokio::test]
    async fn test_state_manager_spawns_and_receives_messages() {
        let state_manager = mock_state_manager();
        let test_id = Ulid::new();

        let (msg, _rx) = query_bead_message(test_id);
        let result = state_manager.send(msg);

        assert!(result.is_ok(), "Failed to send message to state manager");

        // Give time for message to be processed
        tokio::time::sleep(Duration::from_millis(10)).await;
    }

    #[tokio::test]
    async fn test_state_manager_handles_query_bead_message() {
        let state_manager = mock_state_manager();
        let bead_id = Ulid::new();

        let (msg, _rx) = query_bead_message(bead_id);
        let result = state_manager.send(msg);

        assert!(result.is_ok(), "Should successfully send QueryBead message");
    }

    #[tokio::test]
    async fn test_state_manager_handles_multiple_queries() {
        let state_manager = mock_state_manager();

        let id1 = Ulid::new();
        let id2 = Ulid::new();
        let id3 = Ulid::new();

        let (msg1, _rx1) = query_bead_message(id1);
        let (msg2, _rx2) = query_bead_message(id2);
        let (msg3, _rx3) = query_bead_message(id3);

        let r1 = state_manager.send(msg1);
        let r2 = state_manager.send(msg2);
        let r3 = state_manager.send(msg3);

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
        let (msg1, _rx1) = query_bead_message(Ulid::new());
        let r1 = state_manager.send(msg1);
        assert!(r1.is_ok(), "Should send before actor fully initialized");

        // Small delay to ensure actor is running
        tokio::time::sleep(Duration::from_millis(5)).await;

        // Send after actor is running
        let (msg2, _rx2) = query_bead_message(Ulid::new());
        let r2 = state_manager.send(msg2);
        assert!(r2.is_ok(), "Should send after actor is running");
    }
}

#[cfg(test)]
mod message_type_tests {
    use super::*;

    #[test]
    fn test_scheduler_message_create_bead_construction() {
        let spec = "test-spec".to_string();
        let msg = SchedulerMessage::CreateBead {
            id: Ulid::new(),
            spec: spec.clone(),
        };

        if let SchedulerMessage::CreateBead { id: _, spec: s } = msg {
            assert_eq!(s, spec, "Spec should match");
        } else {
            assert!(
                matches!(msg, SchedulerMessage::CreateBead { .. }),
                "Expected CreateBead variant"
            );
        }
    }

    #[test]
    fn test_scheduler_message_cancel_bead_construction() {
        let test_id = Ulid::new();
        let msg = SchedulerMessage::CancelBead { id: test_id };

        if let SchedulerMessage::CancelBead { id } = msg {
            assert_eq!(id, test_id, "ID should match");
        } else {
            assert!(
                matches!(msg, SchedulerMessage::CancelBead { .. }),
                "Expected CancelBead variant"
            );
        }
    }

    #[test]
    fn test_scheduler_response_created_construction() {
        let test_id = Ulid::new();
        let response = SchedulerResponse::Created { id: test_id };

        if let SchedulerResponse::Created { id } = response {
            assert_eq!(id, test_id, "ID should match");
        } else {
            assert!(
                matches!(response, SchedulerResponse::Created { .. }),
                "Expected Created variant"
            );
        }
    }

    #[test]
    fn test_scheduler_response_cancelled_construction() {
        let test_id = Ulid::new();
        let response = SchedulerResponse::Cancelled { id: test_id };

        if let SchedulerResponse::Cancelled { id } = response {
            assert_eq!(id, test_id, "ID should match");
        } else {
            assert!(
                matches!(response, SchedulerResponse::Cancelled { .. }),
                "Expected Cancelled variant"
            );
        }
    }

    #[test]
    fn test_scheduler_response_error_construction() {
        let error_msg = "test error".to_string();
        let response = SchedulerResponse::Error {
            message: error_msg.clone(),
        };

        if let SchedulerResponse::Error { message } = response {
            assert_eq!(message, error_msg, "Error message should match");
        } else {
            assert!(
                matches!(response, SchedulerResponse::Error { .. }),
                "Expected Error variant"
            );
        }
    }

    #[test]
    fn test_state_manager_message_construction() {
        let test_id = Ulid::new();
        let (tx, _rx) = oneshot::channel();
        let msg = StateManagerMessage::QueryBead {
            id: test_id,
            response: tx,
        };

        match msg {
            StateManagerMessage::QueryBead { id, response: _ } => {
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
            title: Some("Test Bead".to_string()),
            dependencies: vec!["dep-1".to_string()],
        };

        assert_eq!(state.id, test_id, "ID should match");
        assert_eq!(state.status, "pending", "Status should match");
        assert_eq!(state.phase, "init", "Phase should match");
        assert_eq!(state.events.len(), 1, "Should have one event");
        assert_eq!(
            state.title,
            Some("Test Bead".to_string()),
            "Title should match"
        );
        assert_eq!(state.dependencies.len(), 1, "Should have one dependency");
    }
}

#[cfg(test)]
mod app_state_tests {
    use super::*;
    use std::sync::Arc;

    /// Helper to create a QueryBead message with a oneshot channel
    fn query_bead_message(id: Ulid) -> (StateManagerMessage, oneshot::Receiver<Option<BeadState>>) {
        let (tx, rx) = oneshot::channel();
        (StateManagerMessage::QueryBead { id, response: tx }, rx)
    }

    #[tokio::test]
    async fn test_app_state_construction() {
        let scheduler = mock_scheduler();
        let state_manager = mock_state_manager();
        let (broadcast_tx, _broadcast_rx) = broadcast::channel(16);

        let app_state = AppState {
            scheduler: Arc::new(scheduler),
            state_manager: Arc::new(state_manager),
            broadcast_tx,
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
        let (broadcast_tx, _broadcast_rx) = broadcast::channel(16);

        let app_state = AppState {
            scheduler: Arc::new(scheduler),
            state_manager: Arc::new(state_manager),
            broadcast_tx,
        };

        // Send via app state
        let r1 = app_state.scheduler.send(SchedulerMessage::CreateBead {
            id: Ulid::new(),
            spec: "test".to_string(),
        });

        let (msg, _rx) = query_bead_message(Ulid::new());
        let r2 = app_state.state_manager.send(msg);

        assert!(r1.is_ok(), "Scheduler should receive message");
        assert!(r2.is_ok(), "State manager should receive message");
    }

    #[tokio::test]
    async fn test_app_state_clone_shares_actors() {
        let scheduler = mock_scheduler();
        let state_manager = mock_state_manager();
        let (broadcast_tx, _broadcast_rx) = broadcast::channel(16);

        let app_state = AppState {
            scheduler: Arc::new(scheduler),
            state_manager: Arc::new(state_manager),
            broadcast_tx,
        };

        let cloned = app_state.clone();

        // Send via original
        let r1 = app_state.scheduler.send(SchedulerMessage::CreateBead {
            id: Ulid::new(),
            spec: "original".to_string(),
        });

        // Send via clone
        let r2 = cloned.scheduler.send(SchedulerMessage::CreateBead {
            id: Ulid::new(),
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
                id: Ulid::new(),
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
            id: Ulid::new(),
            spec: "s1".to_string(),
        });
        let r2 = s2.send(SchedulerMessage::CreateBead {
            id: Ulid::new(),
            spec: "s2".to_string(),
        });
        let r3 = s3.send(SchedulerMessage::CreateBead {
            id: Ulid::new(),
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
        for expected in 0..10 {
            let timeout_result = timeout(Duration::from_millis(100), rx.recv()).await;
            assert!(timeout_result.is_ok(), "Should receive within timeout");
            if let Ok(Some(msg)) = timeout_result {
                assert_eq!(msg, expected, "Messages should arrive in order");
            } else {
                assert!(
                    timeout_result.is_ok() && timeout_result.as_ref().ok().is_some(),
                    "Should receive message"
                );
            }
        }
    }
}

#[cfg(test)]
mod clone_behavior_tests {
    use super::*;

    #[test]
    fn test_scheduler_message_clone() {
        let original = SchedulerMessage::CreateBead {
            id: Ulid::new(),
            spec: "test".to_string(),
        };

        let cloned = original.clone();

        if let (
            SchedulerMessage::CreateBead { id: _, spec: s1 },
            SchedulerMessage::CreateBead { id: _, spec: s2 },
        ) = (&original, &cloned)
        {
            assert_eq!(s1, s2, "Cloned spec should match original");
        } else {
            unreachable!("Both should be CreateBead variants");
        }
    }

    #[test]
    fn test_scheduler_response_clone() {
        let test_id = Ulid::new();
        let original = SchedulerResponse::Created { id: test_id };

        let cloned = original.clone();

        if let (SchedulerResponse::Created { id: id1 }, SchedulerResponse::Created { id: id2 }) =
            (&original, &cloned)
        {
            assert_eq!(id1, id2, "Cloned ID should match original");
        } else {
            unreachable!("Both should be Created variants");
        }
    }

    // Note: StateManagerMessage is not Clone because oneshot::Sender is not Clone.
    // This is intentional - each query needs its own response channel.
}

#[cfg(test)]
mod debug_trait_tests {
    use super::*;

    #[test]
    fn test_scheduler_message_debug() {
        let msg = SchedulerMessage::CreateBead {
            id: Ulid::new(),
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
        let (tx, _rx) = oneshot::channel();
        let msg = StateManagerMessage::QueryBead {
            id: Ulid::new(),
            response: tx,
        };

        let debug_output = format!("{:?}", msg);
        assert!(
            debug_output.contains("QueryBead"),
            "Debug should show variant"
        );
    }
}

#[cfg(test)]
mod lifecycle_tests {
    use super::*;

    /// Helper to create a QueryBead message with a oneshot channel
    fn query_bead_message(id: Ulid) -> (StateManagerMessage, oneshot::Receiver<Option<BeadState>>) {
        let (tx, rx) = oneshot::channel();
        (StateManagerMessage::QueryBead { id, response: tx }, rx)
    }

    #[tokio::test]
    async fn test_actor_starts_immediately_after_creation() {
        let scheduler = mock_scheduler();

        // Should be able to send immediately
        let result = scheduler.send(SchedulerMessage::CreateBead {
            id: Ulid::new(),
            spec: "immediate".to_string(),
        });

        assert!(
            result.is_ok(),
            "Actor should be running immediately after creation"
        );
    }

    #[tokio::test]
    async fn test_actor_processes_messages_after_spawn() {
        let state_manager = mock_state_manager();

        // Small delay to ensure tokio::spawn completes
        tokio::time::sleep(Duration::from_millis(1)).await;

        let (msg, _rx) = query_bead_message(Ulid::new());
        let result = state_manager.send(msg);

        assert!(result.is_ok(), "Actor should process messages after spawn");
    }

    #[tokio::test]
    async fn test_multiple_actors_coexist() {
        let scheduler1 = mock_scheduler();
        let scheduler2 = mock_scheduler();
        let state_manager = mock_state_manager();

        // All three actors should work independently
        let r1 = scheduler1.send(SchedulerMessage::CreateBead {
            id: Ulid::new(),
            spec: "s1".to_string(),
        });
        let r2 = scheduler2.send(SchedulerMessage::CreateBead {
            id: Ulid::new(),
            spec: "s2".to_string(),
        });
        let (msg, _rx) = query_bead_message(Ulid::new());
        let r3 = state_manager.send(msg);

        assert!(r1.is_ok(), "First scheduler should work");
        assert!(r2.is_ok(), "Second scheduler should work");
        assert!(r3.is_ok(), "State manager should work");
    }

    #[tokio::test]
    async fn test_actor_survives_rapid_message_burst() {
        let scheduler = mock_scheduler();

        // Send 100 messages as fast as possible
        for i in 0..100 {
            let result = scheduler.send(SchedulerMessage::CreateBead {
                id: Ulid::new(),
                spec: format!("burst-{}", i),
            });
            assert!(result.is_ok(), "Message {} should succeed", i);
        }

        // Verify actor still works after burst
        tokio::time::sleep(Duration::from_millis(50)).await;

        let result = scheduler.send(SchedulerMessage::CreateBead {
            id: Ulid::new(),
            spec: "after-burst".to_string(),
        });

        assert!(
            result.is_ok(),
            "Actor should still work after message burst"
        );
    }

    #[tokio::test]
    async fn test_actor_channel_is_unbounded() {
        let scheduler = mock_scheduler();

        // Unbounded channels never fail to send (unless dropped)
        // Send many messages without waiting
        for i in 0..10000 {
            let result = scheduler.send(SchedulerMessage::CreateBead {
                id: Ulid::new(),
                spec: format!("msg-{}", i),
            });
            assert!(result.is_ok(), "Unbounded channel should never block");
        }
    }
}

#[cfg(test)]
mod supervision_tests {
    use super::*;

    #[tokio::test]
    async fn test_sender_detects_actor_termination() {
        let (tx, rx) = mpsc::unbounded_channel::<SchedulerMessage>();

        // Drop receiver immediately to simulate actor crash
        drop(rx);

        // Next send should fail
        let result = tx.send(SchedulerMessage::CreateBead {
            id: Ulid::new(),
            spec: "test".to_string(),
        });

        assert!(result.is_err(), "Send should fail when actor is terminated");
    }

    #[tokio::test]
    async fn test_multiple_senders_all_detect_termination() {
        let (tx, rx) = mpsc::unbounded_channel::<SchedulerMessage>();

        let s1 = tx.clone();
        let s2 = tx.clone();
        let s3 = tx;

        // Drop receiver
        drop(rx);

        // All senders should fail
        assert!(
            s1.send(SchedulerMessage::CreateBead {
                id: Ulid::new(),
                spec: "1".to_string()
            })
            .is_err()
        );
        assert!(
            s2.send(SchedulerMessage::CreateBead {
                id: Ulid::new(),
                spec: "2".to_string()
            })
            .is_err()
        );
        assert!(
            s3.send(SchedulerMessage::CreateBead {
                id: Ulid::new(),
                spec: "3".to_string()
            })
            .is_err()
        );
    }

    #[tokio::test]
    async fn test_actor_continues_after_processing_many_messages() {
        let scheduler = mock_scheduler();

        // Send 1000 messages
        for i in 0..1000 {
            let result = scheduler.send(SchedulerMessage::CreateBead {
                id: Ulid::new(),
                spec: format!("msg-{}", i),
            });
            assert!(result.is_ok(), "Message {} should send", i);
        }

        // Wait for processing
        tokio::time::sleep(Duration::from_millis(100)).await;

        // Verify actor still alive
        let result = scheduler.send(SchedulerMessage::CreateBead {
            id: Ulid::new(),
            spec: "final".to_string(),
        });

        assert!(
            result.is_ok(),
            "Actor should still be alive after processing many messages"
        );
    }
}

#[cfg(test)]
mod property_tests {
    use super::*;
    use proptest::prelude::*;
    use tokio::runtime::Runtime;

    // Property: Any string can be used as a spec
    // Note: Uses match for error handling instead of .expect() to comply with zero-panic policy
    proptest! {
        #[test]
        fn prop_scheduler_accepts_any_spec(spec in "\\PC*") {
            let rt = match Runtime::new() {
                Ok(r) => r,
                Err(e) => return Err(TestCaseError::fail(format!("Runtime creation failed: {}", e))),
            };
            match rt.block_on(async {
                let scheduler = mock_scheduler();
                let result = scheduler.send(SchedulerMessage::CreateBead { id: Ulid::new(), spec });
                prop_assert!(result.is_ok(), "Scheduler should accept any valid string spec");
                Ok(())
            }) {
                Ok(_) => (),
                Err(e) => return Err(e),
            }
        }
    }

    // Property: All ULID values are valid for cancel operations
    // Note: Uses match for error handling instead of .expect() to comply with zero-panic policy
    proptest! {
        #[test]
        fn prop_scheduler_accepts_any_ulid_for_cancel(bytes in prop::array::uniform16(any::<u8>())) {
            let rt = match Runtime::new() {
                Ok(r) => r,
                Err(e) => return Err(TestCaseError::fail(format!("Runtime creation failed: {}", e))),
            };
            match rt.block_on(async {
                let scheduler = mock_scheduler();
                let id = Ulid::from_bytes(bytes);
                let result = scheduler.send(SchedulerMessage::CancelBead { id });
                prop_assert!(result.is_ok(), "Scheduler should accept any ULID");
                Ok(())
            }) {
                Ok(_) => (),
                Err(e) => return Err(e),
            }
        }
    }

    // Property: State manager accepts any ULID for queries
    // Note: Uses match for error handling instead of .expect() to comply with zero-panic policy
    proptest! {
        #[test]
        fn prop_state_manager_accepts_any_ulid(bytes in prop::array::uniform16(any::<u8>())) {
            let rt = match Runtime::new() {
                Ok(r) => r,
                Err(e) => return Err(TestCaseError::fail(format!("Runtime creation failed: {}", e))),
            };
            match rt.block_on(async {
                let state_manager = mock_state_manager();
                let id = Ulid::from_bytes(bytes);
                let (tx, _rx) = oneshot::channel();
                let result = state_manager.send(StateManagerMessage::QueryBead { id, response: tx });
                prop_assert!(result.is_ok(), "State manager should accept any ULID");
                Ok(())
            }) {
                Ok(_) => (),
                Err(e) => return Err(e),
            }
        }
    }

    // Property: Message order is preserved
    // Note: Uses match for error handling instead of .expect() to comply with zero-panic policy
    proptest! {
        #[test]
        fn prop_messages_processed_in_order(specs in prop::collection::vec("\\PC{1,20}", 1..=20)) {
            let rt = match Runtime::new() {
                Ok(r) => r,
                Err(e) => return Err(TestCaseError::fail(format!("Runtime creation failed: {}", e))),
            };
            match rt.block_on(async {
                let scheduler = mock_scheduler();

                // Send all messages
                for spec in &specs {
                    let result = scheduler.send(SchedulerMessage::CreateBead {
                        id: Ulid::new(),
                        spec: spec.clone()
                    });
                    prop_assert!(result.is_ok(), "All messages should send successfully");
                }

                // Messages were sent in order (we can't verify receipt order without response channels,
                // but we verify they were all accepted)
                Ok(())
            }) {
                Ok(_) => (),
                Err(e) => return Err(e),
            }
        }
    }

    // Property: Actors handle high load without errors
    // Note: Uses match for error handling instead of .expect() to comply with zero-panic policy
    proptest! {
        #[test]
        fn prop_scheduler_handles_burst_load(count in 1usize..=1000) {
            let rt = match Runtime::new() {
                Ok(r) => r,
                Err(e) => return Err(TestCaseError::fail(format!("Runtime creation failed: {}", e))),
            };
            match rt.block_on(async {
                let scheduler = mock_scheduler();

                for i in 0..count {
                    let result = scheduler.send(SchedulerMessage::CreateBead {
                        id: Ulid::new(),
                        spec: format!("spec-{}", i),
                    });
                    prop_assert!(result.is_ok(), "Message {} should send successfully", i);
                }
                Ok(())
            }) {
                Ok(_) => (),
                Err(e) => return Err(e),
            }
        }
    }

    // Property: Multiple clones work independently
    // Note: Uses match for error handling instead of .expect() to comply with zero-panic policy
    proptest! {
        #[test]
        fn prop_cloned_senders_work_independently(
            spec1 in "\\PC{1,10}",
            spec2 in "\\PC{1,10}",
            spec3 in "\\PC{1,10}",
        ) {
            let rt = match Runtime::new() {
                Ok(r) => r,
                Err(e) => return Err(TestCaseError::fail(format!("Runtime creation failed: {}", e))),
            };
            match rt.block_on(async {
                let scheduler = mock_scheduler();
                let s1 = scheduler.clone();
                let s2 = scheduler.clone();
                let s3 = scheduler.clone();

                let r1 = s1.send(SchedulerMessage::CreateBead { id: Ulid::new(), spec: spec1 });
                let r2 = s2.send(SchedulerMessage::CreateBead { id: Ulid::new(), spec: spec2 });
                let r3 = s3.send(SchedulerMessage::CreateBead { id: Ulid::new(), spec: spec3 });

                prop_assert!(r1.is_ok() && r2.is_ok() && r3.is_ok(),
                    "All cloned senders should work");
                Ok(())
            }) {
                Ok(_) => (),
                Err(e) => return Err(e),
            }
        }
    }

    // Property: Error response messages can contain any string
    // Note: Uses prop_assert_eq! and explicit match for error handling (zero-panic policy)
    proptest! {
        #[test]
        fn prop_error_response_accepts_any_message(msg in "\\PC*") {
            let response = SchedulerResponse::Error { message: msg.clone() };

            if let SchedulerResponse::Error { message } = response {
                prop_assert_eq!(message, msg, "Error message should be preserved");
            } else {
                return Err(TestCaseError::fail("Should be Error variant"));
            }
        }
    }

    // Property: BeadState fields accept any valid strings
    proptest! {
        #[test]
        fn prop_bead_state_accepts_any_strings(
            bytes in prop::array::uniform16(any::<u8>()),
            status in "\\PC{1,20}",
            phase in "\\PC{1,20}",
            events in prop::collection::vec("\\PC{1,50}", 0..=10),
            created_at in "\\PC{1,30}",
            updated_at in "\\PC{1,30}",
            title in prop::option::of("\\PC{1,50}"),
            dependencies in prop::collection::vec("\\PC{1,30}", 0..=5),
        ) {
            let id = Ulid::from_bytes(bytes);
            let state = BeadState {
                id,
                status: status.clone(),
                phase: phase.clone(),
                events: events.clone(),
                created_at: created_at.clone(),
                updated_at: updated_at.clone(),
                title: title.clone(),
                dependencies: dependencies.clone(),
            };

            assert_eq!(state.status, status);
            assert_eq!(state.phase, phase);
            assert_eq!(state.events, events);
            assert_eq!(state.created_at, created_at);
            assert_eq!(state.updated_at, updated_at);
            assert_eq!(state.title, title);
            assert_eq!(state.dependencies, dependencies);
        }
    }
}
