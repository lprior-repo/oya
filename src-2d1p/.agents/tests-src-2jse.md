# Martin Fowler Test Plan: Chaos Test - Kill Scheduler Mid-Execution Recovery

## Test Philosophy

All tests follow BDD (Behavior-Driven Development) principles:
- **GIVEN**: Establish preconditions (system state before action)
- **WHEN**: Perform single action under test
- **THEN**: Assert observable outcomes (external behavior, not internals)

Test names use expressive `given_<context>_when_<action>_then_<outcome>` format.

---

## Happy Path Tests

### `test_given_scheduler_with_active_workflows_when_killed_gracefully_then_recovers_with_consistent_state`

**Given**:
- SchedulerActor running under SupervisorActor
- 3 workflows registered: wf-1 (5 beads), wf-2 (3 beads), wf-3 (7 beads)
- Each workflow has mix of completed, ready, and pending beads
- CheckpointManager available
- ReplayEngine subscribed to event stream

**When**:
- Graceful shutdown initiated (SIGTERM simulation via `actor.stop()`)

**Then**:
- Supervisor detects scheduler exit within 1s
- Scheduler restarts with exponential backoff (100ms initial)
- New scheduler instance reaches Running state within 5s
- Post-recovery state snapshot matches pre-kill:
  - `workflow_count == 3`
  - `wf-1.completed_beads` identical
  - `wf-2.completed_beads` identical
  - `wf-3.completed_beads` identical
  - DAG dependencies intact
  - Ready bead count within +/- 2 (accounts for in-flight completions)
- Supervisor `total_restarts == 1`
- Recovery time logged and < 5000ms

---

### `test_given_scheduler_with_assigned_beads_when_killed_then_workers_reclaim_or_reassign_beads`

**Given**:
- Scheduler with 1 workflow (wf-1, 10 beads in diamond DAG)
- 3 beads in Assigned state (worker-1: bead-A, worker-2: bead-B, worker-3: bead-C)
- Workers still running during scheduler downtime

**When**:
- Scheduler killed (hard stop)
- Supervisor restarts scheduler
- Scheduler reaches Running state

**Then**:
- Post-recovery worker assignments either:
  - Preserved (if checkpointed before kill), OR
  - Cleared (assignments == 0)
- Workers can successfully claim ready beads
- No duplicate assignments (bead not assigned to 2 workers simultaneously)
- Workers with orphaned assignments can re-claim same beads
- Workflow continues execution after recovery

---

### `test_given_scheduler_with_large_workflow_when_killed_then_recovers_under_timeout`

**Given**:
- Scheduler with 1 large workflow (wf-1, 100 beads in linear chain)
- 47 beads completed, 1 ready, 52 pending
- Full event log available in ReplayEngine

**When**:
- Scheduler killed and restarted by supervisor

**Then**:
- Recovery completes within timeout (10s)
- All 100 beads present in workflow
- Exactly 47 beads in completed set
- Ready bead set non-empty (workflow not deadlocked)
- Scheduler responds to RPC queries
- No memory leaks (memory usage within 2x pre-kill baseline)

---

## Error Path Tests

### `test_given_supervisor_with_max_restarts_0_when_scheduler_killed_then_does_not_restart`

**Given**:
- SupervisorConfig with `max_restarts = 0`
- SchedulerActor running under supervisor
- Active workflow in progress

**When**:
- Scheduler killed

**Then**:
- Supervisor does NOT attempt restart
- Supervisor transitions to Stopped state
- Test returns `ChaosTestError::RestartFailed`
- No scheduler instance running after kill
- Workers detect scheduler unavailable (connection refused)

---

### `test_given_supervisor_meltdown_when_multiple_rapid_kills_then_supervisor_shuts_down`

**Given**:
- SupervisorConfig with `max_restarts = 2`, `restart_window_secs = 5`
- Scheduler running under supervisor

**When**:
- Scheduler killed 3 times in rapid succession (< 1s intervals)

**Then**:
- Supervisor detects meltdown (failure rate >= meltdown_threshold)
- Supervisor transitions to ShuttingDown state
- Third kill does NOT trigger restart
- Supervisor stops itself
- Test returns `ChaosTestError::SupervisorMeltdown`

---

### `test_given_checkpoint_unavailable_when_scheduler_killed_then_state_restored_from_event_log`

**Given**:
- Scheduler with active workflows
- CheckpointManager unavailable (returns error)
- ReplayEngine with full event log available

**When**:
- Scheduler killed and restarted

**Then**:
- Scheduler recovers using event log replay instead of checkpoint
- Recovery time longer (event replay overhead)
- Final state consistent with pre-kill (within event log ordering)
- Test logs recovery method used (checkpoint vs replay)

---

### `test_given_event_log_corrupted_when_scheduler_killed_then_returns_partial_state_error`

