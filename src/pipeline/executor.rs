//! Batch stage executor - accumulates all stage data in-memory, persists once.
//!
//! This module implements the simplified state model:
//! - Executes each stage completely in-memory
//! - Accumulates ALL stage data (timing, workspace, input, prompt, output, gates)
//! - Persists ONE rich artifact after stage completion
//! - Only checkpoint at stage boundaries (~13 ops vs 140+)

#![deny(clippy::unwrap_used)]
#![deny(clippy::expect_used)]
#![deny(clippy::panic)]
#![deny(clippy::too_many_lines)]
#![deny(clippy::too_many_arguments)]
#![forbid(unsafe_code)]

use std::path::Path;
use std::time::Duration;

use oya::types::{truncate_clean, StageFailure, StageName as Stage, StageResult};
use restate_sdk::prelude::*;

use crate::orchestrator_types::{
    append_durable_event, resolve_change_identity, set_stage_artifact, DurableEvent,
    FailureSnapshot, GateResultData, StageArtifact, StageInputData, StageOutputData, StageStatus,
    StageTiming, WorkspaceLifecycle,
};
use crate::runtime_tools::prepare_stage_workspace;
use crate::stage_executor::{execute_stage_real, StageExecutionRequest};

use super::state::PipelineState;
use super::{OyaError, RuntimeConfig};

/// Input data for executing a stage and building its artifact.
#[derive(Clone)]
pub struct StageExecutionInput<'a> {
    pub run_id: &'a str,
    pub bead_id: &'a str,
    pub context: &'a str,
    pub model: &'a str,
    pub stage: Stage,
    pub attempt: u32,
    pub last_failure: Option<StageFailure>,
    pub repo_root: &'a Path,
}

/// Execute a stage and accumulate all data into a single rich artifact.
///
/// This function:
/// 1. Records start timestamp
/// 2. Executes the stage (no Restate ops during execution)
/// 3. Records end timestamp
/// 4. Executes all gates and collects results
/// 5. Builds complete artifact with all stage data
/// 6. Returns artifact for later persistence (in caller)
///
/// No Restate operations are performed during stage execution - only
/// timestamp capture via `ctx.run()`.
pub async fn execute_and_accumulate_stage(
    ctx: &WorkflowContext<'_>,
    input: StageExecutionInput<'_>,
    config: &RuntimeConfig,
    _state: &PipelineState,
) -> Result<StageArtifact, OyaError> {
    let started_at = super::state::workflow_timestamp_or_error(ctx).await?;
    let workspace = prepare_workspace_lifecycle(ctx, &input, config).await?;
    let execution_root = resolve_execution_root(input.repo_root, workspace.as_ref());
    let (stage_result, prompt, gates) =
        execute_stage_workflow(ctx, &input, config, execution_root).await?;
    let completed_at = super::state::workflow_timestamp_or_error(ctx).await?;

    Ok(build_stage_artifact(StageArtifactData {
        input: &input,
        workspace,
        prompt,
        timing: calculate_stage_timing(&started_at, &completed_at),
        stage_result,
        gates,
    }))
}

struct StageArtifactData<'a> {
    input: &'a StageExecutionInput<'a>,
    workspace: Option<WorkspaceLifecycle>,
    prompt: String,
    timing: StageTiming,
    stage_result: StageResult,
    gates: Vec<GateResultData>,
}

async fn execute_stage_workflow(
    ctx: &WorkflowContext<'_>,
    input: &StageExecutionInput<'_>,
    config: &RuntimeConfig,
    execution_root: std::path::PathBuf,
) -> Result<(StageResult, String, Vec<GateResultData>), OyaError> {
    execute_stage_real(
        ctx,
        StageExecutionRequest {
            run_id: input.run_id.to_string(),
            bead_id: input.bead_id.to_string(),
            stage: input.stage.clone(),
            attempt: input.attempt,
            context: input.context.to_string(),
            model: input.model.to_string(),
            last_failure: input.last_failure.clone(),
        },
        execution_root,
    )
    .await
}

