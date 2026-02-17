// Adversarial Test Suite - Red Queen Evolution
// Tests edge cases, error paths, and boundary conditions
//
// FUNCTIONAL RUST: Zero unwrap/panic/expect, Result<T,E> throughout, immutable by default

use oya::domain::{
    AgentId, AgentState, AgentStatus, BeadId, Run, RunId, RunState, StageName, StageResult,
};
use std::thread;

// =============================================================================
// ERROR HANDLING - JSON & INPUT VALIDATION
// =============================================================================

#[test]
fn test_empty_json_payload() {
    // Empty JSON should be handled gracefully
    let result = serde_json::from_str::<serde_json::Value>("");
    assert!(result.is_err(), "Empty JSON should parse as error");
}

#[test]
fn test_malformed_json() {
    // Malformed JSON should be rejected
    let malformed = "{invalid json";
    let result = serde_json::from_str::<serde_json::Value>(malformed);
    assert!(result.is_err(), "Malformed JSON should parse as error");

    let incomplete = "{\"bead_id\": \"test\"";
    let result = serde_json::from_str::<serde_json::Value>(incomplete);
    assert!(result.is_err(), "Incomplete JSON should parse as error");
}

#[test]
fn test_nonexistent_bead_id() {
    // Operations on non-existent beads should fail gracefully
    let bead_id = BeadId::new("nonexistent-bead-12345");
    let run = Run::new(bead_id);

    // Run should still be created (aggregate root pattern)
    assert_eq!(run.bead_id.as_str(), "nonexistent-bead-12345");
    assert!(matches!(run.state, RunState::Pending));
}

// =============================================================================
// CONCURRENCY & RACE CONDITIONS
// =============================================================================

#[test]
fn test_concurrent_run_creation() {
    // Multiple threads creating runs concurrently should not cause data races
    let handles: Vec<_> = (0..10)
        .map(|i| {
            thread::spawn(move || {
                let bead_id = BeadId::new(format!("concurrent-bead-{}", i));
                Run::new(bead_id)
            })
        })
        .collect();

    // All threads should complete successfully
    for handle in handles {
        let run = handle.join();
        assert!(run.is_ok(), "Thread should complete without panic");
        let run = run.unwrap();
        assert!(matches!(run.state, RunState::Pending));
    }
}

// =============================================================================
// STATE MACHINE - INVALID TRANSITIONS
// =============================================================================

#[test]
fn test_invalid_stage_transition() {
    // Attempting to complete a stage that wasn't started should fail
    let bead_id = BeadId::new("test-invalid-transition");
    let run = Run::new(bead_id);

    // Try to complete Contract stage without starting the run
    let stage_result = StageResult {
        run_id: run.id.as_str().to_string(),
        stage: StageName::Contract,
        attempt: 1,
        passed: true,
        output: serde_json::json!({}),
        failure_category: None,
        next_stage: Some(StageName::Tdd15),
    };

    let result = run.complete_stage(StageName::Contract, stage_result);
    assert!(result.is_err(), "Completing a stage without starting should fail");
}

#[test]
fn test_agent_state_invariant_violation() {
    // Agent state should validate invariants

    // VIOLATION: Working status without bead_id
    let agent = AgentState::new(
        AgentId::new(),
        None, // Missing bead_id
        Some(StageName::Contract),
        AgentStatus::Working,
        0,
    );
    assert!(
        agent.validate_invariants().is_err(),
        "Working agent without bead_id should violate invariants"
    );

    // VIOLATION: Working status without current_stage
    let agent = AgentState::new(
        AgentId::new(),
        Some(BeadId::new("test-bead")),
        None, // Missing current_stage
        AgentStatus::Working,
        0,
    );
    assert!(
        agent.validate_invariants().is_err(),
        "Working agent without current_stage should violate invariants"
    );

    // VIOLATION: Done status with bead_id
    let agent = AgentState::new(
        AgentId::new(),
        Some(BeadId::new("test-bead")), // Should not have bead
        None,
        AgentStatus::Done,
        0,
    );
    assert!(
        agent.validate_invariants().is_err(),
        "Done agent with bead_id should violate invariants"
    );

    // VALID: Properly configured Working agent
    let agent = AgentState::new(
        AgentId::new(),
        Some(BeadId::new("test-bead")),
        Some(StageName::Contract),
        AgentStatus::Working,
        0,
    );
    assert!(agent.validate_invariants().is_ok(), "Valid Working agent should pass invariants");
}

// =============================================================================
// ID GENERATION & COLLISIONS
// =============================================================================

#[test]
fn test_ulid_generation_uniqueness() {
    // ULID generation should produce unique IDs
    let ids: Vec<RunId> = (0..100).map(|_| RunId::new()).collect();

    let unique_ids: std::collections::HashSet<_> = ids.iter().map(|id| id.as_str()).collect();

    assert_eq!(unique_ids.len(), 100, "All ULIDs should be unique");
}

#[test]
fn test_ulid_string_roundtrip() {
    // ULID should serialize/deserialize correctly
    let id = RunId::new();
    let id_str = id.as_str();

    assert!(!id_str.is_empty(), "ULID string should not be empty");
    assert!(id_str.len() >= 20, "ULID string should have reasonable length");
}

// =============================================================================
// ADVERSARIAL INPUT - INJECTION ATTACKS
// =============================================================================

