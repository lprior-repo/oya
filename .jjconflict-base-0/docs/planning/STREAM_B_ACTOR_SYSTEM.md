# Stream B: Actor System + Supervision

**Timeline**: Weeks 1-4
**Goal**: BEAM OTP supervision trees with fault isolation and automatic recovery using ractor 0.15
**LOC Target**: ~4k LOC

## Overview

Stream B implements a BEAM-style actor system with supervision for fault tolerance:
- 3-tier supervision hierarchy (UniverseSupervisor → Tier-1 → Tier-2 → Ephemeral)
- 8+ actor implementations (StateManager, EventStore, BeadWorker, Queue, RateLimiter, Reconciler)
- One-for-one restart strategy for isolated failures
- 100% recovery rate in chaos tests

## Planning Session

**Session ID**: `stream-b-actor-system`
**Session File**: `~/.local/share/planner/sessions/stream-b-actor-system.yml`
**Status**: Complete - 8 beads generated

## Beads Created (8 total)

| Task | Title | Bead ID | Effort | Priority |
|------|-------|---------|--------|----------|
| task-001 | actor-system: Study ractor 0.15 and implement ping/pong example | `intent-cli-20260201013602-jvq0wsqm` | 2hr | 1 |
| task-002 | supervision: Implement UniverseSupervisor with one_for_one strategy | `intent-cli-20260201013602-pmhnlxc6` | 4hr | 1 |
| task-003 | storage-actors: Implement StateManagerActor and EventStoreActor | `intent-cli-20260201013602-oxbdslfu` | 4hr | 1 |
| task-004 | worker-actor: Implement BeadWorkerActor for bead lifecycle execution | `intent-cli-20260201013602-mgtchiyn` | 4hr | 1 |
| task-005 | queue-actors: Implement FIFOQueueActor and PriorityQueueActor | `intent-cli-20260201013700-jtnlsu5x` | 4hr | 2 |
| task-006 | rate-limiter-actor: Implement token bucket rate limiter | `intent-cli-20260201013602-hh3jm2uw` | 2hr | 2 |
| task-007 | reconciliation-actor: Implement K8s-style ReconciliationLoopActor | `intent-cli-20260201013602-9zu2gjgt` | 4hr | 1 |
| task-008 | supervision-tests: Chaos tests for 100% supervision recovery | `intent-cli-20260201013700-n3vsj0pd` | 4hr | 1 |

## CUE Validation Schemas

All bead-specific CUE schemas stored in: `/home/lewis/src/oya/.beads/schemas/`

```
intent-cli-20260201013602-jvq0wsqm.cue  (task-001: ractor study)
intent-cli-20260201013602-pmhnlxc6.cue  (task-002: UniverseSupervisor)
intent-cli-20260201013602-oxbdslfu.cue  (task-003: Storage actors)
intent-cli-20260201013602-mgtchiyn.cue  (task-004: BeadWorker)
intent-cli-20260201013700-jtnlsu5x.cue  (task-005: Queue actors)
intent-cli-20260201013602-hh3jm2uw.cue  (task-006: RateLimiter)
intent-cli-20260201013602-9zu2gjgt.cue  (task-007: Reconciler)
intent-cli-20260201013700-n3vsj0pd.cue  (task-008: Chaos tests)
```

## Supervision Tree Architecture

```
UniverseSupervisor (one_for_one)
├── Tier 1: StorageSupervisor (one_for_one)
│   ├── Tier 2: StateManagerActor (ephemeral)
│   └── Tier 2: EventStoreActor (ephemeral)
├── Tier 1: WorkflowSupervisor (one_for_one)
│   └── Tier 2: WorkerPoolSupervisor (one_for_one)
│       └── Ephemeral: BeadWorkerActor (pool of 100)
├── Tier 1: QueueSupervisor (one_for_one)
│   ├── Tier 2: FIFOQueueActor
│   ├── Tier 2: PriorityQueueActor
│   └── Tier 2: RateLimiterActor
└── Tier 1: ReconcilerSupervisor (permanent)
    └── Tier 2: ReconciliationLoopActor (permanent)
```

