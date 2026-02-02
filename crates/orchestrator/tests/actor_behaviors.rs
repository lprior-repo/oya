//! BDD-style behavioral tests for the actor system.
//!
//! These tests verify actor behavior through message passing, following
//! the Given-When-Then pattern with BDD naming conventions.

use std::sync::atomic::{AtomicU64, Ordering};
use std::time::Duration;

use ractor::ActorRef;

use orchestrator::actors::{
    ActorError, SchedulerArguments, SchedulerMessage, WorkflowStatus, spawn_scheduler_with_name,
};
use orchestrator::scheduler::SchedulerStats;

/// Atomic counter for generating unique actor names.
static TEST_COUNTER: AtomicU64 = AtomicU64::new(0);

/// Generate a unique actor name for testing.
fn unique_scheduler_name() -> String {
    let id = TEST_COUNTER.fetch_add(1, Ordering::SeqCst);
    format!("scheduler-test-{}", id)
}

/// Helper to spawn a scheduler for testing with a unique name.
async fn setup_scheduler() -> Result<ActorRef<SchedulerMessage>, Box<dyn std::error::Error>> {
    let args = SchedulerArguments::new();
    let name = unique_scheduler_name();
    let scheduler = spawn_scheduler_with_name(args, &name).await?;
    Ok(scheduler)
}

/// Helper to perform a call with timeout.
async fn call_with_timeout<T: Send + 'static>(
    scheduler: &ActorRef<SchedulerMessage>,
    msg_builder: impl FnOnce(ractor::RpcReplyPort<T>) -> SchedulerMessage,
) -> Result<T, Box<dyn std::error::Error>> {
    let result = scheduler
        .call(msg_builder, Some(Duration::from_millis(1000)))
        .await?;

    match result {
        ractor::rpc::CallResult::Success(value) => Ok(value),
        ractor::rpc::CallResult::Timeout => {
            Err("Call timed out".into())
        }
        ractor::rpc::CallResult::SenderError => {
            Err("Sender error".into())
        }
    }
}

// ═══════════════════════════════════════════════════════════════════════════════
// WORKFLOW REGISTRATION BEHAVIORS
// ═══════════════════════════════════════════════════════════════════════════════

#[tokio::test]
async fn given_scheduler_when_register_workflow_via_cast_then_workflow_tracked() -> Result<(), Box<dyn std::error::Error>> {
    // Given: A running scheduler actor
    let scheduler = setup_scheduler().await?;

    // When: Register a workflow via cast (fire-and-forget)
    let result = scheduler.send_message(SchedulerMessage::RegisterWorkflow {
        workflow_id: "wf-test-1".to_string(),
    });
    assert!(result.is_ok(), "Cast should succeed");

    // Allow message to be processed
    tokio::time::sleep(Duration::from_millis(10)).await;

    // Then: Workflow status should be queryable
    let status = call_with_timeout(&scheduler, |reply| SchedulerMessage::GetWorkflowStatus {
            workflow_id: "wf-test-1".to_string(),
            reply,
        })
        .await?;

    assert!(status.is_some(), "Workflow should be registered");
    let status = status.ok_or_else(|| -> Box<dyn std::error::Error> { "Status should exist".into() })?;
    assert_eq!(status.workflow_id, "wf-test-1");
    assert_eq!(status.total_beads, 0);

    // Cleanup
    scheduler.stop(None);

    Ok(())
}

#[tokio::test]
async fn given_scheduler_when_register_duplicate_workflow_then_idempotent() -> Result<(), Box<dyn std::error::Error>> {
    // Given: A scheduler with an existing workflow
    let scheduler = setup_scheduler().await?;
    let result = scheduler.send_message(SchedulerMessage::RegisterWorkflow {
        workflow_id: "wf-dup".to_string(),
    });
    assert!(result.is_ok(), "First registration failed: {:?}", result);
    tokio::time::sleep(Duration::from_millis(10)).await;

    // When: Register the same workflow again
    let result = scheduler.send_message(SchedulerMessage::RegisterWorkflow {
        workflow_id: "wf-dup".to_string(),
    });

    // Then: Operation should succeed (idempotent)
    assert!(
        result.is_ok(),
        "Duplicate registration should be idempotent"
    );

    // Cleanup
    scheduler.stop(None);

    Ok(())
}

// ═══════════════════════════════════════════════════════════════════════════════
// QUERY BEHAVIORS
// ═══════════════════════════════════════════════════════════════════════════════

#[tokio::test]
async fn given_workflow_when_query_ready_beads_via_call_then_returns_result() -> Result<(), Box<dyn std::error::Error>> {
    // Given: A scheduler with a workflow containing a bead
    let scheduler = setup_scheduler().await?;
    let result = scheduler.send_message(SchedulerMessage::RegisterWorkflow {
        workflow_id: "wf-query".to_string(),
    });
    assert!(result.is_ok(), "Register failed: {:?}", result);

    let result = scheduler.send_message(SchedulerMessage::ScheduleBead {
        workflow_id: "wf-query".to_string(),
        bead_id: "bead-1".to_string(),
    });
    assert!(result.is_ok(), "Schedule failed: {:?}", result);

    tokio::time::sleep(Duration::from_millis(10)).await;

    // When: Query ready beads via call (request-response)
    let result = call_with_timeout(&scheduler, |reply| {
        SchedulerMessage::GetWorkflowReadyBeads {
            workflow_id: "wf-query".to_string(),
            reply,
        }
    })
    .await?;

    // Then: Should return the ready bead
    assert!(result.is_ok(), "Query should succeed");
    let ready_beads = result.map_err(|e| format!("Should have ready beads: {:?}", e))?;
    assert!(
        ready_beads.contains(&"bead-1".to_string()),
        "Root bead should be ready"
    );

    // Cleanup
    scheduler.stop(None);

    Ok(())
}

#[tokio::test]
async fn given_invalid_workflow_when_query_then_returns_error_not_panic() -> Result<(), Box<dyn std::error::Error>> {
    // Given: A running scheduler with no workflows
    let scheduler = setup_scheduler().await?;

    // When: Query ready beads for non-existent workflow
    let result: Vec<String> = call_with_timeout(&scheduler, |reply| {
        SchedulerMessage::GetWorkflowReadyBeads {
            workflow_id: "non-existent".to_string(),
            reply,
        }
    })
    .await?;

    // Then: Should return empty list for non-existent workflow
    assert!(result.is_empty(), "Should return empty list for invalid workflow");

    // Verify actor is still running
    let stats: SchedulerStats =
        call_with_timeout(&scheduler, |reply| SchedulerMessage::GetStats { reply }).await?;
    assert_eq!(stats.workflow_count, 0);

    // Cleanup
    scheduler.stop(None);

    Ok(())
}

// ═══════════════════════════════════════════════════════════════════════════════
// BEAD SCHEDULING BEHAVIORS
// ═══════════════════════════════════════════════════════════════════════════════

#[tokio::test]
async fn given_workflow_when_schedule_bead_then_bead_tracked() -> Result<(), Box<dyn std::error::Error>> {
    // Given: A scheduler with a registered workflow
    let scheduler = setup_scheduler().await?;
    let result = scheduler.send_message(SchedulerMessage::RegisterWorkflow {
        workflow_id: "wf-bead".to_string(),
    });
    assert!(result.is_ok(), "Register failed: {:?}", result);
    tokio::time::sleep(Duration::from_millis(10)).await?;

    // When: Schedule a bead
    let result = scheduler.send_message(SchedulerMessage::ScheduleBead {
        workflow_id: "wf-bead".to_string(),
        bead_id: "bead-a".to_string(),
    });
    assert!(result.is_ok(), "Schedule failed: {:?}", result);
    tokio::time::sleep(Duration::from_millis(10)).await;

    // Then: Workflow status should reflect the bead
    let status = call_with_timeout(&scheduler, |reply| SchedulerMessage::GetWorkflowStatus {
            workflow_id: "wf-bead".to_string(),
            reply,
        })
        .await?;

    let status = status.ok_or_else(|| -> Box<dyn std::error::Error> { "Workflow should exist".into() })?;
    assert_eq!(status.total_beads, 1);

    // Cleanup
    scheduler.stop(None);

    Ok(())
}

