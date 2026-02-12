# Checkpoint & Shutdown - Quick Reference Guide

## Architecture Diagram

```
┌─────────────────────────────────────────────────────────────────┐
│                    GRACEFUL SHUTDOWN FLOW                       │
└─────────────────────────────────────────────────────────────────┘

SIGNAL LAYER
┌──────────┐
│ SIGTERM  │         ┌──────────────────────────┐
│ SIGINT   │────────→│ install_signal_handlers()│
│ Ctrl+C   │         └──────────────────────────┘
└──────────┘                    │
                                ↓
                    ┌──────────────────────┐
                    │initiate_shutdown()   │
                    │(broadcast signal)    │
                    └──────────────────────┘
                                │
COORDINATION LAYER              ↓
┌───────────────────────────────────────────────────────────┐
│              ShutdownCoordinator (Arc)                     │
│                                                           │
│  Phase: Running → Initiating → SavingCheckpoints →        │
│         StoppingActors → Complete                         │
│                                                           │
│  Channels:                                                │
│  ├─ shutdown_tx: Broadcast signal to subscribers          │
│  ├─ checkpoint_tx: Collect checkpoint results             │
│  └─ checkpoint_rx: Receive results (timeout 25s)          │
└───────────────────────────────────────────────────────────┘
                                │
SUBSCRIBER COMPONENTS           ↓
┌──────────────────────────────────────────────────────────────┐
│ Supervisor Actor (and other components)                      │
│                                                              │
│ 1. Subscribe to shutdown_tx                                 │
│ 2. Receive ShutdownSignal                                   │
│ 3. Call create_shutdown_checkpoint()                        │
│ 4. Send CheckpointResult via checkpoint_tx                 │
└──────────────────────────────────────────────────────────────┘
                                │
CHECKPOINT LAYER                ↓
┌──────────────────────────────────────────────────────────────┐
│ CheckpointManager                                            │
│                                                              │
│ create_checkpoint(scheduler_state, snapshots) → CheckpointID │
│                                                              │
│ Stores via OrchestratorStore → SurrealDB                    │
└──────────────────────────────────────────────────────────────┘
                                │
                                ↓
                    ┌──────────────────────┐
                    │ CheckpointResult     │
                    │ ├─ component: str    │
                    │ ├─ success: bool     │
                    │ ├─ duration_ms: u64  │
                    │ └─ error: Option<str>│
                    └──────────────────────┘
                                │
                                ↓
                    ┌──────────────────────┐
                    │ ShutdownCoordinator  │
                    │ .shutdown() collects │
                    │ all results          │
                    └──────────────────────┘
                                │
                                ↓
                    ┌──────────────────────┐
                    │ ShutdownStats        │
                    │ ├─ saved: count      │
                    │ ├─ failed: count     │
                    │ ├─ error: Option<str>│
                    │ └─ total_ms: duration│
                    └──────────────────────┘
```

---

## Core Components Cheat Sheet

### 1. ShutdownCoordinator - Main Orchestrator

```rust
// Create
let coordinator = Arc::new(ShutdownCoordinator::new());

// Initiate shutdown (called by signal handler)
coordinator.initiate_shutdown(ShutdownSignal::Sigterm).await?;

// Execute full shutdown sequence
let stats = coordinator.shutdown().await?;

// Subscribe to signals
let mut rx = coordinator.subscribe();
match rx.recv().await {
    Ok(ShutdownSignal::Sigterm) => { /* handle */ },
    _ => {},
}

// Send checkpoint result
let result = CheckpointResult::success("component", 100);
coordinator.checkpoint_sender().send(result).await?;
```

### 2. CheckpointManager - Create & Retrieve

```rust
// Create checkpoint
let mut manager = CheckpointManager::new(store, config);
let checkpoint = manager.create_checkpoint(
    r#"{"state":"json"}"#,
    Some(r#"{"snapshots":"json"}"#)
).await?;

// Retrieve
let latest = manager.get_latest().await?;
let specific = manager.get_checkpoint("cp-id").await?;

// Restore typed state
let state: MyStateType = manager.restore_scheduler_state().await?;
let snapshots: Option<MySnapshotType> = manager.restore_workflow_snapshots().await?;

// Manage sequence
manager.increment_sequence();
let seq = manager.current_sequence();
manager.set_sequence(100);
```

### 3. Supervisor Checkpoint - During Shutdown

