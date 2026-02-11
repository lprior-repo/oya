# Contract Specification: Supervisor Checkpoint on Graceful Shutdown

## Context

**Feature**: Create final checkpoint during supervisor graceful shutdown

**Domain Terms**:
- **Checkpoint**: Snapshot of scheduler state and workflow snapshots at a point in time
- **Graceful Shutdown**: Coordinated shutdown sequence that saves state before stopping
- **Supervisor**: Actor responsible for managing child actors and their lifecycle
- **Shutdown Phase**: Current stage of shutdown (Running, Initiating, SavingCheckpoints, StoppingActors, Complete)
- **ShutdownSignal**: OS signal (SIGTERM/SIGINT) or programmatic request to shut down
- **CheckpointManager**: Component that creates and persists checkpoints
- **ShutdownCoordinator**: Component that coordinates shutdown across all actors

**Assumptions**:
1. CheckpointManager is already implemented and available in supervisor scope
2. ShutdownCoordinator is already integrated with the supervisor
3. Supervisor has access to current scheduler state (children, their state, etc.)
4. Checkpoint creation is fallible (database may be unavailable)
5. Shutdown has a 30-second timeout, checkpoint creation must complete within 25 seconds
6. Multiple components may need to create checkpoints during shutdown
7. Checkpoint results are reported via a channel to ShutdownCoordinator

**Open Questions**:
1. What state should be included in the checkpoint? (children list, their args, restart counts?)
2. Should checkpoint creation be async or blocking during shutdown?
3. What happens if checkpoint creation fails? (log and continue vs. fail shutdown)
4. Should the supervisor checkpoint include child actor states or just supervisor metadata?
5. How do we handle checkpoint when CheckpointManager is not available?

## Preconditions

### System State
- Shutdown signal has been received (SIGTERM, SIGINT, or Programmatic)
- Supervisor is in `ShuttingDown` state
- CheckpointManager is available and initialized
- ShutdownCoordinator is available for reporting checkpoint results

### Component State
- Supervisor has valid `SupervisorActorState` with children HashMap
- All child references are valid (may be stopped/stopping)
- ShutdownCoordinator's checkpoint result channel is open
- At least 25 seconds remaining before shutdown timeout

### Data Validity
- `children` HashMap contains valid entries (may be empty)
- `total_restarts` is accurate
- `failure_times` vector is current
- `child_id_counter` reflects correct state

## Postconditions

### Checkpoint Creation
- CheckpointRecord is created with:
  - Unique checkpoint_id (format: `cp-{timestamp}-{sequence}`)
  - Serialized supervisor state (JSON)
  - Current event_sequence (from child_id_counter or separate sequence)
  - Optional workflow_snapshots (if available)
  - Metadata including shutdown reason

### State Changes
- Checkpoint is persisted to database via CheckpointManager
- CheckpointResult is sent to ShutdownCoordinator channel
- Supervisor continues shutdown sequence (does NOT stop on checkpoint failure)

### Invariants Maintained
- Shutdown timeout is not exceeded
- No new children are spawned during checkpoint creation
- Child actor references remain valid (not dropped)
- Supervisor state remains consistent

### Error Handling
- If checkpoint creation fails:
  - Error is logged with details
  - CheckpointResult with `success: false` is sent
  - Shutdown continues (does NOT abort)
  - Error includes component name "supervisor" and error message

## Invariants

### State Invariants
1. **No Spawning During Shutdown**: No new children are spawned once shutdown begins
2. **Checkpoint Atomicity**: Checkpoint either succeeds completely or fails cleanly
3. **Timeout Compliance**: Checkpoint creation completes within 25 seconds
4. **State Consistency**: Checkpoint reflects consistent snapshot at time of shutdown

### Shutdown Sequence Invariants
1. **Phase Ordering**: SavingCheckpoints phase before StoppingActors phase
2. **No Re-entry**: Shutdown cannot be triggered multiple times
3. **Graceful Degradation**: Checkpoint failure does not prevent shutdown

### Resource Invariants
1. **Channel Availability**: Checkpoint result channel must accept results
2. **Database Connectivity**: CheckpointManager must be able to persist
3. **Memory Safety**: No memory leaks or dropped references

## Error Taxonomy

### SupervisorCheckpointError

```rust
pub enum SupervisorCheckpointError {
    /// CheckpointManager not available in supervisor state
    CheckpointManagerUnavailable,

    /// Failed to serialize supervisor state to JSON
    SerializationFailed {
        /// Source error details
        source: String,
    },

    /// Checkpoint creation timed out (25 second limit)
    CheckpointTimeout {
        /// Duration attempted in milliseconds
        duration_ms: u64,
    },

    /// Database error during checkpoint persistence
    CheckpointPersistenceFailed {
        /// Underlying error from persistence layer
        source: PersistenceError,
    },

    /// Checkpoint result channel closed unexpectedly
    ResultChannelClosed,

    /// Invalid supervisor state for checkpoint (e.g., corrupt data)
    InvalidState {
        /// Description of invalid state
        reason: String,
    },
}
```

