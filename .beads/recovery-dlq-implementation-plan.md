# Implementation Plan: EventSourcingReplay Error Recovery and Retry

**Parent Bead**: src-397b
**Session**: recovery-dlq
**Date**: 2026-02-11

## Executive Summary

Complete implementation of error recovery for EventSourcingReplay including:
1. Dead letter queue for poison events
2. Structured logging for manual review
3. Retry pipeline with exponential backoff
4. Integration into event replay loop
5. Comprehensive testing

**Current State**: `recovery.rs` has config, policy, and error classification.
**Missing**: DLQ storage, event logging, retry execution, replay integration.

---

## Task Breakdown (5 Atomic Beads)

### Task 1: Dead Letter Queue Trait and Storage
**ID**: `task-001`
**Title**: `recovery: Add dead letter queue trait and storage`
**Type**: feature
**Priority**: 0 (P0 - critical for success criteria)
**Effort**: 2hr
**File**: `crates/events/src/replay/recovery.rs`

**Success Criteria**:
- Define `DeadLetterQueue` trait with `enqueue`, `iter`, `len`, `is_empty` methods
- Define `PoisonEvent` struct with event_id, error, timestamp, attempts
- Implement `InMemoryDeadLetterQueue` using `VecDeque`
- Export trait and types from `recovery` module

**Design**:
```rust
pub trait DeadLetterQueue: Send + Sync {
    fn enqueue(&mut self, event: PoisonEvent) -> Result<()>;
    fn iter(&self) -> impl Iterator<Item = &PoisonEvent>;
    fn len(&self) -> usize;
    fn is_empty(&self) -> bool;
}

pub struct PoisonEvent {
    pub event_id: String,
    pub error: Error,
    pub timestamp: DateTime<Utc>,
    pub attempts: u32,
}

pub struct InMemoryDeadLetterQueue {
    events: VecDeque<PoisonEvent>,
}
```

**Tests**:
- Enqueue stores event with metadata
- Multiple events are stored in order
- Iterator returns all stored events
- Empty DLQ returns empty iterator
- DLQ operations return Result (never panic)

---

### Task 2: Event Logging for Poison Events
**ID**: `task-002`
**Title**: `recovery: Add event logging for skipped/poison events`
**Type**: feature
**Priority**: 0 (P0 - required by success criteria)
**Effort**: 1hr
**File**: `crates/events/src/replay/recovery.rs`

**Success Criteria**:
- Add `log_poison_event` function for DLQ events (WARN level)
- Add `log_failed_event` function for non-DLQ failures (ERROR level)
- Log structured fields: event_id, error, attempts, timestamp
- Integration point for calling in retry pipeline

**Design**:
```rust
pub fn log_poison_event(event: &PoisonEvent) {
    tracing::warn!(
        event_id = %event.event_id,
        error = %event.error,
        attempts = event.attempts,
        timestamp = %event.timestamp,
        "Event sent to dead letter queue"
    );
}

pub fn log_failed_event(event_id: &str, error: &Error, attempts: u32) {
    tracing::error!(
        event_id = %event_id,
        error = %error,
        attempts = attempts,
        "Event failed and replay stopped"
    );
}
```

**Tests**:
- DLQ event creates WARN log with all fields
- Failed event creates ERROR log with all fields
- Logs are machine-parseable (structured)
- Logging failure doesn't panic

---

### Task 3: Retry Pipeline with Railway-Oriented Programming
**ID**: `task-003`
**Title**: `recovery: Add retry pipeline with Railway-Oriented Programming`
**Type**: feature
**Priority**: 0 (P0 - core retry mechanism)
**Effort**: 2hr
**File**: `crates/events/src/replay/recovery.rs`

**Success Criteria**:
- Generic `retry_with_policy` async function
- Exponential backoff between retries (using `policy.calculate_backoff`)
- Transient error detection via `is_transient_error`
- Max retry enforcement
- Zero panics, all Results