fn build_stage_artifact(data: StageArtifactData<'_>) -> StageArtifact {
    let stage_input = build_stage_input_data(data.input);
    let status =
        if data.stage_result.passed { StageStatus::Completed } else { StageStatus::Failed };
    StageArtifact {
        stage: data.input.stage.as_str().to_string(),
        attempt: data.input.attempt,
        failure_category: data
            .stage_result
            .failure_category
            .as_ref()
            .map(|category| category.as_str().to_string()),
        next_stage: data.stage_result.next_stage.as_ref().map(|stage| stage.as_str().to_string()),
        timing: data.timing,
        workspace: data.workspace,
        input: stage_input,
        prompt: data.prompt,
        output: build_stage_output_data(&data.input.stage, &data.stage_result),
        task_tracking: None,
        gates: data.gates,
        status,
    }
}

/// Persist a completed stage artifact to Restate KVP.
///
/// This is the ONLY Restate set operation per stage (1.5 ops counting timestamps).
pub async fn persist_stage_artifact(
    ctx: &WorkflowContext<'_>,
    artifact: &StageArtifact,
) -> Result<(), OyaError> {
    let key = format!("{}_{}", artifact.stage, artifact.attempt);
    set_stage_artifact(ctx, &key, artifact)?;
    append_durable_event(
        ctx,
        DurableEvent {
            event_type: "stage_persisted".to_string(),
            run_id: artifact.input.run_id.clone(),
            bead_id: artifact.input.bead_id.clone(),
            stage: artifact.stage.clone(),
            attempt: artifact.attempt,
            status: match artifact.status {
                StageStatus::Completed => "completed".to_string(),
                StageStatus::Failed => "failed".to_string(),
            },
            reason: artifact.failure_category.clone().unwrap_or_else(|| "ok".to_string()),
            at: artifact.timing.completed_at.clone(),
            identity: resolve_change_identity(
                artifact.input.run_id.as_str(),
                artifact.input.bead_id.as_str(),
                artifact.workspace.as_ref().map(|workspace| workspace.name.as_str()),
            ),
        },
    )
    .await
}

// ---------------------------------------------------------------------------
// Helper functions (all pure, no side effects)
// ---------------------------------------------------------------------------

async fn prepare_workspace_lifecycle(
    ctx: &WorkflowContext<'_>,
    input: &StageExecutionInput<'_>,
    config: &RuntimeConfig,
) -> Result<Option<WorkspaceLifecycle>, OyaError> {
    if config.workspace_policy.should_skip() {
        return Ok(None);
    }
    let request = workspace_prep_request(ctx, input, config).await?;
    let workspace_event = run_workspace_prep(ctx, request).await?;
    Ok(workspace_event.0.map(workspace_lifecycle_from_event))
}

async fn workspace_prep_request(
    ctx: &WorkflowContext<'_>,
    input: &StageExecutionInput<'_>,
    config: &RuntimeConfig,
) -> Result<crate::runtime_tools::WorkspacePrepRequest, OyaError> {
    let recorded_at = super::state::workflow_timestamp_or_error(ctx).await?;
    Ok(crate::runtime_tools::WorkspacePrepRequest {
        run_id: input.run_id.to_string(),
        bead_id: input.bead_id.to_string(),
        stage: input.stage.clone(),
        attempt: input.attempt,
        recorded_at,
        workspace_policy: config.workspace_policy,
        repo_root: input.repo_root.to_path_buf(),
    })
}

async fn run_workspace_prep(
    ctx: &WorkflowContext<'_>,
    request: crate::runtime_tools::WorkspacePrepRequest,
) -> Result<Json<Option<crate::orchestrator_types::WorkspaceLifecycleEvent>>, OyaError> {
    let timeout_seconds = workspace_prep_timeout_seconds();
    ctx.run(move || async move {
        match tokio::time::timeout(
            Duration::from_secs(timeout_seconds),
            tokio::task::spawn_blocking(move || prepare_stage_workspace(request)),
        )
        .await
        {
            Ok(join_result) => {
                let result = join_result.map_err(|error| {
                    HandlerError::from(format!("workspace task join failed: {}", error))
                })?;
                result.map(Json).map_err(|error| HandlerError::from(error.0))
            }
            Err(_) => Err(HandlerError::from(format!(
                "workspace preparation timed out after {} seconds",
                timeout_seconds
            ))),
        }
    })
    .await
    .map_err(|error| OyaError(format!("workspace journaling failed: {}", error)))
}