```rust
// In supervisor actor, when receiving ShutdownSignal:
let result = supervisor_state.create_shutdown_checkpoint(
    checkpoint_manager.as_mut(),
    &checkpoint_tx,
).await;

// Result is automatically sent to coordinator via checkpoint_tx
// Shutdown continues regardless of success/failure
```

### 4. CheckpointResult - Status Reporting

```rust
// Success
let result = CheckpointResult::success("scheduler", 150);
// Failure
let result = CheckpointResult::failure("scheduler", "timeout");

// Send to coordinator
checkpoint_tx.send(result).await?;
```

---

## Flow Examples

### Example 1: Basic Shutdown Flow

```rust
// 1. Install signal handlers
let coordinator = Arc::new(ShutdownCoordinator::new());
install_signal_handlers(coordinator.clone()).await?;

// 2. When SIGTERM arrives, handler does:
coordinator.initiate_shutdown(ShutdownSignal::Sigterm).await?;

// 3. Components subscribed to shutdown react:
let mut rx = coordinator.subscribe();
if let Ok(ShutdownSignal::Sigterm) = rx.recv().await {
    // Create checkpoint
    state.create_shutdown_checkpoint(manager, &checkpoint_tx).await?;
}

// 4. Main loop calls shutdown():
let stats = coordinator.shutdown().await?;
println!("Checkpoints saved: {}", stats.checkpoints_saved);
```

### Example 2: Checkpoint Creation During Shutdown

```rust
// Supervisor actor receives signal
tokio::select! {
    signal = rx.recv() => {
        if let Ok(ShutdownSignal::Sigterm) = signal {
            // Get components
            let checkpoint_tx = shutdown_coordinator.checkpoint_sender();
            
            // Create checkpoint
            let result = self.state.create_shutdown_checkpoint(
                checkpoint_manager.as_mut(),
                &checkpoint_tx,
            ).await;
            
            // Checkpoint automatically reported to coordinator
            // Shutdown continues whether result is Ok or Err
        }
    }
}
```

### Example 3: Recovery After Shutdown

```rust
// On restart, recover from checkpoint
let store = OrchestratorStore::connect(config).await?;
let engine = ReplayEngine::new(store);

// Load from checkpoint
let recovery = engine.recover_from_persistence().await?;

// Access recovered state
if let Some(checkpoint_id) = recovery.checkpoint_id {
    println!("Recovered from: {}", checkpoint_id);
}

// Replay events since checkpoint
for event in recovery.events {
    process_event(event).await?;
}
```

---

## Timeout Behavior

```
Shutdown Timeline (30s total):

0s    ┌─ initiate_shutdown() called
      │  Phase: Running → Initiating
      │
~0s   ├─ Broadcasting ShutdownSignal
      │  Components subscribe and react
      │
1-5s  ├─ SavingCheckpoints phase
      │  Components create checkpoints
      │  Timeout: 25 seconds
      │  (5s buffer before forced exit)
      │
6-25s ├─ Waiting for checkpoint results
      │  If timeout: continue with empty results
      │
25-28s├─ StoppingActors phase
      │  (Currently no-op, future enhancement)
      │
28-30s└─ Complete
         Return ShutdownStats
         If still running: Timeout → Force exit
```

---

## Data Structures at a Glance

### CheckpointRecord
```rust
{
    "checkpoint_id": "cp-1707234567890-100",
    "scheduler_state": "{...json...}",
    "event_sequence": 100,
    "created_at": "2024-02-11T12:34:56Z",
    "workflow_snapshots": "{...json...}",  // Optional
    "metadata": {...}                       // Optional
}
```

### SupervisorSnapshot (stored in checkpoint)
```json
{
    "config": {
        "max_restarts": 3,
        "restart_window_secs": 60,
        ...
    },
    "active_children": 5,
    "total_restarts": 2,
    "children": [
        {
            "name": "child-1",
            "restart_count": 1,
            "last_restart": "2024-02-11T12:30:00Z",
            "args": "debug args"
        },
        ...
    ],
    "failure_count_in_window": 1,
    "child_id_counter": 42,
    "snapshot_time": "2024-02-11T12:34:56Z",
    "shutdown_reason": null
}
```

### ShutdownStats (returned by coordinator)
```rust
{
    checkpoints_saved: 3,     // Number succeeded
    checkpoints_failed: 1,    // Number failed
    checkpoint_error: None,   // Overall error if any
    total_duration_ms: 15234  // Total time
}
```

---

## Error Handling Patterns