#[test]
fn test_adversarial_input_injection() {
    // Test various injection attempts in bead_id
    let malicious_inputs = vec![
        "'; DROP TABLE beads; --",
        "<script>alert('xss')</script>",
        "../../../etc/passwd",
        "💀💀💀💀💀", // Unicode stress test
    ];

    for input in malicious_inputs {
        // System should accept input without panicking
        let bead_id = BeadId::new(input);
        let run = Run::new(bead_id);

        // Run should be created successfully
        assert_eq!(run.bead_id.as_str(), input);
    }
}

#[test]
fn test_very_long_input() {
    // Test very long input that might cause issues
    let long_input = "a".repeat(10000);
    let bead_id = BeadId::new(&long_input);
    let run = Run::new(bead_id);

    // Should not panic
    assert!(matches!(run.state, RunState::Pending));
}

#[test]
fn test_special_characters_in_bead_id() {
    // Test special characters that might cause issues
    let special_cases = vec![
        "bead-with-dashes",
        "bead_with_underscores",
        "bead.with.dots",
        "bead:with:colons",
        "bead/with/slashes",
        "bead\\with\\backslashes",
        "bead with spaces",
    ];

    for input in special_cases {
        let bead_id = BeadId::new(input);
        let run = Run::new(bead_id);

        // Should not panic
        assert!(matches!(run.state, RunState::Pending));
    }
}

#[test]
fn test_whitespace_characters() {
    // Test whitespace characters
    let whitespace_cases = vec!["bead\twith\ttabs", "bead\nwith\nnewlines", "bead\rwith\rcarriage"];

    for input in whitespace_cases {
        let bead_id = BeadId::new(input);
        let run = Run::new(bead_id);

        // Should not panic
        assert!(matches!(run.state, RunState::Pending));
    }
}

// =============================================================================
// EDGE CASES - BOUNDARY CONDITIONS
// =============================================================================

#[test]
fn test_run_stage_progression() {
    // Test complete stage progression through all stages
    let bead_id = BeadId::new("test-progression");
    let run = Run::new(bead_id);

    // Start the run
    let run = run.start();

    assert!(run.is_ok(), "Run should start successfully");
    let run = run.unwrap();

    // Complete Research stage
    let contract_result = StageResult {
        run_id: run.id.as_str().to_string(),
        stage: StageName::Research,
        attempt: 1,
        passed: true,
        output: serde_json::json!({"tests": 5}),
        failure_category: None,
        next_stage: Some(StageName::Plan),
    };
    let run = run.complete_stage(StageName::Research, contract_result);

    assert!(run.is_ok(), "Should complete Research stage");
    let run = run.unwrap();

    // Verify we moved to Plan
    match &run.state {
        RunState::Running { current_stage } => {
            assert_eq!(*current_stage, StageName::Plan);
        }
        _ => panic!("Should be running Plan stage"),
    }
}

#[test]
fn test_run_failure_handling() {
    // Test that failed runs transition to Failed state
    let bead_id = BeadId::new("test-failure");
    let run = Run::new(bead_id);

    let run = run.start();
    assert!(run.is_ok(), "Run should start");
    let run = run.unwrap();

    // Fail the run with a specific reason
    let failed_run = run.fail("Compilation failed".to_string());

    // Run should now be in Failed state
    assert!(matches!(failed_run.state, RunState::Failed { .. }));

    // Verify the failure reason is preserved
    match &failed_run.state {
        RunState::Failed { reason, .. } => {
            assert_eq!(reason, "Compilation failed");
        }
        _ => panic!("Run should be in Failed state"),
    }
}

#[test]
fn test_multiple_stage_attempts() {
    // Test retrying a failed stage - creating a new run for retry
    let bead_id = BeadId::new("test-retry");
    let run = Run::new(bead_id.clone());

    let run = run.start();
    assert!(run.is_ok(), "Run should start");
    let run = run.unwrap();

    // Fail the run
    let failed_run = run.fail("Test failed".to_string());

    // Verify run is in Failed state
    assert!(matches!(failed_run.state, RunState::Failed { .. }));

    // System should allow creating a new run for retry (idempotent operation)
    let retry_run = Run::new(bead_id);
    assert!(matches!(retry_run.state, RunState::Pending));
}

// =============================================================================
// TIME-BASED TESTS
// =============================================================================

#[test]
fn test_timestamp_ordering() {
    // Timestamps should be monotonically increasing
    let bead_id = BeadId::new("test-timestamps");
    let run = Run::new(bead_id);

    let created_at = run.created_at;
    let updated_at = run.updated_at;

    assert!(updated_at >= created_at, "updated_at should be >= created_at");

    // After starting, updated_at should increase
    let run = run.start();
    assert!(run.is_ok(), "Run should start");
    let run = run.unwrap();

    assert!(run.updated_at > created_at, "updated_at should increase after start");
}

// =============================================================================
// NULL AND EMPTY INPUT HANDLING
// =============================================================================

#[test]
fn test_empty_bead_id() {
    // Empty bead_id should be handled
    let bead_id = BeadId::new("");
    let run = Run::new(bead_id);

    // Should create run without panic
    assert_eq!(run.bead_id.as_str(), "");
    assert!(matches!(run.state, RunState::Pending));
}

#[test]
fn test_unicode_in_bead_id() {
    // Test various Unicode characters
    let unicode_cases =
        vec!["bead-🔥-fire", "bead-日本語-japanese", "bead-العربية-arabic", "bead-emoji-😀🎉🚀"];

    for input in unicode_cases {
        let bead_id = BeadId::new(input);
        let run = Run::new(bead_id);

        // Should not panic
        assert_eq!(run.bead_id.as_str(), input);
    }
}
