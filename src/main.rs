#![deny(clippy::unwrap_used)]
#![deny(clippy::expect_used)]
#![deny(clippy::panic)]
#![deny(clippy::too_many_lines)]
#![deny(clippy::too_many_arguments)]
#![forbid(unsafe_code)]

use oya::build_opencode_poll_snapshot;
use oya::types::{
    truncate_clean, AgentHealthStatus, BehavioralContext, BehavioralFingerprint, FailureCategory,
    StageFailure, StageName as Stage, TimelineEntry,
};
use oya::usage::{
    is_rate_limit_failure, tier_for_stage, OyaUsageTracker, OyaUsageTrackerClient,
    OyaUsageTrackerImpl, ReportOutcomeRequest, ServeOyaUsageTracker,
};
use restate_sdk::endpoint::Endpoint;
use restate_sdk::prelude::*;
use std::path::{Path, PathBuf};

const USAGE_TRACKER_KEY: &str = "global";
const RED_SEAL_KEY: &str = "red_acceptance_seal";
const DEFAULT_PIPELINE_STAGE_WATCHDOG_SECONDS: u64 = 480;
const DEFAULT_PROVIDER_POOL_RECOVERY_SECONDS: u64 = 180;
const DEFAULT_PROVIDER_UNAVAILABLE_MAX_ATTEMPTS: u32 = 3;

use clap::{Parser, Subcommand};

mod observe;
mod operator_visibility;
mod ops_poller;
mod orchestrator_types;
mod pipeline;
mod runtime_bundle;
mod runtime_tools;
mod runtime_up;
mod stage_executor;
mod stage_runtime;
mod tail;
mod workflow_runner;

use orchestrator_types::*;
use pipeline::{
    execute_and_accumulate_stage, init_pipeline_state, parse_rfc3339_stable,
    persist_stage_artifact, pipeline_input, workflow_timestamp_or_error, PipelineRunInput,
    PipelineState, RuntimeConfig, StageExecutionInput,
};
use runtime_tools::*;

#[cfg(test)]
use stage_runtime::execute_ship_gate_with_gate_runner;

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
#[name = "OyaOrchestrator"]
pub trait OyaOrchestrator {
    async fn run(request: Json<serde_json::Value>) -> Result<String, HandlerError>;
}

#[restate_sdk::service]
#[name = "OyaOpsMonitor"]
pub trait OyaOpsMonitor {
    async fn poll_status() -> Result<Json<OpsMonitorPollResponse>, HandlerError>;
    async fn poll_events(
        request: Json<OpsMonitorEventRequest>,
    ) -> Result<Json<OpsMonitorEventResponse>, HandlerError>;
}

/// Workflow service implementation for orchestrator runs.
pub struct OyaOrchestratorImpl;
/// Service implementation for OpenCode operational monitoring endpoints.
pub struct OyaOpsMonitorImpl;

impl OyaOrchestrator for OyaOrchestratorImpl {
    async fn run(
        &self,
        ctx: WorkflowContext<'_>,
        request: Json<serde_json::Value>,
    ) -> Result<String, HandlerError> {
        let parsed = parse_start_request(request.0)?;
        if let StartAdmission::Idempotent(run_id) =
            enforce_start_request_admission(&ctx, &parsed).await.map_err(terminal_workflow_error)?
        {
            return Ok(run_id);
        }
        let start = build_start_context(&ctx, parsed).await.map_err(terminal_workflow_error)?;
        persist_run_start(&ctx, &start).await.map_err(terminal_workflow_error)?;
        tracing::info!("=== RUN {} STARTED ===", start.run_id);
        tracing::info!("Bead: {}", start.bead_id);
        tracing::info!("Context: {}", start.context);
        tracing::info!("Model: {}", start.model);
        run_pipeline(&ctx, start.run_id.clone(), start.bead_id, start.context, start.model)
            .await
            .map_err(terminal_workflow_error)?;
        Ok(start.run_id)
    }
}

fn terminal_workflow_error(error: OyaError) -> HandlerError {
    TerminalError::new(error.to_string()).into()
}

struct StartContext {
    run_id: String,
    bead_id: String,
    context: String,
    model: String,
    started_at: String,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
struct RedSealRecord {
    bead_id: String,
    stage: String,
    attempt: u32,
    sealed_at: String,
    artifact_key: String,
}

enum StartAdmission {
    Fresh,
    Idempotent(String),
}

async fn enforce_start_request_admission(
    ctx: &WorkflowContext<'_>,
    parsed: &StartRequestPayload,
) -> Result<StartAdmission, OyaError> {
    let existing = ctx
        .get::<String>("run_request")
        .await
        .map_err(|error| OyaError(format!("run_request read failed: {}", error)))?;
    let existing = existing
        .map(|raw| {
            serde_json::from_str::<RunRequestEvent>(&raw)
                .map_err(|error| OyaError(format!("run_request parse failed: {}", error)))
        })
        .transpose()?;
    evaluate_start_request(existing, parsed, ctx.key())
}

fn evaluate_start_request(
    existing: Option<RunRequestEvent>,
    parsed: &StartRequestPayload,
    workflow_run_id: &str,
) -> Result<StartAdmission, OyaError> {
    let Some(existing) = existing else {
        return Ok(StartAdmission::Fresh);
    };
    let (bead_id, context) = normalized_start_inputs(parsed);
    if existing.run_id != workflow_run_id {
        return Err(OyaError(format!(
            "stale lease token: expected run_id={}, found run_id={}",
            workflow_run_id, existing.run_id
        )));
    }
    if existing.bead_id == bead_id && existing.context == context {
        Ok(StartAdmission::Idempotent(existing.run_id))
    } else {
        Err(OyaError("start rejected: workflow already claimed by a different payload".to_string()))
    }
}

fn normalized_start_inputs(parsed: &StartRequestPayload) -> (String, String) {
    (
        parsed.bead_id.clone().unwrap_or_else(|| "unknown".to_string()),
        parsed.context.clone().unwrap_or_default(),
    )
}

async fn build_start_context(
    ctx: &WorkflowContext<'_>,
    parsed: StartRequestPayload,
) -> Result<StartContext, OyaError> {
    let bead_id = parsed.bead_id.map_or_else(|| "unknown".to_string(), std::convert::identity);
    let context = parsed.context.map_or_else(String::new, std::convert::identity);

    // Get model from request or resolve via usage tracker for Plan stage tier
    let model = match parsed.model {
        Some(m) => m,
        None => match resolve_model_for_stage(ctx, &Stage::Contract).await {
            Ok(model) => model,
            Err(error) => {
                let message = error.to_string();
                if is_tracker_backpressure_error(message.as_str()) {
                    let fallback_model = configured_fallback_model();
                    tracing::warn!(
                        "usage tracker unavailable at run start; using fallback model '{}'",
                        fallback_model
                    );
                    fallback_model
                } else {
                    return Err(error);
                }
            }
        },
    };

    let started_at = workflow_timestamp_or_error(ctx).await?;
    Ok(StartContext { run_id: ctx.key().to_string(), bead_id, context, model, started_at })
}

fn configured_fallback_model() -> String {
    std::env::var("OYA_FALLBACK_MODEL")
        .ok()
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty())
        .map_or_else(|| "openai/gpt-5".to_string(), std::convert::identity)
}

/// Resolve the active model for a stage via the OyaUsageTracker VirtualObject.
/// Uses workflow object client calls to read service state.
async fn resolve_model_for_stage(
    ctx: &WorkflowContext<'_>,
    stage: &Stage,
) -> Result<String, OyaError> {
    let tier = tier_for_stage(stage).to_string();
    resolve_model_for_stage_with_backoff(ctx, &tier).await
}

async fn resolve_model_for_stage_cached(
    ctx: &WorkflowContext<'_>,
    state: &mut PipelineState,
    stage: &Stage,
) -> Result<String, OyaError> {
    let tier = tier_for_stage(stage).to_string();
    if let Some(model) = state.resolved_models.get(&tier) {
        return Ok(model.clone());
    }
    match resolve_model_for_stage_with_backoff(ctx, &tier).await {
        Ok(model) => {
            state.resolved_models.insert(tier, model.clone());
            Ok(model)
        }
        Err(error) => {
            let error_message = error.to_string();
            if is_tracker_backpressure_error(error_message.as_str()) {
                tracing::warn!(
                    "usage tracker unavailable for tier '{}'; reusing current model '{}'",
                    tier,
                    state.orchestrator.model
                );
                Ok(state.orchestrator.model.clone())
            } else {
                Err(error)
            }
        }
    }
}

fn invalidate_cached_model_for_stage(state: &mut PipelineState, stage: &Stage) {
    let tier = tier_for_stage(stage).to_string();
    state.resolved_models.remove(&tier);
}

