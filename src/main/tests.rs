use super::*;
use crate::pipeline::MergeQueuePolicy;
use chrono::Datelike;
use oya::types::Gate;

#[test]
fn test_parse_rfc3339_deterministic_valid_input() {
    let result = parse_rfc3339_deterministic("2024-01-15T10:30:00Z");
    assert_eq!(result.year(), 2024);
    assert_eq!(result.month(), 1);
    assert_eq!(result.day(), 15);
}

#[test]
fn test_parse_rfc3339_deterministic_with_timezone() {
    let result = parse_rfc3339_deterministic("2024-01-15T10:30:00+05:00");
    assert_eq!(result.year(), 2024);
}

#[test]
fn test_parse_rfc3339_deterministic_invalid_returns_epoch() {
    let result = parse_rfc3339_deterministic("invalid-timestamp");
    assert_eq!(result, chrono::DateTime::UNIX_EPOCH);
}

#[test]
fn test_parse_rfc3339_deterministic_empty_returns_epoch() {
    let result = parse_rfc3339_deterministic("");
    assert_eq!(result, chrono::DateTime::UNIX_EPOCH);
}

#[test]
fn test_parse_rfc3339_deterministic_malformed_returns_epoch() {
    let result = parse_rfc3339_deterministic("2024-13-45T99:99:99Z");
    assert_eq!(result, chrono::DateTime::UNIX_EPOCH);
}

#[test]
fn test_parse_rfc3339_deterministic_is_deterministic() {
    let result1 = parse_rfc3339_deterministic("garbage");
    let result2 = parse_rfc3339_deterministic("garbage");
    assert_eq!(result1, result2);
    assert_eq!(result1, chrono::DateTime::UNIX_EPOCH);
}

#[test]
fn test_parse_command_parts_moon_command() {
    let parsed = parse_command_parts("moon run :check");
    assert!(parsed.is_ok());

    let command = parsed.map_or_else(
        |_| ParsedCommandParts { program: String::new(), args: Vec::new() },
        std::convert::identity,
    );
    assert_eq!(command.program, "moon");
    assert_eq!(command.args, vec!["run".to_string(), ":check".to_string()]);
}

#[test]
fn test_parse_command_parts_rejects_empty_command() {
    let parsed = parse_command_parts("   ");
    assert!(parsed.is_err());
}

#[test]
fn test_parse_gate_command_accepts_moon_gate_with_passthrough_args() {
    let parsed = parse_gate_command("moon run :test -- --test-threads=1");
    assert!(parsed.is_ok());

    match parsed {
        Ok(GateCommand::Moon { task: MoonTask::Test, passthrough }) => {
            assert_eq!(passthrough, vec!["--".to_string(), "--test-threads=1".to_string()]);
        }
        Ok(_) => panic!("expected moon test gate command"),
        Err(_) => panic!("expected successful gate command parse"),
    }
}

#[test]
fn test_parse_gate_command_rejects_unknown_program() {
    let parsed = parse_gate_command("cargo check");
    assert!(parsed.is_err());
}

#[test]
fn test_parse_gate_command_rejects_unknown_moon_task() {
    let parsed = parse_gate_command("moon run :quick");
    assert!(parsed.is_err());
}

#[test]
fn test_parse_gate_command_accepts_zjj_sync_status() {
    let parsed = parse_gate_command("zjj sync --status");
    assert!(matches!(parsed, Ok(GateCommand::ZjjSyncStatus)));
}

#[test]
fn test_gate_failure_outcome_gpt_review_clippy_routes_to_implementation() {
    let outcome = gate_failure_outcome(&Stage::GptReview, &Gate::ClippyClean);
    assert_eq!(outcome, (FailureCategory::LintFailed, Stage::Implementation));
}

#[test]
fn test_gate_failure_outcome_shipgate_merge_conflict_routes_to_review() {
    let outcome = gate_failure_outcome(&Stage::ShipGate, &Gate::ZjjMergeQueue);
    assert_eq!(outcome, (FailureCategory::MergeConflict, Stage::GptReview));
}

#[test]
fn test_gate_failure_outcome_acceptance_tests_are_red() {
    let outcome = gate_failure_outcome(&Stage::AcceptanceTest, &Gate::AcceptanceTestsAreRed);
    assert_eq!(outcome, (FailureCategory::TestsUnexpectedlyGreen, Stage::AcceptanceTest));
}

