# Contract Specification: IPC Stage Update Wiring

## Context
- **Feature**: Wire stage lifecycle events from EventBus to Zellij UI for real-time display
- **Component**: IpcWorker → Zellij Frontend IPC bridge
- **Domain**: Bead orchestration UI with live stage progression symbols

### Domain Terms
- **EventBus**: In-memory pub/sub broadcasting `BeadEvent` messages
- **IpcWorker**: Actor subscribing to EventBus, forwarding via IPC broadcast channel
- **HostMessage**: IPC protocol message type (orchestrator → Zellij plugin)
- **StageLifecycleEvent**: Subset of HostMessage for stage updates (StageStarted, StageCompleted, StageFailed, etc.)
- **TaskRow**: UI state struct with `stage: Option<String>` field for display
- **StageProgressionSymbols**: Visual indicators (◐ running, ● complete, ✗ failed, ○ pending)

### Assumptions
1. EventBus already emits stage lifecycle events during bead execution
2. IpcWorker has active EventBus subscription with `EventPattern::All`
3. Zellij plugin communicates via stdin/stdout (ZellijIpcClient)
4. Bead ID maps 1:1 to TaskRow slug (bead_id == task.slug)
5. IPC transport is reliable but may fail (graceful degradation required)
6. Multiple stage events may arrive for same bead (latest event wins)
7. Events may arrive out-of-order (process in sequence received)

---

## Preconditions

### For IpcWorker::event_forwarder
- EventBus subscription must be active (`subscription.recv()` must not return Err)
- Broadcast channel must have active receivers (else events dropped silently)
- Stage events must contain valid `bead_id` (non-empty string)
- Stage events must contain valid `timestamp` (Unix epoch seconds)

### For ZellijIpcClient::recv
- stdin must be readable (not closed)
- Input must be valid JSON matching HostMessage schema
- Messages must be newline-terminated

### For TaskRow::update_from_ipc
- Task must exist in UI state (TaskRow found for given bead_id)
- Stage string must be parseable (format: "stage_name" or "stage_name: detail")

### For Renderer::render_pipeline_view
- TaskRow.stage must be updated before render call
- Stage symbols must be mapped from stage status (not hardcoded)

---

## Postconditions

### After IpcWorker::event_to_host_message
- **Return**: `Ok(HostMessage)` with stage event populated
- **Fields**: `bead_id`, `stage` (string), `timestamp` are valid (non-empty, > 0)
- **Conversion**: `StageKind` enum → string (research/plan/implement/review/validate/accept)
- **Truncation**: `feedback` field truncated to 256 chars with "..." suffix if needed

### After IpcWorker::event_forwarder
- **Broadcast**: HostMessage sent to all broadcast channel subscribers
- **No loss**: Events queued even if no receivers (channel buffer = 100)
- **Logging**: Failed conversions logged at WARN level (non-stage events skipped)

### After ZellijIpcClient::recv
- **Return**: `Ok(HostMessage)` with parsed message
- **State**: stdin buffer advanced past consumed message
- **Error**: Returns `Err` on EOF, invalid JSON, or deserialization failure

### After TaskRow::update_from_ipc
- **State**: `task.stage` updated to new value (Some(String))
- **Format**: Stage string stored as "stage_name" or "stage_name: detail"
- **Idempotent**: Calling twice with same event is safe (no-op if unchanged)

### After Renderer::render_pipeline_view
- **Output**: Stage symbols reflect TaskRow.stage (not sample data)
- **Symbols**: ◐ (running), ● (complete), ✗ (failed), ○ (pending)
- **Detail**: Stage detail shown on next line if present (e.g., "↳ collecting lcov")

---

## Invariants

### I1: Event Flow Unidirectionality
- Stage events flow ONLY: EventBus → IpcWorker → IPC Channel → Zellij Plugin
- No reverse flow (plugin never sends stage events to orchestrator)

### I2: Bead ID Mapping
- `HostMessage::Stage*.bead_id` == `TaskRow.slug` (string equality)
- No transformation or truncation of bead_id

### I3: Stage Symbol Consistency
- Stage symbols are deterministic function of task.status + task.stage
- Same (status, stage) tuple always produces same symbol

### I4: Timestamp Monotonicity (Per Bead)
- For a given bead_id, later events have later timestamps
- Out-of-order events processed in arrival order (not sorted)

### I5: Error Propagation
- All IPC errors return `Result<T, IpcError>` (no panics, no unwraps)
- Errors logged but never crash the plugin (graceful degradation)

### I6: Memory Boundedness
- Broadcast channel buffer = 100 (dropped events if plugin slow)
- TaskRow.stage = Option<String> (max 1 stage per task)

---

## Error Taxonomy

