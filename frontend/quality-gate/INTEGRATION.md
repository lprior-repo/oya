# Quality Gate Integration for Oya
# Hooks into the orchestrator pipeline

## Integration Points

### 1. Pre-Commit Hook
Before any autonomous development starts, run spec linting:

```bash
# In your pipeline before Tdd15 stage
cargo run --bin spec-linter -- specs/flow-wasm-v1.yaml
```

### 2. Post-Development Hook  
After agent completes work, run validation:

```bash
# After Tdd15 stage, before Qa stage  
cargo run --bin scenario-runner -- ../scenarios-vault/flow-wasm
```

### 3. Full Quality Gate
Run complete pipeline:

```bash
cargo run --bin quality-gate -- full \
  --spec-path specs/flow-wasm-v1.yaml \
  --scenarios-path ../scenarios-vault/flow-wasm \
  --app-endpoint http://localhost:8080
```

## Oya Stage Integration

Add to your `oya.yaml` or workflow:

```yaml
stages:
  - name: spec-lint
    command: cargo run --bin spec-linter -- specs/flow-wasm-v1.yaml
    on_fail: abort
    
  - name: autonomous-dev
    command: # Agent development stage
    requires: [spec-lint]
    
  - name: behavioral-validation  
    command: cargo run --bin scenario-runner -- ../scenarios-vault/flow-wasm
    requires: [autonomous-dev]
    on_fail: feedback
```

## Configuration

Set environment variables:

```bash
export QUALITY_GATE_FEEDBACK_LEVEL=3  # 1-5
export QUALITY_GATE_MAX_ITERATIONS=5
export QUALITY_GATE_SPEC_THRESHOLD=80
```
