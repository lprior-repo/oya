# Stream B: Actor System + Supervision

## Goal

Implement a resilient actor runtime with supervision and typed failure handling for governed bead execution.

## Canonical Decisions

- Supervision model: one-for-one with bounded backoff.
- Storage-facing actors read/write through Sled-backed services.
- Actor APIs return `Result<T, E>` and avoid panic paths.

## Bead Set (8)

| Task | Title | Bead ID | Effort | Priority |
|---|---|---|---|---|
| task-001 | actor-system: Study ractor and baseline patterns | `intent-cli-20260201013602-jvq0wsqm` | 2hr | 1 |
| task-002 | supervision: Implement UniverseSupervisor with one_for_one strategy | `intent-cli-20260201013602-pmhnlxc6` | 4hr | 1 |
| task-003 | storage-actors: Implement StateManagerActor and EventStoreActor | `intent-cli-20260201013602-oxbdslfu` | 4hr | 1 |
| task-004 | worker-actor: Implement BeadWorkerActor for bead lifecycle execution | `intent-cli-20260201013602-mgtchiyn` | 4hr | 1 |
| task-005 | queue-actors: Implement FIFOQueueActor and PriorityQueueActor | `intent-cli-20260201013700-jtnlsu5x` | 4hr | 2 |
| task-006 | rate-limiter-actor: Implement token bucket rate limiter | `intent-cli-20260201013602-hh3jm2uw` | 2hr | 2 |
| task-007 | reconciliation-actor: Implement reconciliation loop actor | `intent-cli-20260201013602-9zu2gjgt` | 4hr | 1 |
| task-008 | supervision-tests: Chaos tests for recovery behavior | `intent-cli-20260201013700-n3vsj0pd` | 4hr | 1 |

## Core Topology

- `UniverseSupervisor`
- `StorageSupervisor` (state + event actors)
- `WorkflowSupervisor` (worker pool)
- `QueueSupervisor` (queue + limiter actors)
- `ReconcilerSupervisor`

## Quality and Validation

- `moon run :quick` for iterative checks.
- `moon run :ci` for full validation.
- `moon run :ci --force` before merge-critical decisions.

## Success Criteria

- Supervision restarts recover expected failure classes.
- Actor-to-actor contracts remain deterministic.
- Storage actors and event actors integrate cleanly with Stream A.
