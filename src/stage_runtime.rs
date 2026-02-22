use super::*;
use crate::orchestrator_types::GateResultData;
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
    pub(super) repo_root: PathBuf,
}

pub(super) fn execute_ship_gate(request: ShipGateRequest) -> Result<StageExecution, OyaError> {
    if request.attempt == 0 {
        return Err(OyaError("attempt must be greater than 0".to_string()));
    }
    execute_ship_gate_with_gate_runner(|gate| execute_gate(gate, &request.repo_root))
}

pub(super) fn execute_ship_gate_with_gate_runner<F>(run_gate: F) -> Result<StageExecution, OyaError>
where
    F: Fn(Gate) -> Result<GateEvidence, OyaError>,
{
    tracing::info!("SHIP GATE: Running final validation");
    let prompt = ship_gate_prompt();
    let mut gate_results = Vec::new();

    for gate in Stage::ShipGate.gates() {
        let gate_evidence = run_gate(gate.clone())?;
        if let Some(failure) =
            cue_monitor_failure(&gate, &gate_evidence, prompt.as_str(), &mut gate_results)
        {
            return Ok(failure);
        }
        gate_results.push(gate_result_data(&gate, &gate_evidence));
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

fn cue_monitor_failure(
    gate: &Gate,
    gate_evidence: &GateEvidence,
    prompt: &str,
    gate_results: &mut Vec<GateResultData>,
) -> Option<StageExecution> {
    if !should_validate_cue_monitor(gate, gate_evidence) {
        return None;
    }

    if let Some(message) = stale_evidence_message(gate, gate_evidence) {
        let stale_evidence = stale_evidence_failure(gate_evidence.clone(), message);
        gate_results.push(gate_result_data(gate, &stale_evidence));
        return Some(ship_gate_failure(
            gate.clone(),
            stale_evidence,
            prompt.to_string(),
            gate_results.clone(),
        ));
    }

    if let Some(message) = cue_schema_failure_message(gate, gate_evidence) {
        let schema_evidence = main_drift_regression_failure(gate_evidence.clone(), message);
        gate_results.push(gate_result_data(gate, &schema_evidence));
        return Some(ship_gate_failure(
            gate.clone(),
            schema_evidence,
            prompt.to_string(),
            gate_results.clone(),
        ));
    }

    if let Some(message) = main_drift_regression_message(gate, gate_evidence) {
        let regression_evidence = main_drift_regression_failure(gate_evidence.clone(), message);
        gate_results.push(gate_result_data(gate, &regression_evidence));
        return Some(ship_gate_failure(
            gate.clone(),
            regression_evidence,
            prompt.to_string(),
            gate_results.clone(),
        ));
    }

    None
}

fn gate_result_data(gate: &Gate, evidence: &GateEvidence) -> GateResultData {
    GateResultData {
        gate: gate.as_str().to_string(),
        passed: evidence.passed,
        exit_code: evidence.exit_code,
        command: evidence.command.clone(),
        output: truncate_clean(&evidence.output, 4000),
    }
}

fn ship_gate_prompt() -> String {
    "Ship gate executes quality gates only (moon); no OpenCode prompt".to_string()
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

fn should_validate_cue_monitor(gate: &Gate, evidence: &GateEvidence) -> bool {
    *gate == Gate::CueArtifactGenerated && evidence.command.contains(":cue-check")
}

fn main_drift_regression_message(gate: &Gate, evidence: &GateEvidence) -> Option<String> {
    if *gate != Gate::CueArtifactGenerated {
        return None;
    }
    if !signals_main_drift_regression(evidence.output.as_str()) {
        return None;
    }
    Some("main drift monitor blocked land: main gates regressed".to_string())
}

fn cue_schema_failure_message(gate: &Gate, evidence: &GateEvidence) -> Option<String> {
    if *gate != Gate::CueArtifactGenerated {
        return None;
    }
    let line = cue_schema_failure_line(evidence.output.as_str())?;
    let artifact = cue_schema_failure_field(line, "artifact=").unwrap_or("unknown");
    let definition = cue_schema_failure_field(line, "definition=").unwrap_or("unknown");
    let path = cue_schema_failure_field(line, "path=").unwrap_or("unknown");
    Some(format!(
        "cue schema monitor blocked land: artifact={} definition={} path={}",
        artifact, definition, path
    ))
}

fn cue_schema_failure_line(output: &str) -> Option<&str> {
    output.lines().find(|line| line.trim_start().starts_with("cue_schema_failure ")).map(str::trim)
}

fn cue_schema_failure_field<'a>(line: &'a str, key: &str) -> Option<&'a str> {
    line.split_whitespace().find_map(|token| token.strip_prefix(key))
}

fn signals_main_drift_regression(output: &str) -> bool {
    let normalized = output.to_ascii_lowercase();
    normalized.contains("main-drift-monitor")
        && (normalized.contains("regress") || normalized.contains("regression"))
}

fn main_drift_regression_failure(mut evidence: GateEvidence, message: String) -> GateEvidence {
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
        execute_ship_gate_with_gate_runner, is_safe_holdout_line, sanitize_holdout_output,
        stale_evidence_message, FailureCategory, Gate, GateEvidence, Stage,
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

    #[test]
    fn given_main_drift_monitor_regression_when_execute_ship_gate_then_land_is_blocked() {
        let result = execute_ship_gate_with_gate_runner(|gate| {
            if gate == Gate::CueArtifactGenerated {
                return Ok(GateEvidence {
                    command: "moon run :cue-check".to_string(),
                    passed: true,
                    exit_code: 0,
                    output: "main-drift-monitor: main gates regress on origin/main".to_string(),
                    revision: Some("aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa".to_string()),
                    current_revision: Some("aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa".to_string()),
                });
            }
            Ok(GateEvidence {
                command: "moon run :check".to_string(),
                passed: true,
                exit_code: 0,
                output: "ok".to_string(),
                revision: None,
                current_revision: None,
            })
        });

        assert!(result.is_ok());
        let execution = match result {
            Ok(value) => value,
            Err(error) => {
                assert!(false, "unexpected error: {}", error);
                return;
            }
        };
        assert!(!execution.passed);
        assert!(execution.output.contains("main drift monitor blocked land"));
        assert_eq!(execution.gate_results.len(), 1);
        assert!(!execution.gate_results[0].passed);
    }

    #[test]
    fn given_no_main_drift_regression_when_execute_ship_gate_then_land_can_proceed() {
        let result = execute_ship_gate_with_gate_runner(|_gate| {
            Ok(GateEvidence {
                command: "moon run :cue-check".to_string(),
                passed: true,
                exit_code: 0,
                output: "main-drift-monitor: stable".to_string(),
                revision: Some("aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa".to_string()),
                current_revision: Some("aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa".to_string()),
            })
        });

        assert!(result.is_ok());
        let execution = match result {
            Ok(value) => value,
            Err(error) => {
                assert!(false, "unexpected error: {}", error);
                return;
            }
        };
        assert!(execution.passed);
    }

    #[test]
    fn given_cue_schema_failure_when_execute_ship_gate_then_routes_with_deterministic_failure() {
        let result = execute_ship_gate_with_gate_runner(|_gate| {
            Ok(GateEvidence {
                command: "moon run :cue-check".to_string(),
                passed: true,
                exit_code: 0,
                output: "cue_schema_failure artifact=queue definition=#QueueArtifact path=.orchestrator/queue_artifact.json\ninvalid value".to_string(),
                revision: Some("aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa".to_string()),
                current_revision: Some("aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa".to_string()),
            })
        });

        assert!(result.is_ok());
        let execution = match result {
            Ok(value) => value,
            Err(error) => {
                assert!(false, "unexpected error: {}", error);
                return;
            }
        };
        assert!(!execution.passed);
        assert_eq!(execution.failure_category, Some(FailureCategory::OutputParseFailure));
        assert_eq!(execution.next_stage, Some(Stage::Implementation));
        assert!(execution.output.contains("cue schema monitor blocked land"));
        assert_eq!(execution.gate_results.len(), 1);
        assert!(!execution.gate_results[0].passed);
    }
}