async fn resolve_model_for_stage_with_backoff(
    ctx: &WorkflowContext<'_>,
    tier: &str,
) -> Result<String, OyaError> {
    let mut attempt: u32 = 1;
    loop {
        let result = ctx
            .object_client::<OyaUsageTrackerClient>(USAGE_TRACKER_KEY)
            .get_active_model(tier.to_string())
            .call()
            .await;

        match result {
            Ok(model) => return Ok(model.0),
            Err(error) => {
                let message = error.to_string();
                if !is_tracker_backpressure_error(&message) || attempt >= 5 {
                    return Err(OyaError(format!(
                        "usage tracker get_active_model failed: {}",
                        message
                    )));
                }
                let delay = tracker_backoff_duration(tier, attempt);
                ctx.sleep(delay).await.map_err(|sleep_error| {
                    OyaError(format!("durable sleep failed: {}", sleep_error))
                })?;
                attempt += 1;
            }
        }
    }
}

fn is_tracker_backpressure_error(message: &str) -> bool {
    ["all_models_rate_limited", "tier_circuit_open", "tier_token_exhausted"]
        .iter()
        .any(|needle| message.contains(needle))
}

fn tracker_backoff_duration(tier: &str, attempt: u32) -> std::time::Duration {
    let capped_attempt = attempt.min(8);
    let base_ms = 200_u64.saturating_mul(2_u64.pow(capped_attempt.saturating_sub(1)));
    let jitter_seed = tier.bytes().fold(0_u64, |acc, byte| acc.wrapping_add(u64::from(byte)));
    let jitter_ms = (jitter_seed + u64::from(attempt) * 17) % 250;
    std::time::Duration::from_millis((base_ms + jitter_ms).min(8_000))
}

/// Report execution outcome to the usage tracker for model health tracking.
async fn report_stage_outcome(
    ctx: &WorkflowContext<'_>,
    model: &str,
    success: bool,
    is_rate_limit: bool,
) -> Result<(), OyaError> {
    let model = model.to_string();
    ctx.object_client::<OyaUsageTrackerClient>(USAGE_TRACKER_KEY)
        .report_outcome(Json(ReportOutcomeRequest { model, success, is_rate_limit }))
        .call()
        .await
        .map_err(|error| OyaError(format!("usage tracker report_outcome failed: {}", error)))
}

async fn persist_run_start(
    ctx: &WorkflowContext<'_>,
    start: &StartContext,
) -> Result<(), OyaError> {
    write_orchestrator_state(ctx, &build_running_orchestrator_state(start))?;
    set_json_state(ctx, "run_request", &build_run_request_event(start))?;
    append_durable_event(ctx, build_run_started_event(start)).await?;
    append_timeline(
        ctx,
        TimelineEntry::RunStarted {
            bead_id: start.bead_id.clone(),
            context: start.context.clone(),
            at: parse_rfc3339_stable(&start.started_at),
        },
    )
    .await
}

fn build_running_orchestrator_state(start: &StartContext) -> OrchestratorState {
    OrchestratorState {
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
    }
}

fn build_run_request_event(start: &StartContext) -> RunRequestEvent {
    RunRequestEvent {
        run_id: start.run_id.clone(),
        bead_id: start.bead_id.clone(),
        context: start.context.clone(),
        started_at: start.started_at.clone(),
    }
}

fn build_run_started_event(start: &StartContext) -> DurableEvent {
    DurableEvent {
        event_type: "run_started".to_string(),
        run_id: start.run_id.clone(),
        bead_id: start.bead_id.clone(),
        stage: "plan".to_string(),
        attempt: 1,
        status: "running".to_string(),
        reason: "run accepted".to_string(),
        at: start.started_at.clone(),
        identity: resolve_change_identity(start.run_id.as_str(), start.bead_id.as_str(), None),
    }
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

        let snapshot = oya::build_opencode_poll_snapshot(
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
        let payloads = oya::parse_opencode_sse_events(raw.as_str(), max_events)
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
        enforce_red_gate_precondition(ctx, state).await?;
        let artifact = run_pipeline_stage_with_watchdog(ctx, config, input, state).await?;
        persist_stage_artifact(ctx, &artifact).await?;
        set_pipeline_state_from_artifact(state, &artifact);

        let should_continue = should_continue_after_artifact(ctx, state, config, &artifact).await?;
        if !should_continue {
            return Ok(());
        }

        ctx.sleep(std::time::Duration::from_millis(100))
            .await
            .map_err(|error| OyaError(format!("durable sleep failed: {}", error)))?;
    }
}

fn pipeline_stage_watchdog_seconds() -> u64 {
    std::env::var("OYA_PIPELINE_STAGE_WATCHDOG_SECONDS")
        .ok()
        .and_then(|value| value.trim().parse::<u64>().ok())
        .map(|seconds| seconds.clamp(60, 3_600))
        .unwrap_or(DEFAULT_PIPELINE_STAGE_WATCHDOG_SECONDS)
}

async fn run_pipeline_stage_with_watchdog(
    ctx: &WorkflowContext<'_>,
    config: &RuntimeConfig,
    input: &PipelineRunInput,
    state: &PipelineState,
) -> Result<orchestrator_types::StageArtifact, OyaError> {
    let started_at = workflow_timestamp_or_error(ctx).await?;
    let watchdog_seconds = pipeline_stage_watchdog_seconds();
    let execution = run_pipeline_stage(ctx, config, input, state);
    let timeout_result =
        tokio::time::timeout(std::time::Duration::from_secs(watchdog_seconds), execution).await;
    match watchdog_completed_result(timeout_result) {
        Some(result) => result,
        None => {
            build_stage_watchdog_timeout_artifact(ctx, input, state, started_at, watchdog_seconds)
                .await
        }
    }
}

fn watchdog_completed_result<T>(
    timeout_result: Result<T, tokio::time::error::Elapsed>,
) -> Option<T> {
    timeout_result.ok()
}

async fn build_stage_watchdog_timeout_artifact(
    ctx: &WorkflowContext<'_>,
    input: &PipelineRunInput,
    state: &PipelineState,
    started_at: String,
    watchdog_seconds: u64,
) -> Result<orchestrator_types::StageArtifact, OyaError> {
    let completed_at = workflow_timestamp_or_error(ctx).await?;
    let duration_ms = stage_timeout_duration_ms(&started_at, &completed_at);
    let full_log = stage_timeout_log(state, input, watchdog_seconds);
    Ok(orchestrator_types::StageArtifact {
        stage: state.current_stage.as_str().to_string(),
        attempt: state.attempt,
        failure_category: Some("watchdog_timeout".to_string()),
        next_stage: None,
        timing: orchestrator_types::StageTiming { started_at, completed_at, duration_ms },
        workspace: None,
        input: orchestrator_types::StageInputData {
            run_id: input.run_id.clone(),
            bead_id: input.bead_id.clone(),
            context: input.context.clone(),
            model: state.orchestrator.model.clone(),
            last_failure: None,
        },
        prompt: String::new(),
        output: orchestrator_types::StageOutputData {
            success: false,
            exit_code: 124,
            full_log: full_log.clone(),
            feedback: full_log,
            contract_document: None,
            implementation_code: None,
            test_results: None,
            adversarial_report: None,
        },
        task_tracking: None,
        gates: Vec::new(),
        status: orchestrator_types::StageStatus::Failed,
    })
}

fn stage_timeout_duration_ms(started_at: &str, completed_at: &str) -> u64 {
    let started = parse_rfc3339_stable(started_at);
    let completed = parse_rfc3339_stable(completed_at);
    (completed - started).num_milliseconds().max(0) as u64
}

fn stage_timeout_log(
    state: &PipelineState,
    input: &PipelineRunInput,
    watchdog_seconds: u64,
) -> String {
    format!(
        "provider_diagnostics source=pipeline_watchdog outcome=terminal failure_category=watchdog_timeout stage={} attempt={} model={} run_id={} bead_id={} watchdog_seconds={}",
        state.current_stage.as_str(),
        state.attempt,
        state.orchestrator.model,
        input.run_id,
        input.bead_id,
        watchdog_seconds
    )
}

async fn enforce_red_gate_precondition(
    ctx: &WorkflowContext<'_>,
    state: &mut PipelineState,
) -> Result<(), OyaError> {
    if state.current_stage != Stage::Implementation {
        return Ok(());
    }

    if state.red_seal_ready {
        return Ok(());
    }

    if has_red_seal(ctx).await? {
        state.red_seal_ready = true;
        return Ok(());
    }

    let failed_at = workflow_timestamp_or_error(ctx).await?;
    state.last_failure = Some(StageFailure::with_reason(
        FailureCategory::TestsUnexpectedlyGreen,
        "implementation blocked: missing sealed red acceptance tests".to_string(),
        failed_at,
    ));
    state.current_stage = Stage::Red;
    state.attempt = 1;
    let stage = Stage::Red;
    state.orchestrator.model = resolve_model_for_stage_cached(ctx, state, &stage).await?;
    Ok(())
}