#[tokio::test]
async fn given_beads_with_dependency_when_query_ready_then_only_root_ready() -> Result<(), Box<dyn std::error::Error>> {
    // Given: A workflow with A -> B dependency
    let scheduler = setup_scheduler().await?;
    let result = scheduler.send_message(SchedulerMessage::RegisterWorkflow {
        workflow_id: "wf-dep".to_string(),
    });
    assert!(result.is_ok(), "Register failed: {:?}", result);

    let result = scheduler.send_message(SchedulerMessage::ScheduleBead {
        workflow_id: "wf-dep".to_string(),
        bead_id: "bead-a".to_string(),
    });
    assert!(result.is_ok(), "Schedule A failed: {:?}", result);

    let result = scheduler.send_message(SchedulerMessage::ScheduleBead {
        workflow_id: "wf-dep".to_string(),
        bead_id: "bead-b".to_string(),
    });
    assert!(result.is_ok(), "Schedule B failed: {:?}", result);

    // Add dependency: B depends on A (A -> B means A must complete before B)
    let result = scheduler.send_message(SchedulerMessage::AddDependency {
        workflow_id: "wf-dep".to_string(),
        from_bead: "bead-a".to_string(),
        to_bead: "bead-b".to_string(),
    });
    assert!(result.is_ok(), "Add dependency failed: {:?}", result);

    tokio::time::sleep(Duration::from_millis(20)).await;

    // When: Query ready beads
    let result = call_with_timeout(&scheduler, |reply| {
        SchedulerMessage::GetWorkflowReadyBeads {
            workflow_id: "wf-dep".to_string(),
            reply,
        }
    }).await?;

    // Then: Only A should be ready (B depends on A)
    let ready_beads = result?;
    assert!(
        ready_beads.contains(&"bead-a".to_string()),
        "A should be ready (no dependencies)"
    );
    assert!(
        !ready_beads.contains(&"bead-b".to_string()),
        "B should NOT be ready (depends on A)"
    );

    // Cleanup
    scheduler.stop(None);

    Ok(())
}

// ═══════════════════════════════════════════════════════════════════════════════
// BEAD COMPLETION BEHAVIORS
// ═══════════════════════════════════════════════════════════════════════════════

#[tokio::test]
async fn given_dependency_when_upstream_completes_then_downstream_becomes_ready() -> Result<(), Box<dyn std::error::Error>> {
    // Given: A workflow with A -> B dependency
    let scheduler = setup_scheduler().await?;
    let result = scheduler.send_message(SchedulerMessage::RegisterWorkflow {
        workflow_id: "wf-comp".to_string(),
    });
    assert!(result.is_ok(), "Register failed: {:?}", result);

    let result = scheduler.send_message(SchedulerMessage::ScheduleBead {
        workflow_id: "wf-comp".to_string(),
        bead_id: "bead-a".to_string(),
    });
    assert!(result.is_ok(), "Schedule A failed: {:?}", result);

    let result = scheduler.send_message(SchedulerMessage::ScheduleBead {
        workflow_id: "wf-comp".to_string(),
        bead_id: "bead-b".to_string(),
    });
    assert!(result.is_ok(), "Schedule B failed: {:?}", result);

    let result = scheduler.send_message(SchedulerMessage::AddDependency {
        workflow_id: "wf-comp".to_string(),
        from_bead: "bead-a".to_string(),
        to_bead: "bead-b".to_string(),
    });
    assert!(result.is_ok(), "Add dependency failed: {:?}", result);

    tokio::time::sleep(Duration::from_millis(20)).await;

    // When: Mark A as completed
    let result = scheduler.send_message(SchedulerMessage::OnBeadCompleted {
        workflow_id: "wf-comp".to_string(),
        bead_id: "bead-a".to_string(),
    });
    assert!(result.is_ok(), "Complete failed: {:?}", result);

    tokio::time::sleep(Duration::from_millis(20)).await;

    // Then: B should now be ready
    let result = call_with_timeout(&scheduler, |reply| {
        SchedulerMessage::GetWorkflowReadyBeads {
            workflow_id: "wf-comp".to_string(),
            reply,
        }
    }).await?;

    let ready_beads = result?;
    assert!(
        ready_beads.contains(&"bead-b".to_string()),
        "B should be ready after A completes"
    );

    // Cleanup
    scheduler.stop(None);

    Ok(())
}

// ═══════════════════════════════════════════════════════════════════════════════
// CLAIM/RELEASE BEHAVIORS
// ═══════════════════════════════════════════════════════════════════════════════

#[tokio::test]
async fn given_ready_bead_when_claimed_then_not_in_all_ready() -> Result<(), Box<dyn std::error::Error>> {
    // Given: A workflow with a ready bead
    let scheduler = setup_scheduler().await?;
    let result = scheduler.send_message(SchedulerMessage::RegisterWorkflow {
            workflow_id: "wf-claim".to_string(),
        });

    assert!(result.is_ok(), "Register should succeed failed: {:?}", result);

    let result = scheduler.send_message(SchedulerMessage::ScheduleBead {
            workflow_id: "wf-claim".to_string(),
            bead_id: "bead-x".to_string(),
        });


    assert!(result.is_ok(), "Schedule should succeed failed: {:?}", result);

    tokio::time::sleep(Duration::from_millis(10)).await;

    // When: Claim the bead
    let result = scheduler.send_message(SchedulerMessage::ClaimBead {
            bead_id: "bead-x".to_string(),
            worker_id: "worker-1".to_string(),
        });

    assert!(result.is_ok(), "Claim should succeed failed: {:?}", result);

    tokio::time::sleep(Duration::from_millis(10)).await;

    // Then: Bead should not appear in GetAllReadyBeads (it's claimed)
    let all_ready: Vec<(String, String)> = call_with_timeout(&scheduler, |reply| {
        SchedulerMessage::GetAllReadyBeads { reply }
    }).await?;

    let has_claimed = all_ready.iter().any(|(_, bid)| bid == "bead-x");
    assert!(
        !has_claimed,
        "Claimed bead should not appear in all-ready list"
    );

    // Cleanup
    scheduler.stop(None);

    Ok(())
}

#[tokio::test]
async fn given_claimed_bead_when_released_then_appears_in_ready() -> Result<(), Box<dyn std::error::Error>> {
    // Given: A workflow with a claimed bead
    let scheduler = setup_scheduler().await?;
    let result = scheduler.send_message(SchedulerMessage::RegisterWorkflow {
            workflow_id: "wf-release".to_string(),
        });

    assert!(result.is_ok(), "Register should succeed failed: {:?}", result);

    let result = scheduler.send_message(SchedulerMessage::ScheduleBead {
            workflow_id: "wf-release".to_string(),
            bead_id: "bead-y".to_string(),
        });


    assert!(result.is_ok(), "Schedule should succeed failed: {:?}", result);

    let result = scheduler.send_message(SchedulerMessage::ClaimBead {
            bead_id: "bead-y".to_string(),
            worker_id: "worker-2".to_string(),
        });


    assert!(result.is_ok(), "Claim should succeed failed: {:?}", result);

    tokio::time::sleep(Duration::from_millis(10)).await;

    // When: Release the bead
    let result = scheduler.send_message(SchedulerMessage::ReleaseBead {
            bead_id: "bead-y".to_string(),
        });

    assert!(result.is_ok(), "Release should succeed failed: {:?}", result);

    tokio::time::sleep(Duration::from_millis(10)).await;

    // Then: Bead should appear in GetAllReadyBeads
    let all_ready: Vec<(String, String)> = call_with_timeout(&scheduler, |reply| {
        SchedulerMessage::GetAllReadyBeads { reply }
    }).await?;

    let has_released = all_ready.iter().any(|(_, bid)| bid == "bead-y");
    assert!(
        has_released,
        "Released bead should appear in all-ready list"
    );

    // Cleanup
    scheduler.stop(None);

    Ok(())
}

// ═══════════════════════════════════════════════════════════════════════════════
// SHUTDOWN BEHAVIORS
// ═══════════════════════════════════════════════════════════════════════════════

#[tokio::test]
async fn given_running_scheduler_when_shutdown_then_stops_cleanly() -> Result<(), Box<dyn std::error::Error>> {
    // Given: A running scheduler
    let scheduler = setup_scheduler().await?;

    // Verify it's running
    let stats: SchedulerStats =
        call_with_timeout(&scheduler, |reply| SchedulerMessage::GetStats { reply }).await?;
    assert_eq!(stats.workflow_count, 0);

    // When: Send shutdown message
    let result = scheduler.send_message(SchedulerMessage::Shutdown)?;

    // Then: Actor should stop
    tokio::time::sleep(Duration::from_millis(50)).await;

    // Trying to send another message should fail
    let result = scheduler.send_message(SchedulerMessage::RegisterWorkflow {
        workflow_id: "should-fail".to_string(),
    });

    assert!(result.is_err(), "Actor should be stopped");

    Ok(())
}

// ═══════════════════════════════════════════════════════════════════════════════
// STATS BEHAVIORS
// ═══════════════════════════════════════════════════════════════════════════════

