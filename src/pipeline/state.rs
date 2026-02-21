//! Pipeline state management for Restate workflow.
//!
//! This module provides the minimal state needed for pipeline execution
//! and recovery. Stage execution and artifact persistence is handled by
//! the executor module.

use oya::types::{StageFailure, StageName as Stage};
use restate_sdk::prelude::*;

use crate::orchestrator_types::{write_orchestrator_state, OrchestratorState};

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
    pub(crate) red_seal_ready: bool,
    pub(crate) last_failure: Option<StageFailure>,
    pub(crate) orchestrator: OrchestratorState,
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
        stage: Stage::Explore.as_str().to_string(),
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
    Ok(PipelineState {
        current_stage: Stage::Explore,
        attempt: 1,
        red_seal_ready: false,
        last_failure: None,
        orchestrator,
    })
}