async fn has_red_seal(ctx: &WorkflowContext<'_>) -> Result<bool, OyaError> {
    ctx.get::<String>(RED_SEAL_KEY)
        .await
        .map(|value| value.is_some())
        .map_err(|error| OyaError(format!("red seal read failed: {}", error)))
}

fn red_seal_record(
    state: &PipelineState,
    artifact: &orchestrator_types::StageArtifact,
) -> RedSealRecord {
    RedSealRecord {
        bead_id: state.orchestrator.bead_id.clone(),
        stage: artifact.stage.clone(),
        attempt: artifact.attempt,
        sealed_at: artifact.timing.completed_at.clone(),
        artifact_key: format!("{}_{}", artifact.stage, artifact.attempt),
    }
}

fn stage_is_red(artifact: &orchestrator_types::StageArtifact) -> bool {
    artifact.stage == Stage::Red.as_str()
}

fn stage_is_implementation(artifact: &orchestrator_types::StageArtifact) -> bool {
    artifact.stage == Stage::Implementation.as_str()
}

fn seal_red_acceptance(
    ctx: &WorkflowContext<'_>,
    state: &PipelineState,
    artifact: &orchestrator_types::StageArtifact,
) -> Result<(), OyaError> {
    set_json_state(ctx, RED_SEAL_KEY, &red_seal_record(state, artifact))
}

async fn run_pipeline_stage(
    ctx: &WorkflowContext<'_>,
    config: &RuntimeConfig,
    input: &PipelineRunInput,
    state: &PipelineState,
) -> Result<StageArtifact, OyaError> {
    execute_and_accumulate_stage(
        ctx,
        StageExecutionInput {
            run_id: &input.run_id,
            bead_id: &input.bead_id,
            context: &input.context,
            model: &state.orchestrator.model,
            stage: state.current_stage.clone(),
            attempt: state.attempt,
            last_failure: state.last_failure.clone(),
            repo_root: &config.repo_root,
        },
        config,
        state,
    )
    .await
}

fn set_pipeline_state_from_artifact(
    state: &mut PipelineState,
    artifact: &orchestrator_types::StageArtifact,
) {
    state.orchestrator.stage = artifact.stage.clone();
    state.orchestrator.attempt = artifact.attempt;
    state.orchestrator.updated_at = artifact.timing.completed_at.clone();
}

async fn should_continue_after_artifact(
    ctx: &WorkflowContext<'_>,
    state: &mut PipelineState,
    config: &RuntimeConfig,
    artifact: &orchestrator_types::StageArtifact,
) -> Result<bool, OyaError> {
    match artifact.status {
        orchestrator_types::StageStatus::Completed => {
            return completed_stage_next_action(ctx, state, config, artifact).await;
        }
        orchestrator_types::StageStatus::Failed => {
            let failure_category = parse_failure_category(&artifact.failure_category)
                .unwrap_or(FailureCategory::OutputParseFailure);
            let retryable = oya::is_retryable_failure(&failure_category);
            state.last_failure = Some(StageFailure::new(
                failure_category,
                artifact.output.full_log.clone(),
                retryable,
                artifact.timing.completed_at.clone(),
            ));
            handle_failed_stage(ctx, state, config, artifact).await
        }
    }
}

async fn completed_stage_next_action(
    ctx: &WorkflowContext<'_>,
    state: &mut PipelineState,
    config: &RuntimeConfig,
    artifact: &orchestrator_types::StageArtifact,
) -> Result<bool, OyaError> {
    // Report successful outcome for the current model
    report_stage_outcome(ctx, &state.orchestrator.model, true, false).await?;

    if stage_is_red(artifact) {
        seal_red_acceptance(ctx, state, artifact)?;
        state.red_seal_ready = true;
    }

    if stage_is_implementation(artifact) && !state.red_seal_ready {
        state.current_stage = Stage::Red;
        state.attempt = 1;
        let stage = state.current_stage.clone();
        state.orchestrator.model = resolve_model_for_stage_cached(ctx, state, &stage).await?;
        return Ok(true);
    }

    if let Some(next_stage) = state.current_stage.next() {
        state.current_stage = next_stage;
        state.attempt = 1;
        state.last_failure = None;
        emit_behavioral_alert_if_needed(ctx, state, artifact).await?;
        // Resolve model for the next stage's tier
        let stage = state.current_stage.clone();
        state.orchestrator.model = resolve_model_for_stage_cached(ctx, state, &stage).await?;
        return Ok(true);
    }

    if let Err(landing_failure) = run_landing_plane(ctx, state, config, artifact).await {
        state.current_stage = landing_failure.next_stage;
        state.attempt = 1;
        let retryable = oya::is_retryable_failure(&landing_failure.failure_category);
        state.last_failure = Some(StageFailure::new(
            landing_failure.failure_category,
            landing_failure.output,
            retryable,
            artifact.timing.completed_at.clone(),
        ));
        // Resolve model for the retry stage's tier
        let stage = state.current_stage.clone();
        state.orchestrator.model = resolve_model_for_stage_cached(ctx, state, &stage).await?;
        return Ok(true);
    }

    mark_run_completed(ctx, state, artifact).await?;
    Ok(false)
}

async fn handle_failed_stage(
    ctx: &WorkflowContext<'_>,
    state: &mut PipelineState,
    config: &RuntimeConfig,
    artifact: &orchestrator_types::StageArtifact,
) -> Result<bool, OyaError> {
    let failure_category = parse_failure_category(&artifact.failure_category)
        .unwrap_or(FailureCategory::OutputParseFailure);
    let should_rotate_provider = should_rotate_provider_on_failure(&failure_category);
    let previous_model = state.orchestrator.model.clone();

    // Report failure outcome to the usage tracker
    report_stage_outcome(ctx, &state.orchestrator.model, false, should_rotate_provider).await?;
    if should_rotate_provider {
        let stage = state.current_stage.clone();
        invalidate_cached_model_for_stage(state, &stage);
    }
    emit_behavioral_alert_if_needed(ctx, state, artifact).await?;

    if maybe_transition_to_next_stage(ctx, state, artifact).await? {
        return Ok(true);
    }

    if !should_retry_after_failure(state) {
        block_and_file_remediation(ctx, &config.repo_root, state, artifact).await?;
        mark_run_failed(ctx, state, artifact).await?;
        return Err(OyaError(terminal_pipeline_failure_message(state, artifact)));
    }

    apply_retry_backoff_if_needed(ctx, state).await?;
    resolve_retry_model_and_record_rotation(
        ctx,
        RetryModelResolutionInput {
            state,
            artifact,
            should_rotate_provider,
            failure_category: &failure_category,
            previous_model: previous_model.as_str(),
        },
    )
    .await?;
    Ok(true)
}

struct RetryModelResolutionInput<'a> {
    state: &'a mut PipelineState,
    artifact: &'a orchestrator_types::StageArtifact,
    should_rotate_provider: bool,
    failure_category: &'a FailureCategory,
    previous_model: &'a str,
}

async fn resolve_retry_model_and_record_rotation(
    ctx: &WorkflowContext<'_>,
    input: RetryModelResolutionInput<'_>,
) -> Result<(), OyaError> {
    input.state.attempt += 1;
    let stage = input.state.current_stage.clone();
    input.state.orchestrator.model = if input.should_rotate_provider {
        resolve_model_for_stage_with_pool_recovery(ctx, input.state, &stage).await?
    } else {
        resolve_model_for_stage_cached(ctx, input.state, &stage).await?
    };
    if input.should_rotate_provider {
        emit_provider_rotation_event(
            ctx,
            ProviderRotationRecordInput {
                state: input.state,
                artifact: input.artifact,
                previous_model: input.previous_model,
                next_model: input.state.orchestrator.model.as_str(),
                failure_category: input.failure_category,
            },
        )
        .await?;
    }
    Ok(())
}

async fn emit_provider_rotation_event(
    ctx: &WorkflowContext<'_>,
    input: ProviderRotationRecordInput<'_>,
) -> Result<(), OyaError> {
    let reason =
        provider_rotation_reason(input.previous_model, input.next_model, input.failure_category);
    let event = provider_rotation_event(ctx.key(), input.state, input.artifact, reason);
    append_durable_event(ctx, event).await
}

struct ProviderRotationRecordInput<'a> {
    state: &'a PipelineState,
    artifact: &'a orchestrator_types::StageArtifact,
    previous_model: &'a str,
    next_model: &'a str,
    failure_category: &'a FailureCategory,
}

fn provider_rotation_reason(
    previous_model: &str,
    next_model: &str,
    failure_category: &FailureCategory,
) -> String {
    format!(
        "provider rotation category={} from={} to={}",
        failure_category.as_str(),
        previous_model,
        next_model
    )
}