### IpcWorker Errors
```rust
pub enum IpcBridgeError {
    /// Event serialization failed (non-stage event or missing fields)
    EventSerializationFailed {
        event_type: String,
        reason: String,
    },

    /// Invalid event payload (missing required field)
    InvalidEventPayload {
        bead_id: String,
        event_type: String,
        missing_field: String,
    },

    /// Unknown stage kind string (not in research/plan/implement/review/validate/accept)
    UnknownStageKind {
        stage_name: String,
    },

    /// Attempt count overflow (u32::MAX)
    AttemptCountOverflow {
        bead_id: String,
        current_count: u32,
    },

    /// EventBus not ready (subscription failed or died)
    EventBusNotReady {
        since: std::time::Duration,
    },
}
```

### ZellijIpcClient Errors
```rust
pub enum IpcError {
    /// stdin/stdout I/O failure
    Io(String),

    /// Connection failed (TCP or pipe broken)
    ConnectionFailed(String),

    /// Transport error (bincode or JSON serialization)
    Transport(String),
}
```

### Task Update Errors
```rust
pub enum TaskUpdateError {
    /// Bead ID not found in UI state
    BeadNotFound {
        bead_id: String,
    },

    /// Invalid stage string format
    InvalidStageFormat {
        stage: String,
        reason: String,
    },

    /// Stage symbol mapping failed (unknown stage name)
    UnknownStageSymbol {
        stage_name: String,
    },
}
```

---

## Contract Signatures

### IpcWorker Event Forwarding
```rust
/// Forward EventBus events to IPC broadcast channel
///
/// # Preconditions
/// - EventBus subscription is active
/// - Broadcast channel has capacity (> 0)
///
/// # Postconditions
/// - Stage events converted to HostMessage and broadcast
/// - Non-stage events logged and skipped
/// - Conversion errors logged but don't terminate loop
///
/// # Errors
/// - Returns Err if EventBus subscription dies
pub async fn event_forwarder(
    mut subscription: EventSubscription,
    event_tx: broadcast::Sender<HostMessage>,
) -> Result<(), IpcBridgeError>
```

### BeadEvent to HostMessage Conversion
```rust
/// Convert BeadEvent to HostMessage for stage updates
///
/// # Preconditions
/// - Event is a stage lifecycle variant (StageStarted, StageCompleted, etc.)
/// - Event has valid bead_id and timestamp
///
/// # Postconditions
/// - Returns Ok(HostMessage) with stage event populated
/// - StageKind converted to string
/// - Feedback truncated to 256 chars if needed
///
/// # Errors
/// - Returns Err if event is not a stage lifecycle variant
/// - Returns Err if required field missing (bead_id, stage, timestamp)
pub fn event_to_host_message(
    event: &BeadEvent
) -> Result<HostMessage, IpcBridgeError>
```

### Zellij IPC Receive
```rust
/// Receive HostMessage from orchestrator via stdin
///
/// # Preconditions
/// - stdin is open and readable
/// - Input is newline-terminated JSON
///
/// # Postconditions
/// - Returns Ok(HostMessage) with parsed message
/// - stdin buffer advanced past consumed message
///
/// # Errors
/// - Returns Err on EOF (connection closed)
/// - Returns Err on invalid JSON
/// - Returns Err on deserialization failure
pub fn recv(&mut self) -> Result<HostMessage, IpcError>
```

### TaskRow Update from IPC
```rust
/// Update TaskRow stage field from HostMessage
///
/// # Preconditions
/// - TaskRow exists for given bead_id
/// - HostMessage is a stage lifecycle variant
///
/// # Postconditions
/// - task.stage updated to new value
/// - Format: "stage_name" or "stage_name: detail"
///
/// # Errors
/// - Returns Err if bead_id not found
/// - Returns Err if stage string format invalid
pub fn update_from_ipc(
    &mut self,
    msg: &HostMessage,
) -> Result<(), TaskUpdateError>
```

### Stage Symbol Mapping
```rust
/// Map (status, stage) tuple to stage progression symbol
///
/// # Preconditions
/// - status is valid (created/in_progress/failed/passed/integrated)
/// - stage is Some("stage_name") or None
///
/// # Postconditions
/// - Returns one of: ◐ (running), ● (complete), ✗ (failed), ○ (pending)
///
/// # Errors
/// - Returns "?" if stage_name unknown
pub fn stage_symbol_from_status(
    status: &str,
    stage: Option<&str>,
) -> char
```

---

## Non-Goals
- [ ] Implementing BeadOrchestrator (separate bead: bd-3a0a.7)
- [ ] Implementing EventBus stage lifecycle events (separate bead: bd-3a0a.2)
- [ ] Stage execution logic (already exists in AgentSlotActor)
- [ ] Task persistence (already exists in oya-pipeline)
- [ ] Zellij pane layout (already exists in zellij-frontend)
- [ ] TCP IPC transport (use existing stdin/stdout for now)
- [ ] Event replay/snapshotting (out of scope for UI wiring)
- [ ] Multi-bead orchestration DAG (separate concern)

---

## State Machine: Stage Progression

