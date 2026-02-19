#![deny(clippy::unwrap_used)]
#![deny(clippy::expect_used)]
#![deny(clippy::panic)]
#![forbid(unsafe_code)]

use oya::types::{
    truncate_clean, FailureCategory, Gate, GateSummary, StageName as Stage, StageResult,
    TimelineEntry,
};
use oya::usage::{OyaUsageTracker, OyaUsageTrackerClient, OyaUsageTrackerImpl};
use oya::{
    build_opencode_poll_snapshot, build_zjj_workspace_name, is_retryable_failure,
    parse_opencode_sse_events,
};
use restate_sdk::endpoint::Endpoint;
use restate_sdk::prelude::*;
use serde::{Deserialize, Serialize};
use std::io::Write;
use std::path::PathBuf;
use std::process::Command;

use clap::{Parser, Subcommand};

/// Application-level error type used by orchestration and monitor handlers.
#[derive(Debug)]
pub struct OyaError(String);

impl std::fmt::Display for OyaError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl std::error::Error for OyaError {}

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

#[derive(Debug, Serialize)]
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

        let bead_id = parsed.bead_id.unwrap_or_else(|| "unknown".to_string());
        let context = parsed.context.map_or(String::new(), |s| s);
        let model = parsed.model.unwrap_or_else(|| "zai-coding-plan/glm-5".to_string());
        let run_id = ctx.key().to_string();
        let started_at = deterministic_timestamp(&ctx).await?;

        let initial_state = OrchestratorState {
            status: "running".to_string(),
            stage: "plan".to_string(),
            attempt: 1,
            bead_id: bead_id.clone(),
            context: context.clone(),
            model: model.clone(),
            last_failure: String::new(),
            last_output: String::new(),
            last_prompt: String::new(),
            updated_at: started_at.clone(),
        };
        write_orchestrator_state(&ctx, &initial_state)?;
        set_json_state(
            &ctx,
            "run_request",
            &RunRequestEvent {
                run_id: run_id.clone(),
                bead_id: bead_id.clone(),
                context: context.clone(),
                started_at: started_at.clone(),
            },
        )?;
        append_timeline(
            &ctx,
            TimelineEntry::RunStarted {
                bead_id: bead_id.clone(),
                context: context.clone(),
                at: chrono::DateTime::parse_from_rfc3339(&started_at)
                    .map(|dt| dt.with_timezone(&chrono::Utc))
                    .unwrap_or_else(|_| chrono::Utc::now()),
            },
        )
        .await?;

        tracing::info!("=== RUN {} STARTED ===", run_id);
        tracing::info!("Bead: {}", bead_id);
        tracing::info!("Context: {}", context);
        tracing::info!("Model: {}", model);
        run_pipeline(&ctx, run_id.clone(), bead_id, context, model).await?;

        Ok(run_id)
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
    skip_zjj_workspace: bool,
    skip_zjj_gate: bool,
    repo_root: PathBuf,
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

        Ok(Self { skip_zjj_workspace, skip_zjj_gate, repo_root: PathBuf::from(repo_root_str) })
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

