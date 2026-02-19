use oya::types::{FailureCategory, StageName as Stage, StageResult, TimelineEntry};
use oya::usage::{OyaUsageTrackerClient, ReportOutcomeRequest};
use restate_sdk::prelude::*;

use crate::orchestrator_types::{
    append_timeline, set_json_state, stage_attempt_key, write_orchestrator_state, FailureSnapshot,
    OrchestratorState, StageInputEvent, WorkspaceLifecycleEvent,
};
use crate::runtime_tools::{prepare_stage_workspace, WorkspacePrepRequest};
use crate::stage_executor::{execute_stage_real, StageExecutionRequest};

use super::OyaError;

// ── Deterministic helpers ─────────────────────────────────────────────────────

/// Get a deterministic RFC3339 timestamp within a Restate workflow context.
pub(crate) async fn deterministic_timestamp(
    ctx: &WorkflowContext<'_>,
) -> Result<String, TerminalError> {
    ctx.run(|| async move { Ok::<_, HandlerError>(chrono::Utc::now().to_rfc3339()) }).await
}

/// Parse an RFC3339 timestamp string into a DateTime<Utc>, falling back to UNIX_EPOCH.
pub(crate) fn parse_rfc3339_deterministic(s: &str) -> chrono::DateTime<chrono::Utc> {
    chrono::DateTime::parse_from_rfc3339(s)
        .map(|dt| dt.with_timezone(&chrono::Utc))
        .unwrap_or_else(|_| chrono::DateTime::UNIX_EPOCH)
}

/// Read an environment variable deterministically within a Restate workflow context.
pub(crate) async fn deterministic_env_var(
    ctx: &WorkflowContext<'_>,
    key: &str,
) -> Result<Option<String>, TerminalError> {
    let key = key.to_string();
    ctx.run(move || async move { Ok::<_, HandlerError>(std::env::var(&key).ok()) }).await
}

/// Check if an environment variable is set to a truthy value (1 or true).
pub(crate) async fn deterministic_env_bool(
    ctx: &WorkflowContext<'_>,
    key: &str,
) -> Result<bool, TerminalError> {
    let value = deterministic_env_var(ctx, key).await?;
    Ok(value.is_some_and(|v| v == "1" || v.eq_ignore_ascii_case("true")))
}

pub(crate) fn timestamp_error() -> OyaError {
    OyaError("timestamp error".to_string())
}

pub(crate) async fn deterministic_timestamp_or_error(
    ctx: &WorkflowContext<'_>,
) -> Result<String, OyaError> {
    deterministic_timestamp(ctx).await.map_err(|_error| timestamp_error())
}

// ── Data types ────────────────────────────────────────────────────────────────

#[derive(Clone)]
pub(crate) struct PipelineRunInput {
    pub(crate) run_id: String,
    pub(crate) bead_id: String,
    pub(crate) context: String,
}

pub(crate) struct PipelineState {
    pub(crate) current_stage: Stage,
    pub(crate) attempt: u32,
    pub(crate) last_failure: Option<(FailureCategory, String)>,
    pub(crate) orchestrator: OrchestratorState,
}

pub(crate) struct StageAttemptRecord {
    pub(crate) stage_input_key: String,
    pub(crate) workspace_info: Option<WorkspaceLifecycleEvent>,
}

pub(crate) struct StageArtifacts {
    pub(crate) stage_duration_ms: u64,
    pub(crate) event_at: chrono::DateTime<chrono::Utc>,
}

pub(crate) enum StageExecutionResult {
    Continue { stage_result: StageResult, stage_prompt: String },
    Stop,
}

// ── Constructor helpers ───────────────────────────────────────────────────────

pub(crate) fn pipeline_input(run_id: String, bead_id: String, context: String) -> PipelineRunInput {
    PipelineRunInput { run_id, bead_id, context }
}

pub(crate) async fn init_pipeline_state(
    ctx: &WorkflowContext<'_>,
    input: &PipelineRunInput,
    model: String,
) -> Result<PipelineState, OyaError> {
    let updated_at = deterministic_timestamp_or_error(ctx).await?;
    let orchestrator = OrchestratorState {
        status: "running".to_string(),
        stage: Stage::Plan.as_str().to_string(),
        attempt: 1,
        bead_id: input.bead_id.clone(),
        context: input.context.clone(),
        model,
        last_failure: String::new(),
        last_output: String::new(),
        last_prompt: String::new(),
        updated_at,
    };
    write_orchestrator_state(ctx, &orchestrator)?;
    Ok(PipelineState { current_stage: Stage::Plan, attempt: 1, last_failure: None, orchestrator })
}

// ── Stage lifecycle ───────────────────────────────────────────────────────────

pub(crate) async fn mark_stage_running(
    ctx: &WorkflowContext<'_>,
    state: &mut PipelineState,
) -> Result<(), OyaError> {
    let loop_ts = deterministic_timestamp_or_error(ctx).await?;
    state.orchestrator.stage = state.current_stage.as_str().to_string();
    state.orchestrator.attempt = state.attempt;
    state.orchestrator.status = "running".to_string();
    state.orchestrator.updated_at = loop_ts;
    write_orchestrator_state(ctx, &state.orchestrator)
}

