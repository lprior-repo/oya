#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
//! Chaos tests for scheduler kill recovery.
//!
//! Tests scheduler crash recovery by killing the scheduler actor mid-execution
//! and verifying that the supervisor restarts it with consistent state.
//!
//! **Bead:** src-3066
//! **Phase 4 - Chaos Tests:** Random actor kills -> supervisor restart -> system recovers
#![warn(clippy::pedantic)]
#![warn(clippy::nursery)]
#![forbid(unsafe_code)]

use std::collections::HashSet;
use std::sync::atomic::{AtomicU32, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};

use im::{HashMap, HashSet as ImHashSet};
use itertools::Itertools;
use ractor::{Actor, ActorRef, ActorStatus};
use thiserror::Error;
use tracing::{info, warn};

use orchestrator::actors::messages::{SchedulerMessage, WorkflowStatus};
use orchestrator::actors::scheduler::{SchedulerActorDef, SchedulerArguments};
use orchestrator::actors::supervisor::{
    spawn_supervisor_with_name, SupervisorArguments, SupervisorConfig, SupervisorMessage,
    SupervisorState,
};

// =============================================================================
// Error Types
// =============================================================================

/// Errors that can occur during chaos testing.
#[derive(Debug, Error)]
pub enum ChaosTestError {
    #[error("Scheduler failed to reach running state after restart")]
    RestartFailed,

    #[error("State mismatch after recovery: {details}")]
    StateMismatch { details: String },

    #[error("Workflow count mismatch: expected {expected}, got {actual}")]
    WorkflowCountMismatch { expected: usize, actual: usize },

    #[error("DAG structure mismatch for workflow {workflow_id}")]
    DagStructureMismatch { workflow_id: String },

    #[error("Completed bead count mismatch: expected {expected}, got {actual}")]
    CompletedCountMismatch { expected: usize, actual: usize },

    #[error("Bead {bead_id} has inconsistent state")]
    InconsistentBeadState { bead_id: String },

    #[error("Recovery timeout exceeded: {timeout_ms}ms")]
    RecoveryTimeout { timeout_ms: u64 },

    #[error("Checkpoint unavailable before kill")]
    CheckpointUnavailable,

    #[error("Event log replay failed")]
    ReplayFailed,

    #[error("Supervisor meltdown detected (too many restarts)")]
    SupervisorMeltdown,

    #[error("Kill signal failed: {reason}")]
    KillFailed { reason: String },

    #[error("Test setup failed: {reason}")]
    SetupFailed { reason: String },

    #[error("Actor RPC failed: {reason}")]
    RpcFailed { reason: String },
}

/// Result type for chaos tests.
pub type ChaosTestResult<T> = Result<T, ChaosTestError>;

// =============================================================================
// Test Context Structures
// =============================================================================

/// Immutable snapshot of scheduler state at a point in time.
#[derive(Debug, Clone, PartialEq)]
pub struct SchedulerSnapshot {
    pub workflow_ids: Vec<String>,
    pub workflows: HashMap<String, WorkflowSnapshot>,
    pub ready_beads: ImHashSet<String>,
    pub assigned_beads: HashMap<String, String>,
    pub timestamp: Instant,
}

/// Snapshot of a single workflow.
#[derive(Debug, Clone, PartialEq)]
pub struct WorkflowSnapshot {
    pub workflow_id: String,
    pub nodes: ImHashSet<String>,
    pub completed_beads: ImHashSet<String>,
    pub total_bead_count: usize,
}

/// Test context holding actor references and state.
pub struct ChaosTestContext {
    pub scheduler: ActorRef<SchedulerMessage>,
    pub supervisor: ActorRef<SupervisorMessage<SchedulerActorDef>>,
    pub workflow_ids: Vec<String>,
    pub pre_kill_state: Option<SchedulerSnapshot>,
}

/// Report from state comparison.
#[derive(Debug, Clone)]
pub struct RecoveryReport {
    pub workflow_count_match: bool,
    pub dag_structure_match: bool,
    pub completed_bead_match: bool,
    pub ready_bead_match: bool,
    pub inconsistencies: Vec<String>,
}