**Given**:
- Scheduler with active workflows
- ReplayEngine with corrupted event log (gaps/invalid events)

**When**:
- Scheduler killed and restarted

**Then**:
- Replay returns error
- Test returns `ChaosTestError::ReplayFailed`
- Scheduler may start in degraded state (empty workflows)
- Degraded mode logged with warning

---

## Edge Case Tests

### `test_given_scheduler_with_no_workflows_when_killed_then_recovers_with_empty_state`

**Given**:
- Scheduler running under supervisor
- No workflows registered (empty state)

**When**:
- Scheduler killed and restarted

**Then**:
- Scheduler recovers successfully
- Post-recovery state has 0 workflows
- No errors or panics
- Recovery time < 1s (minimal state)

---

### `test_given_scheduler_with_single_bead_workflow_when_killed_then_preserves_completion`

**Given**:
- Scheduler with 1 workflow containing 1 bead (wf-1: bead-A)
- bead-A is Completed

**When**:
- Scheduler killed and restarted

**Then**:
- Workflow wf-1 present after recovery
- bead-A in Completed set
- Ready bead set empty
- Workflow marked complete

---

### `test_given_scheduler_killed_during_bead_assignment_when_recovers_then_no_duplicate_assignments`

**Given**:
- Scheduler with workflow having 1 ready bead (bead-A)
- Test captures scheduler state
- Worker sends ClaimBead request
- Scheduler killed mid-assignment (after state update, before reply sent)

**When**:
- Scheduler restarted
- Same worker re-sends ClaimBead request

**Then**:
- Worker successfully claims bead-A (idempotent claim)
- bead-A NOT in ready set after claim
- bead-A in worker_assignments map
- No other worker can claim bead-A (mutual exclusion)
- No duplicate entries in assignments map

---

### `test_given_scheduler_killed_with_zero_ready_beads_when_recovers_then_resumes_on_next_completion`

**Given**:
- Workflow with linear chain: A -> B -> C
- Beads A, B completed
- Bead C assigned to worker (in-flight)
- 0 beads ready (C waiting on B, B completed but C already assigned)

**When**:
- Scheduler killed
- Worker completes bead-C during downtime
- Scheduler restarted

**Then**:
- Scheduler processes OnStateChanged event for bead-C completion
- bead-C marked completed
- Workflow marked complete
- No beads ready (workflow complete)

---

## Contract Verification Tests

### `test_precondition_scheduler_running_before_kill`

**Given**:
- Test setup calls `setup_chaos_test()`

**When**:
- Pre-kill setup complete

**Then**:
- `scheduler.get_status() == ActorStatus::Running`
- `supervisor.get_status() == ActorStatus::Running`
- At least 1 workflow registered
- At least 1 bead in each state (pending, ready, assigned, completed)

---

### `test_postcondition_scheduler_running_after_recovery`

**Given**:
- Scheduler killed
- Supervisor attempting restart

**When**:
- Recovery completes

**Then**:
- `scheduler.get_status() == ActorStatus::Running`
- `supervisor.get_status() == ActorStatus::Running`
- `supervisor.total_restarts == 1`
- Scheduler responds to RPC messages

---

### `test_invariant_workflow_count_non_decreasing`

**Given**:
- Scheduler snapshot captured pre-kill with `workflow_count = N`

**When**:
- Scheduler recovered
- New snapshot captured

**Then**:
- `post_recovery.workflow_count >= N`
- (Workflows never disappear, only added)

---

### `test_invariant_completed_bead_set_monotonic`

**Given**:
- Pre-kill snapshot with `completed_beads = C_pre`

**When**:
- Scheduler recovered
- Post-recovery snapshot with `completed_beads = C_post`

**Then**:
- `C_pre.is_subset_of(C_post)`
- (Completed beads never un-complete)
- `C_post.len() >= C_pre.len()`

---

### `test_invariant_dag_structure_immutable`

**Given**:
- Pre-kill workflow DAG structure: nodes N, edges E

**When**:
- Scheduler recovered
- Post-recovery DAG structure: nodes N', edges E'

**Then**:
- `N == N'` (exact node set match)
- `E == E'` (exact edge set match)
- Dependency graph isomorphic

---

### `test_invariant_no_ready_assigned_overlap`

**Given**:
- Post-recovery scheduler state

**When**:
- Query ready beads and assigned beads

**Then**:
- `ready_beads.intersection(assigned_beads).is_empty()`
- No bead in both sets

---

### `test_invariant_no_ready_completed_overlap`

**Given**:
- Post-recovery scheduler state

**When**:
- Query ready beads and completed beads

**Then**:
- `ready_beads.intersection(completed_beads).is_empty()`
- No bead in both sets

---

## Given-When-Then Scenarios

### Scenario 1: Graceful Shutdown with Full Workflow

