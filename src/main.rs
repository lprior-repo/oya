#![deny(clippy::unwrap_used)]
#![deny(clippy::expect_used)]
#![deny(clippy::panic)]
#![forbid(unsafe_code)]

use oya::application::workflow::is_retryable_failure;
use oya::domain::{
    AgentId, AgentState, AgentStatus, ApproverMode, Artifact, ArtifactType, BeadId,
    FailureCategory, Gate, GateResult, Run as BeadRun, RunId, RunState, ShipDecision, StageAttempt,
    StageName as Stage, StageResult, StageState,
};
use oya::infrastructure::persistence::{self, OyaDb};
use oya::infrastructure::zjj::zjj_done_has_constraint_violation;
use restate_sdk::endpoint::Endpoint;
use restate_sdk::http_server::HttpServer;
use restate_sdk::prelude::*;
use restate_sdk::service::Discoverable;
use serde::Deserialize;
use std::path::PathBuf;
use std::process::Command;
use std::sync::Arc;

static DB: std::sync::OnceLock<Arc<OyaDb>> = std::sync::OnceLock::new();

#[derive(Debug)]
pub struct OyaError(String);

impl std::fmt::Display for OyaError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl std::error::Error for OyaError {}

impl From<persistence::OyaDbError> for OyaError {
    fn from(e: persistence::OyaDbError) -> Self {
        OyaError(e.to_string())
    }
}

#[restate_sdk::object]
pub trait OyaOrchestrator {
    async fn start(request: Json<serde_json::Value>) -> Result<String, HandlerError>;
    async fn get_status() -> Result<String, HandlerError>;
    async fn ping() -> Result<String, HandlerError>;
}

#[derive(Debug, Deserialize)]
struct StartRequestPayload {
    bead_id: Option<String>,
    context: Option<String>,
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
        ctx: ObjectContext<'_>,
        request: Json<serde_json::Value>,
    ) -> Result<String, HandlerError> {
        let parsed = parse_start_request(request.0)?;

        let bead_id = parsed.bead_id.unwrap_or_else(|| "unknown".to_string());
        let context = parsed.context.unwrap_or_default();
        let run_id = ctx.key().to_string();

        tracing::info!("=== RUN {} STARTED ===", run_id);
        tracing::info!("Bead: {}", bead_id);
        tracing::info!("Context: {}", context);
        let db_for_task = get_db()?;
        let run_id_for_task = run_id.clone();
        let bead_id_for_task = bead_id.clone();
        let context_for_task = context.clone();

        tokio::spawn(async move {
            if let Err(error) = start_or_resume_pipeline(
                db_for_task,
                run_id_for_task,
                bead_id_for_task,
                context_for_task,
            )
            .await
            {
                tracing::error!("Run pipeline background task failed: {}", error);
            }
        });

        Ok(run_id)
    }

    async fn get_status(&self, ctx: ObjectContext<'_>) -> Result<String, HandlerError> {
        let run_id = ctx.key().to_string();
        let db = get_db()?;

        let run =
            db.get_run(&run_id).await?.ok_or_else(|| OyaError("Run not found".to_string()))?;

        let results = db.get_stage_results(&run_id).await?;
        let completed: Vec<String> =
            results.iter().filter(|r| r.passed).map(|r| format!("{:?}", r.stage)).collect();

        Ok(serde_json::json!({
            "run_id": run.id.as_str(),
            "bead_id": run.bead_id.as_str(),
            "status": format!("{:?}", run.state),
            "current_stage": match &run.state {
                RunState::Running { current_stage } => format!("{:?}", current_stage),
                _ => "none".to_string(),
            },
            "completed_stages": completed
        })
        .to_string())
    }

    async fn ping(&self, _ctx: ObjectContext<'_>) -> Result<String, HandlerError> {
        Ok(serde_json::json!({
            "status": "ok",
            "service": "OyaOrchestrator"
        })
        .to_string())
    }
}