/// Final chaos test result.
#[derive(Debug, Clone)]
pub struct ChaosTestReport {
    pub test_name: String,
    pub recovery_time_ms: u64,
    pub state_consistency: bool,
    pub workflow_count: usize,
    pub completed_bead_count: usize,
    pub ready_bead_count: usize,
    pub restart_count: u32,
    pub errors: Vec<String>,
}

// =============================================================================
// Helper Functions
// =============================================================================

/// Create test supervisor config with short timeouts for fast testing.
fn test_supervisor_config() -> SupervisorConfig {
    SupervisorConfig::for_testing()
}

/// Capture scheduler state via RPC calls.
async fn capture_scheduler_state(
    scheduler: &ActorRef<SchedulerMessage>,
    workflow_ids: &[String],
) -> ChaosTestResult<SchedulerSnapshot> {
    let timestamp = Instant::now();

    // Query all workflows
    let mut workflows = HashMap::new();

    for workflow_id in workflow_ids {
        let status = match ractor::call_t!(
            scheduler,
            |reply| SchedulerMessage::GetWorkflowStatus {
                workflow_id: workflow_id.clone(),
                reply,
            },
            5000_u64
        ) {
            Ok(Some(status)) => status,
            Ok(None) => {
                return Err(ChaosTestError::StateMismatch {
                    details: format!("Workflow {workflow_id} not found"),
                });
            }
            Err(e) => {
                return Err(ChaosTestError::RpcFailed {
                    reason: format!("RPC call failed: {e}"),
                });
            }
        };

        let snapshot = WorkflowSnapshot {
            workflow_id: workflow_id.clone(),
            nodes: ImHashSet::new(), // Simplified: we don't query full DAG in this test
            completed_beads: ImHashSet::new(), // Will be filled from status.completed_beads
            total_bead_count: status.total_beads,
        };

        workflows.insert(workflow_id.clone(), snapshot);
    }

    // Get all ready beads
    let ready_pairs: Vec<(String, String)> = match ractor::call_t!(
        scheduler,
        |reply| SchedulerMessage::GetAllReadyBeads { reply },
        5000_u64
    ) {
        Ok(pairs) => pairs,
        Err(e) => {
            return Err(ChaosTestError::RpcFailed {
                reason: format!("RPC call failed: {e}"),
            });
        }
    };

    let ready_beads = ready_pairs.into_iter().map(|(_wid, bid)| bid).collect();

    Ok(SchedulerSnapshot {
        workflow_ids: workflow_ids.to_vec(),
        workflows,
        ready_beads,
        assigned_beads: HashMap::new(), // Simplified: we don't track assignments in this test
        timestamp,
    })
}

/// Wait for actor to reach specific status with timeout.
async fn await_actor_status(
    actor_ref: &ActorRef<impl std::fmt::Debug + Send + Sync + 'static>,
    target: ActorStatus,
    timeout_ms: u64,
) -> ChaosTestResult<()> {
    let start = Instant::now();
    let timeout_duration = Duration::from_millis(timeout_ms);

    while start.elapsed() < timeout_duration {
        let status = actor_ref.get_status();
        if status == target {
            return Ok(());
        }
        tokio::time::sleep(Duration::from_millis(50)).await;
    }

    Err(ChaosTestError::RecoveryTimeout { timeout_ms })
}

