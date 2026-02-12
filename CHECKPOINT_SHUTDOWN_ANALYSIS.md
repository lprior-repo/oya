# Checkpoint Manager & Shutdown System Analysis

## Executive Summary

The codebase has a well-structured graceful shutdown system (`shutdown.rs`) that already supports checkpoint creation during shutdown via a broadcast channel pattern. The `CheckpointManager` exists in the replay/recovery layer, and supervisor-specific checkpoint functionality is in `actors/supervisor/checkpoint.rs`. This document details the architecture and identifies integration points needed for checkpoint creation on shutdown.

---

## 1. CheckpointManager - Definition & Implementation

**Location**: `/home/lewis/src/oya/crates/orchestrator/src/replay/checkpoint.rs`

### Core Structure
```rust
pub struct CheckpointManager {
    store: OrchestratorStore,
    config: CheckpointConfig,
    current_sequence: u64,
    shutdown_tx: Option<mpsc::Sender<()>>,
}

pub struct CheckpointConfig {
    pub interval: Duration,              // Default: 5 minutes
    pub max_checkpoints: usize,          // Default: 10
    pub auto_checkpoint: bool,           // Default: true
}
```

### Key Methods

| Method | Purpose | Returns |
|--------|---------|---------|
| `create_checkpoint(scheduler_state, workflow_snapshots)` | Creates a checkpoint at current event sequence | `PersistenceResult<CheckpointRecord>` |
| `get_latest()` | Retrieves the most recent checkpoint | `PersistenceResult<CheckpointRecord>` |
| `get_checkpoint(checkpoint_id)` | Gets a specific checkpoint by ID | `PersistenceResult<CheckpointRecord>` |
| `increment_sequence()` | Increments the event sequence counter | `()` |
| `set_sequence(sequence)` | Sets explicit sequence number | `()` |
| `current_sequence()` | Gets current event sequence | `u64` |
| `start_periodic()` | Starts periodic checkpoint task | `Option<mpsc::Receiver<()>>` |
| `stop_periodic()` | Stops the periodic task | `()` |
| `run_periodic_loop()` | Background task for periodic checkpoints | Static async fn |

### State Recovery Methods
- `restore_scheduler_state<T>()` - Restores scheduler state from latest checkpoint
- `restore_scheduler_state_by_id<T>(checkpoint_id)` - From specific checkpoint
- `restore_workflow_snapshots<T>()` - Gets workflow snapshots from latest
- `restore_workflow_snapshots_by_id<T>(checkpoint_id)` - From specific checkpoint

---

## 2. Shutdown Signal Handling

**Location**: `/home/lewis/src/oya/crates/orchestrator/src/shutdown.rs`

### ShutdownCoordinator Architecture

The `ShutdownCoordinator` manages graceful shutdown with three main components:

#### Signal Types
```rust
pub enum ShutdownSignal {
    Sigterm,      // SIGTERM received
    Sigint,       // SIGINT (Ctrl+C)
    Programmatic, // Direct API call
}
```

#### Shutdown Phases
```rust
pub enum ShutdownPhase {
    Running,              // Normal operation
    Initiating,           // Signal received
    SavingCheckpoints,    // Phase 1
    StoppingActors,       // Phase 2
    Complete,             // Done
}
```

#### Core Components
```rust
pub struct ShutdownCoordinator {
    phase: Arc<RwLock<ShutdownPhase>>,
    shutdown_initiated: Arc<AtomicBool>,
    shutdown_tx: broadcast::Sender<ShutdownSignal>,
    checkpoint_tx: mpsc::Sender<CheckpointResult>,      // ← FOR CHECKPOINT RESULTS
    checkpoint_rx: Arc<RwLock<mpsc::Receiver<CheckpointResult>>>,
}

pub struct CheckpointResult {
    pub component: String,
    pub success: bool,
    pub duration_ms: u64,
    pub error: Option<String>,
}
```

### Signal Handling Flow

**OS Signal Installation** (`install_signal_handlers`)
```
SIGTERM/SIGINT → Signal Handler Task
                 ↓
                 coordinator.initiate_shutdown(signal)
                 ↓
                 ShutdownPhase::Initiating
                 ↓
                 Broadcast to all subscribers
```

### Graceful Shutdown Sequence
```
1. initiate_shutdown(signal)
   - Sets shutdown_initiated flag
   - Broadcasts signal to subscribers
   - Moves to Initiating phase

2. shutdown() - Executes sequence
   ├─ Phase 1: SavingCheckpoints
   │  └─ save_checkpoints() - Collects results via mpsc channel
   │
   ├─ Phase 2: StoppingActors
   │  └─ stop_actors() - Future: coordinate actor shutdown
   │
   └─ Phase 3: Complete
      └─ Returns ShutdownStats
```

