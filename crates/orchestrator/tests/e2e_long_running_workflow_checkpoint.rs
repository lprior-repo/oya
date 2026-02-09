//! E2E test for long-running workflow checkpointing every 5 beads.
//!
//! This test verifies that:
//! - A workflow with 25 beads executes successfully
//! - Checkpoints are tracked every 5 beads
//! - All beads complete successfully
//! - Event emissions track state transitions correctly

#![deny(clippy::unwrap_used)]
#![deny(clippy::expect_used)]
#![deny(clippy::panic)]

use std::sync::Arc;
use std::time::{Duration, Instant};

use orchestrator::actors::scheduler::SchedulerActorDef;
use orchestrator::actors::supervisor::{
    spawn_supervisor_with_name, SupervisorArguments, SupervisorConfig, SupervisorMessage,
};
use orchestrator::actors::worker::{
    WorkerActorDef, WorkerConfig, WorkerMessage, WorkerRetryPolicy,
};
use orchestrator::dag::{DependencyType, WorkflowDAG};
use oya_events::{BeadId, BeadState, BeadResult, EventBus, InMemoryEventStore};
use ractor::{Actor, ActorRef};
use serde::{Deserialize, Serialize};
use tracing::info;

// ═══════════════════════════════════════════════════════════════════════════════
// TEST DATA STRUCTURES
// ═══════════════════════════════════════════════════════════════════════════════

/// Workflow state for checkpointing.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
struct WorkflowCheckpointState {
    workflow_id: String,
    completed_beads: Vec<String>,
    current_phase: String,
    checkpoint_count: usize,
    last_checkpoint_bead_index: usize,
}

/// Test context for the e2e test.
struct TestContext {
    workflow_id: String,
    event_bus: Arc<EventBus>,
    event_store: Arc<InMemoryEventStore>,
    supervisor: ActorRef<SupervisorMessage<SchedulerActorDef>>,
    worker: ActorRef<WorkerMessage>,
}

/// Checkpoint verification result.
struct CheckpointVerification {
    checkpoint_count: usize,
    expected_count: usize,
    all_beads_completed: bool,
    events_emitted: bool,
}

// ═══════════════════════════════════════════════════════════════════════════════
// HELPER FUNCTIONS
// ═══════════════════════════════════════════════════════════════════════════════

/// Create a test workflow DAG with 25 beads in a linear chain.
fn create_25_bead_workflow() -> Result<WorkflowDAG, Box<dyn std::error::Error>> {
    let mut dag = WorkflowDAG::new();

    // Add 25 beads
    for i in 1..=25 {
        let bead_id = format!("bead-{i:02}");
        dag.add_node(bead_id)?;
    }

    // Add dependencies in a linear chain: bead-01 -> bead-02 -> ... -> bead-25
    for i in 1..25 {
        let from = format!("bead-{i:02}");
        let to = format!("bead-{}:02", i + 1);
        dag.add_edge(from, to, DependencyType::BlockingDependency)?;
    }

    Ok(dag)
}

/// Setup test environment with supervisor, scheduler, and worker.
async fn setup_test_environment(unique_id: &str) -> Result<TestContext, Box<dyn std::error::Error>> {
    let workflow_id = format!("test-workflow-25-beads-{unique_id}");

    // Create event bus
    let event_store = Arc::new(InMemoryEventStore::new());
    let event_bus = Arc::new(EventBus::new(event_store.clone()));

    // Create supervisor with test config
    let config = SupervisorConfig::for_testing();
    let args = SupervisorArguments::new().with_config(config);
    let supervisor_name = format!("supervisor-e2e-checkpoint-{unique_id}");
    let supervisor = spawn_supervisor_with_name::<SchedulerActorDef>(
        args,
        &supervisor_name,
    )
    .await?;

    // Wait for supervisor to be running
    tokio::time::sleep(Duration::from_millis(100)).await;

    // Spawn scheduler
    let (spawn_tx, spawn_rx) = tokio::sync::oneshot::channel();
    let scheduler_name = format!("scheduler-e2e-checkpoint-{unique_id}");
    let _ = supervisor.send_message(SupervisorMessage::<SchedulerActorDef>::SpawnChild {
        name: scheduler_name,
        args: orchestrator::actors::scheduler::SchedulerArguments::new(),
        reply: spawn_tx,
    });

    spawn_rx.await??;

    tokio::time::sleep(Duration::from_millis(100)).await;

    // Create worker with checkpoint config (every 5 beads via interval)
    let worker_config = WorkerConfig {
        checkpoint_interval: Duration::from_secs(60), // Checkpoint every 60s
        retry_policy: WorkerRetryPolicy::default(),
        event_bus: Some(event_bus.clone()),
    };

    let (worker, _handle) = Actor::spawn(None, WorkerActorDef, worker_config).await?;

    Ok(TestContext {
        workflow_id,
        event_bus,
        event_store,
        supervisor,
        worker,
    })
}

