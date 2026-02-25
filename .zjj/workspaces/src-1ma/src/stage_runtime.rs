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
        Stage::Explore => {
            "TASK: Use Codanna-only discovery to produce a minimal context pack: symbols, callers, impact, and exact file paths for this bead. Keep output stable and concise.\n\nJust write the code. Do not explain."
        }
        Stage::Contract => {
            "TASK: Write a design contract as a Rust doc comment in src/lib.rs (create if needed).\n\nInclude:\n1. Purpose and goals\n2. Key functions to implement\n3. Acceptance criteria\n\nJust write the code. Do not explain."
        }
        Stage::Red => {
            "TASK:\n1. Create or update acceptance tests for this bead as ATDD specifications\n2. Ensure tests COMPILE but FAIL (red state)\n3. Do not modify production implementation in this stage\n4. Keep acceptance tests immutable after they are sealed\n\nJust write the code. Do not explain."
        }
        Stage::Implementation => {
            "TASK:\n1. Write tests that encode the contract invariants\n2. Implement the code to make those tests pass (GREEN state)\n3. Use Result<T, E> for all fallible operations - NO unwrap/expect\n4. Pure functions in core, IO only at shell boundaries\n5. Ensure `moon run :test` passes and clippy is clean\n\nCRITICAL: Tests MUST pass. Fix the underlying code issues, never suppress with #[allow(...)].\n\nJust write the code. Do not explain."
        }
        Stage::Witness => {
            "TASK: Prepare implementation for holdout scenario validation and emit only stable artifacts.\n\nJust write the code. Do not explain."
        }
        Stage::ShipGate => "",
    };

    format!("{}{}", header, body)
}

pub(super) fn execute_witness_gate(repo_root: PathBuf) -> Result<StageExecution, OyaError> {
    let prompt = witness_prompt();
    let gate = Gate::HoldoutScenarios;
    let evidence = execute_gate(gate.clone(), &repo_root)?;
    let gate_results = vec![GateResultData {
        gate: gate.as_str().to_string(),
        passed: evidence.passed,
        exit_code: evidence.exit_code,
        command: evidence.command.clone(),
        output: truncate_clean(&sanitize_holdout_output(evidence.output.as_str()), 4000),
    }];

    if evidence.passed {
        return Ok(StageExecution {
            passed: true,
            output: "Holdout scenario suite passed".to_string(),
            failure_category: None,
            next_stage: Some(Stage::ShipGate),
            prompt,
            gate_results,
        });
    }

    let (failure, next_stage) = gate_failure_outcome(&Stage::Witness, &gate);
    Ok(StageExecution {
        passed: false,
        output: format_gate_command_output(
            evidence.command.as_str(),
            evidence.exit_code,
            sanitize_holdout_output(evidence.output.as_str()).as_str(),
        ),
        failure_category: Some(failure),
        next_stage: Some(next_stage),
        prompt,
        gate_results,
    })
}

fn witness_prompt() -> String {
    "Witness executes holdout scenarios only (moon :holdout); no OpenCode prompt".to_string()
}

fn sanitize_holdout_output(raw: &str) -> String {
    let redacted = raw
        .lines()
        .map(str::trim_end)
        .filter(|line| is_safe_holdout_line(line))
        .take(60)
        .collect::<Vec<_>>()
        .join("\n");

    if redacted.is_empty() {
        "holdout output redacted for scenario isolation".to_string()
    } else {
        redacted
    }
}

fn is_safe_holdout_line(line: &str) -> bool {
    let lower = line.to_ascii_lowercase();
    let has_sensitive_tokens = [
        "scenario id",
        "scenario:",
        "assert_",
        "assert!",
        "given/",
        "when/",
        "then/",
        "expected:",
        "fixtures/",
        "vault/",
        "scenarios/",
        ".yaml",
        ".yml",
    ]
    .iter()
    .any(|token| lower.contains(token));

    !has_sensitive_tokens
        && (lower.contains("running ")
            || lower.contains("test result:")
            || lower.contains("finished")
            || lower.contains("blocking")
            || lower.contains("error: test failed"))
}

