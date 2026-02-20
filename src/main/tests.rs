use super::*;
use crate::pipeline::MergeQueuePolicy;
use chrono::Datelike;

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
    let result = execute_ship_gate_with_gate_runner(MergeQueuePolicy::Enforce, |gate| {
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
