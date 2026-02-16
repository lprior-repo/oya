# Stream A: Event Sourcing + Durability

## Goal

Build the persistence foundation for governed execution using Sled, deterministic idempotency, checkpoints, and replay.

## Canonical Decisions

- Datastore baseline: **Sled**.
- Event log is append-only and durable.
- Idempotency keys are deterministic.
- Replay is deterministic and test-verified.

## Bead Set (8)

| Task | Title | Bead ID | Effort | Priority |
|---|---|---|---|---|
| task-001 | event-sourcing: Define Sled tree layout for events and run metadata | `intent-cli-20260201012642-u2duduno` | 4hr | 1 |
| task-002 | event-sourcing: Implement DurableEventStore with bincode serialization | `intent-cli-20260201012642-t73sooov` | 4hr | 1 |
| task-003 | event-sourcing: Benchmark durable append overhead | `intent-cli-20260201012642-xie2aw1d` | 2hr | 2 |
| task-004 | idempotency: Generate deterministic UUID v5 keys from bead+input | `intent-cli-20260201012642-2sgtpztz` | 2hr | 1 |
| task-005 | idempotency: Implement IdempotentExecutor with cache + Sled backing | `intent-cli-20260201012642-z9lgvyom` | 4hr | 1 |
| task-006 | checkpoint: Implement CheckpointManager with zstd compression | `intent-cli-20260201012642-m5hwrwle` | 4hr | 1 |
| task-007 | replay: Implement deterministic replay state machine | `intent-cli-20260201012642-ioyp3n1s` | 4hr | 1 |
| task-008 | integration: Event sourcing integration tests and validation | `intent-cli-20260201012642-srmhpngx` | 2hr | 1 |

## Storage Model (Sled)

- `events`: append-only event records
- `runs`: run metadata and status
- `attempts`: stage attempt state
- `idempotency`: deterministic key to result mapping
- `checkpoints`: compressed snapshots

## Quality and Validation

- Run fast checks with `moon run :quick` during development.
- Run full verification with `moon run :ci`.
- Run uncached verification with `moon run :ci --force`.
- Enforce zero unwrap/panic and deterministic replay behavior.

## Success Criteria

- Replay is deterministic across multiple runs.
- Idempotency prevents duplicate execution.
- Checkpoint save/restore is reliable.
- Durability behavior is validated by integration tests.