fn provider_rotation_event(
    run_id: &str,
    state: &PipelineState,
    artifact: &orchestrator_types::StageArtifact,
    reason: String,
) -> DurableEvent {
    DurableEvent {
        event_type: "provider_rotated".to_string(),
        run_id: run_id.to_string(),
        bead_id: state.orchestrator.bead_id.clone(),
        stage: artifact.stage.clone(),
        attempt: artifact.attempt,
        status: "failed".to_string(),
        reason,
        at: artifact.timing.completed_at.clone(),
        identity: resolve_change_identity(
            run_id,
            state.orchestrator.bead_id.as_str(),
            artifact.workspace.as_ref().map(|workspace| workspace.name.as_str()),
        ),
    }
}

async fn maybe_transition_to_next_stage(
    ctx: &WorkflowContext<'_>,
    state: &mut PipelineState,
    artifact: &orchestrator_types::StageArtifact,
) -> Result<bool, OyaError> {
    let Some(next_stage) = parse_next_stage(&artifact.next_stage) else {
        return Ok(false);
    };
    if next_stage == state.current_stage {
        return Ok(false);
    }
    if !is_allowed_failure_transition(&state.current_stage, &next_stage) {
        return Err(OyaError(format!(
            "invalid failure transition: {} -> {}",
            state.current_stage.as_str(),
            next_stage.as_str()
        )));
    }
    state.current_stage = next_stage;
    state.attempt = 1;
    let stage = state.current_stage.clone();
    state.orchestrator.model = resolve_model_for_stage_cached(ctx, state, &stage).await?;
    Ok(true)
}

fn should_rotate_provider_on_failure(category: &FailureCategory) -> bool {
    is_rate_limit_failure(category) || *category == FailureCategory::ProviderUnavailable
}

fn provider_pool_recovery_seconds() -> u64 {
    std::env::var("OYA_PROVIDER_POOL_RECOVERY_SECONDS")
        .ok()
        .and_then(|value| value.trim().parse::<u64>().ok())
        .map(|seconds| seconds.clamp(60, 900))
        .unwrap_or(DEFAULT_PROVIDER_POOL_RECOVERY_SECONDS)
}

async fn resolve_model_for_stage_with_pool_recovery(
    ctx: &WorkflowContext<'_>,
    state: &mut PipelineState,
    stage: &Stage,
) -> Result<String, OyaError> {
    let tier = tier_for_stage(stage).to_string();
    state.resolved_models.remove(&tier);
    let mut attempts: u32 = 1;
    loop {
        match resolve_model_for_stage_with_backoff(ctx, &tier).await {
            Ok(model) => {
                state.resolved_models.insert(tier.clone(), model.clone());
                return Ok(model);
            }
            Err(error) => {
                let message = error.to_string();
                if !is_tracker_backpressure_error(message.as_str()) || attempts >= 3 {
                    return Err(error);
                }
                let delay_seconds = provider_pool_recovery_seconds() * u64::from(attempts);
                let delay = std::time::Duration::from_secs(delay_seconds.min(1_800));
                tracing::warn!(
                    tier = %tier,
                    attempt = attempts,
                    delay_seconds,
                    "provider pool exhausted; waiting before model re-selection"
                );
                emit_provider_pool_wait_event(ctx, state, stage, attempts, delay_seconds).await?;
                ctx.sleep(delay).await.map_err(|sleep_error| {
                    OyaError(format!("durable sleep failed: {}", sleep_error))
                })?;
                attempts += 1;
            }
        }
    }
}

async fn emit_provider_pool_wait_event(
    ctx: &WorkflowContext<'_>,
    state: &PipelineState,
    stage: &Stage,
    attempt: u32,
    delay_seconds: u64,
) -> Result<(), OyaError> {
    let at = workflow_timestamp_or_error(ctx).await?;
    let reason = format!(
        "provider pool exhausted; recovery wait={}s retry_attempt={}",
        delay_seconds, attempt
    );
    append_durable_event(
        ctx,
        DurableEvent {
            event_type: "provider_pool_wait".to_string(),
            run_id: ctx.key().to_string(),
            bead_id: state.orchestrator.bead_id.clone(),
            stage: stage.as_str().to_string(),
            attempt: state.attempt,
            status: "running".to_string(),
            reason,
            at,
            identity: resolve_change_identity(ctx.key(), state.orchestrator.bead_id.as_str(), None),
        },
    )
    .await
}

fn terminal_pipeline_failure_message(
    state: &PipelineState,
    artifact: &orchestrator_types::StageArtifact,
) -> String {
    let category = artifact.failure_category.clone().unwrap_or_else(|| "unknown".to_string());
    format!(
        "pipeline failed: bead={} stage={} attempt={} category={}",
        state.orchestrator.bead_id, artifact.stage, artifact.attempt, category
    )
}

fn should_retry_after_failure(state: &PipelineState) -> bool {
    state.last_failure.as_ref().is_some_and(|failure| {
        let retryable = failure.retryable
            || transient_provider_retry_stage(&state.current_stage, &failure.category);
        retryable
            && state.attempt < max_attempts_for_failure(&state.current_stage, &failure.category)
    })
}

fn max_attempts_for_failure(stage: &Stage, category: &FailureCategory) -> u32 {
    if transient_provider_retry_stage(stage, category) {
        provider_unavailable_max_attempts()
    } else {
        stage.max_attempts()
    }
}

fn provider_unavailable_max_attempts() -> u32 {
    std::env::var("OYA_PROVIDER_UNAVAILABLE_MAX_ATTEMPTS")
        .ok()
        .and_then(|value| value.trim().parse::<u32>().ok())
        .map(|value| value.clamp(2, 6))
        .unwrap_or(DEFAULT_PROVIDER_UNAVAILABLE_MAX_ATTEMPTS)
}

fn transient_provider_retry_stage(_stage: &Stage, category: &FailureCategory) -> bool {
    *category == FailureCategory::ProviderUnavailable
}

fn is_allowed_failure_transition(current: &Stage, next: &Stage) -> bool {
    current == next || *next == Stage::Implementation
}

async fn apply_retry_backoff_if_needed(
    ctx: &WorkflowContext<'_>,
    state: &PipelineState,
) -> Result<(), OyaError> {
    let delay = state.last_failure.as_ref().and_then(|failure| {
        transient_provider_retry_stage(&state.current_stage, &failure.category)
            .then_some(provider_retry_backoff(state.attempt))
    });
    if let Some(delay) = delay {
        tracing::warn!(
            stage = %state.current_stage.as_str(),
            attempt = state.attempt,
            delay_ms = delay.as_millis(),
            "provider unavailable; applying retry backoff"
        );
        ctx.sleep(delay)
            .await
            .map_err(|error| OyaError(format!("durable retry backoff failed: {}", error)))?;
    }
    Ok(())
}

