# Quality Gate CLI

Autonomous Development Triangle implementation for new-app.

## Commands

### Lint a spec
```bash
cargo run --bin spec-linter -- specs/flow-wasm-v1.yaml
```

### Run validation
```bash
cargo run --bin scenario-runner -- ../scenarios-vault/flow-wasm
```

### Full quality gate
```bash
cargo run --bin quality-gate -- full \
  --spec-path specs/flow-wasm-v1.yaml \
  --scenarios-path ../scenarios-vault/flow-wasm \
  --app-endpoint http://localhost:8080 \
  --level 3
```

## Feedback Levels

| Level | Name | Use Case |
|-------|------|----------|
| 1 | Minimal | Maximum holdout security |
| 2 | Categorical | Agent needs direction |
| 3 | Guided | Best balance (default) |
| 4 | Diagnostic | Agent is stuck |
| 5 | Transparent | Debugging only |

## Architecture

```
src/
├── linter/           # Spec quality validation
├── scenario_runner/  # Holdout scenario execution  
├── feedback/         # Sanitization pipeline
└── bin/
    └── quality-gate.rs  # CLI entry point
```

## Information Barrier

- **Agent sees**: `specs/`, `twins/`, acceptance criteria
- **Agent cannot see**: `scenarios-vault/` (enforced by separate repo)
