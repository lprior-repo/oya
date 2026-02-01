# OpenCode Integration

## Overview

The oya system integrates with OpenCode AI for AI-powered stage execution. This allows you to delegate implementation, testing, refactoring, and other coding tasks to AI while maintaining full pipeline control.

## Supported Stages

The following stages can be executed with AI assistance:

- `implement` - AI-powered code generation
- `unit-test` - AI-generated unit tests
- `integration` - AI-generated integration tests
- `review` - AI code review
- `refactor` - AI-driven refactoring
- `document` - AI documentation generation

## Usage

### Basic AI Stage Execution

```bash
# Create a task
oya new -s my-feature

# Execute a stage with AI
oya ai-stage -s my-feature --stage implement

# With custom prompt
oya ai-stage -s my-feature --stage implement -p "Create a CLI parser using clap"

# With file context
oya ai-stage -s my-feature --stage test -f src/lib.rs -f src/main.rs
```

### Programmatic Usage

```rust
use oya_oya::{AIStageExecutor, Task, Stage};
use oya_opencode::PhaseInput;

// Create the AI executor
let executor = AIStageExecutor::new()?;

// Check if OpenCode is available
if !executor.is_available().await {
    eprintln!("OpenCode not installed");
    return;
}

// Execute a stage
let task = /* load task */;
let stage = Stage::new("implement", "none", 1);
let input = PhaseInput::text("Create a hello world function");

let result = executor.execute_stage(&task, &stage, Some(input)).await?;

if result.passed {
    println!("✓ Stage passed: {}", result.stage_name);
} else {
    eprintln!("✗ Stage failed: {:?}", result.error);
}
```

## Architecture

### Stage-to-Phase Mapping

OYA stages are mapped to OpenCode phases via `StagePhaseMapping`:

| OYA Stage | OpenCode Phase |
|--------------|----------------|
| implement    | implement      |
| unit-test    | test           |
| integration  | test           |
| review       | review         |
| refactor     | refactor       |
| document     | document       |

### Context Building

The `OYAPhaseContextBuilder` converts oya tasks into OpenCode `PhaseContext`:

1. Maps stage name to phase name
2. Generates phase description based on task and language
3. Adds language-specific constraints:
   - Rust: Railway-Oriented Programming, no unwrap/panic
   - Gleam: Functional patterns, pipelines
   - Go: Explicit error handling
   - Python: PEP 8, type hints
   - JavaScript: ES6+ features

### Execution Flow

```
Task + Stage
    ↓
OYAPhaseContextBuilder
    ↓
PhaseContext (constraints, description, input)
    ↓
AIExecutor → OpencodeClient → opencode CLI
    ↓
PhaseOutput
    ↓
StageExecution
```

## Requirements

- OpenCode CLI must be installed and available in PATH
- For remote execution, set `OPENCODE_BASE_URL` environment variable

## Error Handling

All operations use Result types with Railway-Oriented Programming:

```rust
// No unwrap or expect - all errors are explicit
match executor.execute_stage(&task, &stage, None).await {
    Ok(result) if result.passed => {
        // Success case
    }
    Ok(result) => {
        // Stage failed but execution succeeded
        handle_stage_failure(&result);
    }
    Err(e) => {
        // Execution error (OpenCode not available, etc.)
        handle_execution_error(&e);
    }
}
```

## Testing

Run the integration test suite:

```bash
moon run oya:test
```

Unit tests verify:
- Stage-to-phase mapping
- Context building for all languages
- PhaseOutput → StageExecution conversion
- Custom mapping registration
