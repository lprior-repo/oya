# OYA Master Planning Summary

**Date**: 2026-02-16  
**Status**: Canonical plan aligned to current architecture  
**Scope**: Streams A, B, C, D, F (no Stream E UI work)

## Canonical Constraints

- Restate is orchestration authority.
- OpenCode is used as CLI subprocess execution adapter.
- Sled is the persistence baseline until replacement is justified.
- zjj remains workspace isolation and merge-flow primitive.
- Moon is the CI/CD wrapper command surface.
- No UI/frontend stream is in scope.

## Stream Matrix

| Stream | Focus | Beads | Status |
|---|---|---:|---|
| A | Event sourcing + durability | 8 | Planned |
| B | Actor system + supervision | 8 | Planned |
| C | DAG scheduling + merge flow | 8 | Planned |
| D | Process pool + workspace isolation | 6 | Planned |
| F | Integration + chaos + perf | 6 | Planned |

Total planned beads: **36**.

## Current Execution Order

1. Stream A: durable state, idempotency, replay, checkpoints.
2. Stream B: supervision tree and actor behavior contracts.
3. Stream C: dependency DAG, queue policies, merge-flow governance.
4. Stream D: OpenCode subprocess workers and zjj integration.
5. Stream F: end-to-end validation, chaos recovery, load/perf proof.

## Quality Gates

- Fast gate: `moon run :quick`
- Full gate: `moon run :ci`
- Uncached verification: `moon run :ci --force`

## Bead Planning Notes

- Existing bead IDs under `.beads/` remain valid and can be reused.
- Any bead text mentioning SurrealDB or UI work must be rewritten before implementation.
- Stream E is intentionally removed from active scope.

## Deliverables for This Planning Set

- `docs/planning/STREAM_A_EVENT_SOURCING.md`
- `docs/planning/STREAM_B_ACTOR_SYSTEM.md`
- `docs/planning/STREAM_C_DAG_SCHEDULING.md`
- `docs/planning/STREAM_D_PROCESS_POOL.md`
- `docs/planning/STREAM_F_INTEGRATION.md`

## Definition of Ready

A bead is ready when:

- it has explicit contracts and acceptance criteria,
- failure handling is typed and testable,
- required Moon gates are identified,
- zjj workspace behavior is defined where relevant.

## Definition of Done

A bead is done when:

- implementation and tests pass required Moon gates,
- evidence artifacts are persisted,
- bead lifecycle is updated in `br`,
- work is synced and landed through zjj workflow.
