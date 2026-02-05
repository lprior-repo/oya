# Orchestrator Implementation Plan

## Overview

Production-grade orchestrator based on patterns from:
- [Temporal](https://github.com/temporalio/temporal) (701 test files)
- [Restate](https://github.com/restatedev/restate)
- [Hatchet](https://github.com/hatchet-dev/hatchet)

## Test Philosophy: Behavior-Driven Development (Martin Fowler)

### BDD Principles

Every test describes **observable behavior** from the user/system perspective:

1. **GIVEN** - Establish the precondition (system state before action)
2. **WHEN** - Perform the action under test (single action)
3. **THEN** - Assert the observable outcome (external behavior, not internals)

### Test Naming Convention

```rust
#[test]
fn given_<context>_when_<action>_then_<observable_outcome>()

// Examples:
fn given_empty_dag_when_add_first_node_then_node_count_is_one()
fn given_workflow_with_cycle_when_validated_then_error_contains_cycle_nodes()
fn given_bead_running_when_heartbeat_missed_for_30s_then_marked_timeout()
```

## Implementation Phases

### Phase 1: Fix Compile Blockers ✅ IN PROGRESS

**Files:**
- `crates/oya-web/src/actors/tests.rs`
- `crates/oya-web/tests/cors_test.rs`
- `crates/oya-web/tests/rest_api_test.rs`

**Issues:**
- `StateManagerMessage::QueryBead` needs `response` field
- `AppState` needs `broadcast_tx` field
- `BeadState` needs `title`, `dependencies` fields

### Phase 2: Complete WorkflowDAG ✅ IN PROGRESS

**File:** `crates/orchestrator/src/dag/mod.rs`

#### 2a. Core Methods

```rust
// Query methods
pub fn get_dependencies(&self, bead_id: &BeadId) -> Result<Vec<BeadId>, Error>
pub fn get_dependents(&self, bead_id: &BeadId) -> Result<Vec<BeadId>, Error>
pub fn get_all_ancestors(&self, bead_id: &BeadId) -> Result<HashSet<BeadId>, Error>
pub fn get_all_descendants(&self, bead_id: &BeadId) -> Result<HashSet<BeadId>, Error>
pub fn get_roots(&self) -> Vec<BeadId>
pub fn get_leaves(&self) -> Vec<BeadId>

// Ready detection
pub fn get_ready_nodes(&self, completed: &HashSet<BeadId>) -> Vec<BeadId>
pub fn get_blocked_nodes(&self, completed: &HashSet<BeadId>) -> Vec<BeadId>
pub fn is_ready(&self, bead_id: &BeadId, completed: &HashSet<BeadId>) -> Result<bool, Error>

// Ordering
pub fn topological_sort(&self) -> Result<Vec<BeadId>, Error>
pub fn topological_sort_kahn(&self) -> Result<Vec<BeadId>, Error>
pub fn critical_path(&self, weights: &HashMap<BeadId, Duration>) -> Result<Vec<BeadId>, Error>

// Validation
pub fn has_cycle(&self) -> bool
pub fn find_cycles(&self) -> Vec<Vec<BeadId>>
pub fn validate_no_self_loops(&self) -> Result<(), Error>
pub fn is_connected(&self) -> bool

// Mutation
pub fn remove_node(&mut self, bead_id: &BeadId) -> Result<(), Error>
pub fn remove_edge(&mut self, from: &BeadId, to: &BeadId) -> Result<(), Error>

// Subgraph operations
pub fn subgraph(&self, nodes: &[BeadId]) -> Result<WorkflowDAG, Error>
pub fn induced_subgraph(&self, bead_id: &BeadId) -> Result<WorkflowDAG, Error>
```

#### 2b. Error Types

```rust
#[derive(Debug, Clone, thiserror::Error)]
pub enum DagError {
    #[error("Node not found: {0}")]
    NodeNotFound(BeadId),
    #[error("Node already exists: {0}")]
    NodeAlreadyExists(BeadId),
    #[error("Edge already exists: {0} -> {1}")]
    EdgeAlreadyExists(BeadId, BeadId),
    #[error("Self-loop detected: {0}")]
    SelfLoopDetected(BeadId),
    #[error("Cycle detected involving: {0:?}")]
    CycleDetected(Vec<BeadId>),
    #[error("Invalid operation: {0}")]
    InvalidOperation(String),
    #[error("Graph not connected")]
    NotConnected,
}
```

### Phase 3: Unify Scheduler with Real DAG

**File:** `crates/orchestrator/src/scheduler.rs`

Delete internal WorkflowDAG (lines 32-80), use `crate::dag::WorkflowDAG`:

```rust
use crate::dag::{WorkflowDAG, DependencyType, DagError};

pub struct SchedulerState {
    workflows: HashMap<WorkflowId, WorkflowDAG>,
    bead_states: HashMap<BeadId, ScheduledBead>,
    completed_beads: HashSet<BeadId>,
    ready_queue: VecDeque<BeadId>,
    worker_assignments: HashMap<BeadId, WorkerId>,
    bead_to_workflow: HashMap<BeadId, WorkflowId>,
}
```

### Phase 4: Ractor Actor Implementation

**Module Structure:**
```
crates/orchestrator/src/actors/
├── mod.rs
├── scheduler_actor.rs
├── workflow_actor.rs
├── bead_actor.rs
├── supervisor.rs
├── messages.rs
└── errors.rs
```

### Phase 5: SurrealDB Integration

**Schema file:** `crates/orchestrator/src/persistence/schema.surql`

Tables:
- `workflow` - Workflow definitions and DAGs
- `bead` - Bead states and metadata
- `event` - Event log for replay
- `checkpoint` - Scheduler state checkpoints

### Phase 6: Database Replay/Recovery

Event sourcing with:
- Event types for all state transitions
- Replay engine for recovery
- Checkpoint manager for snapshots

### Phase 7: Agent Swarm Management

- AgentPool for worker management
- HealthMonitor for heartbeat tracking
- AgentHandle for individual agent lifecycle

### Phase 8: Task Distribution

Strategies:
- FIFO (default)
- Priority-based
- Round-robin
- Affinity-based

### Phase 9: UI DAG Visualization

JSON serialization of DAG for frontend rendering with layout hints.

## Test Coverage Targets

| Module | Unit | Integration | E2E | Chaos | Property | Total |
|--------|------|-------------|-----|-------|----------|-------|
| DAG | 30 | 15 | 10 | 5 | 10 | **70** |
| Scheduler | 25 | 20 | 15 | 10 | 8 | **78** |
| Actors | 35 | 25 | 15 | 15 | 10 | **100** |
| Persistence | 25 | 30 | 20 | 15 | 10 | **100** |
| Replay | 20 | 25 | 15 | 20 | 8 | **88** |
| Agent Swarm | 30 | 25 | 15 | 20 | 10 | **100** |
| Distribution | 20 | 15 | 10 | 10 | 8 | **63** |
| Visualization | 15 | 10 | 5 | 0 | 5 | **35** |
| **TOTAL** | 200 | 165 | 105 | 95 | 69 | **~634** |

## Test File Structure

```
crates/orchestrator/tests/
├── dag_behaviors.rs           # DAG observable behaviors
├── scheduler_behaviors.rs     # Scheduler observable behaviors
├── actor_behaviors.rs         # Actor system behaviors
├── persistence_behaviors.rs   # Storage behaviors
├── replay_behaviors.rs        # Recovery behaviors
├── agent_swarm_behaviors.rs   # Worker pool behaviors
├── distribution_behaviors.rs  # Routing behaviors
├── visualization_behaviors.rs # Rendering behaviors
└── scenarios/                 # Full user scenarios
    ├── ci_pipeline.rs
    ├── data_etl.rs
    ├── ml_training.rs
    └── failure_recovery.rs
```

## Current Progress

- [x] Phase 1: Fix compile blockers ✅ COMPLETE
  - `oya-web` tests fixed (broadcast_tx, QueryBead, BeadState fields)
  - Module structure corrected (broadcast_tests moved to mod.rs)

- [x] Phase 2a: DAG core methods ✅ COMPLETE (~1900 lines)
  - Query: get_dependencies, get_dependents, get_all_ancestors, get_all_descendants
  - Roots/leaves: get_roots, get_leaves
  - Ready detection: get_ready_nodes, get_blocked_nodes, is_ready
  - Ordering: topological_sort, topological_sort_kahn, critical_path
  - Validation: has_cycle, find_cycles, validate_no_self_loops, is_connected
  - Mutation: remove_node, remove_edge
  - Subgraph: subgraph, induced_subgraph

- [x] Phase 2b: DAG error types ✅ COMPLETE
  - DagError enum with thiserror
  - DagResult type alias
  - Factory methods for all error variants

- [x] Phase 2c: Update behavioral tests ✅ COMPLETE
  - 35 tests passing, 5 ignored (validation features TBD)
  - BDD naming: `given_<context>_when_<action>_then_<outcome>`
  - Covers: nodes, edges, dependencies, ready detection, cycles, ordering, subgraphs

- [x] Phase 3: Scheduler unification ✅ COMPLETE
  - Removed internal placeholder WorkflowDAG
  - Created WorkflowState wrapper with completed tracking
  - Integrated with real `crate::dag::WorkflowDAG`
  - Added `add_dependency()` and `get_workflow_ready_beads()` methods

- [x] Phase 3c: Scheduler behavioral tests ✅ COMPLETE
  - 37 scheduler unit tests (BDD naming)
  - 15 DAG integration tests
  - 27 doc tests
  - Total: 114 orchestrator tests passing (5 ignored for validation TBD)

- [ ] Phase 4: Ractor Actor Implementation (next)

## Key Design Decisions

1. **petgraph** for graph algorithms (toposort, cycle detection, SCC)
2. **DependencyType::BlockingDependency** vs **PreferredOrder** - only blocking deps affect readiness
3. **Zero-unwrap/zero-panic policy** - all fallible operations use Result
4. **BDD naming** - given_when_then for all tests