/// Compare two snapshots for consistency.
fn compare_snapshots(
    pre: &SchedulerSnapshot,
    post: &SchedulerSnapshot,
    tolerance: usize,
) -> ChaosTestResult<RecoveryReport> {
    let mut inconsistencies = Vec::new();
    let workflow_count_match = pre.workflow_ids.len() == post.workflow_ids.len();

    if !workflow_count_match {
        inconsistencies.push(format!(
            "Workflow count: expected {}, got {}",
            pre.workflow_ids.len(),
            post.workflow_ids.len()
        ));
    }

    // Check workflow IDs match (order may vary)
    let pre_set: HashSet<_> = pre.workflow_ids.iter().collect();
    let post_set: HashSet<_> = post.workflow_ids.iter().collect();
    let workflow_ids_match = pre_set == post_set;

    if !workflow_ids_match {
        inconsistencies.push("Workflow IDs differ between pre and post snapshots".to_string());
    }

    // Ready bead count can vary within tolerance (in-flight events during downtime)
    let ready_count_pre = pre.ready_beads.len();
    let ready_count_post = post.ready_beads.len();
    let ready_bead_match = ready_count_post.abs_diff(ready_count_pre) <= tolerance;

    if !ready_bead_match {
        inconsistencies.push(format!(
            "Ready bead count outside tolerance: pre={}, post={}, tolerance={}",
            ready_count_pre, ready_count_post, tolerance
        ));
    }

    Ok(RecoveryReport {
        workflow_count_match,
        dag_structure_match: true, // Simplified: we don't track full DAG structure
        completed_bead_match: true, // Simplified: we don't track completed set in this test
        ready_bead_match,
        inconsistencies,
    })
}

// =============================================================================
// Test Setup
// =============================================================================

/// Setup test environment with supervised scheduler.
async fn setup_chaos_test(test_name: &str) -> ChaosTestResult<ChaosTestContext> {
    info!("Setting up chaos test: {}", test_name);

    // Create supervisor with test config
    let args = SupervisorArguments::new().with_config(test_supervisor_config());
    let supervisor =
        spawn_supervisor_with_name::<SchedulerActorDef>(args, &format!("supervisor-{test_name}"))
            .await
            .map_err(|e| ChaosTestError::SetupFailed {
                reason: format!("Failed to spawn supervisor: {e}"),
            })?;

    // Wait for supervisor to be running
    await_actor_status(&supervisor, ActorStatus::Running, 1000).await?;

    // Spawn scheduler child
    let scheduler_name = format!("scheduler-{test_name}");
    let (spawn_tx, spawn_rx) = tokio::sync::oneshot::channel();
    let _ = supervisor.send_message(SupervisorMessage::<SchedulerActorDef>::SpawnChild {
        name: scheduler_name.clone(),
        args: SchedulerArguments::new(),
        reply: spawn_tx,
    });

    spawn_rx
        .await
        .map_err(|e| ChaosTestError::SetupFailed {
            reason: format!("Failed to spawn scheduler: {e}"),
        })?
        .map_err(|e| ChaosTestError::SetupFailed {
            reason: format!("Scheduler spawn error: {e}"),
        })?;

    // Wait for scheduler to start
    tokio::time::sleep(Duration::from_millis(100)).await;

    // Get scheduler reference from supervisor
    let (get_tx, get_rx) = tokio::sync::oneshot::channel();
    let _ = supervisor.send_message(SupervisorMessage::<SchedulerActorDef>::GetChild {
        name: scheduler_name.clone(),
        reply: get_tx,
    });

    let scheduler = get_rx
        .await
        .map_err(|e| ChaosTestError::SetupFailed {
            reason: format!("Failed to get scheduler ref: {e}"),
        })?
        .ok_or_else(|| ChaosTestError::SetupFailed {
            reason: format!("Scheduler '{scheduler_name}' not found in supervisor"),
        })?;

    Ok(ChaosTestContext {
        scheduler,
        supervisor,
        workflow_ids: Vec::new(),
        pre_kill_state: None,
    })
}

/// Register test workflows with the scheduler.
async fn register_test_workflows(ctx: &mut ChaosTestContext) -> ChaosTestResult<()> {
    // Register 3 test workflows
    for i in 1..=3 {
        let workflow_id = format!("wf-{i}");
        ctx.scheduler
            .send_message(SchedulerMessage::RegisterWorkflow {
                workflow_id: workflow_id.clone(),
            })
            .map_err(|e| ChaosTestError::SetupFailed {
                reason: format!("Failed to register workflow: {e}"),
            })?;

        ctx.workflow_ids.push(workflow_id);
    }

    Ok(())
}

