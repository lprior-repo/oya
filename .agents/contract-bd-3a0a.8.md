# Contract Specification: Stage Lifecycle IPC Integration

## Context

**Feature:** Wire stage updates from EventBus to Zellij UI through IPC bridge

**Bead ID:** bd-3a0a.8

**Domain Terms:**
- **EventBus**: Pub/sub system for bead lifecycle events (StageStarted, StageCompleted, StageFailed, StageReentry, ValidationRan, RecursionExhausted)
- **IpcWorker**: Actor-based bridge between orchestrator and Zellij plugin
- **IpcTransport**: Length-prefixed bincode framing over TCP
- **Stage Lifecycle**: Pipeline phases (Plan, Implement, Validate, Review, Accept)
- **HostMessage**: Orchestrator → Plugin messages (events, responses)
- **GuestMessage**: Plugin → Orchestrator messages (queries, commands)

**Assumptions:**
1. EventBus already emits stage lifecycle events (bd-3a0a.2 completed)
2. IpcWorker exists but only forwards events without type-specific handling
3. Zellij plugin currently displays sample/static data
4. IPC transport layer (oya-ipc) is stable
5. Events crate already has stage lifecycle variants with constructors

**Open Questions:**
1. Should stage updates include progress percentages, or just state transitions?
   - **Decision:** State transitions only (progress is phase-level, not stage-level)
2. How should UI display recursion/reentry attempts?
   - **Decision:** Include attempt count in stage events
3. Should we batch stage updates or send immediately?
   - **Decision:** Send immediately (real-time feedback is critical)
4. What happens if UI disconnects during stage execution?
   - **Decision:** IpcWorker buffers last 100 events per bead, replays on reconnect

**Dependencies:**
- bd-3a0a.7 (BeadOrchestrator slot scheduler loop) - must be complete
- bd-3a0a.2 (stage lifecycle event variants) - must be complete
- bd-3a0a (epic: Recursive DAG Orchestration)

## Preconditions

### For IpcWorker Event Forwarding
- EventBus must be initialized and running
- At least one Zellij plugin client must be connected
- Stage lifecycle events must have valid bead_id and StageKind

### For IPC Message Extension
- HostMessage enum must be extensible (tagged enum)
- oya-ipc crate must support new message types
- Serialization must not exceed 1KB per message

### For UI Rendering
- Plugin must have active IPC connection to orchestrator
- Task/bead data must be loaded (not nil)
- Stage symbols must be defined for each StageKind

## Postconditions

### After StageStarted Event Received
- UI displays stage symbol (e.g., "🔄" for Implement) next to bead
- Stage timestamp is recorded and displayed
- Attempt count is visible if > 1

### After StageCompleted Event Received
- UI displays completion symbol (e.g., "✓" for Implement)
- Stage is marked as completed in bead detail view
- Artifact reference (if any) is accessible

### After StageFailed Event Received
- UI displays failure symbol (e.g., "✗" for Validate)
- Failure reason is shown in bead detail
- Severity level is color-coded (Major, Critical, Fundamental)

### After StageReentry Event Received
- UI displays reentry symbol (e.g., "↩" with from/to stages)
- Previous stage is marked as "reverted"
- New stage shows incremented attempt count

### After ValidationRan Event Received
- UI displays checkmark or X based on `passed` field
- Command output is viewable in detail pane
- Exit code is displayed

### After RecursionExhausted Event Received
- UI displays blocked symbol (e.g., "🚫")
- Total attempts and last failed stage are shown
- Bead is marked as non-retryable

## Invariants

1. **Event Ordering:** Stage events for a single bead are received in causal order
   - StageStarted → StageCompleted OR StageFailed OR StageReentry
   - StageReentry → StageStarted (new stage)
   - No StageCompleted without preceding StageStarted

2. **Attempt Count Monotonicity:** Attempt number never decreases within a bead's lifecycle
   - Each StageReentry increments attempt counter
   - Attempt count resets only on new BeadCreated event

3. **State Consistency:** UI state always reflects the latest received event
   - No stale stage symbols displayed
   - No missing events in the sequence (gap detection triggers reconnect)

4. **Message Size:** All HostMessage variants serialize to < 1KB
   - Stage symbols are single characters
   - Error messages are truncated at 256 chars

5. **Bead Existence:** All stage events reference valid bead IDs
   - BeadCreated event precedes any stage events
   - Unknown bead IDs are logged and dropped

## Error Taxonomy

### IpcBridgeError
```rust
pub enum IpcBridgeError {
    /// EventBus not available or not started
    EventBusNotReady {
        since: std::time::Duration,
    },

    /// Failed to subscribe to EventBus pattern
    SubscriptionFailed {
        pattern: EventPattern,
        reason: String,
    },

    /// Event broadcast channel overflow
    BroadcastOverflow {
        capacity: usize,
        dropped_events: u32,
    },

    /// Failed to serialize BeadEvent to HostMessage
    EventSerializationFailed {
        event_type: String,
        reason: String,
    },

    /// Invalid stage event data (missing required fields)
    InvalidEventPayload {
        bead_id: BeadId,
        event_type: String,
        missing_field: String,
    },

    /// IPC client disconnected while sending stage update
    ClientDisconnected {
        client_id: String,
        pending_updates: usize,
    },

    /// Bead ID not found in local cache
    BeadNotFound {
        bead_id: BeadId,
    },

    /// Stage kind not recognized (variant mismatch)
    UnknownStageKind {
        stage_name: String,
    },

    /// Attempt count overflow (> u32::MAX)
    AttemptCountOverflow {
        bead_id: BeadId,
        current_count: u32,
    },
}
```

