# Quality Gate System
# Orchestrator integration for Autonomous Development Triangle

## Overview

The quality gate module integrates with Oya's orchestrator to enforce
the Autonomous Development Triangle:
- High-Quality Specs
- Digital Twins
- Behavioral Scenarios (holdout)

## Architecture

```
┌─────────────────────────────────────────────────────────────┐
│                  ORCHESTRATOR                         │
│  ┌──────────────────────────────────────────────────────┐  │
│  │            QUALITY GATE STAGE              │  │
│  │                                              │  │
│  │  PHASE 1: SPEC VALIDATION                 │  │
│  │  ├─ Run spec-linter                        │  │
│  │  └─ Score ≥ threshold?                     │  │
│  │                    ↓                            │  │
│  │              NO → FAIL (abort)                │  │
│  │                    ↓                            │  │
│  │              YES → PHASE 2                  │  │
│  │                                              │  │
│  │  PHASE 2: SCENARIO VALIDATION           │  │
│  │  ├─ Run scenario-runner                   │  │
│  │  └─ All pass?                             │  │
│  │                    ↓                            │  │
│  │              YES → PASS ✓                      │  │
│  │                    ↓                            │  │
│  │              RETURN to orchestrator              │  │
│  │                    ↓                            │  │
│  │              NO → SEND FEEDBACK              │  │
│  │                    ↓                            │  │
│  │         iterations < max?                    │  │
│  │              YES → Retry (go to phase 2)   │  │
│  │              NO  → ESCALATE to human        │  │
│  └──────────────────────────────────────────────────────┘  │
└─────────────────────────────────────────────────────────────┘
```

## Configuration

### QualityGateConfig

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| spec_path | PathBuf | specs/flow-wasm-v1.yaml | Path to spec file |
| scenarios_path | PathBuf | ../scenarios-vault/flow-wasm | Path to scenarios |
| app_endpoint | String | http://localhost:8080 | Application endpoint |
| feedback_level | u8 | 3 | Feedback level (1-5) |
| spec_threshold | u32 | 80 | Minimum spec quality score |
| max_iterations | u32 | 5 | Max agent iterations |

## Usage

### Basic Usage

```rust
use oya::quality_gate::{QualityGate, QualityGateConfig};

let config = QualityGateConfig::default();
let mut gate = QualityGate::new(config);

loop {
    let result = gate.run()?;
    
    if result.passed() {
        break; // Success!
    }
    
    if !result.should_retry() {
        // Max iterations reached, escalate
        return Err(e);
    }
}
```

### Integration with Oya Orchestrator

Add quality gate as a stage in your workflow:

```yaml
stages:
  - name: witness
    action: run_holdout_scenarios
    requires: [implementation]  # After agent development
    on_fail: feedback

  - name: ship-gate
    action: run_ship_gate
    requires: [witness]
```

## State Transitions

### Quality Gate States

```
[WAITING] → [SPEC_VALIDATING] → [SCENARIO_VALIDATING] → [PASS] / [FEEDBACK] → [WAITING]
                              ↓
                          [FAIL]
```

### Feedback Loop

1. Agent completes development iteration
2. Quality gate runs:
   - Spec linter (if first iteration)
   - Scenario runner (always)
3. If pass: ✓ Move to next stage
4. If fail: Send sanitized feedback, increment iteration

## Feedback Sanitization

When scenarios fail, the quality gate:

1. Receives raw failure details from scenario runner
2. Removes sensitive information:
   - Exact HTTP requests
   - Exact expected values
   - Scenario IDs and step sequences
3. Sends agent only:
   - Failure category
   - Natural language description
   - Hint about fix
   - Reference to spec section

## Escalation

After max iterations without success:
- Create issue in tracking system
- Attach full validation report
- Notify human reviewer
- Block merge to main

## Metrics Tracked

- Spec quality score
- Scenarios passed/failed
- Iteration count
- Time per iteration
- Common failure categories