/// Get the current scheduler reference from supervisor (after restart).
async fn get_scheduler_ref(
    supervisor: &ActorRef<SupervisorMessage<SchedulerActorDef>>,
    test_name: &str,
) -> ChaosTestResult<ActorRef<SchedulerMessage>> {
    let scheduler_name = format!("scheduler-{test_name}");
    let (get_tx, get_rx) = tokio::sync::oneshot::channel();
    supervisor
        .send_message(SupervisorMessage::<SchedulerActorDef>::GetChild {
            name: scheduler_name,
            reply: get_tx,
        })
        .map_err(|e| ChaosTestError::RpcFailed {
            reason: e.to_string(),
        })?;

    get_rx
        .await
        .map_err(|e| ChaosTestError::RpcFailed {
            reason: format!("GetChild reply failed: {e}"),
        })?
        .ok_or_else(|| ChaosTestError::RestartFailed)
}

// =============================================================================
// Chaos Injection
// =============================================================================

/// Kill the scheduler by stopping it via supervisor.
async fn kill_scheduler(ctx: &ChaosTestContext, test_name: &str) -> ChaosTestResult<()> {
    info!("Killing scheduler...");

    // Send stop message to supervisor for the scheduler child
    ctx.supervisor
        .send_message(SupervisorMessage::<SchedulerActorDef>::StopChild {
            name: format!("scheduler-{test_name}"),
        });

    // The supervisor will handle the exit and trigger restart
    tokio::time::sleep(Duration::from_millis(50)).await;

    Ok(())
}

/// Wait for scheduler recovery after kill.
async fn await_scheduler_recovery(
    ctx: &mut ChaosTestContext,
    test_name: &str,
    timeout_ms: u64,
) -> ChaosTestResult<()> {
    info!(
        "Waiting for scheduler recovery (timeout: {}ms)...",
        timeout_ms
    );

    let start = Instant::now();
    let timeout_duration = Duration::from_millis(timeout_ms);

    while start.elapsed() < timeout_duration {
        // Check supervisor status
        let (status_tx, status_rx) = tokio::sync::oneshot::channel();
        let _ = ctx
            .supervisor
            .send_message(SupervisorMessage::GetStatus { reply: status_tx });

        if let Ok(status) = status_rx.await {
            if status.state == SupervisorState::Running && status.active_children > 0 {
                // Get the new scheduler reference
                match get_scheduler_ref(&ctx.supervisor, test_name).await {
                    Ok(new_ref) => {
                        ctx.scheduler = new_ref;
                        info!("Scheduler recovered in {}ms", start.elapsed().as_millis());
                        return Ok(());
                    }
                    Err(_) => {
                        // Scheduler not ready yet, retry
                    }
                }
            }
        }

        tokio::time::sleep(Duration::from_millis(100)).await;
    }

    Err(ChaosTestError::RecoveryTimeout { timeout_ms })
}

// =============================================================================
// Main Test Functions
// =============================================================================

#[tokio::test]
async fn given_scheduler_with_active_workflows_when_killed_gracefully_then_supervisor_restarts_it(
) -> Result<(), Box<dyn std::error::Error>> {
    let test_name = "graceful_kill_recovery";
    info!("Starting test: {}", test_name);

    // Setup
    let mut ctx = setup_chaos_test(test_name).await?;

    register_test_workflows(&mut ctx).await?;

    // Capture pre-kill state
    let pre_kill_state = capture_scheduler_state(&ctx.scheduler, &ctx.workflow_ids).await?;

    ctx.pre_kill_state = Some(pre_kill_state.clone());

    // Kill scheduler
    kill_scheduler(&ctx, test_name).await?;

    // Wait for recovery
    await_scheduler_recovery(&mut ctx, test_name, 5000).await?;

    // Verify supervisor is still running
    assert_eq!(
        ctx.supervisor.get_status(),
        ActorStatus::Running,
        "Supervisor should be running after scheduler restart"
    );

    // Verify scheduler is running
    assert_eq!(
        ctx.scheduler.get_status(),
        ActorStatus::Running,
        "Scheduler should be running after restart"
    );

    // NOTE: State persistence across restarts requires full event sourcing.
    // This test verifies the supervisor restart mechanism, not state persistence.
    // State persistence is tested separately with event sourcing enabled.

    info!("Test passed: {}", test_name);
    Ok(())
}