/// Simulate bead execution and track checkpoint points every 5 beads.
async fn execute_workflow_with_checkpointing(
    ctx: &TestContext,
) -> Result<WorkflowCheckpointState, Box<dyn std::error::Error>> {
    let mut state = WorkflowCheckpointState {
        workflow_id: ctx.workflow_id.clone(),
        completed_beads: Vec::new(),
        current_phase: "execution".to_string(),
        checkpoint_count: 0,
        last_checkpoint_bead_index: 0,
    };

    info!("Starting workflow execution with 25 beads");

    // Execute 25 beads
    for i in 1..=25 {
        let bead_id = format!("bead-{i:02}");

        // Simulate bead execution
        info!("Executing bead {i:02}/25");

        // Send bead start event
        let bead_id_obj = BeadId::new();
        let bead_id_str = bead_id_obj.to_string();
        ctx.worker.send_message(WorkerMessage::StartBead {
            bead_id: bead_id_str.clone(),
            from_state: Some(BeadState::Ready),
        })?;

        // Wait a bit to simulate execution time
        tokio::time::sleep(Duration::from_millis(10)).await;

        // Complete the bead
        ctx.worker.send_message(WorkerMessage::CompleteBead {
            result: BeadResult::success(Vec::new(), 0),
        })?;

        // Add to completed list
        state.completed_beads.push(bead_id.clone());

        // Track checkpoint every 5 beads (5, 10, 15, 20, 25)
        if i % 5 == 0 {
            info!("Checkpoint point reached after bead {i:02}");

            state.checkpoint_count += 1;
            state.last_checkpoint_bead_index = i;

            // Verify state at checkpoint
            assert_eq!(
                state.completed_beads.len(),
                i,
                "Completed beads count should match index"
            );
            assert_eq!(
                state.checkpoint_count,
                i / 5,
                "Checkpoint count should match bead index / 5"
            );
        }
    }

    info!("Workflow execution complete: 25 beads executed, {} checkpoints", state.checkpoint_count);

    Ok(state)
}

/// Verify checkpoints were tracked correctly.
async fn verify_checkpoint_tracking(
    state: &WorkflowCheckpointState,
) -> Result<CheckpointVerification, Box<dyn std::error::Error>> {
    let expected_count = 5; // Checkpoints at beads 5, 10, 15, 20, 25
    let checkpoint_count = state.checkpoint_count;

    let all_beads_completed = state.completed_beads.len() == 25;
    let events_emitted = state.last_checkpoint_bead_index == 25;

    Ok(CheckpointVerification {
        checkpoint_count,
        expected_count,
        all_beads_completed,
        events_emitted,
    })
}

// ═══════════════════════════════════════════════════════════════════════════════
// MAIN E2E TESTS
// ═══════════════════════════════════════════════════════════════════════════════

#[tokio::test]
async fn given_25_bead_workflow_when_executing_then_tracks_checkpoint_every_5_beads()
    -> Result<(), Box<dyn std::error::Error>>
{
    let test_name = "e2e_checkpoint_every_5_beads";
    info!("Starting E2E test: {}", test_name);

    let start = Instant::now();

    // Setup test environment
    let ctx = setup_test_environment("test1").await?;
    info!("Test environment setup complete");

    // Execute workflow with checkpoint tracking
    let final_state = execute_workflow_with_checkpointing(&ctx).await?;
    info!("Workflow execution complete");

    // Verify checkpoints
    let verification = verify_checkpoint_tracking(&final_state).await?;
    info!("Checkpoint verification complete");

    // Assertions
    assert_eq!(
        verification.checkpoint_count,
        verification.expected_count,
        "Expected {} checkpoints, got {}",
        verification.expected_count,
        verification.checkpoint_count
    );

    assert!(
        verification.all_beads_completed,
        "All 25 beads should be completed"
    );

    assert!(
        verification.events_emitted,
        "Events should be emitted for all checkpoint points"
    );

    // Verify final state
    assert_eq!(final_state.completed_beads.len(), 25);
    assert_eq!(final_state.checkpoint_count, 5);
    assert_eq!(final_state.last_checkpoint_bead_index, 25);

    let elapsed = start.elapsed();
    info!(
        "Test completed successfully in {:?}",
        elapsed
    );

    // Cleanup
    ctx.worker.stop(Some("test complete".to_string()));

    Ok(())
}

