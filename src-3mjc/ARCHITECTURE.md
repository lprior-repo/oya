# Oya Architecture Documentation

**Generated from 850+ beads - Architecture reconstruction from project history**

---

## Executive Summary

Oya is a distributed workflow orchestration system inspired by three major systems:

1. **Erlang/BEAM** - Actor model, supervision trees, fault-tolerant message passing
2. **Restate** - Event sourcing, deterministic replay, checkpoint-based rollback
3. **Hatchet** - Worker pools, isolated task execution, pipeline orchestration

The system manages **workflows as DAGs of beads (tasks)** with:
- Actor-based supervision for fault tolerance
- Event sourcing for state reconstruction and rollback
- Parallel execution across isolated workspaces
- Chaos-tested resilience (100% recovery rate)

---

## System Architecture Overview

```
┌─────────────────────────────────────────────────────────────────────┐
│                        User Layer                                │
│  CLI (oya) | Web UI (oya-ui) | REST API (oya-web)            │
└────────────────────┬────────────────────────────────────────────────┘
                     │
                     ↓
┌─────────────────────────────────────────────────────────────────────┐
│                    SupervisorActor (Tier-1)                        │
│  ┌──────────────┐  ┌──────────────┐  ┌──────────────────┐   │
│  │ReplayEngine  │  │ EventBus     │  │ShutdownCoord    │   │
│  │(singletons)  │  │ (pub/sub)   │  │                 │   │
│  └──────────────┘  └──────────────┘  └──────────────────┘   │
└────────────────────┬────────────────────────────────────────────────┘
                     │
        ┌────────────┼────────────┐
        │            │            │
        ↓            ↓            ↓
┌──────────────┐ ┌──────────────┐ ┌──────────────┐
│SchedulerActor│ │StateManager  │ │ProcessPool  │
│             │ │ Actor        │ │ Actor       │
└──────────────┘ └──────────────┘ └──────────────┘
        │              │              │
        ↓              ↓              ↓
   WorkerActor → zjj        BeadStore → SurrealDB
   (PLANNED)   Workspace    (PLANNED)   (Primary)
```

---

## Pattern Fusion: BEAM + Restate + Hatchet

### 1. Erlang/BEAM Patterns

#### Actor Model (ractor 0.15)
- **Implementation**: `crates/orchestrator/src/actors/`
- **Core Principle**: "Let it crash" with supervision trees
- **Key Patterns**:
  - **Ping/Pong**: Bidirectional message passing for health checks
  - **Call/Cast/Send**: Three messaging modes
    - `call`: Sync with timeout, blocks until reply
    - `cast`: Fire-and-forget, never blocks
    - `send`: One-way message
  - **Deadlock Prevention**: Timeouts prevent indefinite blocking

#### Supervision Tree (3-Tier Hierarchy)
```
Tier-1: SupervisorActor (root, stateless)
├── Tier-2: Functional Actors (supervised one_for_one)
│   ├── SchedulerActor
│   ├── StateManagerActor
│   ├── ProcessPoolActor
│   ├── RateLimiterActor
│   └── EventStoreActor
└── Tier-3: Worker Actors (PLANNED)
    ├── WorkerActor instances
    └── Zellij integration
```

**Invariants**:
- Tier-1 supervisors never share state
- One_for_one restart strategy (isolated failures)
- 100% recovery rate under chaos testing

**Chaos Testing Coverage**:
- Kill tier-1 supervisors sequentially → full recovery
- Kill 5 actors simultaneously → cascading failure recovery
- Random tier-2 actor kills → 100% recovery
- Continuous chaos (5min random kills) → system stable
- Memory leak detection (1hr sustained load) → no leaks

### 2. Restate Patterns

#### Event Sourcing
- **Events**: Immutable record of all state changes
- **ReplayEngine**: Reconstruct state from event log
- **Deterministic Replay**: Same events = same state

**Event Types**:
```rust
enum BeadEvent {
    Created { bead_id },
    Scheduled { bead_id },
    Started { bead_id },
    Completed { bead_id },
    Failed { bead_id, error },
    Cancelled { bead_id },
}
```

**Event Serialization**:
- Format: bincode 1.3 (binary, <1KB per event)
- Performance: <1ms serialization overhead
- Versioning: Schema evolution support

#### Checkpoint & Rollback
- **Checkpoints**: Periodic state snapshots (60s interval)
- **Compression**: zstd achieves >50% size reduction
- **Rollback**: Restore checkpoint + replay events since checkpoint
- **Workflow Versioning**: Git-style version history

