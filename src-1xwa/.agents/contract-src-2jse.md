# Contract Specification: Chaos Test - Kill Scheduler Mid-Execution Recovery

## Context

- **Feature**: Chaos engineering test for scheduler crash recovery
- **Domain Terms**:
  - *SchedulerActor*: Ractor-based workflow scheduler managing DAG execution
  - *Supervisor*: Generic supervisor with one-for-one restart strategy and exponential backoff
  - *WorkflowState*: In-memory representation of workflow DAG and bead completion tracking
  - *Chaos*: Intentional termination of actor mid-execution to test recovery
  - *Recovery*: Restoration of state from checkpoint/event log after restart

- **Assumptions**:
  1. Scheduler is supervised by a SupervisorActor with restart capability
  2. CheckpointManager can persist scheduler state before termination
  3. ReplayEngine can restore state from event log on restart
  4. Workflow state is persisted (in-progress workflows, bead assignments)
  5. Workers continue execution during scheduler downtime (eventual consistency)
  6. Test environment allows SIGTERM/SIGKILL injection

- **Open Questions**:
  1. Should test simulate graceful shutdown (SIGTERM) or hard kill (SIGKILL)?
     - **Decision**: Test both - graceful for normal ops, hard kill for chaos
  2. What is the maximum acceptable downtime for recovery?
     - **Decision**: Measure and report; target < 5s for 100-bead workflow
  3. How to verify state consistency without external observability?
     - **Decision**: Query scheduler state via RPC after recovery, compare to pre-kill snapshot
  4. Should test verify worker orphan handling?
     - **Decision**: Out of scope for this bead; workers re-register on scheduler restart

## Preconditions

- [CP1] SchedulerActor is running and has active workflows in progress
- [CP2] At least one workflow has >= 3 beads in various states (pending, ready, assigned, completed)
- [CP3] Supervisor is configured with reasonable restart limits (max_restarts >= 1)
- [CP4] CheckpointManager is available and can persist state
- [CP5] ReplayEngine is initialized and subscribed to event stream
- [CP6] Test has RPC handle to scheduler for state queries before/after kill
- [CP7] No other system components are crashing (isolate failure to scheduler)

## Postconditions

- [PP1] After kill and supervisor-triggered restart, scheduler reaches Running state
- [PP2] Recovered scheduler state matches pre-kill state:
  - [PP2.1] All workflow IDs are preserved
  - [PP2.2] DAG structure (dependencies) is intact
  - [PP2.3] Completed bead set is identical
  - [PP2.4] Ready bead set is identical (may have new beads from in-flight completions)
- [PP3] Scheduler resumes processing ready beads after recovery
- [PP4] Worker assignments are either:
  - [PP4.1] Preserved (if checkpointed), OR
  - [PP4.2] Cleared (workers must re-claim beads on restart)
- [PP5] No duplicate bead executions occur (idempotency)
- [PP6] Total recovery time (kill → running) is measurable and logged

## Invariants

- [INV1] Workflow count is non-decreasing during normal operation (workflows only added)
- [INV2] Completed bead set is monotonic (beads never un-complete)
- [INV3] DAG structure is immutable after registration
- [INV4] No bead exists in both ready and assigned sets simultaneously
- [INV5] No bead exists in both ready and completed sets simultaneously
- [INV6] Supervisor restart count increments by exactly 1 after kill

## Error Taxonomy

```rust
#[derive(Debug, Clone, thiserror::Error)]
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
}
```

## Contract Signatures

### Core Test Functions

```rust
/// Setup test environment with workflows and beads
fn setup_chaos_test() -> Result<ChaosTestContext, ChaosTestError>
where
    ChaosTestContext {
        scheduler: ActorRef<SchedulerMessage>,
        supervisor: ActorRef<SupervisorMessage<SchedulerActorDef>>,
        workflow_ids: Vec<WorkflowId>,
        pre_kill_state: SchedulerSnapshot,
    }

/// Capture scheduler state before chaos injection
async fn capture_pre_kill_state(
    scheduler: &ActorRef<SchedulerMessage>
) -> Result<SchedulerSnapshot, ChaosTestError>

/// Inject chaos by stopping scheduler actor
async fn kill_scheduler(
    scheduler: &ActorRef<SchedulerMessage>
) -> Result<(), ChaosTestError>

/// Wait for supervisor to restart scheduler and reach running state
async fn await_scheduler_recovery(
    supervisor: &ActorRef<SupervisorMessage<SchedulerActorDef>>,
    timeout_ms: u64
) -> Result<ActorRef<SchedulerMessage>, ChaosTestError>

/// Capture scheduler state after recovery
async fn capture_post_recovery_state(
    scheduler: &ActorRef<SchedulerMessage>
) -> Result<SchedulerSnapshot, ChaosTestError>

/// Verify state consistency across recovery
fn verify_recovery_consistency(
    pre: &SchedulerSnapshot,
    post: &SchedulerSnapshot
) -> Result<RecoveryReport, ChaosTestError>

/// Main chaos test orchestrator
async fn test_chaos_scheduler_kill_recovery() -> Result<ChaosTestResult, ChaosTestError>
```