pub(super) fn stage_success(stage: &Stage) -> (&'static str, Option<Stage>) {
    match stage {
        Stage::Explore => ("Explore context pack completed", Some(Stage::Contract)),
        Stage::Contract => ("Contract written and compiles", Some(Stage::Red)),
        Stage::Red => ("Acceptance tests are RED and sealed", Some(Stage::Implementation)),
        Stage::Implementation => ("Implementation complete, tests GREEN", Some(Stage::Witness)),
        Stage::Witness => ("Witness checks passed", Some(Stage::ShipGate)),
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
            tracing::info!("SHIP GATE: skipping zjj merge queue check (disabled)");
            continue;
        }

        let gate_evidence = run_gate(gate.clone())?;
        if let Some(message) = stale_evidence_message(&gate, &gate_evidence) {
            let stale_evidence = stale_evidence_failure(gate_evidence, message);
            gate_results.push(GateResultData {
                gate: gate.as_str().to_string(),
                passed: false,
                exit_code: stale_evidence.exit_code,
                command: stale_evidence.command.clone(),
                output: truncate_clean(&stale_evidence.output, 4000),
            });
            return Ok(ship_gate_failure(gate, stale_evidence, prompt, gate_results));
        }
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

fn stale_evidence_message(gate: &Gate, evidence: &GateEvidence) -> Option<String> {
    if *gate != Gate::CueArtifactGenerated {
        return None;
    }
    match (&evidence.revision, &evidence.current_revision) {
        (Some(pinned), Some(current)) if pinned != current => Some(format!(
            "stale evidence rejected: pinned_revision={} current_head={}",
            pinned, current
        )),
        (Some(_), Some(_)) => None,
        _ => Some("stale evidence rejected: missing revision metadata".to_string()),
    }
}

fn stale_evidence_failure(mut evidence: GateEvidence, message: String) -> GateEvidence {
    evidence.passed = false;
    evidence.exit_code = 1;
    evidence.output = if evidence.output.is_empty() {
        message
    } else {
        format!("{}\n{}", message, evidence.output)
    };
    evidence
}

#[cfg(test)]
mod tests {
    use super::{
        is_safe_holdout_line, sanitize_holdout_output, stale_evidence_message, Gate, GateEvidence,
    };

    #[test]
    fn sanitize_holdout_output_removes_sensitive_lines() {
        let raw = "running 2 tests\nScenario ID: scn-secret-001\nGiven/ a hidden precondition\nassert_eq!(a, b)\ntest result: FAILED. 1 passed; 1 failed;";
        let sanitized = sanitize_holdout_output(raw);
        assert!(!sanitized.contains("Scenario ID"));
        assert!(!sanitized.contains("Given/"));
        assert!(!sanitized.contains("assert_eq!"));
        assert!(sanitized.contains("running 2 tests"));
        assert!(sanitized.contains("test result: FAILED"));
    }

    #[test]
    fn sanitize_holdout_output_has_safe_fallback_message() {
        let raw = "Scenario: hidden\nTHEN/ secret expectation\nassert!(false)";
        let sanitized = sanitize_holdout_output(raw);
        assert_eq!(sanitized, "holdout output redacted for scenario isolation");
    }

    #[test]
    fn is_safe_holdout_line_allows_summary_but_blocks_leaks() {
        assert!(is_safe_holdout_line("test result: ok. 4 passed; 0 failed;"));
        assert!(!is_safe_holdout_line("Scenario: holdout-password-reset-expired"));
        assert!(!is_safe_holdout_line("expected: HTTP 410 Gone"));
    }

    #[test]
    fn stale_evidence_message_detects_revision_mismatch_for_ship_gate() {
        let evidence = GateEvidence {
            command: "moon run :cue-check".to_string(),
            passed: true,
            exit_code: 0,
            output: "ok".to_string(),
            revision: Some("aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa".to_string()),
            current_revision: Some("bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb".to_string()),
        };
        let message = stale_evidence_message(&Gate::CueArtifactGenerated, &evidence);
        assert!(message.is_some());
    }

    #[test]
    fn stale_evidence_message_ignores_matching_revision() {
        let evidence = GateEvidence {
            command: "moon run :cue-check".to_string(),
            passed: true,
            exit_code: 0,
            output: "ok".to_string(),
            revision: Some("aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa".to_string()),
            current_revision: Some("aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa".to_string()),
        };
        let message = stale_evidence_message(&Gate::CueArtifactGenerated, &evidence);
        assert!(message.is_none());
    }
}
