# PLAN: src-3g9 - Verify codex-grade gpt review and rework routing

## Context
Ensure that the `GptReview` stage utilizes high-tier "codex-grade" models and follows the correct routing policy (failures route to `Tdd15` retry lane).

## Implementation Steps

1.  **Verify Model Assignment**:
    *   Confirm `StageName::model_for_stage(GptReview)` returns `ModelTier::Best`.
    *   Verify `oya.yaml` maps `a` and `s` tiers to Codex-grade models (GPT-5 Codex, Claude Opus).

2.  **Rework Routing Logic**:
    *   **Refactor `src/stage_executor.rs`**:
        *   Update `opencode_failure_stage_execution` to route `Stage::GptReview` to `Stage::Tdd15` instead of `Stage::Implementation` when OpenCode fails.
    *   **Refactor `src/runtime_tools/gates.rs`**:
        *   Update `gate_failure_mapping` to route all "retry lane" stages (`Qa`, `RedQueen`, `GptReview`) to `Stage::Tdd15` instead of `Stage::Implementation` upon gate failure.
        *   Ensure `Stage::ShipGate` still routes to `Stage::GptReview` on merge conflict (existing logic).

3.  **Unit Testing**:
    *   Add/Update tests in `src/runtime_tools/gates.rs` to verify the new mapping logic.
    *   Add/Update tests in `src/stage_executor.rs` (if feasible) to verify failure routing.

## Test Strategy & Quality Gates

### Quality Gates
- `moon run :check`: Ensure no type errors.
- `moon run :test`: Verify all unit and integration tests pass.
- `moon run :clippy`: Ensure strict linting compliance (no unwrap/panic).

### Test Scenarios
- **Success**: `GptReview` passes all gates -> transition to `ShipGate`.
- **OpenCode Failure**: `GptReview` LLM error -> transition to `Tdd15`.
- **Gate Failure (Lint)**: `GptReview` Clippy fail -> transition to `Tdd15`.
- **Gate Failure (Security)**: `GptReview` Security fail -> transition to `Tdd15`.
- **QA Failure**: `Qa` fail -> transition to `Tdd15`.
- **RedQueen Failure**: `RedQueen` fail -> transition to `Tdd15`.

## Verification
- Code review of `src/types/pipeline.rs`, `src/stage_executor.rs`, and `src/runtime_tools/gates.rs`.
- Execution of `moon run :test`.
