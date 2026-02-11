# Stream C: DAG Engine + Scheduling + Merge Queue

**Timeline**: Weeks 3-5
**Goal**: Dependency-aware workflow scheduling with cycle detection, queuing strategies, and merge queue management
**LOC Target**: ~4k LOC

## Overview

Stream C implements intelligent workflow scheduling with:
- WorkflowDAG using petgraph with Kahn's and Tarjan's algorithms
- 4 queueing strategies (FIFO, LIFO, RoundRobin, Priority)
- SchedulerActor with DAG maintenance
- Merge Queue with PR lifecycle management

## Planning Session

**Session ID**: `stream-c-dag-scheduling`
**Session File**: `~/.local/share/planner/sessions/stream-c-dag-scheduling.yml`
**Status**: Complete - 8 beads generated

## Beads Created (8 total)

| Task | Title | Bead ID | Effort | Priority |
|------|-------|---------|--------|----------|
| task-001 | dag: Implement WorkflowDAG with petgraph DiGraph | `intent-cli-20260201014210-onbpr00o` | 4hr | 1 |
| task-002 | dag: Implement Kahn's algorithm for topological sort | `intent-cli-20260201014210-eyeydwsf` | 2hr | 1 |
| task-003 | dag: Implement Tarjan's algorithm for cycle detection | `intent-cli-20260201014210-6wxrwadt` | 4hr | 1 |
| task-004 | queue: Implement LIFOQueueActor for depth-first scheduling | `intent-cli-20260201014210-u8yonjrv` | 2hr | 2 |
| task-005 | queue: Implement RoundRobinQueueActor for fair tenant scheduling | `intent-cli-20260201014210-ucmumtbr` | 4hr | 2 |
| task-006 | dag: Implement SurrealDB graph queries for dependency resolution | `intent-cli-20260201014210-hjahahar` | 4hr | 1 |
| task-007 | scheduler: Implement SchedulerActor with DAG maintenance | `intent-cli-20260201014210-0vjoinp5` | 4hr | 1 |
| task-008 | merge-queue: Implement PR lifecycle management with bead integration | `intent-cli-20260201014210-qnjb0bbj` | 4hr | 1 |

## CUE Validation Schemas

All bead-specific CUE schemas stored in: `/home/lewis/src/oya/.beads/schemas/`

```
intent-cli-20260201014210-onbpr00o.cue  (task-001: WorkflowDAG)
intent-cli-20260201014210-eyeydwsf.cue  (task-002: Kahn's algorithm)
intent-cli-20260201014210-6wxrwadt.cue  (task-003: Tarjan's algorithm)
intent-cli-20260201014210-u8yonjrv.cue  (task-004: LIFO queue)
intent-cli-20260201014210-ucmumtbr.cue  (task-005: RoundRobin queue)
intent-cli-20260201014210-hjahahar.cue  (task-006: SurrealDB queries)
intent-cli-20260201014210-0vjoinp5.cue  (task-007: SchedulerActor)
intent-cli-20260201014210-qnjb0bbj.cue  (task-008: Merge queue)
```

## Key Technical Decisions