#[test]
fn test_execute_ship_gate_skip_zjj_gate() {
    let result = execute_ship_gate_with_gate_runner(MergeQueuePolicy::Skip, |gate| {
        assert_eq!(gate, Gate::MoonCi);
        Ok(GateEvidence {
            command: "moon run :ci".to_string(),
            passed: true,
            exit_code: 0,
            output: "ci ok".to_string(),
        })
    })
    .expect("ship gate should pass when moon ci passes and zjj is skipped");

    assert!(result.passed);
    assert_eq!(result.next_stage, None);
}

#[test]
fn test_execute_ship_gate_zjj_failure_routes_to_review() {
    use std::cell::RefCell;

    let seen = RefCell::new(Vec::new());
    let result = execute_ship_gate_with_gate_runner(MergeQueuePolicy::Enforce, |gate: Gate| {
        seen.borrow_mut().push(gate.clone());
        match gate {
            Gate::MoonCi => Ok(GateEvidence {
                command: "moon run :ci".to_string(),
                passed: true,
                exit_code: 0,
                output: "ci ok".to_string(),
            }),
            Gate::ZjjMergeQueue => Ok(GateEvidence {
                command: "zjj sync --status".to_string(),
                passed: false,
                exit_code: 1,
                output: "queue blocked".to_string(),
            }),
            _ => Ok(GateEvidence {
                command: "unexpected".to_string(),
                passed: false,
                exit_code: 1,
                output: "unexpected".to_string(),
            }),
        }
    })
    .expect("ship gate execution should return a stage result");

    assert_eq!(seen.into_inner(), vec![Gate::MoonCi, Gate::ZjjMergeQueue]);
    assert!(!result.passed);
    assert_eq!(result.failure_category, Some(FailureCategory::MergeConflict));
    assert_eq!(result.next_stage, Some(Stage::GptReview));
}

#[test]
fn test_cli_tail_mode_parses_default_interval() {
    let mode = parse_cli_mode_from(["oya", "tail"]);
    assert_eq!(mode, CliMode::Tail(TailArgs { interval: 2, run_id: None }));
}

#[test]
fn test_cli_tail_mode_parses_interval_and_run_id() {
    let mode = parse_cli_mode_from(["oya", "tail", "--interval", "5", "run-123"]);
    assert_eq!(mode, CliMode::Tail(TailArgs { interval: 5, run_id: Some("run-123".to_string()) }));
}

#[test]
fn test_cli_init_mode_parses() {
    let mode = parse_cli_mode_from(["oya", "init"]);
    assert_eq!(mode, CliMode::Init);
}

#[test]
fn test_cli_up_alias_parses_as_init() {
    let mode = parse_cli_mode_from(["oya", "up"]);
    assert_eq!(mode, CliMode::Init);
}

#[test]
fn test_tests_unexpectedly_green_maps_to_retry_loop() {
    let mut state =
        test_pipeline_state(FailureCategory::TestsUnexpectedlyGreen, Stage::AcceptanceTest, 1);
    assert!(should_retry_after_failure(&state));

    state.attempt = 2;
    assert!(!should_retry_after_failure(&state));
}

#[test]
fn test_infra_failed_is_non_retryable() {
    let state = test_pipeline_state(FailureCategory::TestInfraFailed, Stage::AcceptanceTest, 1);
    assert!(!should_retry_after_failure(&state));
}

fn test_pipeline_state(category: FailureCategory, stage: Stage, attempt: u32) -> PipelineState {
    PipelineState {
        current_stage: stage.clone(),
        attempt,
        last_failure: Some(StageFailure {
            category: category.clone(),
            message: "failure".to_string(),
            retryable: oya::is_retryable_failure(&category),
            failed_at: "2026-02-20T00:00:00Z".to_string(),
        }),
        orchestrator: OrchestratorState {
            status: "running".to_string(),
            stage: stage.as_str().to_string(),
            attempt,
            bead_id: "bead".to_string(),
            context: "ctx".to_string(),
            model: "model".to_string(),
            last_failure: String::new(),
            last_output: String::new(),
            last_prompt: String::new(),
            updated_at: "2026-02-20T00:00:00Z".to_string(),
        },
    }
}