### Checkpoint Integration Points

**Checkpoint Channel**:
- Components call `coordinator.checkpoint_sender()` to get `mpsc::Sender<CheckpointResult>`
- Send results during checkpoint creation
- `ShutdownCoordinator` collects results with **25-second timeout**
- Results tracked in `ShutdownStats`

```rust
pub struct ShutdownStats {
    pub checkpoints_saved: usize,
    pub checkpoints_failed: usize,
    pub checkpoint_error: Option<String>,
    pub total_duration_ms: u64,
}
```

### Timeouts
- **Checkpoint phase**: 25 seconds (leaves 5s buffer for actor shutdown)
- **Total shutdown**: 30 seconds
- Both use `tokio::time::timeout()`

---

## 3. Current Flow in supervisor.rs

**Location**: `/home/lewis/src/oya/crates/orchestrator/src/actors/supervisor/supervisor_actor.rs`

### Supervisor Integration Points

```rust
pub struct SupervisorActorState<A> {
    config: SupervisorConfig,
    state: SupervisorState,
    children: HashMap<String, ChildInfo>,
    total_restarts: u32,
    failure_times: Vec<Instant>,
    child_id_counter: u64,
    shutdown_coordinator: Option<Arc<ShutdownCoordinator>>,    // ← Can store ref
    _shutdown_rx: Option<broadcast::Receiver<ShutdownSignal>>,
    restart_strategy: Box<dyn RestartStrategy<A>>,
    checkpoint_manager: Option<CheckpointManager>,              // ← Can store ref
    replay_engine: Option<ReplayEngine>,
}
```

### What's Needed
The supervisor actor currently:
- ✅ Has fields for `shutdown_coordinator` and `checkpoint_manager`
- ❌ Does NOT currently subscribe to shutdown signals
- ❌ Does NOT currently create checkpoints on shutdown

---

## 4. Graceful Shutdown Checkpoint Functionality

**Location**: `/home/lewis/src/oya/crates/orchestrator/src/actors/supervisor/checkpoint.rs`

### SupervisorCheckpointError Types
```rust
pub enum SupervisorCheckpointError {
    CheckpointManagerUnavailable,
    SerializationFailed { reason: String },
    CheckpointTimeout { duration_ms: u64 },
    CheckpointPersistenceFailed { source: PersistenceError },
    ResultChannelClosed,
    InvalidState { reason: String },
}
```

### Snapshot Data Structures
```rust
pub struct SupervisorSnapshot {
    pub config: SupervisorConfig,
    pub active_children: usize,
    pub total_restarts: u32,
    pub children: Vec<ChildSnapshot>,
    pub failure_count_in_window: usize,
    pub child_id_counter: u64,
    pub snapshot_time: DateTime<Utc>,
    pub shutdown_reason: Option<String>,
}

pub struct ChildSnapshot {
    pub name: String,
    pub restart_count: u32,
    pub last_restart: Option<DateTime<Utc>>,
    pub args: String,
}
```

### Checkpoint Creation Method

**Signature**:
```rust
pub async fn create_shutdown_checkpoint(
    &self,
    checkpoint_manager: Option<&mut CheckpointManager>,
    checkpoint_tx: &mpsc::Sender<CheckpointResult>,
) -> Result<(), SupervisorCheckpointError>
```

**Flow**:
1. Validates `CheckpointManager` availability
2. Serializes supervisor state via `serialize_supervisor_state()`
3. Creates checkpoint with **25-second timeout**
4. Sends `CheckpointResult` via `checkpoint_tx`
5. Reports all results regardless of success/failure

**Error Handling**: All errors are logged and reported via `CheckpointResult`, allowing shutdown to continue gracefully.

### Helper Functions
- `serialize_supervisor_state<A>(state)` - JSON serialization
- `build_snapshot<A>(state)` - State → SupervisorSnapshot
- `build_child_snapshots<A>(state)` - Per-child snapshots
- `report_checkpoint_result()` - Channel reporting

---

## 5. Persistence Layer - Checkpoint Storage

**Location**: `/home/lewis/src/oya/crates/orchestrator/src/persistence/checkpoint_store.rs`

### CheckpointRecord Structure
```rust
pub struct CheckpointRecord {
    pub record_id: Option<RecordId>,         // SurrealDB ID
    pub checkpoint_id: String,               // User-visible ID
    pub scheduler_state: String,             // JSON state
    pub event_sequence: u64,                 // Event counter
    pub created_at: DateTime<Utc>,           // Timestamp
    pub workflow_snapshots: Option<String>,  // Optional JSON
    pub metadata: Option<serde_json::Value>, // Custom metadata
}
```