### DAG Implementation
- **Library**: petgraph::DiGraph for graph representation
- **Node Type**: BeadId with metadata
- **Edge Type**: Dependency relations (depends_on, blocks)
- **Validation**: No cycles allowed (enforced by Tarjan's)

### Algorithms

#### Kahn's Topological Sort
- **Complexity**: O(V + E)
- **Determinism**: Sort by BeadId when in-degree=0 (stable ordering)
- **Output**: Vec<BeadId> in valid execution order
- **Cycle Detection**: Remaining nodes after sort indicate cycle

#### Tarjan's SCC Algorithm
- **Complexity**: O(V + E)
- **Purpose**: Find all strongly connected components (cycles)
- **Output**: Vec<Vec<BeadId>> - each inner vec is a cycle
- **Usage**: Validate DAG before scheduling

### Queueing Strategies

#### FIFO Queue (from Stream B)
- Data structure: VecDeque
- Ordering: First-in-first-out
- Use case: Default workflow scheduling

#### LIFO Queue (depth-first)
- Data structure: VecDeque with pop_back
- Ordering: Last-in-first-out (stack)
- Use case: Depth-first workflow traversal

#### RoundRobin Queue (fair)
- Data structure: HashMap<TenantId, VecDeque>
- Ordering: Fair rotation across tenants
- Use case: Multi-tenant fairness

#### Priority Queue (from Stream B)
- Data structure: BinaryHeap
- Ordering: Highest priority first
- Use case: Critical path prioritization

### SurrealDB Graph Queries

**Find Ready Beads**:
```surreal
SELECT id FROM bead
WHERE state = 'pending'
AND ->depends_on->bead[WHERE state != 'completed'] == NONE;
```

**Find Blocked Beads**:
```surreal
SELECT id FROM bead
WHERE state = 'pending'
AND ->depends_on->bead[WHERE state != 'completed'] != NONE;
```

**Get Dependency Chain**:
```surreal
-- Recursive traversal for transitive dependencies
SELECT id, ->depends_on->bead.* AS dependencies FROM bead:$bead_id;
```

### Scheduler Architecture

**SchedulerActor Responsibilities**:
1. Maintain WorkflowDAG per workflow
2. Subscribe to BeadCompleted events
3. Query ready beads from SurrealDB
4. Dispatch to appropriate queue (FIFO/LIFO/RoundRobin/Priority)
5. Rebuild DAG from DB on restart

**Integration Flow**:
```
SurrealDB → SchedulerActor → WorkflowDAG → Ready Beads → Queue Selection → QueueActor
```

### Merge Queue Design

**PR Lifecycle States**:
- `Pending` - Queued for testing
- `Testing` - Test bead executing
- `Merging` - Merge in progress
- `Merged` - Successfully merged
- `Failed` - Tests failed or merge conflict

**Features**:
- Automatic test bead creation on PR submission
- Conflict detection and rebase handling
- Batch merging strategies (sequential for MVP)
- Priority-based merge ordering
- Automatic retry on transient failures
- Integration with BeadWorkerActor for testing

## Success Criteria

- ✅ WorkflowDAG with Kahn's + Tarjan's implemented
- ✅ 4 queueing strategies working (FIFO/LIFO/RoundRobin/Priority)
- ✅ SchedulerActor with DAG maintenance
- ✅ Merge Queue with PR lifecycle management
- ✅ SurrealDB graph queries working
- ✅ Integration tests: 10-node DAG execution, rate limiting, merge queue

## Dependencies

**Rust Crates**:
- `petgraph = "0.6"` - Graph data structures and algorithms
- Integration with existing `crates/merge-queue/`

## Critical Files

```
crates/orchestrator/src/dag/graph.rs        - WorkflowDAG
crates/orchestrator/src/dag/algorithms.rs   - Kahn's and Tarjan's
crates/orchestrator/src/dag/queries.rs      - SurrealDB graph queries
crates/orchestrator/src/dag/scheduler.rs    - SchedulerActor
crates/orchestrator/src/queues/lifo.rs      - LIFOQueue
crates/orchestrator/src/queues/round_robin.rs - RoundRobinQueue
crates/merge-queue/src/lib.rs               - Merge queue
crates/merge-queue/src/pr_lifecycle.rs      - PR state machine
```

## Quality Gates

All beads enforce:
- Deterministic algorithms (same DAG → same order)
- Proptest for graph property testing
- Cycle detection before scheduling
- Fair queueing verified (<5% variance)
- Merge queue handles 10 PRs correctly

## Next Steps

1. Implement WorkflowDAG with petgraph
2. Add Kahn's and Tarjan's algorithms
3. Complete queue actor implementations
4. Integrate SchedulerActor with event sourcing
5. Wire merge queue to bead lifecycle
