//! Pipeline state management for Restate workflow.
//!
//! This module provides the minimal state needed for pipeline execution
//! and recovery. Stage execution and artifact persistence is handled by
//! the executor module.

use oya::types::{StageFailure, StageName as Stage};
use restate_sdk::prelude::*;
use std::collections::HashMap;

use crate::orchestrator_types::{write_orchestrator_state, OrchestratorState};
use oya::usage::tier_for_stage;

use super::OyaError;

// ── Stable helpers ─────────────────────────────────────────────────────

/// Get an RFC3339 timestamp recorded through workflow journaling.
pub(crate) async fn workflow_timestamp(ctx: &WorkflowContext<'_>) -> Result<String, TerminalError> {
    ctx.run(|| async move { Ok::<_, HandlerError>(chrono::Utc::now().to_rfc3339()) }).await
}

/// Parse an RFC3339 timestamp string into a DateTime<Utc>, falling back to UNIX_EPOCH.
pub(crate) fn parse_rfc3339_stable(s: &str) -> chrono::DateTime<chrono::Utc> {
    chrono::DateTime::parse_from_rfc3339(s)
        .map(|dt| dt.with_timezone(&chrono::Utc))
        .unwrap_or_else(|_| chrono::DateTime::UNIX_EPOCH)
}

/// Read an environment variable reliably within a Restate workflow context.
pub(crate) async fn stable_env_var(
    ctx: &WorkflowContext<'_>,
    key: &str,
) -> Result<Option<String>, TerminalError> {
    let key = key.to_string();
    ctx.run(move || async move { Ok::<_, HandlerError>(std::env::var(&key).ok()) }).await
}

/// Check if an environment variable is set to a truthy value (1 or true).
pub(crate) async fn stable_env_bool(
    ctx: &WorkflowContext<'_>,
    key: &str,
) -> Result<bool, TerminalError> {
    let value = stable_env_var(ctx, key).await?;
    Ok(value.is_some_and(|v| v == "1" || v.eq_ignore_ascii_case("true")))
}

pub(crate) fn timestamp_error() -> OyaError {
    OyaError("timestamp error".to_string())
}

pub(crate) async fn workflow_timestamp_or_error(
    ctx: &WorkflowContext<'_>,
) -> Result<String, OyaError> {
    workflow_timestamp(ctx).await.map_err(|_error| timestamp_error())
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
    pub(crate) resolved_models: HashMap<String, String>,
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
    let updated_at = workflow_timestamp_or_error(ctx).await?;
    let stage = Stage::Explore;
    let tier = tier_for_stage(&stage).to_string();
    let mut resolved_models = HashMap::new();
    resolved_models.insert(tier, model.clone());
    let orchestrator = OrchestratorState {
        status: "running".to_string(),
        stage: stage.as_str().to_string(),
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
        current_stage: stage,
        attempt: 1,
        red_seal_ready: false,
        last_failure: None,
        resolved_models,
        orchestrator,
    })
}