**Workflow Versioning Features**:
- Automatic checkpoints before each stage
- `oya rollback --to <version>` command
- Diff view between versions
- Branch/merge workflows
- Cherry-pick stages across versions

**Checkpoint Format**:
```
{
  version: "1.0",
  timestamp: ISO8601,
  workflow_dag: DAG snapshot,
  bead_states: HashMap<BeadId, BeadState>,
  compression: "zstd"
}
```

#### Time-Travel Queries
- **EventSourcingQueryAPI**: Query events by time range
- **Capabilities**:
  - Find events between T1 and T2
  - Reconstruct state at any timestamp
  - Event correlation and pattern detection
- **Uses**: Bottleneck identification, failure analysis

### 3. Hatchet Patterns

#### Worker Pool Pattern
- **ProcessPoolActor**: Manages worker state map
- **Worker States**: Idle, Claimed, Unhealthy, Dead
- **Invariants**:
  - Each worker in exactly one state
  - Pool size maintained
  - No worker starvation (round-robin)

**Worker Actor Responsibilities** (PLANNED):
1. Receive bead assignments from Scheduler
2. Claim beads from ready queue
3. Create isolated workspaces with zjj
4. Execute pipeline stages
5. Report completion/failure via EventBus
6. Handle cleanup and resource release
7. Support graceful shutdown

**Worker Messages**:
```rust
enum WorkerMessage {
    AssignBead { bead_id, workflow_id },
    ExecuteStage { stage, workspace },
    ReportResult { result },
    HealthCheck,
}
```

#### Task Isolation (zjj)
- **Workspace Isolation**: Each bead in isolated JJ workspace
- **Safety**: Prevents cross-bead contamination
- **Cleanup**: Automatic workspace disposal on completion

#### Pipeline Orchestration
- **9-Stage Pipeline**:
  1. implement - Code implementation
  2. unit-test - Unit testing
  3. coverage - Code coverage analysis
  4. lint - Static analysis/linting
  5. static - Static analysis
  6. integration - Integration testing
  7. security - Security scanning
  8. review - Code review
  9. accept - Final approval

- **Parallelization Strategy**:
  ```
  Sequential: implement → unit-test → coverage → lint → ... (~9 hrs)
  Parallel:   implement → [unit-test, coverage, lint] → 
                         [static, integration] → 
                         [security, review] → accept (~3 hrs)
  ```

---

## Message Passing Architecture

### EventBus
- **Role**: Central pub/sub for all events
- **Subscriptions**: Actors subscribe to event types
- **Broadcast**: Events pushed to WebSocket clients (<50ms latency)
- **Guarantee**: No event loss for connected clients

### Actor Message Protocols

#### SchedulerActor Messages
```rust
enum SchedulerMessage {
    RegisterWorkflow { workflow_id, reply },
    ScheduleBead { workflow_id, bead_id, reply },
    AddDependency { workflow_id, from, to, reply },
    ClaimReadyBead { workflow_id, bead_id, worker_id, reply },
    ReleaseBead { workflow_id, bead_id, reply },
    MarkCompleted { workflow_id, bead_id, reply },
    GetReadyBeads { workflow_id, reply },
    GetStats { workflow_id, reply },
}
```

#### StateManagerActor Messages
```rust
enum StateMessage {
    SaveState { workflow_id, state, reply },
    LoadState { workflow_id, reply },
    DeleteState { workflow_id, reply },
}
```

**Invariants**:
- `SaveState` always fsyncs before Ok reply (zero data loss)
- All messages use bincode serialization
- Original state preserved on serialization failure

#### ReplayEngine Protocol
```rust
enum ReplayMessage {
    AppendEvent { event, reply },
    ReplayFromCheckpoint { checkpoint_id, reply },
    GetEvents { filter: EventFilter, reply },
}
```

---

## Storage Layer

### SurrealDB (Primary Backend)

**Schema Tables**:
- `events` - Event log with timestamps
- `bead` - Workflow nodes with state tracking
- `depends_on` / `blocks` - Graph relations for DAG modeling
- `token_bucket` - Rate limiting state
- `concurrency_limit` - Resource management
- `webhooks` - External notification configs

**Connection Setup**:
- Backend: kv-rocksdb
- Connection pooling with resource cleanup
- Fail-fast on unavailable (no silent failures)

### DurableEventStore (Abstraction)

