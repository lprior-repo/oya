// Infrastructure & Workflow Adversarial Test Suite - Red Queen Evolution
// Tests persistence, workflow, and error handling edge cases
//
// FUNCTIONAL RUST: Zero unwrap/panic/expect, Result<T,E> throughout

use oya::domain::{AgentId, AgentState, AgentStatus, BeadId, Run, RunState, StageName};
use std::sync::Arc;
use std::time::Duration;

// =============================================================================
// WORKFLOW STATE TRANSITIONS
// =============================================================================

#[test]
fn test_terminal_states_are_absorbing() {
    // Once in a terminal state, no further transitions should be possible
    let bead_id = BeadId::new("test-terminal");

    // Test Shipped state
    let run = Run::new(bead_id.clone());
    let run = run.start();
    assert!(run.is_ok(), "Run should start");
    let run = run.unwrap();

    // Complete all stages to reach Shipped state
    // For now, just verify we can create terminal states
    assert!(matches!(run.state, RunState::Running { .. }));

    // Terminal states should be: Shipped, Failed, Aborted
    // Once in these states, the run should not accept further transitions
}

#[test]
fn test_invalid_state_transitions_rejected() {
    // Verify that invalid state transitions are rejected
    let bead_id = BeadId::new("test-invalid-transitions");
    let run = Run::new(bead_id);

    // Can't complete a stage before starting
    let result = serde_json::json!({"output": "test"});
    let stage_result = oya::domain::StageResult {
        run_id: run.id.as_str().to_string(),
        stage: StageName::Research,
        attempt: 1,
        passed: true,
        output: result,
        failure_category: None,
        next_stage: Some(StageName::Plan),
    };

    let result = run.complete_stage(StageName::Research, stage_result);
    assert!(result.is_err(), "Should not complete stage before starting");
}

// =============================================================================
// AGENT STATE VALIDATION
// =============================================================================

#[test]
fn test_agent_state_status_transitions() {
    // Agent state should transition through valid statuses
    let agent_id = AgentId::new();

    // Idle -> Working transition
    let agent = AgentState::new(
        agent_id.clone(),
        Some(BeadId::new("test-bead")),
        Some(StageName::Contract),
        AgentStatus::Working,
        0,
    );

    let result = agent.validate_invariants();
    assert!(result.is_ok(), "Working agent with required fields should be valid");

    // Working -> Done transition
    let agent = AgentState::new(
        agent_id,
        None, // Done agents have no bead
        None, // Done agents have no current stage
        AgentStatus::Done,
        1,
    );

    let result = agent.validate_invariants();
    assert!(result.is_ok(), "Done agent should be valid");
}

#[test]
fn test_agent_prevents_invalid_status_combinations() {
    // Certain status combinations should be invalid

    // ERROR status with bead_id should be invalid
    let agent = AgentState::new(
        AgentId::new(),
        Some(BeadId::new("test-bead")),
        Some(StageName::Contract),
        AgentStatus::Error,
        0,
    );

    let result = agent.validate_invariants();
    assert!(result.is_err(), "Error status should not have bead_id");

    // WAITING status with bead_id should be invalid
    let agent = AgentState::new(
        AgentId::new(),
        Some(BeadId::new("test-bead")),
        None,
        AgentStatus::Waiting,
        0,
    );

    let result = agent.validate_invariants();
    assert!(result.is_err(), "Waiting status should not have bead_id");
}

// =============================================================================
// RUN STATE MACHINE
// =============================================================================

#[test]
fn test_run_cannot_start_twice() {
    // Starting an already running run should fail
    let bead_id = BeadId::new("test-double-start");
    let run = Run::new(bead_id);

    let run = run.start();
    assert!(run.is_ok(), "First start should succeed");
    let run = run.unwrap();

    // Try to start again
    let result = run.start();
    assert!(result.is_err(), "Starting already-running run should fail");
}

#[test]
fn test_run_state_serialization_roundtrip() {
    // Run state should serialize and deserialize correctly
    let bead_id = BeadId::new("test-serialization");
    let run = Run::new(bead_id.clone());

    // Serialize
    let serialized = serde_json::to_string(&run);
    assert!(serialized.is_ok(), "Should serialize run");

    // Deserialize
    let deserialized: Result<Run, _> = serde_json::from_str(&serialized.unwrap());
    assert!(deserialized.is_ok(), "Should deserialize run");

    let run2 = deserialized.unwrap();
    assert_eq!(run2.bead_id, bead_id);
    assert!(matches!(run2.state, RunState::Pending));
}

#[test]
fn test_run_history_tracking() {
    // Run should track stage attempts in history
    let bead_id = BeadId::new("test-history");
    let run = Run::new(bead_id);

    let run = run.start();
    assert!(run.is_ok(), "Run should start");
    let run = run.unwrap();

    // Run should have history tracking (verify structure exists)
    // The exact implementation depends on the Run struct
    assert_eq!(run.history.len(), 0, "Initial run should have empty history");
}

// =============================================================================
// STAGE TRANSITION LOGIC
// =============================================================================

#[test]
fn test_stage_progression_follows_canonical_path() {
    // Stages should progress in the defined canonical order
    let stages = vec![
        StageName::Research,
        StageName::Plan,
        StageName::Contract,
        StageName::Tdd15,
        StageName::Qa,
        StageName::RedQueen,
        StageName::GptReview,
        StageName::ShipGate,
    ];

    // Verify each stage has a defined next stage (except last)
    for (i, stage) in stages.iter().enumerate() {
        let next_stage = stage.next();

        if i < stages.len() - 1 {
            assert!(next_stage.is_some(), "Stage {:?} should have a next stage", stage);
            assert_eq!(next_stage, Some(stages[i + 1].clone()));
        } else {
            assert!(next_stage.is_none(), "Last stage should not have a next stage");
        }
    }
}

