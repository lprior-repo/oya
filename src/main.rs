#![deny(clippy::unwrap_used)]
#![deny(clippy::expect_used)]
#![deny(clippy::panic)]
#![deny(clippy::too_many_lines)]
#![deny(clippy::too_many_arguments)]
#![forbid(unsafe_code)]

use oya::beads::moon_command::generate_moon_command;
use oya::config;
use oya::types::{
    truncate_clean, FailureCategory, Gate, GateSummary, StageName as Stage, StageResult,
    TimelineEntry,
};
use oya::usage::{OyaUsageTracker, OyaUsageTrackerClient, OyaUsageTrackerImpl};
use oya::{
    build_opencode_poll_snapshot, build_zjj_workspace_name, is_retryable_failure,
    parse_opencode_sse_events,
};
use reqwest::blocking as blocking_reqwest;
use restate_sdk::endpoint::Endpoint;
use restate_sdk::prelude::*;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::process::Command;

use clap::{Parser, Subcommand};

mod ops_poller;
mod workflow_runner;

/// Application-level error type used by orchestration and monitor handlers.
#[derive(Debug)]
pub struct OyaError(String);

impl std::fmt::Display for OyaError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl std::error::Error for OyaError {}

impl From<TerminalError> for OyaError {
    fn from(err: TerminalError) -> Self {
        OyaError(format!("terminal error: {}", err))
    }
}

#[restate_sdk::workflow]
pub trait OyaOrchestrator {
    async fn start(request: Json<serde_json::Value>) -> Result<String, HandlerError>;
}

#[restate_sdk::service]
pub trait OyaOpsMonitor {
    async fn poll_status() -> Result<Json<OpsMonitorPollResponse>, HandlerError>;
    async fn poll_events(
        request: Json<OpsMonitorEventRequest>,
    ) -> Result<Json<OpsMonitorEventResponse>, HandlerError>;
}

#[derive(Debug, Deserialize)]
struct StartRequestPayload {
    bead_id: Option<String>,
    context: Option<String>,
    model: Option<String>,
}

#[derive(Debug, Deserialize)]
/// Request body for polling OpenCode event stream snapshots.
pub struct OpsMonitorEventRequest {
    /// Maximum number of events to return in one poll.
    max_events: Option<usize>,
    /// Long-poll timeout in seconds for the event endpoint.
    timeout_seconds: Option<u64>,
}

#[derive(Debug, Serialize)]
/// Aggregated OpenCode status counters at one observation timestamp.
pub struct OpsMonitorPollResponse {
    source: String,
    observed_at: String,
    busy_sessions: Vec<String>,
    pending_permissions: usize,
    pending_questions: usize,
}

#[derive(Debug, Serialize)]
/// One raw OpenCode SSE event plus optional parsed JSON payload.
pub struct OpsMonitorEventEnvelope {
    raw: String,
    parsed: Option<serde_json::Value>,
}

#[derive(Debug, Serialize)]
/// Event polling response with bounded event payloads and timing metadata.
pub struct OpsMonitorEventResponse {
    source: String,
    observed_at: String,
    events: Vec<OpsMonitorEventEnvelope>,
    count: usize,
    timeout_seconds: u64,
}

#[derive(Debug, Serialize)]
struct WorkspaceLifecycleEvent {
    workspace: String,
    queue_command: String,
    queue_passed: bool,
    queue_exit_code: i32,
    queue_output: String,
    add_command: String,
    add_passed: bool,
    add_exit_code: i32,
    add_output: String,
    recorded_at: String,
}

#[derive(Debug, Clone, Serialize)]
struct OrchestratorState {
    status: String,
    stage: String,
    attempt: u32,
    bead_id: String,
    context: String,
    model: String,
    last_failure: String,
    last_output: String,
    last_prompt: String,
    updated_at: String,
}

#[derive(Debug, Serialize)]
struct RunRequestEvent {
    run_id: String,
    bead_id: String,
    context: String,
    started_at: String,
}

#[derive(Debug, Serialize)]
struct FailureSnapshot {
    category: String,
    message: String,
}

#[derive(Debug, Serialize)]
struct StageInputEvent {
    run_id: String,
    bead_id: String,
    stage: String,
    attempt: u32,
    context: String,
    last_failure: Option<FailureSnapshot>,
    started_at: String,
}

#[derive(Debug, Serialize)]
struct StageResultEvent {
    passed: bool,
    failure_category: Option<String>,
    next_stage: Option<String>,
    output: String,
}

#[derive(Debug, Serialize)]
struct SkillOutputEvent {
    success: bool,
    exit_code: i32,
    full_log: String,
    feedback: String,
    contract_document: Option<String>,
    implementation_code: Option<String>,
    test_results: Option<String>,
    adversarial_report: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
struct GateEventSummary {
    gate: String,
    state_key: String,
    artifact_id: String,
    passed: bool,
    exit_code: i32,
}

#[derive(Debug, Serialize)]
struct StageEnvelopeEvent {
    run_id: String,
    bead_id: String,
    stage: String,
    attempt: u32,
    status: String,
    input_key: String,
    prompt_key: String,
    result_key: String,
    skill_output_key: String,
    gate_events: Vec<GateEventSummary>,
    recorded_at: String,
}

fn parse_start_request(request: serde_json::Value) -> Result<StartRequestPayload, TerminalError> {
    match request {
        serde_json::Value::Object(_) => serde_json::from_value(request)
            .map_err(|e| TerminalError::new_with_code(400, format!("Invalid JSON body: {}", e))),
        serde_json::Value::String(raw) => serde_json::from_str::<StartRequestPayload>(&raw)
            .map_err(|e| {
                TerminalError::new_with_code(400, format!("Invalid JSON string body: {}", e))
            }),
        other => Err(TerminalError::new_with_code(
            400,
            format!("Invalid request payload type: expected object or JSON string, got {}", other),
        )),
    }
}

/// Workflow service implementation for orchestrator runs.
pub struct OyaOrchestratorImpl;
/// Service implementation for OpenCode operational monitoring endpoints.
pub struct OyaOpsMonitorImpl;

impl OyaOrchestrator for OyaOrchestratorImpl {
    async fn start(
        &self,
        ctx: WorkflowContext<'_>,
        request: Json<serde_json::Value>,
    ) -> Result<String, HandlerError> {
        let parsed = parse_start_request(request.0)?;
        let start = build_start_context(&ctx, parsed).await?;
        persist_run_start(&ctx, &start).await?;
        tracing::info!("=== RUN {} STARTED ===", start.run_id);
        tracing::info!("Bead: {}", start.bead_id);
        tracing::info!("Context: {}", start.context);
        tracing::info!("Model: {}", start.model);
        run_pipeline(&ctx, start.run_id.clone(), start.bead_id, start.context, start.model).await?;
        Ok(start.run_id)
    }
}

struct StartContext {
    run_id: String,
    bead_id: String,
    context: String,
    model: String,
    started_at: String,
}

async fn build_start_context(
    ctx: &WorkflowContext<'_>,
    parsed: StartRequestPayload,
) -> Result<StartContext, OyaError> {
    let bead_id = parsed.bead_id.map_or_else(|| "unknown".to_string(), std::convert::identity);
    let context = parsed.context.map_or_else(String::new, std::convert::identity);
    let model =
        parsed.model.map_or_else(|| "zai-coding-plan/glm-5".to_string(), std::convert::identity);
    let started_at = deterministic_timestamp(ctx).await?;
    Ok(StartContext { run_id: ctx.key().to_string(), bead_id, context, model, started_at })
}

async fn persist_run_start(
    ctx: &WorkflowContext<'_>,
    start: &StartContext,
) -> Result<(), OyaError> {
    write_orchestrator_state(
        ctx,
        &OrchestratorState {
            status: "running".to_string(),
            stage: "plan".to_string(),
            attempt: 1,
            bead_id: start.bead_id.clone(),
            context: start.context.clone(),
            model: start.model.clone(),
            last_failure: String::new(),
            last_output: String::new(),
            last_prompt: String::new(),
            updated_at: start.started_at.clone(),
        },
    )?;
    set_json_state(
        ctx,
        "run_request",
        &RunRequestEvent {
            run_id: start.run_id.clone(),
            bead_id: start.bead_id.clone(),
            context: start.context.clone(),
            started_at: start.started_at.clone(),
        },
    )?;
    append_timeline(
        ctx,
        TimelineEntry::RunStarted {
            bead_id: start.bead_id.clone(),
            context: start.context.clone(),
            at: parse_rfc3339_deterministic(&start.started_at),
        },
    )
    .await
}

impl OyaOpsMonitor for OyaOpsMonitorImpl {
    async fn poll_status(
        &self,
        _ctx: Context<'_>,
    ) -> Result<Json<OpsMonitorPollResponse>, HandlerError> {
        let config = opencode_config()?;
        let session_status = fetch_opencode_text(&config, "/session/status", 10).await?;
        let permission = fetch_opencode_text(&config, "/permission", 10).await?;
        let question = fetch_opencode_text(&config, "/question", 10).await?;

        let snapshot = build_opencode_poll_snapshot(
            session_status.as_str(),
            permission.as_str(),
            question.as_str(),
        )
        .map_err(|error| OyaError(format!("OpenCode snapshot parse failed: {}", error)))?;

        Ok(Json(OpsMonitorPollResponse {
            source: config.base_url,
            observed_at: chrono::Utc::now().to_rfc3339(),
            busy_sessions: snapshot.busy_sessions,
            pending_permissions: snapshot.pending_permissions,
            pending_questions: snapshot.pending_questions,
        }))
    }

