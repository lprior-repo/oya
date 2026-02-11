# Contract Specification: Event Replay Error Recovery Integration

## Context

- **Feature**: Error recovery and retry logic integration into EventSourcingReplay
- **Bead ID**: src-397b
- **Module**: `crates/events/src/replay/recovery.rs`
- **Domain Terms**:
  - *Transient error*: Temporary failures that may succeed on retry (network timeouts, lock contention)
  - *Poison event*: An event that permanently fails application (corrupted data, invalid schema)
  - *Dead Letter Queue (DLQ)*: Storage for events that cannot be processed after max retries
  - *Exponential backoff*: Retry delay that doubles with each attempt: `delay = base * 2^attempt`

- **Assumptions**:
  - Event store is durable and events are not lost
  - Projection application is deterministic (same event + state = same result)
  - Transient errors are temporary and may succeed on retry
  - Permanent errors indicate data corruption or logic errors
  - DLQ events are logged for manual review
  - Replay state machine tracks progress through `ReplayState`
  - Events are applied using `apply_event` from `apply.rs`

- **Open Questions**:
  - None - all components are already defined

## Preconditions

- **RecoveryConfig**: Must have `max_retries >= 0`, `base_backoff_ms >= 0`, `max_backoff_ms >= base_backoff_ms`
- **RetryPolicy**: Must be configured with valid `RecoveryConfig`
- **Event application**: Event must be in correct order (validated by `ApplyContext`)
- **DLQ enabled**: Dead letter queue must be enabled to skip poison events
- **Logging infrastructure**: Must be available to record skipped events

## Postconditions

- **After successful retry**: Event is applied to state, progress is updated
- **After max retries exhausted**: Event is sent to DLQ (if enabled) or operation fails
- **After permanent error**: Event is immediately sent to DLQ (if enabled) or operation fails
- **After DLQ skip**: Event ID and error are logged, replay continues with next event
- **After operation failure**: Replay state transitions to `Failed`, error is propagated

## Invariants

- **Retry count**: Never exceeds `max_retries` configuration
- **Backoff delay**: Always bounded by `[base_backoff_ms, max_backoff_ms]`
- **Event ordering**: Events are applied in strict ULID order (no skipping/reordering)
- **DLQ consistency**: Events in DLQ are always poison events (exhausted retries or permanent errors)
- **State consistency**: Either all events applied successfully OR operation failed entirely
- **Progress tracking**: Only successfully applied events increment progress counter

## Error Taxonomy

```rust
pub enum ReplayError {
    /// Transient error that will be retried
    Transient {
        attempt: u32,
        next_delay: Duration,
        underlying_error: Error,
    },

    /// Permanent error (no retry)
    Permanent {
        underlying_error: Error,
        reason: String,
    },

    /// Max retries exhausted
    RetriesExhausted {
        event_id: String,
        last_error: Error,
        total_attempts: u32,
    },

    /// DLQ operation failed
    DeadLetterQueueFailed {
        event_id: String,
        error: Error,
    },

    /// Invalid recovery configuration
    InvalidConfig {
        parameter: String,
        reason: String,
    },
}
```

### Error Classifications

**Transient Errors** (retry with exponential backoff):
- `Error::Connection(Timeout)` - network timeout
- `Error::Connection(PoolExhausted)` - connection pool busy
- `Error::StoreFailed` with "timeout", "lock", "temporary" in reason
- `Error::ProjectionFailed` with "timeout", "lock" in reason

**Permanent Errors** (skip to DLQ or fail):
- `Error::Serialization` - data corruption
- `Error::InvalidEvent` - schema violation
- `Error::EventNotFound` - missing event data
- `Error::InvalidTransition` - logic error
- `Error::ChannelClosed` - channel closed
- `Error::Internal` - critical failure

**Configuration Errors** (fail immediately):
- Invalid `max_retries` (negative)
- Invalid backoff settings (negative, max < base)
- Invalid DLQ configuration

## Contract Signatures

