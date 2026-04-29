# Flow Extender

## Purpose

`flow_extender` recommends safe graph patches when a workflow is missing common durability and control-flow protections. It supports three operator modes:

- suggest candidate rules (`suggest_extensions`)
- preview non-destructive patches (`preview_extension`)
- apply concrete graph mutations (`apply_extension`)

## Architecture

- **Rule registry**: `src/flow_extender/mod.rs` builds a static `rules()` list keyed by `ExtensionKey`.
- **Rule contract**: each rule carries preconditions, postconditions, invariants via `RuleContract`.
- **Restate semantics**: each analyzed suggestion and compound-plan step now carries explicit
  `ExtensionSemantics` metadata (`compatible_service_kinds`, `requires`, `provides`).
- **Planner**: each `plan_*` function checks workflow state and returns a `PatchPlan` only when its preconditions hold.
- **Preview**: `preview_from_patch` converts `PatchPlan` into temporary node/connection endpoints (`new-<idx>`), never mutating workflow state.
- **Executor**: `execute_patch` creates nodes, resolves endpoints, then attempts connections while preserving graph safety guards.

## Restate Guardrails

- Service-like workflows do not receive object/workflow-only recommendations (for example,
  `add-durable-checkpoint`, `add-compensation-branch`, `add-signal-resolution`).
- Signal resolution now maps to durable promises (`durable-promise` -> `resolve-promise`) so
  recommendations match Workflow promise semantics.
- Conflict detection reports semantic mismatches when requested extensions cannot run in the
  inferred service/object/workflow context.

Current extension keys:

- `add-entry-trigger`
- `add-timeout-guard`
- `add-durable-checkpoint`
- `add-compensation-branch`
- `add-signal-resolution`

## Operator Runbook

### In UI

- Open a node, then use **Extend Flow** in the selected node panel.
- Select one or more suggestions.
- Use **Apply** or **Apply Selected** to mutate workflow.
- Use **Clear** to reject selected suggestions.

Telemetry hooks:

- accepted: `single-apply`, `bulk-apply`
- rejected: `checkbox-toggle`, `bulk-clear`

### In CLI

Use `flow-extend` binary for scripted operations:

```bash
cargo run --bin flow_extend -- suggest path/to/workflow.json
cargo run --bin flow_extend -- preview path/to/workflow.json add-timeout-guard
cargo run --bin flow_extend -- apply path/to/workflow.json add-timeout-guard --output out.json
```

## Validation Surface

- Contract tests: `tests/flow_extender_contracts.rs`
- Scenario-runner extension cases: `specs/scenarios/flow_extender/*.yaml`
- Scenario aggregation test: `tests/scenario_runner_extension_cases.rs`
