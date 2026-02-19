use oya::types::{FailureCategory, GateSummary, StageResult, TimelineEntry};
use oya::{is_retryable_failure, types::truncate_clean};
use restate_sdk::prelude::*;

use crate::orchestrator_types::{
    append_timeline, write_orchestrator_state, WorkspaceLifecycleEvent,
};

use super::state::{
    deterministic_timestamp_or_error, parse_rfc3339_deterministic, PipelineState, StageArtifacts,
};
use super::OyaError;

struct FailureTransitionContext<'a> {
    stage_result: &'a StageResult,
    workspace_info: &'a Option<WorkspaceLifecycleEvent>,
    artifacts: &'a StageArtifacts,
    fail_ts: &'a str,
}

struct FailureEntry {
    attempt: u32,
    category: String,
    message: String,
    retry_scheduled: bool,
    at: chrono::DateTime<chrono::Utc>,
}

pub(crate) async fn handle_stage_transition(
    ctx: &WorkflowContext<'_>,
    state: &mut PipelineState,
    stage_result: &StageResult,
    workspace_info: &Option<WorkspaceLifecycleEvent>,
    artifacts: &StageArtifacts,
) -> Result<bool, OyaError> {
    if stage_result.passed {
        return handle_success_transition(ctx, state, stage_result, workspace_info, artifacts)
            .await;
    }
    handle_failure_transition(ctx, state, stage_result, workspace_info, artifacts).await
}

async fn handle_success_transition(
    ctx: &WorkflowContext<'_>,
    state: &mut PipelineState,
    stage_result: &StageResult,
    workspace_info: &Option<WorkspaceLifecycleEvent>,
    artifacts: &StageArtifacts,
) -> Result<bool, OyaError> {
    let gates = state
        .current_stage
        .gates()
        .iter()
        .map(|gate| GateSummary { gate: gate.as_str().to_string(), passed: true })
        .collect();
    append_timeline(
        ctx,
        TimelineEntry::StageCompleted {
            stage: state.current_stage.as_str().to_string(),
            attempt: state.attempt,
            workspace: workspace_info.as_ref().map(|w| w.workspace.clone()),
            duration_ms: artifacts.stage_duration_ms,
            gates,
            at: artifacts.event_at,
        },
    )
    .await?;
    match stage_result.next_stage.clone() {
        Some(next_stage) => {
            state.current_stage = next_stage;
            state.attempt = 1;
            state.last_failure = None;
            Ok(false)
        }
        None => mark_run_shipped(ctx, state, artifacts.stage_duration_ms).await,
    }
}

async fn mark_run_shipped(
    ctx: &WorkflowContext<'_>,
    state: &mut PipelineState,
    stage_duration_ms: u64,
) -> Result<bool, OyaError> {
    let shipped_ts = deterministic_timestamp_or_error(ctx).await?;
    state.orchestrator.status = "shipped".to_string();
    state.orchestrator.stage = "none".to_string();
    state.orchestrator.updated_at = shipped_ts.clone();
    write_orchestrator_state(ctx, &state.orchestrator)?;
    append_timeline(
        ctx,
        TimelineEntry::RunShipped {
            total_duration_ms: stage_duration_ms,
            stages_passed: 8,
            at: parse_rfc3339_deterministic(&shipped_ts),
        },
    )
    .await?;
    Ok(true)
}

async fn handle_failure_transition(
    ctx: &WorkflowContext<'_>,
    state: &mut PipelineState,
    stage_result: &StageResult,
    workspace_info: &Option<WorkspaceLifecycleEvent>,
    artifacts: &StageArtifacts,
) -> Result<bool, OyaError> {
    let fail_ts = deterministic_timestamp_or_error(ctx).await?;
    let failure_context =
        FailureTransitionContext { stage_result, workspace_info, artifacts, fail_ts: &fail_ts };
    state.last_failure =
        stage_result.failure_category.clone().zip(Some(stage_result.output.to_string()));
    if let Some(category) =
        stage_result.failure_category.clone().filter(|value| !is_retryable_failure(value))
    {
        return mark_non_retryable_failure(ctx, state, &failure_context, &category).await;
    }
    schedule_or_fail_retry(ctx, state, &failure_context).await
}

async fn mark_non_retryable_failure(
    ctx: &WorkflowContext<'_>,
    state: &mut PipelineState,
    failure: &FailureTransitionContext<'_>,
    category: &FailureCategory,
) -> Result<bool, OyaError> {
    state.orchestrator.status = "failed".to_string();
    state.orchestrator.updated_at = failure.fail_ts.to_string();
    write_orchestrator_state(ctx, &state.orchestrator)?;
    let entry = FailureEntry {
        attempt: state.attempt,
        category: format!("{:?}", category),
        message: truncate_clean(&failure.stage_result.output.to_string(), 500),
        retry_scheduled: false,
        at: parse_rfc3339_deterministic(failure.fail_ts),
    };
    append_failure_entries(ctx, state, failure, &entry).await?;
    Ok(true)
}

async fn schedule_or_fail_retry(
    ctx: &WorkflowContext<'_>,
    state: &mut PipelineState,
    failure: &FailureTransitionContext<'_>,
) -> Result<bool, OyaError> {
    let message = truncate_clean(&failure.stage_result.output.to_string(), 500);
    let fail_at = parse_rfc3339_deterministic(failure.fail_ts);
    state.attempt += 1;
    if state.attempt > state.current_stage.max_attempts() {
        state.orchestrator.status = "failed".to_string();
        state.orchestrator.updated_at = failure.fail_ts.to_string();
        write_orchestrator_state(ctx, &state.orchestrator)?;
        let entry = FailureEntry {
            attempt: state.attempt,
            category: "max_attempts_exceeded".to_string(),
            message,
            retry_scheduled: false,
            at: fail_at,
        };
        append_failure_entries(ctx, state, failure, &entry).await?;
        return Ok(true);
    }
    let entry = FailureEntry {
        attempt: state.attempt - 1,
        category: failure
            .stage_result
            .failure_category
            .as_ref()
            .map_or_else(|| "unknown".to_string(), |value| format!("{:?}", value)),
        message,
        retry_scheduled: true,
        at: fail_at,
    };
    append_failure_entries(ctx, state, failure, &entry).await?;
    Ok(false)
}

async fn append_failure_entries(
    ctx: &WorkflowContext<'_>,
    state: &PipelineState,
    failure: &FailureTransitionContext<'_>,
    entry: &FailureEntry,
) -> Result<(), OyaError> {
    append_timeline(
        ctx,
        TimelineEntry::StageFailed {
            stage: state.current_stage.as_str().to_string(),
            attempt: entry.attempt,
            workspace: failure.workspace_info.as_ref().map(|w| w.workspace.clone()),
            duration_ms: failure.artifacts.stage_duration_ms,
            category: entry.category.clone(),
            message: entry.message.clone(),
            retry_scheduled: entry.retry_scheduled,
            at: entry.at,
        },
    )
    .await?;
    if !entry.retry_scheduled {
        append_timeline(
            ctx,
            TimelineEntry::RunFailed {
                stage: state.current_stage.as_str().to_string(),
                category: entry.category.clone(),
                at: entry.at,
            },
        )
        .await?;
    }
    Ok(())
}
