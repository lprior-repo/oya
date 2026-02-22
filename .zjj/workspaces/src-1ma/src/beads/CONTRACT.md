# Quality Gates - Contract

## Design Contract: `quality-gates-v1`

### Purpose and Goals

- Define atomic, testable quality gates for the OYA project
- Ensure deterministic, contract-driven validation across all stages
- Guarantee zero panic, zero unwrap, zero expect via type safety
- Enable highly decomposed workflow with clear separation of concerns

### Key Functions

- `select_gates_for_stage(stage: StageName) -> Vector<Gate>` - Select gates per stage
- `execute_gate(gate: &Gate) -> GateResult` - Execute single gate
- `aggregate_gate_results(results: Vector<GateResult>) -> GateSummary` - Aggregate results
- `build_gate_report(summary: GateSummary, results: Vector<GateResult>) -> GateReport` - Build report
- `make_gate_decision(summary: &GateSummary) -> GateDecision` - Make pass/fail decision
- `generate_moon_command(gate: &Gate) -> String` - Generate moon command
- `run_quality_gate_pipeline(stage: StageName) -> Result<QualityGateOutcome, QualityGateError>` - Orchestrate

### Acceptance Criteria

- Gate selection returns non-empty vector for each stage
- Gate execution returns GateResult with Pass/Fail status
- Aggregation correctly counts passed/failed gates
- Report includes timestamp, unique ID, and all results
- Decision is Pass only if all gates passed
- Moon commands are valid for each gate type
- Pipeline orchestrates all steps with proper error handling
- Zero `unwrap()`, `expect()`, `panic!()` in all functions
- All functions are pure (no I/O in core logic)
- All tests pass (63 bead tests, 400+ total)

### Constraints

- Max 120 lines per bead file
- Use `im::Vector` for all collections
- Use `thiserror` for error types
- Zero panic/unwrap/expect policy
- Deterministic outputs for equivalent inputs