fn workspace_lifecycle_from_event(
    event: crate::orchestrator_types::WorkspaceLifecycleEvent,
) -> WorkspaceLifecycle {
    WorkspaceLifecycle {
        name: event.workspace,
        path: event.workspace_path,
        queue_command: event.queue_command,
        queue_passed: event.queue_passed,
        queue_exit_code: event.queue_exit_code,
        add_command: event.add_command,
        add_passed: event.add_passed,
        add_exit_code: event.add_exit_code,
        coordination: event.coordination,
    }
}

fn workspace_prep_timeout_seconds() -> u64 {
    std::env::var("OYA_WORKSPACE_PREP_TIMEOUT_SECONDS")
        .ok()
        .and_then(|value| value.trim().parse::<u64>().ok())
        .map(|seconds| seconds.clamp(10, 300))
        .unwrap_or(45)
}

fn resolve_execution_root(
    repo_root: &Path,
    workspace: Option<&WorkspaceLifecycle>,
) -> std::path::PathBuf {
    workspace
        .map(|lifecycle| std::path::PathBuf::from(lifecycle.path.as_str()))
        .filter(|path| !path.as_os_str().is_empty())
        .unwrap_or_else(|| repo_root.to_path_buf())
}

fn calculate_stage_timing(started_at: &str, completed_at: &str) -> StageTiming {
    let start_dt = chrono::DateTime::parse_from_rfc3339(started_at)
        .map(|dt| dt.with_timezone(&chrono::Utc))
        .unwrap_or_else(|_| chrono::DateTime::UNIX_EPOCH);
    let end_dt = chrono::DateTime::parse_from_rfc3339(completed_at)
        .map(|dt| dt.with_timezone(&chrono::Utc))
        .unwrap_or_else(|_| chrono::DateTime::UNIX_EPOCH);
    let duration_ms = (end_dt - start_dt).num_milliseconds().max(0) as u64;

    StageTiming {
        started_at: started_at.to_string(),
        completed_at: completed_at.to_string(),
        duration_ms,
    }
}

fn build_stage_input_data(input: &StageExecutionInput<'_>) -> StageInputData {
    StageInputData {
        run_id: input.run_id.to_string(),
        bead_id: input.bead_id.to_string(),
        context: input.context.to_string(),
        model: input.model.to_string(),
        last_failure: input.last_failure.as_ref().map(|failure| FailureSnapshot {
            category: failure.category.as_str().to_string(),
            message: truncate_clean(&failure.message, 2000),
        }),
    }
}