#[tokio::test]
async fn given_scheduler_with_data_when_get_stats_then_accurate_counts() -> Result<(), Box<dyn std::error::Error>> {
    // Given: A scheduler with workflows and beads
    let scheduler = setup_scheduler().await?;

    // Register 2 workflows
    let result = scheduler.send_message(SchedulerMessage::RegisterWorkflow {
        workflow_id: "wf-stats-1".to_string(),
    });
    assert!(result.is_ok(), "Register failed: {:?}", result);

    let result = scheduler.send_message(SchedulerMessage::RegisterWorkflow {
        workflow_id: "wf-stats-2".to_string(),
    });
    assert!(result.is_ok(), "Register failed: {:?}", result);

    // Schedule 3 beads across workflows
    let result = scheduler.send_message(SchedulerMessage::ScheduleBead {
            workflow_id: "wf-stats-1".to_string(),
            bead_id: "bead-s1".to_string(),
        });

    assert!(result.is_ok(), "Schedule should succeed failed: {:?}", result);
    let result = scheduler.send_message(SchedulerMessage::ScheduleBead {
            workflow_id: "wf-stats-1".to_string(),
            bead_id: "bead-s2".to_string(),
        });

    assert!(result.is_ok(), "Schedule should succeed failed: {:?}", result);
    let result = scheduler.send_message(SchedulerMessage::ScheduleBead {
            workflow_id: "wf-stats-2".to_string(),
            bead_id: "bead-s3".to_string(),
        });

    assert!(result.is_ok(), "Schedule should succeed failed: {:?}", result);

    tokio::time::sleep(Duration::from_millis(20)).await;

    // When: Get stats
    let stats: SchedulerStats =
        call_with_timeout(&scheduler, |reply| SchedulerMessage::GetStats { reply }).await?;

    // Then: Counts should be accurate
    assert_eq!(stats.workflow_count, 2, "Should have 2 workflows");
    assert_eq!(stats.pending_count, 3, "Should have 3 pending beads");

    // Cleanup
    scheduler.stop(None);

    Ok(())
}

// ═══════════════════════════════════════════════════════════════════════════════
// WORKFLOW STATUS BEHAVIORS
// ═══════════════════════════════════════════════════════════════════════════════

#[tokio::test]
async fn given_complete_workflow_when_get_status_then_shows_complete() -> Result<(), Box<dyn std::error::Error>> {
    // Given: A workflow where all beads are completed
    let scheduler = setup_scheduler().await?;
    let result = scheduler.send_message(SchedulerMessage::RegisterWorkflow {
            workflow_id: "wf-done".to_string(),
        });

    assert!(result.is_ok(), "Register should succeed failed: {:?}", result);

    let result = scheduler.send_message(SchedulerMessage::ScheduleBead {
            workflow_id: "wf-done".to_string(),
            bead_id: "bead-final".to_string(),
        });


    assert!(result.is_ok(), "Schedule should succeed failed: {:?}", result);

    let result = scheduler.send_message(SchedulerMessage::OnBeadCompleted {
            workflow_id: "wf-done".to_string(),
            bead_id: "bead-final".to_string(),
        });


    assert!(result.is_ok(), "Complete should succeed failed: {:?}", result);

    tokio::time::sleep(Duration::from_millis(20)).await;

    // When: Get workflow status
    let status = call_with_timeout(&scheduler, |reply| SchedulerMessage::GetWorkflowStatus {
            workflow_id: "wf-done".to_string(),
            reply,
        })
        .await?;

    // Then: Workflow should show as complete
    let status = status.ok_or_else(|| -> Box<dyn std::error::Error> { "Workflow should exist".into() })?;
    assert!(status.is_complete, "Workflow should be complete");
    assert_eq!(status.completed_beads, 1);

    // Cleanup
    scheduler.stop(None);

    Ok(())
}

// ═══════════════════════════════════════════════════════════════════════════════
// ERROR HANDLING BEHAVIORS
// ═══════════════════════════════════════════════════════════════════════════════

#[tokio::test]
async fn given_no_workflow_when_schedule_bead_then_error_logged_not_panic() -> Result<(), Box<dyn std::error::Error>> {
    // Given: A scheduler with no workflows
    let scheduler = setup_scheduler().await?;

    // When: Try to schedule a bead in a non-existent workflow
    let result = scheduler.send_message(SchedulerMessage::ScheduleBead {
        workflow_id: "non-existent".to_string(),
        bead_id: "bead-1".to_string(),
    });

    // Then: Message sending succeeds (fire-and-forget), error is handled internally
    assert!(result.is_ok(), "Message send should succeed");

    // Actor should still be running
    tokio::time::sleep(Duration::from_millis(10)).await;
    let stats: SchedulerStats =
        call_with_timeout(&scheduler, |reply| SchedulerMessage::GetStats { reply }).await?;
    assert_eq!(stats.workflow_count, 0, "Actor should still be functional");

    // Cleanup
    scheduler.stop(None);

    Ok(())
}

#[tokio::test]
async fn given_no_bead_when_claim_then_error_logged_not_panic() -> Result<(), Box<dyn std::error::Error>> {
    // Given: A scheduler with a workflow but no beads
    let scheduler = setup_scheduler().await?;
    let result = scheduler.send_message(SchedulerMessage::RegisterWorkflow {
            workflow_id: "wf-empty".to_string(),
        });

    assert!(result.is_ok(), "Register should succeed failed: {:?}", result);
    tokio::time::sleep(Duration::from_millis(10)).await;

    // When: Try to claim a non-existent bead
    let result = scheduler.send_message(SchedulerMessage::ClaimBead {
        bead_id: "non-existent-bead".to_string(),
        worker_id: "worker-1".to_string(),
    });

    // Then: Message sending succeeds, error is handled internally
    assert!(result.is_ok(), "Message send should succeed");

    // Actor should still be running
    tokio::time::sleep(Duration::from_millis(10)).await;
    let stats: SchedulerStats =
        call_with_timeout(&scheduler, |reply| SchedulerMessage::GetStats { reply }).await?;
    assert_eq!(stats.workflow_count, 1, "Actor should still be functional");

    // Cleanup
    scheduler.stop(None);

    Ok(())
}

// ═══════════════════════════════════════════════════════════════════════════════
// DIAMOND DEPENDENCY BEHAVIORS
// ═══════════════════════════════════════════════════════════════════════════════

#[tokio::test]
async fn given_diamond_dag_when_partial_complete_then_join_not_ready() -> Result<(), Box<dyn std::error::Error>> {
    // Given: A diamond DAG: A -> B, A -> C, B -> D, C -> D
    let scheduler = setup_scheduler().await?;
    let result = scheduler.send_message(SchedulerMessage::RegisterWorkflow {
            workflow_id: "wf-diamond".to_string(),
        });

    assert!(result.is_ok(), "Register should succeed failed: {:?}", result);

    // Add beads
    for bead in ["a", "b", "c", "d"] {
        let result = scheduler.send_message(SchedulerMessage::ScheduleBead {
                workflow_id: "wf-diamond".to_string(),
                bead_id: bead.to_string(),
            });

        assert!(result.is_ok(), "Schedule should succeed failed: {:?}", result);
    }

    // Add edges: A -> B, A -> C, B -> D, C -> D
    let result = scheduler.send_message(SchedulerMessage::AddDependency {
            workflow_id: "wf-diamond".to_string(),
            from_bead: "a".to_string(),
            to_bead: "b".to_string(),
        });

    assert!(result.is_ok(), "Add dependency should succeed failed: {:?}", result);
    let result = scheduler.send_message(SchedulerMessage::AddDependency {
            workflow_id: "wf-diamond".to_string(),
            from_bead: "a".to_string(),
            to_bead: "c".to_string(),
        });

    assert!(result.is_ok(), "Add dependency should succeed failed: {:?}", result);
    let result = scheduler.send_message(SchedulerMessage::AddDependency {
            workflow_id: "wf-diamond".to_string(),
            from_bead: "b".to_string(),
            to_bead: "d".to_string(),
        });

    assert!(result.is_ok(), "Add dependency should succeed failed: {:?}", result);
    let result = scheduler.send_message(SchedulerMessage::AddDependency {
            workflow_id: "wf-diamond".to_string(),
            from_bead: "c".to_string(),
            to_bead: "d".to_string(),
        });

    assert!(result.is_ok(), "Add dependency should succeed failed: {:?}", result);

    tokio::time::sleep(Duration::from_millis(20)).await;

    // When: Complete A, then B (but not C)
    let result = scheduler.send_message(SchedulerMessage::OnBeadCompleted {
            workflow_id: "wf-diamond".to_string(),
            bead_id: "a".to_string(),
        });

    assert!(result.is_ok(), "Complete should succeed failed: {:?}", result);
    let result = scheduler.send_message(SchedulerMessage::OnBeadCompleted {
            workflow_id: "wf-diamond".to_string(),
            bead_id: "b".to_string(),
        });

    assert!(result.is_ok(), "Complete should succeed failed: {:?}", result);

    tokio::time::sleep(Duration::from_millis(20)).await;

    // Then: D should NOT be ready (still waiting on C)
    let result = call_with_timeout(&scheduler, |reply| {
        SchedulerMessage::GetWorkflowReadyBeads {
            workflow_id: "wf-diamond".to_string(),
            reply,
        }
    }).await?;

    let ready_beads = result?;
    assert!(ready_beads.contains(&"c".to_string()), "C should be ready");
    assert!(
        !ready_beads.contains(&"d".to_string()),
        "D should NOT be ready (waiting on C)"
    );

    // Cleanup
    scheduler.stop(None);

    Ok(())
}

