#![deny(clippy::unwrap_used)]
#![deny(clippy::expect_used)]
#![deny(clippy::panic)]
#![deny(clippy::too_many_lines)]
#![deny(clippy::too_many_arguments)]
#![forbid(unsafe_code)]

use oya::build_opencode_poll_snapshot;
use oya::types::{
    truncate_clean, FailureCategory, StageFailure, StageName as Stage, TimelineEntry,
};
use oya::usage::{
    is_rate_limit_failure, tier_for_stage, OyaUsageTracker, OyaUsageTrackerClient,
    OyaUsageTrackerImpl, ReportOutcomeRequest, ServeOyaUsageTracker,
};
use restate_sdk::endpoint::Endpoint;
use restate_sdk::prelude::*;
use std::path::{Path, PathBuf};

const USAGE_TRACKER_KEY: &str = "global";

use clap::{Parser, Subcommand};

mod ops_poller;
mod orchestrator_types;
mod pipeline;
mod runtime_tools;
mod runtime_up;
mod stage_executor;
mod stage_runtime;
mod tail;
mod workflow_runner;

use orchestrator_types::*;
use pipeline::{
    execute_and_accumulate_stage, init_pipeline_state, parse_rfc3339_deterministic,
    persist_stage_artifact, pipeline_input, PipelineRunInput, PipelineState, RuntimeConfig,
    StageExecutionInput,
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

    // Get model from request or resolve via usage tracker for Plan stage tier
    let model = match parsed.model {
        Some(m) => m,
        None => resolve_model_for_stage(ctx, &Stage::Plan).await?,
    };

    let started_at = pipeline::deterministic_timestamp(ctx)
        .await
        .map_err(|_| OyaError("timestamp error".to_string()))?;
    Ok(StartContext { run_id: ctx.key().to_string(), bead_id, context, model, started_at })
}

/// Resolve the active model for a stage via the OyaUsageTracker VirtualObject.
/// Uses workflow object client calls to read service state.
async fn resolve_model_for_stage(
    ctx: &WorkflowContext<'_>,
    stage: &Stage,
) -> Result<String, OyaError> {
    let tier = tier_for_stage(stage).to_string();
    ctx.object_client::<OyaUsageTrackerClient>(USAGE_TRACKER_KEY)
        .get_active_model(tier)
        .call()
        .await
        .map(|result| result.0)
        .map_err(|error| OyaError(format!("usage tracker get_active_model failed: {}", error)))
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
        let artifact = run_pipeline_stage(ctx, config, input, state).await?;
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
            handle_failed_stage(ctx, state, artifact).await
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

    if let Some(next_stage) = state.current_stage.next() {
        state.current_stage = next_stage;
        state.attempt = 1;
        state.last_failure = None;
        // Resolve model for the next stage's tier
        state.orchestrator.model = resolve_model_for_stage(ctx, &state.current_stage).await?;
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
        state.orchestrator.model = resolve_model_for_stage(ctx, &state.current_stage).await?;
        return Ok(true);
    }

    mark_run_completed(ctx, state, artifact).await?;
    Ok(false)
}

async fn handle_failed_stage(
    ctx: &WorkflowContext<'_>,
    state: &mut PipelineState,
    artifact: &orchestrator_types::StageArtifact,
) -> Result<bool, OyaError> {
    let failure_category = parse_failure_category(&artifact.failure_category)
        .unwrap_or(FailureCategory::OutputParseFailure);
    let is_rate_limit = is_rate_limit_failure(&failure_category);

    // Report failure outcome to the usage tracker
    report_stage_outcome(ctx, &state.orchestrator.model, false, is_rate_limit).await?;

    if let Some(next_stage) = parse_next_stage(&artifact.next_stage) {
        if next_stage != state.current_stage {
            state.current_stage = next_stage;
            state.attempt = 1;
            // Resolve model for the new stage's tier
            state.orchestrator.model = resolve_model_for_stage(ctx, &state.current_stage).await?;
            return Ok(true);
        }
    }

    if !should_retry_after_failure(state) {
        mark_run_failed(ctx, state, artifact).await?;
        return Ok(false);
    }

    state.attempt += 1;
    Ok(true)
}

