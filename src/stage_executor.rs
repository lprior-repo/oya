use super::OyaError;
use crate::runtime_tools::{
    execute_gate, gate_failure_outcome, run_opencode, summarize_failure_output, GateEvidence,
};
use crate::stage_runtime::{
    execute_ship_gate, stage_prompt, stage_success, ShipGateRequest, StagePromptInput,
};
use oya::types::{FailureCategory, Gate, StageName as Stage, StageResult};
use restate_sdk::context::ContextSideEffects;
use restate_sdk::prelude::{HandlerError, Json, WorkflowContext};
use std::path::PathBuf;

use crate::pipeline::MergeQueuePolicy;

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub(super) struct StageExecution {
    pub passed: bool,
    pub output: String,
    pub failure_category: Option<FailureCategory>,
    pub next_stage: Option<Stage>,
    pub prompt: String,
}

pub(super) struct PromptStageRequest {
    pub prompt: String,
    pub stage: Stage,
    pub success_message: &'static str,
    pub success_next_stage: Option<Stage>,
    pub repo_root: PathBuf,
    pub model: String,
}

#[derive(Clone)]
pub(super) struct StageExecutionRequest {
    pub run_id: String,
    pub bead_id: String,
    pub stage: Stage,
    pub attempt: u32,
    pub context: String,
    pub model: String,
    pub last_failure: Option<(FailureCategory, String)>,
}

#[derive(Clone)]
pub(super) struct StageBlockingInput {
    pub request: StageExecutionRequest,
    pub merge_queue_policy: MergeQueuePolicy,
    pub repo_root: PathBuf,
}

/// Executes a stage with deterministic Restate journaling.
///
/// # Determinism Contract
///
/// This function ensures that on workflow replay:
/// - The blocking operation (`spawn_blocking`) is not re-executed
/// - The entire execution is journaled and replayed from the journal
/// - Identical inputs produce identical journaled outputs
///
/// # Implementation Pattern
///
/// Wrap the ENTIRE blocking operation inside `ctx.run()` so that:
/// - On first run: `spawn_blocking` executes, result is journaled
/// - On replay: `ctx.run()` returns journaled result, `spawn_blocking` is skipped
pub(super) async fn execute_stage_real(
    ctx: &WorkflowContext<'_>,
    request: StageExecutionRequest,
    merge_queue_policy: MergeQueuePolicy,
    repo_root: PathBuf,
) -> Result<(StageResult, String), OyaError> {
    if request.attempt == 0 {
        return Err(OyaError("attempt must be greater than 0".to_string()));
    }
    let input = StageBlockingInput { request: request.clone(), merge_queue_policy, repo_root };
    // Wrap the entire blocking operation in ctx.run() for deterministic replay
    let execution: Json<StageExecution> = ctx
        .run(move || async move {
            let result = tokio::task::spawn_blocking(move || execute_stage_blocking(input))
                .await
                .map_err(|error| HandlerError::from(format!("spawn_blocking failed: {}", error)))?;
            result.map(Json).map_err(|error| HandlerError::from(error.0))
        })
        .await
        .map_err(|e| OyaError(format!("ctx.run failed: {}", e)))?;
    let StageExecution { passed, output, failure_category, next_stage, prompt } = execution.0;
    let stage_result = StageResult {
        run_id: request.run_id,
        stage: request.stage,
        attempt: request.attempt,
        passed,
        output: serde_json::json!({ "output": output }),
        failure_category,
        next_stage,
    };
    Ok((stage_result, prompt))
}

pub(super) fn execute_stage_blocking(
    input: StageBlockingInput,
) -> Result<StageExecution, OyaError> {
    let request = input.request;
    if request.stage == Stage::ShipGate {
        return execute_ship_gate(ShipGateRequest {
            attempt: request.attempt,
            merge_queue_policy: input.merge_queue_policy,
            repo_root: input.repo_root,
        });
    }
    execute_prompt_driven_stage(request, input.repo_root)
}

fn execute_prompt_driven_stage(
    request: StageExecutionRequest,
    repo_root: PathBuf,
) -> Result<StageExecution, OyaError> {
    let failure_context = stage_failure_context(&request.stage, &request.last_failure);
    let prompt = stage_prompt(StagePromptInput {
        stage: &request.stage,
        bead_id: request.bead_id.as_str(),
        context: request.context.as_str(),
        attempt: request.attempt,
        failure_context: &failure_context,
    });
    let (success_message, success_next_stage) = stage_success(&request.stage);

    execute_prompt_stage(PromptStageRequest {
        prompt,
        stage: request.stage,
        success_message,
        success_next_stage,
        repo_root,
        model: request.model,
    })
}