async fn run_pipeline(
    ctx: &WorkflowContext<'_>,
    run_id: String,
    bead_id: String,
    context: String,
    model: String,
) -> Result<(), OyaError> {
    // Load all runtime config deterministically at workflow start
    let config = RuntimeConfig::load(ctx).await?;

    let mut current_stage = Stage::Plan;
    let mut attempt = 1u32;
    let mut last_failure: Option<(FailureCategory, String)> = None;
    let initial_ts =
        deterministic_timestamp(ctx).await.map_err(|_e| OyaError("timestamp error".to_string()))?;
    let mut orchestrator_state = OrchestratorState {
        status: "running".to_string(),
        stage: current_stage.as_str().to_string(),
        attempt,
        bead_id: bead_id.clone(),
        context: context.clone(),
        model: model.clone(),
        last_failure: String::new(),
        last_output: String::new(),
        last_prompt: String::new(),
        updated_at: initial_ts,
    };
    write_orchestrator_state(ctx, &orchestrator_state)?;

    loop {
        let loop_ts = deterministic_timestamp(ctx)
            .await
            .map_err(|_e| OyaError("timestamp error".to_string()))?;
        orchestrator_state.stage = current_stage.as_str().to_string();
        orchestrator_state.attempt = attempt;
        orchestrator_state.status = "running".to_string();
        orchestrator_state.updated_at = loop_ts.clone();
        write_orchestrator_state(ctx, &orchestrator_state)?;

        let stage_start = deterministic_timestamp(ctx)
            .await
            .map_err(|_e| OyaError("timestamp error".to_string()))?;
        let stage_input_key = stage_attempt_key(&current_stage, attempt, "input");
        let failure_snapshot = last_failure.as_ref().map(|(category, message)| FailureSnapshot {
            category: format!("{:?}", category),
            message: truncate_clean(message, 2000),
        });
        set_json_state(
            ctx,
            &stage_input_key,
            &StageInputEvent {
                run_id: run_id.clone(),
                bead_id: bead_id.clone(),
                stage: current_stage.as_str().to_string(),
                attempt,
                context: context.clone(),
                last_failure: failure_snapshot,
                started_at: stage_start.clone(),
            },
        )?;

        let workspace_ts = deterministic_timestamp(ctx)
            .await
            .map_err(|_e| OyaError("timestamp error".to_string()))?;

        let workspace_info = prepare_stage_workspace(
            &run_id,
            &bead_id,
            &current_stage,
            attempt,
            workspace_ts,
            config.skip_zjj_workspace,
            &config.repo_root,
        )?;

        if let Some(ref workspace_event) = workspace_info {
            let workspace_key = stage_attempt_key(&current_stage, attempt, "workspace");
            set_json_state(ctx, &workspace_key, workspace_event)?;
        }

        let started_at_ts = chrono::DateTime::parse_from_rfc3339(&stage_start)
            .map(|dt| dt.with_timezone(&chrono::Utc))
            .unwrap_or_else(|_| chrono::Utc::now());

        append_timeline(
            ctx,
            TimelineEntry::StageStarted {
                stage: current_stage.as_str().to_string(),
                attempt,
                workspace: workspace_info.as_ref().map(|w| w.workspace.clone()),
                at: started_at_ts,
            },
        )
        .await?;

        // 1. Get active model for current stage from OyaUsageTracker
        let tier = current_stage.model_for_stage().as_str().to_string();
        let tracker = ctx.object_client::<OyaUsageTrackerClient>("global");
        let active_model = tracker
            .get_active_model(tier)
            .call()
            .await
            .map_err(|e| OyaError(format!("Failed to get active model from tracker: {}", e)))?;

        let model = active_model.0;
        orchestrator_state.model = model.clone();
        write_orchestrator_state(ctx, &orchestrator_state)?;

        let stage_started_at = chrono::Utc::now();

        let (stage_result, stage_prompt) = match execute_stage_real(
            &run_id,
            &bead_id,
            current_stage.clone(),
            attempt,
            &context,
            &model,
            last_failure.clone(),
            &config,
        )
        .await
        {
            Ok(result) => {
                // 2. Report outcome to tracker for health tracking
                let is_rate_limit =
                    matches!(result.0.failure_category, Some(FailureCategory::RateLimited));
                let report_req = oya::usage::ReportOutcomeRequest {
                    model: model.clone(),
                    success: result.0.passed,
                    is_rate_limit,
                };
                if let Err(e) = tracker.report_outcome(Json(report_req)).call().await {
                    tracing::warn!("Failed to report outcome to tracker: {}", e);
                }
                result
            }
            Err(error) => {
                let fail_ts = deterministic_timestamp(ctx)
                    .await
                    .map_err(|_e| OyaError("timestamp error".to_string()))?;
                orchestrator_state.status = "failed".to_string();
                orchestrator_state.last_failure = format!("Stage execution error: {}", error);
                orchestrator_state.updated_at = fail_ts.clone();
                write_orchestrator_state(ctx, &orchestrator_state)?;
                let fail_ts_dt = chrono::DateTime::parse_from_rfc3339(&fail_ts)
                    .map(|dt| dt.with_timezone(&chrono::Utc))
                    .unwrap_or_else(|_| chrono::Utc::now());
                append_timeline(
                    ctx,
                    TimelineEntry::RunFailed {
                        stage: current_stage.as_str().to_string(),
                        category: "execution_error".to_string(),
                        at: fail_ts_dt,
                    },
                )
                .await?;
                return Ok(());
            }
        };

        orchestrator_state.last_prompt = stage_prompt.clone();
        orchestrator_state.last_output = truncate_clean(&stage_result.output.to_string(), 6000);
        orchestrator_state.last_failure = if stage_result.passed {
            String::new()
        } else {
            truncate_clean(&stage_result.output.to_string(), 6000)
        };
        let result_ts = deterministic_timestamp(ctx)
            .await
            .map_err(|_e| OyaError("timestamp error".to_string()))?;
        orchestrator_state.updated_at = result_ts;
        write_orchestrator_state(ctx, &orchestrator_state)?;

        let prompt_key = format!("prompt_{}_{}", current_stage.as_str(), attempt);
        ctx.set(&prompt_key, stage_prompt);

        let stage_result_key = stage_attempt_key(&current_stage, attempt, "result");
        set_json_state(
            ctx,
            &stage_result_key,
            &StageResultEvent {
                passed: stage_result.passed,
                failure_category: stage_result
                    .failure_category
                    .as_ref()
                    .map(|value| format!("{:?}", value)),
                next_stage: stage_result
                    .next_stage
                    .as_ref()
                    .map(|value| value.as_str().to_string()),
                output: truncate_clean(&stage_result.output.to_string(), 6000),
            },
        )?;

        let stage_log = truncate_clean(&stage_result.output.to_string(), 12000);
        let skill_output_key = stage_attempt_key(&current_stage, attempt, "skill_output");
        set_json_state(
            ctx,
            &skill_output_key,
            &SkillOutputEvent {
                success: stage_result.passed,
                exit_code: if stage_result.passed { 0 } else { 1 },
                full_log: stage_log.clone(),
                feedback: stage_result
                    .failure_category
                    .as_ref()
                    .map_or(String::new(), |value| format!("{:?}", value)),
                contract_document: if current_stage == Stage::Contract {
                    Some(stage_log.clone())
                } else {
                    None
                },
                implementation_code: if current_stage == Stage::Tdd15 {
                    Some(stage_log.clone())
                } else {
                    None
                },
                test_results: if current_stage == Stage::Qa || current_stage == Stage::RedQueen {
                    Some(stage_log.clone())
                } else {
                    None
                },
                adversarial_report: if current_stage == Stage::RedQueen {
                    Some(stage_log)
                } else {
                    None
                },
            },
        )?;

        let mut gate_events = Vec::new();
        for gate in current_stage.gates() {
            let gate_evidence = execute_gate(gate.clone(), &config.repo_root)?;
            let gate_key =
                stage_attempt_key(&current_stage, attempt, &format!("gate_{}", gate.as_str()));
            set_json_state(
                ctx,
                &gate_key,
                &StageResultEvent {
                    passed: gate_evidence.passed,
                    failure_category: None,
                    next_stage: None,
                    output: format!(
                        "command={} exit_code={}\n{}",
                        gate_evidence.command,
                        gate_evidence.exit_code,
                        truncate_clean(&gate_evidence.output, 4000)
                    ),
                },
            )?;

            gate_events.push(GateEventSummary {
                gate: gate.as_str().to_string(),
                state_key: gate_key,
                artifact_id: String::new(),
                passed: gate_evidence.passed,
                exit_code: gate_evidence.exit_code,
            });
        }

        let stage_event_key = stage_attempt_key(&current_stage, attempt, "event");
        let event_ts = deterministic_timestamp(ctx)
            .await
            .map_err(|_e| OyaError("timestamp error".to_string()))?;
        set_json_state(
            ctx,
            &stage_event_key,
            &StageEnvelopeEvent {
                run_id: run_id.clone(),
                bead_id: bead_id.clone(),
                stage: current_stage.as_str().to_string(),
                attempt,
                status: if stage_result.passed {
                    "passed".to_string()
                } else {
                    "failed".to_string()
                },
                input_key: stage_input_key,
                prompt_key,
                result_key: stage_result_key,
                skill_output_key,
                gate_events,
                recorded_at: event_ts.clone(),
            },
        )?;

        let stage_duration_ms =
            (chrono::Utc::now() - stage_started_at).num_milliseconds().max(0) as u64;

        let event_ts_dt = chrono::DateTime::parse_from_rfc3339(&event_ts)
            .map(|dt| dt.with_timezone(&chrono::Utc))
            .unwrap_or_else(|_| chrono::Utc::now());

        if stage_result.passed {
            let gate_summaries: Vec<GateSummary> = current_stage
                .gates()
                .iter()
                .map(|g| GateSummary { gate: g.as_str().to_string(), passed: true })
                .collect();

            append_timeline(
                ctx,
                TimelineEntry::StageCompleted {
                    stage: current_stage.as_str().to_string(),
                    attempt,
                    workspace: workspace_info.as_ref().map(|w| w.workspace.clone()),
                    duration_ms: stage_duration_ms,
                    gates: gate_summaries,
                    at: event_ts_dt,
                },
            )
            .await?;

            match stage_result.next_stage.clone() {
                Some(next_stage) => {
                    current_stage = next_stage;
                    attempt = 1;
                    last_failure = None;
                }
                None => {
                    let shipped_ts = deterministic_timestamp(ctx)
                        .await
                        .map_err(|_e| OyaError("timestamp error".to_string()))?;
                    orchestrator_state.status = "shipped".to_string();
                    orchestrator_state.stage = "none".to_string();
                    orchestrator_state.updated_at = shipped_ts.clone();
                    write_orchestrator_state(ctx, &orchestrator_state)?;
                    let shipped_ts_dt = chrono::DateTime::parse_from_rfc3339(&shipped_ts)
                        .map(|dt| dt.with_timezone(&chrono::Utc))
                        .unwrap_or_else(|_| chrono::Utc::now());
                    append_timeline(
                        ctx,
                        TimelineEntry::RunShipped {
                            total_duration_ms: stage_duration_ms,
                            stages_passed: 8,
                            at: shipped_ts_dt,
                        },
                    )
                    .await?;
                    return Ok(());
                }
            }
        } else {
            let fail_ts = deterministic_timestamp(ctx)
                .await
                .map_err(|_e| OyaError("timestamp error".to_string()))?;

            let fail_ts_dt = chrono::DateTime::parse_from_rfc3339(&fail_ts)
                .map(|dt| dt.with_timezone(&chrono::Utc))
                .unwrap_or_else(|_| chrono::Utc::now());

            let category = stage_result.failure_category.clone();
            let category_str = category
                .as_ref()
                .map(|c| format!("{:?}", c))
                .unwrap_or_else(|| "unknown".to_string());
            let message = truncate_clean(&stage_result.output.to_string(), 500);

            last_failure = category.clone().zip(Some(stage_result.output.to_string()));

            if let Some(non_retryable) =
                category.clone().filter(|value| !is_retryable_failure(value))
            {
                orchestrator_state.status = "failed".to_string();
                orchestrator_state.updated_at = fail_ts.clone();
                write_orchestrator_state(ctx, &orchestrator_state)?;
                append_timeline(
                    ctx,
                    TimelineEntry::StageFailed {
                        stage: current_stage.as_str().to_string(),
                        attempt,
                        workspace: workspace_info.as_ref().map(|w| w.workspace.clone()),
                        duration_ms: stage_duration_ms,
                        category: format!("{:?}", non_retryable),
                        message: message.clone(),
                        retry_scheduled: false,
                        at: fail_ts_dt,
                    },
                )
                .await?;
                append_timeline(
                    ctx,
                    TimelineEntry::RunFailed {
                        stage: current_stage.as_str().to_string(),
                        category: format!("{:?}", non_retryable),
                        at: fail_ts_dt,
                    },
                )
                .await?;
                return Ok(());
            }

            attempt += 1;
            if attempt > current_stage.max_attempts() {
                orchestrator_state.status = "failed".to_string();
                orchestrator_state.updated_at = fail_ts.clone();
                write_orchestrator_state(ctx, &orchestrator_state)?;
                append_timeline(
                    ctx,
                    TimelineEntry::StageFailed {
                        stage: current_stage.as_str().to_string(),
                        attempt: attempt - 1,
                        workspace: workspace_info.as_ref().map(|w| w.workspace.clone()),
                        duration_ms: stage_duration_ms,
                        category: "max_attempts_exceeded".to_string(),
                        message: message.clone(),
                        retry_scheduled: false,
                        at: fail_ts_dt,
                    },
                )
                .await?;
                append_timeline(
                    ctx,
                    TimelineEntry::RunFailed {
                        stage: current_stage.as_str().to_string(),
                        category: "max_attempts_exceeded".to_string(),
                        at: fail_ts_dt,
                    },
                )
                .await?;
                return Ok(());
            }

            append_timeline(
                ctx,
                TimelineEntry::StageFailed {
                    stage: current_stage.as_str().to_string(),
                    attempt: attempt - 1,
                    workspace: workspace_info.as_ref().map(|w| w.workspace.clone()),
                    duration_ms: stage_duration_ms,
                    category: category_str,
                    message,
                    retry_scheduled: true,
                    at: fail_ts_dt,
                },
            )
            .await?;
        }

        tokio::time::sleep(std::time::Duration::from_millis(100)).await;
    }
}

