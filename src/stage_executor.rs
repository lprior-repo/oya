use super::OyaError;
use crate::orchestrator_types::GateResultData;
use crate::runtime_tools::{
    execute_gate, gate_failure_outcome, run_opencode, summarize_failure_output,
    validate_write_path, GateEvidence,
};
use crate::stage_runtime::{
    execute_ship_gate, stage_prompt, stage_success, ShipGateRequest, StagePromptInput,
};
use oya::types::{
    truncate_clean, FailureCategory, Gate, StageFailure, StageName as Stage, StageResult,
};
use restate_sdk::context::ContextSideEffects;
use restate_sdk::prelude::{HandlerError, Json, WorkflowContext};
use std::path::PathBuf;
use std::process::Command;
use std::time::Duration;

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub(super) struct StageExecution {
    pub passed: bool,
    pub output: String,
    pub failure_category: Option<FailureCategory>,
    pub next_stage: Option<Stage>,
    pub prompt: String,
    pub gate_results: Vec<GateResultData>,
}

pub(super) struct PromptStageRequest {
    pub prompt: String,
    pub stage: Stage,
    pub attempt: u32,
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
    pub last_failure: Option<StageFailure>,
}

#[derive(Clone)]
pub(super) struct StageBlockingInput {
    pub request: StageExecutionRequest,
    pub repo_root: PathBuf,
}

/// Executes a stage with stable Restate journaling.
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
    repo_root: PathBuf,
) -> Result<(StageResult, String, Vec<GateResultData>), OyaError> {
    validate_attempt(request.attempt)?;
    let input = StageBlockingInput { request: request.clone(), repo_root };
    let execution = stage_execution_journaled(ctx, input).await?;
    let StageExecution { passed, output, failure_category, next_stage, prompt, gate_results } =
        execution.0;
    let stage_result = StageResult {
        run_id: request.run_id,
        stage: request.stage,
        attempt: request.attempt,
        passed,
        output: serde_json::json!({ "output": output }),
        failure_category,
        next_stage,
    };
    Ok((stage_result, prompt, gate_results))
}

async fn stage_execution_journaled(
    ctx: &WorkflowContext<'_>,
    input: StageBlockingInput,
) -> Result<Json<StageExecution>, OyaError> {
    let timeout_seconds = stage_timeout_seconds(&input.request.stage);
    let timeout_stage = input.request.stage.clone();
    let timeout_attempt = input.request.attempt;
    let timeout_model = input.request.model.clone();
    ctx.run(move || async move {
        match tokio::time::timeout(
            Duration::from_secs(timeout_seconds),
            tokio::task::spawn_blocking(move || execute_stage_blocking(input)),
        )
        .await
        {
            Ok(join_result) => {
                let result = join_result.map_err(|error| {
                    HandlerError::from(format!("spawn_blocking failed: {}", error))
                })?;
                result.map(Json).map_err(|error| HandlerError::from(error.0))
            }
            Err(_) => Ok(Json(stage_timeout_execution(
                &timeout_stage,
                &timeout_model,
                timeout_attempt,
                timeout_seconds,
            ))),
        }
    })
    .await
    .map_err(|e| OyaError(format!("ctx.run failed: {}", e)))
}

fn stage_timeout_seconds(stage: &Stage) -> u64 {
    let default = match stage {
        Stage::JjWorkspace => 600,
        Stage::Implementation => 1_200,
        Stage::Main => 1_200,
    };

    std::env::var("OYA_STAGE_TIMEOUT_SECONDS")
        .ok()
        .and_then(|value| value.trim().parse::<u64>().ok())
        .map(|value| value.clamp(60, 3_600))
        .unwrap_or(default)
}

fn stage_timeout_execution(
    stage: &Stage,
    model: &str,
    attempt: u32,
    timeout_seconds: u64,
) -> StageExecution {
    StageExecution {
        passed: false,
        output: format!(
            "provider_diagnostics source=stage_executor_timeout stage={} attempt={} model={} timeout_seconds={}",
            stage.as_str(),
            attempt,
            model,
            timeout_seconds
        ),
        failure_category: Some(FailureCategory::ProviderUnavailable),
        next_stage: Some(stage.clone()),
        prompt: String::new(),
        gate_results: Vec::new(),
    }
}

fn validate_attempt(attempt: u32) -> Result<(), OyaError> {
    if attempt == 0 {
        Err(OyaError("attempt must be greater than 0".to_string()))
    } else {
        Ok(())
    }
}