async fn start_or_resume_pipeline(
    db: Arc<OyaDb>,
    run_id: String,
    bead_id: String,
    context: String,
) -> Result<(), OyaError> {
    let mut run = BeadRun::new(BeadId::new(bead_id.clone()));
    run.id = RunId(run_id.clone());
    run = run.start().map_err(|e| OyaError(e.to_string()))?;

    match db.insert_bead_run_if_absent(&run).await {
        Ok(()) => {}
        Err(persistence::OyaDbError::DuplicateRunKey(_)) => {
            tracing::info!("Run {} already exists; skipping duplicate start", run_id);
            return Ok(());
        }
        Err(error) => return Err(OyaError(error.to_string())),
    }

    let agent_id = AgentId::new();
    tracing::info!("Agent: {}", agent_id.as_str());
    let agent_state = AgentState::new(
        agent_id,
        Some(run.bead_id.clone()),
        match &run.state {
            RunState::Running { current_stage } => Some(current_stage.clone()),
            _ => None,
        },
        AgentStatus::Working,
        1,
    );
    agent_state.validate_invariants().map_err(OyaError)?;
    db.insert_agent_state(&agent_state).await?;

    run_pipeline(db, run, agent_state, run_id, bead_id, context).await
}

async fn run_pipeline(
    db: Arc<OyaDb>,
    mut run: BeadRun,
    mut agent_state: AgentState,
    run_id: String,
    bead_id: String,
    context: String,
) -> Result<(), OyaError> {
    let mut current_stage = match &run.state {
        RunState::Running { current_stage } => current_stage.clone(),
        _ => return Err(OyaError("Failed to start run".to_string())),
    };
    let mut attempt = 1u32;
    let mut last_failure: Option<(FailureCategory, String)> = None;

    loop {
        tracing::info!("");
        tracing::info!("=== STAGE: {:?} (attempt {}) ===", current_stage, attempt);
        tracing::info!(
            "RESTATE_CALL stage.execute stage={} attempt={}",
            current_stage.as_str(),
            attempt
        );
        if let Some((ref cat, ref msg)) = last_failure {
            tracing::info!("Previous failure: {:?} - {} chars of output", cat, msg.len());
        }

        let last_failure_clone = last_failure.clone();
        agent_state.current_stage = Some(current_stage.clone());
        agent_state.implementation_attempt = attempt;
        agent_state.status = AgentStatus::Working;
        agent_state.last_update = chrono::Utc::now();
        db.insert_agent_state(&agent_state).await?;

        let stage_result = match execute_stage_real(
            &db,
            &run_id,
            &bead_id,
            current_stage.clone(),
            attempt,
            &context,
            last_failure_clone,
        )
        .await
        {
            Ok(result) => result,
            Err(error) => {
                run = run.fail(format!("Stage execution error: {}", error));
                db.update_run_state(&run_id, &run.state).await?;

                agent_state.status = AgentStatus::Error;
                agent_state.bead_id = None;
                agent_state.current_stage = None;
                agent_state.feedback = Some(format!("stage_execution_error: {}", error));
                agent_state.validate_invariants().map_err(OyaError)?;
                db.insert_agent_state(&agent_state).await?;

                tracing::error!("=== RUN {} FAILED (stage execution error) ===", run_id);
                return Ok(());
            }
        };

        db.insert_stage_result(&stage_result).await?;
        for gate in current_stage.gates() {
            let gate_evidence = execute_gate(gate.clone())?;
            let gate_artifact_id =
                format!("gate-{}-{:03}-{}", current_stage.as_str(), attempt, gate.as_str());
            let gate_artifact = Artifact {
                id: gate_artifact_id.clone(),
                run_id: run_id.clone(),
                artifact_type: ArtifactType::QualityGateReport,
                location: format!(
                    "inline://gate-output\ncommand: {}\nexit_code: {}\noutput:\n{}",
                    gate_evidence.command,
                    gate_evidence.exit_code,
                    truncate_text(&gate_evidence.output, 8000)
                ),
                checksum: None,
                produced_by_stage: current_stage.clone(),
            };
            db.insert_artifact(&gate_artifact).await?;

            let gate_result = GateResult {
                run_id: run_id.clone(),
                gate_name: format!("{}:{:03}:{}", current_stage.as_str(), attempt, gate.as_str()),
                command: Some(gate_evidence.command),
                passed: gate_evidence.passed,
                exit_code: gate_evidence.exit_code,
                log_ref: Some(format!("artifact://{}", gate_artifact_id)),
            };
            gate_result
                .validate()
                .map_err(|e| OyaError(format!("Invalid gate evidence: {:?}", e)))?;
            db.insert_gate_result(&gate_result).await?;
        }

        if stage_result.passed {
            tracing::info!("STAGE {:?} PASSED", current_stage);
            last_failure = None;

            run = run
                .complete_stage(current_stage.clone(), stage_result.clone())
                .map_err(|e| OyaError(e.to_string()))?;

            db.update_run_state(&run_id, &run.state).await?;

            match &run.state {
                RunState::Running { current_stage: next } => {
                    current_stage = next.clone();
                    attempt = 1;
                }
                RunState::Shipped { .. } => {
                    let decision = ShipDecision {
                        run_id: run_id.clone(),
                        shipped: true,
                        rationale: "All stages passed".to_string(),
                        approver_mode: ApproverMode::Auto,
                        timestamp: chrono::Utc::now(),
                    };
                    db.insert_ship_decision(&decision).await?;

                    agent_state.status = AgentStatus::Done;
                    agent_state.bead_id = None;
                    agent_state.current_stage = None;
                    agent_state.last_update = chrono::Utc::now();
                    agent_state.validate_invariants().map_err(OyaError)?;
                    db.insert_agent_state(&agent_state).await?;

                    tracing::info!("");
                    tracing::info!("=== RUN {} SHIPPED ===", run_id);
                    return Ok(());
                }
                _ => {
                    return Err(OyaError("Unexpected run state after success".to_string()));
                }
            }
        } else {
            tracing::warn!("STAGE {:?} FAILED: {:?}", current_stage, stage_result.failure_category);

            last_failure =
                stage_result.failure_category.clone().zip(Some(stage_result.output.to_string()));

            if let Some(category) = stage_result.failure_category.clone() {
                if !is_retryable_failure(&category) {
                    let terminal_reason = format!(
                        "Non-retryable failure in stage {}: {:?}",
                        current_stage.as_str(),
                        category
                    );
                    run = run.fail(terminal_reason);
                    db.update_run_state(&run_id, &run.state).await?;

                    agent_state.status = AgentStatus::Error;
                    agent_state.bead_id = None;
                    agent_state.current_stage = None;
                    agent_state.feedback = last_failure
                        .as_ref()
                        .map(|(failure_category, _)| format!("{:?}", failure_category));
                    agent_state.validate_invariants().map_err(OyaError)?;
                    db.insert_agent_state(&agent_state).await?;

                    tracing::error!(
                        "=== RUN {} FAILED (non-retryable category {:?}) ===",
                        run_id,
                        category
                    );
                    return Ok(());
                }
            }

            attempt += 1;

            agent_state.feedback = Some(format!("{:?}", last_failure));
            agent_state.last_update = chrono::Utc::now();

            if attempt > current_stage.max_attempts() {
                run = run.fail("Max attempts exceeded".to_string());
                db.update_run_state(&run_id, &run.state).await?;

                agent_state.status = AgentStatus::Error;
                agent_state.bead_id = None;
                agent_state.current_stage = None;
                agent_state.validate_invariants().map_err(OyaError)?;
                db.insert_agent_state(&agent_state).await?;

                tracing::error!("=== RUN {} FAILED (max attempts reached) ===", run_id);
                return Ok(());
            }

            agent_state.status = AgentStatus::Working;
            agent_state.validate_invariants().map_err(OyaError)?;
            db.insert_agent_state(&agent_state).await?;

            tracing::info!(
                "Retrying stage {:?} (attempt {}) with failure context",
                current_stage,
                attempt
            );
        }

        tokio::time::sleep(std::time::Duration::from_millis(100)).await;
    }
}