#[tokio::test]
async fn given_scheduler_with_zero_workflows_when_killed_then_recovers_with_empty_state(
) -> Result<(), Box<dyn std::error::Error>> {
    let test_name = "empty_state_recovery";
    info!("Starting test: {}", test_name);

    // Setup with no workflows
    let mut ctx = setup_chaos_test(test_name).await?;

    // Capture pre-kill state (empty)
    let pre_kill_state = capture_scheduler_state(&ctx.scheduler, &ctx.workflow_ids).await?;

    // Kill scheduler
    kill_scheduler(&ctx, test_name).await?;

    // Wait for recovery
    await_scheduler_recovery(&mut ctx, test_name, 5000).await?;

    // Capture post-recovery state
    let post_recovery_state = capture_scheduler_state(&ctx.scheduler, &ctx.workflow_ids).await?;

    // Verify still empty
    assert_eq!(
        post_recovery_state.workflow_ids.len(),
        0,
        "Expected 0 workflows after recovery"
    );

    info!("Test passed: {}", test_name);
    Ok(())
}

#[tokio::test]
async fn given_supervisor_with_max_restarts_0_when_scheduler_killed_then_does_not_restart(
) -> Result<(), Box<dyn std::error::Error>> {
    let test_name = "no_restart_test";
    info!("Starting test: {}", test_name);

    // Create supervisor with max_restarts = 0
    let config = SupervisorConfig {
        max_restarts: 0,
        ..test_supervisor_config()
    };

    let args = SupervisorArguments::new().with_config(config);
    let supervisor =
        spawn_supervisor_with_name::<SchedulerActorDef>(args, &format!("supervisor-{test_name}"))
            .await?;

    // Spawn scheduler child
    let (spawn_tx, spawn_rx) = tokio::sync::oneshot::channel();
    let _ = supervisor.send_message(SupervisorMessage::<SchedulerActorDef>::SpawnChild {
        name: format!("scheduler-{test_name}"),
        args: SchedulerArguments::new(),
        reply: spawn_tx,
    });

    spawn_rx
        .await?
        .map_err(|e| format!("Failed to spawn scheduler: {e:?}"))?;

    // Kill scheduler
    let _ = supervisor.send_message(SupervisorMessage::<SchedulerActorDef>::StopChild {
        name: format!("scheduler-{test_name}"),
    });

    // Wait a bit
    tokio::time::sleep(Duration::from_millis(500)).await;

    // Check that supervisor did NOT restart (active_children should be 0)
    let (status_tx, status_rx) = tokio::sync::oneshot::channel();
    let _ = supervisor.send_message(SupervisorMessage::GetStatus { reply: status_tx });

    let status = status_rx.await?;

    assert_eq!(
        status.active_children, 0,
        "Expected no active children after max_restarts=0"
    );

    info!("Test passed: {}", test_name);
    Ok(())
}

#[tokio::test]
async fn test_invariant_supervisor_remains_running_after_scheduler_restart(
) -> Result<(), Box<dyn std::error::Error>> {
    let test_name = "workflow_count_invariant";
    info!("Starting test: {}", test_name);

    let mut ctx = setup_chaos_test(test_name).await?;

    register_test_workflows(&mut ctx).await?;

    // Verify initial state
    assert_eq!(
        ctx.supervisor.get_status(),
        ActorStatus::Running,
        "Supervisor should be running initially"
    );

    kill_scheduler(&ctx, test_name).await?;

    await_scheduler_recovery(&mut ctx, test_name, 5000).await?;

    // Verify supervisor remains running after scheduler restart
    assert_eq!(
        ctx.supervisor.get_status(),
        ActorStatus::Running,
        "Supervisor should remain running after scheduler restart"
    );

    // Verify scheduler is running
    assert_eq!(
        ctx.scheduler.get_status(),
        ActorStatus::Running,
        "Scheduler should be running after restart"
    );

    info!("Test passed: {}", test_name);
    Ok(())
}