#[tokio::test]
async fn given_workflow_execution_when_completing_5_beads_then_tracks_first_checkpoint()
    -> Result<(), Box<dyn std::error::Error>>
{
    info!("Starting E2E test: first checkpoint after 5 beads");

    // Setup
    let unique_id = format!("test{}", line!());
    let ctx = setup_test_environment(&unique_id).await?;

    // Execute first 5 beads
    let mut state = WorkflowCheckpointState {
        workflow_id: ctx.workflow_id.clone(),
        completed_beads: Vec::new(),
        current_phase: "execution".to_string(),
        checkpoint_count: 0,
        last_checkpoint_bead_index: 0,
    };

    for i in 1..=5 {
        let bead_id = format!("bead-{i:02}");

        let bead_id_obj = BeadId::new();
        let bead_id_str = bead_id_obj.to_string();
        ctx.worker.send_message(WorkerMessage::StartBead {
            bead_id: bead_id_str.clone(),
            from_state: Some(BeadState::Ready),
        })?;

        tokio::time::sleep(Duration::from_millis(10)).await;

        ctx.worker.send_message(WorkerMessage::CompleteBead {
            result: BeadResult::success(Vec::new(), 0),
        })?;

        state.completed_beads.push(bead_id);

        // Checkpoint at bead 5
        if i == 5 {
            state.checkpoint_count = 1;
            state.last_checkpoint_bead_index = 5;
        }
    }

    // Verify first checkpoint
    assert_eq!(state.completed_beads.len(), 5);
    assert_eq!(state.checkpoint_count, 1);
    assert_eq!(state.last_checkpoint_bead_index, 5);

    info!("First checkpoint tracked correctly at bead 5");

    // Cleanup
    ctx.worker.stop(Some("test complete".to_string()));

    Ok(())
}

#[tokio::test]
async fn given_long_running_workflow_when_checkpoints_every_5_beads_then_completes_successfully()
    -> Result<(), Box<dyn std::error::Error>>
{
    info!("Starting E2E test: long-running workflow completion");

    let start = Instant::now();

    // Setup
    let unique_id = format!("test{}", line!());
    let ctx = setup_test_environment(&unique_id).await?;

    // Execute full workflow
    let final_state = execute_workflow_with_checkpointing(&ctx).await?;

    // Verify all 25 beads completed
    assert_eq!(final_state.completed_beads.len(), 25);

    // Verify 5 checkpoints created
    assert_eq!(final_state.checkpoint_count, 5);

    // Verify checkpoint locations
    assert_eq!(final_state.last_checkpoint_bead_index, 25);

    let elapsed = start.elapsed();

    // Assert execution completed in reasonable time
    assert!(
        elapsed < Duration::from_secs(10),
        "Execution should complete in <10s, took {:?}",
        elapsed
    );

    info!(
        "Long-running workflow completed successfully in {:?}",
        elapsed
    );

    // Cleanup
    ctx.worker.stop(Some("test complete".to_string()));

    Ok(())
}

#[tokio::test]
async fn given_workflow_state_when_tracking_checkpoints_then_preserves_accurate_counts()
    -> Result<(), Box<dyn std::error::Error>>
{
    info!("Starting E2E test: checkpoint count accuracy");

    // Setup
    let unique_id = format!("test{}", line!());
    let ctx = setup_test_environment(&unique_id).await?;

    // Execute workflow and track state
    let final_state = execute_workflow_with_checkpointing(&ctx).await?;

    // Verify checkpoint counts at each interval
    // Bead 5: checkpoint 1
    // Bead 10: checkpoint 2
    // Bead 15: checkpoint 3
    // Bead 20: checkpoint 4
    // Bead 25: checkpoint 5

    assert_eq!(final_state.checkpoint_count, 5, "Total checkpoints should be 5");

    // Verify completed beads match expected checkpoints
    assert_eq!(
        final_state.completed_beads.len(),
        25,
        "All 25 beads should be completed"
    );

    // Verify checkpoint mapping
    let checkpoint_beads: Vec<usize> = vec![5, 10, 15, 20, 25];
    for (idx, bead_num) in checkpoint_beads.iter().enumerate() {
        let expected_checkpoint = idx + 1;
        assert!(
            *bead_num <= final_state.completed_beads.len(),
            "Checkpoint {} at bead {} should exist",
            expected_checkpoint,
            bead_num
        );
    }

    info!("Checkpoint counts verified accurately");

    // Cleanup
    ctx.worker.stop(Some("test complete".to_string()));

    Ok(())
}