    async fn poll_events(
        &self,
        _ctx: Context<'_>,
        request: Json<OpsMonitorEventRequest>,
    ) -> Result<Json<OpsMonitorEventResponse>, HandlerError> {
        let config = opencode_config()?;
        let max_events = request.0.max_events.map_or(20, |value| value.clamp(1, 200));
        let timeout_seconds = request.0.timeout_seconds.map_or(15, |value| value.clamp(1, 30));
        let raw = fetch_opencode_text(&config, "/event", timeout_seconds).await?;
        let payloads = parse_opencode_sse_events(raw.as_str(), max_events)
            .map_err(|error| OyaError(format!("OpenCode event parse failed: {}", error)))?;

        let events = payloads
            .iter()
            .map(|payload| OpsMonitorEventEnvelope {
                raw: payload.clone(),
                parsed: serde_json::from_str::<serde_json::Value>(payload.as_str()).ok(),
            })
            .collect::<Vec<_>>();

        Ok(Json(OpsMonitorEventResponse {
            source: config.base_url,
            observed_at: chrono::Utc::now().to_rfc3339(),
            count: events.len(),
            events,
            timeout_seconds,
        }))
    }
}

/// Get a deterministic RFC3339 timestamp within a Restate workflow context.
/// Wraps chrono::Utc::now() in ctx.run() for journal consistency on replay.
async fn deterministic_timestamp(ctx: &WorkflowContext<'_>) -> Result<String, TerminalError> {
    ctx.run(|| async move { Ok::<_, HandlerError>(chrono::Utc::now().to_rfc3339()) }).await
}

/// Parse an RFC3339 timestamp string into a DateTime<Utc>, falling back to UNIX_EPOCH.
/// This is deterministic: on parse failure, always returns 1970-01-01T00:00:00Z.
/// Use this instead of `.unwrap_or_else(|_| chrono::Utc::now())` in workflow contexts.
fn parse_rfc3339_deterministic(s: &str) -> chrono::DateTime<chrono::Utc> {
    chrono::DateTime::parse_from_rfc3339(s)
        .map(|dt| dt.with_timezone(&chrono::Utc))
        .unwrap_or_else(|_| chrono::DateTime::UNIX_EPOCH)
}

/// Read an environment variable deterministically within a Restate workflow context.
/// Wraps std::env::var() in ctx.run() for journal consistency on replay.
async fn deterministic_env_var(
    ctx: &WorkflowContext<'_>,
    key: &str,
) -> Result<Option<String>, TerminalError> {
    let key = key.to_string();
    ctx.run(move || async move { Ok::<_, HandlerError>(std::env::var(&key).ok()) }).await
}

/// Check if an environment variable is set to a truthy value (1 or true).
/// Deterministic via ctx.run() for Restate workflow replay consistency.
async fn deterministic_env_bool(
    ctx: &WorkflowContext<'_>,
    key: &str,
) -> Result<bool, TerminalError> {
    let value = deterministic_env_var(ctx, key).await?;
    Ok(value.is_some_and(|v| v == "1" || v.eq_ignore_ascii_case("true")))
}

/// Runtime configuration read deterministically from environment at workflow start.
/// All non-deterministic env access is centralized here and journaled via ctx.run().
struct RuntimeConfig {
    workspace_policy: WorkspacePreparationPolicy,
    merge_queue_policy: MergeQueuePolicy,
    repo_root: PathBuf,
}

#[derive(Clone, Copy)]
enum WorkspacePreparationPolicy {
    Prepare,
    Skip,
}

impl WorkspacePreparationPolicy {
    fn from_skip_flag(skip: bool) -> Self {
        if skip {
            Self::Skip
        } else {
            Self::Prepare
        }
    }

    fn should_skip(self) -> bool {
        matches!(self, Self::Skip)
    }
}

#[derive(Clone, Copy)]
enum MergeQueuePolicy {
    Enforce,
    Skip,
}

impl MergeQueuePolicy {
    fn from_skip_flag(skip: bool) -> Self {
        if skip {
            Self::Skip
        } else {
            Self::Enforce
        }
    }

    fn should_run(self, gate: &Gate) -> bool {
        !(matches!(self, Self::Skip) && *gate == Gate::ZjjMergeQueue)
    }
}

impl RuntimeConfig {
    /// Read all configuration deterministically from environment.
    /// Must be called at the start of a workflow before any spawn_blocking.
    async fn load(ctx: &WorkflowContext<'_>) -> Result<Self, OyaError> {
        let skip_zjj_workspace = deterministic_env_bool(ctx, "OYA_SKIP_ZJJ_WORKSPACE")
            .await
            .map_err(|_e| OyaError("config error: OYA_SKIP_ZJJ_WORKSPACE".to_string()))?;

        let skip_zjj_gate = deterministic_env_bool(ctx, "OYA_SKIP_ZJJ_GATE")
            .await
            .map_err(|_e| OyaError("config error: OYA_SKIP_ZJJ_GATE".to_string()))?;

        let repo_root_str = Self::deterministic_repo_root(ctx)
            .await
            .map_err(|e| OyaError(format!("config error: repo_root: {}", e)))?;

        Ok(Self {
            workspace_policy: WorkspacePreparationPolicy::from_skip_flag(skip_zjj_workspace),
            merge_queue_policy: MergeQueuePolicy::from_skip_flag(skip_zjj_gate),
            repo_root: PathBuf::from(repo_root_str),
        })
    }

    async fn deterministic_repo_root(ctx: &WorkflowContext<'_>) -> Result<String, TerminalError> {
        ctx.run(|| async move {
            if let Ok(configured_root) = std::env::var("OYA_REPO_ROOT") {
                return Ok::<_, HandlerError>(configured_root);
            }
            std::env::current_dir()
                .map(|p| p.to_string_lossy().to_string())
                .map_err(|e| HandlerError::from(format!("Failed to resolve repo root: {}", e)))
        })
        .await
    }
}

#[derive(Clone)]
struct PipelineRunInput {
    run_id: String,
    bead_id: String,
    context: String,
}

struct PipelineState {
    current_stage: Stage,
    attempt: u32,
    last_failure: Option<(FailureCategory, String)>,
    orchestrator: OrchestratorState,
}

struct StageAttemptRecord {
    stage_input_key: String,
    workspace_info: Option<WorkspaceLifecycleEvent>,
}

struct StageArtifacts {
    stage_duration_ms: u64,
    event_at: chrono::DateTime<chrono::Utc>,
}

struct RecordStageOutputsInput<'a> {
    input: &'a PipelineRunInput,
    attempt_record: &'a StageAttemptRecord,
    stage_result: &'a StageResult,
    stage_prompt: &'a str,
    stage_started_at: chrono::DateTime<chrono::Utc>,
    repo_root: &'a PathBuf,
}