**Operations**:
```rust
trait EventStore {
    async fn get_events(&self, filter: EventFilter) -> Result<Stream<Event>>;
    async fn get_events_since(&self, seq: u64) -> Result<Stream<Event>>;
    async fn append(&self, event: Event) -> Result<()>;
    async fn append_batch(&self, events: Vec<Event>) -> Result<()>;
}
```

**Filter Capabilities**:
- bead_id filtering
- timestamp range (after, before)
- event_type filtering
- Streaming results (no OOM)

**Performance Targets**:
- Replay 1000 events <5s
- fsync overhead <5ms (target 2-3ms)
- Query ready beads <100ms (1000-bead DB)

---

## DAG Execution Engine

### Topological Sort & Dependency Resolution

**Algorithm**: Kahn's algorithm for topological sorting
- Input: Workflow DAG (nodes = beads, edges = dependencies)
- Output: Linear execution order
- Validation: Detect cycles (reject cyclic workflows)

**SurrealDB Query** (`find_ready_beads`):
```sql
SELECT bead_id FROM bead
WHERE state = 'pending'
  AND NOT EXISTS (
    SELECT 1 FROM depends_on
    WHERE from_bead IN (
      SELECT bead_id FROM bead
      WHERE state != 'completed'
    )
  )
ORDER BY bead_id ASC
```

**Performance**: <100ms for 1000-bead database

### Parallel Execution Strategies

#### Diamond DAG Pattern
```
A → B,C → D

1. A completes → B and C both become ready (parallel)
2. B and C execute concurrently
3. Both complete → D becomes ready
```

#### Parallel Fanout
```
split → [chunk1, chunk2, chunk3] → aggregate

Independent chunks execute in parallel, then aggregated
```

#### Stage Parallelization
- **Dependency Analysis**: Auto-detect parallelizable stages
- **Batch Execution**: Execute independent stages in batches
- **Resource Limits**: Limit parallelism to avoid overload
- **Progress Tracking**: Show parallel progress in UI

---

## State Management

### Event Sourcing State Machine

**State Transitions**:
```
Created → Scheduled → Started → Completed
                          ↓
                        Failed (→ retry or rollback)
```

**State Tracking**:
- **WorkflowState**: Per-workflow state with bead map and dependencies
- **BeadScheduleState**: Pending, Ready, Assigned, Completed
- **WorkerState**: Idle, Claimed, Unhealthy, Dead

### Progress Tracking

**ReplayProgress**:
```rust
struct ReplayProgress {
    events_total: AtomicU64,
    events_processed: AtomicU64,
    percent_complete: AtomicU64,
    eta: Duration,
}
```

**Updates**: Every 100 events via tokio::sync::watch
**Thread Safety**: AtomicU64 counters, watch channels

### Checkpoint System

**CheckpointManager**:
- Periodic snapshots every 60 seconds
- Serialization: bincode → zstd compression
- Target: >50% size reduction
- Property test: ∀ checkpoint, apply_events_since(checkpoint) → current state

---

## Error Handling & Resilience

### Retry Patterns
- **Exponential Backoff**: 100ms, 200ms, 400ms, max 3 retries
- **Transient vs Permanent**: Distinguish network/IO vs logic errors
- **Circuit Breaker**: Prevent cascade failures
  - States: Closed (open), Open (block), Half-Open (test)
  - Pattern: Fail fast → open → wait → test → close

### Idempotency
- **Duplicate Events**: Replay idempotent, no double-processing
- **Out-of-Order Events**: Reorder → apply correctly
- **Safe Retries**: Operations safe to retry (no side effects)

### Graceful Shutdown
- **ShutdownCoordinator**: Coordinates graceful termination
- **Checkpoint on Shutdown**: Persist state before exit
- **Drain Queues**: Complete in-flight work before stopping

---

## Workflow Versioning & Rollback

### Features
1. Automatic workflow checkpoints before each stage execution
2. Version history with git-style commit messages
3. `oya rollback --to <version>` command
4. Diff view between workflow versions
5. Branch/merge workflows for parallel experiments
6. Cherry-pick stages across versions

### Implementation
- **ReplayEngine Extension**: Add checkpoint store (persistent backend)
- **Workflow Metadata**: Version, parent, timestamp stored in bead-store
- **JJ Integration**: Store version hash in workflow
- **Checkpoint Format**: Snapshot of workflow DAG + bead states