fn provider_retry_backoff(attempt: u32) -> std::time::Duration {
    let millis = 1_000_u64.saturating_mul(2_u64.saturating_pow(attempt.saturating_sub(1)));
    std::time::Duration::from_millis(millis.min(8_000))
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
struct BehavioralInterventionAlert {
    stage: String,
    attempt: u32,
    status: String,
    action: String,
    message: String,
}

async fn emit_behavioral_alert_if_needed(
    ctx: &WorkflowContext<'_>,
    state: &PipelineState,
    artifact: &orchestrator_types::StageArtifact,
) -> Result<(), OyaError> {
    if let Some(alert) = build_behavioral_alert(state, artifact) {
        tracing::warn!(
            stage = %alert.stage,
            attempt = alert.attempt,
            status = %alert.status,
            action = %alert.action,
            "{}",
            alert.message
        );
        let key = format!("behavioral_alert_{}_{}", alert.stage, alert.attempt);
        set_json_state(ctx, key.as_str(), &alert)?;
    }
    Ok(())
}

fn build_behavioral_alert(
    state: &PipelineState,
    artifact: &orchestrator_types::StageArtifact,
) -> Option<BehavioralInterventionAlert> {
    let fingerprint = behavioral_fingerprint(state, artifact);
    let stuck = fingerprint.is_stuck(300, 2);
    let retry_loop =
        fingerprint.is_retry_loop(state.current_stage.max_attempts().saturating_sub(1));
    let status = alert_status(stuck, retry_loop);
    if !status.needs_intervention() {
        return None;
    }
    Some(BehavioralInterventionAlert {
        stage: state.current_stage.as_str().to_string(),
        attempt: state.attempt,
        status: status.as_str().to_string(),
        action: "escalate_to_operator".to_string(),
        message: format!(
            "behavioral intervention required: stage={} attempt={} duration_ms={} retry_count={}",
            state.current_stage.as_str(),
            state.attempt,
            artifact.timing.duration_ms,
            state.attempt.saturating_sub(1)
        ),
    })
}

fn behavioral_fingerprint(
    state: &PipelineState,
    artifact: &orchestrator_types::StageArtifact,
) -> BehavioralFingerprint {
    let context = BehavioralContext::new(
        Some(state.orchestrator.bead_id.clone()),
        state.current_stage.as_str().to_string(),
    );
    BehavioralFingerprint::new(
        format!("oya-{}", state.orchestrator.bead_id),
        context,
        state.attempt,
        artifact.timing.duration_ms / 1_000,
        state.attempt.saturating_sub(1),
    )
}

fn alert_status(stuck: bool, retry_loop: bool) -> AgentHealthStatus {
    if retry_loop {
        AgentHealthStatus::RetryLoop
    } else if stuck {
        AgentHealthStatus::Stuck
    } else {
        AgentHealthStatus::Healthy
    }
}

async fn block_and_file_remediation(
    ctx: &WorkflowContext<'_>,
    repo_root: &Path,
    state: &PipelineState,
    artifact: &orchestrator_types::StageArtifact,
) -> Result<(), OyaError> {
    let bead_id = state.orchestrator.bead_id.clone();
    let title = format!("[remediation] {} retry-exhausted", bead_id);
    let description = remediation_description(state, artifact);
    let root = repo_root.to_path_buf();

    let blocked_args = vec!["update", &bead_id, "--status", "blocked"];
    run_br_command(ctx, &root, &blocked_args).await?;

    let create_args = vec![
        "create",
        title.as_str(),
        "--type",
        "bug",
        "--priority",
        "1",
        "--parent",
        bead_id.as_str(),
        "--description",
        description.as_str(),
    ];
    let create_output = run_br_command(ctx, &root, &create_args).await?;
    let child_bead_id = extract_child_bead_id(&create_output, bead_id.as_str());
    record_stack_transition(
        ctx,
        bead_id.as_str(),
        child_bead_id.as_deref(),
        StackTransitionKind::ParentBlockedChildReady,
        "retry_exhausted",
    )
    .await
}

fn remediation_description(
    state: &PipelineState,
    artifact: &orchestrator_types::StageArtifact,
) -> String {
    let category = artifact.failure_category.clone().unwrap_or_else(|| "unknown".to_string());
    let summary = truncate_clean(&artifact.output.full_log, 1200);
    format!(
        "Parent bead {} exhausted automatic retries at stage '{}' (attempt {}).\nFailure category: {}\n\nLatest failure:\n{}",
        state.orchestrator.bead_id,
        artifact.stage,
        artifact.attempt,
        category,
        summary
    )
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
enum StackReadiness {
    Ready,
    Blocked,
}

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
struct StackPairState {
    parent: StackReadiness,
    child: StackReadiness,
}

impl StackPairState {
    const fn new(parent: StackReadiness, child: StackReadiness) -> Self {
        Self { parent, child }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
enum StackTransitionKind {
    ParentBlockedChildReady,
    ChildBlocked,
    ChildReady,
    MainMovedChildReady,
    ParentReady,
}

#[cfg(test)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum StackMovement {
    ParentMoved,
    MainMoved,
}

#[cfg(test)]
fn stack_rebase_transition_from_movement(movement: StackMovement) -> StackTransitionKind {
    match movement {
        StackMovement::ParentMoved => StackTransitionKind::ParentReady,
        StackMovement::MainMoved => StackTransitionKind::MainMovedChildReady,
    }
}

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
struct StackTransitionMetadata {
    parent_bead_id: String,
    child_bead_id: Option<String>,
    transition: StackTransitionKind,
    before: StackPairState,
    after: StackPairState,
    reason: String,
    recorded_at: String,
}

fn apply_stack_transition(
    state: StackPairState,
    transition: StackTransitionKind,
) -> Result<StackPairState, OyaError> {
    match (state.parent, state.child, transition) {
        (StackReadiness::Ready, _, StackTransitionKind::ParentBlockedChildReady) => {
            Ok(StackPairState::new(StackReadiness::Blocked, StackReadiness::Ready))
        }
        (StackReadiness::Blocked, StackReadiness::Ready, StackTransitionKind::ChildBlocked) => {
            Ok(StackPairState::new(StackReadiness::Blocked, StackReadiness::Blocked))
        }
        (StackReadiness::Blocked, StackReadiness::Blocked, StackTransitionKind::ChildReady) => {
            Ok(StackPairState::new(StackReadiness::Blocked, StackReadiness::Ready))
        }
        (
            StackReadiness::Blocked,
            StackReadiness::Blocked,
            StackTransitionKind::MainMovedChildReady,
        ) => Ok(StackPairState::new(StackReadiness::Blocked, StackReadiness::Ready)),
        (StackReadiness::Blocked, StackReadiness::Blocked, StackTransitionKind::ParentReady) => {
            Ok(StackPairState::new(StackReadiness::Ready, StackReadiness::Ready))
        }
        (StackReadiness::Blocked, StackReadiness::Ready, StackTransitionKind::ParentReady) => {
            Ok(StackPairState::new(StackReadiness::Ready, StackReadiness::Ready))
        }
        _ => Err(OyaError(format!("invalid stack transition: {:?}", transition))),
    }
}

struct StackTransitionRecordInput {
    parent_bead_id: String,
    child_bead_id: Option<String>,
    transition: StackTransitionKind,
    before: StackPairState,
    after: StackPairState,
    reason: String,
    recorded_at: String,
}

fn build_stack_transition_metadata(input: StackTransitionRecordInput) -> StackTransitionMetadata {
    StackTransitionMetadata {
        parent_bead_id: input.parent_bead_id,
        child_bead_id: input.child_bead_id,
        transition: input.transition,
        before: input.before,
        after: input.after,
        reason: input.reason,
        recorded_at: input.recorded_at,
    }
}

#[derive(Debug, Clone)]
struct BrCommandOutput {
    stdout: String,
    stderr: String,
}

fn extract_child_bead_id(output: &BrCommandOutput, parent_bead_id: &str) -> Option<String> {
    [output.stdout.as_str(), output.stderr.as_str()]
        .iter()
        .find_map(|text| find_distinct_bead_id(text, parent_bead_id))
}

fn find_distinct_bead_id(text: &str, parent_bead_id: &str) -> Option<String> {
    text.split_whitespace().find_map(|token| {
        let normalized =
            token.trim_matches(|ch: char| !ch.is_ascii_alphanumeric() && ch != '-').to_string();
        let looks_like_bead = normalized.starts_with("src-") && normalized.len() > 4;
        if looks_like_bead && normalized != parent_bead_id {
            Some(normalized)
        } else {
            None
        }
    })
}

async fn record_stack_transition(
    ctx: &WorkflowContext<'_>,
    parent_bead_id: &str,
    child_bead_id: Option<&str>,
    transition: StackTransitionKind,
    reason: &str,
) -> Result<(), OyaError> {
    let before = StackPairState::new(StackReadiness::Ready, StackReadiness::Ready);
    let after = apply_stack_transition(before.clone(), transition)?;
    let recorded_at = workflow_timestamp_or_error(ctx).await?;
    let metadata = build_stack_transition_metadata(StackTransitionRecordInput {
        parent_bead_id: parent_bead_id.to_string(),
        child_bead_id: child_bead_id.map(str::to_string),
        transition,
        before,
        after,
        reason: reason.to_string(),
        recorded_at,
    });
    set_json_state(ctx, "stack_transition", &metadata)
}

async fn run_br_command(
    ctx: &WorkflowContext<'_>,
    repo_root: &Path,
    args: &[&str],
) -> Result<BrCommandOutput, OyaError> {
    let root = repo_root.to_path_buf();
    let arg_list = args.iter().map(|value| (*value).to_string()).collect::<Vec<_>>();
    let display = refs_to_display(&arg_list);

    let result = ctx
        .run(move || async move {
            let args_for_spawn = arg_list.clone();
            let command_result = tokio::task::spawn_blocking(move || {
                let refs = args_for_spawn.iter().map(String::as_str).collect::<Vec<_>>();
                run_command_with_timeout_with_exit("br", &refs, 60, &root)
            })
            .await
            .map_err(|error| HandlerError::from(format!("br command join failed: {}", error)))?;

            command_result
                .map(|(passed, stdout, stderr, exit_code)| {
                    if passed {
                        Ok(stdout)
                    } else {
                        Err(HandlerError::from(format!(
                            "br {} failed (exit {}): {} {}",
                            display, exit_code, stdout, stderr
                        )))
                    }
                })
                .map_err(|error| HandlerError::from(error.0))?
        })
        .await
        .map_err(|error| OyaError(format!("br command failed: {}", error)))?;

    Ok(BrCommandOutput { stdout: result, stderr: String::new() })
}

fn refs_to_display(args: &[String]) -> String {
    args.join(" ")
}

struct LandingFailure {
    failure_category: FailureCategory,
    next_stage: Stage,
    output: String,
}

struct CommandStep {
    id: String,
    label: String,
    program: String,
    args: Vec<String>,
    timeout_seconds: u64,
    failure_category: FailureCategory,
    next_stage: Stage,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
struct LandingCommandResult {
    passed: bool,
    stdout: String,
    stderr: String,
    exit_code: i32,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
struct LandingStepTelemetry {
    step_id: String,
    command: String,
    started_at: String,
    completed_at: String,
    passed: bool,
    exit_code: i32,
    stdout: String,
    stderr: String,
}

struct LandingStepTemplate {
    id: &'static str,
    label: &'static str,
    program: &'static str,
    args: &'static [&'static str],
    timeout_seconds: u64,
    failure_category: FailureCategory,
    next_stage: Stage,
}

const LANDING_STEPS: &[LandingStepTemplate] = &[
    LandingStepTemplate {
        id: "moon_ci",
        label: "moon ci",
        program: "moon",
        args: &["run", ":ci"],
        timeout_seconds: 1_800,
        failure_category: FailureCategory::TestFailed,
        next_stage: Stage::Implementation,
    },
    LandingStepTemplate {
        id: "zjj_sync",
        label: "zjj sync",
        program: "zjj",
        args: &["sync"],
        timeout_seconds: 120,
        failure_category: FailureCategory::MergeConflict,
        next_stage: Stage::Implementation,
    },
    LandingStepTemplate {
        id: "zjj_done",
        label: "zjj done",
        program: "zjj",
        args: &["done"],
        timeout_seconds: 120,
        failure_category: FailureCategory::MergeConflict,
        next_stage: Stage::Implementation,
    },
];

async fn run_landing_plane(
    ctx: &WorkflowContext<'_>,
    state: &PipelineState,
    config: &RuntimeConfig,
    artifact: &orchestrator_types::StageArtifact,
) -> Result<(), LandingFailure> {
    let run_root = resolve_landing_run_root(config, artifact);
    for template in LANDING_STEPS {
        let step = landing_step_from_template(template);
        run_landing_step(ctx, &run_root, step).await?;
    }

    run_landing_step(ctx, &run_root, closing_step(&state.orchestrator.bead_id)).await?;
    run_landing_step(ctx, &run_root, sync_flush_step()).await?;

    Ok(())
}

fn landing_step_from_template(template: &LandingStepTemplate) -> CommandStep {
    CommandStep {
        id: template.id.to_string(),
        label: template.label.to_string(),
        program: template.program.to_string(),
        args: template.args.iter().map(|value| value.to_string()).collect::<Vec<_>>(),
        timeout_seconds: template.timeout_seconds,
        failure_category: template.failure_category.clone(),
        next_stage: template.next_stage.clone(),
    }
}

fn closing_step(bead_id: &str) -> CommandStep {
    CommandStep {
        id: "br_close".to_string(),
        label: "br close".to_string(),
        program: "br".to_string(),
        args: vec!["close".to_string(), bead_id.to_string()],
        timeout_seconds: 60,
        failure_category: FailureCategory::OutputParseFailure,
        next_stage: Stage::ShipGate,
    }
}

fn sync_flush_step() -> CommandStep {
    CommandStep {
        id: "br_sync_flush_only".to_string(),
        label: "br sync --flush-only".to_string(),
        program: "br".to_string(),
        args: vec!["sync".to_string(), "--flush-only".to_string()],
        timeout_seconds: 60,
        failure_category: FailureCategory::OutputParseFailure,
        next_stage: Stage::ShipGate,
    }
}

fn resolve_landing_run_root(
    config: &RuntimeConfig,
    artifact: &orchestrator_types::StageArtifact,
) -> PathBuf {
    artifact
        .workspace
        .as_ref()
        .map(|workspace| PathBuf::from(workspace.path.as_str()))
        .filter(|path| !path.as_os_str().is_empty())
        .unwrap_or_else(|| config.repo_root.clone())
}

async fn run_landing_step(
    ctx: &WorkflowContext<'_>,
    repo_root: &Path,
    step: CommandStep,
) -> Result<(), LandingFailure> {
    let telemetry_key = landing_step_key(step.id.as_str());
    if landing_step_completed(ctx, &telemetry_key).await? {
        return Ok(());
    }

    let command = build_landing_command(&step);
    let started_at = pipeline::workflow_timestamp(ctx).await.map_err(|error| {
        landing_failure_from_step(&step, format!("{} timestamp failed: {}", step.label, error))
    })?;

    let result = run_landing_command(ctx, repo_root, &step).await?;

    let completed_at = pipeline::workflow_timestamp(ctx).await.map_err(|error| {
        landing_failure_from_step(&step, format!("{} timestamp failed: {}", step.label, error))
    })?;
    let telemetry = build_landing_telemetry(&step, &command, &started_at, &completed_at, &result);
    persist_landing_step(ctx, &telemetry_key, telemetry, &step)?;

    if result.0.passed {
        return Ok(());
    }

    Err(build_landing_failure(step, command, &result.0))
}

fn build_landing_command(step: &CommandStep) -> String {
    format!("{} {}", step.program.as_str(), step.args.join(" "))
}

fn build_landing_telemetry(
    step: &CommandStep,
    command: &str,
    started_at: &str,
    completed_at: &str,
    result: &Json<LandingCommandResult>,
) -> LandingStepTelemetry {
    LandingStepTelemetry {
        step_id: step.id.clone(),
        command: command.to_string(),
        started_at: started_at.to_string(),
        completed_at: completed_at.to_string(),
        passed: result.0.passed,
        exit_code: result.0.exit_code,
        stdout: truncate_clean(&result.0.stdout, 4000),
        stderr: truncate_clean(&result.0.stderr, 4000),
    }
}

fn build_landing_failure(
    step: CommandStep,
    command: String,
    result: &LandingCommandResult,
) -> LandingFailure {
    LandingFailure {
        failure_category: step.failure_category,
        next_stage: step.next_stage,
        output: truncate_clean(
            &format!(
                "command={} exit_code={}\n{}\n{}",
                command, result.exit_code, result.stdout, result.stderr
            ),
            6000,
        ),
    }
}

async fn run_landing_command(
    ctx: &WorkflowContext<'_>,
    repo_root: &Path,
    step: &CommandStep,
) -> Result<Json<LandingCommandResult>, LandingFailure> {
    let args = step.args.to_vec();
    let program = step.program.to_string();
    let root = repo_root.to_path_buf();
    let timeout_seconds = step.timeout_seconds;

    ctx.run(move || async move {
        let command_result = tokio::task::spawn_blocking(move || {
            let arg_refs = args.iter().map(String::as_str).collect::<Vec<_>>();
            run_command_with_timeout_with_exit(&program, &arg_refs, timeout_seconds, &root)
        })
        .await
        .map_err(|error| HandlerError::from(format!("landing command join failed: {}", error)))?;

        command_result
            .map(|(passed, stdout, stderr, exit_code)| {
                Json(LandingCommandResult { passed, stdout, stderr, exit_code })
            })
            .map_err(|error| HandlerError::from(error.0))
    })
    .await
    .map_err(|error| LandingFailure {
        failure_category: step.failure_category.clone(),
        next_stage: step.next_stage.clone(),
        output: format!("{} failed before completion: {}", step.label, error),
    })
}

fn landing_step_key(step_id: &str) -> String {
    format!("landing_step_{}", step_id)
}

async fn landing_step_completed(
    ctx: &WorkflowContext<'_>,
    key: &str,
) -> Result<bool, LandingFailure> {
    let stored = ctx.get::<String>(key).await.map_err(|error| LandingFailure {
        failure_category: FailureCategory::OutputParseFailure,
        next_stage: Stage::Implementation,
        output: format!("landing telemetry read failed: {}", error),
    })?;

    Ok(stored
        .and_then(|raw| serde_json::from_str::<LandingStepTelemetry>(&raw).ok())
        .is_some_and(|entry| entry.passed))
}

fn persist_landing_step(
    ctx: &WorkflowContext<'_>,
    key: &str,
    telemetry: LandingStepTelemetry,
    step: &CommandStep,
) -> Result<(), LandingFailure> {
    let encoded = serde_json::to_string(&telemetry).map_err(|error| {
        landing_failure_from_step(step, format!("landing telemetry encode failed: {}", error))
    })?;
    ctx.set(key, encoded);
    Ok(())
}

fn landing_failure_from_step(step: &CommandStep, output: String) -> LandingFailure {
    LandingFailure {
        failure_category: step.failure_category.clone(),
        next_stage: step.next_stage.clone(),
        output,
    }
}

fn parse_next_stage(next_stage: &Option<String>) -> Option<Stage> {
    next_stage.as_deref().and_then(|value| Stage::try_from(value).ok())
}

fn parse_failure_category(category: &Option<String>) -> Option<FailureCategory> {
    category.as_deref().and_then(|value| match value {
        "test_failed" => Some(FailureCategory::TestFailed),
        "tests_unexpectedly_green" => Some(FailureCategory::TestsUnexpectedlyGreen),
        "test_infra_failed" => Some(FailureCategory::TestInfraFailed),
        "compile_failed" => Some(FailureCategory::CompileFailed),
        "lint_failed" => Some(FailureCategory::LintFailed),
        "merge_conflict" => Some(FailureCategory::MergeConflict),
        "rate_limited" => Some(FailureCategory::RateLimited),
        "auth_failed" => Some(FailureCategory::AuthFailed),
        "context_overflow" => Some(FailureCategory::ContextOverflow),
        "provider_unavailable" => Some(FailureCategory::ProviderUnavailable),
        "watchdog_timeout" => Some(FailureCategory::WatchdogTimeout),
        "output_parse_failure" => Some(FailureCategory::OutputParseFailure),
        "max_attempts_exceeded" => Some(FailureCategory::MaxAttemptsExceeded),
        _ => None,
    })
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum LifecycleStatus {
    Queued,
    Running,
    Shipped,
    Failed,
}

impl LifecycleStatus {
    const fn as_str(self) -> &'static str {
        match self {
            Self::Queued => "queued",
            Self::Running => "running",
            Self::Shipped => "shipped",
            Self::Failed => "failed",
        }
    }

    fn parse(value: &str) -> Result<Self, OyaError> {
        match value {
            "queued" => Ok(Self::Queued),
            "running" => Ok(Self::Running),
            "shipped" => Ok(Self::Shipped),
            "failed" => Ok(Self::Failed),
            _ => Err(OyaError(format!("unknown lifecycle status: {}", value))),
        }
    }
}

fn lifecycle_transition(
    current: &str,
    target: LifecycleStatus,
) -> Result<Option<LifecycleStatus>, OyaError> {
    let current = LifecycleStatus::parse(current)?;
    if current == target {
        return Ok(None);
    }
    let allowed = matches!(
        (current, target),
        (LifecycleStatus::Queued, LifecycleStatus::Running)
            | (LifecycleStatus::Running, LifecycleStatus::Shipped)
            | (LifecycleStatus::Running, LifecycleStatus::Failed)
    );
    if !allowed {
        return Err(OyaError(format!(
            "invalid lifecycle transition: {} -> {}",
            current.as_str(),
            target.as_str()
        )));
    }
    Ok(Some(target))
}

async fn mark_run_completed(
    ctx: &WorkflowContext<'_>,
    state: &mut PipelineState,
    final_artifact: &orchestrator_types::StageArtifact,
) -> Result<(), OyaError> {
    let Some(next_status) =
        lifecycle_transition(state.orchestrator.status.as_str(), LifecycleStatus::Shipped)?
    else {
        return Ok(());
    };
    let started_at = state.orchestrator.updated_at.clone();
    let completed_at = workflow_timestamp_or_error(ctx).await?;

    state.orchestrator.status = next_status.as_str().to_string();
    state.orchestrator.stage = "none".to_string();
    state.orchestrator.updated_at = completed_at.clone();
    orchestrator_types::write_orchestrator_state(ctx, &state.orchestrator)?;
    append_durable_event(
        ctx,
        DurableEvent {
            event_type: "run_completed".to_string(),
            run_id: ctx.key().to_string(),
            bead_id: state.orchestrator.bead_id.clone(),
            stage: final_artifact.stage.clone(),
            attempt: final_artifact.attempt,
            status: "shipped".to_string(),
            reason: "terminal success".to_string(),
            at: completed_at.clone(),
            identity: resolve_change_identity(
                ctx.key(),
                state.orchestrator.bead_id.as_str(),
                final_artifact.workspace.as_ref().map(|workspace| workspace.name.as_str()),
            ),
        },
    )
    .await?;

    // Build lean timeline as JSON array
    let timeline = serde_json::json!([
        {"event": "RunStarted", "at": started_at},
        {"event": "RunShipped", "at": completed_at, "total_duration_ms": final_artifact.timing.duration_ms}
    ]);

    orchestrator_types::set_timeline_once(ctx, &timeline.to_string())
}

async fn mark_run_failed(
    ctx: &WorkflowContext<'_>,
    state: &mut PipelineState,
    artifact: &orchestrator_types::StageArtifact,
) -> Result<(), OyaError> {
    let Some(next_status) =
        lifecycle_transition(state.orchestrator.status.as_str(), LifecycleStatus::Failed)?
    else {
        return Ok(());
    };
    let started_at = state.orchestrator.updated_at.clone();
    state.orchestrator.status = next_status.as_str().to_string();
    state.orchestrator.updated_at = artifact.timing.completed_at.clone();
    orchestrator_types::write_orchestrator_state(ctx, &state.orchestrator)?;
    append_durable_event(
        ctx,
        DurableEvent {
            event_type: "run_failed".to_string(),
            run_id: ctx.key().to_string(),
            bead_id: state.orchestrator.bead_id.clone(),
            stage: artifact.stage.clone(),
            attempt: artifact.attempt,
            status: "failed".to_string(),
            reason: artifact.failure_category.clone().unwrap_or_else(|| "unknown".to_string()),
            at: artifact.timing.completed_at.clone(),
            identity: resolve_change_identity(
                ctx.key(),
                state.orchestrator.bead_id.as_str(),
                artifact.workspace.as_ref().map(|workspace| workspace.name.as_str()),
            ),
        },
    )
    .await?;

    // Build lean timeline as JSON array
    let timeline = serde_json::json!([
        {"event": "RunStarted", "at": started_at},
        {"event": "RunFailed", "stage": artifact.stage, "at": artifact.timing.completed_at}
    ]);

    orchestrator_types::set_timeline_once(ctx, &timeline.to_string())
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
#[command(name = "oya", about = "OYA Orchestrator - AI governance runtime", version = env!("CARGO_PKG_VERSION"))]
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
    #[command(about = "Live TUI for monitoring pipeline invocations")]
    Tail(TailArgs),
    #[command(about = "Show latest operator status fields for a run")]
    Status(StatusArgs),
    #[command(about = "Show operator diagnostics for stuck/cleanup-pending runs")]
    Doctor(DoctorArgs),
    #[command(about = "Prune deterministic cleanup-pending run artifacts")]
    CleanupReconcile(CleanupReconcileArgs),
    #[command(about = "Run holdout quality gate checks")]
    Check,
    #[command(about = "Bootstrap local runtime (restate + opencode + oya service)", alias = "up")]
    Init,
    #[command(about = "Install self-contained oya binary into current repo (.oya/bin/oya)")]
    Bundle,
    #[command(about = "Stream JSON bridge events from .oya/bridge/events.jsonl")]
    Observe(ObserveArgs),
}

#[derive(clap::ValueEnum, Debug, Clone, Copy, PartialEq, Eq)]
enum RunIdMode {
    Bead,
    Unique,
}

#[derive(Parser, Debug, Clone, PartialEq)]
struct RunArgs {
    #[arg(help = "Bead ID to process (e.g., src-abc123)")]
    bead_id: String,
    #[arg(long, default_value = "http://127.0.0.1:8080", help = "Restate ingress URL")]
    restate_url: String,
    #[arg(long, default_value = "local docker validation", help = "Context string for workflow")]
    context: String,
    #[arg(
        long,
        default_value = "3600",
        value_parser = clap::value_parser!(u64).range(1..=86_400),
        help = "Timeout in seconds for workflow completion (1-86400)"
    )]
    timeout: u64,
    #[arg(
        long,
        value_parser = clap::value_parser!(u64).range(1..=3600),
        help = "Poll interval in seconds for status checks (1-3600)"
    )]
    poll_interval: Option<u64>,
    #[arg(long, help = "OpenCode model to use (overrides oya.yaml and built-in defaults)")]
    model: Option<String>,
    #[arg(
        long,
        value_enum,
        default_value = "unique",
        help = "Run ID mode: bead (stable) or unique (timestamped)"
    )]
    run_id_mode: RunIdMode,
}