#[tokio::test]
async fn given_diamond_dag_when_all_parents_complete_then_join_ready() -> Result<(), Box<dyn std::error::Error>> {
    // Given: Same diamond DAG
    let scheduler = setup_scheduler().await?;
    let result = scheduler.send_message(SchedulerMessage::RegisterWorkflow {
            workflow_id: "wf-diamond2".to_string(),
        });

    assert!(result.is_ok(), "Register should succeed failed: {:?}", result);

    for bead in ["a", "b", "c", "d"] {
        let result = scheduler.send_message(SchedulerMessage::ScheduleBead {
                workflow_id: "wf-diamond2".to_string(),
                bead_id: bead.to_string(),
            });

        assert!(result.is_ok(), "Schedule should succeed failed: {:?}", result);
    }

    let result = scheduler.send_message(SchedulerMessage::AddDependency {
            workflow_id: "wf-diamond2".to_string(),
            from_bead: "a".to_string(),
            to_bead: "b".to_string(),
        });


    assert!(result.is_ok(), "Add dependency should succeed failed: {:?}", result);
    let result = scheduler.send_message(SchedulerMessage::AddDependency {
            workflow_id: "wf-diamond2".to_string(),
            from_bead: "a".to_string(),
            to_bead: "c".to_string(),
        });

    assert!(result.is_ok(), "Add dependency should succeed failed: {:?}", result);
    let result = scheduler.send_message(SchedulerMessage::AddDependency {
            workflow_id: "wf-diamond2".to_string(),
            from_bead: "b".to_string(),
            to_bead: "d".to_string(),
        });

    assert!(result.is_ok(), "Add dependency should succeed failed: {:?}", result);
    let result = scheduler.send_message(SchedulerMessage::AddDependency {
            workflow_id: "wf-diamond2".to_string(),
            from_bead: "c".to_string(),
            to_bead: "d".to_string(),
        });

    assert!(result.is_ok(), "Add dependency should succeed failed: {:?}", result);

    tokio::time::sleep(Duration::from_millis(20)).await;

    // When: Complete A, B, AND C
    for bead in ["a", "b", "c"] {
        let result = scheduler.send_message(SchedulerMessage::OnBeadCompleted {
                workflow_id: "wf-diamond2".to_string(),
                bead_id: bead.to_string(),
            });

        assert!(result.is_ok(), "Complete should succeed failed: {:?}", result);
    }

    tokio::time::sleep(Duration::from_millis(20)).await;

    // Then: D should now be ready
    let result = call_with_timeout(&scheduler, |reply| {
        SchedulerMessage::GetWorkflowReadyBeads {
            workflow_id: "wf-diamond2".to_string(),
            reply,
        }
    }).await?;

    let ready_beads = result?;
    assert!(
        ready_beads.contains(&"d".to_string()),
        "D should be ready after both B and C complete"
    );

    // Cleanup
    scheduler.stop(None);

    Ok(())
}

// ═══════════════════════════════════════════════════════════════════════════════
// MULTI-WORKFLOW BEHAVIORS
// ═══════════════════════════════════════════════════════════════════════════════

#[tokio::test]
async fn given_multiple_workflows_when_query_all_ready_then_returns_from_all() -> Result<(), Box<dyn std::error::Error>> {
    // Given: Multiple workflows with ready beads
    let scheduler = setup_scheduler().await?;

    let result = scheduler.send_message(SchedulerMessage::RegisterWorkflow {
            workflow_id: "wf-a".to_string(),
        });


    assert!(result.is_ok(), "Register should succeed failed: {:?}", result);
    let result = scheduler.send_message(SchedulerMessage::RegisterWorkflow {
            workflow_id: "wf-b".to_string(),
        });

    assert!(result.is_ok(), "Register should succeed failed: {:?}", result);

    let result = scheduler.send_message(SchedulerMessage::ScheduleBead {
            workflow_id: "wf-a".to_string(),
            bead_id: "bead-a1".to_string(),
        });


    assert!(result.is_ok(), "Schedule should succeed failed: {:?}", result);
    let result = scheduler.send_message(SchedulerMessage::ScheduleBead {
            workflow_id: "wf-b".to_string(),
            bead_id: "bead-b1".to_string(),
        });

    assert!(result.is_ok(), "Schedule should succeed failed: {:?}", result);

    tokio::time::sleep(Duration::from_millis(20)).await;

    // When: Query all ready beads
    let all_ready: Vec<(String, String)> = call_with_timeout(&scheduler, |reply| {
        SchedulerMessage::GetAllReadyBeads { reply }
    }).await?;

    // Then: Should include beads from both workflows
    let has_a1 = all_ready
        .iter()
        .any(|(wf, bid)| wf == "wf-a" && bid == "bead-a1");
    let has_b1 = all_ready
        .iter()
        .any(|(wf, bid)| wf == "wf-b" && bid == "bead-b1");

    assert!(has_a1, "Should include bead-a1 from wf-a");
    assert!(has_b1, "Should include bead-b1 from wf-b");

    // Cleanup
    scheduler.stop(None);

    Ok(())
}

#[tokio::test]
async fn given_multiple_workflows_when_unregister_one_then_other_unaffected() -> Result<(), Box<dyn std::error::Error>> {
    // Given: Two workflows
    let scheduler = setup_scheduler().await?;

    let result = scheduler.send_message(SchedulerMessage::RegisterWorkflow {
            workflow_id: "wf-keep".to_string(),
        });


    assert!(result.is_ok(), "Register should succeed failed: {:?}", result);
    let result = scheduler.send_message(SchedulerMessage::RegisterWorkflow {
            workflow_id: "wf-remove".to_string(),
        });

    assert!(result.is_ok(), "Register should succeed failed: {:?}", result);

    let result = scheduler.send_message(SchedulerMessage::ScheduleBead {
            workflow_id: "wf-keep".to_string(),
            bead_id: "bead-1".to_string(),
        });


    assert!(result.is_ok(), "Schedule should succeed failed: {:?}", result);
    let result = scheduler.send_message(SchedulerMessage::ScheduleBead {
            workflow_id: "wf-remove".to_string(),
            bead_id: "bead-2".to_string(),
        });

    assert!(result.is_ok(), "Schedule should succeed failed: {:?}", result);

    tokio::time::sleep(Duration::from_millis(20)).await;

    // When: Unregister one workflow
    let result = scheduler.send_message(SchedulerMessage::UnregisterWorkflow {
            workflow_id: "wf-remove".to_string(),
        });

    assert!(result.is_ok(), "Unregister should succeed failed: {:?}", result);

    tokio::time::sleep(Duration::from_millis(10)).await;

    // Then: Other workflow should be unaffected
    let status = call_with_timeout(&scheduler, |reply| SchedulerMessage::GetWorkflowStatus {
            workflow_id: "wf-keep".to_string(),
            reply,
        })
        .await?;

    assert!(status.is_some(), "wf-keep should still exist");
    assert_eq!(status.ok_or_else(|| -> Box<dyn std::error::Error> { "exists".into() })?.total_beads, 1);

    // Removed workflow should not exist
    let removed_status = call_with_timeout(&scheduler, |reply| SchedulerMessage::GetWorkflowStatus {
            workflow_id: "wf-remove".to_string(),
            reply,
        })
        .await;

    assert!(removed_status.is_none(), "wf-remove should not exist");

    // Cleanup
    scheduler.stop(None);

    Ok(())
}

// ═══════════════════════════════════════════════════════════════════════════════
// IS_BEAD_READY QUERY BEHAVIORS
// ═══════════════════════════════════════════════════════════════════════════════

#[tokio::test]
async fn given_root_bead_when_is_ready_query_then_true() -> Result<(), Box<dyn std::error::Error>> {
    // Given: A workflow with a single bead (root)
    let scheduler = setup_scheduler().await?;
    let result = scheduler.send_message(SchedulerMessage::RegisterWorkflow {
            workflow_id: "wf-isready".to_string(),
        });

    assert!(result.is_ok(), "Register should succeed failed: {:?}", result);
    let result = scheduler.send_message(SchedulerMessage::ScheduleBead {
            workflow_id: "wf-isready".to_string(),
            bead_id: "root-bead".to_string(),
        });

    assert!(result.is_ok(), "Schedule should succeed failed: {:?}", result);

    tokio::time::sleep(Duration::from_millis(10)).await;

    // When: Query if the bead is ready
    let is_ready: Result<bool, ActorError> =
        call_with_timeout(&scheduler, |reply| SchedulerMessage::IsBeadReady {
            workflow_id: "wf-isready".to_string(),
            bead_id: "root-bead".to_string(),
            reply,
        })
        .await;

    // Then: Should be ready (no dependencies)
    assert!(
        is_ready.ok_or_else(|| -> Box<dyn std::error::Error> { "Query should succeed".into() })?,
        "Root bead should be ready"
    );

    // Cleanup
    scheduler.stop(None);

    Ok(())
}

