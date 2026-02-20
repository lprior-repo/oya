#![deny(clippy::unwrap_used)]
#![deny(clippy::expect_used)]
#![deny(clippy::panic)]
#![deny(clippy::too_many_lines)]
#![deny(clippy::too_many_arguments)]
#![forbid(unsafe_code)]

use oya::build_opencode_poll_snapshot;
use oya::types::{truncate_clean, FailureCategory, Gate, StageName as Stage, TimelineEntry};
use oya::usage::ServeOyaUsageTracker;
use oya::usage::{OyaUsageTracker, OyaUsageTrackerImpl};
use restate_sdk::endpoint::Endpoint;
use restate_sdk::prelude::*;
use std::path::PathBuf;

use clap::{Parser, Subcommand};

mod ops_poller;
mod orchestrator_types;
mod pipeline;
mod runtime_tools;
mod stage_executor;
mod stage_runtime;
mod workflow_runner;

use orchestrator_types::*;
use pipeline::{
    execute_and_accumulate_stage, init_pipeline_state, parse_rfc3339_deterministic,
    persist_stage_artifact, pipeline_input, RuntimeConfig, PipelineRunInput, PipelineState,
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
    let model =
        parsed.model.map_or_else(|| "zai-coding-plan/glm-5".to_string(), std::convert::identity);
    let started_at = pipeline::deterministic_timestamp(ctx)
        .await
        .map_err(|_| OyaError("timestamp error".to_string()))?;
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
        // Execute stage and accumulate all data into single artifact (no Restate sets during execution)
        let artifact = execute_and_accumulate_stage(
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
        .await?;

        // Persist single artifact after stage completes (only Restate set per stage)
        persist_stage_artifact(ctx, &artifact).await?;

        // Update orchestrator state for recovery
        state.orchestrator.stage = artifact.stage.clone();
        state.orchestrator.attempt = artifact.attempt;
        state.orchestrator.updated_at = artifact.timing.completed_at.clone();

        // Determine next action based on artifact status
        match artifact.status {
            orchestrator_types::StageStatus::Completed => {
                // Stage passed - move to next stage or complete
                if let Some(next_stage) = state.current_stage.next() {
                    state.current_stage = next_stage;
                    state.attempt = 1;
                    state.last_failure = None;
                } else {
                    // All stages complete - mark run as shipped
                    return mark_run_completed(ctx, state, &artifact).await;
                }
            }
            orchestrator_types::StageStatus::Failed => {
                // Stage failed - check retry count
                if state.attempt >= state.current_stage.max_attempts() {
                    return mark_run_failed(ctx, state, &artifact).await;
                }
                // Retry same stage
                state.attempt += 1;
                state.last_failure = Some((
                    oya::types::FailureCategory::OutputParseFailure,
                    artifact.output.full_log.clone(),
                ));
            }
        }

        tokio::time::sleep(std::time::Duration::from_millis(100)).await;
    }
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
#[path = "main/tests.rs"]
mod tests;
