# Contract Specification

## Context
- Feature: Harden lifecycle DAG validity, quality gates, and durable observability.
- Domain terms: lifecycle step, dependency edge, terminal failure, compensation, durable status.
- Assumptions:
  - Lifecycle execution remains step-based command orchestration in `src/lifecycle/workflow.rs`.
  - Durable state is written through Restate handlers in `src/restate_oya/handlers.rs`.
  - `oya status` remains the operator-facing status view.
- Open questions:
  - Should quality gate relevance be path-based only, or path + semantic assertions from bead metadata?

## Preconditions
- Lifecycle step graph is well-formed:
  - every step id is unique,
  - every dependency references an existing step,
  - graph is acyclic.
- Step execution precondition:
  - all dependencies for step `S` are `succeeded`.
- Quality gate precondition:
  - workspace diff against `main@origin` is available.

## Postconditions
- DAG validation failure halts run before first mutating effect.
- Step with unmet dependencies is never executed.
- PR creation is only allowed when quality gates pass.
- Empty/non-meaningful diffs (including `.beads`-only changes) cannot open PR.
- Final status always includes truthful outcome and compensation outcomes.

## Invariants
- No successful lifecycle result may include `pr_create=succeeded` if quality gate failed.
- Compensation outcomes are persisted separately from forward journal and never omitted.
- `oya status` must be deterministic for a given completed workflow key.
- Status output remains bounded: large artifacts are summarized, not dumped unboundedly.

## Error Taxonomy
- `Error::DagInvalid` - dependency graph has cycles, missing nodes, or duplicate ids.
- `Error::DependencyNotMet` - step attempted before deps reached succeeded state.
- `Error::QualityGateFailed` - diff or policy checks failed (e.g., `.beads`-only changes).
- `Error::CompensationFailed` - one or more compensations failed during unwind.
- `Error::TelemetryWriteFailed` - durable telemetry persistence failed.

## Contract Signatures
- `fn validate_step_graph(steps: &[LifecycleStep]) -> Result<(), LifecycleError>`
- `fn dependencies_satisfied(statuses: &StepStatusMap, step: &LifecycleStep) -> Result<(), LifecycleError>`
- `fn evaluate_quality_gate(diff: &WorkspaceDiff) -> Result<QualityGateOutcome, LifecycleError>`
- `fn finalize_run(state: &LifecycleState, journals: &Journals) -> Result<LifecycleRunOutcome, LifecycleRunFailure>`

## Non-goals
- No scheduler-level parallel step execution in this phase.
- No global cross-workflow transaction semantics outside existing Restate durability boundaries.
