# OYA Implementation Plan

## Purpose

Track the active implementation direction with zero legacy drift.

## Canonical Baseline

- Restate orchestrates run/state transitions.
- OpenCode Adapter is invoked as CLI subprocess for AI outputs.
- Sled is the active persistence baseline.
- zjj remains workspace isolation + merge-flow primitive.
- Moon is the CI/CD wrapper command surface.
- Beads (`br`) remain intake/lifecycle source of truth.
- No UI/frontend stream is active.

## Stage Pipeline

`contract -> tdd15 -> qa -> red_queen -> gpt_review -> ship_gate`

Retry lane:

- Failures in `qa`, `red_queen`, or `gpt_review` route back to `tdd15`.

## Implementation Waves

1. **Wave A**: Event sourcing durability, idempotency, checkpoints, replay.
2. **Wave B**: Actor supervision and recovery semantics.
3. **Wave C**: DAG scheduling and merge-flow integration.
4. **Wave D**: Process pool + OpenCode subprocess + zjj workspace lifecycle.
5. **Wave F**: E2E/chaos/perf verification.

## Gate Commands

- Fast: `moon run :quick`
- Full: `moon run :ci`
- Uncached full: `moon run :ci --force`

## Evidence Requirements

Each stage attempt must persist:

- structured stage response,
- gate command outcomes,
- failure category when applicable,
- artifact references for audits.

## Done Criteria

A run can ship only when:

- mandatory stages pass,
- required Moon gates pass,
- artifacts and rationale are persisted,
- merge-flow policy is satisfied.

## Architectural References

- `ARCHITECTURE_MASTER_PLAN.md`
- `docs/UBIQUITOUS_LANGUAGE.md`
- `docs/OPENCODE_INTEGRATION.md`
