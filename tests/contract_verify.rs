//! Contract verification tests
//!
//! These tests verify that our fakes/mock expectations match real tool behavior.
//! Run occasionally to detect when tools change.
//!
//! Run with: `cargo test --test contract_verify -- --ignored`

use std::process::Command;

/// Verify moon check exit codes match our expectations
#[test]
#[ignore = "runs real moon - may be slow"]
fn verify_moon_check_exit_codes() {
    // Success case - should return 0
    let output =
        Command::new("moon").args(["run", ":check"]).output().expect("moon should be installed");

    // Just verify exit code is 0 or 1 (not something unexpected)
    let exit_code = output.status.code().unwrap_or(-1);
    assert!(exit_code == 0 || exit_code == 1, "moon check should return 0 or 1, got {}", exit_code);

    // Verify output contains expected text
    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);
    let combined = format!("{} {}", stdout, stderr);

    // Moon outputs various status messages
    assert!(
        combined.contains("check")
            || combined.contains("Check")
            || combined.contains("error")
            || exit_code == 0,
        "moon check output should contain expected keywords"
    );
}

/// Verify moon test exit codes
#[test]
#[ignore = "runs real moon - may be slow"]
fn verify_moon_test_exit_codes() {
    let output =
        Command::new("moon").args(["run", ":test"]).output().expect("moon should be installed");

    let exit_code = output.status.code().unwrap_or(-1);
    assert!(exit_code == 0 || exit_code == 1, "moon test should return 0 or 1, got {}", exit_code);
}

/// Verify zjj exit codes match our expectations
#[test]
#[ignore = "runs real zjj"]
fn verify_zjj_exit_codes() {
    // Test zjj status (should succeed)
    let output = Command::new("zjj").arg("status").output();

    if let Ok(output) = output {
        let exit_code = output.status.code().unwrap_or(-1);
        assert!(
            exit_code == 0 || exit_code == 1,
            "zjj status should return 0 or 1, got {}",
            exit_code
        );
    } else {
        println!("zjj not installed, skipping");
    }
}

/// Verify opencode is available and returns JSON
#[test]
#[ignore = "runs real opencode - requires API access"]
fn verify_opencode_json_format() {
    let output = Command::new("opencode").args(["run", "--format", "json", "echo hello"]).output();

    if let Ok(output) = output {
        let stdout = String::from_utf8_lossy(&output.stdout);

        // Verify it's valid JSON
        let parsed: serde_json::Value =
            serde_json::from_str(&stdout).expect("opencode should return valid JSON");

        // Verify it has stdout field
        assert!(parsed.get("stdout").is_some(), "opencode JSON should have 'stdout' field");
    } else {
        println!("opencode not available, skipping");
    }
}

/// Contract: Workspace name format
#[test]
fn contract_workspace_name_format() {
    use oya::build_zjj_workspace_name;

    // Valid inputs should produce names matching pattern
    let name = build_zjj_workspace_name("run-123", "research", 1).unwrap();

    // Should start with 'oya-'
    assert!(name.starts_with("oya-"));

    // Should contain run_id, stage, attempt
    assert!(name.contains("run-123"));
    assert!(name.contains("research"));
    assert!(name.contains("a1"));
}

/// Contract: Stage ordering is preserved
#[test]
fn contract_stage_ordering() {
    use oya::types::StageName;

    let expected_order = vec![
        StageName::Research,
        StageName::Plan,
        StageName::Contract,
        StageName::Tdd15,
        StageName::Qa,
        StageName::RedQueen,
        StageName::GptReview,
        StageName::ShipGate,
    ];

    // Verify each stage transitions to the next
    for window in expected_order.windows(2) {
        let current = &window[0];
        let expected_next = &window[1];

        assert_eq!(
            current.next(),
            Some(expected_next.clone()),
            "{:?} should transition to {:?}",
            current,
            expected_next
        );
    }

    // Verify ShipGate is terminal
    assert_eq!(StageName::ShipGate.next(), None);
}

/// Contract: Gate definitions
#[test]
fn contract_gate_definitions() {
    use oya::types::{Gate, StageName};

    // Research, Plan, Contract only need to compile
    for stage in [StageName::Research, StageName::Plan, StageName::Contract] {
        let gates = stage.gates();
        assert_eq!(gates.len(), 1, "{:?} should have 1 gate", stage);
        assert_eq!(gates[0], Gate::Compiles);
    }

    // Tdd15 needs compile + tests
    let tdd15_gates = StageName::Tdd15.gates();
    assert!(tdd15_gates.contains(&Gate::Compiles));
    assert!(tdd15_gates.contains(&Gate::TestsPass));

    // ShipGate has the most gates
    let ship_gates = StageName::ShipGate.gates();
    assert_eq!(ship_gates.len(), 2);
    assert!(ship_gates.contains(&Gate::MoonCi));
    assert!(ship_gates.contains(&Gate::ZjjMergeQueue));
}

/// Contract: Failure categories are retryable/non-retryable
#[test]
fn contract_failure_category_retryability() {
    use oya::is_retryable_failure;
    use oya::types::FailureCategory;

    // Retryable failures
    let retryable = vec![
        FailureCategory::TestFailed,
        FailureCategory::LintFailed,
        FailureCategory::OutputParseFailure,
    ];

    for category in retryable {
        assert!(is_retryable_failure(&category), "{:?} should be retryable", category);
    }

    // Non-retryable failures (examples)
    let non_retryable = vec![
        FailureCategory::AuthFailed,
        FailureCategory::RateLimited,
        FailureCategory::ProviderUnavailable,
    ];

    for category in non_retryable {
        assert!(!is_retryable_failure(&category), "{:?} should NOT be retryable", category);
    }
}

/// Contract: Max attempts is consistent
#[test]
fn contract_max_attempts() {
    use oya::types::StageName;

    for stage in [
        StageName::Research,
        StageName::Plan,
        StageName::Contract,
        StageName::Tdd15,
        StageName::Qa,
        StageName::RedQueen,
        StageName::GptReview,
        StageName::ShipGate,
    ] {
        assert_eq!(stage.max_attempts(), 3, "{:?} should have 3 max attempts", stage);
    }
}