fn should_retry_after_failure(state: &PipelineState) -> bool {
    state.last_failure.as_ref().is_some_and(|failure| {
        failure.retryable && state.attempt < state.current_stage.max_attempts()
    })
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
        next_stage: Stage::GptReview,
    },
    LandingStepTemplate {
        id: "zjj_done",
        label: "zjj done",
        program: "zjj",
        args: &["done"],
        timeout_seconds: 120,
        failure_category: FailureCategory::MergeConflict,
        next_stage: Stage::GptReview,
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
        next_stage: Stage::GptReview,
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
        next_stage: Stage::GptReview,
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
    let started_at = pipeline::deterministic_timestamp(ctx).await.map_err(|error| {
        landing_failure_from_step(&step, format!("{} timestamp failed: {}", step.label, error))
    })?;

    let result = run_landing_command(ctx, repo_root, &step).await?;

    let completed_at = pipeline::deterministic_timestamp(ctx).await.map_err(|error| {
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
        next_stage: Stage::GptReview,
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
        "output_parse_failure" => Some(FailureCategory::OutputParseFailure),
        "max_attempts_exceeded" => Some(FailureCategory::MaxAttemptsExceeded),
        _ => None,
    })
}

async fn mark_run_completed(
    ctx: &WorkflowContext<'_>,
    state: &mut PipelineState,
    final_artifact: &orchestrator_types::StageArtifact,
) -> Result<(), OyaError> {
    let completed_at = ctx
        .run(|| async { Ok::<_, HandlerError>(chrono::Utc::now().to_rfc3339()) })
        .await
        .map_err(|e| OyaError(format!("timestamp failed: {}", e)))?;

    state.orchestrator.status = "shipped".to_string();
    state.orchestrator.stage = "none".to_string();
    state.orchestrator.updated_at = completed_at.clone();
    orchestrator_types::write_orchestrator_state(ctx, &state.orchestrator)?;

    // Build lean timeline as JSON array
    let timeline = serde_json::json!([
        {"event": "RunStarted", "at": state.orchestrator.updated_at},
        {"event": "RunShipped", "at": completed_at, "total_duration_ms": final_artifact.timing.duration_ms}
    ]);

    orchestrator_types::set_timeline_once(ctx, &timeline.to_string())
}

async fn mark_run_failed(
    ctx: &WorkflowContext<'_>,
    state: &mut PipelineState,
    artifact: &orchestrator_types::StageArtifact,
) -> Result<(), OyaError> {
    state.orchestrator.status = "failed".to_string();
    state.orchestrator.updated_at = artifact.timing.completed_at.clone();
    orchestrator_types::write_orchestrator_state(ctx, &state.orchestrator)?;

    // Build lean timeline as JSON array
    let timeline = serde_json::json!([
        {"event": "RunStarted", "at": state.orchestrator.updated_at},
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
    #[command(about = "Bootstrap local runtime (restate + opencode + oya service)", alias = "up")]
    Init,
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

#[derive(Parser, Debug, Clone, PartialEq)]
struct TailArgs {
    #[arg(long, default_value = "2", help = "Refresh interval in seconds")]
    interval: u64,
    #[arg(help = "Filter to specific run ID (optional)")]
    run_id: Option<String>,
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
        Some(CliCommand::Init) => CliMode::Init,
    }
}

#[derive(Debug, Clone, PartialEq)]
enum CliMode {
    Serve,
    OpsPoll,
    Run(RunArgs),
    Tail(TailArgs),
    Init,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mode = parse_cli_mode();

    match mode {
        CliMode::OpsPoll => ops_poller::run_ops_poller().await,
        CliMode::Serve => run_server().await,
        CliMode::Run(args) => workflow_runner::run_workflow(args).await,
        CliMode::Init => runtime_up::run_local_up()
            .map_err(|e| Box::new(std::io::Error::other(e)) as Box<dyn std::error::Error>),
        CliMode::Tail(args) => {
            // Need to enter tokio runtime for the tail app
            tail::run_tail(args.interval, args.run_id)
                .map_err(|e| Box::new(std::io::Error::other(e)) as Box<dyn std::error::Error>)
        }
    }
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
#[path = "main/tests.rs"]
mod tests;