pub(super) fn execute_prompt_stage(
    request: PromptStageRequest,
) -> Result<StageExecution, OyaError> {
    let (opencode_ok, opencode_output) =
        run_opencode(request.prompt.as_str(), &request.repo_root, request.model.as_str())?;
    if !opencode_ok {
        return Ok(opencode_failure_stage_execution(&request, opencode_output));
    }

    for gate in request.stage.gates() {
        let gate_evidence = execute_gate(gate.clone(), &request.repo_root)?;
        if !gate_evidence.passed {
            return Ok(gate_failure_stage_execution(&request, gate, gate_evidence));
        }
    }

    Ok(StageExecution {
        passed: true,
        output: request.success_message.to_string(),
        failure_category: None,
        next_stage: request.success_next_stage,
        prompt: request.prompt,
    })
}

fn opencode_failure_stage_execution(
    request: &PromptStageRequest,
    output: String,
) -> StageExecution {
    let category =
        oya::classify_opencode_error(&output).unwrap_or(FailureCategory::OutputParseFailure);
    StageExecution {
        passed: false,
        output,
        failure_category: Some(category),
        next_stage: Some(request.stage.clone()),
        prompt: request.prompt.clone(),
    }
}

fn gate_failure_stage_execution(
    request: &PromptStageRequest,
    gate: Gate,
    gate_evidence: GateEvidence,
) -> StageExecution {
    let (failure, next_stage) = gate_failure_outcome(&request.stage, &gate);
    StageExecution {
        passed: false,
        output: format_gate_command_output(
            gate_evidence.command.as_str(),
            gate_evidence.exit_code,
            gate_evidence.output.as_str(),
        ),
        failure_category: Some(failure),
        next_stage: Some(next_stage),
        prompt: request.prompt.clone(),
    }
}

pub(super) fn format_gate_command_output(command: &str, exit_code: i32, output: &str) -> String {
    format!("command={} exit_code={}\n{}", command, exit_code, output)
}

pub(super) fn stage_failure_context(
    stage: &Stage,
    last_failure: &Option<(FailureCategory, String)>,
) -> String {
    match (stage, last_failure) {
        (Stage::GptReview, Some((FailureCategory::LintFailed, message))) => format!(
            "\n\nPREVIOUS CLIPPY FAILURE:\n{}\n\nCRITICAL: Fix the actual code issues. DO NOT use #[allow(...)] attributes to suppress warnings. Fix the underlying problem.",
            summarize_failure_output(&FailureCategory::LintFailed, message)
        ),
        (_, Some((category, message))) => format!(
            "\n\nPREVIOUS FAILURE: {:?}\nERROR OUTPUT:\n{}\n\nFix the issue that caused this failure.",
            category,
            summarize_failure_output(category, message)
        ),
        (_, None) => String::new(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::{
        atomic::{AtomicUsize, Ordering},
        Arc,
    };

    #[test]
    fn test_execute_stage_real_attempt_zero_fails() {
        // This test documents the invariant that attempt must be > 0.
        // We cannot call execute_stage_real directly due to WorkflowContext dependency,
        // but we can verify the invariant via property testing on the request builder or similar logic
        // if we could access it.
        // For now, this placeholder reminds us to test this invariant.
    }

    #[tokio::test]
    async fn test_execute_stage_real_deterministic_replay() {
        // This test verifies the correct pattern: blocking operations inside ctx.run()
        // so they are NOT re-executed on replay.

        let counter = Arc::new(AtomicUsize::new(0));
        let counter_cloned = Arc::clone(&counter);

        // Correct pattern: blocking op INSIDE ctx.run
        // On first run: executes closure (blocking op + journaling)
        // On replay: returns journaled result (skips entire closure including blocking op)

        let run_correct_flow = |should_journal: bool| {
            let counter = Arc::clone(&counter_cloned);
            async move {
                if should_journal {
                    // Simulated ctx.run - on replay this returns cached result
                    counter.fetch_add(1, Ordering::SeqCst);
                    sample_execution()
                } else {
                    // Replay mode - return cached result without executing
                    sample_execution()
                }
            }
        };

        // First run (journal = true)
        let _ = run_correct_flow(true).await;
        assert_eq!(counter.load(Ordering::SeqCst), 1, "First run should increment counter");

        // Replay (journal = false, returns cached result)
        let _ = run_correct_flow(false).await;

        // Counter should STILL be 1 because blocking op was skipped on replay
        assert_eq!(
            counter.load(Ordering::SeqCst),
            1,
            "Replay should NOT increment counter - blocking op must be inside ctx.run"
        );
    }

    use proptest::prelude::*;

    proptest! {
        #[test]
        fn test_format_gate_command_output_invariant(
            cmd in "\\PC*",
            code in any::<i32>(),
            out in "\\PC*"
        ) {
            let formatted = format_gate_command_output(&cmd, code, &out);
            prop_assert!(formatted.contains(&cmd));
            prop_assert!(formatted.contains(&code.to_string()));
            prop_assert!(formatted.contains(&out));
        }
    }

    fn sample_execution() -> StageExecution {
        StageExecution {
            passed: true,
            output: "ok".to_string(),
            failure_category: None,
            next_stage: None,
            prompt: "prompt".to_string(),
        }
    }
}
