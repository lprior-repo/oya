#![deny(clippy::unwrap_used)]
#![deny(clippy::expect_used)]
#![deny(clippy::panic)]
#![forbid(unsafe_code)]

use oya::types::{FailureCategory, Gate, StageName as Stage, StageResult};
use restate_sdk::endpoint::Endpoint;
use restate_sdk::http_server::HttpServer;
use restate_sdk::prelude::*;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::process::Command;

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

#[derive(Debug, Deserialize)]
struct StartRequestPayload {
    bead_id: Option<String>,
    context: Option<String>,
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

fn parse_start_request(request: serde_json::Value) -> Result<StartRequestPayload, OyaError> {
    match request {
        serde_json::Value::Object(_) => serde_json::from_value(request)
            .map_err(|e| OyaError(format!("Invalid JSON body: {}", e))),
        serde_json::Value::String(raw) => serde_json::from_str::<StartRequestPayload>(&raw)
            .map_err(|e| OyaError(format!("Invalid JSON string body: {}", e))),
        other => Err(OyaError(format!(
            "Invalid request payload type: expected object or JSON string, got {}",
            other
        ))),
    }
}

pub struct OyaOrchestratorImpl;

impl OyaOrchestrator for OyaOrchestratorImpl {
    async fn start(
        &self,
        ctx: WorkflowContext<'_>,
        request: Json<serde_json::Value>,
    ) -> Result<String, HandlerError> {
        let parsed = parse_start_request(request.0)?;

        let bead_id = parsed.bead_id.unwrap_or_else(|| "unknown".to_string());
        let context = parsed.context.unwrap_or_default();
        let run_id = ctx.key().to_string();
        let started_at = chrono::Utc::now().to_rfc3339();

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

async fn run_pipeline(
    ctx: &WorkflowContext<'_>,
    run_id: String,
    bead_id: String,
    context: String,
) -> Result<(), OyaError> {
    let mut current_stage = Stage::Research;
    let mut attempt = 1u32;
    let mut last_failure: Option<(FailureCategory, String)> = None;
    let mut orchestrator_state = OrchestratorState {
        status: "running".to_string(),
        stage: current_stage.as_str().to_string(),
        attempt,
        bead_id: bead_id.clone(),
        context: context.clone(),
        last_failure: String::new(),
        last_output: String::new(),
        last_prompt: String::new(),
        updated_at: chrono::Utc::now().to_rfc3339(),
    };
    write_orchestrator_state(ctx, &orchestrator_state)?;

    loop {
        orchestrator_state.stage = current_stage.as_str().to_string();
        orchestrator_state.attempt = attempt;
        orchestrator_state.status = "running".to_string();
        orchestrator_state.updated_at = chrono::Utc::now().to_rfc3339();
        write_orchestrator_state(ctx, &orchestrator_state)?;

        let stage_start = chrono::Utc::now().to_rfc3339();
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

        let (stage_result, stage_prompt) = match execute_stage_real(
            &run_id,
            &bead_id,
            current_stage.clone(),
            attempt,
            &context,
            last_failure.clone(),
        )
        .await
        {
            Ok(result) => result,
            Err(error) => {
                orchestrator_state.status = "failed".to_string();
                orchestrator_state.last_failure = format!("Stage execution error: {}", error);
                orchestrator_state.updated_at = chrono::Utc::now().to_rfc3339();
                write_orchestrator_state(ctx, &orchestrator_state)?;
                append_timeline(
                    ctx,
                    TimelineEvent {
                        at: chrono::Utc::now().to_rfc3339(),
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
        orchestrator_state.updated_at = chrono::Utc::now().to_rfc3339();
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
            let gate_evidence = execute_gate(gate.clone())?;
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
                recorded_at: chrono::Utc::now().to_rfc3339(),
            },
        )?;

        if stage_result.passed {
            append_timeline(
                ctx,
                TimelineEvent {
                    at: chrono::Utc::now().to_rfc3339(),
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
                    orchestrator_state.status = "shipped".to_string();
                    orchestrator_state.stage = "none".to_string();
                    orchestrator_state.updated_at = chrono::Utc::now().to_rfc3339();
                    write_orchestrator_state(ctx, &orchestrator_state)?;
                    append_timeline(
                        ctx,
                        TimelineEvent {
                            at: chrono::Utc::now().to_rfc3339(),
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
            append_timeline(
                ctx,
                TimelineEvent {
                    at: chrono::Utc::now().to_rfc3339(),
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
                orchestrator_state.status = "failed".to_string();
                orchestrator_state.updated_at = chrono::Utc::now().to_rfc3339();
                write_orchestrator_state(ctx, &orchestrator_state)?;
                append_timeline(
                    ctx,
                    TimelineEvent {
                        at: chrono::Utc::now().to_rfc3339(),
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
                orchestrator_state.status = "failed".to_string();
                orchestrator_state.updated_at = chrono::Utc::now().to_rfc3339();
                write_orchestrator_state(ctx, &orchestrator_state)?;
                append_timeline(
                    ctx,
                    TimelineEvent {
                        at: chrono::Utc::now().to_rfc3339(),
                        event: "run_failed_max_attempts".to_string(),
                        stage: Some(current_stage.as_str().to_string()),
                        attempt: Some(attempt),
                        detail: None,
                    },
                )
                .await?;
                return Ok(());
            }

            append_timeline(
                ctx,
                TimelineEvent {
                    at: chrono::Utc::now().to_rfc3339(),
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

fn is_retryable_failure(category: &FailureCategory) -> bool {
    matches!(
        category,
        FailureCategory::TestFailed
            | FailureCategory::LintFailed
            | FailureCategory::OutputParseFailure
    )
}

async fn execute_stage_real(
    run_id: &str,
    bead_id: &str,
    stage: Stage,
    attempt: u32,
    context: &str,
    last_failure: Option<(FailureCategory, String)>,
) -> Result<(StageResult, String), OyaError> {
    let bead_id = bead_id.to_string();
    let context = context.to_string();
    let run_id = run_id.to_string();
    let stage_for_closure = stage.clone();

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
            )
        }
        Stage::ShipGate => execute_ship_gate(&bead_id, attempt, &context, &last_failure),
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
    MoonCheck { failure: FailureCategory, next_stage: Stage },
    MoonTest { failure: FailureCategory, next_stage: Stage },
    MoonQuick { failure: FailureCategory, next_stage: Stage },
}

fn execute_prompt_stage(
    prompt: String,
    opencode_fail_stage: Stage,
    success_message: &str,
    success_next_stage: Option<Stage>,
    checks: &[StageCheck],
) -> Result<StageExecution, OyaError> {
    let (opencode_ok, opencode_output) = run_opencode(&prompt)?;
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
            StageCheck::MoonCheck { failure, next_stage } => {
                let (ok, output) = run_moon_check()?;
                if ok {
                    None
                } else {
                    Some((failure.clone(), output, next_stage.clone()))
                }
            }
            StageCheck::MoonTest { failure, next_stage } => {
                let (ok, output) = run_moon_test()?;
                if ok {
                    None
                } else {
                    Some((failure.clone(), output, next_stage.clone()))
                }
            }
            StageCheck::MoonQuick { failure, next_stage } => {
                let (ok, output) = run_moon_quick()?;
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

fn run_command_with_timeout(
    command_name: &str,
    args: &[&str],
    timeout_seconds: u64,
) -> Result<(bool, String), OyaError> {
    let (passed, output, _exit_code) =
        run_command_with_timeout_with_exit(command_name, args, timeout_seconds)?;
    Ok((passed, output))
}

fn run_command_with_timeout_with_exit(
    command_name: &str,
    args: &[&str],
    timeout_seconds: u64,
) -> Result<(bool, String, i32), OyaError> {
    let timeout_duration = timeout_seconds.to_string();
    let output = Command::new("timeout")
        .arg(timeout_duration)
        .arg(command_name)
        .args(args)
        .current_dir(repo_root()?)
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

fn run_opencode(prompt: &str) -> Result<(bool, String), OyaError> {
    tracing::info!("Running opencode with prompt ({} chars)", prompt.len());
    run_command_with_timeout(
        "opencode",
        &["run", "--format", "json", prompt],
        OPENCODE_TIMEOUT_SECONDS,
    )
}

fn run_moon_check() -> Result<(bool, String), OyaError> {
    tracing::info!("Running moon :check");
    run_command_with_timeout("moon", &["run", ":check"], MOON_TIMEOUT_SECONDS)
}

fn run_moon_test() -> Result<(bool, String), OyaError> {
    tracing::info!("Running moon :test");
    run_command_with_timeout("moon", &["run", ":test"], MOON_TIMEOUT_SECONDS)
}

fn run_moon_quick() -> Result<(bool, String), OyaError> {
    tracing::info!("Running moon :quick");
    run_command_with_timeout("moon", &["run", ":quick"], MOON_TIMEOUT_SECONDS)
}

fn run_moon_ci() -> Result<(bool, String), OyaError> {
    tracing::info!("Running moon :ci");
    run_command_with_timeout("moon", &["run", ":ci"], MOON_TIMEOUT_SECONDS)
}

fn run_zjj_done_dry_run() -> Result<(bool, String), OyaError> {
    tracing::info!("Running zjj done --dry-run");

    let (success, combined) =
        run_command_with_timeout("zjj", &["done", "--dry-run"], ZJJ_TIMEOUT_SECONDS)?;

    Ok((success, combined))
}

fn execute_gate(gate: Gate) -> Result<GateEvidence, OyaError> {
    match gate {
        Gate::Compiles => {
            let (passed, output, exit_code) = run_command_with_timeout_with_exit(
                "moon",
                &["run", ":check"],
                MOON_TIMEOUT_SECONDS,
            )?;
            Ok(GateEvidence { command: "moon run :check".to_string(), passed, exit_code, output })
        }
        Gate::TestsPass | Gate::EdgeCases | Gate::NoVulnerabilities => {
            let (passed, output, exit_code) = run_command_with_timeout_with_exit(
                "moon",
                &["run", ":test"],
                MOON_TIMEOUT_SECONDS,
            )?;
            Ok(GateEvidence { command: "moon run :test".to_string(), passed, exit_code, output })
        }
        Gate::ClippyClean | Gate::Security => {
            let (passed, output, exit_code) = run_command_with_timeout_with_exit(
                "moon",
                &["run", ":quick"],
                MOON_TIMEOUT_SECONDS,
            )?;
            Ok(GateEvidence { command: "moon run :quick".to_string(), passed, exit_code, output })
        }
        Gate::MoonCi => {
            let (passed, output, exit_code) =
                run_command_with_timeout_with_exit("moon", &["run", ":ci"], MOON_TIMEOUT_SECONDS)?;
            Ok(GateEvidence { command: "moon run :ci".to_string(), passed, exit_code, output })
        }
        Gate::ZjjMergeQueue => {
            let (passed, output, exit_code) = run_command_with_timeout_with_exit(
                "zjj",
                &["done", "--dry-run"],
                ZJJ_TIMEOUT_SECONDS,
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
        let timeout_line = trimmed
            .lines()
            .find(|line| line.contains("Command timed out after"))
            .map(str::trim)
            .unwrap_or("Command timed out after unknown duration");

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
        .map_err(|error| OyaError(format!("timeline read failed: {}", error)))?
        .unwrap_or_default();

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
        Stage::Research => vec![StageCheck::MoonCheck {
            failure: FailureCategory::CompileFailed,
            next_stage: Stage::Research,
        }],
        Stage::Plan => vec![StageCheck::MoonCheck {
            failure: FailureCategory::CompileFailed,
            next_stage: Stage::Plan,
        }],
        Stage::Contract => vec![StageCheck::MoonCheck {
            failure: FailureCategory::CompileFailed,
            next_stage: Stage::Contract,
        }],
        Stage::Tdd15 => vec![
            StageCheck::MoonCheck {
                failure: FailureCategory::CompileFailed,
                next_stage: Stage::Tdd15,
            },
            StageCheck::MoonTest { failure: FailureCategory::TestFailed, next_stage: Stage::Tdd15 },
        ],
        Stage::Qa => vec![StageCheck::MoonTest {
            failure: FailureCategory::TestFailed,
            next_stage: Stage::Tdd15,
        }],
        Stage::RedQueen => vec![StageCheck::MoonTest {
            failure: FailureCategory::TestFailed,
            next_stage: Stage::Tdd15,
        }],
        Stage::GptReview => vec![
            StageCheck::MoonQuick {
                failure: FailureCategory::LintFailed,
                next_stage: Stage::GptReview,
            },
            StageCheck::MoonTest { failure: FailureCategory::TestFailed, next_stage: Stage::Tdd15 },
        ],
        Stage::ShipGate => Vec::new(),
    }
}

fn execute_ship_gate(
    _bead_id: &str,
    _attempt: u32,
    _context: &str,
    _last_failure: &Option<(FailureCategory, String)>,
) -> Result<StageExecution, OyaError> {
    tracing::info!("SHIP GATE: Running final validation");
    let prompt = "Ship gate executes quality gates only (moon/zjj); no OpenCode prompt".to_string();

    let (ci_ok, ci_output) = run_moon_ci()?;
    if !ci_ok {
        return Ok(StageExecution {
            passed: false,
            output: ci_output,
            failure_category: Some(FailureCategory::CompileFailed),
            next_stage: Some(Stage::Tdd15),
            prompt,
        });
    }

    let skip_zjj_gate = std::env::var("OYA_SKIP_ZJJ_GATE")
        .ok()
        .is_some_and(|value| value == "1" || value.eq_ignore_ascii_case("true"));

    if !skip_zjj_gate {
        let (zjj_ok, zjj_output) = run_zjj_done_dry_run()?;
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

    let (quick_ok, quick_output) = run_moon_quick()?;
    if !quick_ok {
        return Ok(StageExecution {
            passed: false,
            output: quick_output,
            failure_category: Some(FailureCategory::LintFailed),
            next_stage: Some(Stage::GptReview),
            prompt,
        });
    }

    let (test_ok, test_output) = run_moon_test()?;
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

fn repo_root() -> Result<PathBuf, OyaError> {
    if let Ok(configured_root) = std::env::var("OYA_REPO_ROOT") {
        return Ok(PathBuf::from(configured_root));
    }
    std::env::current_dir().map_err(|e| OyaError(format!("Failed to resolve repo root: {}", e)))
}

fn resolve_bind_addr() -> Result<std::net::SocketAddr, OyaError> {
    let configured = std::env::var("OYA_BIND_ADDR").ok();
    let value = configured.unwrap_or_else(|| "127.0.0.1:9080".to_string());

    value.parse().map_err(|e| OyaError(format!("Invalid OYA_BIND_ADDR '{}': {}", value, e)))
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    tracing_subscriber::fmt().with_max_level(tracing::Level::INFO).init();

    tracing::info!("OYA Orchestrator starting on port 9080");
    tracing::info!("Using REAL execution: opencode CLI + moon/zjj quality gates");

    let service = OyaOrchestratorImpl.serve();
    let service_options = restate_sdk::endpoint::ServiceOptions::new()
        .inactivity_timeout(std::time::Duration::from_secs(30 * 60))
        .abort_timeout(std::time::Duration::from_secs(5 * 60));
    let endpoint = Endpoint::builder().bind_with_options(service, service_options).build();

    let bind_addr = resolve_bind_addr()?;
    HttpServer::new(endpoint).listen_and_serve(bind_addr).await;

    Ok(())
}