pub(super) fn execute_stage_blocking(
    input: StageBlockingInput,
) -> Result<StageExecution, OyaError> {
    let request = input.request;
    if request.stage == Stage::Main {
        return execute_ship_gate(ShipGateRequest {
            attempt: request.attempt,
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
        attempt: request.attempt,
        success_message,
        success_next_stage,
        repo_root,
        model: request.model,
    })
}

pub(super) fn execute_prompt_stage(
    request: PromptStageRequest,
) -> Result<StageExecution, OyaError> {
    let baseline_paths = if request.stage == Stage::JjWorkspace {
        Some(collect_changed_paths(&request.repo_root)?)
    } else {
        None
    };

    let (opencode_ok, opencode_output) =
        run_opencode(request.prompt.as_str(), &request.repo_root, request.model.as_str())?;
    if !opencode_ok {
        return Ok(opencode_failure_stage_execution(&request, opencode_output));
    }

    if let Some(baseline) = baseline_paths.as_ref() {
        let violations =
            stage_write_violations_since(&request.stage, &request.repo_root, baseline)?;
        if !violations.is_empty() {
            return Ok(write_violation_stage_execution(&request, violations));
        }
    }

    let mut gate_results = Vec::new();
    for gate in request.stage.gates() {
        let gate_evidence = execute_gate(gate.clone(), &request.repo_root)?;
        gate_results.push(GateResultData {
            gate: gate.as_str().to_string(),
            passed: gate_evidence.passed,
            exit_code: gate_evidence.exit_code,
            command: gate_evidence.command.clone(),
            output: truncate_clean(&gate_evidence.output, 4000),
        });
        if !gate_evidence.passed {
            return Ok(gate_failure_stage_execution(&request, gate, gate_evidence, gate_results));
        }
    }

    Ok(StageExecution {
        passed: true,
        output: request.success_message.to_string(),
        failure_category: None,
        next_stage: request.success_next_stage,
        prompt: request.prompt,
        gate_results,
    })
}

fn stage_write_violations_since(
    stage: &Stage,
    repo_root: &PathBuf,
    baseline_paths: &std::collections::BTreeSet<String>,
) -> Result<Vec<String>, OyaError> {
    let changed_paths = collect_changed_paths(repo_root)?;
    Ok(changed_paths
        .difference(baseline_paths)
        .filter_map(|relative| {
            let absolute = repo_root.join(relative);
            validate_write_path(stage, absolute.as_path(), repo_root.as_path())
                .err()
                .map(|error| format!("{} ({})", relative, error))
        })
        .collect::<Vec<_>>())
}

fn collect_changed_paths(
    repo_root: &PathBuf,
) -> Result<std::collections::BTreeSet<String>, OyaError> {
    let mut changed = std::collections::BTreeSet::new();
    collect_changed_from_git(repo_root, &["diff", "--name-only"], &mut changed)?;
    collect_changed_from_git(
        repo_root,
        &["ls-files", "--others", "--exclude-standard"],
        &mut changed,
    )?;
    Ok(changed)
}

fn collect_changed_from_git(
    repo_root: &PathBuf,
    args: &[&str],
    changed: &mut std::collections::BTreeSet<String>,
) -> Result<(), OyaError> {
    let output =
        Command::new("git").arg("-C").arg(repo_root).args(args).output().map_err(|error| {
            OyaError(format!("failed to run git {}: {}", args.join(" "), error))
        })?;

    if !output.status.success() {
        return Err(OyaError(format!(
            "git {} failed: {}",
            args.join(" "),
            String::from_utf8_lossy(&output.stderr)
        )));
    }

    let stdout = String::from_utf8_lossy(&output.stdout);
    for line in stdout.lines().map(str::trim).filter(|line| !line.is_empty()) {
        changed.insert(line.to_string());
    }
    Ok(())
}

fn write_violation_stage_execution(
    request: &PromptStageRequest,
    violations: Vec<String>,
) -> StageExecution {
    let details = violations.join("\n");
    StageExecution {
        passed: false,
        output: format!(
            "write allowlist violation at stage={} attempt={}\n{}",
            request.stage.as_str(),
            request.attempt,
            details
        ),
        failure_category: Some(FailureCategory::OutputParseFailure),
        next_stage: Some(request.stage.clone()),
        prompt: request.prompt.clone(),
        gate_results: Vec::new(),
    }
}

fn opencode_failure_stage_execution(
    request: &PromptStageRequest,
    output: String,
) -> StageExecution {
    let category =
        oya::classify_opencode_error(&output).unwrap_or(FailureCategory::OutputParseFailure);
    let output = with_provider_diagnostics(category.clone(), request, output);
    let next_stage = request.stage.clone();
    StageExecution {
        passed: false,
        output,
        failure_category: Some(category),
        next_stage: Some(next_stage),
        prompt: request.prompt.clone(),
        gate_results: Vec::new(),
    }
}

fn with_provider_diagnostics(
    category: FailureCategory,
    request: &PromptStageRequest,
    output: String,
) -> String {
    if category != FailureCategory::ProviderUnavailable {
        return output;
    }
    let detail = summarize_failure_output(&category, &output);
    format!(
        "provider_diagnostics source=opencode stage={} attempt={} model={} detail={}\n{}",
        request.stage.as_str(),
        request.attempt,
        request.model,
        detail,
        output
    )
}

fn gate_failure_stage_execution(
    request: &PromptStageRequest,
    gate: Gate,
    gate_evidence: GateEvidence,
    gate_results: Vec<GateResultData>,
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
        gate_results,
    }
}

