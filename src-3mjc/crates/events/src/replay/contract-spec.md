# Contract Specification

## Context
- Feature: EventSourcingReplay state application logic
- Bead ID: src-d02f
- Domain terms:
  - **Event sourcing**: Persistence pattern where state changes are stored as a sequence of events
  - **Replay**: Process of applying events to reconstruct state
  - **Deterministic**: Same events in same order must produce same state
  - **Immutable state transitions**: State updates return new state rather than mutating in-place
  - **ULID**: Universally Unique Lexicographically Sortable Identifier (time-ordered)
  - **Railway-Oriented Programming**: Error handling pattern using Result<T, E> with functional composition

- Assumptions:
  - Events are stored with ULID-based event IDs that are time-ordered
  - Each event has a timestamp for validation
  - State transitions must follow valid state machine rules
  - Events are applied sequentially (no concurrent application)
  - The system already has BeadEvent, BeadState, and related types defined

- Open questions:
  - None - the existing code provides sufficient context

## Preconditions

### For `apply_event`:
- State MUST be initialized (not in invalid/corrupted state)
- Event MUST be valid (well-formed, with required fields)
- Event MUST NOT have been previously applied to this state
- Context MUST be properly initialized and consistent with previous applications
- For state transition events: current state in state MUST match `from` field
- Event ordering MUST be valid (ULID monotonic, timestamp non-decreasing)

### For `apply_events`:
- All individual event preconditions apply
- Events slice MUST be non-empty or explicitly handled as no-op
- Events MUST be pre-sorted in application order

### For `EventSourcedState::validate_transition`:
- Bead ID MUST exist in state (for non-creation events)
- `from` state MUST match current state in the state object
- Transition MUST be valid per state machine rules

### For `EventSourcedState::apply_event`:
- Event MUST have passed validation
- Any invariants in the state implementation MUST be satisfiable

## Postconditions

### For `apply_event`:
- State MUST reflect the changes described by the event
- Context MUST track the event as applied (last_event updated)
- Return value MUST be Ok(()) on success
- Return value MUST be Err(ApplyError) with specific error variant on failure
- State MUST remain unchanged if application fails (atomicity)

### For `apply_events`:
- All events in the slice MUST be applied in order
- State MUST reflect cumulative application of all events
- Context MUST track all events as applied
- If any event fails, processing MUST stop immediately (fail-fast)
- State MUST reflect only events applied before the failure

### For `EventSourcedState::validate_transition`:
- Return Ok(()) if transition is valid
- Return Err(ApplyError::InvalidTransition) if transition violates state machine
- No state mutations occur

### For `EventSourcedState::apply_event`:
- State MUST be updated to reflect the event
- Return Ok(()) on success
- Return Err(ApplyError) if application fails

## Invariants

### State invariants:
- State version MUST increment with each applied event (if versioned)
- Bead state MUST always be a valid BeadState enum value
- Applied event sequence MUST be monotonic (ULIDs strictly increasing)
- Timestamps MUST be non-decreasing for each bead

### Context invariants:
- last_events map MUST contain exactly one entry per bead with applied events
- Event IDs in context MUST be strictly increasing for each bead
- Timestamps in context MUST be non-decreasing for each bead

### Ordering invariants:
- For any bead, event sequence MUST be reconstructable from context
- No event MAY be applied out of ULID order
- No event MAY have timestamp earlier than previous event for same bead

## Error Taxonomy

### ApplyError variants:

1. **ApplyError::OutOfOrder**
   - When: Event ULID is not greater than last applied event ULID for same bead
   - Context: Detects replay attacks, duplicate events, or sequencing errors
   - Contains: bead_id, event_id (current), expected (last event_id), actual (current event_id)

2. **ApplyError::BeadNotFound**
   - When: Event references a bead_id that doesn't exist in state
   - Context: Non-creation event for unknown bead
   - Contains: bead_id

3. **ApplyError::InvalidTransition**
   - When: State transition violates state machine rules
   - Context: Attempting invalid state change (e.g., Complete -> Pending)
   - Contains: bead_id, from (current state), to (target state)

4. **ApplyError::TimestampInconsistent**
   - When: Event timestamp is earlier than last event's timestamp for same bead
   - Context: Detects clock skew or event reordering
   - Contains: expected (min timestamp), actual (event timestamp)

5. **ApplyError::Internal**
   - When: Unexpected internal inconsistency (e.g., failed ULID parse)
   - Context: Should never occur in normal operation
   - Contains: descriptive error message

## Contract Signatures

```rust
/// Apply a single event to state with full validation
pub fn apply_event<S>(
    state: &mut S,
    event: &BeadEvent,
    context: &mut ApplyContext,
) -> ApplyResult<()>
where
    S: EventSourcedState

/// Apply a sequence of events to state
pub fn apply_events<S>(
    state: &mut S,
    events: &[BeadEvent],
    context: &mut ApplyContext,
) -> ApplyResult<()>
where
    S: EventSourcedState

/// Trait for states that can be updated from events
pub trait EventSourcedState {
    /// Validate a state transition before applying
    fn validate_transition(
        &self,
        bead_id: BeadId,
        from: BeadState,
        to: BeadState,
    ) -> ApplyResult<()>

    /// Apply an event to the state (immutable transition)
    fn apply_event(&mut self, event: &BeadEvent) -> ApplyResult<()>
}

/// Context for tracking event ordering during replay
pub struct ApplyContext {
    last_events: HashMap<BeadId, EventMetadata>
}

impl ApplyContext {
    pub fn new() -> Self
    pub fn record_applied(&mut self, bead_id: BeadId, event: &BeadEvent)
    pub fn last_event(&self, bead_id: &BeadId) -> Option<&EventMetadata>
    pub fn is_in_order(&self, event: &BeadEvent) -> ApplyResult<bool>
}

/// Metadata for tracking applied events
#[derive(Debug, Clone)]
pub struct EventMetadata {
    pub event_id: String,
    pub timestamp: DateTime<Utc>
}

/// Result type for event application
pub type ApplyResult<T> = Result<T, ApplyError>
```

## Non-goals

- NOT handling concurrent event application (single-threaded replay)
- NOT implementing event storage/retrieval (handled by loader module)
- NOT implementing checkpoint/resume logic (handled by resume module)
- NOT implementing error recovery (handled by recovery module)
- NOT providing async/event-driven replay (synchronous batch processing)
- NOT implementing optimistic concurrency control
- NOT handling event schema migration/versioning
- NOT implementing snapshot/projection building (use case specific)