```
┌─────────┐
│ Pending │  ○ (no stage active)
└────┬────┘
     │ StageStarted
     ▼
┌─────────┐
│ Running │  ◐ (stage in progress)
└────┬────┘
     │
     ├────────────────────┐
     │                    │
     │ StageCompleted     │ StageFailed
     ▼                    ▼
┌─────────┐         ┌─────────┐
│ Next    │         │ Previous │  ✗ (failure)
│ Stage   │         │ Stage   │
└─────────┘         └─────────┘
                         │
                         │ StageReentry
                         ▼
                   ┌─────────┐
                   │ Running │  ◐ (retry)
                   └─────────┘
```

### Transitions
- **Pending → Running**: `StageStarted` event received
- **Running → Running**: Next `StageStarted` (forward progression)
- **Running → Previous**: `StageReentry` event (back to earlier stage)
- **Running → Complete**: `StageCompleted` for final stage (accept)
- **Running → Failed**: `StageFailed` or `RecursionExhausted`

---

## Test Vectors

### Valid Stage Events
```rust
// StageStarted
HostMessage::StageStarted {
    bead_id: "bd-3a0a.8".into(),
    stage: "implement".into(),
    attempt: 1,
    timestamp: 1739097600,
}

// StageCompleted
HostMessage::StageCompleted {
    bead_id: "bd-3a0a.8".into(),
    stage: "implement".into(),
    artifact_ref: Some("artifacts/contract.md".into()),
    timestamp: 1739097660,
}

// StageFailed
HostMessage::StageFailed {
    bead_id: "bd-3a0a.8".into(),
    stage: "validate".into(),
    feedback: "3 tests failed: test_foo, test_bar, test_baz".repeat(10),
    severity: "minor".into(),
    timestamp: 1739097720,
}
// Expected: feedback truncated to 256 chars with "..." suffix
```

### Invalid Stage Events
```rust
// Missing bead_id (should fail)
HostMessage::StageStarted {
    bead_id: "".into(),
    stage: "implement".into(),
    attempt: 1,
    timestamp: 1739097600,
}

// Unknown stage kind (should log warning)
HostMessage::StageStarted {
    bead_id: "bd-3a0a.8".into(),
    stage: "unknown-stage".into(),
    attempt: 1,
    timestamp: 1739097600,
}

// Zero timestamp (should fail)
HostMessage::StageStarted {
    bead_id: "bd-3a0a.8".into(),
    stage: "implement".into(),
    attempt: 1,
    timestamp: 0,
}
```

---

## Performance Constraints

### Throughput
- **Min**: 100 stage events/second (single bead execution)
- **Max**: 1000 stage events/second (parallel bead execution)

### Latency
- **Event → UI Update**: < 100ms (p95)
- **IPC Round-trip**: < 50ms (localhost)

### Memory
- **Broadcast Channel**: 100 messages × ~1KB = ~100KB
- **TaskRow Stage Field**: 1 string per task (avg 20 bytes)

---

## Security Considerations

### Input Validation
- All `bead_id` fields validated as non-empty strings
- All `stage` strings validated against known stage names
- All `timestamp` fields validated > 0

### Sandboxing
- Zellij plugin runs in WASM sandbox (no direct filesystem access)
- IPC messages limited to 8KB buffer (DoS protection)

### Data Sanitization
- `feedback` field truncated to 256 chars (prevent log injection)
- Newlines escaped in stage detail strings (prevent UI corruption)

---

## Compliance with Project Standards

### Zero Panics / Zero Unwraps
- ✅ All functions return `Result<T, Error>`
- ✅ No `.unwrap()`, `.expect()`, or `panic!()` in code paths
- ✅ Railway-oriented programming with `?` operator

### Functional Patterns
- ✅ Pure functions for event conversion (`event_to_host_message`)
- ✅ Immutable data structures (`TaskRow` updates return new instances)
- ✅ `map`, `and_then`, `filter` for data transformation

### Error Handling
- ✅ Exhaustive error enums (no `catch-all`)
- ✅ Error context preserved (bead_id, stage_name)
- ✅ Graceful degradation (UI shows last known state on IPC failure)

---

## Success Criteria

### Functional
- ✅ Stage events flow from EventBus to Zellij UI
- ✅ UI displays live stage symbols (not sample data)
- ✅ Stage details shown on next line (e.g., "↳ collecting lcov")
- ✅ Failed stages show ✗ symbol with feedback

### Non-Functional
- ✅ No panics or crashes in IPC handling
- ✅ UI remains responsive if IPC fails (graceful degradation)
- ✅ Memory bounded (channel buffer = 100)
- ✅ All tests pass (unit + integration)

---

## Exit Checklist

- [ ] All preconditions documented
- [ ] All postconditions documented
- [ ] All invariants documented
- [ ] Error taxonomy exhaustive (every failure mode covered)
- [ ] Contract signatures use `Result<T, Error>` for fallible ops
- [ ] Test vectors cover happy/error/edge cases
- [ ] Performance constraints specified
- [ ] Security considerations addressed
- [ ] Compliance with project standards verified