pub(super) fn format_gate_command_output(command: &str, exit_code: i32, output: &str) -> String {
    format!("command={} exit_code={}\n{}", command, exit_code, output)
}

pub(super) fn stage_failure_context(stage: &Stage, last_failure: &Option<StageFailure>) -> String {
    match (stage, last_failure) {
        (Stage::Implementation, Some(StageFailure { category, message, .. })) if *category == FailureCategory::LintFailed => {
            format!(
                "\n\nPREVIOUS CLIPPY FAILURE:\n{}\n\nCRITICAL: Fix the actual code issues. DO NOT use #[allow(...)] attributes to suppress warnings. Fix the underlying problem.",
                summarize_failure_output(&FailureCategory::LintFailed, message)
            )
        }
        (_, Some(StageFailure { category, message, .. })) => format!(
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
    use proptest::prelude::*;
    use std::sync::{
        atomic::{AtomicUsize, Ordering},
        Arc,
    };

    #[test]
    fn test_validate_attempt_rejects_zero() {
        let result = validate_attempt(0);
        assert!(result.is_err());
    }

    #[test]
    fn test_stage_timeout_seconds_has_defaults_and_override_clamp() {
        std::env::remove_var("OYA_STAGE_TIMEOUT_SECONDS");
        assert_eq!(stage_timeout_seconds(&Stage::JjWorkspace), 600);
        assert_eq!(stage_timeout_seconds(&Stage::Implementation), 1_200);

        std::env::set_var("OYA_STAGE_TIMEOUT_SECONDS", "30");
        assert_eq!(stage_timeout_seconds(&Stage::Main), 60);

        std::env::set_var("OYA_STAGE_TIMEOUT_SECONDS", "9999");
        assert_eq!(stage_timeout_seconds(&Stage::Implementation), 3_600);

        std::env::remove_var("OYA_STAGE_TIMEOUT_SECONDS");
    }

    #[test]
    fn test_stage_timeout_execution_marks_retryable_failure() {
        let execution = stage_timeout_execution(&Stage::JjWorkspace, "model", 2, 123);
        assert!(!execution.passed);
        assert!(execution.output.contains("123"));
        assert!(execution.output.contains("source=stage_executor_timeout"));
        assert_eq!(execution.failure_category, Some(FailureCategory::ProviderUnavailable));
        assert_eq!(execution.next_stage, Some(Stage::JjWorkspace));
    }

    #[test]
    fn test_stable_replay_contract_uses_cached_result() {
        let counter = Arc::new(AtomicUsize::new(0));
        let mut journaled: Option<StageExecution> = None;

        let first = mock_journal_replay(&mut journaled, {
            let counter = Arc::clone(&counter);
            move || {
                counter.fetch_add(1, Ordering::SeqCst);
                sample_execution()
            }
        });
        let replay = mock_journal_replay(&mut journaled, {
            let counter = Arc::clone(&counter);
            move || {
                counter.fetch_add(1, Ordering::SeqCst);
                StageExecution {
                    passed: false,
                    output: "should-not-run".to_string(),
                    failure_category: Some(FailureCategory::OutputParseFailure),
                    next_stage: None,
                    prompt: "replay".to_string(),
                    gate_results: Vec::new(),
                }
            }
        });

        assert!(first.passed);
        assert!(replay.passed);
        assert_eq!(counter.load(Ordering::SeqCst), 1);
    }

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
            gate_results: Vec::new(),
        }
    }

    fn mock_journal_replay<F>(journaled: &mut Option<StageExecution>, execute: F) -> StageExecution
    where
        F: FnOnce() -> StageExecution,
    {
        match journaled.clone() {
            Some(value) => value,
            None => {
                let value = execute();
                *journaled = Some(value.clone());
                value
            }
        }
    }
}