```rust
/// Apply a single event with retry logic and DLQ support
///
/// # Railway-Oriented Programming
/// - Returns `Ok(())` if event applied successfully (after retries)
/// - Returns `Err(ReplayError::RetriesExhausted)` if max retries exceeded
/// - Returns `Err(ReplayError::Permanent)` if event is poison
/// - Returns `Err(ReplayError::DeadLetterQueueFailed)` if DLQ write fails
///
/// # Arguments
/// * `state` - Mutable state implementing EventSourcedState
/// * `event` - Event to apply
/// * `context` - ApplyContext for ordering validation
/// * `policy` - RetryPolicy configuration
/// * `dlq` - Optional DeadLetterQueue for poison events
///
/// # Preconditions
/// * Event ordering must be valid (checked by context)
/// * RetryPolicy must be valid (max_retries >= 0, backoff settings valid)
/// * DLQ must be Some if policy.enable_dlq is true
pub fn apply_event_with_recovery<S, DLQ>(
    state: &mut S,
    event: &BeadEvent,
    context: &mut ApplyContext,
    policy: &RetryPolicy,
    dlq: Option<&mut DLQ>,
) -> Result<(), ReplayError>
where
    S: EventSourcedState,
    DLQ: DeadLetterQueue;

/// Apply events with retry logic, streaming progress
///
/// # Arguments
/// * `state` - Mutable state
/// * `events` - Iterator of events to apply
/// * `context` - ApplyContext for ordering
/// * `policy` - RetryPolicy configuration
/// * `dlq` - Optional DLQ
/// * `tracker` - Progress tracker for updates
///
/// # Returns
/// * `Ok(())` if all events applied (some may be in DLQ)
/// * `Err(ReplayError)` if critical failure occurs
///
/// # Guarantees
/// * Either all events applied OR operation failed entirely
/// * Progress tracker updated only for successfully applied events
/// * DLQ contains all poison events (if enabled)
pub fn apply_events_with_recovery<S, DLQ, I>(
    state: &mut S,
    events: I,
    context: &mut ApplyContext,
    policy: &RetryPolicy,
    dlq: Option<&mut DLQ>,
    tracker: Option<&ReplayTracker>,
) -> Result<(), ReplayError>
where
    S: EventSourcedState,
    DLQ: DeadLetterQueue,
    I: Iterator<Item = BeadEvent>;

/// Dead letter queue for poison events
pub trait DeadLetterQueue {
    /// Add a poison event to the DLQ with metadata
    fn push(
        &mut self,
        event: &BeadEvent,
        error: &Error,
        attempt: u32,
        timestamp: DateTime<Utc>,
    ) -> Result<(), Error>;

    /// Iterate over all poison events
    fn iter(&self) -> Box<dyn Iterator<Item = &PoisonEvent> + '_>;

    /// Get count of poison events
    fn len(&self) -> usize;

    /// Check if DLQ is empty
    fn is_empty(&self) -> bool;
}

/// Poison event metadata
#[derive(Debug, Clone)]
pub struct PoisonEvent {
    pub event_id: String,
    pub bead_id: BeadId,
    pub error: String,
    pub attempt: u32,
    pub timestamp: DateTime<Utc>,
    pub event_data: Vec<u8>, // Serialized event for manual review
}
```

## Non-goals

- **NOT** implementing automatic DLQ reprocessing (manual review required)
- **NOT** implementing circuit breaker pattern (retry policy is sufficient)
- **NOT** implementing distributed retry coordination (single-node replay only)
- **NOT** implementing event deduplication (assumed by store)
- **NOT** implementing retry state persistence (replay restarts from checkpoint)

## Integration Points

1. **EventSourcingReplay**: Uses `apply_events_with_recovery` instead of `apply_events`
2. **Progress tracking**: `ReplayTracker` only updated on successful application
3. **State machine**: Transitions to `Failed` on critical errors
4. **Logging**: DLQ events logged at WARN level with full context
5. **Metrics**: Track retry count, DLQ size, backoff delays

## Success Criteria

- Transient errors retried with exponential backoff (100ms → 200ms → 400ms → 800ms)
- Max 3 retries before DLQ (configurable)
- Poison events sent to DLQ with full metadata
- All DLQ events logged for manual review
- Replay continues after DLQ skip (if DLQ enabled)
- Zero unwraps, zero panics
- Railway-Oriented Programming (Result types throughout)
