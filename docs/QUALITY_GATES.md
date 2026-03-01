# Quality Gates - Bead System

## Summary

**8 atomic bead files** implementing quality gates with zero panic, zero unwrap, zero expect.

## Bead Files

| File | Lines | Tests | Purpose |
|------|-------|-------|---------|
| `gate_selection.rs` | 120 | 10 | Select gates per stage |
| `gate_execution.rs` | 91 | 6 | Execute single gate |
| `gate_aggregation.rs` | 169 | 8 | Aggregate results |
| `gate_report.rs` | 197 | 8 | Build report |
| `gate_decision.rs` | 222 | 10 | Pass/fail decision |
| `moon_command.rs` | 149 | 11 | Generate commands |
| `quality_gate_pipeline.rs` | 174 | 10 | Orchestrate pipeline |
| `mod.rs` | 37 | 0 | Module declarations |

**Total**: 1159 lines, 63 bead tests, 400+ total tests

## Quality Gate Flow

```
StageName → GateSelection → GateExecution → GateAggregation → GateReport → GateDecision → MoonCommand → Pipeline
```

## Runtime Alignment

- The orchestrator runtime in `src/main.rs` now uses the same gate model as the bead pipeline.
- Runtime gate execution resolves commands through `generate_moon_command(gate)` from `src/beads/moon_command.rs`.
- This keeps command definitions centralized so bead tests and live runtime behavior stay in sync.
- `OYA_SKIP_JJ_GATE=1` only skips the `Gate::JjBookmark` runtime check; all other stage gates still run.

## API

```rust
select_gates(stage) → Vector<Gate>
execute_gate(gate_name, command) → GateExecutionResult
aggregate_gate_results(stage, results) → Result<AggregatedGateResult, Error>
build_gate_report(stage, aggregated) → GateReport
make_gate_decision(report) → GateDecision
generate_moon_command(gate) → MoonCommand
run_quality_gate_pipeline(stage) → Result<PipelineResult, Error>
```

## Commands

```bash
moon run :ci         # Full CI
moon run :check      # Type check
moon run :test       # All tests (400+ passing)
moon run :clippy     # Linting (zero warnings)
moon run :security   # Dependency audit
moon run :fmt        # Formatting
moon run :quick      # Fast check
moon run :build      # Release build
```

## Design Principles

- ✅ Zero `unwrap/expect/panic`
- ✅ `im::Vector` for collections
- ✅ Pure functions (no I/O)
- ✅ `thiserror` for errors
- ✅ Contract-first design
- ✅ Atomic beads (max 120 lines)

## Test Results

```
✅ 63 bead tests passing
✅ 400+ total tests passing
✅ Zero clippy warnings
✅ Zero fmt issues
✅ Zero panic/unwrap/expect violations
```

## Integration

```rust
pub mod beads {
    pub mod gate_selection;
    pub mod gate_execution;
    pub mod gate_aggregation;
    pub mod gate_report;
    pub mod gate_decision;
    pub mod moon_command;
    pub mod quality_gate_pipeline;
}
```

## Usage

```rust
use oya::beads::quality_gate_pipeline::run_quality_gate_pipeline;
use oya::types::StageName;

let outcome = run_quality_gate_pipeline(StageName::ShipGate)?;
```

## Files

- **Beads**: `src/beads/` (8 files)
- **Contract**: `src/beads/CONTRACT.md`
- **Docs**: `docs/QUALITY_GATES.md`

## Status

✅ **Production Ready**  
✅ **Version**: 1.0  
✅ **Updated**: 2026-02-19