### Rollback Flow
```
1. User: oya rollback --to v5
2. System: Load checkpoint v5
3. System: Replay events from v5+1 to current
4. Result: State exactly as it was after v5
5. Scheduler: Re-schedule beads from v5 state
```

---

## Distributed Execution (Future)

### oya remote Command
- **Worker Registration**: Workers register with capabilities (CPU, RAM, OS, arch)
- **Worker Pools**: Organized by team/project/environment
- **Health Checks**: Heartbeat + load monitoring
- **Load Balancing**: Least-loaded, round-robin, affinity
- **Fault Tolerance**: Retry on worker failure
- **Security**: TLS-encrypted communication (libp2p or gRPC)
- **Service Discovery**: mDNS or etcd

### DistributedExecutionCoordinator
- **Node Registry**: Track active workers
- **Work Distribution**: Assign beads to optimal workers
- **Result Aggregation**: Collect results from remote nodes
- **Network Partition Handling**: Split-brain detection and reconciliation

---

## Performance Optimization

### Metrics Collection

**Agent Metrics**:
- beads_completed/failed/running
- average_stage_duration
- resource_usage (CPU, memory)
- uptime, health_status

**Workflow Metrics**:
- total_execution_time
- bead_success_rate
- parallelization_efficiency
- critical_path_duration

**System Metrics**:
- scheduler_queue_depth
- event_bus_throughput
- database_latency
- cache_hit_rates

**MetricsCollector**: Prometheus export format
- Counters: beads_completed_total, stages_completed_total
- Gauges: agents_active, workflows_running, queue_depth
- Histograms: stage_duration_seconds, workflow_duration_seconds
- Endpoints: GET /metrics, GET /api/metrics/agents/:id

### Workflow Optimization Analyzer
- **Critical Path Analysis**: Identify longest execution path
- **Parallelization Detection**: Find independent beads
- **Bottleneck Detection**: Identify slow-executing beads
- **Resource Utilization Analysis**: Optimize allocation
- **Workflow Topology Optimization**: Suggest improvements

---

## Web API & Endpoints

### Framework: axum 0.7 + tower 0.5

**Endpoints**:
- `POST /api/workflows` - Create workflow
- `GET /api/workflows/:id/graph` - Workflow DAG visualization
- `GET /api/beads/:id` - Query bead status
- `POST /api/beads/:id/cancel` - Cancel bead execution
- `GET /api/health` - Health check
- `GET /metrics` - Prometheus metrics
- `GET /api/metrics/agents/:id` - Agent-specific metrics

**Middleware**:
- CORS for cross-origin requests
- Tracing for distributed tracing
- Compression (gzip) for bandwidth optimization

**Error Handling**:
- RFC 7807 Problem Details format
- <100ms latency target
- Idempotent operations (safe to retry)

---

## CLI Commands

### Core Commands
```bash
oya new -s <slug>              # Create task, init JJ workspace, create bead
oya stage -s <slug> --stage <name>  # Execute specific pipeline stage
oya approve -s <slug>           # Update bead to closed, validate completion
oya show -s <slug>               # Display task metadata, stage results
oya list                         # List all tasks with filtering
```

### Future Commands
```bash
oya rollback --to <version>      # Rollback workflow to previous version
oya diff <v1> <v2>             # Compare workflow versions
oya watch                        # Continuous integration with hot reload
oya remote                       # Distributed workflow execution
doctor                           # Health checks and diagnostics
benchmark                        # Performance testing and profiling
```

---

## Quality Standards

### Functional Rust Policy
- **Zero Panics**: No `panic!` in production code
- **Zero Unwraps**: No `.unwrap()` or `.expect()`
- **Zero Unsafe**: `#![forbid(unsafe_code)]`
- **Railway-Oriented**: All fallible operations return `Result<T, Error>`
- **Pure Functions**: No mutable by default, prefer immutability

### Testing
- **Test-First**: Tests written before implementation
- **Property-Based**: Proptest for exhaustive validation
- **Chaos Testing**: Validated 100% recovery rate
- **Coverage**: Target >80% code coverage
- **Zero Test Code Restrictions**: `#![allow(clippy::unwrap_used)]` in test modules

### Lints (workspace.lints)
```toml
[workspace.lints.clippy]
unwrap_used = "deny"
expect_used = "deny"
panic = "deny"
todo = "deny"
unimplemented = "deny"
arithmetic_side_effects = "deny"
option_if_let_else = "deny"
manual_let_else = "deny"
```

---

## File Organization