async fn execute_stage_real(
    db: &OyaDb,
    run_id: &str,
    bead_id: &str,
    stage: Stage,
    attempt: u32,
    context: &str,
    last_failure: Option<(FailureCategory, String)>,
) -> Result<StageResult, OyaError> {
    let now = chrono::Utc::now();

    let stage_attempt = StageAttempt {
        run_id: run_id.to_string(),
        stage: stage.clone(),
        attempt,
        session_id: None,
        state: StageState::Running,
        started_at: now,
        completed_at: None,
    };
    db.insert_stage_attempt(&stage_attempt).await?;

    let bead_id = bead_id.to_string();
    let context = context.to_string();
    let run_id = run_id.to_string();
    let stage_for_closure = stage.clone();

    let (passed, output, failure_category, next_stage) =
        tokio::task::spawn_blocking(move || match stage_for_closure {
            Stage::Research => execute_research(&bead_id, attempt, &context, &last_failure),
            Stage::Plan => execute_plan(&bead_id, attempt, &context, &last_failure),
            Stage::Contract => execute_contract(&bead_id, attempt, &context, &last_failure),
            Stage::Tdd15 => execute_tdd15(&bead_id, attempt, &context, &last_failure),
            Stage::Qa => execute_qa(&bead_id, attempt, &context, &last_failure),
            Stage::RedQueen => execute_red_queen(&bead_id, attempt, &context, &last_failure),
            Stage::GptReview => execute_gpt_review(&bead_id, attempt, &context, &last_failure),
            Stage::ShipGate => execute_ship_gate(&bead_id, attempt, &context, &last_failure),
        })
        .await
        .map_err(|e| OyaError(format!("spawn_blocking failed: {}", e)))??;

    let stage_key = serde_json::to_string(&stage)
        .map_err(|e| OyaError(format!("failed to serialize stage key: {}", e)))?;
    let attempt_state = if passed { "passed" } else { "failed" };
    db.update_stage_attempt_state(&run_id, &stage_key, attempt, attempt_state).await?;

    Ok(StageResult {
        run_id,
        stage,
        attempt,
        passed,
        output: serde_json::json!({ "output": output }),
        failure_category,
        next_stage,
    })
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

    if !success && zjj_done_has_constraint_violation(&combined) {
        let guidance = "zjj/bead DB constraint violation detected. Run `zjj recover --diagnose`, then repair bead closed_at consistency before retrying.";
        return Ok((false, format!("{}\n{}", combined, guidance)));
    }

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

fn execute_contract(
    bead_id: &str,
    attempt: u32,
    context: &str,
    last_failure: &Option<(FailureCategory, String)>,
) -> Result<(bool, String, Option<FailureCategory>, Option<Stage>), OyaError> {
    let failure_context = match last_failure {
        Some((cat, msg)) => format!(
            "\n\nPREVIOUS FAILURE: {:?}\nERROR OUTPUT:\n{}\n\nFix the issue that caused this failure.",
            cat,
            msg.chars().take(2000).collect::<String>()
        ),
        None => String::new(),
    };

    let prompt = format!(
        r#"You are creating a design contract for: {}

Request context: {}
Attempt: {}
{}

TASK: Write a design contract as a Rust doc comment in src/lib.rs (create if needed).

Include:
1. Purpose and goals
2. Key functions to implement
3. Acceptance criteria

Just write the code. Do not explain."#,
        bead_id, context, attempt, failure_context
    );

    let (opencode_ok, opencode_output) = run_opencode(&prompt)?;

    if !opencode_ok {
        return Ok((
            false,
            opencode_output,
            Some(FailureCategory::OutputParseFailure),
            Some(Stage::Contract),
        ));
    }

    let (check_ok, check_output) = run_moon_check()?;

    if check_ok {
        Ok((true, "Contract written and compiles".to_string(), None, Some(Stage::Tdd15)))
    } else {
        Ok((false, check_output, Some(FailureCategory::CompileFailed), Some(Stage::Contract)))
    }
}

fn execute_research(
    bead_id: &str,
    attempt: u32,
    context: &str,
    last_failure: &Option<(FailureCategory, String)>,
) -> Result<(bool, String, Option<FailureCategory>, Option<Stage>), OyaError> {
    let failure_context = match last_failure {
        Some((cat, msg)) => format!(
            "\n\nPREVIOUS FAILURE: {:?}\nERROR OUTPUT:\n{}\n\nFix the issue that caused this failure.",
            cat,
            msg.chars().take(2000).collect::<String>()
        ),
        None => String::new(),
    };

    let prompt = format!(
        r#"You are doing discovery/research for: {}

Request context: {}
Attempt: {}
{}

TASK:
1. Read existing source in src/
2. Summarize implementation constraints in docs/RESEARCH_NOTES.md
3. Keep output concise and implementation-ready

Just write files. Do not explain."#,
        bead_id, context, attempt, failure_context
    );

    let (opencode_ok, opencode_output) = run_opencode(&prompt)?;

    if !opencode_ok {
        return Ok((
            false,
            opencode_output,
            Some(FailureCategory::OutputParseFailure),
            Some(Stage::Research),
        ));
    }

    let (check_ok, check_output) = run_moon_check()?;
    if check_ok {
        Ok((true, "Research completed".to_string(), None, Some(Stage::Plan)))
    } else {
        Ok((false, check_output, Some(FailureCategory::CompileFailed), Some(Stage::Research)))
    }
}

fn execute_plan(
    bead_id: &str,
    attempt: u32,
    context: &str,
    last_failure: &Option<(FailureCategory, String)>,
) -> Result<(bool, String, Option<FailureCategory>, Option<Stage>), OyaError> {
    let failure_context = match last_failure {
        Some((cat, msg)) => format!(
            "\n\nPREVIOUS FAILURE: {:?}\nERROR OUTPUT:\n{}\n\nFix the issue that caused this failure.",
            cat,
            msg.chars().take(2000).collect::<String>()
        ),
        None => String::new(),
    };

    let prompt = format!(
        r#"You are producing an implementation plan for: {}

Request context: {}
Attempt: {}
{}

TASK:
1. Create/update PLAN.md with exact implementation steps
2. Include test strategy and quality gates
3. Keep plan aligned to current codebase

Just write files. Do not explain."#,
        bead_id, context, attempt, failure_context
    );

    let (opencode_ok, opencode_output) = run_opencode(&prompt)?;

    if !opencode_ok {
        return Ok((
            false,
            opencode_output,
            Some(FailureCategory::OutputParseFailure),
            Some(Stage::Plan),
        ));
    }

    let (check_ok, check_output) = run_moon_check()?;
    if check_ok {
        Ok((true, "Planning completed".to_string(), None, Some(Stage::Contract)))
    } else {
        Ok((false, check_output, Some(FailureCategory::CompileFailed), Some(Stage::Plan)))
    }
}

fn execute_tdd15(
    bead_id: &str,
    attempt: u32,
    context: &str,
    last_failure: &Option<(FailureCategory, String)>,
) -> Result<(bool, String, Option<FailureCategory>, Option<Stage>), OyaError> {
    let failure_context = match last_failure {
        Some((cat, msg)) => format!(
            "\n\nPREVIOUS FAILURE: {:?}\nERROR OUTPUT:\n{}\n\nFix the issue that caused this failure.",
            cat,
            msg.chars().take(2000).collect::<String>()
        ),
        None => String::new(),
    };

    let prompt = format!(
        r#"You are implementing TDD for: {}

Previous context: {}
Attempt: {}
{}

TASK: 
1. Write tests in src/lib.rs for the functionality
2. Implement the code to pass those tests
3. Ensure `cargo test` passes

Just write the code. Do not explain."#,
        bead_id, context, attempt, failure_context
    );

    let (opencode_ok, opencode_output) = run_opencode(&prompt)?;

    if !opencode_ok {
        return Ok((
            false,
            opencode_output,
            Some(FailureCategory::OutputParseFailure),
            Some(Stage::Tdd15),
        ));
    }

    let (check_ok, check_output) = run_moon_check()?;
    if !check_ok {
        return Ok((false, check_output, Some(FailureCategory::CompileFailed), Some(Stage::Tdd15)));
    }

    let (test_ok, test_output) = run_moon_test()?;

    if test_ok {
        Ok((true, "Tests written and passing".to_string(), None, Some(Stage::Qa)))
    } else {
        Ok((false, test_output, Some(FailureCategory::TestFailed), Some(Stage::Tdd15)))
    }
}

fn execute_qa(
    bead_id: &str,
    attempt: u32,
    context: &str,
    last_failure: &Option<(FailureCategory, String)>,
) -> Result<(bool, String, Option<FailureCategory>, Option<Stage>), OyaError> {
    let failure_context = match last_failure {
        Some((cat, msg)) => format!(
            "\n\nPREVIOUS FAILURE: {:?}\nERROR OUTPUT:\n{}\n\nFix the issue that caused this failure.",
            cat,
            msg.chars().take(2000).collect::<String>()
        ),
        None => String::new(),
    };

    let prompt = format!(
        r#"You are performing QA for: {}

Previous context: {}
Attempt: {}
{}

TASK: 
1. Add edge case tests
2. Add error handling tests  
3. Ensure all code paths are covered
4. Fix any issues found

Just write the code. Do not explain."#,
        bead_id, context, attempt, failure_context
    );

    let (opencode_ok, opencode_output) = run_opencode(&prompt)?;

    if !opencode_ok {
        return Ok((
            false,
            opencode_output,
            Some(FailureCategory::OutputParseFailure),
            Some(Stage::Qa),
        ));
    }

    let (test_ok, test_output) = run_moon_test()?;

    if test_ok {
        Ok((true, "QA tests added and passing".to_string(), None, Some(Stage::RedQueen)))
    } else {
        Ok((false, test_output, Some(FailureCategory::TestFailed), Some(Stage::Tdd15)))
    }
}

fn execute_red_queen(
    bead_id: &str,
    attempt: u32,
    context: &str,
    last_failure: &Option<(FailureCategory, String)>,
) -> Result<(bool, String, Option<FailureCategory>, Option<Stage>), OyaError> {
    let failure_context = match last_failure {
        Some((cat, msg)) => format!(
            "\n\nPREVIOUS FAILURE: {:?}\nERROR OUTPUT:\n{}\n\nFix the issue that caused this failure.",
            cat,
            msg.chars().take(2000).collect::<String>()
        ),
        None => String::new(),
    };

    let prompt = format!(
        r#"You are running adversarial Red Queen testing for: {}

Previous context: {}
Attempt: {}
{}

TASK:
1. Write adversarial tests that try to break the code
2. Test boundary conditions
3. Test malformed inputs
4. Fix any vulnerabilities found

Just write the code. Do not explain."#,
        bead_id, context, attempt, failure_context
    );

    let (opencode_ok, opencode_output) = run_opencode(&prompt)?;

    if !opencode_ok {
        return Ok((
            false,
            opencode_output,
            Some(FailureCategory::OutputParseFailure),
            Some(Stage::RedQueen),
        ));
    }

    let (test_ok, test_output) = run_moon_test()?;

    if test_ok {
        Ok((true, "Adversarial tests pass".to_string(), None, Some(Stage::GptReview)))
    } else {
        Ok((false, test_output, Some(FailureCategory::TestFailed), Some(Stage::Tdd15)))
    }
}

fn execute_gpt_review(
    bead_id: &str,
    attempt: u32,
    context: &str,
    last_failure: &Option<(FailureCategory, String)>,
) -> Result<(bool, String, Option<FailureCategory>, Option<Stage>), OyaError> {
    let failure_context = match last_failure {
        Some((FailureCategory::LintFailed, msg)) => {
            format!(
                "\n\nPREVIOUS CLIPPY FAILURE:\n{}\n\nCRITICAL: Fix the actual code issues. DO NOT use #[allow(...)] attributes to suppress warnings. Fix the underlying problem.",
                msg.chars().take(3000).collect::<String>()
            )
        }
        Some((cat, msg)) => format!(
            "\n\nPREVIOUS FAILURE: {:?}\nERROR OUTPUT:\n{}\n\nFix the issue.",
            cat,
            msg.chars().take(2000).collect::<String>()
        ),
        None => String::new(),
    };

    let prompt = format!(
        r#"You are reviewing code for: {}

Previous context: {}
Attempt: {}
{}

TASK:
1. Review all code in src/
2. Fix any code quality issues
3. Add missing documentation
4. Ensure clippy is happy with no warnings

IMPORTANT RULES:
- DO NOT use #[allow(...)] attributes to suppress warnings
- Fix the actual underlying code issues
- Remove dead code instead of allowing it
- Fix type issues properly, don't work around them

Just fix the code. Do not explain."#,
        bead_id, context, attempt, failure_context
    );

    let (opencode_ok, opencode_output) = run_opencode(&prompt)?;

    if !opencode_ok {
        return Ok((
            false,
            opencode_output,
            Some(FailureCategory::OutputParseFailure),
            Some(Stage::GptReview),
        ));
    }

    let (clippy_ok, clippy_output) = run_moon_quick()?;
    if !clippy_ok {
        return Ok((
            false,
            clippy_output,
            Some(FailureCategory::LintFailed),
            Some(Stage::GptReview),
        ));
    }

    let (test_ok, test_output) = run_moon_test()?;

    if test_ok {
        Ok((true, "Code review complete, clippy clean".to_string(), None, Some(Stage::ShipGate)))
    } else {
        Ok((false, test_output, Some(FailureCategory::TestFailed), Some(Stage::Tdd15)))
    }
}

fn execute_ship_gate(
    _bead_id: &str,
    _attempt: u32,
    _context: &str,
    _last_failure: &Option<(FailureCategory, String)>,
) -> Result<(bool, String, Option<FailureCategory>, Option<Stage>), OyaError> {
    tracing::info!("SHIP GATE: Running final validation");

    let (ci_ok, ci_output) = run_moon_ci()?;
    if !ci_ok {
        return Ok((false, ci_output, Some(FailureCategory::CompileFailed), Some(Stage::Tdd15)));
    }

    let (zjj_ok, zjj_output) = run_zjj_done_dry_run()?;
    if !zjj_ok {
        return Ok((
            false,
            zjj_output,
            Some(FailureCategory::MergeConflict),
            Some(Stage::GptReview),
        ));
    }

    let (quick_ok, quick_output) = run_moon_quick()?;
    if !quick_ok {
        return Ok((
            false,
            quick_output,
            Some(FailureCategory::LintFailed),
            Some(Stage::GptReview),
        ));
    }

    let (test_ok, test_output) = run_moon_test()?;
    if !test_ok {
        return Ok((false, test_output, Some(FailureCategory::TestFailed), Some(Stage::Tdd15)));
    }

    tracing::info!("SHIP GATE: ALL CHECKS PASSED");
    Ok((true, "All gates passed - ready to ship".to_string(), None, None))
}

fn get_db() -> Result<Arc<OyaDb>, OyaError> {
    DB.get().cloned().ok_or_else(|| OyaError("DB not initialized".to_string()))
}

fn repo_root() -> Result<PathBuf, OyaError> {
    if let Ok(configured_root) = std::env::var("OYA_REPO_ROOT") {
        return Ok(PathBuf::from(configured_root));
    }
    std::env::current_dir().map_err(|e| OyaError(format!("Failed to resolve repo root: {}", e)))
}

fn resolve_bind_addr() -> Result<std::net::SocketAddr, OyaError> {
    let configured = std::env::var("OYA_BIND_ADDR").ok();
    let value = configured.map_or_else(|| "127.0.0.1:9080".to_string(), |address| address);

    value.parse().map_err(|e| OyaError(format!("Invalid OYA_BIND_ADDR '{}': {}", value, e)))
}

const REQUIRED_OBJECT_HANDLERS: [&str; 3] = ["start", "get_status", "ping"];

fn validate_required_object_handlers<'a, I>(handler_names: I) -> Result<(), OyaError>
where
    I: IntoIterator<Item = &'a str>,
{
    let mut counts = std::collections::BTreeMap::<String, usize>::new();
    for handler in handler_names {
        let current = counts.get(handler).copied().map_or(0, |count| count);
        counts.insert(handler.to_string(), current + 1);
    }

    let missing: Vec<&str> = REQUIRED_OBJECT_HANDLERS
        .iter()
        .copied()
        .filter(|required| !counts.contains_key(*required))
        .collect();
    if !missing.is_empty() {
        return Err(OyaError(format!(
            "Startup self-check failed: missing required handlers [{}]",
            missing.join(", ")
        )));
    }

    let duplicate: Vec<String> = counts
        .iter()
        .filter(|(_, count)| **count > 1)
        .map(|(name, count)| format!("{}x{}", name, count))
        .collect();
    if !duplicate.is_empty() {
        return Err(OyaError(format!(
            "Startup self-check failed: duplicate handler wiring [{}]",
            duplicate.join(", ")
        )));
    }

    Ok(())
}

fn startup_self_check_guard<S>(_service: &S) -> Result<(), OyaError>
where
    S: Discoverable,
{
    let metadata = S::discover();
    let handler_names: Vec<&str> =
        metadata.handlers.iter().map(|handler| handler.name.as_str()).collect();

    validate_required_object_handlers(handler_names)
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    tracing_subscriber::fmt().with_max_level(tracing::Level::INFO).init();

    let db = OyaDb::connect("oya-db")
        .await
        .map_err(|e| format!("Failed to connect to Sled DB: {}", e))?;
    db.init_schema().await.map_err(|e| format!("Failed to initialize schema: {}", e))?;
    DB.set(Arc::new(db)).map_err(|_| "Failed to set DB")?;

    tracing::info!("OYA Orchestrator starting on port 9080");
    tracing::info!("Using REAL execution: opencode CLI + moon/zjj quality gates");

    let service = OyaOrchestratorImpl.serve();
    startup_self_check_guard(&service)?;
    let endpoint = Endpoint::builder().bind(service).build();

    let bind_addr = resolve_bind_addr()?;
    HttpServer::new(endpoint).listen_and_serve(bind_addr).await;

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn validate_required_object_handlers_passes_with_all_required_handlers() {
        let handlers = ["start", "get_status", "ping"];

        assert!(validate_required_object_handlers(handlers).is_ok());
    }

    #[test]
    fn validate_required_object_handlers_fails_when_required_handler_is_missing() {
        let handlers = ["start", "get_status"];

        let result = validate_required_object_handlers(handlers);

        assert!(result.is_err());
        let message =
            result.err().map_or_else(|| "missing error".to_string(), |error| error.to_string());
        assert!(message.contains("ping"));
    }

    #[test]
    fn validate_required_object_handlers_fails_on_duplicate_wiring() {
        let handlers = ["start", "start", "get_status", "ping"];

        let result = validate_required_object_handlers(handlers);

        assert!(result.is_err());
        let message =
            result.err().map_or_else(|| "missing error".to_string(), |error| error.to_string());
        assert!(message.contains("startx2"));
    }

    #[test]
    fn startup_self_check_guard_passes_for_orchestrator_service_wiring() {
        let service = OyaOrchestratorImpl.serve();

        assert!(startup_self_check_guard(&service).is_ok());
    }

    #[test]
    fn parse_start_request_accepts_json_object_payload() {
        let request = serde_json::json!({
            "bead_id": "manual-e2e-bead",
            "context": "object payload"
        });

        let parsed = parse_start_request(request);

        assert!(parsed.is_ok());
        assert_eq!(
            parsed.as_ref().ok().and_then(|value| value.bead_id.as_deref()),
            Some("manual-e2e-bead")
        );
        assert_eq!(
            parsed.as_ref().ok().and_then(|value| value.context.as_deref()),
            Some("object payload")
        );
    }

    #[test]
    fn parse_start_request_accepts_json_string_payload() {
        let request =
            serde_json::json!("{\"bead_id\":\"manual-e2e-bead\",\"context\":\"string payload\"}");

        let parsed = parse_start_request(request);

        assert!(parsed.is_ok());
        assert_eq!(
            parsed.as_ref().ok().and_then(|value| value.bead_id.as_deref()),
            Some("manual-e2e-bead")
        );
        assert_eq!(
            parsed.as_ref().ok().and_then(|value| value.context.as_deref()),
            Some("string payload")
        );
    }

    #[test]
    fn parse_start_request_rejects_non_object_non_string_payload() {
        let request = serde_json::json!(123);

        let result = parse_start_request(request);

        assert!(result.is_err());
        let message =
            result.err().map_or_else(|| "missing error".to_string(), |error| error.to_string());
        assert!(message.contains("expected object or JSON string"));
    }
}