struct StageEnvelopeInput<'a> {
    input: &'a PipelineRunInput,
    attempt_record: &'a StageAttemptRecord,
    prompt_key: &'a str,
    stage_result_key: &'a str,
    skill_output_key: &'a str,
    gate_events: &'a [GateEventSummary],
    event_ts: &'a str,
    stage_result: &'a StageResult,
}

enum StageExecutionResult {
    Continue { stage_result: StageResult, stage_prompt: String },
    Stop,
}

fn timestamp_error() -> OyaError {
    OyaError("timestamp error".to_string())
}

async fn deterministic_timestamp_or_error(ctx: &WorkflowContext<'_>) -> Result<String, OyaError> {
    deterministic_timestamp(ctx).await.map_err(|_error| timestamp_error())
}

fn pipeline_input(run_id: String, bead_id: String, context: String) -> PipelineRunInput {
    PipelineRunInput { run_id, bead_id, context }
}

async fn init_pipeline_state(
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

async fn mark_stage_running(
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

async fn prepare_stage_attempt(
    ctx: &WorkflowContext<'_>,
    state: &PipelineState,
    input: &PipelineRunInput,
    config: &RuntimeConfig,
) -> Result<StageAttemptRecord, OyaError> {
    let stage_start = deterministic_timestamp_or_error(ctx).await?;
    let stage_input_key = stage_attempt_key(&state.current_stage, state.attempt, "input");
    let failure_snapshot = state.last_failure.as_ref().map(|(category, message)| FailureSnapshot {
        category: format!("{:?}", category),
        message: truncate_clean(message, 2000),
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
    config: &RuntimeConfig,
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

async fn execute_stage_with_tracker(
    ctx: &WorkflowContext<'_>,
    state: &mut PipelineState,
    input: &PipelineRunInput,
    config: &RuntimeConfig,
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
    match execute_stage_real(request, config).await {
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
    let report_req = oya::usage::ReportOutcomeRequest {
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

async fn record_stage_outputs(
    ctx: &WorkflowContext<'_>,
    state: &mut PipelineState,
    request: RecordStageOutputsInput<'_>,
) -> Result<StageArtifacts, OyaError> {
    update_orchestrator_after_stage(ctx, state, request.stage_result, request.stage_prompt).await?;
    let prompt_key = format!("prompt_{}_{}", state.current_stage.as_str(), state.attempt);
    ctx.set(&prompt_key, request.stage_prompt.to_string());
    let stage_result_key = stage_attempt_key(&state.current_stage, state.attempt, "result");
    set_stage_result_json(ctx, &stage_result_key, request.stage_result)?;
    let skill_output_key = stage_attempt_key(&state.current_stage, state.attempt, "skill_output");
    set_skill_output_json(ctx, &state.current_stage, &skill_output_key, request.stage_result)?;
    let gate_events = record_gate_events(ctx, state, request.repo_root)?;
    let event_ts = deterministic_timestamp_or_error(ctx).await?;
    set_stage_envelope(
        ctx,
        state,
        StageEnvelopeInput {
            input: request.input,
            attempt_record: request.attempt_record,
            prompt_key: &prompt_key,
            stage_result_key: &stage_result_key,
            skill_output_key: &skill_output_key,
            gate_events: &gate_events,
            event_ts: &event_ts,
            stage_result: request.stage_result,
        },
    )?;
    let duration = (chrono::Utc::now() - request.stage_started_at).num_milliseconds().max(0) as u64;
    Ok(StageArtifacts {
        stage_duration_ms: duration,
        event_at: parse_rfc3339_deterministic(&event_ts),
    })
}

async fn update_orchestrator_after_stage(
    ctx: &WorkflowContext<'_>,
    state: &mut PipelineState,
    stage_result: &StageResult,
    stage_prompt: &str,
) -> Result<(), OyaError> {
    state.orchestrator.last_prompt = stage_prompt.to_string();
    state.orchestrator.last_output = truncate_clean(&stage_result.output.to_string(), 6000);
    state.orchestrator.last_failure = if stage_result.passed {
        String::new()
    } else {
        truncate_clean(&stage_result.output.to_string(), 6000)
    };
    state.orchestrator.updated_at = deterministic_timestamp_or_error(ctx).await?;
    write_orchestrator_state(ctx, &state.orchestrator)
}

fn set_stage_result_json(
    ctx: &WorkflowContext<'_>,
    key: &str,
    stage_result: &StageResult,
) -> Result<(), OyaError> {
    set_json_state(
        ctx,
        key,
        &StageResultEvent {
            passed: stage_result.passed,
            failure_category: stage_result
                .failure_category
                .as_ref()
                .map(|value| format!("{:?}", value)),
            next_stage: stage_result.next_stage.as_ref().map(|value| value.as_str().to_string()),
            output: truncate_clean(&stage_result.output.to_string(), 6000),
        },
    )
}

fn set_skill_output_json(
    ctx: &WorkflowContext<'_>,
    stage: &Stage,
    key: &str,
    stage_result: &StageResult,
) -> Result<(), OyaError> {
    let stage_log = truncate_clean(&stage_result.output.to_string(), 12000);
    set_json_state(
        ctx,
        key,
        &SkillOutputEvent {
            success: stage_result.passed,
            exit_code: if stage_result.passed { 0 } else { 1 },
            full_log: stage_log.clone(),
            feedback: stage_result
                .failure_category
                .as_ref()
                .map_or(String::new(), |value| format!("{:?}", value)),
            contract_document: (stage == &Stage::Contract).then_some(stage_log.clone()),
            implementation_code: (stage == &Stage::Tdd15).then_some(stage_log.clone()),
            test_results: (stage == &Stage::Qa || stage == &Stage::RedQueen)
                .then_some(stage_log.clone()),
            adversarial_report: (stage == &Stage::RedQueen).then_some(stage_log),
        },
    )
}

fn record_gate_events(
    ctx: &WorkflowContext<'_>,
    state: &PipelineState,
    repo_root: &PathBuf,
) -> Result<Vec<GateEventSummary>, OyaError> {
    state
        .current_stage
        .gates()
        .into_iter()
        .map(|gate| record_single_gate_event(ctx, state, repo_root, gate))
        .collect()
}

fn record_single_gate_event(
    ctx: &WorkflowContext<'_>,
    state: &PipelineState,
    repo_root: &PathBuf,
    gate: Gate,
) -> Result<GateEventSummary, OyaError> {
    let gate_evidence = execute_gate(gate.clone(), repo_root)?;
    let gate_key =
        stage_attempt_key(&state.current_stage, state.attempt, &format!("gate_{}", gate.as_str()));
    set_json_state(
        ctx,
        &gate_key,
        &StageResultEvent {
            passed: gate_evidence.passed,
            failure_category: None,
            next_stage: None,
            output: format_gate_command_output(
                gate_evidence.command.as_str(),
                gate_evidence.exit_code,
                truncate_clean(&gate_evidence.output, 4000).as_str(),
            ),
        },
    )?;
    Ok(GateEventSummary {
        gate: gate.as_str().to_string(),
        state_key: gate_key,
        artifact_id: String::new(),
        passed: gate_evidence.passed,
        exit_code: gate_evidence.exit_code,
    })
}

fn set_stage_envelope(
    ctx: &WorkflowContext<'_>,
    state: &PipelineState,
    input: StageEnvelopeInput<'_>,
) -> Result<(), OyaError> {
    let stage_event_key = stage_attempt_key(&state.current_stage, state.attempt, "event");
    set_json_state(
        ctx,
        &stage_event_key,
        &StageEnvelopeEvent {
            run_id: input.input.run_id.clone(),
            bead_id: input.input.bead_id.clone(),
            stage: state.current_stage.as_str().to_string(),
            attempt: state.attempt,
            status: if input.stage_result.passed {
                "passed".to_string()
            } else {
                "failed".to_string()
            },
            input_key: input.attempt_record.stage_input_key.clone(),
            prompt_key: input.prompt_key.to_string(),
            result_key: input.stage_result_key.to_string(),
            skill_output_key: input.skill_output_key.to_string(),
            gate_events: input.gate_events.to_vec(),
            recorded_at: input.event_ts.to_string(),
        },
    )
}

async fn handle_stage_transition(
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

async fn run_pipeline(
    ctx: &WorkflowContext<'_>,
    run_id: String,
    bead_id: String,
    context: String,
    model: String,
) -> Result<(), OyaError> {
    let config = RuntimeConfig::load(ctx).await?;
    let input = pipeline_input(run_id, bead_id, context);
    let mut state = init_pipeline_state(ctx, &input, model).await?;
    run_pipeline_loop(ctx, &config, &input, &mut state).await
}

async fn run_pipeline_loop(
    ctx: &WorkflowContext<'_>,
    config: &RuntimeConfig,
    input: &PipelineRunInput,
    state: &mut PipelineState,
) -> Result<(), OyaError> {
    loop {
        mark_stage_running(ctx, state).await?;
        let attempt_record = prepare_stage_attempt(ctx, state, input, config).await?;
        let stage_started_at = chrono::Utc::now();
        let execution = execute_stage_with_tracker(ctx, state, input, config).await?;
        let StageExecutionResult::Continue { stage_result, stage_prompt } = execution else {
            return Ok(());
        };
        let artifacts = record_stage_outputs(
            ctx,
            state,
            RecordStageOutputsInput {
                input,
                attempt_record: &attempt_record,
                stage_result: &stage_result,
                stage_prompt: &stage_prompt,
                stage_started_at,
                repo_root: &config.repo_root,
            },
        )
        .await?;
        if handle_stage_transition(
            ctx,
            state,
            &stage_result,
            &attempt_record.workspace_info,
            &artifacts,
        )
        .await?
        {
            return Ok(());
        }
        tokio::time::sleep(std::time::Duration::from_millis(100)).await;
    }
}

// Re-export from lib for backwards compatibility
// is_retryable_failure imported from oya lib above

#[derive(Clone)]
struct StageExecutionRequest {
    run_id: String,
    bead_id: String,
    stage: Stage,
    attempt: u32,
    context: String,
    model: String,
    last_failure: Option<(FailureCategory, String)>,
}

#[derive(Clone)]
struct StageBlockingInput {
    request: StageExecutionRequest,
    merge_queue_policy: MergeQueuePolicy,
    repo_root: PathBuf,
}

async fn execute_stage_real(
    request: StageExecutionRequest,
    config: &RuntimeConfig,
) -> Result<(StageResult, String), OyaError> {
    if request.attempt == 0 {
        return Err(OyaError("attempt must be greater than 0".to_string()));
    }
    let input = StageBlockingInput {
        request: request.clone(),
        merge_queue_policy: config.merge_queue_policy,
        repo_root: config.repo_root.clone(),
    };
    let execution = tokio::task::spawn_blocking(move || execute_stage_blocking(input))
        .await
        .map_err(|error| OyaError(format!("spawn_blocking failed: {}", error)))??;
    let StageExecution { passed, output, failure_category, next_stage, prompt } = execution;
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

fn execute_stage_blocking(input: StageBlockingInput) -> Result<StageExecution, OyaError> {
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

struct StageExecution {
    passed: bool,
    output: String,
    failure_category: Option<FailureCategory>,
    next_stage: Option<Stage>,
    prompt: String,
}

struct PromptStageRequest {
    prompt: String,
    stage: Stage,
    success_message: &'static str,
    success_next_stage: Option<Stage>,
    repo_root: PathBuf,
    model: String,
}

fn execute_prompt_stage(request: PromptStageRequest) -> Result<StageExecution, OyaError> {
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

fn format_gate_command_output(command: &str, exit_code: i32, output: &str) -> String {
    format!("command={} exit_code={}\n{}", command, exit_code, output)
}

const OPENCODE_TIMEOUT_SECONDS: u64 = 300;
const MOON_TIMEOUT_SECONDS: u64 = 900;
const ZJJ_TIMEOUT_SECONDS: u64 = 60;

struct GateEvidence {
    command: String,
    passed: bool,
    exit_code: i32,
    output: String,
}

#[derive(Debug, Clone)]
struct OpenCodeConfig {
    base_url: String,
    password: Option<String>,
}

#[derive(Clone, Copy)]
enum OpenCodeEndpoint {
    SessionStatus,
    Permission,
    Question,
}

impl OpenCodeEndpoint {
    const fn path(self) -> &'static str {
        match self {
            Self::SessionStatus => "session/status",
            Self::Permission => "permission",
            Self::Question => "question",
        }
    }
}

fn opencode_endpoint_url(config: &OpenCodeConfig, endpoint: OpenCodeEndpoint) -> String {
    format!("{}/{}", config.base_url.trim_end_matches('/'), endpoint.path())
}

#[derive(Clone, Copy)]
struct HttpClientSettings {
    timeout_secs: u64,
    connect_timeout_secs: u64,
    pool_max_idle_per_host: usize,
    pool_idle_timeout_secs: u64,
    tcp_keepalive_secs: Option<u64>,
}

fn build_http_client(settings: HttpClientSettings) -> Result<reqwest::Client, reqwest::Error> {
    let mut builder = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(settings.timeout_secs))
        .connect_timeout(std::time::Duration::from_secs(settings.connect_timeout_secs))
        .pool_max_idle_per_host(settings.pool_max_idle_per_host)
        .pool_idle_timeout(std::time::Duration::from_secs(settings.pool_idle_timeout_secs));

    if let Some(tcp_keepalive_secs) = settings.tcp_keepalive_secs {
        builder = builder.tcp_keepalive(std::time::Duration::from_secs(tcp_keepalive_secs));
    }

    builder.build()
}

fn workflow_http_client_settings() -> HttpClientSettings {
    HttpClientSettings {
        timeout_secs: 30,
        connect_timeout_secs: 10,
        pool_max_idle_per_host: 10,
        pool_idle_timeout_secs: 60,
        tcp_keepalive_secs: Some(60),
    }
}

fn poller_http_client_settings() -> HttpClientSettings {
    HttpClientSettings {
        timeout_secs: 10,
        connect_timeout_secs: 5,
        pool_max_idle_per_host: 5,
        pool_idle_timeout_secs: 30,
        tcp_keepalive_secs: None,
    }
}

fn opencode_http_client_settings(timeout_seconds: u64) -> HttpClientSettings {
    HttpClientSettings {
        timeout_secs: timeout_seconds,
        connect_timeout_secs: 10,
        pool_max_idle_per_host: 10,
        pool_idle_timeout_secs: 60,
        tcp_keepalive_secs: Some(60),
    }
}

fn opencode_config() -> Result<OpenCodeConfig, OyaError> {
    let base_url = std::env::var("OYA_OPENCODE_BASE_URL")
        .ok()
        .filter(|value| !value.trim().is_empty())
        .unwrap_or_else(|| "http://127.0.0.1:4097".to_string());

    if !is_valid_http_url(base_url.as_str()) {
        return Err(OyaError(format!("Invalid OYA_OPENCODE_BASE_URL '{}'", base_url)));
    }

    let password =
        std::env::var("OYA_OPENCODE_PASSWORD").ok().filter(|value| !value.trim().is_empty());

    Ok(OpenCodeConfig { base_url, password })
}

async fn fetch_opencode_text(
    config: &OpenCodeConfig,
    path: &str,
    timeout_seconds: u64,
) -> Result<String, OyaError> {
    let client = build_http_client(opencode_http_client_settings(timeout_seconds))
        .map_err(|error| OyaError(format!("OpenCode HTTP client build failed: {}", error)))?;

    let url = format!("{}{}", config.base_url.trim_end_matches('/'), path);
    let request = config.password.as_ref().map_or_else(
        || client.get(url.clone()),
        |password| client.get(url.clone()).basic_auth("opencode", Some(password)),
    );

    let response = request
        .send()
        .await
        .map_err(|error| OyaError(format!("OpenCode request failed for {}: {}", path, error)))?;
    let status = response.status();
    let text = response.text().await.map_err(|error| {
        OyaError(format!("OpenCode response read failed for {}: {}", path, error))
    })?;

    if !status.is_success() {
        return Err(OyaError(format!(
            "OpenCode request failed for {} with status {}: {}",
            path,
            status.as_u16(),
            truncate_clean(text.as_str(), 4000)
        )));
    }

    Ok(text)
}

fn run_command_with_timeout(
    command_name: &str,
    args: &[&str],
    timeout_seconds: u64,
    repo_root: &PathBuf,
) -> Result<(bool, String), OyaError> {
    let (passed, stdout, stderr, _exit_code) =
        run_command_with_timeout_with_exit(command_name, args, timeout_seconds, repo_root)?;
    Ok((passed, combine_command_output(stdout, stderr)))
}

fn combine_command_output(stdout: String, stderr: String) -> String {
    format!("{}\n{}", stdout, stderr)
}

#[tracing::instrument(
    name = "cli_command",
    skip(repo_root),
    fields(
        command = %command_name,
        args = ?args,
        timeout_seconds = timeout_seconds,
        repo_root = %repo_root.display()
    )
)]
fn run_command_with_timeout_with_exit(
    command_name: &str,
    args: &[&str],
    timeout_seconds: u64,
    repo_root: &PathBuf,
) -> Result<(bool, String, String, i32), OyaError> {
    let start = std::time::Instant::now();
    let result = if has_timeout_command() {
        run_with_timeout_command(command_name, args, timeout_seconds, repo_root)?
    } else {
        run_with_spawn_fallback(command_name, args, timeout_seconds, repo_root)?
    };
    log_cli_command(CommandLog {
        command_name,
        args,
        timeout_seconds,
        duration_ms: start.elapsed().as_millis(),
        result: &result,
    });
    Ok(result)
}

fn has_timeout_command() -> bool {
    Command::new("which").arg("timeout").output().is_ok_and(|output| output.status.success())
}

fn run_with_timeout_command(
    command_name: &str,
    args: &[&str],
    timeout_seconds: u64,
    repo_root: &PathBuf,
) -> Result<(bool, String, String, i32), OyaError> {
    let output = Command::new("timeout")
        .arg(timeout_seconds.to_string())
        .arg(command_name)
        .args(args)
        .current_dir(repo_root)
        .output()
        .map_err(|error| {
            OyaError(format!("Failed to run {} with timeout: {}", command_name, error))
        })?;
    Ok(command_output_result(output))
}

fn run_with_spawn_fallback(
    command_name: &str,
    args: &[&str],
    timeout_seconds: u64,
    repo_root: &PathBuf,
) -> Result<(bool, String, String, i32), OyaError> {
    let child = Command::new(command_name)
        .args(args)
        .current_dir(repo_root)
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .spawn()
        .map_err(|error| spawn_error(command_name, error))?;
    wait_for_child(child, timeout_seconds)
}

fn spawn_error(command_name: &str, error: std::io::Error) -> OyaError {
    if error.kind() == std::io::ErrorKind::NotFound {
        OyaError(format!(
            "Command '{}' not found. Please ensure it is installed and in PATH.",
            command_name
        ))
    } else {
        OyaError(format!("Failed to spawn {}: {}", command_name, error))
    }
}

fn wait_for_child(
    mut child: std::process::Child,
    timeout_seconds: u64,
) -> Result<(bool, String, String, i32), OyaError> {
    let timeout = std::time::Duration::from_secs(timeout_seconds);
    let start_wait = std::time::Instant::now();
    loop {
        if start_wait.elapsed() > timeout {
            let _kill = child.kill();
            let _wait = child.wait();
            return Ok((
                false,
                String::new(),
                format!("Command timed out after {} seconds", timeout_seconds),
                124,
            ));
        }
        if let Some(result) = child_wait_result(&mut child)? {
            return Ok(result);
        }
        std::thread::sleep(std::time::Duration::from_millis(100));
    }
}

fn child_wait_result(
    child: &mut std::process::Child,
) -> Result<Option<(bool, String, String, i32)>, OyaError> {
    match child.try_wait() {
        Ok(Some(status)) => {
            let exit_code = status.code().map_or(128, std::convert::identity);
            let stdout = read_child_stdout(child.stdout.take());
            let stderr = read_child_stderr(child.stderr.take());
            Ok(Some((status.success(), stdout, stderr, exit_code)))
        }
        Ok(None) => Ok(None),
        Err(error) => Err(OyaError(format!("Failed to wait for process: {}", error))),
    }
}

fn read_child_stdout(pipe: Option<std::process::ChildStdout>) -> String {
    pipe.map_or_else(String::new, read_pipe_to_string)
}

fn read_child_stderr(pipe: Option<std::process::ChildStderr>) -> String {
    pipe.map_or_else(String::new, read_pipe_to_string)
}

fn read_pipe_to_string<T: std::io::Read>(mut stream: T) -> String {
    let mut buffer = String::new();
    let _read = std::io::Read::read_to_string(&mut stream, &mut buffer);
    buffer
}

fn command_output_result(output: std::process::Output) -> (bool, String, String, i32) {
    let stdout = String::from_utf8_lossy(&output.stdout).to_string();
    let stderr = String::from_utf8_lossy(&output.stderr).to_string();
    let exit_code = output.status.code().map_or(128, std::convert::identity);
    (output.status.success(), stdout, stderr, exit_code)
}

struct CommandLog<'a> {
    command_name: &'a str,
    args: &'a [&'a str],
    timeout_seconds: u64,
    duration_ms: u128,
    result: &'a (bool, String, String, i32),
}

fn log_cli_command(command: CommandLog<'_>) {
    let (success, stdout, stderr, exit_code) = command.result;
    tracing::debug!(stdout = %stdout, stderr = %stderr, "CLI command detailed output");
    tracing::info!(
        command = %command.command_name,
        args = ?command.args,
        timeout_seconds = command.timeout_seconds,
        exit_code = *exit_code,
        duration_ms = command.duration_ms,
        stdout_len = stdout.len(),
        stderr_len = stderr.len(),
        timed_out = *exit_code == 124,
        success = *success,
        "CLI command execution"
    );
}

fn run_opencode(
    prompt: &str,
    repo_root: &PathBuf,
    model: &str,
) -> Result<(bool, String), OyaError> {
    tracing::info!("Running opencode with prompt ({} chars) model={}", prompt.len(), model);
    let opencode_command =
        std::env::var("OPENCODE_PATH").unwrap_or_else(|_| "opencode".to_string());
    match run_command_with_timeout(
        &opencode_command,
        &["run", "--format", "json", "--model", model, prompt],
        OPENCODE_TIMEOUT_SECONDS,
        repo_root,
    ) {
        Ok(res) => Ok(res),
        Err(err) => {
            // If opencode binary is not found, attempt HTTP fallback to OpenCode API
            let msg = err.to_string();
            if msg.contains("not found") || msg.contains("not found.") {
                tracing::warn!("opencode CLI not found on PATH, attempting HTTP fallback: {}", msg);
                match opencode_config() {
                    Ok(config) => run_opencode_via_http_blocking(&config, prompt, model),
                    Err(cfg_err) => Err(OyaError(format!(
                        "opencode CLI missing and opencode HTTP config invalid: {} / {}",
                        msg, cfg_err
                    ))),
                }
            } else {
                Err(err)
            }
        }
    }
}

fn run_opencode_via_http_blocking(
    config: &OpenCodeConfig,
    prompt: &str,
    model: &str,
) -> Result<(bool, String), OyaError> {
    let settings = opencode_http_client_settings(OPENCODE_TIMEOUT_SECONDS);
    let builder = blocking_reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(settings.timeout_secs))
        .connect_timeout(std::time::Duration::from_secs(settings.connect_timeout_secs));

    // other builder settings (pool, keepalive) are not available on blocking client in the same way
    let client = builder
        .build()
        .map_err(|e| OyaError(format!("Failed to build blocking HTTP client: {}", e)))?;

    let url = format!("{}/run", config.base_url.trim_end_matches('/'));
    let payload = serde_json::json!({ "model": model, "prompt": prompt, "format": "json" });

    let request = config.password.as_ref().map_or_else(
        || client.post(&url).json(&payload),
        |pwd| client.post(&url).basic_auth("opencode", Some(pwd)).json(&payload),
    );

    let response = request
        .send()
        .map_err(|e| OyaError(format!("OpenCode HTTP request failed for /run: {}", e)))?;
    let status = response.status();
    let text = response
        .text()
        .map_err(|e| OyaError(format!("OpenCode /run response read failed: {}", e)))?;

    if !status.is_success() {
        return Err(OyaError(format!(
            "OpenCode /run failed with status {}: {}",
            status.as_u16(),
            truncate_clean(text.as_str(), 4000)
        )));
    }

    match oya::parse_opencode_output(text.as_str()) {
        Ok(output) => Ok((true, output.stdout)),
        Err(parse_err) => {
            Err(OyaError(format!("OpenCode /run returned invalid output: {}", parse_err)))
        }
    }
}

fn execute_gate(gate: Gate, repo_root: &PathBuf) -> Result<GateEvidence, OyaError> {
    let command = generate_moon_command(&gate).command;
    let timeout_seconds = match gate {
        Gate::ZjjMergeQueue => ZJJ_TIMEOUT_SECONDS,
        _ => MOON_TIMEOUT_SECONDS,
    };

    let parsed_command = parse_gate_command(command.as_str())?;
    let (program, args) = parsed_command.command_parts();
    let arg_refs: Vec<&str> = args.iter().map(String::as_str).collect();

    let (passed, stdout, stderr, exit_code) = run_command_with_timeout_with_exit(
        program.as_str(),
        &arg_refs,
        timeout_seconds,
        repo_root,
    )?;
    let output = combine_command_output(stdout, stderr);

    Ok(GateEvidence { command, passed, exit_code, output })
}

enum GateCommand {
    Moon { task: MoonTask, passthrough: Vec<String> },
    ZjjSyncStatus,
}

struct ParsedCommandParts {
    program: String,
    args: Vec<String>,
}

#[derive(Clone, Copy)]
enum MoonTask {
    Check,
    Test,
    Clippy,
    Security,
    Ci,
}

fn parse_gate_command(command: &str) -> Result<GateCommand, OyaError> {
    let parsed = parse_command_parts(command)?;
    parse_gate_command_parts(parsed)
}

fn parse_gate_command_parts(command: ParsedCommandParts) -> Result<GateCommand, OyaError> {
    match (command.program.as_str(), command.args.as_slice()) {
        ("moon", moon_args) => parse_moon_gate_command(moon_args),
        ("zjj", zjj_args) if zjj_args == ["sync", "--status"] => Ok(GateCommand::ZjjSyncStatus),
        _ => Err(OyaError(format!(
            "unsupported gate command: {} {}",
            command.program,
            command.args.join(" ")
        ))),
    }
}

fn parse_moon_gate_command(args: &[String]) -> Result<GateCommand, OyaError> {
    let (task, passthrough) = match args {
        [run, task_name, rest @ ..] if run == "run" => {
            MoonTask::from_task_name(task_name).map(|task| (task, rest.to_vec())).ok_or_else(
                || OyaError(format!("unsupported moon gate command args: {}", args.join(" "))),
            )?
        }
        _ => {
            return Err(OyaError(format!(
                "unsupported moon gate command args: {}",
                args.join(" ")
            )));
        }
    };

    Ok(GateCommand::Moon { task, passthrough })
}

impl GateCommand {
    fn command_parts(self) -> (String, Vec<String>) {
        match self {
            GateCommand::Moon { task, passthrough } => {
                let mut args = vec!["run".to_string(), task.as_task_name().to_string()];
                args.extend(passthrough);
                ("moon".to_string(), args)
            }
            GateCommand::ZjjSyncStatus => {
                ("zjj".to_string(), vec!["sync".to_string(), "--status".to_string()])
            }
        }
    }
}

impl MoonTask {
    fn from_task_name(value: &str) -> Option<Self> {
        match value {
            ":check" => Some(Self::Check),
            ":test" => Some(Self::Test),
            ":clippy" => Some(Self::Clippy),
            ":security" => Some(Self::Security),
            ":ci" => Some(Self::Ci),
            _ => None,
        }
    }

    fn as_task_name(&self) -> &'static str {
        match self {
            MoonTask::Check => ":check",
            MoonTask::Test => ":test",
            MoonTask::Clippy => ":clippy",
            MoonTask::Security => ":security",
            MoonTask::Ci => ":ci",
        }
    }
}

fn parse_command_parts(command: &str) -> Result<ParsedCommandParts, OyaError> {
    let parts: Vec<String> = command.split_whitespace().map(str::to_string).collect();
    if parts.is_empty() {
        return Err(OyaError("gate command cannot be empty".to_string()));
    }

    let program = parts
        .first()
        .cloned()
        .ok_or_else(|| OyaError("gate command program missing".to_string()))?;
    let args = parts.iter().skip(1).cloned().collect();

    Ok(ParsedCommandParts { program, args })
}

fn gate_failure_outcome(stage: &Stage, gate: &Gate) -> (FailureCategory, Stage) {
    gate_failure_mapping(stage, gate).unwrap_or_else(|| default_gate_failure_outcome(stage))
}

fn default_gate_failure_outcome(stage: &Stage) -> (FailureCategory, Stage) {
    (FailureCategory::TestFailed, stage.clone())
}

fn gate_failure_mapping(stage: &Stage, gate: &Gate) -> Option<(FailureCategory, Stage)> {
    match (stage, gate) {
        (&Stage::Plan, &Gate::Compiles) => Some((FailureCategory::CompileFailed, Stage::Plan)),
        (&Stage::Contract, &Gate::Compiles) => {
            Some((FailureCategory::CompileFailed, Stage::Contract))
        }
        (&Stage::Tdd15, &Gate::Compiles) => Some((FailureCategory::CompileFailed, Stage::Tdd15)),
        (&Stage::Tdd15, &Gate::TestsPass) => Some((FailureCategory::TestFailed, Stage::Tdd15)),
        (&Stage::Qa, &Gate::TestsPass) | (&Stage::Qa, &Gate::EdgeCases) => {
            Some((FailureCategory::TestFailed, Stage::Tdd15))
        }
        (&Stage::RedQueen, &Gate::NoVulnerabilities) => {
            Some((FailureCategory::TestFailed, Stage::Tdd15))
        }
        (&Stage::GptReview, &Gate::ClippyClean) => {
            Some((FailureCategory::LintFailed, Stage::GptReview))
        }
        (&Stage::GptReview, &Gate::Security) => Some((FailureCategory::TestFailed, Stage::Tdd15)),
        (&Stage::ShipGate, &Gate::MoonCi) => Some((FailureCategory::TestFailed, Stage::Tdd15)),
        (&Stage::ShipGate, &Gate::ZjjMergeQueue) => {
            Some((FailureCategory::MergeConflict, Stage::GptReview))
        }
        _ => None,
    }
}

fn stage_uses_workspace(stage: &Stage) -> bool {
    matches!(
        stage,
        Stage::Contract
            | Stage::Tdd15
            | Stage::Qa
            | Stage::RedQueen
            | Stage::GptReview
            | Stage::ShipGate
    )
}

#[derive(Clone)]
struct WorkspacePrepRequest {
    run_id: String,
    bead_id: String,
    stage: Stage,
    attempt: u32,
    recorded_at: String,
    workspace_policy: WorkspacePreparationPolicy,
    repo_root: PathBuf,
}

struct WorkspaceCommandResult {
    command: String,
    passed: bool,
    exit_code: i32,
    output: String,
}

fn ensure_workspace_name(request: &WorkspacePrepRequest) -> Result<String, OyaError> {
    build_zjj_workspace_name(request.run_id.as_str(), request.stage.as_str(), request.attempt)
        .map_err(|error| OyaError(format!("Invalid workspace name for stage prep: {}", error)))
}

fn queue_workspace(
    request: &WorkspacePrepRequest,
    workspace: &str,
) -> Result<WorkspaceCommandResult, OyaError> {
    let command = format!("zjj queue --add {} --bead {}", workspace, request.bead_id);
    let args = ["queue", "--add", workspace, "--bead", request.bead_id.as_str()];
    run_workspace_command(request, &command, &args, "zjj queue", workspace)
}

fn add_workspace(
    request: &WorkspacePrepRequest,
    workspace: &str,
) -> Result<WorkspaceCommandResult, OyaError> {
    let command = format!("zjj add {} --idempotent", workspace);
    let args = ["add", workspace, "--idempotent"];
    run_workspace_command(request, &command, &args, "zjj add", workspace)
}

fn run_workspace_command(
    request: &WorkspacePrepRequest,
    command: &str,
    args: &[&str],
    operation: &str,
    workspace: &str,
) -> Result<WorkspaceCommandResult, OyaError> {
    let (passed, stdout, stderr, exit_code) =
        run_command_with_timeout_with_exit("zjj", args, ZJJ_TIMEOUT_SECONDS, &request.repo_root)?;
    let output = combine_command_output(stdout, stderr);
    if !passed {
        return Err(OyaError(format!(
            "{} failed for workspace {} (exit={}): {}",
            operation,
            workspace,
            exit_code,
            truncate_clean(output.as_str(), 2000)
        )));
    }
    Ok(WorkspaceCommandResult { command: command.to_string(), passed, exit_code, output })
}

fn prepare_stage_workspace(
    request: WorkspacePrepRequest,
) -> Result<Option<WorkspaceLifecycleEvent>, OyaError> {
    if request.workspace_policy.should_skip() || !stage_uses_workspace(&request.stage) {
        return Ok(None);
    }
    let workspace = ensure_workspace_name(&request)?;
    let queue = queue_workspace(&request, &workspace)?;
    let add = add_workspace(&request, &workspace)?;
    Ok(Some(WorkspaceLifecycleEvent {
        workspace,
        queue_command: queue.command,
        queue_passed: queue.passed,
        queue_exit_code: queue.exit_code,
        queue_output: truncate_clean(queue.output.as_str(), 4000),
        add_command: add.command,
        add_passed: add.passed,
        add_exit_code: add.exit_code,
        add_output: truncate_clean(add.output.as_str(), 4000),
        recorded_at: request.recorded_at,
    }))
}

fn first_non_empty_line_after_marker<'a>(message: &'a str, marker: &str) -> Option<&'a str> {
    let mut marker_seen = false;

    for line in message.lines() {
        if marker_seen {
            let trimmed = line.trim();
            if !trimmed.is_empty() {
                return Some(trimmed);
            }
        }

        if line.trim_start().starts_with(marker) {
            marker_seen = true;
        }
    }

    None
}

fn summarize_failure_output(category: &FailureCategory, message: &str) -> String {
    let trimmed = message.trim();
    if trimmed.is_empty() {
        return "No error output captured.".to_string();
    }

    if matches!(category, FailureCategory::OutputParseFailure)
        && trimmed.contains("Command timed out after")
    {
        let timeout_line = trimmed
            .lines()
            .find(|line| line.contains("Command timed out after"))
            .map(str::trim)
            .unwrap_or("Command timed out after unknown duration");

        let stderr_preview = first_non_empty_line_after_marker(trimmed, "stderr:")
            .map(|line| truncate_clean(line, 180));
        let stdout_preview = first_non_empty_line_after_marker(trimmed, "stdout:")
            .map(|line| truncate_clean(line, 180));

        let details = match stderr_preview {
            Some(line) => format!("stderr: {}", line),
            None => match stdout_preview {
                Some(line) => format!("stdout: {}", line),
                None => "No stdout/stderr preview available.".to_string(),
            },
        };

        return format!(
            "{}\n{}\nKeep fixes narrowly scoped so the next run completes within timeout.",
            timeout_line, details
        );
    }

    truncate_clean(trimmed, 1200)
}

fn to_json_string<T: Serialize>(value: &T) -> Result<String, OyaError> {
    serde_json::to_string(value).map_err(|error| OyaError(format!("json encode failed: {}", error)))
}

fn set_json_state<T: Serialize>(
    ctx: &WorkflowContext<'_>,
    key: &str,
    value: &T,
) -> Result<(), OyaError> {
    let encoded = to_json_string(value)?;
    ctx.set(key, encoded);
    Ok(())
}

fn write_orchestrator_state(
    ctx: &WorkflowContext<'_>,
    state: &OrchestratorState,
) -> Result<(), OyaError> {
    set_json_state(ctx, "state", state)
}

async fn append_timeline(ctx: &WorkflowContext<'_>, entry: TimelineEntry) -> Result<(), OyaError> {
    let existing = ctx
        .get::<String>("timeline")
        .await
        .map_err(|error| OyaError(format!("timeline read failed: {}", error)))?;
    let existing = existing.unwrap_or_default();

    let event_seq = ctx
        .get::<u32>("event_seq")
        .await
        .map_err(|error| OyaError(format!("event_seq read failed: {}", error)))?
        .map_or(1, |value| value + 1);
    ctx.set("event_seq", event_seq);

    let event_key = format!("event_{:04}", event_seq);
    set_json_state(ctx, &event_key, &entry)?;

    let line = to_json_string(&entry)?;
    let next = if existing.is_empty() { line } else { format!("{}\n{}", existing, line) };

    ctx.set("timeline", next);
    Ok(())
}

fn stage_attempt_key(stage: &Stage, attempt: u32, suffix: &str) -> String {
    format!("{}_{}_{}", stage.as_str(), attempt, suffix)
}

fn stage_failure_context(
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

struct StagePromptInput<'a> {
    stage: &'a Stage,
    bead_id: &'a str,
    context: &'a str,
    attempt: u32,
    failure_context: &'a str,
}

fn stage_prompt(input: StagePromptInput<'_>) -> String {
    let header = format!(
        "You are executing stage '{}' for: {}\n\nRequest context: {}\nAttempt: {}\n{}\n\n",
        input.stage.as_str(),
        input.bead_id,
        input.context,
        input.attempt,
        input.failure_context
    );

    let body = match input.stage {
        Stage::Plan => {
            "TASK:\n1. Create/update PLAN.md with exact implementation steps\n2. Include test strategy and quality gates\n3. Keep plan aligned to current codebase\n\nJust write files. Do not explain."
        }
        Stage::Contract => {
            "TASK: Write a design contract as a Rust doc comment in src/lib.rs (create if needed).\n\nInclude:\n1. Purpose and goals\n2. Key functions to implement\n3. Acceptance criteria\n\nJust write the code. Do not explain."
        }
        Stage::Tdd15 => {
            "TASK:\n1. Write tests in src/lib.rs for the functionality\n2. Implement the code to pass those tests\n3. Ensure `moon run :test` passes\n\nJust write the code. Do not explain."
        }
        Stage::Qa => {
            "TASK:\n1. Add edge case tests\n2. Add error handling tests\n3. Ensure all code paths are covered\n4. Fix any issues found\n\nJust write the code. Do not explain."
        }
        Stage::RedQueen => {
            "TASK:\n1. Write adversarial tests that try to break the code\n2. Test boundary conditions\n3. Test malformed inputs\n4. Fix any vulnerabilities found\n\nJust write the code. Do not explain."
        }
        Stage::GptReview => {
            "TASK:\n1. Review all code in src/\n2. Fix any code quality issues\n3. Add missing documentation\n4. Ensure clippy is happy with no warnings\n\nIMPORTANT RULES:\n- DO NOT use #[allow(...)] attributes to suppress warnings\n- Fix the actual underlying code issues\n- Remove dead code instead of allowing it\n- Fix type issues properly, don't work around them\n\nJust fix the code. Do not explain."
        }
        Stage::ShipGate => "",
    };

    format!("{}{}", header, body)
}

fn stage_success(stage: &Stage) -> (&'static str, Option<Stage>) {
    match stage {
        Stage::Plan => ("Planning completed", Some(Stage::Contract)),
        Stage::Contract => ("Contract written and compiles", Some(Stage::Tdd15)),
        Stage::Tdd15 => ("Tests written and passing", Some(Stage::Qa)),
        Stage::Qa => ("QA tests added and passing", Some(Stage::RedQueen)),
        Stage::RedQueen => ("Adversarial tests pass", Some(Stage::GptReview)),
        Stage::GptReview => ("Code review complete, clippy clean", Some(Stage::ShipGate)),
        Stage::ShipGate => ("All gates passed - ready to ship", None),
    }
}

struct ShipGateRequest {
    attempt: u32,
    merge_queue_policy: MergeQueuePolicy,
    repo_root: PathBuf,
}

fn execute_ship_gate(request: ShipGateRequest) -> Result<StageExecution, OyaError> {
    if request.attempt == 0 {
        return Err(OyaError("attempt must be greater than 0".to_string()));
    }
    execute_ship_gate_with_gate_runner(request.merge_queue_policy, |gate| {
        execute_gate(gate, &request.repo_root)
    })
}

fn execute_ship_gate_with_gate_runner<F>(
    merge_queue_policy: MergeQueuePolicy,
    run_gate: F,
) -> Result<StageExecution, OyaError>
where
    F: Fn(Gate) -> Result<GateEvidence, OyaError>,
{
    tracing::info!("SHIP GATE: Running final validation");
    let prompt = ship_gate_prompt();

    for gate in Stage::ShipGate.gates() {
        if !merge_queue_policy.should_run(&gate) {
            tracing::info!("SHIP GATE: skipping zjj merge queue check (OYA_SKIP_ZJJ_GATE=1)");
            continue;
        }

        let gate_evidence = run_gate(gate.clone())?;
        if !gate_evidence.passed {
            return Ok(ship_gate_failure(gate, gate_evidence, prompt));
        }
    }

    tracing::info!("SHIP GATE: ALL CHECKS PASSED");
    Ok(StageExecution {
        passed: true,
        output: "All gates passed - ready to ship".to_string(),
        failure_category: None,
        next_stage: None,
        prompt,
    })
}

fn ship_gate_prompt() -> String {
    "Ship gate executes quality gates only (moon/zjj); no OpenCode prompt".to_string()
}

fn ship_gate_failure(gate: Gate, evidence: GateEvidence, prompt: String) -> StageExecution {
    let (failure, next_stage) = gate_failure_outcome(&Stage::ShipGate, &gate);
    StageExecution {
        passed: false,
        output: format_gate_command_output(
            evidence.command.as_str(),
            evidence.exit_code,
            evidence.output.as_str(),
        ),
        failure_category: Some(failure),
        next_stage: Some(next_stage),
        prompt,
    }
}

fn is_valid_http_url(value: &str) -> bool {
    let trimmed = value.trim();
    if trimmed.is_empty() {
        return false;
    }

    match reqwest::Url::parse(trimmed) {
        Ok(url) => {
            let scheme_valid = url.scheme() == "http" || url.scheme() == "https";
            let host_valid = url.host_str().is_some();
            let creds_valid = url.username().is_empty() && url.password().is_none();
            // Ensure URL has no path, query, or fragment (should be base URL)
            let path_valid = url.path() == "/" || url.path().is_empty();
            let no_query = url.query().is_none();
            let no_fragment = url.fragment().is_none();
            scheme_valid && host_valid && creds_valid && path_valid && no_query && no_fragment
        }
        Err(_) => false,
    }
}

fn resolve_bind_addr() -> Result<std::net::SocketAddr, OyaError> {
    let configured = std::env::var("OYA_BIND_ADDR").ok();
    let value = configured.unwrap_or_else(|| "127.0.0.1:9080".to_string());

    value.parse().map_err(|e| OyaError(format!("Invalid OYA_BIND_ADDR '{}': {}", value, e)))
}

#[derive(Parser, Debug)]
#[command(name = "oya", about = "OYA Orchestrator - AI governance runtime")]
struct Cli {
    #[command(subcommand)]
    command: Option<CliCommand>,
}

#[derive(Subcommand, Debug)]
enum CliCommand {
    #[command(about = "Run the Restate orchestrator server (default)")]
    Serve,
    #[command(about = "Continuously poll OpenCode status and stream to stdout")]
    OpsPoll,
    #[command(about = "Run a bead through the TDD15 pipeline via Restate")]
    Run(RunArgs),
}

#[derive(Parser, Debug, Clone, PartialEq)]
struct RunArgs {
    #[arg(help = "Bead ID to process (e.g., src-abc123)")]
    bead_id: String,
    #[arg(long, default_value = "http://127.0.0.1:8080", help = "Restate ingress URL")]
    restate_url: String,
    #[arg(long, default_value = "local docker validation", help = "Context string for workflow")]
    context: String,
    #[arg(long, default_value = "3600", help = "Timeout in seconds for workflow completion")]
    timeout: u64,
    #[arg(long, help = "Poll interval in seconds for status checks")]
    poll_interval: Option<u64>,
    #[arg(long, help = "OpenCode model to use (overrides oya.yaml and built-in defaults)")]
    model: Option<String>,
}

type DynError = Box<dyn std::error::Error>;

fn parse_cli_mode() -> CliMode {
    let cli = Cli::parse();
    match cli.command {
        None | Some(CliCommand::Serve) => CliMode::Serve,
        Some(CliCommand::OpsPoll) => CliMode::OpsPoll,
        Some(CliCommand::Run(args)) => CliMode::Run(args),
    }
}

#[derive(Debug, Clone, PartialEq)]
enum CliMode {
    Serve,
    OpsPoll,
    Run(RunArgs),
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mode = parse_cli_mode();

    match mode {
        CliMode::OpsPoll => ops_poller::run_ops_poller().await,
        CliMode::Serve => run_server().await,
        CliMode::Run(args) => workflow_runner::run_workflow(args).await,
    }
}

async fn run_server() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize OpenTelemetry with dual-layer output:
    // - JSON logs to stdout (for OpenObserve log stream)
    // - OTLP traces to OpenObserve trace backend
    let _shutdown_guard = oya::telemetry::init_default()?;

    tracing::info!(
        service = "oya-orchestrator",
        port = 9080,
        execution_mode = "real",
        "OYA Orchestrator starting"
    );
    tracing::info!("Using REAL execution: opencode CLI + moon/zjj quality gates");

    let workflow_service = OyaOrchestratorImpl.serve();
    let monitor_service = OyaOpsMonitorImpl.serve();
    let service_option_input = default_restate_service_option_input();
    let workflow_service_options = build_restate_service_options(service_option_input);
    let monitor_service_options = build_restate_service_options(service_option_input);
    let endpoint = Endpoint::builder()
        .bind_with_options(workflow_service, workflow_service_options)
        .bind_with_options(monitor_service, monitor_service_options)
        .bind(OyaUsageTrackerImpl.serve())
        .build();

    let bind_addr = resolve_bind_addr()?;
    HttpServer::new(endpoint).listen_and_serve(bind_addr).await;

    Ok(())
}

#[derive(Clone, Copy)]
struct RestateServiceOptionInput {
    inactivity_timeout: std::time::Duration,
    abort_timeout: std::time::Duration,
    retry_policy_max_attempts: u64,
}

fn default_restate_service_option_input() -> RestateServiceOptionInput {
    RestateServiceOptionInput {
        inactivity_timeout: std::time::Duration::from_secs(30 * 60),
        abort_timeout: std::time::Duration::from_secs(5 * 60),
        retry_policy_max_attempts: 2,
    }
}

fn build_restate_service_options(
    input: RestateServiceOptionInput,
) -> restate_sdk::endpoint::ServiceOptions {
    restate_sdk::endpoint::ServiceOptions::new()
        .inactivity_timeout(input.inactivity_timeout)
        .abort_timeout(input.abort_timeout)
        .retry_policy_max_attempts(input.retry_policy_max_attempts)
        .retry_policy_kill_on_max_attempts()
}

#[cfg(test)]
mod tests;