### UiRenderingError
```rust
pub enum UiRenderingError {
    /// No active IPC connection to orchestrator
    NoOrchestratorConnection,

    /// Stage symbol not defined for StageKind
    SymbolNotDefined {
        stage_kind: StageKind,
    },

    /// Failed to update task row state
    TaskUpdateFailed {
        task_index: usize,
        reason: String,
    },

    /// Invalid state transition (e.g., Completed → Started)
    InvalidTransition {
        from: StageState,
        to: StageState,
        bead_id: BeadId,
    },

    /// Rendering buffer overflow (too many stage symbols)
    BufferOverflow {
        max_symbols: usize,
        actual_symbols: usize,
    },
}
```

## Contract Signatures

### IpcWorker Extensions
```rust
impl IpcWorkerActorDef {
    /// Subscribe to stage lifecycle events from EventBus
    /// Returns subscription ID for cleanup
    pub async fn subscribe_to_stage_events(
        &self,
        event_bus: Arc<EventBus>,
    ) -> Result<String, IpcBridgeError>;

    /// Convert BeadEvent to HostMessage for stage updates
    pub fn event_to_host_message(
        &self,
        event: &BeadEvent,
    ) -> Result<HostMessage, IpcBridgeError>;

    /// Broadcast stage update to all connected UI clients
    pub async fn broadcast_stage_update(
        &self,
        message: HostMessage,
    ) -> Result<(), IpcBridgeError>;
}

/// Event forwarder task (background)
pub async fn stage_event_forwarder(
    mut subscription: EventSubscription,
    event_tx: broadcast::Sender<HostMessage>,
    event_bus: Arc<EventBus>,
) -> Result<(), IpcBridgeError>;
```

### HostMessage Extensions
```rust
pub enum HostMessage {
    // ... existing variants ...

    /// Stage started for a bead
    StageStarted {
        bead_id: String,
        stage: String,  // "plan", "implement", "validate", "review", "accept"
        attempt: u32,
        timestamp: u64,
    },

    /// Stage completed successfully
    StageCompleted {
        bead_id: String,
        stage: String,
        artifact_ref: Option<String>,
        timestamp: u64,
    },

    /// Stage failed with feedback
    StageFailed {
        bead_id: String,
        stage: String,
        feedback: String,
        severity: String,  // "minor", "major", "fundamental"
        timestamp: u64,
    },

    /// Bead reentered earlier stage
    StageReentry {
        bead_id: String,
        from_stage: String,
        to_stage: String,
        reason: String,
        attempt: u32,
        timestamp: u64,
    },

    /// Validation command executed
    ValidationRan {
        bead_id: String,
        passed: bool,
        output: String,
        command: String,
        exit_code: i32,
        timestamp: u64,
    },

    /// Recursion limits exhausted
    RecursionExhausted {
        bead_id: String,
        total_attempts: u32,
        last_stage: String,
        timestamp: u64,
    },
}
```

### Plugin/Renderer Extensions
```rust
impl OyaPlugin {
    /// Handle incoming stage update from orchestrator
    pub fn handle_stage_update(
        &mut self,
        message: HostMessage,
    ) -> Result<(), PluginError>;

    /// Update task row with stage symbol
    fn update_task_stage_symbol(
        &mut self,
        bead_id: &str,
        stage: StageKind,
        state: StageState,
    ) -> Result<(), PluginError>;

    /// Get display symbol for stage state
    fn stage_symbol(stage: StageKind, state: StageState) -> &'static str;
}

impl TaskRow {
    /// Update with stage lifecycle event
    pub fn apply_stage_event(
        &mut self,
        event: &BeadEvent,
    ) -> Result<(), UiRenderingError>;

    /// Get current stage display string
    pub fn stage_display(&self) -> String;
}
```

## Non-goals

1. **Progress Tracking:** This bead does NOT add phase-level progress (0-100%)
   - Progress events already exist in PhaseProgress variant
   - Stage updates are binary state transitions only

2. **Event Filtering:** This bead does NOT implement complex subscription patterns
   - Simple All/ByBead patterns are sufficient
   - UI can filter client-side if needed

3. **Persistent Event Log:** This bead does NOT persist stage events to disk
   - EventBus already has persistence layer
   - UI displays current state, not history

4. **Multi-Orchestrator Support:** This bead assumes single orchestrator instance
   - Future bead may add load balancing/failover

5. **Stage Duration Metrics:** This bead does NOT track stage timing
   - Events have timestamps but UI doesn't calculate duration
   - Metrics dashboard is separate concern