### Snapshot Structures

```rust
/// Immutable snapshot of scheduler state at a point in time
#[derive(Debug, Clone, PartialEq)]
pub struct SchedulerSnapshot {
    pub workflow_ids: Vec<String>,
    pub workflows: HashMap<WorkflowId, WorkflowSnapshot>,
    pub ready_beads: HashSet<BeadId>,
    pub assigned_beads: HashMap<BeadId, String>,
    pub timestamp: Instant,
}

#[derive(Debug, Clone, PartialEq)]
pub struct WorkflowSnapshot {
    pub workflow_id: String,
    pub dag: DAGStructure,
    pub completed_beads: HashSet<BeadId>,
    pub total_bead_count: usize,
}

#[derive(Debug, Clone, PartialEq)]
pub struct DAGStructure {
    pub nodes: HashSet<BeadId>,
    pub edges: HashSet<(BeadId, BeadId)>,
}
```

### Test Result Structures

```rust
#[derive(Debug, Clone)]
pub struct ChaosTestResult {
    pub test_name: String,
    pub recovery_time_ms: u64,
    pub state_consistency: bool,
    pub workflow_count: usize,
    pub completed_bead_count: usize,
    pub ready_bead_count: usize,
    pub restart_count: u32,
    pub errors: Vec<ChaosTestError>,
}

#[derive(Debug, Clone)]
pub struct RecoveryReport {
    pub workflow_count_match: bool,
    pub dag_structure_match: bool,
    pub completed_bead_match: bool,
    pub ready_bead_match: bool,
    pub inconsistencies: Vec<String>,
}
```

## Non-goals

- [NG1] Do NOT test supervisor meltdown scenarios (max restarts exceeded)
  - Rationale: Covered by separate supervisor tests
- [NG2] Do NOT test worker orphan handling during scheduler downtime
  - Rationale: Workers re-register; out of scope for scheduler recovery
- [NG3] Do NOT test distributed consensus across multiple schedulers
  - Rationale: Single scheduler design; no leader election yet
- [NG4] Do NOT test network partitions (only process crashes)
  - Rationale: Network chaos is separate concern
- [NG5] Do NOT test data loss scenarios (corrupted checkpoints/event log)
  - Rationale: Covered by replay engine tests
- [NG6] Do NOT optimize for fast recovery (focus on correctness)
  - Rationale: Performance optimization comes after correctness proven

## Dependencies

- `ractor` crate for actor framework
- `tokio` for async runtime and timers
- `crates/orchestrator` for SchedulerActor, SupervisorActorDef
- `crates/orchestrator::scheduler` for WorkflowState, SchedulerStats
- `crates/orchestrator::dag` for WorkflowDAG
- `crates/orchestrator::replay` for ReplayEngine
- `crates/orchestrator::shutdown` for CheckpointManager
- `im` crate for persistent HashMap/HashSet (structural sharing)
- `tracing` for observability
- `thiserror` for error derive

## Test Environment Requirements

1. Isolated test database (SurrealDB instance for persistence)
2. Mock EventBus for deterministic event ordering
3. Configurable SupervisorConfig (short timeouts for testing)
4. Test workflow factory with deterministic DAG generation
5. Process injection capability (SIGTERM/SIGKILL simulation via actor stop)
6. Timeout guards (fail test if recovery exceeds threshold)

## Success Criteria

Test passes if and only if:
1. Scheduler restarts successfully (no SupervisorMeltdown)
2. All workflows present before kill are present after recovery
3. DAG structures are identical (no dependency loss)
4. Completed bead sets are identical (no un-completion)
5. Ready bead sets are consistent (may differ due to in-flight events during downtime)
6. No duplicate bead executions detected
7. Recovery time is measurable and logged
