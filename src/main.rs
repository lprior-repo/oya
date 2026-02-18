#![deny(clippy::unwrap_used)]
#![deny(clippy::expect_used)]
#![deny(clippy::panic)]
#![forbid(unsafe_code)]
// Allow explicit match patterns for Option/Result defaults - preferred over unwrap_or*
// per functional-rust-generator skill (which explicitly lists 'match' as preferred)
#![allow(clippy::manual_unwrap_or)]
#![allow(clippy::manual_unwrap_or_default)]
#![allow(clippy::unnecessary_option_map_or_else)]

use oya::types::{FailureCategory, Gate, StageName as Stage, StageResult};
use oya::{
    build_opencode_poll_snapshot, build_zjj_workspace_name, is_retryable_failure,
    parse_opencode_sse_events,
};
use restate_sdk::endpoint::Endpoint;
use restate_sdk::http_server::HttpServer;
use restate_sdk::prelude::*;
use serde::{Deserialize, Serialize};
use std::io::Write;
use std::path::PathBuf;
use std::process::Command;

use clap::{Parser, Subcommand};

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
}

#[derive(Debug, Deserialize)]
/// Request body for polling OpenCode event stream snapshots.
pub struct OpsMonitorEventRequest {
    max_events: Option<usize>,
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

#[derive(Debug, Serialize)]
struct TimelineEvent {
    at: String,
    event: String,
    stage: Option<String>,
    attempt: Option<u32>,
    detail: Option<String>,
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

pub struct OyaOrchestratorImpl;
pub struct OyaOpsMonitorImpl;

impl OyaOrchestrator for OyaOrchestratorImpl {
    async fn start(
        &self,
        ctx: WorkflowContext<'_>,
        request: Json<serde_json::Value>,
    ) -> Result<String, HandlerError> {
        let parsed = parse_start_request(request.0)?;

        let bead_id = match parsed.bead_id {
            Some(s) => s,
            None => "unknown".to_string(),
        };
        let context = parsed.context.map_or(String::new(), |s| s);
        let run_id = ctx.key().to_string();
        let started_at = deterministic_timestamp(&ctx).await?;

        let initial_state = OrchestratorState {
            status: "running".to_string(),
            stage: "research".to_string(),
            attempt: 1,
            bead_id: bead_id.clone(),
            context: context.clone(),
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
            TimelineEvent {
                at: started_at,
                event: "run_accepted".to_string(),
                stage: Some("research".to_string()),
                attempt: Some(1),
                detail: None,
            },
        )
        .await?;

        tracing::info!("=== RUN {} STARTED ===", run_id);
        tracing::info!("Bead: {}", bead_id);
        tracing::info!("Context: {}", context);
        run_pipeline(&ctx, run_id.clone(), bead_id, context).await?;

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
) -> Result<(), OyaError> {
    // Load all runtime config deterministically at workflow start
    let config = RuntimeConfig::load(ctx).await?;

    let mut current_stage = Stage::Research;
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
            message: truncate_text(message, 2000),
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
        append_timeline(
            ctx,
            TimelineEvent {
                at: stage_start,
                event: "stage_start".to_string(),
                stage: Some(current_stage.as_str().to_string()),
                attempt: Some(attempt),
                detail: None,
            },
        )
        .await?;

        let workspace_ts = deterministic_timestamp(ctx)
            .await
            .map_err(|_e| OyaError("timestamp error".to_string()))?;
        if let Some(workspace_event) = prepare_stage_workspace(
            &run_id,
            &bead_id,
            &current_stage,
            attempt,
            workspace_ts,
            config.skip_zjj_workspace,
            &config.repo_root,
        )? {
            let workspace_key = stage_attempt_key(&current_stage, attempt, "workspace");
            set_json_state(ctx, &workspace_key, &workspace_event)?;
            append_timeline(
                ctx,
                TimelineEvent {
                    at: workspace_event.recorded_at.clone(),
                    event: "stage_workspace_ready".to_string(),
                    stage: Some(current_stage.as_str().to_string()),
                    attempt: Some(attempt),
                    detail: Some(format!(
                        "workspace={} queue_exit={} add_exit={}",
                        workspace_event.workspace,
                        workspace_event.queue_exit_code,
                        workspace_event.add_exit_code
                    )),
                },
            )
            .await?;
        }

        let (stage_result, stage_prompt) = match execute_stage_real(
            &run_id,
            &bead_id,
            current_stage.clone(),
            attempt,
            &context,
            last_failure.clone(),
            &config,
        )
        .await
        {
            Ok(result) => result,
            Err(error) => {
                let fail_ts = deterministic_timestamp(ctx)
                    .await
                    .map_err(|_e| OyaError("timestamp error".to_string()))?;
                orchestrator_state.status = "failed".to_string();
                orchestrator_state.last_failure = format!("Stage execution error: {}", error);
                orchestrator_state.updated_at = fail_ts.clone();
                write_orchestrator_state(ctx, &orchestrator_state)?;
                append_timeline(
                    ctx,
                    TimelineEvent {
                        at: fail_ts,
                        event: "run_failed_stage_execution".to_string(),
                        stage: Some(current_stage.as_str().to_string()),
                        attempt: Some(attempt),
                        detail: Some(error.to_string()),
                    },
                )
                .await?;
                return Ok(());
            }
        };

        orchestrator_state.last_prompt = stage_prompt.clone();
        orchestrator_state.last_output = truncate_text(&stage_result.output.to_string(), 6000);
        orchestrator_state.last_failure = if stage_result.passed {
            String::new()
        } else {
            truncate_text(&stage_result.output.to_string(), 6000)
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
                output: truncate_text(&stage_result.output.to_string(), 6000),
            },
        )?;