```
crates/orchestrator/src/
├── actors/
│   ├── mod.rs                    # Actor definitions
│   ├── errors.rs                 # Actor errors
│   ├── messages.rs               # Shared message types
│   ├── scheduler_actor.rs        # Workflow scheduling
│   ├── supervisor.rs             # Root supervisor
│   ├── queue.rs                 # ProcessPool, RoundRobin
│   └── storage/
│       └── state_manager.rs     # SurrealDB integration
├── replay/
│   ├── mod.rs                   # ReplayEngine
│   └── checkpoint.rs           # CheckpointManager
├── scheduler.rs                # Functional scheduler core
├── shutdown.rs                 # ShutdownCoordinator
└── lib.rs                     # Orchestrator entry

crates/events/src/
├── event.rs                    # BeadEvent types
├── bus.rs                      # EventBus implementation
├── durable_store.rs            # Event storage abstraction
└── replay/progress.rs          # Progress tracking

crates/core/src/
├── task.rs                     # Task domain types
├── workflow.rs                 # Workflow domain types
├── stage.rs                    # Stage definitions
├── slug.rs                     # Slug/ID types
└── error.rs                    # Shared error types

crates/bead-store/src/
├── store.rs                    # Bead tracking (PLANNED)
└── query.rs                    # Bead query APIs (PLANNED)

crates/oya-ui/src/
├── plugin.rs                   # Zellij WASM plugin
├── layout.rs                   # 3-pane layout (BeadList, Detail, Graph)
├── components.rs               # Timeline, GraphNode, GraphEdge
└── render.rs                   # Rendering logic

crates/oya-zellij/src/
├── timer.rs                    # RefreshTimer with 60s checkpoint
├── web_client.rs               # HTTP client for API calls
└── pipeline.rs                 # Pipeline stage rendering
```

---

## Implementation Status

### ✅ Implemented
- SupervisorActor with ReplayEngine init
- SchedulerActor basic structure
- StateManagerActor (SurrealDB + bincode)
- QueueActor (worker state tracking, round-robin)
- Event system (BeadEvent bincode serialization)
- EventBus pub/sub
- Axum REST API with tower middleware
- Chaos testing infrastructure
- Rate limiter (token bucket)
- Progress tracking (EventSourcingReplay)
- CheckpointManager with zstd compression

### ⏳ Planned/Open
- WorkerActor (3-4 days) - src-17nv
- BeadStore (2-3 days) - src-1p3y
- AgentRegistry - src-1p8i
- MetricsCollector (2-3 days) - src-23q2
- System health monitoring (2-3 days) - src-2bxp
- Workflow graph API (2-3 days) - src-3h80
- Workflow simulation/what-if (3-4 days) - src-2da2
- Workflow versioning and rollback (4-5 days) - src-11ra
- oya watch command (2-3 days) - src-1dgz
- oya remote distributed execution (P3) - src-2323

---

## Key Invariants and Guarantees

1. **Zero-Panic**: No unwrap/expect/panic in production code
2. **Functional Core**: Pure functions with Result<T, Error> returns
3. **Supervision**: 100% recovery rate, no data loss during failures
4. **Idempotency**: Operations safe to retry
5. **Event Ordering**: Serialized event processing via ReplayEngine
6. **Consistency**: Single ReplayEngine instance at supervisor level
7. **Fairness**: Round-robin queue prevents tenant starvation
8. **Rate Limiting**: Token bucket never exceeds capacity or goes negative
9. **Thread Safety**: All shared state protected (AtomicU64, watch channels, actor mailbox)
10. **Durability**: fsync before acknowledging all state changes

---

## References

**Key Beads for Deep Dive**:
- src-1wv9 - ractor 0.15 Actor trait and supervision patterns
- src-1ndp - Workflow DAG execution engine
- src-17nv - Worker actor implementation
- src-1ddg - SurrealDB and DurableEventStore integration
- src-1e3q - EventSourcingQueryAPI for workflow history
- src-11ra - Workflow versioning and rollback
- src-1iq5 - Parallel stage execution
- src-103n - EventSourcingReplay progress tracking
- src-16k2 - Checkpoint serialization testing

**Pattern Inspiration**:
- **Erlang/BEAM**: Actor model, supervision trees, fault-tolerant messaging
- **Restate**: Event sourcing, deterministic replay, checkpoint/rollback
- **Hatchet**: Worker pools, isolated task execution, pipeline orchestration

---

*Generated from 850+ beads by architecture mining agents*
*Last Updated: 2026-02-08*
