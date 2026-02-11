# Contract Specification: IPC Worker Bead Operations

## Context
- **Feature**: Bead lifecycle operations (Start, Cancel, Retry) in IPC Worker Actor
- **File**: `crates/orchestrator/src/actors/ipc_worker.rs`
- **Domain Terms**:
  - `BeadState`: Pending, Ready, Dispatched, Assigned, Running, Completed, Failed, Cancelled
  - `IpcWorkerState`: Actor state with store, scheduler, event_bus, agent_pool
  - `ActorError`: Business logic errors (BeadNotFound, InvalidStateTransition, Internal)
  - `HostMessage`: Acknowledgment responses to Zellij plugin

- **Assumptions**:
  - Operations are synchronous (message replies via RpcReplyPort)
  - EventBus publishing is best-effort (failure doesn't block operation)
  - State transitions are validated before persistence
  - Retry operations require explicit Failed state (not auto-retry)

- **Open Questions**: None resolved by existing architecture

---

## Function: execute_start_bead

### Preconditions
1. `bead_id` must be non-empty string
2. `state.store` must be `Some` (persistence required)
3. `state.scheduler_state` may be `Some` (not required for start operation)

### Postconditions
1. If bead not found: Return `Err(ActorError::BeadNotFound(bead_id))`
2. If bead in terminal state (Completed/Failed/Cancelled): Return `Err(ActorError::InvalidStateTransition)`
3. If bead already Running: Return `Ok(HostMessage::Ack)` (idempotent)
4. On success: Bead state = Running, started_at set, return `Ok(HostMessage::Ack)`

### Invariants
1. Bead state transition must follow valid state machine
2. `started_at` timestamp must be set when transitioning to Running
3. `updated_at` timestamp must be updated on any state change
4. EventBus receives `StateChanged` event if available

### Valid State Transitions for Start
- `Pending → Running` ✓
- `Ready → Running` ✓
- `Dispatched → Running` ✓
- `Assigned → Running` ✓
- `Running → Running` ✓ (idempotent no-op)

### Invalid State Transitions for Start
- `Completed → Running` ✗ (terminal)
- `Failed → Running` ✗ (use retry instead)
- `Cancelled → Running` ✗ (terminal)

### Signature
```rust
fn execute_start_bead(
    state: &IpcWorkerState,
    bead_id: &str,
) -> Result<HostMessage, ActorError>
```

---

## Function: execute_cancel_bead

### Preconditions
1. `bead_id` must be non-empty string
2. `state.store` must be `Some`

### Postconditions
1. If bead not found: Return `Err(ActorError::BeadNotFound(bead_id))`
2. If bead already Cancelled: Return `Ok(HostMessage::Ack)` (idempotent)
3. If bead in Completed/Failed: Return `Err(ActorError::InvalidStateTransition)` (already terminal)
4. On success: Bead state = Cancelled, completed_at set, return `Ok(HostMessage::Ack)`

### Invariants
1. Cancelled beads must have `completed_at` timestamp set
2. `updated_at` must be updated
3. EventBus receives `StateChanged` event if available
4. Worker assignment should be cleared (implementation detail)

### Valid State Transitions for Cancel
- `Pending → Cancelled` ✓
- `Ready → Cancelled` ✓
- `Dispatched → Cancelled` ✓
- `Assigned → Cancelled` ✓
- `Running → Cancelled` ✓
- `Cancelled → Cancelled` ✓ (idempotent no-op)

### Invalid State Transitions for Cancel
- `Completed → Cancelled` ✗ (already terminal)
- `Failed → Cancelled` ✗ (already terminal)

### Signature
```rust
fn execute_cancel_bead(
    state: &IpcWorkerState,
    bead_id: &str,
) -> Result<HostMessage, ActorError>
```

---

## Function: execute_retry_bead

### Preconditions
1. `bead_id` must be non-empty string
2. `state.store` must be `Some`

### Postconditions
1. If bead not found: Return `Err(ActorError::BeadNotFound(bead_id))`
2. If bead not Failed: Return `Err(ActorError::InvalidStateTransition)` (only Failed can retry)
3. On success: Bead state = Ready, retry_count += 1, error_message cleared, return `Ok(HostMessage::Ack)`

### Invariants
1. `retry_count` must be incremented on each retry
2. `error_message` must be cleared (reset to None)
3. `started_at` must be cleared (reset to None)
4. `completed_at` must be cleared (reset to None)
5. `updated_at` must be updated
6. EventBus receives `StateChanged` event if available

### Valid State Transitions for Retry
- `Failed → Ready` ✓ (with retry_count increment)

### Invalid State Transitions for Retry
- `Pending → Ready` ✗ (not failed)
- `Ready → Ready` ✗ (not failed)
- `Running → Ready` ✗ (active)
- `Completed → Ready` ✗ (terminal)
- `Cancelled → Ready` ✗ (terminal)

### Signature
```rust
fn execute_retry_bead(
    state: &IpcWorkerState,
    bead_id: &str,
) -> Result<HostMessage, ActorError>
```

---

## Error Taxonomy

| Error Variant | When Returned | Recovery |
|--------------|---------------|----------|
| `ActorError::BeadNotFound` | Bead ID not in persistence | Caller may create bead |
| `ActorError::InvalidStateTransition` | State transition not allowed | Caller must check current state |
| `ActorError::Internal("Store not initialized")` | store is None | System configuration issue |
| `ActorError::Internal(...)` | Persistence layer failure | May be retryable if transient |

---

## Non-goals
- Async worker spawning (delegated to agent pool)
- Workflow scheduling (handled by SchedulerActor)
- Dependency resolution (handled by DAG module)
- Automatic retry on failure (requires explicit retry command)
