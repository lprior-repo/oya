# Contract Specification

## Context

**Feature:** Prototype Restate: Implement canonical stage workflow transitions

**Domain Terms:**
- **Run**: Aggregate root - single execution of a Bead through the pipeline
- **Stage**: Discrete step in pipeline (Contract, Tdd15, Qa, RedQueen, GptReview, ShipGate)
- **Attempt**: One try at passing a Stage (max 3 per Stage)
- **Gate**: Quality check at end of each Stage (lint, compile, test, security, etc.)
- **Restate Workflow**: Durable state machine orchestrating Run lifecycle
- **State Transition**: Movement between stages or terminal states (Pending → Running → Shipped/Failed)

**Assumptions:**
1. Restate SDK v0.8.0 is available (already in Cargo.toml)
2. Sled persistence from Stream A (prototype-sled) is complete
3. Canonical DAG is defined: Contract → Tdd15 → Qa → RedQueen → GptReview → ShipGate → Shipped
4. Retry lane: qa/red_queen/gpt_review failures route back to tdd15 (max 3 total attempts per stage)
5. OpenCode subprocess execution happens outside Restate (this is workflow orchestration only)

**Open Questions:**
None - domain types and transition rules are well-defined in ubiquitous language and architecture docs.

---

## Preconditions

### For Workflow Initialization
1. RunId must be a valid ULID (generated from persistence layer)
2. BeadId must reference an existing bead in the tracker
3. Initial Run state must be Pending

### For Stage Transition
1. Run must exist in persistence layer
2. Current stage must be completed (attempt passed)
3. Next stage must be canonical (no skipping stages)
4. Attempt number must be <= max_attempts() for the stage

### For Retry Logic
1. Stage must have failed (gate check failed)
2. Attempt number must be < max_attempts() for retry eligibility
3. Failure category must be retryable (not auth_failed, context_overflow, etc.)

### For Terminal States
1. Shipped: All stages passed, ShipGate gate checks passed
2. Failed: Max retries exceeded or non-retryable failure
3. Aborted: Manual cancellation or policy violation

---

## Postconditions

### For Successful Stage Transition
1. Run state updated to Running with new current_stage
2. StageAttempt record persisted with attempt number incremented
3. updated_at timestamp set to Utc::now()
4. Workflow state durably persisted in Restate
5. Returns Ok(()) or Ok(NextStage)

### For Retry Routing
1. Run state transitioned back to Tdd15 (retry lane entry point)
2. Attempt count for failed stage preserved
3. Failure context (category, reason) persisted
4. Retry backoff applied (exponential: 2^attempt seconds)

### For Terminal State Reached
1. Run state set to Shipped/Failed/Aborted
2. Final timestamp recorded (shipped_at, failed_at, aborted_at)
3. No further transitions possible (absorbing state)
4. Workflow completes with terminal status

### For Workflow Durability
1. All state transitions persisted to Sled before Restate acknowledgment
2. Idempotency: duplicate workflow requests return same state
3. Replay: workflow state can be reconstructed from Sled after restart

---

## Invariants

### State Machine Invariants
1. **Forward Progress**: Stage transitions are monotonic (no going back except via retry lane)
2. **Canonical Path**: Contract → Tdd15 → Qa → RedQueen → GptReview → ShipGate → Shipped
3. **Retry Bounded**: Total attempts per stage <= max_attempts() (3)
4. **Terminal Absorbing**: Shipped/Failed/Aborted states have no outgoing transitions

### Attempt Invariants
1. **Sequential**: Attempts for given (run_id, stage) are strictly increasing (1, 2, 3)
2. **Mutually Exclusive**: Only one attempt active per (run_id, stage) at a time
3. **Completion**: If attempt completed_at is Some, state must be Passed or Failed

### Retry Lane Invariants
1. **Single Entry Point**: All retries route through Tdd15 stage
2. **Context Preservation**: Failure information carried forward to retry attempt
3. **Backoff Determinism**: Same failure and attempt number always produce same backoff duration

### Persistence Invariants
1. **Event Sourcing**: All state transitions generate events in event log
2. **Referential Integrity**: All StageAttempts reference valid RunId
3. **At-Least-Once Delivery**: Workflow guarantees state persisted, may retry safely

---

## Error Taxonomy

```rust
pub enum WorkflowError {
    /// Run not found in persistence layer
    RunNotFound(String),

    /// Invalid state transition (e.g., Shipped → Running)
    InvalidTransition {
        from: String,
        to: String,
    },

    /// Attempt limit exceeded for stage
    AttemptLimitExceeded {
        stage: String,
        attempt: u32,
        max: u32,
    },

    /// Non-canonical stage transition (e.g., Contract → Qa, skipping Tdd15)
    NonCanonicalTransition {
        from: String,
        to: String,
    },

    /// Retry requested but failure is not retryable
    NonRetryableFailure {
        category: String,
        reason: String,
    },

    /// Workflow state corruption (Restate state != persistence state)
    StateCorruption {
        workflow_state: String,
        persistent_state: String,
    },

    /// Persistence layer error
    Persistence(String),

    /// Restate SDK error
    Restate(String),

    /// Context overflow (too large to pass to next stage)
    ContextOverflow {
        size_bytes: usize,
        max_bytes: usize,
    },

    /// Concurrent modification (optimistic lock failure)
    ConcurrentModification {
        expected_version: u64,
        actual_version: u64,
    },
}
```

---

## Contract Signatures

### Core Workflow API