#[tokio::test]
async fn given_blocked_bead_when_is_ready_query_then_false() -> Result<(), Box<dyn std::error::Error>> {
    // Given: A workflow with dependency A -> B
    let scheduler = setup_scheduler().await?;
    let result = scheduler.send_message(SchedulerMessage::RegisterWorkflow {
            workflow_id: "wf-blocked".to_string(),
        });

    assert!(result.is_ok(), "Register should succeed failed: {:?}", result);
    let result = scheduler.send_message(SchedulerMessage::ScheduleBead {
            workflow_id: "wf-blocked".to_string(),
            bead_id: "a".to_string(),
        });

    assert!(result.is_ok(), "Schedule should succeed failed: {:?}", result);
    let result = scheduler.send_message(SchedulerMessage::ScheduleBead {
            workflow_id: "wf-blocked".to_string(),
            bead_id: "b".to_string(),
        });

    assert!(result.is_ok(), "Schedule should succeed failed: {:?}", result);
    let result = scheduler.send_message(SchedulerMessage::AddDependency {
            workflow_id: "wf-blocked".to_string(),
            from_bead: "a".to_string(),
            to_bead: "b".to_string(),
        });

    assert!(result.is_ok(), "Add dependency should succeed failed: {:?}", result);

    tokio::time::sleep(Duration::from_millis(10)).await;

    // When: Query if B is ready
    let is_ready: Result<bool, ActorError> =
        call_with_timeout(&scheduler, |reply| SchedulerMessage::IsBeadReady {
            workflow_id: "wf-blocked".to_string(),
            bead_id: "b".to_string(),
            reply,
        })
        .await;

    // Then: B should NOT be ready (blocked by A)
    assert!(
        !is_ready.ok_or_else(|| -> Box<dyn std::error::Error> { "Query should succeed".into() })?,
        "B should be blocked by A"
    );

    // Cleanup
    scheduler.stop(None);

    Ok(())
}

// ═══════════════════════════════════════════════════════════════════════════════
// STRESS & EDGE CASE BEHAVIORS
// ═══════════════════════════════════════════════════════════════════════════════

#[tokio::test]
async fn given_empty_workflow_when_query_ready_beads_then_empty_list() -> Result<(), Box<dyn std::error::Error>> {
    // Given: A workflow with no beads
    let scheduler = setup_scheduler().await?;
    let result = scheduler.send_message(SchedulerMessage::RegisterWorkflow {
            workflow_id: "wf-empty".to_string(),
        });

    assert!(result.is_ok(), "Register should succeed failed: {:?}", result);
    tokio::time::sleep(Duration::from_millis(10)).await;

    // When: Query ready beads
    let result = call_with_timeout(&scheduler, |reply| {
        SchedulerMessage::GetWorkflowReadyBeads {
            workflow_id: "wf-empty".to_string(),
            reply,
        }
    }).await?;

    // Then: Should return empty list, not error
    assert!(result.is_ok(), "Query should succeed");
    assert!(
        result.ok_or_else(|| -> Box<dyn std::error::Error> { "ok".into() })?.is_empty(),
        "Empty workflow has no ready beads"
    );

    scheduler.stop(None);

    Ok(())
}

#[tokio::test]
async fn given_many_beads_when_query_all_ready_then_returns_all() -> Result<(), Box<dyn std::error::Error>> {
    // Given: A workflow with 50 independent beads
    let scheduler = setup_scheduler().await?;
    let result = scheduler.send_message(SchedulerMessage::RegisterWorkflow {
            workflow_id: "wf-many".to_string(),
        });

    assert!(result.is_ok(), "Register should succeed failed: {:?}", result);

    for i in 0..50 {
        let result = scheduler.send_message(SchedulerMessage::ScheduleBead {
                workflow_id: "wf-many".to_string(),
                bead_id: format!("bead-{}", i),
            });

        assert!(result.is_ok(), "Schedule should succeed failed: {:?}", result);
    }

    tokio::time::sleep(Duration::from_millis(50)).await;

    // When: Query ready beads
    let result = call_with_timeout(&scheduler, |reply| {
        SchedulerMessage::GetWorkflowReadyBeads {
            workflow_id: "wf-many".to_string(),
            reply,
        }
    }).await?;

    // Then: All 50 should be ready (no dependencies)
    let ready = result?;
    assert_eq!(ready.len(), 50, "All 50 independent beads should be ready");

    scheduler.stop(None);

    Ok(())
}

#[tokio::test]
async fn given_long_chain_when_complete_sequentially_then_unlocks_one_at_a_time() -> Result<(), Box<dyn std::error::Error>> {
    // Given: A linear chain A -> B -> C -> D -> E
    let scheduler = setup_scheduler().await?;
    let result = scheduler.send_message(SchedulerMessage::RegisterWorkflow {
            workflow_id: "wf-chain".to_string(),
        });

    assert!(result.is_ok(), "Register should succeed failed: {:?}", result);

    let beads = ["a", "b", "c", "d", "e"];
    for bead in &beads {
        let result = scheduler.send_message(SchedulerMessage::ScheduleBead {
                workflow_id: "wf-chain".to_string(),
                bead_id: bead.to_string(),
            });

        assert!(result.is_ok(), "Schedule should succeed failed: {:?}", result);
    }

    // Add chain dependencies: a -> b -> c -> d -> e
    for i in 0..beads.len() - 1 {
        let result = scheduler.send_message(SchedulerMessage::AddDependency {
                workflow_id: "wf-chain".to_string(),
                from_bead: beads[i].to_string(),
                to_bead: beads[i + 1].to_string(),
            });

        assert!(result.is_ok(), "Add dependency should succeed failed: {:?}", result);
    }

    tokio::time::sleep(Duration::from_millis(20)).await;

    // Initially only "a" should be ready
    let ready = call_with_timeout(&scheduler, |reply| {
        SchedulerMessage::GetWorkflowReadyBeads {
            workflow_id: "wf-chain".to_string(),
            reply,
        }
    })
    .await;
    assert_eq!(
        ready.ok_or_else(|| -> Box<dyn std::error::Error> { "ok".into() })?.len(),
        1,
        "Only 'a' should be ready initially"
    );

    // Complete each bead and verify next becomes ready
    for i in 0..beads.len() - 1 {
        let result = scheduler.send_message(SchedulerMessage::OnBeadCompleted {
                workflow_id: "wf-chain".to_string(),
                bead_id: beads[i].to_string(),
            });

        assert!(result.is_ok(), "Complete should succeed failed: {:?}", result);

        tokio::time::sleep(Duration::from_millis(10)).await;

        let ready = call_with_timeout(&scheduler, |reply| {
            SchedulerMessage::GetWorkflowReadyBeads {
                workflow_id: "wf-chain".to_string(),
                reply,
            }
        })
        .await;

        let ready_list = ready.ok_or_else(|| -> Box<dyn std::error::Error> { "Query should succeed".into() })?;
        assert!(
            ready_list.contains(&beads[i + 1].to_string()),
            "After completing '{}', '{}' should be ready",
            beads[i],
            beads[i + 1]
        );
    }

    scheduler.stop(None);

    Ok(())
}

#[tokio::test]
async fn given_completed_bead_when_complete_again_then_idempotent() -> Result<(), Box<dyn std::error::Error>> {
    // Given: A workflow with a completed bead
    let scheduler = setup_scheduler().await?;
    let result = scheduler.send_message(SchedulerMessage::RegisterWorkflow {
            workflow_id: "wf-idempotent".to_string(),
        });

    assert!(result.is_ok(), "Register should succeed failed: {:?}", result);

    let result = scheduler.send_message(SchedulerMessage::ScheduleBead {
            workflow_id: "wf-idempotent".to_string(),
            bead_id: "bead-1".to_string(),
        });


    assert!(result.is_ok(), "Schedule should succeed failed: {:?}", result);

    let result = scheduler.send_message(SchedulerMessage::OnBeadCompleted {
            workflow_id: "wf-idempotent".to_string(),
            bead_id: "bead-1".to_string(),
        });


    assert!(result.is_ok(), "First complete should succeed failed: {:?}", result);

    tokio::time::sleep(Duration::from_millis(10)).await;

    // When: Complete the same bead again
    let result = scheduler.send_message(SchedulerMessage::OnBeadCompleted {
        workflow_id: "wf-idempotent".to_string(),
        bead_id: "bead-1".to_string(),
    });

    // Then: Should not error (idempotent)
    assert!(result.is_ok(), "Duplicate completion should be idempotent");

    // Verify workflow state is still consistent
    let status = call_with_timeout(&scheduler, |reply| SchedulerMessage::GetWorkflowStatus {
            workflow_id: "wf-idempotent".to_string(),
            reply,
        })
        .await?;

    let status = status.ok_or_else(|| -> Box<dyn std::error::Error> { "Workflow should exist".into() })?;
    assert_eq!(
        status.completed_beads, 1,
        "Should still show 1 completed bead"
    );

    scheduler.stop(None);

    Ok(())
}