**Design**:
```rust
pub async fn retry_with_policy<F, T, E>(
    policy: &RetryPolicy,
    operation: F,
) -> Result<T, E>
where
    F: Fn() -> Pin<Box<dyn Future<Output = Result<T, E>> + Send>>,
    E: Into<Error> + Clone,
{
    let mut attempt = 0;

    loop {
        match operation().await {
            Ok(result) => return Ok(result),
            Err(err) => {
                let error = err.into();
                match policy.should_retry(&error, attempt) {
                    RecoveryStrategy::Retry { attempt: next, delay } => {
                        tokio::time::sleep(delay).await;
                        attempt = next;
                    }
                    RecoveryStrategy::SkipToDlq => return Err(error),
                    RecoveryStrategy::Fail => return Err(error),
                }
            }
        }
    }
}
```

**Tests**:
- Success on first attempt
- Transient error retries and succeeds
- Multiple retries with backoff succeed
- Exhausted retries return last error
- Permanent error fails immediately
- Backoff delays are correct (100ms, 200ms, 400ms)
- Backoff capped at max_backoff_ms

---

### Task 4: Integration into Event Replay Loop
**ID**: `task-004`
**Title**: `recovery: Integrate error recovery into event replay loop`
**Type**: feature
**Priority**: 0 (P0 - enables DLQ in practice)
**Effort**: 2hr
**File**: `crates/events/src/replay/apply.rs`

**Success Criteria**:
- Modify `apply_events` to use retry logic
- Poison events sent to DLQ when enabled
- Replay continues after poison event (if DLQ enabled)
- Replay stops on permanent error or DLQ disabled
- State is consistent after skipping poison events

**Design**:
```rust
pub fn apply_events<S>(
    state: &mut S,
    events: &[BeadEvent],
    context: &mut ApplyContext,
    recovery_config: &RecoveryConfig,
    dlq: &mut impl DeadLetterQueue,
) -> ApplyResult<ReplaySummary>
where
    S: EventSourcedState,
{
    let policy = RetryPolicy::with_config(recovery_config.clone());
    let mut applied = 0;
    let mut skipped = 0;

    for event in events {
        let result = retry_with_policy(&policy, || {
            // Apply event logic
        }).await;

        match result {
            Ok(()) => {
                context.record_applied(bead_id, event);
                applied += 1;
            }
            Err(error) => {
                if recovery_config.enable_dlq {
                    let poison = PoisonEvent {
                        event_id: event.id.clone(),
                        error: error.clone(),
                        timestamp: Utc::now(),
                        attempts: policy.config().max_retries,
                    };
                    dlq.enqueue(poison)?;
                    log_poison_event(&poison);
                    skipped += 1;
                } else {
                    log_failed_event(&event.id, &error, policy.config().max_retries);
                    return Err(error);
                }
            }
        }
    }

    Ok(ReplaySummary { applied, skipped })
}
```

**Tests**:
- All events apply successfully
- Transient error retries and succeeds
- Poison event sent to DLQ and replay continues
- Multiple poison events captured in DLQ
- DLQ disabled stops replay on failure
- State is valid after skipping poison events
- Empty event list completes
- All events poison → all in DLQ, replay completes

---

### Task 5: End-to-End Tests for Error Recovery
**ID**: `task-005`
**Title**: `recovery: Add end-to-end tests for error recovery`
**Type**: task
**Priority**: 1 (P1 - verification)
**Effort**: 2hr
**File**: `crates/events/src/replay/recovery.rs` (tests module)

**Success Criteria**:
- Integration tests for all retry scenarios
- Integration tests for DLQ scenarios
- Tests verify zero panics
- Tests verify Result types used correctly
- Tests cover happy paths, error paths, edge cases

**Test Scenarios**:
1. **Happy Path**: All events apply successfully
2. **Retry Success**: Transient error retries and succeeds
3. **DLQ Capture**: Poison event to DLQ, replay continues
4. **Mixed Results**: Some success, some DLQ
5. **All Poison**: All events to DLQ, replay completes
6. **DLQ Disabled**: Stop replay on failure
7. **Permanent Error**: Fail immediately
8. **Edge Cases**: Empty list, single event, alternating