```rust
/// Initialize a new Run workflow in Restate
///
/// Preconditions:
/// - run_id is unique ULID
/// - bead_id references existing bead
/// - initial state is Pending
///
/// Postconditions:
/// - Workflow started in Restate
/// - Run persisted to Sled
/// - Returns Ok(WorkflowId)
///
/// Errors:
/// - Persistence: Sled write failed
/// - Restate: Workflow initialization failed
pub async fn start_run_workflow(
    &self,
    run_id: &RunId,
    bead_id: &BeadId,
) -> Result<WorkflowId, WorkflowError>

/// Transition Run to next canonical stage
///
/// Preconditions:
/// - run_id exists
/// - current stage is completed
/// - next stage is canonical
/// - attempt count within limits
///
/// Postconditions:
/// - State updated to Running(next_stage)
/// - StageAttempt persisted
/// - Workflow progressed
///
/// Errors:
/// - RunNotFound: run doesn't exist
/// - InvalidTransition: current stage not completed
/// - NonCanonicalTransition: skipping stages
/// - AttemptLimitExceeded: too many retries
pub async fn advance_to_next_stage(
    &self,
    run_id: &RunId,
    completed_stage: StageName,
    stage_result: &StageResult,
) -> Result<Option<StageName>, WorkflowError>

/// Handle failed stage attempt with retry routing
///
/// Preconditions:
/// - run_id exists
/// - stage failed (gate check failed)
/// - attempt < max_attempts
/// - failure category is retryable
///
/// Postconditions:
/// - Routed back to Tdd15 (retry lane entry)
/// - Backoff timer scheduled
/// - Failure context preserved
/// - Returns Ok(RetryScheduled) with backoff duration
///
/// Errors:
/// - NonRetryableFailure: failure category cannot retry
/// - AttemptLimitExceeded: max retries reached
/// - ContextOverflow: retry context too large
pub async fn handle_stage_failure(
    &self,
    run_id: &RunId,
    failed_stage: StageName,
    attempt: u32,
    failure: &FailureCategory,
    reason: &str,
) -> Result<RetryAction, WorkflowError>

/// Complete workflow (terminal state reached)
///
/// Preconditions:
/// - run_id exists
/// - terminal_state is Shipped/Failed/Aborted
/// - all gates passed (for Shipped)
///
/// Postconditions:
/// - Run state set to terminal
/// - Final timestamp recorded
/// - Workflow completed
/// - No further transitions possible
///
/// Errors:
/// - InvalidTransition: terminal state not reachable from current state
/// - Persistence: failed to persist final state
pub async fn complete_workflow(
    &self,
    run_id: &RunId,
    terminal_state: TerminalState,
    rationale: Option<&str>,
) -> Result<(), WorkflowError>

/// Replay workflow from persisted state (idempotency)
///
/// Preconditions:
/// - run_id exists in persistence
/// - Workflow state is recoverable
///
/// Postconditions:
/// - Returns Ok(Run) with current state
/// - Idempotent: multiple calls return same state
/// - Workflow can be resumed from this state
///
/// Errors:
/// - RunNotFound: run doesn't exist
/// - StateCorruption: Restate state != persistence state
pub async fn replay_workflow(
    &self,
    run_id: &RunId,
) -> Result<Run, WorkflowError>

/// Get next stage in canonical DAG (pure function)
///
/// Preconditions:
/// - stage is valid StageName
///
/// Postconditions:
/// - Returns Some(next_stage) if not terminal
/// - Returns None if stage is ShipGate
///
/// Errors: None (pure function, total)
pub fn get_next_canonical_stage(stage: StageName) -> Option<StageName>

/// Calculate retry backoff duration (pure function)
///
/// Preconditions:
/// - attempt_number >= 1
///
/// Postconditions:
/// - Returns exponential backoff: 2^attempt seconds
/// - Bounded at 300 seconds (5 minutes max)
///
/// Errors: None (pure function, total)
pub fn calculate_backoff(attempt_number: u32) -> Duration

/// Validate stage transition is canonical (pure function)
///
/// Preconditions:
/// - from_stage and to_stage are valid StageName
///
/// Postconditions:
/// - Returns true if to_stage is canonical next step
/// - Returns false if transition skips stages or is invalid
///
/// Errors: None (pure function, total)
pub fn is_canonical_transition(from_stage: StageName, to_stage: StageName) -> bool

/// Check if failure category is retryable (pure function)
///
/// Preconditions:
/// - category is valid FailureCategory
///
/// Postconditions:
/// - Returns true for retryable failures
/// - Returns false for terminal failures (auth, context_overflow, etc.)
///
/// Errors: None (pure function, total)
pub fn is_retryable_failure(category: &FailureCategory) -> bool
```

---

## Non-goals

1. **OpenCode subprocess execution**: This is workflow orchestration only, subprocess happens elsewhere
2. **Dynamic DAG modifications**: Canonical stages are fixed, no custom pipelines in prototype
3. **Parallel stage execution**: Stages are strictly sequential in this prototype
4. **Manual override APIs**: No vibe checks or manual transitions (governed execution only)
5. **Workflow metrics/observability**: Add in production, out of scope for prototype
6. **Multi-run orchestration**: Single Run workflow only, no bead-level orchestration yet

---

## Verification Checklist

- [ ] Every error variant is used at least once in error paths
- [ ] All preconditions have corresponding validation
- [ ] All postconditions are asserted in tests
- [ ] All invariants have invariant-violation tests
- [ ] Pure functions (get_next_stage, backoff, is_canonical, is_retryable) have property tests
- [ ] Retry logic verified with deterministic state machine tests
- [ ] Idempotency verified for start_run_workflow and replay_workflow
- [ ] Durability verified: workflow survives restart