fn build_stage_output_data(stage: &Stage, stage_result: &StageResult) -> StageOutputData {
    let full_log = truncate_clean(&stage_result.output.to_string(), 12000);
    let feedback = stage_result
        .failure_category
        .as_ref()
        .map_or_else(|| "Success".to_string(), |c| c.as_str().to_string());

    // Stage-specific output fields
    let (contract_document, implementation_code, test_results, adversarial_report) = match stage {
        Stage::Explore => (None, None, None, None),
        Stage::Contract => (Some(full_log.clone()), None, None, None),
        Stage::Red => (None, None, Some(full_log.clone()), None),
        Stage::Implementation => (None, Some(full_log.clone()), None, None),
        Stage::Witness => (None, None, None, Some(full_log.clone())),
        _ => (None, None, None, None),
    };

    StageOutputData {
        success: stage_result.passed,
        exit_code: if stage_result.passed { 0 } else { 1 },
        full_log,
        feedback,
        contract_document,
        implementation_code,
        test_results,
        adversarial_report,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use oya::types::FailureCategory;

    #[test]
    fn test_calculate_stage_timing_with_valid_rfc3339() {
        let started_at = "2026-02-19T20:38:15Z";
        let completed_at = "2026-02-19T20:38:17Z";

        let timing = calculate_stage_timing(started_at, completed_at);

        assert_eq!(timing.started_at, started_at);
        assert_eq!(timing.completed_at, completed_at);
        assert_eq!(timing.duration_ms, 2000);
    }

    #[test]
    fn test_calculate_stage_timing_handles_invalid_timestamps() {
        let started_at = "invalid";
        let completed_at = "also-invalid";

        let timing = calculate_stage_timing(started_at, completed_at);

        // Should fall back to UNIX_EPOCH for both, resulting in 0 duration
        assert_eq!(timing.started_at, started_at);
        assert_eq!(timing.completed_at, completed_at);
        assert_eq!(timing.duration_ms, 0);
    }

    #[test]
    fn test_calculate_stage_timing_negative_duration_becomes_zero() {
        let started_at = "2026-02-19T20:38:17Z";
        let completed_at = "2026-02-19T20:38:15Z"; // End before start

        let timing = calculate_stage_timing(started_at, completed_at);

        // Negative duration should become 0
        assert_eq!(timing.duration_ms, 0);
    }

    #[test]
    fn test_build_stage_input_data_with_last_failure() {
        let input = StageExecutionInput {
            run_id: "test-run",
            bead_id: "test-bead",
            context: "test context",
            model: "test-model",
            stage: Stage::Contract,
            attempt: 1,
            last_failure: Some(StageFailure {
                category: FailureCategory::TestFailed,
                message: "test failed".to_string(),
                retryable: false,
                failed_at: "2026-02-20T00:00:00Z".to_string(),
            }),
            repo_root: Path::new("/tmp"),
        };

        let stage_input = build_stage_input_data(&input);

        assert_eq!(stage_input.run_id, "test-run");
        assert_eq!(stage_input.bead_id, "test-bead");
        assert_eq!(stage_input.context, "test context");
        assert_eq!(stage_input.model, "test-model");
        assert!(stage_input.last_failure.is_some());
        let failure = stage_input.last_failure.unwrap();
        assert_eq!(failure.category, "test_failed");
        assert_eq!(failure.message, "test failed");
    }

    #[test]
    fn test_build_stage_input_data_without_last_failure() {
        let input = StageExecutionInput {
            run_id: "test-run",
            bead_id: "test-bead",
            context: "test context",
            model: "test-model",
            stage: Stage::Contract,
            attempt: 1,
            last_failure: None,
            repo_root: Path::new("/tmp"),
        };

        let stage_input = build_stage_input_data(&input);

        assert_eq!(stage_input.run_id, "test-run");
        assert!(stage_input.last_failure.is_none());
    }

    #[test]
    fn test_build_stage_output_data_for_contract_stage() {
        let stage_result = StageResult {
            run_id: "test-run".to_string(),
            stage: Stage::Contract,
            attempt: 1,
            passed: true,
            output: serde_json::json!("contract output"),
            failure_category: None,
            next_stage: None,
        };

        let stage_output = build_stage_output_data(&Stage::Contract, &stage_result);

        assert!(stage_output.success);
        assert_eq!(stage_output.exit_code, 0);
        assert!(stage_output.contract_document.is_some());
        assert!(stage_output.implementation_code.is_none());
        assert!(stage_output.test_results.is_none());
        assert!(stage_output.adversarial_report.is_none());
    }

    #[test]
    fn test_build_stage_output_data_for_failed_stage() {
        let stage_result = StageResult {
            run_id: "test-run".to_string(),
            stage: Stage::Contract,
            attempt: 1,
            passed: false,
            output: serde_json::json!("stage failed"),
            failure_category: Some(FailureCategory::CompileFailed),
            next_stage: None,
        };

        let stage_output = build_stage_output_data(&Stage::Contract, &stage_result);

        assert!(!stage_output.success);
        assert_eq!(stage_output.exit_code, 1);
        assert_eq!(stage_output.feedback, "compile_failed");
    }

    #[test]
    fn test_workspace_prep_timeout_seconds_defaults_and_clamps() {
        std::env::remove_var("OYA_WORKSPACE_PREP_TIMEOUT_SECONDS");
        assert_eq!(workspace_prep_timeout_seconds(), 45);

        std::env::set_var("OYA_WORKSPACE_PREP_TIMEOUT_SECONDS", "1");
        assert_eq!(workspace_prep_timeout_seconds(), 10);

        std::env::set_var("OYA_WORKSPACE_PREP_TIMEOUT_SECONDS", "900");
        assert_eq!(workspace_prep_timeout_seconds(), 300);

        std::env::remove_var("OYA_WORKSPACE_PREP_TIMEOUT_SECONDS");
    }
}
