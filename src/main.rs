#![deny(clippy::unwrap_used)]
#![deny(clippy::expect_used)]
#![deny(clippy::panic)]
#![forbid(unsafe_code)]

mod opencode_client;

use oya::domain::{
    AgentId, AgentState, AgentStatus, ApproverMode, BeadId, FailureCategory, GateResult,
    Run as BeadRun, RunId, RunState, ShipDecision, StageAttempt, StageName as Stage, StageResult,
    StageState,
};
use oya::infrastructure::persistence::{self, OyaDb};
use oya::infrastructure::zjj::zjj_done_has_constraint_violation;
use restate_sdk::endpoint::Endpoint;
use restate_sdk::http_server::HttpServer;
use restate_sdk::prelude::*;
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

impl From<persistence::OyaDbError> for OyaError {
    fn from(e: persistence::OyaDbError) -> Self {
        OyaError(e.to_string())
    }
}

impl From<OyaError> for HandlerError {
    fn from(e: OyaError) -> Self {
        HandlerError::from(e.0)
    }
}

#[restate_sdk::object]
pub trait OyaOrchestrator {
    async fn start(request: String) -> Result<String, HandlerError>;
    async fn get_status() -> Result<String, HandlerError>;
}

pub struct OyaOrchestratorImpl;

impl OyaOrchestrator for OyaOrchestratorImpl {
    async fn start(&self, ctx: ObjectContext<'_>, request: String) -> Result<String, HandlerError> {
        let parsed: serde_json::Value =
            serde_json::from_str(&request).map_err(|e| OyaError(format!("Invalid JSON: {}", e)))?;

        let bead_id = parsed["bead_id"].as_str().map_or("unknown", |s| s).to_string();
        let context = parsed["context"].as_str().map_or("", |s| s).to_string();

        let db = get_db()?;

        // 1. Create and persist new run (Pending)
        let mut run = BeadRun::new(BeadId::new(bead_id.clone()));
        let run_id = ctx.key().to_string();
        run.id = RunId(run_id.clone());

        // 2. Initialize Agent State
        // For this single-process runner, we'll assign a new AgentId per run for now,
        // or effectively treat this process as a fresh agent context.
        let agent_id = AgentId::new();
        let mut agent_state = AgentState::new(
            agent_id.clone(),
            None, // Will link when work starts
            None,
            AgentStatus::Idle,
            0,
        );
        db.insert_agent_state(&agent_state).await?;

        tracing::info!("=== RUN {} STARTED ===", run_id);
        tracing::info!("Bead: {}", bead_id);
        tracing::info!("Context: {}", context);
        tracing::info!("Agent: {}", agent_id.as_str());

        // 3. Start the run (Pending -> Running)
        run = run.start().map_err(|e| OyaError(e.to_string()))?;
        db.insert_bead_run(&run).await?;

        // Update Agent to Working on this Bead
        agent_state.bead_id = Some(run.bead_id.clone());
        agent_state.status = AgentStatus::Working;

        let mut current_stage = match &run.state {
            RunState::Running { current_stage } => current_stage.clone(),
            _ => return Err(HandlerError::from("Failed to start run")),
        };
        agent_state.current_stage = Some(current_stage.clone());
        agent_state.validate_invariants().map_err(OyaError)?;
        db.insert_agent_state(&agent_state).await?;

        let mut attempt = 1u32;
        let mut last_failure: Option<(FailureCategory, String)> = None;

        loop {
            tracing::info!("");
            tracing::info!("=== STAGE: {:?} (attempt {}) ===", current_stage, attempt);
            if let Some((ref cat, ref msg)) = last_failure {
                tracing::info!("Previous failure: {:?} - {} chars of output", cat, msg.len());
            }

            let last_failure_clone = last_failure.clone();
            agent_state.current_stage = Some(current_stage.clone());
            agent_state.implementation_attempt = attempt;
            agent_state.status = AgentStatus::Working;
            // Need to update timestamp manually or make it automatic in setter?
            // The domain struct is public fields for now, so manual update.
            agent_state.last_update = chrono::Utc::now();
            db.insert_agent_state(&agent_state).await?;

            let stage = current_stage.clone();
            let run_id_clone = run_id.clone();
            let bead_id_clone = bead_id.clone();
            let context_clone = context.clone();
            let db_clone = Arc::clone(&db);

            let stage_result = execute_stage_real(
                &db_clone,
                &run_id_clone,
                &bead_id_clone,
                stage,
                attempt,
                &context_clone,
                last_failure_clone,
            )
            .await
            .map_err(|e| HandlerError::from(e.to_string()))?;

            db.insert_stage_result(&stage_result).await?;
            for gate in current_stage.gates() {
                let command = Some(format!("moon gate:{}", gate.as_str()));
                let gate_result = GateResult {
                    run_id: run_id.clone(),
                    gate_name: format!(
                        "{}:{:03}:{}",
                        current_stage.as_str(),
                        attempt,
                        gate.as_str()
                    ),
                    command,
                    passed: stage_result.passed,
                    exit_code: if stage_result.passed { 0 } else { 1 },
                    log_ref: None,
                };
                gate_result
                    .validate()
                    .map_err(|e| OyaError(format!("Invalid gate evidence: {:?}", e)))?;
                db.insert_gate_result(&gate_result).await?;
            }

            if stage_result.passed {
                tracing::info!("STAGE {:?} PASSED", current_stage);
                last_failure = None;

                // Update domain state (transition to next stage or ship)
                run = run
                    .complete_stage(current_stage.clone(), stage_result.clone())
                    .map_err(|e| OyaError(e.to_string()))?;

                db.update_run_state(&run_id, &run.state).await?;

                match &run.state {
                    RunState::Running { current_stage: next } => {
                        current_stage = next.clone();
                        attempt = 1;

                        // Agent remains Working, just stage updates next loop
                    }
                    RunState::Shipped { .. } => {
                        // Record decision for audit
                        let decision = ShipDecision {
                            run_id: run_id.clone(),
                            shipped: true,
                            rationale: "All stages passed".to_string(),
                            approver_mode: ApproverMode::Auto,
                            timestamp: chrono::Utc::now(),
                        };
                        db.insert_ship_decision(&decision).await?;

                        // Agent is Done
                        agent_state.status = AgentStatus::Done;
                        agent_state.bead_id = None; // As per Done invariant
                        agent_state.current_stage = None;
                        agent_state.last_update = chrono::Utc::now();
                        agent_state.validate_invariants().map_err(OyaError)?;
                        db.insert_agent_state(&agent_state).await?;

                        tracing::info!("");
                        tracing::info!("=== RUN {} SHIPPED ===", run_id);
                        return Ok(run_id);
                    }
                    _ => {
                        return Err(HandlerError::from("Unexpected run state after success"));
                    }
                }
            } else {
                tracing::warn!(
                    "STAGE {:?} FAILED: {:?}",
                    current_stage,
                    stage_result.failure_category
                );

                last_failure = stage_result
                    .failure_category
                    .clone()
                    .zip(Some(stage_result.output.to_string()));

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
                    return Ok(run_id);
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

            let _ = ctx.sleep(std::time::Duration::from_millis(100)).await;
        }
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

fn run_opencode(prompt: &str) -> Result<(bool, String), OyaError> {
    tracing::info!("Running opencode with prompt ({} chars)", prompt.len());

    let output = Command::new("opencode")
        .args(["run", "--format", "json", prompt])
        .current_dir(repo_root()?)
        .output()
        .map_err(|e| OyaError(format!("Failed to run opencode: {}", e)))?;

    let stdout = String::from_utf8_lossy(&output.stdout).to_string();
    let stderr = String::from_utf8_lossy(&output.stderr).to_string();

    tracing::debug!("OpenCode exit code: {}", output.status.code().map_or(-1, |c| c));
    if !stderr.is_empty() {
        tracing::debug!("OpenCode stderr: {}", stderr);
    }

    let success = output.status.success();
    Ok((success, stdout))
}

fn run_moon_check() -> Result<(bool, String), OyaError> {
    tracing::info!("Running moon :check");

    let output = Command::new("moon")
        .args(["run", ":check"])
        .current_dir(repo_root()?)
        .output()
        .map_err(|e| OyaError(format!("Failed to run moon :check: {}", e)))?;

    let stdout = String::from_utf8_lossy(&output.stdout).to_string();
    let stderr = String::from_utf8_lossy(&output.stderr).to_string();
    let combined = format!("{}\n{}", stdout, stderr);

    let success = output.status.success();
    tracing::info!(
        "moon :check: {} ({})",
        if success { "PASS" } else { "FAIL" },
        output.status.code().map_or(-1, |c| c)
    );

    Ok((success, combined))
}

fn run_moon_test() -> Result<(bool, String), OyaError> {
    tracing::info!("Running moon :test");

    let output = Command::new("moon")
        .args(["run", ":test"])
        .current_dir(repo_root()?)
        .output()
        .map_err(|e| OyaError(format!("Failed to run moon :test: {}", e)))?;

    let stdout = String::from_utf8_lossy(&output.stdout).to_string();
    let stderr = String::from_utf8_lossy(&output.stderr).to_string();
    let combined = format!("{}\n{}", stdout, stderr);

    let success = output.status.success();
    tracing::info!(
        "moon :test: {} ({})",
        if success { "PASS" } else { "FAIL" },
        output.status.code().map_or(-1, |c| c)
    );

    Ok((success, combined))
}

fn run_moon_quick() -> Result<(bool, String), OyaError> {
    tracing::info!("Running moon :quick");

    let output = Command::new("moon")
        .args(["run", ":quick"])
        .current_dir(repo_root()?)
        .output()
        .map_err(|e| OyaError(format!("Failed to run moon :quick: {}", e)))?;

    let stdout = String::from_utf8_lossy(&output.stdout).to_string();
    let stderr = String::from_utf8_lossy(&output.stderr).to_string();
    let combined = format!("{}\n{}", stdout, stderr);

    let success = output.status.success();
    tracing::info!(
        "moon :quick: {} ({})",
        if success { "PASS" } else { "FAIL" },
        output.status.code().map_or(-1, |c| c)
    );

    Ok((success, combined))
}

fn run_moon_ci() -> Result<(bool, String), OyaError> {
    tracing::info!("Running moon :ci");

    let output = Command::new("moon")
        .args(["run", ":ci"])
        .current_dir(repo_root()?)
        .output()
        .map_err(|e| OyaError(format!("Failed to run moon :ci: {}", e)))?;

    let stdout = String::from_utf8_lossy(&output.stdout).to_string();
    let stderr = String::from_utf8_lossy(&output.stderr).to_string();
    let combined = format!("{}\n{}", stdout, stderr);

    let success = output.status.success();
    tracing::info!(
        "moon :ci: {} ({})",
        if success { "PASS" } else { "FAIL" },
        output.status.code().map_or(-1, |c| c)
    );

    Ok((success, combined))
}

fn run_zjj_done_dry_run() -> Result<(bool, String), OyaError> {
    tracing::info!("Running zjj done --dry-run");

    let output = Command::new("zjj")
        .args(["done", "--dry-run"])
        .current_dir(repo_root()?)
        .output()
        .map_err(|e| OyaError(format!("Failed to run zjj done --dry-run: {}", e)))?;

    let stdout = String::from_utf8_lossy(&output.stdout).to_string();
    let stderr = String::from_utf8_lossy(&output.stderr).to_string();
    let combined = format!("{}\n{}", stdout, stderr);
    let success = output.status.success();

    if !success && zjj_done_has_constraint_violation(&combined) {
        let guidance = "zjj/bead DB constraint violation detected. Run `zjj recover --diagnose`, then repair bead closed_at consistency before retrying.";
        return Ok((false, format!("{}\n{}", combined, guidance)));
    }

    Ok((success, combined))
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

    let endpoint = Endpoint::builder().bind(OyaOrchestratorImpl.serve()).build();

    let bind_addr: std::net::SocketAddr =
        std::env::var("OYA_BIND_ADDR").unwrap_or_else(|_| "127.0.0.1:9080".to_string()).parse()?;
    HttpServer::new(endpoint).listen_and_serve(bind_addr).await;

    Ok(())
}
