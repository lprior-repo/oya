# Contract Specification

## Context
- Feature: EventSourcingReplay - Implement state application logic
- Bead ID: src-d02f
- File: crates/events/src/replay/apply.rs
- Domain terms:
  - **Event sourcing**: Persistence pattern where state changes are stored as immutable events
  - **Event replay**: Reconstructing state by replaying events in order
  - **Deterministic application**: Same events + same order = same state
  - **ULID**: Universally Unique Lexicographically Sortable Identifier (time-ordered)
  - **ApplyContext**: Tracks last applied event per bead for ordering validation
  - **EventSourcedState**: Trait for types that can apply events to themselves
  - **AllBeadsState**: Projection state containing all bead projections
  - **BeadEvent**: Enum of all possible events in the system

- Assumptions:
  - Events have ULIDs that are time-ordered
  - Events have timestamps for additional ordering validation
  - State transitions must be validated before application
  - Multiple events can be applied in sequence
  - Events are stored in append-only log

- Open questions:
  - Should `AllBeadsState` directly implement `EventSourcedState`, or should we create a wrapper?
    - **Decision**: Direct implementation is simpler and more ergonomic
  - How do we handle state transition validation for all event types?
    - **Decision**: Delegate to `AllBeadsProjection` logic, add explicit transition validation
  - Should we support partial replay (from a specific event)?
    - **Decision**: Not in scope - full replay only for now
  - How do we handle events for beads that don't exist in state?
    - **Decision**: Return `BeadNotFound` error for non-bead events without prior `Created`

## Preconditions
- Event ID must be a valid ULID (parseable)
- Event timestamp must not be before the last applied event's timestamp
- For `StateChanged` events:
  - Bead must exist in state (must have `Created` event first)
  - `from` state must match current state
  - `to` state must be a valid transition from `from`
- For non-creation events:
  - Bead must exist in state
- For all events:
  - Event must be in order (ULID > last applied event's ULID)

## Postconditions
- After successful `apply_event`:
  - State reflects all changes from the event
  - `ApplyContext` records the event as the last applied for that bead
  - State counts are updated correctly
  - History is appended for state changes
  - Return `Ok(())`
- After failed `apply_event`:
  - State is unchanged (immutability preserved)
  - `ApplyContext` is unchanged
  - Return `Err(ApplyError)` with specific error variant
- After successful `apply_events`:
  - All events have been applied in order
  - State reflects all changes from all events
  - Context tracks last event for each bead
- After failed `apply_events`:
  - State reflects only events up to the failure point
  - Context reflects only events up to the failure point
  - Returns error from first failing event

## Invariants
- **Ordering invariant**: Events are applied in strict ULID order per bead
  - If `event1.event_id < event2.event_id`, then `event1` must be applied before `event2`
  - Enforced by `ApplyContext::is_in_order`
- **Idempotency invariant**: Applying the same event twice returns `OutOfOrder` error
  - Same event ID = exact duplicate, rejected
- **Consistency invariant**: State transitions are valid
  - Only allowed transitions per state machine
  - `from` must match current state
- **Determinism invariant**: Same events in same order = same state
  - No external state during application
  - No randomness in application logic
- **Immutability invariant**: State updates are functional (return new state or use interior mutability carefully)
  - `apply_event` takes `&mut S` for efficiency but should be conceptually pure
- **Counting invariant**: `state_counts` always matches actual bead states
  - Every state change updates counts atomically

## Error Taxonomy

### ApplyError variants (exhaustive)

1. **OutOfOrder** - Event sequence violation
   - When: Current event's ULID <= last applied event's ULID for same bead
   - Context: Bead ID, event ID, expected ID, actual ID
   - Recovery: Replay events in correct order from beginning
   - Severity: Error (data corruption if ignored)

2. **BeadNotFound** - Event for non-existent bead
   - When: Applying non-`Created` event to bead not in state
   - Context: Bead ID
   - Recovery: Apply `Created` event first
   - Severity: Error (missing required data)

3. **InvalidTransition** - State machine violation
   - When: Transition from `from` to `to` is not allowed
   - Context: Bead ID, from state, to state
   - Recovery: Fix event sequence or state machine definition
   - Severity: Error (logic error)

4. **TimestampInconsistent** - Time ordering violation
   - When: Event timestamp < last event timestamp
   - Context: Expected timestamp, actual timestamp
   - Recovery: Fix clock skew or event timestamps
   - Severity: Error (ordering violation)

5. **Internal** - Unexpected system error
   - When: ULID parsing fails, invariant violations
   - Context: Error message
   - Recovery: Fix bug or data corruption
   - Severity: Critical (system error)

## Contract Signatures

### Core trait
```rust
pub trait EventSourcedState {
    /// Validate a state transition before applying.
    /// Returns error if transition is invalid.
    fn validate_transition(
        &self,
        bead_id: BeadId,
        from: BeadState,
        to: BeadState,
    ) -> ApplyResult<()>;

    /// Apply an event to the state.
    /// Updates state in place for efficiency, but should be conceptually pure.
    fn apply_event(&mut self, event: &BeadEvent) -> ApplyResult<()>;
}
```

### Core functions
```rust
/// Apply a single event to state with full validation.
pub fn apply_event<S>(
    state: &mut S,
    event: &BeadEvent,
    context: &mut ApplyContext,
) -> ApplyResult<()>
where
    S: EventSourcedState;

/// Apply a sequence of events to state.
/// Stops at first error (short-circuit evaluation).
pub fn apply_events<S>(
    state: &mut S,
    events: &[BeadEvent],
    context: &mut ApplyContext,
) -> ApplyResult<()>
where
    S: EventSourcedState;
```

### Context management
```rust
impl ApplyContext {
    pub fn new() -> Self;
    pub fn record_applied(&mut self, bead_id: BeadId, event: &BeadEvent);
    pub fn last_event(&self, bead_id: &BeadId) -> Option<&EventMetadata>;
    pub fn is_in_order(&self, event: &BeadEvent) -> ApplyResult<bool>;
}
```

## State Machine Rules

### Valid state transitions for a bead:
```
Pending -> Ready
Pending -> Blocked
Pending -> Completed (if cancelled)

Ready -> Scheduled
Ready -> Blocked (if dependency fails)

Scheduled -> Running
Scheduled -> Blocked (if pre-check fails)
Scheduled -> Ready (if de-scheduled)

Running -> Completed
Running -> Failed
Running -> Blocked (if dependency fails during execution)

Blocked -> Ready (if unblocked)
Blocked -> Pending (if reset)
Blocked -> Completed (if cancelled)

Failed -> Ready (if retry allowed)
Failed -> Completed (if max retries exhausted)

Completed -> (no transitions, terminal state)
```

### Special cases:
- Direct transitions allowed for administrative actions
- Bulk state updates skip validation (use with caution)
- Recursion exhausted auto-transitions to Completed

## Non-goals
- Partial replay from specific event ID (future enhancement)
- Event filtering/skip logic during replay (future enhancement)
- Concurrent event application (single-threaded only)
- Event versioning/migration (all events same version)
- Performance optimization beyond correctness (profile first)
- State snapshot/checkpointing (use full replay)
- Cross-bead transaction semantics (each bead independent)
- Event deduplication (duplicate ULIDs = error)
