use super::*;
use crate::orchestrator_types::GateResultData;
use crate::pipeline::MergeQueuePolicy;
use crate::stage_executor::StageExecution;
use oya::types::Gate;

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
        Stage::Contract => {
            "TASK: Write a design contract as a Rust doc comment in src/lib.rs (create if needed).\n\nInclude:\n1. Purpose and goals\n2. Key functions to implement\n3. Acceptance criteria\n\nJust write the code. Do not explain."
        }
        Stage::Implementation => {
            "TASK:\n1. Write tests that encode the contract invariants\n2. Implement the code to make those tests pass (GREEN state)\n3. Use Result<T, E> for all fallible operations - NO unwrap/expect\n4. Pure functions in core, IO only at shell boundaries\n5. Ensure `moon run :test` passes and clippy is clean\n\nCRITICAL: Tests MUST pass. Fix the underlying code issues, never suppress with #[allow(...)].\n\nJust write the code. Do not explain."
        }
        Stage::ShipGate => "",
    };

    format!("{}{}", header, body)
}

pub(super) fn stage_success(stage: &Stage) -> (&'static str, Option<Stage>) {
    match stage {
        Stage::Contract => ("Contract written and compiles", Some(Stage::Implementation)),
        Stage::Implementation => ("Implementation complete, tests GREEN", Some(Stage::ShipGate)),
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
    let mut gate_results = Vec::new();

    for gate in Stage::ShipGate.gates() {
        if !merge_queue_policy.should_run(&gate) {
            tracing::info!("SHIP GATE: skipping zjj merge queue check (OYA_SKIP_ZJJ_GATE=1)");
            continue;
        }

        let gate_evidence = run_gate(gate.clone())?;
        gate_results.push(GateResultData {
            gate: gate.as_str().to_string(),
            passed: gate_evidence.passed,
            exit_code: gate_evidence.exit_code,
            command: gate_evidence.command.clone(),
            output: truncate_clean(&gate_evidence.output, 4000),
        });
        if !gate_evidence.passed {
            return Ok(ship_gate_failure(gate, gate_evidence, prompt, gate_results));
        }
    }

    tracing::info!("SHIP GATE: ALL CHECKS PASSED");
    Ok(StageExecution {
        passed: true,
        output: "All gates passed - ready to ship".to_string(),
        failure_category: None,
        next_stage: None,
        prompt,
        gate_results,
    })
}

fn ship_gate_prompt() -> String {
    "Ship gate executes quality gates only (moon/zjj); no OpenCode prompt".to_string()
}

fn ship_gate_failure(
    gate: Gate,
    evidence: GateEvidence,
    prompt: String,
    gate_results: Vec<GateResultData>,
) -> StageExecution {
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
        gate_results,
    }
}
