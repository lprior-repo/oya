# Stream C: DAG Engine + Scheduling + Merge Flow

## Goal

Implement deterministic dependency scheduling and queue policy execution for governed bead runs.

## Canonical Decisions

- DAG implementation uses deterministic algorithms (Kahn + Tarjan).
- Scheduling state is persisted in Sled-backed run metadata.
- Merge flow integrates with zjj lifecycle, not UI workflows.

## Bead Set (8)

| Task | Title | Bead ID | Effort | Priority |
|---|---|---|---|---|
| task-001 | dag: Implement WorkflowDAG with petgraph DiGraph | `intent-cli-20260201014210-onbpr00o` | 4hr | 1 |
| task-002 | dag: Implement Kahn's algorithm for topological sort | `intent-cli-20260201014210-eyeydwsf` | 2hr | 1 |
| task-003 | dag: Implement Tarjan's algorithm for cycle detection | `intent-cli-20260201014210-6wxrwadt` | 4hr | 1 |
| task-004 | queue: Implement LIFOQueueActor for depth-first scheduling | `intent-cli-20260201014210-u8yonjrv` | 2hr | 2 |
| task-005 | queue: Implement RoundRobinQueueActor for fair tenant scheduling | `intent-cli-20260201014210-ucmumtbr` | 4hr | 2 |
| task-006 | dag: Implement ready/blocked dependency queries from Sled-backed state | `intent-cli-20260201014210-hjahahar` | 4hr | 1 |
| task-007 | scheduler: Implement SchedulerActor with DAG maintenance | `intent-cli-20260201014210-0vjoinp5` | 4hr | 1 |
| task-008 | merge-flow: Implement lifecycle integration with bead state and zjj | `intent-cli-20260201014210-qnjb0bbj` | 4hr | 1 |

## Quality and Validation

- Deterministic ordering for equivalent DAG inputs.
- Cycle detection blocks invalid schedules before execution.
- Queue policy behavior tested under load and contention.
- Validate with `moon run :quick` and `moon run :ci`.

## Success Criteria

- DAG scheduling is deterministic and reproducible.
- Ready/blocked decisions remain correct across restarts.
- Merge-flow state transitions remain auditable and policy-driven.