// Re-export from lib for backwards compatibility
// is_retryable_failure imported from oya lib above

async fn execute_stage_real(
    run_id: &str,
    bead_id: &str,
    stage: Stage,
    attempt: u32,
    context: &str,
    model: &str,
    last_failure: Option<(FailureCategory, String)>,
    config: &RuntimeConfig,
) -> Result<(StageResult, String), OyaError> {
    let bead_id = bead_id.to_string();
    let context = context.to_string();
    let model = model.to_string();
    let run_id = run_id.to_string();
    let stage_for_closure = stage.clone();
    let skip_zjj_gate = config.skip_zjj_gate;
    let repo_root = config.repo_root.clone();
    let model_for_closure = model.clone();

    let execution = tokio::task::spawn_blocking(move || match stage_for_closure {
        Stage::Plan
        | Stage::Contract
        | Stage::Tdd15
        | Stage::Qa
        | Stage::RedQueen
        | Stage::GptReview => {
            let failure_context = stage_failure_context(&stage_for_closure, &last_failure);
            let prompt =
                stage_prompt(&stage_for_closure, &bead_id, &context, attempt, &failure_context);
            let (success_message, success_next_stage) = stage_success(&stage_for_closure);
            let checks = stage_checks(&stage_for_closure);
            execute_prompt_stage(
                prompt,
                stage_for_closure.clone(),
                success_message,
                success_next_stage,
                &checks,
                &repo_root,
                &model_for_closure,
            )
        }
        Stage::ShipGate => {
            execute_ship_gate(&bead_id, attempt, &context, &last_failure, skip_zjj_gate, &repo_root)
        }
    })
    .await
    .map_err(|e| OyaError(format!("spawn_blocking failed: {}", e)))??;

    let StageExecution { passed, output, failure_category, next_stage, prompt } = execution;

    let stage_result = StageResult {
        run_id,
        stage,
        attempt,
        passed,
        output: serde_json::json!({ "output": output }),
        failure_category,
        next_stage,
    };

    Ok((stage_result, prompt))
}