### Error Conditions

| Error | When it Occurs | Recovery Strategy |
|-------|----------------|-------------------|
| `CheckpointManagerUnavailable` | CheckpointManager reference is None | Log error, send failed CheckpointResult, continue shutdown |
| `SerializationFailed` | JSON serialization of state fails | Log error, send failed CheckpointResult, continue shutdown |
| `CheckpointTimeout` | Checkpoint creation exceeds 25 seconds | Log timeout, send failed CheckpointResult, force shutdown |
| `CheckpointPersistenceFailed` | Database write fails | Log error, send failed CheckpointResult, continue shutdown |
| `ResultChannelClosed` | ShutdownCoordinator dropped result channel | Log error, continue shutdown (cannot report) |
| `InvalidState` | Supervisor state is corrupt/invalid | Log error, send failed CheckpointResult, continue shutdown |

## Contract Signatures

### Core Function

```rust
impl<A: GenericSupervisableActor> SupervisorActorDef<A>
where
    A::Arguments: Clone + Send + Sync,
    A::Msg: Send,
{
    /// Create supervisor checkpoint during graceful shutdown.
    ///
    /// This function is called during the SavingCheckpoints phase of shutdown
    /// to create a final checkpoint of supervisor state before stopping children.
    ///
    /// # Errors
    ///
    /// Returns `SupervisorCheckpointError` if:
    /// - CheckpointManager is not available
    /// - State serialization fails
    /// - Checkpoint persistence fails
    /// - Timeout is exceeded
    ///
    /// Note: All errors are logged and a failed CheckpointResult is sent to
    /// the ShutdownCoordinator before returning the error. Shutdown continues
    /// regardless of checkpoint result.
    async fn create_shutdown_checkpoint(
        &self,
        state: &SupervisorActorState<A>,
        checkpoint_tx: &mpsc::Sender<CheckpointResult>,
    ) -> Result<(), SupervisorCheckpointError> {
        // Implementation...
    }
}
```

### Helper Functions

```rust
/// Serialize supervisor state to JSON format for checkpoint storage.
///
/// # Errors
///
/// Returns `SupervisorCheckpointError::SerializationFailed` if JSON
/// serialization fails.
fn serialize_supervisor_state<A>(
    state: &SupervisorActorState<A>,
) -> Result<String, SupervisorCheckpointError>
where
    A: GenericSupervisableActor,
    A::Arguments: Clone + Send + Sync,
    A::Msg: Send;

/// Send checkpoint result to shutdown coordinator.
///
/// # Errors
///
/// Returns `SupervisorCheckpointError::ResultChannelClosed` if the
/// channel is closed.
async fn report_checkpoint_result(
    checkpoint_tx: &mpsc::Sender<CheckpointResult>,
    result: CheckpointResult,
) -> Result<(), SupervisorCheckpointError>;
```

### State Structure for Checkpoint

```rust
/// Serializable snapshot of supervisor state for checkpoint.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SupervisorSnapshot {
    /// Supervisor configuration
    pub config: SupervisorConfig,
    /// Number of active children at checkpoint time
    pub active_children: usize,
    /// Total restarts performed at checkpoint time
    pub total_restarts: u32,
    /// Child information (names, restart counts, args)
    pub children: Vec<ChildSnapshot>,
    /// Failure timestamps within restart window
    pub failure_count_in_window: usize,
    /// Current child ID counter
    pub child_id_counter: u64,
    /// Time of snapshot
    pub snapshot_time: DateTime<Utc>,
    /// Shutdown reason (if applicable)
    pub shutdown_reason: Option<String>,
}

/// Snapshot of a single child for checkpoint.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChildSnapshot {
    /// Child name
    pub name: String,
    /// Number of times this child was restarted
    pub restart_count: u32,
    /// Last restart time (if any)
    pub last_restart: Option<DateTime<Utc>>,
    /// Actor arguments (JSON-serialized)
    pub args: String,
}
```

## Non-goals

**Out of Scope for This Work**:
- Implementing checkpoint restoration (separate feature)
- Checkpoint compression or optimization
- Incremental checkpoints (only full snapshots)
- Cross-supervisor coordination (each supervisor checkpoints independently)
- Checkpoint validation or verification
- Automatic checkpoint pruning (handled by CheckpointManager)
- Checkpoint during normal operation (only on shutdown)
- Checkpoint for child actor state (only supervisor metadata)
- Distributed transaction coordination
- Checkpoint migration or versioning

**Future Enhancements**:
- Incremental checkpoints (delta since last checkpoint)
- Checkpoint verification/validation
- Cross-supervisor atomic checkpoints
- Checkpoint compression
- Checkpoint encryption