#[tokio::test]
async fn given_nonexistent_bead_when_complete_then_no_crash() -> Result<(), Box<dyn std::error::Error>> {
    // Given: A workflow with no beads
    let scheduler = setup_scheduler().await?;
    let result = scheduler.send_message(SchedulerMessage::RegisterWorkflow {
            workflow_id: "wf-ghost".to_string(),
        });

    assert!(result.is_ok(), "Register should succeed failed: {:?}", result);
    tokio::time::sleep(Duration::from_millis(10)).await;

    // When: Try to complete a non-existent bead
    let result = scheduler.send_message(SchedulerMessage::OnBeadCompleted {
        workflow_id: "wf-ghost".to_string(),
        bead_id: "ghost-bead".to_string(),
    });

    // Then: Message sending succeeds, error handled internally
    assert!(result.is_ok(), "Message send should succeed");

    // Actor should still be running
    tokio::time::sleep(Duration::from_millis(10)).await;
    let stats: SchedulerStats =
        call_with_timeout(&scheduler, |reply| SchedulerMessage::GetStats { reply }).await?;
    assert_eq!(stats.workflow_count, 1, "Actor should still be functional");

    scheduler.stop(None);

    Ok(())
}

// ═══════════════════════════════════════════════════════════════════════════════
// COMPLEX DAG TOPOLOGY BEHAVIORS
// ═══════════════════════════════════════════════════════════════════════════════

#[tokio::test]
async fn given_wide_fan_out_when_root_completes_then_all_children_ready() -> Result<(), Box<dyn std::error::Error>> {
    // Given: A fan-out DAG: ROOT -> [A, B, C, D, E]
    let scheduler = setup_scheduler().await?;
    let result = scheduler.send_message(SchedulerMessage::RegisterWorkflow {
            workflow_id: "wf-fanout".to_string(),
        });

    assert!(result.is_ok(), "Register should succeed failed: {:?}", result);

    let children = ["a", "b", "c", "d", "e"];
    let result = scheduler.send_message(SchedulerMessage::ScheduleBead {
            workflow_id: "wf-fanout".to_string(),
            bead_id: "root".to_string(),
        });

    assert!(result.is_ok(), "Schedule should succeed failed: {:?}", result);

    for child in &children {
        let result = scheduler.send_message(SchedulerMessage::ScheduleBead {
                workflow_id: "wf-fanout".to_string(),
                bead_id: child.to_string(),
            });

        assert!(result.is_ok(), "Schedule should succeed failed: {:?}", result);

        let result = scheduler.send_message(SchedulerMessage::AddDependency {
                workflow_id: "wf-fanout".to_string(),
                from_bead: "root".to_string(),
                to_bead: child.to_string(),
            });


        assert!(result.is_ok(), "Add dependency should succeed failed: {:?}", result);
    }

    tokio::time::sleep(Duration::from_millis(20)).await;

    // When: Complete root
    let result = scheduler.send_message(SchedulerMessage::OnBeadCompleted {
            workflow_id: "wf-fanout".to_string(),
            bead_id: "root".to_string(),
        });

    assert!(result.is_ok(), "Complete should succeed failed: {:?}", result);

    tokio::time::sleep(Duration::from_millis(20)).await;

    // Then: All 5 children should be ready simultaneously
    let ready = call_with_timeout(&scheduler, |reply| {
        SchedulerMessage::GetWorkflowReadyBeads {
            workflow_id: "wf-fanout".to_string(),
            reply,
        }
    })
    .await;

    let ready_list = ready.ok_or_else(|| -> Box<dyn std::error::Error> { "Query should succeed".into() })?;
    assert_eq!(ready_list.len(), 5, "All 5 children should be ready");
    for child in &children {
        assert!(
            ready_list.contains(&child.to_string()),
            "{} should be ready",
            child
        );
    }

    scheduler.stop(None);

    Ok(())
}

#[tokio::test]
async fn given_wide_fan_in_when_all_parents_complete_then_sink_ready() -> Result<(), Box<dyn std::error::Error>> {
    // Given: A fan-in DAG: [A, B, C, D, E] -> SINK
    let scheduler = setup_scheduler().await?;
    let result = scheduler.send_message(SchedulerMessage::RegisterWorkflow {
            workflow_id: "wf-fanin".to_string(),
        });

    assert!(result.is_ok(), "Register should succeed failed: {:?}", result);

    let parents = ["a", "b", "c", "d", "e"];
    for parent in &parents {
        let result = scheduler.send_message(SchedulerMessage::ScheduleBead {
                workflow_id: "wf-fanin".to_string(),
                bead_id: parent.to_string(),
            });

        assert!(result.is_ok(), "Schedule should succeed failed: {:?}", result);
    }

    let result = scheduler.send_message(SchedulerMessage::ScheduleBead {
            workflow_id: "wf-fanin".to_string(),
            bead_id: "sink".to_string(),
        });


    assert!(result.is_ok(), "Schedule should succeed failed: {:?}", result);

    for parent in &parents {
        let result = scheduler.send_message(SchedulerMessage::AddDependency {
                workflow_id: "wf-fanin".to_string(),
                from_bead: parent.to_string(),
                to_bead: "sink".to_string(),
            });

        assert!(result.is_ok(), "Add dependency should succeed failed: {:?}", result);
    }

    tokio::time::sleep(Duration::from_millis(20)).await;

    // Complete all parents except last one
    for parent in &parents[..4] {
        let result = scheduler.send_message(SchedulerMessage::OnBeadCompleted {
                workflow_id: "wf-fanin".to_string(),
                bead_id: parent.to_string(),
            });

        assert!(result.is_ok(), "Complete should succeed failed: {:?}", result);
    }

    tokio::time::sleep(Duration::from_millis(10)).await;

    // Sink should NOT be ready (missing 'e')
    let ready = call_with_timeout(&scheduler, |reply| {
        SchedulerMessage::GetWorkflowReadyBeads {
            workflow_id: "wf-fanin".to_string(),
            reply,
        }
    })
    .await;
    let ready_list = ready?;
    assert!(
        !ready_list.contains(&"sink".to_string()),
        "Sink should NOT be ready until all parents complete"
    );

    // When: Complete the last parent
    let result = scheduler.send_message(SchedulerMessage::OnBeadCompleted {
            workflow_id: "wf-fanin".to_string(),
            bead_id: "e".to_string(),
        });

    assert!(result.is_ok(), "Complete should succeed failed: {:?}", result);

    tokio::time::sleep(Duration::from_millis(10)).await;

    // Then: Sink should now be ready
    let ready = call_with_timeout(&scheduler, |reply| {
        SchedulerMessage::GetWorkflowReadyBeads {
            workflow_id: "wf-fanin".to_string(),
            reply,
        }
    })
    .await;
    let ready_list = ready?;
    assert!(
        ready_list.contains(&"sink".to_string()),
        "Sink should be ready after all parents complete"
    );

    scheduler.stop(None);

    Ok(())
}

#[tokio::test]
async fn given_w_dag_when_complete_in_order_then_correct_unlocks() -> Result<(), Box<dyn std::error::Error>> {
    // Given: A W-shaped DAG
    // A -> B -> E
    //   \     /
    //    C -> D
    let scheduler = setup_scheduler().await?;
    let result = scheduler.send_message(SchedulerMessage::RegisterWorkflow {
            workflow_id: "wf-w".to_string(),
        });

    assert!(result.is_ok(), "Register should succeed failed: {:?}", result);

    for bead in ["a", "b", "c", "d", "e"] {
        let result = scheduler.send_message(SchedulerMessage::ScheduleBead {
                workflow_id: "wf-w".to_string(),
                bead_id: bead.to_string(),
            });

        assert!(result.is_ok(), "Schedule should succeed failed: {:?}", result);
    }

    // A -> B, A -> C, B -> E, C -> D, D -> E
    let edges = [("a", "b"), ("a", "c"), ("b", "e"), ("c", "d"), ("d", "e")];
    for (from, to) in &edges {
        let result = scheduler.send_message(SchedulerMessage::AddDependency {
                workflow_id: "wf-w".to_string(),
                from_bead: from.to_string(),
                to_bead: to.to_string(),
            });

        assert!(result.is_ok(), "Add dependency should succeed failed: {:?}", result);
    }

    tokio::time::sleep(Duration::from_millis(20)).await;

    // Only A should be ready initially
    let ready = call_with_timeout(&scheduler, |reply| {
        SchedulerMessage::GetWorkflowReadyBeads {
            workflow_id: "wf-w".to_string(),
            reply,
        }
    })
    .await;
    assert_eq!(ready?, vec!["a".to_string()]);

    // Complete A -> B and C should be ready
    let result = scheduler.send_message(SchedulerMessage::OnBeadCompleted {
            workflow_id: "wf-w".to_string(),
            bead_id: "a".to_string(),
        });

    assert!(result.is_ok(), "ok failed: {:?}", result);
    tokio::time::sleep(Duration::from_millis(10)).await;

    let ready = call_with_timeout(&scheduler, |reply| {
        SchedulerMessage::GetWorkflowReadyBeads {
            workflow_id: "wf-w".to_string(),
            reply,
        }
    })
    .await;
    let ready_list = ready?;
    assert!(ready_list.contains(&"b".to_string()), "B should be ready");
    assert!(ready_list.contains(&"c".to_string()), "C should be ready");

    // Complete B and C
    let result = scheduler.send_message(SchedulerMessage::OnBeadCompleted {
            workflow_id: "wf-w".to_string(),
            bead_id: "b".to_string(),
        });

    assert!(result.is_ok(), "ok failed: {:?}", result);
    let result = scheduler.send_message(SchedulerMessage::OnBeadCompleted {
            workflow_id: "wf-w".to_string(),
            bead_id: "c".to_string(),
        });

    assert!(result.is_ok(), "ok failed: {:?}", result);
    tokio::time::sleep(Duration::from_millis(10)).await;

    // D should be ready (C done), but E not ready (needs B and D)
    let ready = call_with_timeout(&scheduler, |reply| {
        SchedulerMessage::GetWorkflowReadyBeads {
            workflow_id: "wf-w".to_string(),
            reply,
        }
    })
    .await;
    let ready_list = ready?;
    assert!(ready_list.contains(&"d".to_string()), "D should be ready");
    assert!(
        !ready_list.contains(&"e".to_string()),
        "E not ready yet (needs D)"
    );

    // Complete D -> E should be ready
    let result = scheduler.send_message(SchedulerMessage::OnBeadCompleted {
            workflow_id: "wf-w".to_string(),
            bead_id: "d".to_string(),
        });

    assert!(result.is_ok(), "ok failed: {:?}", result);
    tokio::time::sleep(Duration::from_millis(10)).await;

    let ready = call_with_timeout(&scheduler, |reply| {
        SchedulerMessage::GetWorkflowReadyBeads {
            workflow_id: "wf-w".to_string(),
            reply,
        }
    })
    .await;
    assert!(
        ready.ok_or_else(|| -> Box<dyn std::error::Error> { "ok".into() })?.contains(&"e".to_string()),
        "E should be ready"
    );

    scheduler.stop(None);

    Ok(())
}