**Given**:
```
Scheduler is running
Workflow wf-1 registered with diamond DAG:
  A -> B
  A -> C
  B -> D
  C -> D
Beads A, B completed
Bead C assigned to worker-1
Bead D ready
```

**When**:
```
Kill signal sent (actor.stop())
Supervisor detects exit
```

**Then**:
```
Scheduler restarts within 5s
Workflow wf-1 present
Beads A, B in completed set
Bead C either assigned (checkpointed) or cleared
Bead D in ready set
No duplicate completions
```

---

### Scenario 2: Hard Kill During Assignment

**Given**:
```
Scheduler is running
Workflow wf-1 with 1 ready bead: bead-X
Worker-1 sends ClaimBead request
```

**When**:
```
Scheduler processes ClaimBead
State update: bead-X -> assigned
Kill signal sent before reply sent
```

**Then**:
```
Scheduler restarts
Worker-1 re-sends ClaimBead (or new worker claims)
bead-X assigned successfully
No duplicate assignment
Worker receives success reply
```

---

### Scenario 3: Multiple Sequential Kills

**Given**:
```
Scheduler running under supervisor
SupervisorConfig: max_restarts=3, restart_window_secs=10
```

**When**:
```
Kill scheduler (restart #1)
Wait for recovery (backoff: 100ms)
Kill scheduler again (restart #2)
Wait for recovery (backoff: 200ms)
Kill scheduler again (restart #3)
Wait for recovery (backoff: 400ms)
```

**Then**:
```
All 3 kills result in successful recovery
supervisor.total_restarts == 3
supervisor.state == Running
No meltdown (failure rate < threshold)
Final state consistent across all restarts
```

---

### Scenario 4: Kill During Workflow Registration

**Given**:
```
Scheduler running
Client sends RegisterWorkflow for wf-new
```

**When**:
```
Scheduler partially processes registration
Kill signal sent mid-registration
```

**Then**:
```
Scheduler restarts
Either:
  a) wf-new registered (idempotent registration), OR
  b) wf-new not registered (client retries)
No partial/corrupted workflow state
No deadlock in registration
```

---

## Performance Tests

### `test_given_100_bead_workflow_when_killed_then_recovers_within_10_seconds`

**Given**:
- Scheduler with 100-bead linear workflow
- 50 beads completed, 50 pending
- Full event log

**When**:
- Scheduler killed and restarted

**Then**:
- Recovery time <= 10s
- All 100 beads present
- Exactly 50 completed

---

### `test_given_10_concurrent_workflows_when_killed_then_recovers_all_workflows`

**Given**:
- Scheduler with 10 workflows (20 beads each)
- Mixed completion states

**When**:
- Scheduler killed and restarted

**Then**:
- All 10 workflows recovered
- Total bead count = 200
- Recovery time scales linearly with workflow count

---

## Test Implementation Notes

### Test Utilities

```rust
/// Create test workflow with deterministic DAG
fn create_test_workflow(id: &str, bead_count: usize) -> WorkflowDefinition

/// Wait for actor to reach specific status with timeout
async fn await_actor_status(
    actor: &ActorRef<impl Message>,
    target: ActorStatus,
    timeout_ms: u64
) -> Result<(), ChaosTestError>

/// Compare two scheduler snapshots for consistency
fn compare_snapshots(
    pre: &SchedulerSnapshot,
    post: &SchedulerSnapshot,
    tolerance: usize // Allow small differences for in-flight events
) -> Result<RecoveryReport, ChaosTestError>
```

### Test Isolation

- Each test uses unique SurrealDB namespace
- Each test uses unique EventBus instance
- Each test uses fresh SupervisorActor instance
- Test timeout: 30s (fail fast if stuck)

### Retry Logic

- State assertions retry up to 3 times with 100ms backoff
- Handles eventual consistency (in-flight events during downtime)
- Exponential backoff for async polling

---

## Test Execution Order

1. **Setup validation**: Precondition tests
2. **Happy path**: Graceful kill, hard kill, large workflow
3. **Error path**: Max restarts, meltdown, checkpoint unavailable
4. **Edge cases**: Empty state, single bead, mid-assignment, zero ready
5. **Contract verification**: Invariants, postconditions
6. **Performance**: Large workflow, concurrent workflows

---

## Coverage Metrics

- **Total tests**: 27
- **Happy path**: 3
- **Error path**: 4
- **Edge cases**: 4
- **Contract verification**: 6
- **Scenarios**: 4
- **Performance**: 2
- **Utilities**: 4 (helper functions)

**Estimated execution time**: ~45s (parallel) / ~120s (sequential)

---

## Exit Criteria

Test suite is complete when:

1. All failure modes have corresponding error variants
2. All preconditions have validation tests
3. All postconditions have assertion tests
4. All invariants have verification tests
5. Test names unambiguously describe behavior
6. All tests use Given-When-Then structure
7. Performance tests have measurable thresholds
8. Error path tests verify specific error returns