#[derive(Parser, Debug, Clone, PartialEq)]
struct ObserveArgs {
    #[arg(help = "Filter to specific run ID (optional)")]
    run_id: Option<String>,
    #[arg(long, default_value = "false", help = "Follow new events as they are appended")]
    follow: bool,
    #[arg(
        long,
        default_value = "2",
        value_parser = clap::value_parser!(u64).range(1..=3600),
        help = "Refresh interval seconds when --follow is used"
    )]
    interval: u64,
    #[arg(
        long,
        default_value = "50",
        value_parser = clap::value_parser!(u64).range(1..=10_000),
        help = "How many recent events to print first"
    )]
    limit: u64,
}

#[derive(Parser, Debug, Clone, PartialEq)]
struct TailArgs {
    #[arg(
        long,
        default_value = "2",
        value_parser = clap::value_parser!(u64).range(1..=3600),
        help = "Refresh interval in seconds (1-3600)"
    )]
    interval: u64,
    #[arg(help = "Filter to specific run ID (optional)")]
    run_id: Option<String>,
    #[arg(long, default_value = "false", help = "Use event stream mode (deterministic lines)")]
    events: bool,
    #[arg(long, default_value = "false", help = "Follow new events when --events is enabled")]
    follow: bool,
    #[arg(
        long,
        default_value = "50",
        value_parser = clap::value_parser!(u64).range(1..=10_000),
        help = "Bounded output window for --events mode"
    )]
    limit: u64,
}