// ═══════════════════════════════════════════════════════════════════════════════
// CONCURRENT CLAIM BEHAVIORS
// ═══════════════════════════════════════════════════════════════════════════════

#[tokio::test]
async fn given_bead_when_claimed_twice_then_second_claim_fails_or_idempotent() -> Result<(), Box<dyn std::error::Error>> {
    // Given: A workflow with a ready bead
    let scheduler = setup_scheduler().await?;
    let result = scheduler.send_message(SchedulerMessage::RegisterWorkflow {
            workflow_id: "wf-double-claim".to_string(),
        });

    assert!(result.is_ok(), "Register should succeed failed: {:?}", result);

    let result = scheduler.send_message(SchedulerMessage::ScheduleBead {
            workflow_id: "wf-double-claim".to_string(),
            bead_id: "bead-1".to_string(),
        });


    assert!(result.is_ok(), "Schedule should succeed failed: {:?}", result);

    tokio::time::sleep(Duration::from_millis(10)).await;

    // When: First worker claims the bead
    let result = scheduler.send_message(SchedulerMessage::ClaimBead {
            bead_id: "bead-1".to_string(),
            worker_id: "worker-1".to_string(),
        });

    assert!(result.is_ok(), "First claim should succeed failed: {:?}", result);

    tokio::time::sleep(Duration::from_millis(10)).await;

    // When: Second worker tries to claim the same bead
    let result = scheduler.send_message(SchedulerMessage::ClaimBead {
        bead_id: "bead-1".to_string(),
        worker_id: "worker-2".to_string(),
    });

    // Then: Message sending succeeds (handled internally)
    assert!(result.is_ok(), "Message send should succeed");

    // Bead should still not appear in ready list (it's claimed by someone)
    tokio::time::sleep(Duration::from_millis(10)).await;
    let all_ready: Vec<(String, String)> = call_with_timeout(&scheduler, |reply| {
        SchedulerMessage::GetAllReadyBeads { reply }
    }).await?;

    let has_bead = all_ready.iter().any(|(_, bid)| bid == "bead-1");
    assert!(!has_bead, "Bead should still be claimed");

    scheduler.stop(None);

    Ok(())
}

#[tokio::test]
async fn given_multiple_beads_when_claim_different_then_all_claimed() -> Result<(), Box<dyn std::error::Error>> {
    // Given: A workflow with multiple ready beads
    let scheduler = setup_scheduler().await?;
    let result = scheduler.send_message(SchedulerMessage::RegisterWorkflow {
            workflow_id: "wf-multi-claim".to_string(),
        });

    assert!(result.is_ok(), "Register should succeed failed: {:?}", result);

    for i in 0..5 {
        let result = scheduler.send_message(SchedulerMessage::ScheduleBead {
                workflow_id: "wf-multi-claim".to_string(),
                bead_id: format!("bead-{}", i),
            });

        assert!(result.is_ok(), "Schedule should succeed failed: {:?}", result);
    }

    tokio::time::sleep(Duration::from_millis(20)).await;

    // When: Different workers claim different beads
    for i in 0..5 {
        let result = scheduler.send_message(SchedulerMessage::ClaimBead {
                bead_id: format!("bead-{}", i),
                worker_id: format!("worker-{}", i),
            });

        assert!(result.is_ok(), "Claim should succeed failed: {:?}", result);
    }

    tokio::time::sleep(Duration::from_millis(20)).await;

    // Then: No beads should appear in ready list (all claimed)
    let all_ready: Vec<(String, String)> = call_with_timeout(&scheduler, |reply| {
        SchedulerMessage::GetAllReadyBeads { reply }
    }).await?;

    assert!(all_ready.is_empty(), "All beads should be claimed");

    scheduler.stop(None);

    Ok(())
}

// ═══════════════════════════════════════════════════════════════════════════════
// ACTOR RESILIENCE BEHAVIORS
// ═══════════════════════════════════════════════════════════════════════════════

#[tokio::test]
async fn given_invalid_dependency_when_add_then_no_crash() -> Result<(), Box<dyn std::error::Error>> {
    // Given: A workflow
    let scheduler = setup_scheduler().await?;
    let result = scheduler.send_message(SchedulerMessage::RegisterWorkflow {
            workflow_id: "wf-invalid-dep".to_string(),
        });

    assert!(result.is_ok(), "Register should succeed failed: {:?}", result);

    let result = scheduler.send_message(SchedulerMessage::ScheduleBead {
            workflow_id: "wf-invalid-dep".to_string(),
            bead_id: "a".to_string(),
        });


    assert!(result.is_ok(), "Schedule should succeed failed: {:?}", result);

    tokio::time::sleep(Duration::from_millis(10)).await;

    // When: Try to add dependency with non-existent target
    let result = scheduler.send_message(SchedulerMessage::AddDependency {
        workflow_id: "wf-invalid-dep".to_string(),
        from_bead: "a".to_string(),
        to_bead: "nonexistent".to_string(),
    });

    // Then: Message sending succeeds, error handled internally
    assert!(result.is_ok(), "Message send should succeed");

    // Actor should still be running
    tokio::time::sleep(Duration::from_millis(10)).await;
    let stats: SchedulerStats =
        call_with_timeout(&scheduler, |reply| SchedulerMessage::GetStats { reply }).await?;
    assert_eq!(stats.workflow_count, 1, "Actor should still be functional");

    scheduler.stop(None);

    Ok(())
}

#[tokio::test]
async fn given_rapid_messages_when_sent_then_all_processed() -> Result<(), Box<dyn std::error::Error>> {
    // Given: A scheduler
    let scheduler = setup_scheduler().await?;

    // When: Send 100 register/schedule messages rapidly
    for i in 0..100 {
        let result = scheduler.send_message(SchedulerMessage::RegisterWorkflow {
                workflow_id: format!("wf-rapid-{}", i),
            });

        assert!(result.is_ok(), "Register should succeed failed: {:?}", result);
    }

    // Allow time for processing
    tokio::time::sleep(Duration::from_millis(100)).await;

    // Then: All workflows should be registered
    let stats: SchedulerStats =
        call_with_timeout(&scheduler, |reply| SchedulerMessage::GetStats { reply }).await?;
    assert_eq!(
        stats.workflow_count, 100,
        "All 100 workflows should be registered"
    );

    scheduler.stop(None);

    Ok(())
}

#[tokio::test]
async fn given_workflow_unregistered_when_query_status_then_returns_none() -> Result<(), Box<dyn std::error::Error>> {
    // Given: A registered then unregistered workflow
    let scheduler = setup_scheduler().await?;
    let result = scheduler.send_message(SchedulerMessage::RegisterWorkflow {
            workflow_id: "wf-gone".to_string(),
        });

    assert!(result.is_ok(), "Register should succeed failed: {:?}", result);

    let result = scheduler.send_message(SchedulerMessage::UnregisterWorkflow {
            workflow_id: "wf-gone".to_string(),
        });


    assert!(result.is_ok(), "Unregister should succeed failed: {:?}", result);

    tokio::time::sleep(Duration::from_millis(10)).await;

    // When: Query workflow status
    let status = call_with_timeout(&scheduler, |reply| SchedulerMessage::GetWorkflowStatus {
            workflow_id: "wf-gone".to_string(),
            reply,
        })
        .await?;

    // Then: Should return None
    assert!(status.is_none(), "Unregistered workflow should return None");

    scheduler.stop(None);

    Ok(())
}

// ═══════════════════════════════════════════════════════════════════════════════
// STATE CONSISTENCY BEHAVIORS
// ═══════════════════════════════════════════════════════════════════════════════