struct StageExecution {
    passed: bool,
    output: String,
    failure_category: Option<FailureCategory>,
    next_stage: Option<Stage>,
    prompt: String,
}

#[derive(Clone)]
enum StageCheck {
    Check { failure: FailureCategory, next_stage: Stage },
    Test { failure: FailureCategory, next_stage: Stage },
    Quick { failure: FailureCategory, next_stage: Stage },
}

fn execute_prompt_stage(
    prompt: String,
    opencode_fail_stage: Stage,
    success_message: &str,
    success_next_stage: Option<Stage>,
    checks: &[StageCheck],
    repo_root: &PathBuf,
    model: &str,
) -> Result<StageExecution, OyaError> {
    let (opencode_ok, opencode_output) = run_opencode(&prompt, repo_root, model)?;
    if !opencode_ok {
        let failure_category = oya::classify_opencode_error(&opencode_output)
            .unwrap_or(FailureCategory::OutputParseFailure);

        return Ok(StageExecution {
            passed: false,
            output: opencode_output,
            failure_category: Some(failure_category),
            next_stage: Some(opencode_fail_stage),
            prompt,
        });
    }

    let failed_check = checks.iter().try_fold(None, |found, check| {
        if found.is_some() {
            return Ok(found);
        }

        let next = match check {
            StageCheck::Check { failure, next_stage } => {
                let (ok, output) = run_moon_check(repo_root)?;
                if ok {
                    None
                } else {
                    Some((failure.clone(), output, next_stage.clone()))
                }
            }
            StageCheck::Test { failure, next_stage } => {
                let (ok, output) = run_moon_test(repo_root)?;
                if ok {
                    None
                } else {
                    Some((failure.clone(), output, next_stage.clone()))
                }
            }
            StageCheck::Quick { failure, next_stage } => {
                let (ok, output) = run_moon_quick(repo_root)?;
                if ok {
                    None
                } else {
                    Some((failure.clone(), output, next_stage.clone()))
                }
            }
        };

        Ok(next)
    })?;

    match failed_check {
        Some((failure, output, next_stage)) => Ok(StageExecution {
            passed: false,
            output,
            failure_category: Some(failure),
            next_stage: Some(next_stage),
            prompt,
        }),
        None => Ok(StageExecution {
            passed: true,
            output: success_message.to_string(),
            failure_category: None,
            next_stage: success_next_stage,
            prompt,
        }),
    }
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
    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(timeout_seconds))
        .build()
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
    Ok((passed, format!("{}\n{}", stdout, stderr)))
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
    let timeout_duration = timeout_seconds.to_string();
    let output = Command::new("timeout")
        .arg(&timeout_duration)
        .arg(command_name)
        .args(args)
        .current_dir(repo_root)
        .output()
        .map_err(|e| OyaError(format!("Failed to run {}: {}", command_name, e)))?;

    let stdout = String::from_utf8_lossy(&output.stdout).to_string();
    let stderr = String::from_utf8_lossy(&output.stderr).to_string();
    let exit_code = output.status.code().map_or(-1, |code| code);
    let timed_out = output.status.code() == Some(124);
    let success = output.status.success();
    let duration_ms = start.elapsed().as_millis();

    // Log full output at DEBUG level for detailed troubleshooting
    tracing::debug!(
        stdout = %stdout,
        stderr = %stderr,
        "CLI command detailed output"
    );

    // Log summary at INFO level with all key context
    tracing::info!(
        command = %command_name,
        args = ?args,
        timeout_seconds = timeout_seconds,
        exit_code = exit_code,
        duration_ms = duration_ms,
        stdout_len = stdout.len(),
        stderr_len = stderr.len(),
        timed_out = timed_out,
        success = success,
        "CLI command execution"
    );

    Ok((success, stdout, stderr, exit_code))
}

fn run_opencode(
    prompt: &str,
    repo_root: &PathBuf,
    model: &str,
) -> Result<(bool, String), OyaError> {
    tracing::info!("Running opencode with prompt ({} chars) model={}", prompt.len(), model);
    run_command_with_timeout(
        "opencode",
        &["run", "--format", "json", "--model", model, prompt],
        OPENCODE_TIMEOUT_SECONDS,
        repo_root,
    )
}

fn run_moon_check(repo_root: &PathBuf) -> Result<(bool, String), OyaError> {
    tracing::info!("Running moon :check");
    run_command_with_timeout("moon", &["run", ":check"], MOON_TIMEOUT_SECONDS, repo_root)
}

fn run_moon_test(repo_root: &PathBuf) -> Result<(bool, String), OyaError> {
    tracing::info!("Running moon :test");
    run_command_with_timeout("moon", &["run", ":test"], MOON_TIMEOUT_SECONDS, repo_root)
}

fn run_moon_quick(repo_root: &PathBuf) -> Result<(bool, String), OyaError> {
    tracing::info!("Running moon :quick");
    run_command_with_timeout("moon", &["run", ":quick"], MOON_TIMEOUT_SECONDS, repo_root)
}

fn run_moon_ci(repo_root: &PathBuf) -> Result<(bool, String), OyaError> {
    tracing::info!("Running moon :ci");
    run_command_with_timeout("moon", &["run", ":ci"], MOON_TIMEOUT_SECONDS, repo_root)
}

fn run_zjj_done_dry_run(repo_root: &PathBuf) -> Result<(bool, String), OyaError> {
    tracing::info!("Running zjj done --dry-run");

    let (success, combined) =
        run_command_with_timeout("zjj", &["done", "--dry-run"], ZJJ_TIMEOUT_SECONDS, repo_root)?;

    Ok((success, combined))
}

