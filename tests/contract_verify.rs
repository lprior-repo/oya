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
    // Use 128 as fallback for signal-terminated processes (Unix convention)
    let exit_code = output.status.code().unwrap_or(128);
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

    let exit_code = output.status.code().unwrap_or(128);
    assert!(exit_code == 0 || exit_code == 1, "moon test should return 0 or 1, got {}", exit_code);
}

/// Verify zjj exit codes match our expectations
#[test]
#[ignore = "runs real zjj"]
fn verify_zjj_exit_codes() {
    // Test zjj status (should succeed)
    let output = Command::new("zjj").arg("status").output();

    if let Ok(output) = output {
        let exit_code = output.status.code().unwrap_or(128);
        assert!(
            exit_code == 0 || exit_code == 1,
            "zjj status should return 0 or 1, got {}",
            exit_code
        );
    } else {
        println!("zjj not installed, skipping");
    }
}

/// Verify opencode is available and returns JSONL (JSON Lines)
#[test]
#[ignore = "runs real opencode - requires API access"]
fn verify_opencode_json_format() {
    let output = Command::new("opencode").args(["run", "--format", "json", "echo hello"]).output();

    if let Ok(output) = output {
        let stdout = String::from_utf8_lossy(&output.stdout);

        // Openencode returns JSONL (JSON Lines) - one JSON object per line
        // Parse each line and verify it's valid JSON
        let mut found_content = false;
        for line in stdout.lines() {
            if line.trim().is_empty() {
                continue;
            }
            let parsed: serde_json::Value = serde_json::from_str(line).unwrap_or_else(|e| {
                panic!(
                    "opencode should return valid JSONL, but line '{}' failed to parse: {}",
                    line, e
                )
            });

            // Check for stdout or content field (different event types have different fields)
            if parsed.get("stdout").is_some() || parsed.get("content").is_some() {
                found_content = true;
            }
        }

        assert!(found_content, "opencode JSONL should contain events with stdout or content field");
    } else {
        println!("opencode not available, skipping");
    }
}

/// Contract: Workspace name format
#[test]
fn contract_workspace_name_format() {
    use oya::build_zjj_workspace_name;

    // Valid inputs should produce names matching pattern
    let name = build_zjj_workspace_name("run-123", "plan", 1).unwrap();

    // Should start with 'oya-'
    assert!(name.starts_with("oya-"));

    // Should contain run_id, stage, attempt
    assert!(name.contains("run-123"));
    assert!(name.contains("plan"));
    assert!(name.contains("a1"));
}

/// Contract: Stage ordering is preserved
#[test]
fn contract_stage_ordering() {
    use oya::types::StageName;

    // Staged flow: Explore -> Contract -> Red -> Implementation -> Witness -> ShipGate.
    let expected_order = vec![
        StageName::Explore,
        StageName::Contract,
        StageName::Red,
        StageName::Implementation,
        StageName::Witness,
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

    // Verify implementation transitions through witness before close.
    assert_eq!(StageName::Implementation.next(), Some(StageName::Witness));
}

/// Contract: Gate definitions
#[test]
fn contract_gate_definitions() {
    use oya::types::{Gate, StageName};

    // Contract and Red only need to compile.
    for stage in [StageName::Contract, StageName::Red] {
        let gates = stage.gates();
        assert_eq!(gates.len(), 1, "{:?} should have 1 gate", stage);
        assert_eq!(gates[0], Gate::Compiles);
    }

    // Tdd15 needs compile + tests
    let tdd15_gates = StageName::Implementation.gates();
    assert!(tdd15_gates.contains(&Gate::Compiles));
    assert!(tdd15_gates.contains(&Gate::TestsPass));

    // Witness uses holdout scenario gate.
    let witness_gates = StageName::Witness.gates();
    assert_eq!(witness_gates.len(), 1);
    assert!(witness_gates.contains(&Gate::HoldoutScenarios));

    // ShipGate closes with artifact + merge queue gates.
    let ship_gates = StageName::ShipGate.gates();
    assert_eq!(ship_gates.len(), 1);
    assert!(ship_gates.contains(&Gate::CueArtifactGenerated));
}

/// Contract: Failure categories are retryable/non-retryable
#[test]
fn contract_failure_category_retryability() {
    use oya::is_retryable_failure;
    use oya::types::FailureCategory;

    // Retryable failures (code-level issues AI can fix)
    let retryable = vec![
        FailureCategory::TestFailed,
        FailureCategory::LintFailed,
        FailureCategory::OutputParseFailure,
        FailureCategory::CompileFailed,
    ];

    for category in retryable {
        assert!(is_retryable_failure(&category), "{:?} should be retryable", category);
    }

    // Non-retryable failures (infrastructure/external issues)
    let non_retryable = vec![
        FailureCategory::AuthFailed,
        FailureCategory::ProviderUnavailable,
        FailureCategory::ContextOverflow,
        FailureCategory::MergeConflict,
        FailureCategory::MaxAttemptsExceeded,
        FailureCategory::RateLimited,
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
        StageName::Contract,
        StageName::Contract,
        StageName::Implementation,
        StageName::Implementation,
        StageName::ShipGate,
        StageName::ShipGate,
        StageName::ShipGate,
    ] {
        assert_eq!(stage.max_attempts(), 2, "{:?} should have 2 max attempts", stage);
    }
}