        let stage_log = truncate_text(&stage_result.output.to_string(), 12000);
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
                        truncate_text(&gate_evidence.output, 4000)
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

        if stage_result.passed {
            append_timeline(
                ctx,
                TimelineEvent {
                    at: event_ts,
                    event: "stage_pass".to_string(),
                    stage: Some(current_stage.as_str().to_string()),
                    attempt: Some(attempt),
                    detail: None,
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
                    append_timeline(
                        ctx,
                        TimelineEvent {
                            at: shipped_ts,
                            event: "run_shipped".to_string(),
                            stage: Some("ship_gate".to_string()),
                            attempt: Some(attempt),
                            detail: None,
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
            append_timeline(
                ctx,
                TimelineEvent {
                    at: fail_ts,
                    event: "stage_fail".to_string(),
                    stage: Some(current_stage.as_str().to_string()),
                    attempt: Some(attempt),
                    detail: None,
                },
            )
            .await?;

            let category = stage_result.failure_category.clone();
            last_failure = category.clone().zip(Some(stage_result.output.to_string()));

            if let Some(non_retryable) =
                category.clone().filter(|value| !is_retryable_failure(value))
            {
                let nr_fail_ts = deterministic_timestamp(ctx)
                    .await
                    .map_err(|_e| OyaError("timestamp error".to_string()))?;
                orchestrator_state.status = "failed".to_string();
                orchestrator_state.updated_at = nr_fail_ts.clone();
                write_orchestrator_state(ctx, &orchestrator_state)?;
                append_timeline(
                    ctx,
                    TimelineEvent {
                        at: nr_fail_ts,
                        event: "run_failed_non_retryable".to_string(),
                        stage: Some(current_stage.as_str().to_string()),
                        attempt: Some(attempt),
                        detail: Some(format!("{:?}", non_retryable)),
                    },
                )
                .await?;
                return Ok(());
            }

            attempt += 1;
            if attempt > current_stage.max_attempts() {
                let max_fail_ts = deterministic_timestamp(ctx)
                    .await
                    .map_err(|_e| OyaError("timestamp error".to_string()))?;
                orchestrator_state.status = "failed".to_string();
                orchestrator_state.updated_at = max_fail_ts.clone();
                write_orchestrator_state(ctx, &orchestrator_state)?;
                append_timeline(
                    ctx,
                    TimelineEvent {
                        at: max_fail_ts,
                        event: "run_failed_max_attempts".to_string(),
                        stage: Some(current_stage.as_str().to_string()),
                        attempt: Some(attempt),
                        detail: None,
                    },
                )
                .await?;
                return Ok(());
            }

            let retry_ts = deterministic_timestamp(ctx)
                .await
                .map_err(|_e| OyaError("timestamp error".to_string()))?;
            append_timeline(
                ctx,
                TimelineEvent {
                    at: retry_ts,
                    event: "stage_retry".to_string(),
                    stage: Some(current_stage.as_str().to_string()),
                    attempt: Some(attempt),
                    detail: Some(format!("next_attempt={}", attempt)),
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
    last_failure: Option<(FailureCategory, String)>,
    config: &RuntimeConfig,
) -> Result<(StageResult, String), OyaError> {
    let bead_id = bead_id.to_string();
    let context = context.to_string();
    let run_id = run_id.to_string();
    let stage_for_closure = stage.clone();
    let skip_zjj_gate = config.skip_zjj_gate;
    let repo_root = config.repo_root.clone();

    let execution = tokio::task::spawn_blocking(move || match stage_for_closure {
        Stage::Research
        | Stage::Plan
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
) -> Result<StageExecution, OyaError> {
    let (opencode_ok, opencode_output) = run_opencode(&prompt, repo_root)?;
    if !opencode_ok {
        return Ok(StageExecution {
            passed: false,
            output: opencode_output,
            failure_category: Some(FailureCategory::OutputParseFailure),
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
    let base_url = match std::env::var("OYA_OPENCODE_BASE_URL")
        .ok()
        .filter(|value| !value.trim().is_empty())
    {
        Some(s) => s,
        None => "http://127.0.0.1:4097".to_string(),
    };

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
            truncate_text(text.as_str(), 4000)
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
    let (passed, output, _exit_code) =
        run_command_with_timeout_with_exit(command_name, args, timeout_seconds, repo_root)?;
    Ok((passed, output))
}

fn run_command_with_timeout_with_exit(
    command_name: &str,
    args: &[&str],
    timeout_seconds: u64,
    repo_root: &PathBuf,
) -> Result<(bool, String, i32), OyaError> {
    let timeout_duration = timeout_seconds.to_string();
    let output = Command::new("timeout")
        .arg(timeout_duration)
        .arg(command_name)
        .args(args)
        .current_dir(repo_root)
        .output()
        .map_err(|e| OyaError(format!("Failed to run {}: {}", command_name, e)))?;

    let stdout = String::from_utf8_lossy(&output.stdout).to_string();
    let stderr = String::from_utf8_lossy(&output.stderr).to_string();
    let exit_code = output.status.code().map_or(-1, |code| code);
    let timed_out = output.status.code() == Some(124);

    let combined = if timed_out {
        format!(
            "Command timed out after {}s\n\nstdout:\n{}\n\nstderr:\n{}",
            timeout_seconds, stdout, stderr
        )
    } else {
        format!("{}\n{}", stdout, stderr)
    };

    tracing::info!(
        "{} {:?}: {} ({})",
        command_name,
        args,
        if output.status.success() { "PASS" } else { "FAIL" },
        exit_code
    );

    Ok((output.status.success(), combined, exit_code))
}

fn run_opencode(prompt: &str, repo_root: &PathBuf) -> Result<(bool, String), OyaError> {
    tracing::info!("Running opencode with prompt ({} chars)", prompt.len());
    run_command_with_timeout(
        "opencode",
        &["run", "--format", "json", prompt],
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
            let (passed, output, exit_code) = run_command_with_timeout_with_exit(
                "moon",
                &["run", ":check"],
                MOON_TIMEOUT_SECONDS,
                repo_root,
            )?;
            Ok(GateEvidence { command: "moon run :check".to_string(), passed, exit_code, output })
        }
        Gate::TestsPass | Gate::EdgeCases | Gate::NoVulnerabilities => {
            let (passed, output, exit_code) = run_command_with_timeout_with_exit(
                "moon",
                &["run", ":test"],
                MOON_TIMEOUT_SECONDS,
                repo_root,
            )?;
            Ok(GateEvidence { command: "moon run :test".to_string(), passed, exit_code, output })
        }
        Gate::ClippyClean | Gate::Security => {
            let (passed, output, exit_code) = run_command_with_timeout_with_exit(
                "moon",
                &["run", ":quick"],
                MOON_TIMEOUT_SECONDS,
                repo_root,
            )?;
            Ok(GateEvidence { command: "moon run :quick".to_string(), passed, exit_code, output })
        }
        Gate::MoonCi => {
            let (passed, output, exit_code) = run_command_with_timeout_with_exit(
                "moon",
                &["run", ":ci"],
                MOON_TIMEOUT_SECONDS,
                repo_root,
            )?;
            Ok(GateEvidence { command: "moon run :ci".to_string(), passed, exit_code, output })
        }
        Gate::ZjjMergeQueue => {
            let (passed, output, exit_code) = run_command_with_timeout_with_exit(
                "zjj",
                &["done", "--dry-run"],
                ZJJ_TIMEOUT_SECONDS,
                repo_root,
            )?;
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
    let (queue_passed, queue_output, queue_exit_code) = run_command_with_timeout_with_exit(
        "zjj",
        &["queue", "--add", workspace.as_str(), "--bead", bead_id],
        ZJJ_TIMEOUT_SECONDS,
        repo_root,
    )?;
    if !queue_passed {
        return Err(OyaError(format!(
            "zjj queue failed for workspace {} (exit={}): {}",
            workspace,
            queue_exit_code,
            truncate_text(queue_output.as_str(), 2000)
        )));
    }

    let add_command = format!("zjj add {} --idempotent", workspace);
    let (add_passed, add_output, add_exit_code) = run_command_with_timeout_with_exit(
        "zjj",
        &["add", workspace.as_str(), "--idempotent"],
        ZJJ_TIMEOUT_SECONDS,
        repo_root,
    )?;
    if !add_passed {
        return Err(OyaError(format!(
            "zjj add failed for workspace {} (exit={}): {}",
            workspace,
            add_exit_code,
            truncate_text(add_output.as_str(), 2000)
        )));
    }

    Ok(Some(WorkspaceLifecycleEvent {
        workspace,
        queue_command,
        queue_passed,
        queue_exit_code,
        queue_output: truncate_text(queue_output.as_str(), 4000),
        add_command,
        add_passed,
        add_exit_code,
        add_output: truncate_text(add_output.as_str(), 4000),
        recorded_at,
    }))
}

fn truncate_text(input: &str, max_chars: usize) -> String {
    let collected: String = input.chars().take(max_chars).collect();
    if input.chars().count() > max_chars {
        format!("{}\n...[truncated]", collected)
    } else {
        collected
    }
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
        let timeout_line = match trimmed
            .lines()
            .find(|line| line.contains("Command timed out after"))
            .map(str::trim)
        {
            Some(s) => s,
            None => "Command timed out after unknown duration",
        };

        let stderr_preview = first_non_empty_line_after_marker(trimmed, "stderr:")
            .map(|line| truncate_text(line, 180));
        let stdout_preview = first_non_empty_line_after_marker(trimmed, "stdout:")
            .map(|line| truncate_text(line, 180));

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

    truncate_text(trimmed, 1200)
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

async fn append_timeline(ctx: &WorkflowContext<'_>, event: TimelineEvent) -> Result<(), OyaError> {
    let existing = ctx
        .get::<String>("timeline")
        .await
        .map_err(|error| OyaError(format!("timeline read failed: {}", error)))?;
    let existing = match existing {
        Some(s) => s,
        None => String::new(),
    };

    let event_seq = ctx
        .get::<u32>("event_seq")
        .await
        .map_err(|error| OyaError(format!("event_seq read failed: {}", error)))?
        .map_or(1, |value| value + 1);
    ctx.set("event_seq", event_seq);

    let event_key = format!("event_{:04}", event_seq);
    set_json_state(ctx, &event_key, &event)?;

    let line = to_json_string(&event)?;
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
        Stage::Research => {
            "TASK:\n1. Read existing source in src/\n2. Summarize implementation constraints in docs/RESEARCH_NOTES.md\n3. Keep output concise and implementation-ready\n\nJust write files. Do not explain."
        }
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
        Stage::Research => ("Research completed", Some(Stage::Plan)),
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
        Stage::Research => vec![StageCheck::Check {
            failure: FailureCategory::CompileFailed,
            next_stage: Stage::Research,
        }],
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
    let value = match configured {
        Some(s) => s,
        None => "127.0.0.1:9080".to_string(),
    };

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
}

fn parse_cli_mode() -> CliMode {
    let cli = Cli::parse();
    match cli.command {
        None | Some(CliCommand::Serve) => CliMode::Serve,
        Some(CliCommand::OpsPoll) => CliMode::OpsPoll,
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum CliMode {
    Serve,
    OpsPoll,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mode = parse_cli_mode();

    match mode {
        CliMode::OpsPoll => run_ops_poller().await,
        CliMode::Serve => run_server().await,
    }
}

async fn run_server() -> Result<(), Box<dyn std::error::Error>> {
    tracing_subscriber::fmt().with_max_level(tracing::Level::INFO).init();

    tracing::info!("OYA Orchestrator starting on port 9080");
    tracing::info!("Using REAL execution: opencode CLI + moon/zjj quality gates");

    let workflow_service = OyaOrchestratorImpl.serve();
    let monitor_service = OyaOpsMonitorImpl.serve();
    let workflow_service_options = restate_sdk::endpoint::ServiceOptions::new()
        .inactivity_timeout(std::time::Duration::from_secs(30 * 60))
        .abort_timeout(std::time::Duration::from_secs(5 * 60));
    let monitor_service_options = restate_sdk::endpoint::ServiceOptions::new()
        .inactivity_timeout(std::time::Duration::from_secs(30 * 60))
        .abort_timeout(std::time::Duration::from_secs(5 * 60));
    let endpoint = Endpoint::builder()
        .bind_with_options(workflow_service, workflow_service_options)
        .bind_with_options(monitor_service, monitor_service_options)
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
            truncate_text(&text, 200)
        )));
    }

    Ok(text)
}
