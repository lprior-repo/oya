# Contract Specification

## Context
- Feature: Event replay state machine for event sourcing system
- Domain terms:
  - **ReplayState**: Lifecycle state of event replay operation
  - **State transition**: Valid movement from one state to another
  - **Terminal state**: Final state (Complete or Failed) with no exit transitions
  - **Active state**: Intermediate state (Loading or Replaying) where work occurs
- Assumptions:
  - State transitions are validated at runtime
  - Invalid transitions return Result::Err with detailed error context
  - Failure can occur from any state (fail transition is always valid)
  - Progress tracking is maintained during Replaying state
- Open questions: None (implementation already exists and is verified)

## Preconditions
- State transitions must follow valid state machine rules:
  - Uninitialized → Loading only
  - Loading → Replaying only
  - Replaying → Complete only
  - Any state → Failed (fail transition)
- `update_progress()` can only be called from Replaying state
- `events_total` must be provided when transitioning to Replaying state
- `events_processed` must not exceed `events_total` (invariant)

## Postconditions
- Successful state transition returns new state in Ok variant
- Failed transition returns Error::InvalidState with current and attempted states
- Terminal states (Complete, Failed) cannot transition to other states
- Progress updates preserve events_total while updating events_processed
- State queries (is_terminal, is_active, description) return accurate information

## Invariants
- Uninitialized state has no associated data
- Loading state contains events_loaded count
- Replaying state contains events_processed and events_total
- Complete state contains final events_processed count
- Failed state contains error message
- events_processed <= events_total (when both are present)
- Terminal states cannot transition (except via fail which creates new Failed state)
- Active states (Loading, Replaying) are not terminal
- Only Replaying state allows progress updates

## Error Taxonomy
- **Error::InvalidState** - Raised when state transition is invalid
  - Contains current state name and attempted operation
  - Examples: Trying to start_loading from Loading state
  - Semantic meaning: "Precondition violated - cannot perform operation in current state"
- **Error::Internal** - Raised for system-level failures
  - Example: Progress channel closed unexpectedly
  - Semantic meaning: "Runtime error outside state machine logic"

## Contract Signatures

### State Creation
```rust
fn ReplayState::default() -> Self
// Postcondition: Returns Uninitialized state
```

### State Transitions
```rust
fn ReplayState::start_loading(&self) -> Result<Self, Error>
// Precondition: Current state must be Uninitialized
// Postcondition: Returns Loading { events_loaded: 0 }
// Error: InvalidState if current state is not Uninitialized

fn ReplayState::start_replaying(&self, events_total: u64) -> Result<Self, Error>
// Precondition: Current state must be Loading
// Postcondition: Returns Replaying { events_processed: 0, events_total }
// Error: InvalidState if current state is not Loading

fn ReplayState::update_progress(&self, events_processed: u64) -> Result<Self, Error>
// Precondition: Current state must be Replaying
// Precondition: events_processed <= self.events_total
// Postcondition: Returns Replaying with updated events_processed
// Error: InvalidState if current state is not Replaying

fn ReplayState::complete(&self) -> Result<Self, Error>
// Precondition: Current state must be Replaying
// Postcondition: Returns Complete { events_processed }
// Error: InvalidState if current state is not Replaying

fn ReplayState::fail(&self, error: String) -> ReplayState
// Precondition: None (can fail from any state)
// Postcondition: Returns Failed { error }
```

### State Queries
```rust
fn ReplayState::is_terminal(&self) -> bool
// Postcondition: Returns true for Complete or Failed, false otherwise

fn ReplayState::is_active(&self) -> bool
// Postcondition: Returns true for Loading or Replaying, false otherwise

fn ReplayState::description(&self) -> &str
// Postcondition: Returns human-readable state description
```

## Non-goals
- Persistence of state (state is in-memory only)
- Automatic recovery from Failed state
- State transition history/audit log
- Concurrent state mutation (not thread-safe, use external synchronization)
- State visualization/monitoring (handled by ReplayProgress/ReplayTracker)
