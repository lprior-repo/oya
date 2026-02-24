#![deny(clippy::unwrap_used)]
#![deny(clippy::expect_used)]
#![deny(clippy::panic)]
#![warn(clippy::pedantic)]
#![warn(clippy::nursery)]
#![forbid(unsafe_code)]

//! Pipeline state management for Restate workflow.
//!
//! This module provides the minimal state needed for pipeline execution
//! and recovery. Stage execution and artifact persistence is handled by
//! the executor module.

use oya::types::{ModelId, StageFailure, StageName as Stage};
use restate_sdk::prelude::*;

use super::OyaError;
use crate::orchestrator_types::{write_orchestrator_state, OrchestratorState};

// ── Stable helpers ─────────────────────────────────────────────────────

/// Get an RFC3339 timestamp recorded through workflow journaling.
pub async fn workflow_timestamp(ctx: &WorkflowContext<'_>) -> Result<String, TerminalError> {
    ctx.run(|| async move { Ok::<_, HandlerError>(chrono::Utc::now().to_rfc3339()) }).await
}

/// Parse an RFC3339 timestamp string into a `DateTime<Utc>`, falling back to `UNIX_EPOCH`.
#[must_use]
pub fn parse_rfc3339_stable(s: &str) -> chrono::DateTime<chrono::Utc> {
    chrono::DateTime::parse_from_rfc3339(s)
        .map(|dt| dt.with_timezone(&chrono::Utc))
        .unwrap_or(chrono::DateTime::UNIX_EPOCH)
}

/// Read an environment variable reliably within a Restate workflow context.
pub async fn stable_env_var(
    ctx: &WorkflowContext<'_>,
    key: &str,
) -> Result<Option<String>, TerminalError> {
    let key = key.to_string();
    ctx.run(move || async move { Ok::<_, HandlerError>(std::env::var(&key).ok()) }).await
}

#[must_use]
pub fn timestamp_error() -> OyaError {
    OyaError("timestamp error".to_string())
}

pub async fn workflow_timestamp_or_error(ctx: &WorkflowContext<'_>) -> Result<String, OyaError> {
    workflow_timestamp(ctx).await.map_err(|_error| timestamp_error())
}

// ── Data types ────────────────────────────────────────────────────────────────

#[derive(Clone)]
pub struct PipelineRunInput {
    pub run_id: String,
    pub bead_id: String,
    pub context: String,
}

pub struct PipelineState {
    pub current_stage: Stage,
    pub attempt: u32,
    pub last_failure: Option<StageFailure>,
    pub orchestrator: OrchestratorState,
}

// ── Constructor helpers ───────────────────────────────────────────────────────

pub const fn pipeline_input(run_id: String, bead_id: String, context: String) -> PipelineRunInput {
    PipelineRunInput { run_id, bead_id, context }
}

pub async fn init_pipeline_state(
    ctx: &WorkflowContext<'_>,
    input: &PipelineRunInput,
    model: ModelId,
    updated_at: String,
) -> Result<PipelineState, OyaError> {
    let stage = Stage::JjWorkspace;
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
    Ok(PipelineState { current_stage: stage, attempt: 1, last_failure: None, orchestrator })
}