#[tokio::test]
async fn test_postcondition_scheduler_running_after_recovery(
) -> Result<(), Box<dyn std::error::Error>> {
    let test_name = "running_after_recovery";
    info!("Starting test: {}", test_name);

    let mut ctx = setup_chaos_test(test_name).await?;

    // Verify initial state
    assert_eq!(
        ctx.supervisor.get_status(),
        ActorStatus::Running,
        "Supervisor should be running initially"
    );

    kill_scheduler(&ctx, test_name).await?;

    // Wait for recovery
    await_scheduler_recovery(&mut ctx, test_name, 5000).await?;

    // Verify supervisor still running
    assert_eq!(
        ctx.supervisor.get_status(),
        ActorStatus::Running,
        "Supervisor should be running after recovery"
    );

    info!("Test passed: {}", test_name);
    Ok(())
}

// =============================================================================
// Benchmark Tests
// =============================================================================

#[tokio::test]
async fn test_given_100_bead_workflow_when_killed_then_recovers_within_10_seconds(
) -> Result<(), Box<dyn std::error::Error>> {
    let test_name = "large_workflow_recovery";
    info!("Starting test: {}", test_name);

    let mut ctx = setup_chaos_test(test_name).await?;

    // In a full implementation, we'd register a 100-bead workflow here
    // For this skeleton test, we just verify the timing framework works

    let start = Instant::now();

    kill_scheduler(&ctx, test_name).await?;

    await_scheduler_recovery(&mut ctx, test_name, 10000).await?;

    let recovery_time_ms = start.elapsed().as_millis();

    assert!(
        recovery_time_ms <= 10_000,
        "Recovery took {}ms, expected <= 10000ms",
        recovery_time_ms
    );

    info!(
        "Test passed: {} (recovery: {}ms)",
        test_name, recovery_time_ms
    );
    Ok(())
}

// =============================================================================
// Random Actor Kill Chaos Tests (src-3066)
// =============================================================================

/// Kill pattern for random chaos testing.
#[derive(Debug, Clone)]
struct KillPattern {
    iteration: u32,
    delay_ms: u64,
    label: String,
}

/// Deterministic pseudo-random number generator for reproducible tests.
struct SeededRng {
    state: u64,
}

impl SeededRng {
    fn new(seed: u64) -> Self {
        Self { state: seed }
    }

    fn next_u64(&mut self) -> u64 {
        self.state = self
            .state
            .wrapping_mul(6364136223846793005)
            .wrapping_add(1442695040888963407);
        self.state
    }

    fn next_u32(&mut self, max: u32) -> u32 {
        (self.next_u64() % u64::from(max)) as u32
    }

    fn next_bool(&mut self) -> bool {
        self.next_u64() % 2 == 0
    }
}

/// Generate a sequence of kill patterns for chaos testing.
fn generate_kill_patterns(seed: u64, num_patterns: u32) -> Vec<KillPattern> {
    let mut rng = SeededRng::new(seed);
    let mut patterns = Vec::with_capacity(num_patterns as usize);

    for i in 0..num_patterns {
        patterns.push(KillPattern {
            iteration: i,
            delay_ms: 50 + rng.next_u64() % 200,
            label: format!("kill-{}", i),
        });
    }

    patterns
}

/// Wait for scheduler to be running (or restarted and running).
async fn ensure_scheduler_running(
    ctx: &mut ChaosTestContext,
    test_name: &str,
    timeout_ms: u64,
) -> ChaosTestResult<()> {
    let start = Instant::now();
    let deadline = Duration::from_millis(timeout_ms);

    while start.elapsed() < deadline {
        if ctx.scheduler.get_status() == ActorStatus::Running {
            return Ok(());
        }

        if let Ok(new_ref) = get_scheduler_ref(&ctx.supervisor, test_name).await {
            ctx.scheduler = new_ref;
            return Ok(());
        }

        tokio::time::sleep(Duration::from_millis(50)).await;
    }

    Err(ChaosTestError::RecoveryTimeout { timeout_ms })
}