pub(crate) async fn prepare_stage_attempt(
    ctx: &WorkflowContext<'_>,
    state: &PipelineState,
    input: &PipelineRunInput,
    config: &super::RuntimeConfig,
) -> Result<StageAttemptRecord, OyaError> {
    let stage_start = deterministic_timestamp_or_error(ctx).await?;
    let stage_input_key = stage_attempt_key(&state.current_stage, state.attempt, "input");
    let failure_snapshot = state.last_failure.as_ref().map(|(category, message)| FailureSnapshot {
        category: format!("{:?}", category),
        message: oya::types::truncate_clean(message, 2000),
    });
    set_json_state(
        ctx,
        &stage_input_key,
        &StageInputEvent {
            run_id: input.run_id.clone(),
            bead_id: input.bead_id.clone(),
            stage: state.current_stage.as_str().to_string(),
            attempt: state.attempt,
            context: input.context.clone(),
            last_failure: failure_snapshot,
            started_at: stage_start.clone(),
        },
    )?;
    let workspace_info =
        prepare_workspace_and_timeline(ctx, state, input, config, &stage_start).await?;
    Ok(StageAttemptRecord { stage_input_key, workspace_info })
}

async fn prepare_workspace_and_timeline(
    ctx: &WorkflowContext<'_>,
    state: &PipelineState,
    input: &PipelineRunInput,
    config: &super::RuntimeConfig,
    stage_start: &str,
) -> Result<Option<WorkspaceLifecycleEvent>, OyaError> {
    let workspace_ts = deterministic_timestamp_or_error(ctx).await?;
    let workspace_info = prepare_stage_workspace(WorkspacePrepRequest {
        run_id: input.run_id.clone(),
        bead_id: input.bead_id.clone(),
        stage: state.current_stage.clone(),
        attempt: state.attempt,
        recorded_at: workspace_ts,
        workspace_policy: config.workspace_policy,
        repo_root: config.repo_root.clone(),
    })?;
    persist_workspace_event(ctx, state, &workspace_info)?;
    append_timeline(
        ctx,
        TimelineEntry::StageStarted {
            stage: state.current_stage.as_str().to_string(),
            attempt: state.attempt,
            workspace: workspace_info.as_ref().map(|w| w.workspace.clone()),
            at: parse_rfc3339_deterministic(stage_start),
        },
    )
    .await?;
    Ok(workspace_info)
}

fn persist_workspace_event(
    ctx: &WorkflowContext<'_>,
    state: &PipelineState,
    workspace_info: &Option<WorkspaceLifecycleEvent>,
) -> Result<(), OyaError> {
    if let Some(workspace_event) = workspace_info {
        let workspace_key = stage_attempt_key(&state.current_stage, state.attempt, "workspace");
        set_json_state(ctx, &workspace_key, workspace_event)?;
    }
    Ok(())
}

// ── Stage execution ───────────────────────────────────────────────────────────

pub(crate) async fn execute_stage_with_tracker(
    ctx: &WorkflowContext<'_>,
    state: &mut PipelineState,
    input: &PipelineRunInput,
    config: &super::RuntimeConfig,
) -> Result<StageExecutionResult, OyaError> {
    let tracker = ctx.object_client::<OyaUsageTrackerClient>("global");
    let model = resolve_stage_model(&tracker, &state.current_stage).await?;
    state.orchestrator.model = model.clone();
    write_orchestrator_state(ctx, &state.orchestrator)?;
    let request = StageExecutionRequest {
        run_id: input.run_id.clone(),
        bead_id: input.bead_id.clone(),
        stage: state.current_stage.clone(),
        attempt: state.attempt,
        context: input.context.clone(),
        model: model.clone(),
        last_failure: state.last_failure.clone(),
    };
    match execute_stage_real(ctx, request, config.merge_queue_policy, config.repo_root.clone())
        .await
    {
        Ok((stage_result, stage_prompt)) => {
            report_stage_outcome(&tracker, &model, &stage_result).await;
            Ok(StageExecutionResult::Continue { stage_result, stage_prompt })
        }
        Err(error) => handle_stage_execution_error(ctx, state, error).await,
    }
}

async fn resolve_stage_model(
    tracker: &OyaUsageTrackerClient<'_>,
    stage: &Stage,
) -> Result<String, OyaError> {
    let tier = stage.model_for_stage().as_str().to_string();
    let active_model: Json<String> =
        tracker.get_active_model(tier).call().await.map_err(|error| {
            OyaError(format!("Failed to get active model from tracker: {}", error))
        })?;
    Ok(active_model.0)
}

async fn report_stage_outcome(
    tracker: &OyaUsageTrackerClient<'_>,
    model: &str,
    stage_result: &StageResult,
) {
    let report_req = ReportOutcomeRequest {
        model: model.to_string(),
        success: stage_result.passed,
        is_rate_limit: matches!(stage_result.failure_category, Some(FailureCategory::RateLimited)),
    };
    if let Err(error) = tracker.report_outcome(Json(report_req)).call().await {
        tracing::warn!("Failed to report outcome to tracker: {}", error);
    }
}

async fn handle_stage_execution_error(
    ctx: &WorkflowContext<'_>,
    state: &mut PipelineState,
    error: OyaError,
) -> Result<StageExecutionResult, OyaError> {
    let fail_ts = deterministic_timestamp_or_error(ctx).await?;
    state.orchestrator.status = "failed".to_string();
    state.orchestrator.last_failure = format!("Stage execution error: {}", error);
    state.orchestrator.updated_at = fail_ts.clone();
    write_orchestrator_state(ctx, &state.orchestrator)?;
    append_timeline(
        ctx,
        TimelineEntry::RunFailed {
            stage: state.current_stage.as_str().to_string(),
            category: "execution_error".to_string(),
            at: parse_rfc3339_deterministic(&fail_ts),
        },
    )
    .await?;
    Ok(StageExecutionResult::Stop)
}