#[derive(Parser, Debug, Clone, PartialEq)]
struct StatusArgs {
    #[arg(help = "Run ID to inspect (optional, defaults to latest)")]
    run_id: Option<String>,
}

#[derive(Parser, Debug, Clone, PartialEq)]
struct DoctorArgs {
    #[arg(
        long,
        default_value = "900",
        value_parser = clap::value_parser!(u64).range(60..=86_400),
        help = "Mark running runs as stuck after this many seconds"
    )]
    stuck_after_seconds: u64,
}

#[derive(Parser, Debug, Clone, PartialEq)]
struct CleanupReconcileArgs {
    #[arg(
        long,
        default_value = "10",
        value_parser = clap::value_parser!(u64).range(0..=10_000),
        help = "Keep this many most-recent cleanup-pending run artifacts"
    )]
    keep_latest: u64,
}

type DynError = Box<dyn std::error::Error>;

fn parse_cli_mode() -> CliMode {
    parse_cli_mode_from(std::env::args_os())
}

fn parse_cli_mode_from<I, T>(args: I) -> CliMode
where
    I: IntoIterator<Item = T>,
    T: Into<std::ffi::OsString> + Clone,
{
    let cli = Cli::parse_from(args);
    match cli.command {
        None | Some(CliCommand::Serve) => CliMode::Serve,
        Some(CliCommand::OpsPoll) => CliMode::OpsPoll,
        Some(CliCommand::Run(args)) => CliMode::Run(args),
        Some(CliCommand::Tail(args)) => CliMode::Tail(args),
        Some(CliCommand::Status(args)) => CliMode::Status(args),
        Some(CliCommand::Doctor(args)) => CliMode::Doctor(args),
        Some(CliCommand::CleanupReconcile(args)) => CliMode::CleanupReconcile(args),
        Some(CliCommand::Check) => CliMode::Check,
        Some(CliCommand::Init) => CliMode::Init,
        Some(CliCommand::Bundle) => CliMode::Bundle,
        Some(CliCommand::Observe(args)) => CliMode::Observe(args),
    }
}

