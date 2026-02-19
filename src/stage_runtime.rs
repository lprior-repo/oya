use super::*;
use crate::pipeline::MergeQueuePolicy;
use crate::stage_executor::StageExecution;

fn format_gate_command_output(command: &str, exit_code: i32, output: &str) -> String {
    format!("command={} exit_code={}\n{}", command, exit_code, output)
}

pub(super) struct StagePromptInput<'a> {
    pub(super) stage: &'a Stage,
    pub(super) bead_id: &'a str,
    pub(super) context: &'a str,
    pub(super) attempt: u32,
    pub(super) failure_context: &'a str,
}

pub(super) fn stage_prompt(input: StagePromptInput<'_>) -> String {
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

pub(super) fn stage_success(stage: &Stage) -> (&'static str, Option<Stage>) {
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

pub(super) struct ShipGateRequest {
    pub(super) attempt: u32,
    pub(super) merge_queue_policy: MergeQueuePolicy,
    pub(super) repo_root: PathBuf,
}

pub(super) fn execute_ship_gate(request: ShipGateRequest) -> Result<StageExecution, OyaError> {
    if request.attempt == 0 {
        return Err(OyaError("attempt must be greater than 0".to_string()));
    }
    execute_ship_gate_with_gate_runner(request.merge_queue_policy, |gate| {
        execute_gate(gate, &request.repo_root)
    })
}

pub(super) fn execute_ship_gate_with_gate_runner<F>(
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