### Storage Operations

| Operation | Method | Notes |
|-----------|--------|-------|
| Create | `save_checkpoint(record)` | Uses `checkpoint_id` as unique key |
| Read Latest | `get_latest_checkpoint()` | Orders by `event_sequence DESC` |
| Read by ID | `get_checkpoint(checkpoint_id)` | Direct lookup |
| Read by Sequence | `get_checkpoint_by_sequence(seq)` | Exact match |
| List | `list_checkpoints(limit)` | DESC by sequence |
| Prune | `prune_checkpoints(keep_count)` | Keeps N newest, deletes rest |
| Delete | `delete_checkpoint(checkpoint_id)` | Single deletion |

**Backend**: SurrealDB (in-memory or persistent)

---

## 6. Existing Test Patterns

### Checkpoint Creation Tests

**Location**: `crates/orchestrator/src/replay/checkpoint.rs` (lines 378-441)

```rust
#[tokio::test]
async fn test_create_checkpoint() {
    let mut manager = require_manager!(setup_manager().await);
    
    let result = manager
        .create_checkpoint(r#"{"state":"active"}"#, None)
        .await;
    
    assert!(result.is_ok(), "checkpoint creation should succeed");
    if let Ok(cp) = result {
        assert_eq!(cp.event_sequence, 0);
    }
}

#[tokio::test]
async fn test_checkpoint_with_snapshots() {
    let mut manager = require_manager!(setup_manager().await);
    
    let result = manager
        .create_checkpoint(
            r#"{"state":"active"}"#,
            Some(r#"{"wf-1":{"beads":["a","b"]}}"#),
        )
        .await;
    
    assert!(result.is_ok());
    if let Ok(cp) = result {
        assert!(cp.workflow_snapshots.is_some());
    }
}
```

### Checkpoint Restoration Tests

**Location**: `crates/orchestrator/src/replay/engine.rs` (lines 354-387)

```rust
#[tokio::test]
async fn test_recovery_from_checkpoint() {
    let checkpoint = CheckpointRecord::new("cp-resume", "{}", engine.current_sequence());
    let saved = store.save_checkpoint(&checkpoint).await;
    assert!(saved.is_ok(), "checkpoint save should succeed");
    
    let recovery = engine.recover_from_persistence().await;
    assert!(recovery.is_ok());
    assert_eq!(recovery.ok()?.checkpoint_id, Some("cp-resume".to_string()));
}
```

### Persistence Layer Tests

**Location**: `crates/orchestrator/src/persistence/checkpoint_store.rs` (lines 307-449)

```rust
#[tokio::test]
async fn test_save_and_get_checkpoint() {
    let store = require_store!(setup_store().await);
    
    let record = CheckpointRecord::new("cp-001", r#"{"workflows":{}}"#, 100);
    let saved = store.save_checkpoint(&record).await;
    assert!(saved.is_ok());
    
    let retrieved = store.get_checkpoint("cp-001").await;
    assert!(retrieved.is_ok());
}

#[tokio::test]
async fn test_prune_checkpoints() {
    let store = require_store!(setup_store().await);
    
    // Create multiple checkpoints
    for i in 0..5 {
        let cp = CheckpointRecord::new(format!("cp-{i}"), "{}", i as u64);
        let _ = store.save_checkpoint(&cp).await;
    }
    
    // Prune to keep only 2
    let deleted = store.prune_checkpoints(2).await;
    assert!(deleted.is_ok());
    assert_eq!(deleted.ok(), Some(3));
}
```

### Attack/Adversarial Tests

**Location**: `crates/orchestrator/src/replay/checkpoint_attack_tests/mod.rs`

Pattern for comprehensive testing:
- Happy path verification (basic + with snapshots)
- Input boundary attacks (empty IDs, malformed IDs)
- Corrupted data handling (invalid JSON, type mismatches)
- Edge cases (non-existent checkpoints, channel closure)
- Concurrent access patterns

---

## 7. Integration Requirements

### What Needs to Be Added

#### A. Supervisor Shutdown Signal Subscription

**In SupervisorActorState**:
```rust
// Currently exists but unused:
pub shutdown_coordinator: Option<Arc<ShutdownCoordinator>>,
pub _shutdown_rx: Option<broadcast::Receiver<ShutdownSignal>>,

// Needs to be:
// 1. Subscribed to in supervisor initialization
// 2. Listened to in supervisor message loop
// 3. Used to trigger create_shutdown_checkpoint() when ShutdownSignal received
```

#### B. Checkpoint Manager Integration

**In Supervisor Actor Message Loop**:
- When `ShutdownSignal` received, trigger checkpoint creation
- Use stored `checkpoint_manager` reference
- Get `checkpoint_tx` from `shutdown_coordinator`
- Call `create_shutdown_checkpoint()` on supervisor state