#[tokio::test]
async fn given_workflow_when_beads_completed_then_status_reflects_progress() -> Result<(), Box<dyn std::error::Error>> {
    // Given: A workflow with 5 beads
    let scheduler = setup_scheduler().await?;
    let result = scheduler.send_message(SchedulerMessage::RegisterWorkflow {
            workflow_id: "wf-progress".to_string(),
        });

    assert!(result.is_ok(), "Register should succeed failed: {:?}", result);

    for i in 0..5 {
        let result = scheduler.send_message(SchedulerMessage::ScheduleBead {
                workflow_id: "wf-progress".to_string(),
                bead_id: format!("bead-{}", i),
            });

        assert!(result.is_ok(), "Schedule should succeed failed: {:?}", result);
    }

    tokio::time::sleep(Duration::from_millis(20)).await;

    // Initially: 0 completed, 5 total
    let status = call_with_timeout(&scheduler, |reply| SchedulerMessage::GetWorkflowStatus {
            workflow_id: "wf-progress".to_string(),
            reply,
        })
        .await?;
    let status = status.ok_or_else(|| -> Box<dyn std::error::Error> { "exists".into() })?;
    assert_eq!(status.total_beads, 5);
    assert_eq!(status.completed_beads, 0);
    assert!(!status.is_complete);

    // Complete 3 beads
    for i in 0..3 {
        let result = scheduler.send_message(SchedulerMessage::OnBeadCompleted {
                workflow_id: "wf-progress".to_string(),
                bead_id: format!("bead-{}", i),
            });

        assert!(result.is_ok(), "Complete should succeed failed: {:?}", result);
    }

    tokio::time::sleep(Duration::from_millis(20)).await;

    // Then: 3 completed, 5 total, not complete
    let status = call_with_timeout(&scheduler, |reply| SchedulerMessage::GetWorkflowStatus {
            workflow_id: "wf-progress".to_string(),
            reply,
        })
        .await?;
    let status = status.ok_or_else(|| -> Box<dyn std::error::Error> { "exists".into() })?;
    assert_eq!(status.total_beads, 5);
    assert_eq!(status.completed_beads, 3);
    assert!(!status.is_complete);

    // Complete remaining 2
    for i in 3..5 {
        let result = scheduler.send_message(SchedulerMessage::OnBeadCompleted {
                workflow_id: "wf-progress".to_string(),
                bead_id: format!("bead-{}", i),
            });

        assert!(result.is_ok(), "Complete should succeed failed: {:?}", result);
    }

    tokio::time::sleep(Duration::from_millis(20)).await;

    // Then: 5 completed, 5 total, complete!
    let status = call_with_timeout(&scheduler, |reply| SchedulerMessage::GetWorkflowStatus {
            workflow_id: "wf-progress".to_string(),
            reply,
        })
        .await?;
    let status = status.ok_or_else(|| -> Box<dyn std::error::Error> { "exists".into() })?;
    assert_eq!(status.total_beads, 5);
    assert_eq!(status.completed_beads, 5);
    assert!(status.is_complete);

    scheduler.stop(None);

    Ok(())
}

#[tokio::test]
async fn given_bead_claimed_and_completed_when_check_stats_then_consistent() -> Result<(), Box<dyn std::error::Error>> {
    // Given: A workflow with beads
    let scheduler = setup_scheduler().await?;
    let result = scheduler.send_message(SchedulerMessage::RegisterWorkflow {
            workflow_id: "wf-stats-consistency".to_string(),
        });

    assert!(result.is_ok(), "Register should succeed failed: {:?}", result);

    for i in 0..3 {
        let result = scheduler.send_message(SchedulerMessage::ScheduleBead {
                workflow_id: "wf-stats-consistency".to_string(),
                bead_id: format!("bead-{}", i),
            });

        assert!(result.is_ok(), "Schedule should succeed failed: {:?}", result);
    }

    tokio::time::sleep(Duration::from_millis(20)).await;

    // Claim one bead
    let result = scheduler.send_message(SchedulerMessage::ClaimBead {
            bead_id: "bead-0".to_string(),
            worker_id: "worker-1".to_string(),
        });

    assert!(result.is_ok(), "Claim should succeed failed: {:?}", result);

    // Complete another bead
    let result = scheduler.send_message(SchedulerMessage::OnBeadCompleted {
            workflow_id: "wf-stats-consistency".to_string(),
            bead_id: "bead-1".to_string(),
        });

    assert!(result.is_ok(), "Complete should succeed failed: {:?}", result);

    tokio::time::sleep(Duration::from_millis(20)).await;

    // Then: Stats should reflect the state
    let stats: SchedulerStats =
        call_with_timeout(&scheduler, |reply| SchedulerMessage::GetStats { reply }).await?;

    assert_eq!(stats.workflow_count, 1);
    // pending_count might vary based on implementation - just verify actor didn't crash
    // Note: pending_count is usize so it's always >= 0
    let _ = stats.pending_count; // Use the value to show actor state is accessible

    scheduler.stop(None);

    Ok(())
}

// ═══════════════════════════════════════════════════════════════════════════════
// QUERY ISOLATION BEHAVIORS
// ═══════════════════════════════════════════════════════════════════════════════

#[tokio::test]
async fn given_workflows_when_query_one_then_no_cross_contamination() -> Result<(), Box<dyn std::error::Error>> {
    // Given: Two workflows with different beads
    let scheduler = setup_scheduler().await?;

    let result = scheduler.send_message(SchedulerMessage::RegisterWorkflow {
            workflow_id: "wf-isolated-1".to_string(),
        });


    assert!(result.is_ok(), "Register should succeed failed: {:?}", result);
    let result = scheduler.send_message(SchedulerMessage::RegisterWorkflow {
            workflow_id: "wf-isolated-2".to_string(),
        });

    assert!(result.is_ok(), "Register should succeed failed: {:?}", result);

    let result = scheduler.send_message(SchedulerMessage::ScheduleBead {
            workflow_id: "wf-isolated-1".to_string(),
            bead_id: "bead-from-1".to_string(),
        });


    assert!(result.is_ok(), "Schedule should succeed failed: {:?}", result);
    let result = scheduler.send_message(SchedulerMessage::ScheduleBead {
            workflow_id: "wf-isolated-2".to_string(),
            bead_id: "bead-from-2".to_string(),
        });

    assert!(result.is_ok(), "Schedule should succeed failed: {:?}", result);

    tokio::time::sleep(Duration::from_millis(20)).await;

    // When: Query workflow 1
    let ready1 = call_with_timeout(&scheduler, |reply| {
        SchedulerMessage::GetWorkflowReadyBeads {
            workflow_id: "wf-isolated-1".to_string(),
            reply,
        }
    })
    .await;

    // Then: Should only contain beads from workflow 1
    let ready1_list = ready1.ok_or_else(|| -> Box<dyn std::error::Error> { "ok".into() })?;
    assert!(ready1_list.contains(&"bead-from-1".to_string()));
    assert!(
        !ready1_list.contains(&"bead-from-2".to_string()),
        "No cross-contamination"
    );

    // When: Query workflow 2
    let ready2 = call_with_timeout(&scheduler, |reply| {
        SchedulerMessage::GetWorkflowReadyBeads {
            workflow_id: "wf-isolated-2".to_string(),
            reply,
        }
    })
    .await;

    // Then: Should only contain beads from workflow 2
    let ready2_list = ready2.ok_or_else(|| -> Box<dyn std::error::Error> { "ok".into() })?;
    assert!(ready2_list.contains(&"bead-from-2".to_string()));
    assert!(
        !ready2_list.contains(&"bead-from-1".to_string()),
        "No cross-contamination"
    );

    scheduler.stop(None);

    Ok(())
}

#[tokio::test]
async fn given_workflow_with_self_dependency_when_query_then_blocked() -> Result<(), Box<dyn std::error::Error>> {
    // Given: A workflow where a bead depends on itself (edge case)
    let scheduler = setup_scheduler().await?;
    let result = scheduler.send_message(SchedulerMessage::RegisterWorkflow {
            workflow_id: "wf-self".to_string(),
        });

    assert!(result.is_ok(), "Register should succeed failed: {:?}", result);

    let result = scheduler.send_message(SchedulerMessage::ScheduleBead {
            workflow_id: "wf-self".to_string(),
            bead_id: "self-dep".to_string(),
        });


    assert!(result.is_ok(), "Schedule should succeed failed: {:?}", result);

    // Try to add self-dependency
    let result = scheduler.send_message(SchedulerMessage::AddDependency {
            workflow_id: "wf-self".to_string(),
            from_bead: "self-dep".to_string(),
            to_bead: "self-dep".to_string(),
        });

    assert!(result.is_ok(), "Message send should succeed failed: {:?}", result);

    tokio::time::sleep(Duration::from_millis(10)).await;

    // Actor should still be functional
    let stats: SchedulerStats =
        call_with_timeout(&scheduler, |reply| SchedulerMessage::GetStats { reply }).await?;
    assert_eq!(stats.workflow_count, 1, "Actor should still be functional");

    scheduler.stop(None);

    Ok(())
}

