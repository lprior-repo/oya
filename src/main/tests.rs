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
fn test_gate_failure_outcome_shipgate_merge_conflict_routes_to_implementation() {
    let outcome = gate_failure_outcome(&Stage::ShipGate, &Gate::ZjjMergeQueue);
    assert_eq!(outcome, (FailureCategory::MergeConflict, Stage::Implementation));
}

#[test]
fn test_gate_failure_outcome_shipgate_moon_ci_returns_ci_failed() {
    let outcome = gate_failure_outcome(&Stage::ShipGate, &Gate::MoonCi);
    assert_eq!(
        outcome,
        (FailureCategory::CiFailed, Stage::Implementation),
        "ShipGate MoonCi failure should return CiFailed category routing to Implementation"
    );
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

/// Given: ShipGate executes with MoonCi passing
/// When: Skip policy is active (zjj skipped)
/// Then: gate_results should contain moon_ci check
#[test]
fn test_execute_ship_gate_records_moon_ci_check_in_gate_results() {
    let result = execute_ship_gate_with_gate_runner(MergeQueuePolicy::Skip, |gate| {
        assert_eq!(gate, Gate::MoonCi);
        Ok(GateEvidence {
            command: "moon run :ci".to_string(),
            passed: true,
            exit_code: 0,
            output: "ci ok".to_string(),
        })
    })
    .expect("ship gate should pass");

    assert!(
        result.gate_results.iter().any(|gr| gr.gate == "moon_ci"),
        "gate_results should contain moon_ci check"
    );
}

/// Given: ShipGate executes both gates with Enforce policy
/// When: Both gates pass
/// Then: gate_results should contain both moon_ci and zjj_merge_queue checks
#[test]
fn test_execute_ship_gate_records_zjj_check_in_gate_results() {
    let result =
        execute_ship_gate_with_gate_runner(MergeQueuePolicy::Enforce, |gate: Gate| match gate {
            Gate::MoonCi => Ok(GateEvidence {
                command: "moon run :ci".to_string(),
                passed: true,
                exit_code: 0,
                output: "ci ok".to_string(),
            }),
            Gate::ZjjMergeQueue => Ok(GateEvidence {
                command: "zjj sync --status".to_string(),
                passed: true,
                exit_code: 0,
                output: "queue ready".to_string(),
            }),
            _ => Ok(GateEvidence {
                command: "unexpected".to_string(),
                passed: false,
                exit_code: 1,
                output: "unexpected".to_string(),
            }),
        })
        .expect("ship gate should pass when both gates pass");

    assert!(
        result.gate_results.iter().any(|gr| gr.gate == "zjj_merge_queue"),
        "gate_results should contain zjj_merge_queue check"
    );
    assert!(
        result.gate_results.iter().any(|gr| gr.gate == "moon_ci"),
        "gate_results should contain moon_ci check"
    );
    assert_eq!(result.gate_results.len(), 2, "gate_results should have exactly 2 checks");
}

/// Given: ShipGate gate_results
/// When: Comparing expected stage checks against actual gate_results
/// Then: Should be able to detect missing gates
#[test]
fn test_ship_gate_missing_gate_in_metadata_detected() {
    let result = execute_ship_gate_with_gate_runner(MergeQueuePolicy::Skip, |gate| {
        assert_eq!(gate, Gate::MoonCi);
        Ok(GateEvidence {
            command: "moon run :ci".to_string(),
            passed: true,
            exit_code: 0,
            output: "ci ok".to_string(),
        })
    })
    .expect("ship gate should pass");

    // Expected checks for ShipGate with Enforce policy
    let expected_gates = Stage::ShipGate.gates();
    let expected_gate_names: Vec<&str> = expected_gates.iter().map(|g| g.as_str()).collect();

    // Actual gates in gate_results
    let actual_gate_names: Vec<&str> =
        result.gate_results.iter().map(|gr| gr.gate.as_str()).collect();

    // Detect missing gates - zjj_merge_queue is missing when Skip policy is used
    let missing_gates: Vec<&&str> = expected_gate_names
        .iter()
        .filter(|expected| !actual_gate_names.contains(expected))
        .collect();

    assert_eq!(missing_gates.len(), 1, "Should detect one missing gate");
    assert_eq!(*missing_gates[0], "zjj_merge_queue", "Missing gate should be zjj_merge_queue");
}

/// Given: ShipGate with inconsistent checks
/// When: Expected checks don't match actual gate_results
/// Then: Inconsistency should be detectable
#[test]
fn test_ship_gate_inconsistency_between_checks_and_metadata_reported() {
    let result =
        execute_ship_gate_with_gate_runner(MergeQueuePolicy::Enforce, |gate: Gate| match gate {
            Gate::MoonCi => Ok(GateEvidence {
                command: "moon run :ci".to_string(),
                passed: true,
                exit_code: 0,
                output: "ci ok".to_string(),
            }),
            Gate::ZjjMergeQueue => Ok(GateEvidence {
                command: "zjj sync --status".to_string(),
                passed: true,
                exit_code: 0,
                output: "queue ready".to_string(),
            }),
            _ => Ok(GateEvidence {
                command: "unexpected".to_string(),
                passed: false,
                exit_code: 1,
                output: "unexpected".to_string(),
            }),
        })
        .expect("ship gate should pass");

    // Verify consistency: expected gates match actual gate_results
    let expected_gates = Stage::ShipGate.gates();
    let expected_gate_names: Vec<&str> = expected_gates.iter().map(|g| g.as_str()).collect();
    let actual_gate_names: Vec<&str> =
        result.gate_results.iter().map(|gr| gr.gate.as_str()).collect();

    // When Enforce policy is used, all expected gates should be present
    let all_expected_present =
        expected_gate_names.iter().all(|expected| actual_gate_names.contains(expected));

    assert!(all_expected_present, "All expected gates should be in gate_results");
    assert_eq!(
        expected_gate_names.len(),
        actual_gate_names.len(),
        "Expected and actual gate counts should match"
    );
}

#[test]
fn test_execute_ship_gate_zjj_failure_routes_to_implementation() {
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
    assert_eq!(result.next_stage, Some(Stage::Implementation));
}

/// Given: ShipGate executes with MoonCi failing
/// When: MoonCi gate returns passed=false
/// Then: failure_category should be CiFailed (not TestFailed)
/// Bug fix: Previously returned TestFailed which is semantically incorrect
/// since moon run :ci can fail for compile/lint/security/test reasons
#[test]
fn test_execute_ship_gate_moon_ci_failure_returns_ci_failed_category() {
    let result =
        execute_ship_gate_with_gate_runner(MergeQueuePolicy::Skip, |gate: Gate| match gate {
            Gate::MoonCi => Ok(GateEvidence {
                command: "moon run :ci".to_string(),
                passed: false,
                exit_code: 1,
                output: "error: test failed".to_string(),
            }),
            _ => Ok(GateEvidence {
                command: "unexpected".to_string(),
                passed: false,
                exit_code: 1,
                output: "unexpected".to_string(),
            }),
        })
        .expect("ship gate execution should return a stage result");

    assert!(!result.passed, "MoonCi failure should result in passed=false");
    assert_eq!(
        result.failure_category,
        Some(FailureCategory::CiFailed),
        "MoonCi failure should return CiFailed category, not TestFailed"
    );
    assert_eq!(
        result.next_stage,
        Some(Stage::Implementation),
        "MoonCi failure should route to Implementation for fixes"
    );
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
        test_pipeline_state(FailureCategory::TestsUnexpectedlyGreen, Stage::Implementation, 1);
    assert!(should_retry_after_failure(&state));

    state.attempt = 2;
    assert!(!should_retry_after_failure(&state));
}

#[test]
fn test_infra_failed_is_non_retryable() {
    let state = test_pipeline_state(FailureCategory::TestInfraFailed, Stage::Implementation, 1);
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

// ============================================================================
// URL Validation Tests for OYA_OPENCODE_BASE_URL (src-3g9)
// ============================================================================

#[test]
fn test_is_valid_http_url_accepts_valid_http_url() {
    assert!(is_valid_http_url("http://127.0.0.1:4097"));
    assert!(is_valid_http_url("http://localhost:4097"));
    assert!(is_valid_http_url("http://example.com"));
}

#[test]
fn test_is_valid_http_url_accepts_valid_https_url() {
    assert!(is_valid_http_url("https://127.0.0.1:4097"));
    assert!(is_valid_http_url("https://api.example.com"));
}

#[test]
fn test_is_valid_http_url_accepts_trailing_slash() {
    // Trailing slash should be accepted for base URLs
    assert!(is_valid_http_url("http://127.0.0.1:4097/"));
    assert!(is_valid_http_url("https://api.example.com/"));
}

#[test]
fn test_is_valid_http_url_rejects_credentials() {
    // URLs with embedded credentials should be rejected
    assert!(!is_valid_http_url("http://user:pass@127.0.0.1:4097"));
    assert!(!is_valid_http_url("http://user@127.0.0.1:4097"));
    assert!(!is_valid_http_url("https://:pass@api.example.com"));
}

#[test]
fn test_is_valid_http_url_rejects_paths() {
    // Base URLs should not have paths (other than root)
    assert!(!is_valid_http_url("http://127.0.0.1:4097/api"));
    assert!(!is_valid_http_url("http://127.0.0.1:4097/v1/endpoint"));
    assert!(!is_valid_http_url("https://api.example.com/path/to/resource"));
}

#[test]
fn test_is_valid_http_url_rejects_query_strings() {
    assert!(!is_valid_http_url("http://127.0.0.1:4097?foo=bar"));
    assert!(!is_valid_http_url("https://api.example.com/?token=secret"));
}

#[test]
fn test_is_valid_http_url_rejects_fragments() {
    assert!(!is_valid_http_url("http://127.0.0.1:4097#anchor"));
    assert!(!is_valid_http_url("https://api.example.com/#section"));
}

#[test]
fn test_is_valid_http_url_rejects_invalid_schemes() {
    assert!(!is_valid_http_url("ftp://127.0.0.1:4097"));
    assert!(!is_valid_http_url("file:///etc/passwd"));
    assert!(!is_valid_http_url("javascript:alert(1)"));
}

#[test]
fn test_is_valid_http_url_rejects_empty_and_whitespace() {
    assert!(!is_valid_http_url(""));
    assert!(!is_valid_http_url("   "));
    assert!(!is_valid_http_url("\t\n"));
}

#[test]
fn test_is_valid_http_url_rejects_malformed_urls() {
    assert!(!is_valid_http_url("not-a-url"));
    assert!(!is_valid_http_url("http://"));
    assert!(!is_valid_http_url("://missing-scheme.com"));
    assert!(!is_valid_http_url("http://[invalid-ipv6"));
}

#[test]
fn test_is_valid_http_url_rejects_control_characters() {
    // URLs with embedded control characters should be rejected
    // Null byte is NOT whitespace and should cause rejection
    assert!(!is_valid_http_url("http://127.0.0.1:4097\x00"));
    // Escape character is NOT whitespace and should cause rejection
    assert!(!is_valid_http_url("http://127.0.0.1:4097\x1b"));
    // Control character in the middle of the URL
    assert!(!is_valid_http_url("http://127.0.0.1:4097/test\x00path"));
    // Backspace character
    assert!(!is_valid_http_url("http://127.0.0.1:4097\x08"));
}

// ============================================================================
// CLI Input Validation Tests for oversized inputs (src-1dr)
// ============================================================================

/// Maximum allowed length for bead_id input to prevent DoS/memory exhaustion
const MAX_BEAD_ID_LEN: usize = 128;

/// Maximum allowed length for context string input
const MAX_CONTEXT_LEN: usize = 4096;

/// Maximum allowed length for model name input
const MAX_MODEL_NAME_LEN: usize = 128;

/// Maximum allowed length for restate URL input
const MAX_RESTATE_URL_LEN: usize = 2048;

/// Maximum allowed timeout value in seconds (24 hours)
const MAX_TIMEOUT_SECS: u64 = 86_400;

/// Maximum allowed poll interval in seconds (1 hour)
const MAX_POLL_INTERVAL_SECS: u64 = 3600;

/// Validates CLI RunArgs for oversized inputs that could cause memory issues.
fn validate_run_args(args: &RunArgs) -> Result<(), String> {
    validate_bead_id(&args.bead_id)?;
    validate_context(&args.context)?;
    validate_restate_url(&args.restate_url)?;
    validate_timeout(args.timeout)?;
    args.model.as_ref().map_or(Ok(()), |model| validate_model(model))?;
    args.poll_interval.map_or(Ok(()), |interval| validate_poll_interval(interval))?;
    Ok(())
}

fn validate_bead_id(bead_id: &str) -> Result<(), String> {
    let trimmed = bead_id.trim();
    if trimmed.is_empty() {
        return Err("bead_id cannot be empty".to_string());
    }
    if trimmed.len() > MAX_BEAD_ID_LEN {
        return Err(format!(
            "bead_id exceeds maximum length: {} > {}",
            trimmed.len(),
            MAX_BEAD_ID_LEN
        ));
    }
    if contains_forbidden_control_chars(trimmed) {
        return Err("bead_id contains invalid control characters".to_string());
    }
    Ok(())
}

fn validate_context(context: &str) -> Result<(), String> {
    if context.len() > MAX_CONTEXT_LEN {
        return Err(format!(
            "context exceeds maximum length: {} > {}",
            context.len(),
            MAX_CONTEXT_LEN
        ));
    }
    if contains_forbidden_control_chars(context) {
        return Err("context contains invalid control characters".to_string());
    }
    Ok(())
}

fn validate_model(model: &str) -> Result<(), String> {
    let trimmed = model.trim();
    if trimmed.is_empty() {
        return Err("model cannot be empty when provided".to_string());
    }
    if trimmed.len() > MAX_MODEL_NAME_LEN {
        return Err(format!(
            "model exceeds maximum length: {} > {}",
            trimmed.len(),
            MAX_MODEL_NAME_LEN
        ));
    }
    if contains_forbidden_control_chars(trimmed) {
        return Err("model contains invalid control characters".to_string());
    }
    Ok(())
}

fn validate_restate_url(url: &str) -> Result<(), String> {
    let trimmed = url.trim();
    if trimmed.is_empty() {
        return Err("restate_url cannot be empty".to_string());
    }
    if trimmed.len() > MAX_RESTATE_URL_LEN {
        return Err(format!(
            "restate_url exceeds maximum length: {} > {}",
            trimmed.len(),
            MAX_RESTATE_URL_LEN
        ));
    }
    if contains_forbidden_control_chars(trimmed) {
        return Err("restate_url contains invalid control characters".to_string());
    }
    if !is_valid_http_url(trimmed) {
        return Err(format!(
            "restate_url is not a valid HTTP/HTTPS URL: {}",
            sanitize_url_for_logging(trimmed)
        ));
    }
    Ok(())
}

fn validate_timeout(timeout: u64) -> Result<(), String> {
    if timeout == 0 {
        return Err("timeout must be greater than 0".to_string());
    }
    if timeout > MAX_TIMEOUT_SECS {
        return Err(format!(
            "timeout exceeds maximum value: {} > {} seconds",
            timeout, MAX_TIMEOUT_SECS
        ));
    }
    Ok(())
}

fn validate_poll_interval(interval: u64) -> Result<(), String> {
    if interval == 0 {
        return Err("poll_interval must be greater than 0".to_string());
    }
    if interval > MAX_POLL_INTERVAL_SECS {
        return Err(format!(
            "poll_interval exceeds maximum value: {} > {} seconds",
            interval, MAX_POLL_INTERVAL_SECS
        ));
    }
    Ok(())
}

#[test]
fn test_validate_run_args_accepts_valid_input() {
    let args = RunArgs {
        bead_id: "src-abc123".to_string(),
        restate_url: "http://127.0.0.1:8080".to_string(),
        context: "test context".to_string(),
        timeout: 3600,
        poll_interval: Some(5),
        model: Some("claude-3-opus".to_string()),
    };
    assert!(validate_run_args(&args).is_ok());
}

#[test]
fn test_validate_run_args_accepts_minimal_input() {
    let args = RunArgs {
        bead_id: "x".to_string(),
        restate_url: "http://localhost".to_string(),
        context: String::new(),
        timeout: 1,
        poll_interval: None,
        model: None,
    };
    assert!(validate_run_args(&args).is_ok());
}

#[test]
fn test_validate_bead_id_rejects_oversized_input() {
    let oversized = "x".repeat(129);
    let result = validate_bead_id(&oversized);
    assert!(result.is_err());
    let error = result.err();
    assert!(error.is_some());
    let error_msg = error.unwrap();
    assert!(error_msg.contains("exceeds maximum length"));
    assert!(error_msg.contains("129"));
    assert!(error_msg.contains("128"));
}

#[test]
fn test_validate_bead_id_rejects_empty_input() {
    let result = validate_bead_id("");
    assert!(result.is_err());
    let error = result.err();
    assert!(error.is_some());
    assert!(error.unwrap().contains("cannot be empty"));
}

#[test]
fn test_validate_bead_id_rejects_whitespace_only_input() {
    let result = validate_bead_id("   ");
    assert!(result.is_err());
    let error = result.err();
    assert!(error.is_some());
    assert!(error.unwrap().contains("cannot be empty"));
}

#[test]
fn test_validate_bead_id_rejects_control_characters() {
    let result = validate_bead_id("test\x00bead");
    assert!(result.is_err());
    let error = result.err();
    assert!(error.is_some());
    assert!(error.unwrap().contains("invalid control characters"));
}

#[test]
fn test_validate_bead_id_accepts_at_max_length() {
    let max_len = "x".repeat(128);
    let result = validate_bead_id(&max_len);
    assert!(result.is_ok());
}

#[test]
fn test_validate_context_rejects_oversized_input() {
    let oversized = "x".repeat(4097);
    let result = validate_context(&oversized);
    assert!(result.is_err());
    let error = result.err();
    assert!(error.is_some());
    let error_msg = error.unwrap();
    assert!(error_msg.contains("exceeds maximum length"));
    assert!(error_msg.contains("4097"));
    assert!(error_msg.contains("4096"));
}

#[test]
fn test_validate_context_accepts_empty_input() {
    let result = validate_context("");
    assert!(result.is_ok());
}

#[test]
fn test_validate_context_rejects_control_characters() {
    let result = validate_context("test\x1bcontext");
    assert!(result.is_err());
    let error = result.err();
    assert!(error.is_some());
    assert!(error.unwrap().contains("invalid control characters"));
}

#[test]
fn test_validate_context_accepts_at_max_length() {
    let max_len = "x".repeat(4096);
    let result = validate_context(&max_len);
    assert!(result.is_ok());
}

#[test]
fn test_validate_context_accepts_newlines_and_tabs() {
    // Newlines, carriage returns, and tabs are allowed in context
    let result = validate_context("line1\nline2\ttab\r\nwindows");
    assert!(result.is_ok());
}

#[test]
fn test_validate_model_rejects_oversized_input() {
    let oversized = "x".repeat(129);
    let result = validate_model(&oversized);
    assert!(result.is_err());
    let error = result.err();
    assert!(error.is_some());
    let error_msg = error.unwrap();
    assert!(error_msg.contains("exceeds maximum length"));
    assert!(error_msg.contains("129"));
    assert!(error_msg.contains("128"));
}

#[test]
fn test_validate_model_rejects_empty_input() {
    let result = validate_model("");
    assert!(result.is_err());
    let error = result.err();
    assert!(error.is_some());
    assert!(error.unwrap().contains("cannot be empty"));
}

#[test]
fn test_validate_model_rejects_whitespace_only_input() {
    let result = validate_model("   ");
    assert!(result.is_err());
    let error = result.err();
    assert!(error.is_some());
    assert!(error.unwrap().contains("cannot be empty"));
}

#[test]
fn test_validate_model_rejects_control_characters() {
    let result = validate_model("model\x00name");
    assert!(result.is_err());
    let error = result.err();
    assert!(error.is_some());
    assert!(error.unwrap().contains("invalid control characters"));
}

#[test]
fn test_validate_model_accepts_at_max_length() {
    let max_len = "x".repeat(128);
    let result = validate_model(&max_len);
    assert!(result.is_ok());
}

#[test]
fn test_validate_restate_url_rejects_oversized_input() {
    // Use a valid URL structure that is longer than MAX_RESTATE_URL_LEN
    // Need to make sure it's still a valid URL format but just too long
    let long_host = "a".repeat(2100); // This will make the URL longer than 2048
    let oversized = format!("http://{}.example.com", long_host);
    let result = validate_restate_url(&oversized);
    assert!(result.is_err());
    let error = result.err();
    assert!(error.is_some());
    let error_msg = error.unwrap();
    assert!(
        error_msg.contains("exceeds maximum length") || error_msg.contains("not a valid"),
        "Error should mention length or validity: {}",
        error_msg
    );
}

#[test]
fn test_validate_restate_url_rejects_empty_input() {
    let result = validate_restate_url("");
    assert!(result.is_err());
    let error = result.err();
    assert!(error.is_some());
    assert!(error.unwrap().contains("cannot be empty"));
}

#[test]
fn test_validate_restate_url_rejects_invalid_url_format() {
    let result = validate_restate_url("not-a-url");
    assert!(result.is_err());
    let error = result.err();
    assert!(error.is_some());
    assert!(error.unwrap().contains("not a valid HTTP/HTTPS URL"));
}

#[test]
fn test_validate_restate_url_rejects_control_characters() {
    let result = validate_restate_url("http://127.0.0.1:8080\x00");
    assert!(result.is_err());
    let error = result.err();
    assert!(error.is_some());
    assert!(error.unwrap().contains("invalid control characters"));
}

// ---------------------------------------------------------------------------
// src-1rf: Tests for URL credential sanitization in error messages
// ---------------------------------------------------------------------------

#[test]
fn test_validate_restate_url_sanitizes_credentials_in_error() {
    // URL with embedded credentials should have them masked in error output
    // Note: This URL will fail validation because it has credentials, but
    // the error message should NOT reveal the password
    let result = validate_restate_url("https://user:secret@example.com");
    assert!(result.is_err());
    let error_msg = result.err().unwrap_or_default();
    // The error should show the sanitized URL, not the original with credentials
    assert!(
        !error_msg.contains("secret"),
        "Error message should not contain password: {}",
        error_msg
    );
    assert!(
        error_msg.contains("***"),
        "Error message should contain sanitized credentials: {}",
        error_msg
    );
}

#[test]
fn test_validate_restate_url_sanitizes_username_only_in_error() {
    // URL with only username (no password) should also be sanitized
    let result = validate_restate_url("https://admin@example.com");
    assert!(result.is_err());
    let error_msg = result.err().unwrap_or_default();
    assert!(
        !error_msg.contains("admin"),
        "Error message should not contain username: {}",
        error_msg
    );
}

#[test]
fn test_validate_timeout_rejects_zero() {
    let result = validate_timeout(0);
    assert!(result.is_err());
    let error = result.err();
    assert!(error.is_some());
    assert!(error.unwrap().contains("must be greater than 0"));
}

#[test]
fn test_validate_timeout_rejects_oversized_input() {
    let result = validate_timeout(86_401);
    assert!(result.is_err());
    let error = result.err();
    assert!(error.is_some());
    let error_msg = error.unwrap();
    assert!(error_msg.contains("exceeds maximum value"));
    assert!(error_msg.contains("86401"));
    assert!(error_msg.contains("86400"));
}

#[test]
fn test_validate_timeout_accepts_at_max_value() {
    let result = validate_timeout(86_400);
    assert!(result.is_ok());
}

#[test]
fn test_validate_timeout_accepts_one_second() {
    let result = validate_timeout(1);
    assert!(result.is_ok());
}

#[test]
fn test_validate_poll_interval_rejects_zero() {
    let result = validate_poll_interval(0);
    assert!(result.is_err());
    let error = result.err();
    assert!(error.is_some());
    assert!(error.unwrap().contains("must be greater than 0"));
}

#[test]
fn test_validate_poll_interval_rejects_oversized_input() {
    let result = validate_poll_interval(3601);
    assert!(result.is_err());
    let error = result.err();
    assert!(error.is_some());
    let error_msg = error.unwrap();
    assert!(error_msg.contains("exceeds maximum value"));
    assert!(error_msg.contains("3601"));
    assert!(error_msg.contains("3600"));
}

#[test]
fn test_validate_poll_interval_accepts_at_max_value() {
    let result = validate_poll_interval(3600);
    assert!(result.is_ok());
}

#[test]
fn test_validate_poll_interval_accepts_one_second() {
    let result = validate_poll_interval(1);
    assert!(result.is_ok());
}

/// Security test: oversized bead_id should be rejected before any processing
#[test]
fn test_validate_run_args_rejects_oversized_bead_id_before_processing() {
    let oversized = "a".repeat(10_000_000); // 10MB of data
    let args = RunArgs {
        bead_id: oversized.clone(),
        restate_url: "http://127.0.0.1:8080".to_string(),
        context: String::new(),
        timeout: 3600,
        poll_interval: None,
        model: None,
    };
    let result = validate_run_args(&args);
    assert!(result.is_err());
    let error = result.err();
    assert!(error.is_some());
    let error_msg = error.unwrap();
    assert!(error_msg.contains("exceeds maximum length"));
    // Ensure we didn't try to process the full 10MB
    assert!(error_msg.contains("10000000"));
}

/// Security test: oversized context should be rejected before any processing
#[test]
fn test_validate_run_args_rejects_oversized_context_before_processing() {
    let oversized = "a".repeat(100_000_000); // 100MB of data
    let args = RunArgs {
        bead_id: "valid-id".to_string(),
        restate_url: "http://127.0.0.1:8080".to_string(),
        context: oversized.clone(),
        timeout: 3600,
        poll_interval: None,
        model: None,
    };
    let result = validate_run_args(&args);
    assert!(result.is_err());
    let error = result.err();
    assert!(error.is_some());
    let error_msg = error.unwrap();
    assert!(error_msg.contains("exceeds maximum length"));
}

// ============================================================================
// TailArgs Validation Tests (src-1dr)
// ============================================================================

#[test]
fn test_validate_tail_args_accepts_valid_input() {
    let args = TailArgs { interval: 5, run_id: Some("run-123".to_string()) };
    assert!(validate_tail_args(&args).is_ok());
}

#[test]
fn test_validate_tail_args_accepts_minimal_input() {
    let args = TailArgs { interval: 1, run_id: None };
    assert!(validate_tail_args(&args).is_ok());
}

#[test]
fn test_validate_interval_rejects_zero() {
    let result = validate_interval(0);
    assert!(result.is_err());
    let error = result.err();
    assert!(error.is_some());
    assert!(error.unwrap().contains("must be greater than 0"));
}

#[test]
fn test_validate_interval_rejects_oversized_input() {
    let result = validate_interval(3601);
    assert!(result.is_err());
    let error = result.err();
    assert!(error.is_some());
    let error_msg = error.unwrap();
    assert!(error_msg.contains("exceeds maximum value"));
}

#[test]
fn test_validate_interval_accepts_at_max_value() {
    let result = validate_interval(3600);
    assert!(result.is_ok());
}

#[test]
fn test_validate_run_id_rejects_oversized_input() {
    let oversized = "x".repeat(129);
    let result = validate_run_id(&oversized);
    assert!(result.is_err());
    let error = result.err();
    assert!(error.is_some());
    let error_msg = error.unwrap();
    assert!(error_msg.contains("exceeds maximum length"));
}

#[test]
fn test_validate_run_id_rejects_empty_input() {
    let result = validate_run_id("");
    assert!(result.is_err());
    let error = result.err();
    assert!(error.is_some());
    assert!(error.unwrap().contains("cannot be empty"));
}

#[test]
fn test_validate_run_id_rejects_whitespace_only_input() {
    let result = validate_run_id("   ");
    assert!(result.is_err());
    let error = result.err();
    assert!(error.is_some());
    assert!(error.unwrap().contains("cannot be empty"));
}

#[test]
fn test_validate_run_id_rejects_control_characters() {
    let result = validate_run_id("run\x00123");
    assert!(result.is_err());
    let error = result.err();
    assert!(error.is_some());
    assert!(error.unwrap().contains("invalid control characters"));
}

#[test]
fn test_validate_run_id_accepts_at_max_length() {
    let max_len = "x".repeat(128);
    let result = validate_run_id(&max_len);
    assert!(result.is_ok());
}