### Pattern 1: Non-Fatal Checkpoint Errors
```rust
// If checkpoint fails, error is logged and reported
// but shutdown CONTINUES

match create_shutdown_checkpoint(...).await {
    Ok(_) => { /* logged as success */ },
    Err(e) => {
        error!("Checkpoint failed: {}", e);  // Logged
        report_result(CheckpointResult::failure(...)).await?; // Reported
        // Shutdown proceeds anyway
    }
}
```

### Pattern 2: Result Collection with Timeout
```rust
// ShutdownCoordinator waits 25s for checkpoint results
// If timeout: returns empty vec, continues shutdown
// Not an error condition - graceful degradation

match timeout(CHECKPOINT_TIMEOUT, async {
    // Collect checkpoint results
}).await {
    Ok(results) => { /* use results */ },
    Err(_timeout) => { /* continue with empty */ },
}
```

### Pattern 3: Type-Safe Deserialization
```rust
// Checkpoint stores JSON, deserialization is typed
let state: SupervisorSnapshot = 
    serde_json::from_str(&checkpoint.scheduler_state)?;

// Type mismatch returns error
// Null handling via Option types
```

---

## Testing Patterns

### Test Template: Checkpoint Creation
```rust
#[tokio::test]
async fn test_create_shutdown_checkpoint() {
    // Setup
    let store = setup_store().await.expect("store setup");
    let mut manager = CheckpointManager::new(store, CheckpointConfig::default());
    
    // Execute
    let result = manager
        .create_checkpoint(r#"{"state":"active"}"#, None)
        .await;
    
    // Assert
    assert!(result.is_ok());
    assert_eq!(result.ok()?.event_sequence, 0);
}
```

### Test Template: Shutdown Signal Handling
```rust
#[tokio::test]
async fn test_shutdown_signal_broadcast() {
    let coordinator = ShutdownCoordinator::new();
    let mut rx = coordinator.subscribe();
    
    coordinator.initiate_shutdown(ShutdownSignal::Programmatic).await.ok();
    
    let signal = rx.recv().await;
    assert_eq!(signal.ok(), Some(ShutdownSignal::Programmatic));
}
```

### Test Template: Checkpoint Result Collection
```rust
#[tokio::test]
async fn test_checkpoint_result_collection() {
    let coordinator = ShutdownCoordinator::new();
    let tx = coordinator.checkpoint_sender();
    
    tx.send(CheckpointResult::success("test", 100)).await.ok();
    
    let stats = coordinator.shutdown().await.ok().unwrap();
    assert_eq!(stats.checkpoints_saved, 1);
}
```

---

## Integration Checklist

- [ ] Supervisor actor has `shutdown_coordinator: Option<Arc<ShutdownCoordinator>>`
- [ ] Supervisor actor has `checkpoint_manager: Option<CheckpointManager>`
- [ ] Supervisor subscribes to `shutdown_tx` on initialization
- [ ] Supervisor message loop handles `ShutdownSignal`
- [ ] Supervisor calls `create_shutdown_checkpoint()` when signal received
- [ ] Checkpoint result is sent via coordinator's `checkpoint_tx`
- [ ] Tests cover: checkpoint creation, signal subscription, shutdown flow
- [ ] Timeouts verified: checkpoint phase ≤ 25s, total ≤ 30s
- [ ] Error handling: no panics, all errors logged and reported
- [ ] Documentation: explain checkpoint ID format (cp-{timestamp}-{sequence})

---

## Key Files Reference

```
┌─ shutdown.rs (524 lines)
│  ├─ ShutdownCoordinator (main orchestration)
│  ├─ ShutdownSignal enum
│  ├─ ShutdownPhase enum
│  ├─ CheckpointResult struct
│  ├─ install_signal_handlers() function
│  └─ Tests: 9 test cases

├─ replay/checkpoint.rs (442 lines)
│  ├─ CheckpointManager (CRUD + deserialization)
│  ├─ CheckpointConfig struct
│  └─ Tests: 5 test cases

├─ actors/supervisor/checkpoint.rs (333 lines)
│  ├─ create_shutdown_checkpoint() method
│  ├─ SupervisorSnapshot struct
│  ├─ SupervisorCheckpointError enum
│  └─ Tests: 5 test cases

└─ persistence/checkpoint_store.rs (525+ lines)
   ├─ CheckpointRecord struct
   ├─ OrchestratorStore methods
   ├─ Database CRUD operations
   └─ Tests: 9+ test cases
```

Total: ~1800 lines of production code + tests, ready for integration.