**Test Helpers**:
```rust
struct MockEvent {
    id: String,
    apply_result: Result<(), Error>,
}

fn create_test_events(results: Vec<Result<(), Error>>) -> Vec<MockEvent> {
    // Create test events with predetermined results
}
```

---

## Implementation Order (Dependencies)

```
task-001 (DLQ Trait)
    ↓
task-002 (Logging) - can be parallel with 003
    ↓
task-003 (Retry Pipeline) - can be parallel with 002
    ↓
task-004 (Integration) - depends on 001, 002, 003
    ↓
task-005 (E2E Tests) - depends on 004
```

**Parallelization Opportunities**:
- Tasks 002 and 003 can be done in parallel
- Task 005 can start while 004 is in progress (write tests alongside)

---

## Quality Gates

Each bead must pass:
1. **Zero panics**: No `unwrap()`, `expect()`, `panic!`, `todo!`, `unimplemented!`
2. **Railway-Oriented Programming**: All `Result<T, E>` chains with `?` operator
3. **Tests**: Unit tests for all public functions
4. **Documentation**: Doc comments on all public items
5. **Style**: Passes `moon run :quick` (fmt + clippy)

---

## Success Criteria Verification

After all 5 tasks complete:

- [x] Retry transient errors with exponential backoff ✓ (task-003, task-004)
- [x] Dead letter queue for poison events ✓ (task-001, task-004)
- [x] Log all skipped events for manual review ✓ (task-002, task-004)
- [x] Max 3 retries before DLQ ✓ (default in RecoveryConfig)
- [x] Zero unwraps, zero panics ✓ (all tasks)
- [x] Railway-Oriented Programming ✓ (task-003 design)

---

## Risk Mitigation

| Risk | Impact | Mitigation |
|------|--------|------------|
| DLQ trait design doesn't fit use cases | High | Review CheckpointStore pattern first, test early |
| Retry pipeline too generic, hard to use | Medium | Start with concrete use case, generalize if needed |
| Integration breaks existing tests | High | Run test suite before/after, fix regressions |
| Logging infrastructure missing | Low | Check dependencies first, add if needed |
| Async testing complexity | Medium | Use tokio::test, follow existing patterns |

---

## Open Questions

1. Should DLQ persist to disk or be in-memory only?
   - **Decision**: In-memory for MVP, add file persistence later
   - **Rationale**: Simpler, sufficient for initial use case

2. Should there be a DLQ size limit?
   - **Decision**: No limit for initial implementation
   - **Rationale**: Poison events should be rare, memory usage acceptable

3. Should `apply_events` return count of skipped events?
   - **Decision**: Yes, return `ReplaySummary { applied, skipped }`
   - **Rationale**: Useful for monitoring and debugging

---

## Files to Modify

- `crates/events/src/replay/recovery.rs` (add DLQ, logging, retry)
- `crates/events/src/replay/mod.rs` (re-export new types)
- `crates/events/src/replay/apply.rs` (integrate retry logic)

---

## Estimated Total Effort

- Task 1: 2hr (DLQ trait and implementation)
- Task 2: 1hr (logging)
- Task 3: 2hr (retry pipeline)
- Task 4: 2hr (integration)
- Task 5: 2hr (E2E tests)

**Total**: 9 hours across 5 atomic beads

**Parallel execution**: ~5 hours (002/003 parallel, 005 with 004)

---

## Next Steps

1. **Claim parent bead** `src-397b` if not already claimed
2. **Create child beads** using `br create` for each task
3. **Start with Task 1** (DLQ trait) - foundational
4. **Proceed in dependency order**, parallelizing where possible
5. **Update parent bead** status as tasks complete

---

## References

- Parent bead: `src-397b`
- Recovery module: `/home/lewis/src/oya/crates/events/src/replay/recovery.rs`
- Error types: `/home/lewis/src/oya/crates/events/src/error.rs`
- Contract spec: `/home/lewis/src/oya/crates/events/src/replay/contract-spec.md`
- Bead template: `/home/lewis/src/oya/.beads/BEAD_TEMPLATE.md`