#### C. Required Flow

```
ShutdownSignal → Supervisor receives via subscription
                 ↓
                 Extract checkpoint_manager & checkpoint_tx
                 ↓
                 Call create_shutdown_checkpoint()
                 ↓
                 Serializes supervisor state
                 ↓
                 Creates checkpoint via CheckpointManager
                 ↓
                 Reports CheckpointResult to shutdown_tx
                 ↓
                 ShutdownCoordinator collects results
                 ↓
                 Included in ShutdownStats
```

#### D. Shutdown Coordinator Usage in Main

**Pattern Already Exists**:
```rust
let shutdown_coordinator = Arc::new(ShutdownCoordinator::new());

// Install OS signal handlers
let _signal_handler = install_signal_handlers(shutdown_coordinator.clone()).await?;

// When shutdown needed:
let stats = shutdown_coordinator.shutdown().await?;
println!("Shutdown stats: {:?}", stats);
```

---

## 8. Summary Table

| Component | Location | Status | Purpose |
|-----------|----------|--------|---------|
| **CheckpointManager** | `replay/checkpoint.rs` | ✅ Implemented | Create, retrieve, restore checkpoints |
| **ShutdownCoordinator** | `shutdown.rs` | ✅ Implemented | Orchestrate graceful shutdown |
| **SupervisorCheckpointError** | `actors/supervisor/checkpoint.rs` | ✅ Implemented | Type-safe error handling |
| **SupervisorSnapshot** | `actors/supervisor/checkpoint.rs` | ✅ Implemented | Serializable supervisor state |
| **create_shutdown_checkpoint()** | `actors/supervisor/checkpoint.rs` | ✅ Implemented | Create checkpoint during shutdown |
| **CheckpointRecord** | `persistence/checkpoint_store.rs` | ✅ Implemented | Database record type |
| **OrchestratorStore** | `persistence/checkpoint_store.rs` | ✅ Implemented | Storage CRUD operations |
| **Signal Handlers** | `shutdown.rs` | ✅ Implemented | OS signal → ShutdownSignal |
| **Supervisor Signal Subscription** | `actors/supervisor/supervisor_actor.rs` | ❌ Missing | Subscribe to ShutdownSignal |
| **Supervisor Checkpoint Trigger** | `actors/supervisor/supervisor_actor.rs` | ❌ Missing | Call create_shutdown_checkpoint() |

---

## 9. Key Design Patterns

### 1. Broadcast Pattern for Signal Coordination
- `ShutdownCoordinator` broadcasts `ShutdownSignal` to all subscribers
- Components subscribe with `coordinator.subscribe()`
- Multiple components can react independently

### 2. Channel Pattern for Result Collection
- Components send `CheckpointResult` via `checkpoint_tx` from coordinator
- `ShutdownCoordinator` collects all results with timeout
- Results aggregated in `ShutdownStats`

### 3. Error Handling Strategy
- All checkpoint errors reported via `CheckpointResult`
- Shutdown continues regardless of checkpoint success/failure
- Non-fatal errors logged but don't block shutdown progression

### 4. Timeout-Based Coordination
- Checkpoint phase: 25 seconds (graceful degradation on timeout)
- Total shutdown: 30 seconds (forcing exit if exceeded)
- Timeouts return empty results rather than errors

### 5. State Serialization
- Supervisor state → JSON via serde
- Stored in `checkpoint.scheduler_state`
- Deserialized on recovery for type-safe access

---

## 10. File Quick Reference

```
crates/orchestrator/src/
├── shutdown.rs                              ← Shutdown coordination
├── replay/
│   ├── checkpoint.rs                        ← CheckpointManager
│   ├── engine.rs                            ← Recovery from checkpoint
│   └── checkpoint_attack_tests/mod.rs       ← Adversarial tests
├── persistence/
│   └── checkpoint_store.rs                  ← Storage CRUD
└── actors/supervisor/
    ├── supervisor_actor.rs                  ← Supervisor state (needs integration)
    └── checkpoint.rs                        ← Supervisor checkpoint creation
```

---

## 11. Next Steps for Implementation

1. **Subscribe to Signals** - In supervisor actor initialization, subscribe to `shutdown_coordinator`
2. **Listen in Loop** - Add handler in supervisor message loop for `ShutdownSignal`
3. **Trigger Checkpoint** - When signal received, call `create_shutdown_checkpoint()`
4. **Report Results** - Checkpoint result automatically sent via `checkpoint_tx`
5. **Test Integration** - Write tests combining supervisor + shutdown checkpoint flow
6. **Verify Timeouts** - Ensure checkpoint creation respects 25-second limit