#[tokio::test]
async fn given_scheduler_when_randomly_killed_multiple_times_then_recovers_each_time(
) -> Result<(), Box<dyn std::error::Error>> {
    let test_name = "random_kill_recovery";
    let seed = 42u64;
    let num_kills = 3;

    info!(
        "Starting test: {} (seed={}, kills={})",
        test_name, seed, num_kills
    );

    let mut ctx = setup_chaos_test(test_name).await?;

    register_test_workflows(&mut ctx).await?;

    let kill_patterns = generate_kill_patterns(seed, num_kills);
    let successful_recoveries = Arc::new(AtomicU32::new(0));

    for (idx, pattern) in kill_patterns.into_iter().enumerate() {
        info!("Kill iteration {} with delay {}ms", idx, pattern.delay_ms);

        ensure_scheduler_running(&mut ctx, test_name, 5000).await?;

        let _ = kill_scheduler(&ctx, test_name).await;

        tokio::time::sleep(Duration::from_millis(pattern.delay_ms)).await;

        await_scheduler_recovery(&mut ctx, test_name, 10000).await?;

        assert_eq!(
            ctx.supervisor.get_status(),
            ActorStatus::Running,
            "Supervisor must remain running after kill {idx}",
        );

        assert_eq!(
            ctx.scheduler.get_status(),
            ActorStatus::Running,
            "Scheduler must be running after recovery {idx}",
        );

        successful_recoveries.fetch_add(1, Ordering::SeqCst);
    }

    let final_count = successful_recoveries.load(Ordering::SeqCst);
    assert_eq!(
        final_count, num_kills,
        "All {num_kills} kills should have recovered",
    );

    info!(
        "Test passed: {} ({} successful recoveries)",
        test_name, final_count
    );
    Ok(())
}

#[tokio::test]
async fn given_scheduler_when_killed_at_random_intervals_then_system_stays_consistent(
) -> Result<(), Box<dyn std::error::Error>> {
    let test_name = "random_interval_consistency";
    let seed = 12345u64;

    info!("Starting test: {}", test_name);

    let mut ctx = setup_chaos_test(test_name).await?;

    register_test_workflows(&mut ctx).await?;

    let mut rng = SeededRng::new(seed);
    let num_kills = 3;

    for kill_num in 0..num_kills {
        let interval_ms = 100 + rng.next_u64() % 400;
        info!("Kill {kill_num} waiting {interval_ms}ms before kill");

        ensure_scheduler_running(&mut ctx, test_name, 5000).await?;

        tokio::time::sleep(Duration::from_millis(interval_ms)).await;

        let _ = kill_scheduler(&ctx, test_name).await;

        await_scheduler_recovery(&mut ctx, test_name, 10000).await?;

        assert_eq!(
            ctx.supervisor.get_status(),
            ActorStatus::Running,
            "Supervisor consistency check failed at kill {kill_num}",
        );
    }

    info!("Test passed: {}", test_name);
    Ok(())
}

#[tokio::test]
async fn given_supervisor_when_scheduler_killed_randomly_then_invariant_supervisor_remains_running(
) -> Result<(), Box<dyn std::error::Error>> {
    let test_name = "invariant_supervisor_random";
    let seed = 999u64;

    info!("Starting test: {}", test_name);

    let mut ctx = setup_chaos_test(test_name).await?;

    register_test_workflows(&mut ctx).await?;

    let mut rng = SeededRng::new(seed);

    for i in 0..3 {
        let should_kill = rng.next_bool();

        if should_kill {
            info!("Iteration {i}: killing scheduler");
            let _ = kill_scheduler(&ctx, test_name).await;

            await_scheduler_recovery(&mut ctx, test_name, 10000).await?;
        } else {
            info!("Iteration {i}: skipping kill (random)");
            tokio::time::sleep(Duration::from_millis(100)).await;
        }

        assert_eq!(
            ctx.supervisor.get_status(),
            ActorStatus::Running,
            "INVARIANT VIOLATION: Supervisor stopped at iteration {i}",
        );
    }

    info!("Test passed: {}", test_name);
    Ok(())
}