fn execute_gate(gate: Gate, repo_root: &PathBuf) -> Result<GateEvidence, OyaError> {
    match gate {
        Gate::Compiles => {
            let (passed, stdout, stderr, exit_code) = run_command_with_timeout_with_exit(
                "moon",
                &["run", ":check"],
                MOON_TIMEOUT_SECONDS,
                repo_root,
            )?;
            let output = format!("{}\n{}", stdout, stderr);
            Ok(GateEvidence { command: "moon run :check".to_string(), passed, exit_code, output })
        }
        Gate::TestsPass | Gate::EdgeCases | Gate::NoVulnerabilities => {
            let (passed, stdout, stderr, exit_code) = run_command_with_timeout_with_exit(
                "moon",
                &["run", ":test"],
                MOON_TIMEOUT_SECONDS,
                repo_root,
            )?;
            let output = format!("{}\n{}", stdout, stderr);
            Ok(GateEvidence { command: "moon run :test".to_string(), passed, exit_code, output })
        }
        Gate::ClippyClean | Gate::Security => {
            let (passed, stdout, stderr, exit_code) = run_command_with_timeout_with_exit(
                "moon",
                &["run", ":quick"],
                MOON_TIMEOUT_SECONDS,
                repo_root,
            )?;
            let output = format!("{}\n{}", stdout, stderr);
            Ok(GateEvidence { command: "moon run :quick".to_string(), passed, exit_code, output })
        }
        Gate::MoonCi => {
            let (passed, stdout, stderr, exit_code) = run_command_with_timeout_with_exit(
                "moon",
                &["run", ":ci"],
                MOON_TIMEOUT_SECONDS,
                repo_root,
            )?;
            let output = format!("{}\n{}", stdout, stderr);
            Ok(GateEvidence { command: "moon run :ci".to_string(), passed, exit_code, output })
        }
        Gate::ZjjMergeQueue => {
            let (passed, stdout, stderr, exit_code) = run_command_with_timeout_with_exit(
                "zjj",
                &["done", "--dry-run"],
                ZJJ_TIMEOUT_SECONDS,
                repo_root,
            )?;
            let output = format!("{}\n{}", stdout, stderr);
            Ok(GateEvidence {
                command: "zjj done --dry-run".to_string(),
                passed,
                exit_code,
                output,
            })
        }
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

fn prepare_stage_workspace(
    run_id: &str,
    bead_id: &str,
    stage: &Stage,
    attempt: u32,
    recorded_at: String,
    skip_zjj_workspace: bool,
    repo_root: &PathBuf,
) -> Result<Option<WorkspaceLifecycleEvent>, OyaError> {
    if skip_zjj_workspace || !stage_uses_workspace(stage) {
        return Ok(None);
    }

    let workspace = build_zjj_workspace_name(run_id, stage.as_str(), attempt)
        .map_err(|error| OyaError(format!("Invalid workspace name for stage prep: {}", error)))?;

    let queue_command = format!("zjj queue --add {} --bead {}", workspace, bead_id);
    let (queue_passed, queue_stdout, queue_stderr, queue_exit_code) =
        run_command_with_timeout_with_exit(
            "zjj",
            &["queue", "--add", workspace.as_str(), "--bead", bead_id],
            ZJJ_TIMEOUT_SECONDS,
            repo_root,
        )?;
    let queue_output = format!("{}\n{}", queue_stdout, queue_stderr);
    if !queue_passed {
        return Err(OyaError(format!(
            "zjj queue failed for workspace {} (exit={}): {}",
            workspace,
            queue_exit_code,
            truncate_clean(queue_output.as_str(), 2000)
        )));
    }

    let add_command = format!("zjj add {} --idempotent", workspace);
    let (add_passed, add_stdout, add_stderr, add_exit_code) = run_command_with_timeout_with_exit(
        "zjj",
        &["add", workspace.as_str(), "--idempotent"],
        ZJJ_TIMEOUT_SECONDS,
        repo_root,
    )?;
    let add_output = format!("{}\n{}", add_stdout, add_stderr);
    if !add_passed {
        return Err(OyaError(format!(
            "zjj add failed for workspace {} (exit={}): {}",
            workspace,
            add_exit_code,
            truncate_clean(add_output.as_str(), 2000)
        )));
    }

    Ok(Some(WorkspaceLifecycleEvent {
        workspace,
        queue_command,
        queue_passed,
        queue_exit_code,
        queue_output: truncate_clean(queue_output.as_str(), 4000),
        add_command,
        add_passed,
        add_exit_code,
        add_output: truncate_clean(add_output.as_str(), 4000),
        recorded_at,
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

fn stage_prompt(
    stage: &Stage,
    bead_id: &str,
    context: &str,
    attempt: u32,
    failure_context: &str,
) -> String {
    let header = format!(
        "You are executing stage '{}' for: {}\n\nRequest context: {}\nAttempt: {}\n{}\n\n",
        stage.as_str(),
        bead_id,
        context,
        attempt,
        failure_context
    );

    let body = match stage {
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

fn stage_checks(stage: &Stage) -> Vec<StageCheck> {
    match stage {
        Stage::Plan => vec![StageCheck::Check {
            failure: FailureCategory::CompileFailed,
            next_stage: Stage::Plan,
        }],
        Stage::Contract => vec![StageCheck::Check {
            failure: FailureCategory::CompileFailed,
            next_stage: Stage::Contract,
        }],
        Stage::Tdd15 => vec![
            StageCheck::Check { failure: FailureCategory::CompileFailed, next_stage: Stage::Tdd15 },
            StageCheck::Test { failure: FailureCategory::TestFailed, next_stage: Stage::Tdd15 },
        ],
        Stage::Qa => vec![StageCheck::Test {
            failure: FailureCategory::TestFailed,
            next_stage: Stage::Tdd15,
        }],
        Stage::RedQueen => vec![StageCheck::Test {
            failure: FailureCategory::TestFailed,
            next_stage: Stage::Tdd15,
        }],
        Stage::GptReview => vec![
            StageCheck::Quick {
                failure: FailureCategory::LintFailed,
                next_stage: Stage::GptReview,
            },
            StageCheck::Test { failure: FailureCategory::TestFailed, next_stage: Stage::Tdd15 },
        ],
        Stage::ShipGate => Vec::new(),
    }
}

fn execute_ship_gate(
    _bead_id: &str,
    _attempt: u32,
    _context: &str,
    _last_failure: &Option<(FailureCategory, String)>,
    skip_zjj_gate: bool,
    repo_root: &PathBuf,
) -> Result<StageExecution, OyaError> {
    tracing::info!("SHIP GATE: Running final validation");
    let prompt = "Ship gate executes quality gates only (moon/zjj); no OpenCode prompt".to_string();

    let (ci_ok, ci_output) = run_moon_ci(repo_root)?;
    if !ci_ok {
        return Ok(StageExecution {
            passed: false,
            output: ci_output,
            failure_category: Some(FailureCategory::CompileFailed),
            next_stage: Some(Stage::Tdd15),
            prompt,
        });
    }

    if !skip_zjj_gate {
        let (zjj_ok, zjj_output) = run_zjj_done_dry_run(repo_root)?;
        if !zjj_ok {
            return Ok(StageExecution {
                passed: false,
                output: zjj_output,
                failure_category: Some(FailureCategory::MergeConflict),
                next_stage: Some(Stage::GptReview),
                prompt,
            });
        }
    } else {
        tracing::info!("SHIP GATE: skipping zjj dry-run check (OYA_SKIP_ZJJ_GATE=1)");
    }

    let (quick_ok, quick_output) = run_moon_quick(repo_root)?;
    if !quick_ok {
        return Ok(StageExecution {
            passed: false,
            output: quick_output,
            failure_category: Some(FailureCategory::LintFailed),
            next_stage: Some(Stage::GptReview),
            prompt,
        });
    }

    let (test_ok, test_output) = run_moon_test(repo_root)?;
    if !test_ok {
        return Ok(StageExecution {
            passed: false,
            output: test_output,
            failure_category: Some(FailureCategory::TestFailed),
            next_stage: Some(Stage::Tdd15),
            prompt,
        });
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
            scheme_valid && host_valid && creds_valid
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
    #[arg(long, default_value = "zai-coding-plan/glm-5", help = "OpenCode model to use")]
    model: String,
}

#[derive(Debug, Clone)]
struct WorkflowConfig {
    bead_id: String,
    run_id: String,
    restate_ingress: String,
    restate_admin: String,
    context: String,
    model: String,
    timeout_secs: u64,
    poll_interval_secs: u64,
    repo_root: PathBuf,
    stages: &'static [&'static str],
}

impl WorkflowConfig {
    fn from_args(args: RunArgs, repo_root: PathBuf) -> Self {
        let restate_ingress = args.restate_url.trim_end_matches('/').to_string();
        let restate_admin = restate_ingress.replace(":8080", ":9070");
        Self {
            run_id: args.bead_id.clone(),
            bead_id: args.bead_id,
            restate_ingress,
            restate_admin,
            context: args.context,
            model: args.model,
            timeout_secs: args.timeout,
            poll_interval_secs: args.poll_interval.unwrap_or(5),
            repo_root,
            stages: &["plan", "contract", "tdd15", "qa", "red_queen", "gpt_review", "ship_gate"],
        }
    }
}

#[derive(Debug, Clone)]
struct WorkflowStatus {
    status: String,
    stage: String,
    attempt: u32,
    orchestration_status: String,
    last_failure: String,
}

#[derive(Debug, Clone)]
struct WorkflowResult {
    bead_id: String,
    run_id: String,
    status: String,
    final_stage: String,
    error: Option<String>,
    repo_root: PathBuf,
}

impl WorkflowStatus {
    fn from_query_response(body: &str) -> Option<Self> {
        let response: serde_json::Value = serde_json::from_str(body).ok()?;
        let rows = response.get("rows")?.as_array()?;
        let row = rows.first()?;
        let status = row.get("status").and_then(|s| s.as_str())?.to_string();

        let state_json_str = row.get("state_json").and_then(|s| s.as_str()).unwrap_or("{}");
        let state_outer: serde_json::Value = serde_json::from_str(state_json_str).ok()?;
        let state_str = state_outer.as_str().unwrap_or("{}");
        let state: serde_json::Value = serde_json::from_str(state_str).ok()?;

        Some(Self {
            status,
            stage: state.get("stage").and_then(|s| s.as_str()).unwrap_or("unknown").to_string(),
            attempt: state.get("attempt").and_then(|a| a.as_u64()).unwrap_or(0) as u32,
            orchestration_status: state
                .get("status")
                .and_then(|s| s.as_str())
                .unwrap_or("unknown")
                .to_string(),
            last_failure: state
                .get("last_failure")
                .and_then(|s| s.as_str())
                .unwrap_or("")
                .to_string(),
        })
    }

    fn is_complete(&self) -> bool {
        self.status == "completed"
    }

    fn is_failed(&self) -> bool {
        self.status == "failed"
    }
}

fn find_repo_root() -> Result<PathBuf, String> {
    let current =
        std::env::current_dir().map_err(|e| format!("Failed to get current directory: {}", e))?;

    std::iter::successors(Some(current.as_path()), |p| p.parent())
        .find(|p| p.join(".beads").exists())
        .map(PathBuf::from)
        .ok_or_else(|| {
            format!("No .beads/ directory found in {} or any parent directory", current.display())
        })
}

fn validate_bead_exists(bead_id: &str, repo_root: &PathBuf) -> Result<bool, String> {
    let output = Command::new("br")
        .args(["show", bead_id, "--json"])
        .current_dir(repo_root)
        .output()
        .map_err(|e| format!("Failed to run 'br show {}': {}", bead_id, e))?;

    Ok(output.status.success())
}

async fn run_workflow(args: RunArgs) -> Result<(), Box<dyn std::error::Error>> {
    let repo_root = find_repo_root()
        .map_err(|e| format!("Failed to find repo root (no .beads/ directory found): {}", e))?;

    if !validate_bead_exists(&args.bead_id, &repo_root)? {
        return Err(format!(
            "Bead '{}' not found. Run 'br list' to see available beads.",
            args.bead_id
        )
        .into());
    }

    let config = WorkflowConfig::from_args(args, repo_root);
    execute_workflow(config).await
}

async fn execute_workflow(config: WorkflowConfig) -> Result<(), Box<dyn std::error::Error>> {
    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(30))
        .build()
        .map_err(|e| format!("Failed to build HTTP client: {}", e))?;

    eprintln!(
        "{}",
        serde_json::json!({
            "type": "workflow_starting",
            "bead_id": config.bead_id,
            "run_id": config.run_id,
            "context": config.context,
            "model": config.model,
            "repo_root": config.repo_root.display().to_string(),
            "restate_ingress": config.restate_ingress,
            "restate_admin": config.restate_admin,
            "timeout_seconds": config.timeout_secs,
            "poll_interval_seconds": config.poll_interval_secs,
            "pipeline_stages": config.stages,
            "tool": "oya",
            "action": "run"
        })
    );

    start_workflow(&client, &config).await?;

    eprintln!(
        "{}",
        serde_json::json!({
            "type": "workflow_submitted",
            "bead_id": config.bead_id,
            "run_id": config.run_id,
            "timeout_seconds": config.timeout_secs,
            "poll_interval_seconds": config.poll_interval_secs,
            "message": "Workflow submitted to Restate, polling for completion"
        })
    );

    let final_status = poll_until_complete(&client, &config).await?;

    let result = WorkflowResult {
        bead_id: config.bead_id.clone(),
        run_id: config.run_id.clone(),
        status: final_status.orchestration_status.clone(),
        final_stage: final_status.stage.clone(),
        error: if final_status.last_failure.is_empty() {
            None
        } else {
            Some(final_status.last_failure.clone())
        },
        repo_root: config.repo_root.clone(),
    };

    output_result(&result, &config)?;

    if result.status == "shipped" {
        Ok(())
    } else {
        Err(format!("Workflow ended with status: {}", result.status).into())
    }
}

fn start_workflow<'a>(
    client: &'a reqwest::Client,
    config: &'a WorkflowConfig,
) -> std::pin::Pin<Box<dyn std::future::Future<Output = Result<(), Box<dyn std::error::Error>>> + 'a>>
{
    let start_url =
        format!("{}/OyaOrchestrator/{}/start/send", config.restate_ingress, config.run_id);
    let payload = serde_json::json!({
        "bead_id": config.bead_id,
        "context": config.context,
        "model": config.model
    });

    Box::pin(async move {
        let response = client
            .post(&start_url)
            .header("content-type", "application/json")
            .json(&payload)
            .send()
            .await
            .map_err(|e| format!("Failed to start workflow: {}", e))?;

        if !response.status().is_success() {
            let status = response.status();
            let body = response.text().await.unwrap_or_else(|_| "<no body>".to_string());
            return Err(format!("Failed to start workflow (HTTP {}): {}", status, body).into());
        }

        Ok(())
    })
}

fn poll_until_complete<'a>(
    client: &'a reqwest::Client,
    config: &'a WorkflowConfig,
) -> std::pin::Pin<
    Box<dyn std::future::Future<Output = Result<WorkflowStatus, Box<dyn std::error::Error>>> + 'a>,
> {
    let start_time = std::time::Instant::now();
    let timeout_duration = std::time::Duration::from_secs(config.timeout_secs);
    let poll_interval = std::time::Duration::from_secs(config.poll_interval_secs);

    Box::pin(poll_iteration(client, config, start_time, timeout_duration, poll_interval, None))
}

fn poll_iteration<'a>(
    client: &'a reqwest::Client,
    config: &'a WorkflowConfig,
    start_time: std::time::Instant,
    timeout_duration: std::time::Duration,
    poll_interval: std::time::Duration,
    last_status: Option<WorkflowStatus>,
) -> std::pin::Pin<
    Box<dyn std::future::Future<Output = Result<WorkflowStatus, Box<dyn std::error::Error>>> + 'a>,
> {
    Box::pin(async move {
        if start_time.elapsed() > timeout_duration {
            return Err(format!("Workflow timed out after {} seconds", config.timeout_secs).into());
        }

        let status = fetch_workflow_status(client, config).await?;

        let status_changed = last_status.as_ref().is_none_or(|last| {
            last.status != status.status
                || last.stage != status.stage
                || last.attempt != status.attempt
        });

        if status_changed {
            let elapsed_secs = start_time.elapsed().as_secs();
            eprintln!(
                "{}",
                serde_json::json!({
                    "type": "stage_progress",
                    "bead_id": config.bead_id,
                    "run_id": config.run_id,
                    "invocation_status": status.status,
                    "orchestration_status": status.orchestration_status,
                    "current_stage": status.stage,
                    "attempt": status.attempt,
                    "elapsed_seconds": elapsed_secs,
                    "remaining_seconds": config.timeout_secs.saturating_sub(elapsed_secs),
                    "last_failure": if status.last_failure.is_empty() { serde_json::Value::Null } else { serde_json::json!(status.last_failure) },
                    "pipeline_stages": config.stages,
                    "repo_root": config.repo_root.display().to_string()
                })
            );
        }

        if status.is_complete() {
            return Ok(status);
        }

        if status.is_failed() {
            return Err(format!("Workflow failed: {}", status.last_failure).into());
        }

        tokio::time::sleep(poll_interval).await;

        poll_iteration(client, config, start_time, timeout_duration, poll_interval, Some(status))
            .await
    })
}

fn fetch_workflow_status<'a>(
    client: &'a reqwest::Client,
    config: &'a WorkflowConfig,
) -> std::pin::Pin<
    Box<dyn std::future::Future<Output = Result<WorkflowStatus, Box<dyn std::error::Error>>> + 'a>,
> {
    let query_payload = serde_json::json!({
        "query": format!(
            "select i.status, s.value_utf8 as state_json from sys_invocation i \
             left join state s on s.service_name = i.target_service_name \
             and s.service_key = i.target_service_key and s.key = 'state' \
             where i.target_service_name = 'OyaOrchestrator' \
             and i.target_service_key = '{}' \
             and i.target_handler_name = 'start' \
             order by i.modified_at desc limit 1",
            config.run_id
        )
    });
    let restate_admin = config.restate_admin.clone();

    Box::pin(async move {
        let response = client
            .post(format!("{}/query", restate_admin))
            .header("content-type", "application/json")
            .header("accept", "application/json")
            .json(&query_payload)
            .send()
            .await
            .map_err(|e| format!("Query request failed: {}", e))?;

        let body = response.text().await.unwrap_or_else(|_| "{}".to_string());

        WorkflowStatus::from_query_response(&body).ok_or_else(|| "No workflow status found".into())
    })
}

fn output_result(
    result: &WorkflowResult,
    config: &WorkflowConfig,
) -> Result<(), Box<dyn std::error::Error>> {
    println!(
        "{}",
        serde_json::json!({
            "type": "workflow_result",
            "bead_id": result.bead_id,
            "run_id": result.run_id,
            "status": result.status,
            "final_stage": result.final_stage,
            "error": result.error,
            "repo_root": result.repo_root.display().to_string(),
            "pipeline_stages": config.stages,
            "is_success": result.status == "shipped",
            "next_steps": if result.status == "shipped" {
                serde_json::json!([
                    {"action": "review_code", "path": format!("{}/src/", result.repo_root.display()), "description": "Review generated source code"},
                    {"action": "run_ci", "command": "moon run :ci", "description": "Run CI quality gates"},
                    {"action": "merge_workspace", "command": "zjj done", "description": "Merge zjj workspace to main"},
                    {"action": "close_bead", "command": format!("br close {}", result.bead_id), "description": "Close the bead issue"}
                ])
            } else {
                serde_json::json!([
                    {"action": "review_error", "description": "Review the error output above to understand the failure"},
                    {"action": "fix_issue", "path": format!("{}/src/", result.repo_root.display()), "description": "Fix the underlying issue in the source code"},
                    {"action": "rerun", "command": format!("oya run {}", result.bead_id), "description": "Re-run the workflow after fixing"}
                ])
            }
        })
    );
    Ok(())
}

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
        CliMode::OpsPoll => run_ops_poller().await,
        CliMode::Serve => run_server().await,
        CliMode::Run(args) => run_workflow(args).await,
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
    let workflow_service_options = restate_sdk::endpoint::ServiceOptions::new()
        .inactivity_timeout(std::time::Duration::from_secs(30 * 60))
        .abort_timeout(std::time::Duration::from_secs(5 * 60))
        .retry_policy_max_attempts(2)
        .retry_policy_kill_on_max_attempts();
    let monitor_service_options = restate_sdk::endpoint::ServiceOptions::new()
        .inactivity_timeout(std::time::Duration::from_secs(30 * 60))
        .abort_timeout(std::time::Duration::from_secs(5 * 60))
        .retry_policy_max_attempts(2)
        .retry_policy_kill_on_max_attempts();
    let endpoint = Endpoint::builder()
        .bind_with_options(workflow_service, workflow_service_options)
        .bind_with_options(monitor_service, monitor_service_options)
        .bind(OyaUsageTrackerImpl.serve())
        .build();

    let bind_addr = resolve_bind_addr()?;
    HttpServer::new(endpoint).listen_and_serve(bind_addr).await;

    Ok(())
}

async fn run_ops_poller() -> Result<(), Box<dyn std::error::Error>> {
    let config = opencode_config()?;
    let interval_ms: u64 = std::env::var("OYA_POLL_INTERVAL_MS")
        .ok()
        .and_then(|value| value.parse::<u64>().ok())
        .map_or(2000, |value: u64| value.clamp(500, 30000));

    let mut stderr = std::io::stderr().lock();
    writeln!(stderr, "[oya:ops-poll] source={} interval_ms={}", config.base_url, interval_ms)
        .map_err(|error| OyaError(format!("Failed to write poller banner: {}", error)))?;
    writeln!(stderr, "[oya:ops-poll] columns: ts | busy | perm | quest | event_preview")
        .map_err(|error| OyaError(format!("Failed to write poller banner: {}", error)))?;

    let client = reqwest::Client::builder().timeout(std::time::Duration::from_secs(10)).build()?;

    loop {
        match poll_opencode_status(&client, &config).await {
            Ok(status_line) => {
                let mut stdout = std::io::stdout().lock();
                writeln!(stdout, "{}", status_line).map_err(|error| {
                    OyaError(format!("Failed to write poll status line: {}", error))
                })?;
            }
            Err(error) => {
                let mut stderr = std::io::stderr().lock();
                writeln!(stderr, "[oya:ops-poll] error: {}", error).map_err(|io_error| {
                    OyaError(format!("Failed to write poll error: {}", io_error))
                })?;
            }
        }
        tokio::time::sleep(std::time::Duration::from_millis(interval_ms)).await;
    }
}

async fn poll_opencode_status(
    client: &reqwest::Client,
    config: &OpenCodeConfig,
) -> Result<String, OyaError> {
    let status_url = format!("{}/session/status", config.base_url.trim_end_matches('/'));
    let perm_url = format!("{}/permission", config.base_url.trim_end_matches('/'));
    let question_url = format!("{}/question", config.base_url.trim_end_matches('/'));

    let status_raw =
        fetch_text_with_client(client, &status_url, config.password.as_deref()).await?;
    let perm_raw = fetch_text_with_client(client, &perm_url, config.password.as_deref()).await?;
    let question_raw =
        fetch_text_with_client(client, &question_url, config.password.as_deref()).await?;

    let snapshot = build_opencode_poll_snapshot(&status_raw, &perm_raw, &question_raw)
        .map_err(|error| OyaError(format!("Parse failed: {}", error)))?;

    let busy_preview = if snapshot.busy_sessions.is_empty() {
        "-".to_string()
    } else if snapshot.busy_sessions.len() <= 3 {
        snapshot.busy_sessions.join(",")
    } else {
        format!("{},...+{}", snapshot.busy_sessions[0], snapshot.busy_sessions.len() - 1)
    };

    let ts = chrono::Utc::now().format("%H:%M:%S%.3f");
    Ok(format!(
        "{} | {} | {} | {}",
        ts, busy_preview, snapshot.pending_permissions, snapshot.pending_questions
    ))
}

async fn fetch_text_with_client(
    client: &reqwest::Client,
    url: &str,
    password: Option<&str>,
) -> Result<String, OyaError> {
    let request = password
        .map_or_else(|| client.get(url), |pwd| client.get(url).basic_auth("opencode", Some(pwd)));

    let response = request
        .send()
        .await
        .map_err(|error| OyaError(format!("Request failed for {}: {}", url, error)))?;

    let status = response.status();
    let text = response
        .text()
        .await
        .map_err(|error| OyaError(format!("Read failed for {}: {}", url, error)))?;

    if !status.is_success() {
        return Err(OyaError(format!(
            "Status {} for {}: {}",
            status.as_u16(),
            url,
            truncate_clean(&text, 200)
        )));
    }

    Ok(text)
}