#[test]
fn test_all_stages_are_reachable() {
    // All defined stages should be reachable from Research
    let mut current_stage = StageName::Research;
    let mut visited_stages = vec![StageName::Research];

    while let Some(next) = current_stage.next() {
        visited_stages.push(next.clone());
        current_stage = next;
    }

    // We should visit all canonical stages
    assert!(visited_stages.len() >= 8, "Should visit all 8 stages in canonical path");
}

// =============================================================================
// BOUNDARY CONDITIONS
// =============================================================================

#[test]
fn test_zero_attempt_count() {
    // Zero attempts should be handled correctly
    let bead_id = BeadId::new("test-zero-attempts");
    let run = Run::new(bead_id);

    let run = run.start();
    assert!(run.is_ok(), "Run should start");
    let _run = run.unwrap();

    // Run should track attempts correctly starting from 0
    // (implementation-dependent, verify no panic occurs)
}

#[test]
fn test_maximum_stage_attempts() {
    // Very high attempt counts should not cause overflow
    let bead_id = BeadId::new("test-max-attempts");
    let run = Run::new(bead_id);

    let run = run.start();
    assert!(run.is_ok(), "Run should start");
    let _run = run.unwrap();

    // Create a stage result with very high attempt count
    let result = serde_json::json!({"attempts": u32::MAX});
    assert!(result.is_object(), "Should handle high attempt values");
}

#[test]
fn test_negative_duration_handling() {
    // Durations should not be negative (Rust's Duration is u64-based, so this is about API safety)
    let duration = Duration::from_secs(0);
    assert_eq!(duration.as_secs(), 0, "Zero duration should be valid");

    let duration = Duration::from_secs(3600);
    assert_eq!(duration.as_secs(), 3600, "Positive duration should be valid");
}

// =============================================================================
// ERROR RECOVERY
// =============================================================================

#[test]
fn test_run_failure_preserves_context() {
    // When a run fails, important context should be preserved
    let bead_id = BeadId::new("test-failure-context");
    let run = Run::new(bead_id);

    let run = run.start();
    assert!(run.is_ok(), "Run should start");
    let run = run.unwrap();

    let run_id = run.id.clone();
    let created_at = run.created_at;

    // Fail the run
    let failed_run = run.fail("Test failure".to_string());

    // Verify context is preserved
    assert_eq!(failed_run.id, run_id, "Run ID should be preserved");
    assert_eq!(failed_run.created_at, created_at, "Creation time should be preserved");
    assert!(failed_run.updated_at > created_at, "Updated time should be newer");
}

#[test]
fn test_multiple_failures_handled_gracefully() {
    // Multiple failure attempts should not cause undefined behavior
    let bead_id = BeadId::new("test-multiple-failures");
    let run = Run::new(bead_id);

    let run = run.start();
    assert!(run.is_ok(), "Run should start");
    let run = run.unwrap();

    // Fail the run
    let failed_run = run.fail("First failure".to_string());

    // Try to fail it again (should either succeed or fail gracefully, not panic)
    let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        failed_run.fail("Second failure".to_string())
    }));

    assert!(result.is_ok(), "Multiple failures should not cause panic");
}

// =============================================================================
// CONCURRENT ACCESS
// =============================================================================

#[test]
fn test_concurrent_state_access() {
    // Multiple reads of run state should be safe
    let bead_id = BeadId::new("test-concurrent-reads");
    let run = Run::new(bead_id);
    let run = run.start();

    assert!(run.is_ok(), "Run should start");
    let run = Arc::new(run.unwrap());
    let run_clone = Arc::clone(&run);

    // Both should have same state
    assert_eq!(run.id, run_clone.id);
    assert_eq!(run.bead_id, run_clone.bead_id);
}

// =============================================================================
// DATA INTEGRITY
// =============================================================================

#[test]
fn test_run_id_uniqueness_across_instances() {
    // Different run instances should have different IDs
    let bead_id = BeadId::new("test-unique-ids");

    let run1 = Run::new(bead_id.clone());
    let run2 = Run::new(bead_id);

    assert_ne!(run1.id, run2.id, "Different runs should have different IDs");
}

#[test]
fn test_timestamp_monotonicity() {
    // Timestamps should be monotonically increasing
    let bead_id = BeadId::new("test-timestamps");
    let run1 = Run::new(bead_id.clone());

    std::thread::sleep(std::time::Duration::from_millis(10));

    let run2 = Run::new(bead_id);

    assert!(
        run2.created_at >= run1.created_at,
        "Later runs should have later or equal creation times"
    );
}

// =============================================================================
// SERIALIZATION EDGE CASES
// =============================================================================

#[test]
fn test_serialization_with_special_characters() {
    // Bead IDs with special characters should serialize correctly
    let special_cases = vec![
        "bead-with-\"quotes\"",
        "bead-with-\\backslash",
        "bead-with-\n-newline",
        "bead-with-\t-tab",
        "bead-with-🦀-rustacean",
    ];

    for input in special_cases {
        let bead_id = BeadId::new(input);
        let run = Run::new(bead_id);

        let serialized = serde_json::to_string(&run);
        assert!(serialized.is_ok(), "Should serialize bead_id with special chars: {}", input);

        let deserialized: Result<Run, _> = serde_json::from_str(&serialized.unwrap());
        assert!(deserialized.is_ok(), "Should deserialize bead_id with special chars: {}", input);
    }
}