#[derive(Debug, Clone, PartialEq)]
enum CliMode {
    Serve,
    OpsPoll,
    Run(RunArgs),
    Tail(TailArgs),
    Status(StatusArgs),
    Doctor(DoctorArgs),
    CleanupReconcile(CleanupReconcileArgs),
    Check,
    Init,
    Bundle,
    Observe(ObserveArgs),
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mode = parse_cli_mode();

    match mode {
        CliMode::OpsPoll => ops_poller::run_ops_poller().await,
        CliMode::Serve => run_server().await,
        CliMode::Run(args) => workflow_runner::run_workflow(args).await,
        CliMode::Status(args) => operator_visibility::run_status_command(args.run_id)
            .map_err(|e| Box::new(std::io::Error::other(e)) as Box<dyn std::error::Error>),
        CliMode::Doctor(args) => operator_visibility::run_doctor_command(args.stuck_after_seconds)
            .map_err(|e| Box::new(std::io::Error::other(e)) as Box<dyn std::error::Error>),
        CliMode::CleanupReconcile(args) => {
            operator_visibility::run_cleanup_reconciler_command(args.keep_latest)
                .map_err(|e| Box::new(std::io::Error::other(e)) as Box<dyn std::error::Error>)
        }
        CliMode::Check => run_quality_gate_command()
            .map_err(|e| Box::new(std::io::Error::other(e)) as Box<dyn std::error::Error>),
        CliMode::Init => runtime_up::run_local_up()
            .map_err(|e| Box::new(std::io::Error::other(e)) as Box<dyn std::error::Error>),
        CliMode::Bundle => runtime_bundle::install_local_binary()
            .map(|path| {
                println!("[oya] Local binary installed: {}", path.display());
            })
            .map_err(|e| Box::new(std::io::Error::other(e)) as Box<dyn std::error::Error>),
        CliMode::Observe(args) => observe::run(args)
            .map_err(|e| Box::new(std::io::Error::other(e)) as Box<dyn std::error::Error>),
        CliMode::Tail(args) => {
            if args.events {
                operator_visibility::run_tail_events_command(
                    args.run_id,
                    args.limit,
                    args.follow,
                    args.interval,
                )
                .map_err(|e| Box::new(std::io::Error::other(e)) as Box<dyn std::error::Error>)
            } else {
                // Need to enter tokio runtime for the tail app
                tail::run_tail(args.interval, args.run_id)
                    .map_err(|e| Box::new(std::io::Error::other(e)) as Box<dyn std::error::Error>)
            }
        }
    }
}

fn run_quality_gate_command() -> Result<(), String> {
    let gate = oya::quality_gate::QualityGate::new(oya::quality_gate::QualityGateConfig::default());
    let result = gate.run().map_err(|error| format!("quality gate execution failed: {}", error))?;

    if result.passed() {
        println!(
            "[oya] quality gate passed: scenarios {}/{}",
            result.scenarios_passed_count, result.scenarios_total_count
        );
        return Ok(());
    }

    Err(format!(
        "quality gate failed on iteration {}/{}: {}",
        result.iteration, result.max_iterations, result.message
    ))
}

async fn run_server() -> Result<(), Box<dyn std::error::Error>> {
    let _shutdown_guard = oya::telemetry::init_default()?;
    report_server_start_messages();
    let endpoint = build_restate_endpoint();
    let bind_addr = resolve_bind_addr()?;
    HttpServer::new(endpoint).listen_and_serve(bind_addr).await;

    Ok(())
}

fn report_server_start_messages() {
    // Initialize OpenTelemetry with dual-layer output:
    // - JSON logs to stdout (for OpenObserve log stream)
    // - OTLP traces to OpenObserve trace backend
    tracing::info!(
        service = "oya-orchestrator",
        port = 9080,
        execution_mode = "real",
        "OYA Orchestrator starting"
    );
    tracing::info!("Using REAL execution: opencode CLI + moon/zjj quality gates");

    tracing::info!(
        workflow_service = "OyaOrchestrator",
        monitor_service = "OyaOpsMonitor",
        usage_service = "OyaUsageTracker",
        "Discovered Restate services before binding"
    );
}

fn build_restate_endpoint() -> Endpoint {
    let workflow_service = OyaOrchestratorImpl.serve();
    let monitor_service = OyaOpsMonitorImpl.serve();
    let service_option_input = default_restate_service_option_input();
    let workflow_service_options = build_restate_service_options(service_option_input);
    let monitor_service_options = build_restate_service_options(service_option_input);

    let workflow_discovery =
        <ServeOyaOrchestrator<OyaOrchestratorImpl> as restate_sdk::service::Discoverable>::discover(
        );
    let monitor_discovery =
        <ServeOyaOpsMonitor<OyaOpsMonitorImpl> as restate_sdk::service::Discoverable>::discover();
    let usage_discovery =
        <ServeOyaUsageTracker<OyaUsageTrackerImpl> as restate_sdk::service::Discoverable>::discover(
        );

    tracing::info!(
        workflow_service = workflow_discovery.name.to_string(),
        workflow_service_type = ?workflow_discovery.ty,
        workflow_handlers = workflow_discovery.handlers.len(),
        monitor_service = monitor_discovery.name.to_string(),
        monitor_service_type = ?monitor_discovery.ty,
        monitor_handlers = monitor_discovery.handlers.len(),
        usage_service = usage_discovery.name.to_string(),
        usage_service_type = ?usage_discovery.ty,
        usage_handlers = usage_discovery.handlers.len(),
        "Discovered Restate services before binding"
    );

    Endpoint::builder()
        .bind_with_options(workflow_service, workflow_service_options)
        .bind_with_options(monitor_service, monitor_service_options)
        .bind(OyaUsageTrackerImpl.serve())
        .build()
}

#[derive(Clone, Copy)]
struct RestateServiceOptionInput {
    inactivity_timeout: std::time::Duration,
    abort_timeout: std::time::Duration,
    retry_policy_max_attempts: u64,
}

fn default_restate_service_option_input() -> RestateServiceOptionInput {
    RestateServiceOptionInput {
        inactivity_timeout: std::time::Duration::from_secs(45 * 60),
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
#[path = "main/tests.rs"]
mod tests;