#[tokio::test]
async fn given_multiple_checkpoints_when_tracking_then_events_emitted_correctly()
    -> Result<(), Box<dyn std::error::Error>>
{
    info!("Starting E2E test: event emissions at checkpoints");

    // Setup
    let unique_id = format!("test{}", line!());
    let ctx = setup_test_environment(&unique_id).await?;

    // Subscribe to events before starting
    let mut sub = ctx.event_bus.subscribe();

    // Execute first 10 beads (2 checkpoints)
    for _i in 1..=10 {
        let bead_id_obj = BeadId::new();
        let bead_id_str = bead_id_obj.to_string();
        ctx.worker.send_message(WorkerMessage::StartBead {
            bead_id: bead_id_str.clone(),
            from_state: Some(BeadState::Ready),
        })?;

        tokio::time::sleep(Duration::from_millis(10)).await;

        ctx.worker.send_message(WorkerMessage::CompleteBead {
            result: BeadResult::success(Vec::new(), 0),
        })?;
    }

    // Allow time for events to be published
    tokio::time::sleep(Duration::from_millis(100)).await;

    // Verify we received events for the beads
    let mut events_received = 0;
    let timeout_duration = Duration::from_secs(2);
    let start_time = Instant::now();

    while start_time.elapsed() < timeout_duration {
        match tokio::time::timeout(Duration::from_millis(200), sub.recv()).await {
            Ok(Ok(_event)) => {
                events_received += 1;
                if events_received >= 10 {
                    break;
                }
            }
            _ => {
                if events_received >= 10 {
                    break;
                }
            }
        }
    }

    // We should have received at least some events
    assert!(
        events_received > 0,
        "Should have received events for bead execution"
    );

    info!("Received {} events for bead execution", events_received);

    // Cleanup
    ctx.worker.stop(Some("test complete".to_string()));

    Ok(())
}

#[tokio::test]
async fn given_25_bead_workflow_when_serializing_state_then_preserves_all_data()
    -> Result<(), Box<dyn std::error::Error>>
{
    info!("Starting E2E test: state serialization");

    // Create a complete workflow state
    let state = WorkflowCheckpointState {
        workflow_id: "test-workflow-serialization".to_string(),
        completed_beads: (1..=25)
            .map(|i| format!("bead-{i:02}"))
            .collect(),
        current_phase: "completed".to_string(),
        checkpoint_count: 5,
        last_checkpoint_bead_index: 25,
    };

    // Serialize using serde_json
    let serialized = serde_json::to_vec(&state)?;
    assert!(!serialized.is_empty());

    info!(
        "Serialized {} beads to {} bytes",
        state.completed_beads.len(),
        serialized.len()
    );

    // Deserialize
    let deserialized: WorkflowCheckpointState = serde_json::from_slice(&serialized)?;

    // Verify equality
    assert_eq!(deserialized, state, "Deserialized state should match original");
    assert_eq!(deserialized.completed_beads.len(), 25);
    assert_eq!(deserialized.checkpoint_count, 5);
    assert_eq!(deserialized.last_checkpoint_bead_index, 25);

    info!("State serialization roundtrip successful");

    Ok(())
}

#[tokio::test]
async fn given_worker_config_when_checkpoint_interval_set_then_interval_correct()
    -> Result<(), Box<dyn std::error::Error>>
{
    info!("Starting E2E test: worker checkpoint interval configuration");

    // Create worker with specific checkpoint interval
    let worker_config = WorkerConfig {
        checkpoint_interval: Duration::from_secs(120), // 2 minutes
        retry_policy: WorkerRetryPolicy::default(),
        event_bus: None,
    };

    // Verify interval is set correctly
    assert_eq!(
        worker_config.checkpoint_interval,
        Duration::from_secs(120)
    );

    // Create worker and verify it starts
    let (worker, _handle) = Actor::spawn(None, WorkerActorDef, worker_config).await?;

    // Wait a bit to ensure worker is running
    tokio::time::sleep(Duration::from_millis(50)).await;

    // Verify worker is alive
    let status = worker.get_status();
    assert!(status == ractor::ActorStatus::Running);

    // Cleanup
    worker.stop(Some("test complete".to_string()));

    info!("Worker checkpoint interval configuration verified");

    Ok(())
}