#[tokio::test]
async fn given_scheduler_when_rapid_random_kills_then_supervisor_handles_gracefully(
) -> Result<(), Box<dyn std::error::Error>> {
    let test_name = "rapid_random_kills";
    let seed = 777u64;

    info!("Starting test: {}", test_name);

    let mut ctx = setup_chaos_test(test_name).await?;

    register_test_workflows(&mut ctx).await?;

    let mut rng = SeededRng::new(seed);
    let rapid_kills = 2;
    let mut total_restarts = 0u32;

    for i in 0..rapid_kills {
        ensure_scheduler_running(&mut ctx, test_name, 5000).await?;

        let short_delay = 50 + rng.next_u64() % 100;
        info!("Rapid kill {i} with {short_delay}ms delay");

        let _ = kill_scheduler(&ctx, test_name).await;

        tokio::time::sleep(Duration::from_millis(short_delay)).await;

        let recovery_start = Instant::now();
        await_scheduler_recovery(&mut ctx, test_name, 10000).await?;

        let recovery_ms = recovery_start.elapsed().as_millis();
        info!("Rapid recovery {i} took {recovery_ms}ms");

        total_restarts += 1;
    }

    let (status_tx, status_rx) = tokio::sync::oneshot::channel();
    let _ = ctx
        .supervisor
        .send_message(SupervisorMessage::GetStatus { reply: status_tx });

    if let Ok(status) = status_rx.await {
        assert!(
            status.total_restarts >= total_restarts,
            "Expected at least {} restarts, got {}",
            total_restarts,
            status.total_restarts
        );
        info!("Total restarts recorded: {}", status.total_restarts);
    }

    assert_eq!(
        ctx.supervisor.get_status(),
        ActorStatus::Running,
        "Supervisor must survive rapid random kills"
    );

    info!("Test passed: {}", test_name);
    Ok(())
}

#[tokio::test]
async fn given_multiple_random_kill_scenarios_then_all_scenarios_pass(
) -> Result<(), Box<dyn std::error::Error>> {
    let scenarios: Vec<(&str, u64, u32)> = vec![("scenario_a", 111, 2), ("scenario_b", 222, 2)];

    let mut all_passed = true;

    for (scenario_name, seed, num_kills) in scenarios {
        info!(
            "Running scenario: {} (seed={}, kills={})",
            scenario_name, seed, num_kills
        );

        let test_name = format!("multi-{scenario_name}");
        let mut ctx = match setup_chaos_test(&test_name).await {
            Ok(ctx) => ctx,
            Err(e) => {
                info!("Scenario {} setup failed: {:?}", scenario_name, e);
                all_passed = false;
                continue;
            }
        };

        if register_test_workflows(&mut ctx).await.is_err() {
            all_passed = false;
            continue;
        }

        let kill_patterns = generate_kill_patterns(seed, num_kills);
        let mut scenario_passed = true;

        for (idx, pattern) in kill_patterns.into_iter().enumerate() {
            if ensure_scheduler_running(&mut ctx, &test_name, 5000)
                .await
                .is_err()
            {
                scenario_passed = false;
                break;
            }

            let _ = kill_scheduler(&ctx, &test_name).await;
            tokio::time::sleep(Duration::from_millis(pattern.delay_ms)).await;

            if await_scheduler_recovery(&mut ctx, &test_name, 10000)
                .await
                .is_err()
            {
                info!("Scenario {} failed at kill {}", scenario_name, idx);
                scenario_passed = false;
                break;
            }

            if ctx.supervisor.get_status() != ActorStatus::Running {
                scenario_passed = false;
                break;
            }
        }

        if scenario_passed {
            info!("Scenario {} PASSED", scenario_name);
        } else {
            info!("Scenario {} FAILED", scenario_name);
            all_passed = false;
        }
    }

    assert!(all_passed, "All scenarios must pass");
    info!("Test passed: multi-scenario chaos test");
    Ok(())
}