## Key Technical Decisions

### Supervision Strategy
- **Root**: one_for_one (isolated tier-1 failures)
- **Restart**: Exponential backoff for repeated failures
- **Isolation**: One tier-1 crash doesn't affect others
- **Recovery**: <1s restart time (p99)

### Actor Message Protocol
- **Serialization**: bincode for binary efficiency
- **Ordering**: FIFO mailbox guarantees
- **Latency**: <1ms message passing (target)
- **Error Handling**: All actors return `Result<T, Error>`

### Actor Implementations

#### StateManagerActor
- Wraps SurrealDB CRUD operations
- Message handlers: SaveState, LoadState, DeleteState
- Error propagation for DB failures

#### EventStoreActor
- Wraps DurableEventStore from Stream A
- Message handlers: AppendEvent, ReadEvents, ReplayEvents
- Integrates with fsync guarantees

#### BeadWorkerActor
- Executes bead lifecycle (scheduled → running → completed)
- Integrates with WorkspaceManager (Stream D)
- Checkpoints every 60s
- Emits BeadEvent for every state transition

#### QueueActors (FIFO, Priority)
- FIFOQueueActor: VecDeque for FIFO ordering
- PriorityQueueActor: BinaryHeap for priority ordering
- Both support: Enqueue, Dequeue, Peek, Length

#### RateLimiterActor
- Token bucket algorithm
- Refill interval: 1s
- Capacity: Configurable (default 100 tokens)
- Non-blocking acquire (returns None if empty)

#### ReconciliationLoopActor
- K8s-style reconciliation every 1s
- Detects: Orphaned beads, dead workers, stuck beads
- Actions: Reschedule, Respawn, Cancel
- Permanent actor (never stops)

## Success Criteria

- ✅ UniverseSupervisor with 3-tier hierarchy implemented
- ✅ 8+ actor implementations complete
- ✅ Supervision tests pass (100% recovery rate)
- ✅ Actor message protocol using bincode
- ✅ Integration with event sourcing (actors append events)
- ✅ Message passing latency <1ms

## Dependencies

**Rust Crates**:
- `ractor = "0.15"` - BEAM-style actors
- `bincode = "1.3"` - Message serialization
- `tokio-test = "0.4"` - Async actor testing

## Critical Files

```
crates/orchestrator/src/supervision.rs          - UniverseSupervisor
crates/orchestrator/src/actors/storage.rs       - StateManagerActor
crates/orchestrator/src/actors/event_store.rs   - EventStoreActor
crates/orchestrator/src/actors/worker.rs        - BeadWorkerActor
crates/orchestrator/src/actors/queue.rs         - Queue actors
crates/orchestrator/src/actors/rate_limiter.rs  - RateLimiterActor
crates/reconciler/src/lib.rs                    - ReconciliationLoopActor
crates/orchestrator/tests/chaos.rs              - Chaos test suite
```

## Quality Gates

All beads enforce:
- Zero unwraps, zero panics (clippy forbid)
- Railway-Oriented Programming
- tokio-test for async testing
- 100% supervision recovery in chaos tests
- Actor restart latency <1s (p99)

## Chaos Test Scenarios

1. **Kill random tier-2 actors** (10 actors every 5s for 5min)
2. **Kill tier-1 supervisors** (sequentially, verify restart)
3. **Cascading failures** (kill 5 actors simultaneously)
4. **Continuous chaos** (5 minutes of random kills)
5. **Memory leak detection** (1 hour sustained load)

## Next Steps

1. Begin with task-001: Study ractor 0.15
2. Implement supervision tree tier by tier
3. Use CUE schemas for post-implementation validation
4. Run chaos tests continuously during development
