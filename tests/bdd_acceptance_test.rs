#![deny(clippy::unwrap_used)]
#![deny(clippy::expect_used)]
#![deny(clippy::panic)]
#![forbid(unsafe_code)]

//! BDD Acceptance Scenarios for Oya Orchestration
//!
//! This module codifies the critical prototype paths as executable acceptance scenarios,
//! following Behavior-Driven Development (BDD) principles with Given-When-Then format.

use oya::application::workflow::{
    calculate_backoff, determine_retry_action, is_retryable_failure, is_terminal_state,
    validate_transition, RetryAction, WorkflowError,
};
use oya::domain::{
    determine_transition, AgentHealthStatus, AgentId, AgentState, AgentStatus, BeadId,
    BehavioralFingerprint, CircuitBreaker, CircuitConfig, CircuitState, FailureCategory,
    HealthMetrics, Run, RunId, RunState, StageAttempt, StageName as Stage, StageResult, StageState,
    StageTransition, TransitionReason,
};
use std::time::Duration;

// =============================================================================
// HAPPY PATH SCENARIOS
// =============================================================================

#[test]
fn scenario_bead_run_completes_all_stages_successfully() {
    // Given a new bead run in Pending state
    let bead_id = BeadId::new("test-bead-123");
    let run = Run::new(bead_id);

    // Then the run should be in Pending state
    assert!(matches!(run.state, RunState::Pending));

    // When the run is started
    let run = match run.start() {
        Ok(r) => r,
        Err(_) => {
            assert!(false, "Run should start successfully");
            unreachable!()
        }
    };

    // Then it should be Running at Contract stage
    match &run.state {
        RunState::Running { current_stage } => {
            assert_eq!(*current_stage, Stage::Contract);
        }
        _ => {
            assert!(false, "Expected Running state at Contract stage");
        }
    }
}

#[test]
fn scenario_stage_progresses_through_canonical_dag() {
    // Given the canonical stage DAG
    let stages = [
        (Stage::Contract, Some(Stage::Tdd15)),
        (Stage::Tdd15, Some(Stage::Qa)),
        (Stage::Qa, Some(Stage::RedQueen)),
        (Stage::RedQueen, Some(Stage::GptReview)),
        (Stage::GptReview, Some(Stage::ShipGate)),
        (Stage::ShipGate, None), // Terminal
    ];

    // When each stage transitions
    for (current, expected_next) in stages {
        let actual_next = current.next();

        // Then the next stage should match the canonical DAG
        assert_eq!(
            actual_next, expected_next,
            "Stage {:?} should transition to {:?}",
            current, expected_next
        );
    }
}

#[test]
fn scenario_run_completes_at_ship_gate() {
    // Given a run at ShipGate stage
    let bead_id = BeadId::new("test-bead-456");
    let mut run = Run::new(bead_id);
    run.state = RunState::Running { current_stage: Stage::ShipGate };

    // When the stage is completed successfully
    let result = StageResult {
        run_id: run.id.as_str().to_string(),
        stage: Stage::ShipGate,
        attempt: 1,
        passed: true,
        output: serde_json::json!({"status": "success"}),
        failure_category: None,
        next_stage: None,
    };

    let run = run.complete_stage(Stage::ShipGate, result);

    // Then the run should transition to Shipped state
    assert!(run.is_ok(), "Run should complete successfully");
    let run = match run {
        Ok(r) => r,
        Err(_) => {
            assert!(false, "Run should be Ok");
            unreachable!()
        }
    };
    assert!(matches!(run.state, RunState::Shipped { .. }), "Run should be in Shipped state");
}

#[test]
fn scenario_agent_transitions_through_workflow() {
    // Given a new agent
    let agent_id = AgentId::new();
    let bead_id = BeadId::new("test-bead");

    // When agent starts working
    let mut agent = AgentState::new(
        agent_id.clone(),
        Some(bead_id.clone()),
        Some(Stage::Contract),
        AgentStatus::Working,
        1,
    );

    // Then agent should have valid working state
    assert!(agent.validate_invariants().is_ok());
    assert_eq!(agent.status, AgentStatus::Working);
    assert_eq!(agent.bead_id, Some(bead_id));

    // When agent completes work
    agent.status = AgentStatus::Done;
    agent.bead_id = None;
    agent.current_stage = None;

    // Then agent should have valid done state
    assert!(agent.validate_invariants().is_ok());
    assert_eq!(agent.status, AgentStatus::Done);
}

// =============================================================================
// RETRY PATH SCENARIOS
// =============================================================================

#[test]
fn scenario_failed_stage_retries_with_backoff() {
    // Given a failed stage at attempt 1
    let stage = Stage::Tdd15;
    let attempt = 1;
    let failure = FailureCategory::TestFailed;

    // When determining retry action
    let result = determine_retry_action(stage.clone(), attempt, &failure, "Test failed");

    // Then it should schedule a retry with exponential backoff
    match result {
        Ok(RetryAction::Scheduled { backoff_duration, next_stage }) => {
            assert_eq!(backoff_duration, Duration::from_secs(2)); // 2^1 = 2 seconds
            assert_eq!(next_stage, Stage::Tdd15);
        }
        _ => {
            assert!(false, "Expected Scheduled retry action");
        }
    }
}

#[test]
fn scenario_retry_backoff_increases_exponentially() {
    // Given increasing attempt numbers
    let attempts = [1, 2, 3, 4];
    let expected_backoffs = [2, 4, 8, 16]; // 2^n seconds

    // When calculating backoff for each attempt
    for (attempt, expected_secs) in attempts.iter().zip(expected_backoffs.iter()) {
        let backoff = calculate_backoff(*attempt);

        // Then backoff should follow exponential pattern
        assert_eq!(
            backoff,
            Duration::from_secs(*expected_secs),
            "Attempt {} should have {} seconds backoff",
            attempt,
            expected_secs
        );
    }
}

#[test]
fn scenario_retryable_failures_trigger_retry() {
    // Given retryable failure categories
    let retryable = [
        FailureCategory::TestFailed,
        FailureCategory::TestInfraFailed,
        FailureCategory::CompileFailed,
        FailureCategory::LintFailed,
        FailureCategory::MergeConflict,
        FailureCategory::RateLimited,
    ];

    // When checking if each is retryable
    for failure in retryable {
        // Then it should be retryable
        assert!(is_retryable_failure(&failure), "{:?} should be retryable", failure);
    }
}

#[test]
fn scenario_stage_retries_multiple_times_before_success() {
    // Given a stage that fails twice then succeeds
    let stage = Stage::Contract;
    let failure = FailureCategory::CompileFailed;

    // Attempt 1: Failure - should schedule retry
    let result1 = determine_retry_action(stage.clone(), 1, &failure, "Compile error");
    assert!(
        matches!(result1, Ok(RetryAction::Scheduled { .. })),
        "Attempt 1 should schedule retry"
    );

    // Attempt 2: Failure - should schedule retry
    let result2 = determine_retry_action(stage.clone(), 2, &failure, "Still compiling");
    assert!(
        matches!(result2, Ok(RetryAction::Scheduled { .. })),
        "Attempt 2 should schedule retry"
    );

    // Attempt 3: Would exceed max attempts (3) - should return error
    let result3 = determine_retry_action(stage.clone(), 3, &failure, "Still failing");
    assert!(
        matches!(result3, Err(WorkflowError::AttemptLimitExceeded { .. })),
        "Attempt 3 should exceed max attempts"
    );
}

// =============================================================================
// TERMINAL FAILURE SCENARIOS
// =============================================================================

#[test]
fn scenario_non_retryable_failure_terminates_run() {
    // Given a non-retryable failure
    let stage = Stage::Contract;
    let attempt = 1;
    let failure = FailureCategory::AuthFailed;

    // When determining retry action
    let result = determine_retry_action(stage, attempt, &failure, "Invalid credentials");

    // Then it should return terminal failure
    match result {
        Ok(RetryAction::TerminalFailure { reason }) => {
            assert!(reason.contains("Non-retryable failure"));
            assert!(reason.contains("auth_failed"));
        }
        _ => {
            assert!(false, "Expected TerminalFailure for non-retryable failure");
        }
    }
}

#[test]
fn scenario_max_attempts_exceeded_terminates_run() {
    // Given max attempts reached
    let stage = Stage::Contract;
    let max_attempts = stage.max_attempts(); // 3
    let failure = FailureCategory::TestFailed;

    // When determining retry action at max attempts
    let result = determine_retry_action(stage, max_attempts, &failure, "Still failing");

    // Then it should return attempt limit exceeded error
    assert!(
        matches!(result, Err(WorkflowError::AttemptLimitExceeded { stage: _, attempt: 3, max: 3 })),
        "Should return AttemptLimitExceeded error"
    );
}

#[test]
fn scenario_run_fails_and_enters_terminal_state() {
    // Given a running bead
    let bead_id = BeadId::new("test-bead-fail");
    let run = Run::new(bead_id);
    let run = match run.start() {
        Ok(r) => r,
        Err(_) => {
            assert!(false, "Run should start");
            unreachable!()
        }
    };

    // When the run fails
    let failed_run = run.fail("Max attempts exceeded".to_string());

    // Then it should be in Failed terminal state
    assert!(
        matches!(failed_run.state, RunState::Failed { reason: _, failed_at: _ }),
        "Run should be in Failed state"
    );
    assert!(is_terminal_state(&failed_run.state));
}

#[test]
fn scenario_terminal_states_have_no_outgoing_transitions() {
    // Given terminal states
    let terminal_states = [
        RunState::Shipped { completed_at: chrono::Utc::now() },
        RunState::Failed { reason: "test".to_string(), failed_at: chrono::Utc::now() },
        RunState::Aborted { reason: "test".to_string(), aborted_at: chrono::Utc::now() },
    ];

    // When validating transitions from terminal states
    for state in terminal_states {
        let result = validate_transition(&state, Stage::Contract);

        // Then all should fail
        assert!(result.is_err(), "Terminal state {:?} should have no outgoing transitions", state);
    }
}

// =============================================================================
// CIRCUIT BREAKER SCENARIOS
// =============================================================================

#[test]
fn scenario_circuit_opens_after_failure_threshold() {
    // Given a closed circuit with failure threshold of 3
    let config = CircuitConfig::new(3, 2, 60);
    let cb = CircuitBreaker::new("test-scope", config);

    // When failures exceed threshold
    let cb = cb.record_failure().record_failure().record_failure();

    // Then circuit should be open
    assert_eq!(cb.state, CircuitState::Open);
    assert!(cb.opened_at.is_some());
    assert!(!cb.state.allows_operations());
}

#[test]
fn scenario_circuit_half_opens_after_timeout() {
    // Given an open circuit with 0 second timeout
    let config = CircuitConfig::new(2, 2, 0);
    let cb = CircuitBreaker::new("test-service", config).record_failure().record_failure();

    assert_eq!(cb.state, CircuitState::Open);

    // When timeout elapses and we try half-open
    let cb = cb.try_half_open();

    // Then circuit should be half-open
    assert_eq!(cb.state, CircuitState::HalfOpen);
    assert!(cb.state.allows_operations());
}

// =============================================================================
// HEALTH MONITORING SCENARIOS
// =============================================================================

#[test]
fn scenario_agent_detected_as_stuck_after_idle_timeout() {
    // Given an agent with high idle time
    let fingerprint = BehavioralFingerprint::new(
        "agent-1",
        Some("bead-123".to_string()),
        "implement",
        0,   // No failures
        600, // 600 seconds idle (over 300 threshold)
        0,   // No retries
    );

    // When checking if stuck
    let is_stuck = fingerprint.is_stuck(300, 5);

    // Then it should be detected as stuck
    assert!(is_stuck);
    assert_eq!(fingerprint.health_status(), AgentHealthStatus::Stuck);
    assert!(fingerprint.health_status().needs_intervention());
}

#[test]
fn scenario_agent_detected_in_retry_loop() {
    // Given an agent with high retry count
    let fingerprint = BehavioralFingerprint::new(
        "agent-1",
        Some("bead-123".to_string()),
        "tdd15",
        0,  // No failures
        60, // 60 seconds idle
        15, // 15 retries (over 10 threshold)
    );

    // When checking for retry loop
    let is_retry_loop = fingerprint.is_retry_loop(10);

    // Then it should be detected as retry loop
    assert!(is_retry_loop);
    assert_eq!(fingerprint.health_status(), AgentHealthStatus::RetryLoop);
}

// =============================================================================
// TRANSITION DECISION SCENARIOS
// =============================================================================

#[test]
fn scenario_successful_stage_advances_to_next() {
    // Given each stage when successful
    let test_cases = [
        (Stage::Contract, StageTransition::Advance(Stage::Tdd15)),
        (Stage::Tdd15, StageTransition::Advance(Stage::Qa)),
        (Stage::Qa, StageTransition::Advance(Stage::RedQueen)),
        (Stage::RedQueen, StageTransition::Advance(Stage::GptReview)),
        (Stage::GptReview, StageTransition::Advance(Stage::ShipGate)),
        (Stage::ShipGate, StageTransition::Complete),
    ];

    // When determining transition on success
    for (stage, expected_transition) in test_cases {
        let decision = determine_transition(stage.clone(), true, false);

        // Then it should advance appropriately
        assert_eq!(
            decision.transition(),
            expected_transition,
            "Stage {:?} should transition to {:?}",
            stage,
            expected_transition
        );
    }
}

#[test]
fn scenario_failed_stage_with_retries_retries() {
    // Given any stage fails with retries available
    let stages = [Stage::Contract, Stage::Tdd15, Stage::Qa, Stage::RedQueen, Stage::GptReview];

    // When determining transition on failure
    for stage in stages {
        let decision = determine_transition(stage.clone(), false, false);

        // Then it should retry
        assert_eq!(
            decision.transition(),
            StageTransition::Retry,
            "Stage {:?} should retry when retries available",
            stage
        );
        assert_eq!(decision.reason(), TransitionReason::StageFailedRetry);
    }
}

#[test]
fn scenario_failed_stage_without_retries_blocks() {
    // Given any stage fails with no retries left
    let stages = [Stage::Contract, Stage::Tdd15, Stage::Qa, Stage::RedQueen, Stage::GptReview];

    // When determining transition on failure with exhausted retries
    for stage in stages {
        let decision = determine_transition(stage.clone(), false, true);

        // Then it should block
        assert_eq!(
            decision.transition(),
            StageTransition::Block,
            "Stage {:?} should block when retries exhausted",
            stage
        );
        assert_eq!(decision.reason(), TransitionReason::StageFailedMaxAttemptsReached);
    }
}

// =============================================================================
// CONCURRENCY SCENARIOS
// =============================================================================

#[test]
fn scenario_concurrent_run_id_generation_is_unique() {
    // Given multiple threads generating run IDs
    use std::sync::mpsc;
    use std::thread;

    let (tx, rx) = mpsc::channel();
    let mut handles = vec![];

    // When spawning threads to generate IDs
    for _ in 0..10 {
        let tx = tx.clone();
        let handle = thread::spawn(move || {
            for _ in 0..10 {
                let run = Run::new(BeadId::new("concurrent"));
                tx.send(run.id.as_str().to_string()).unwrap();
            }
        });
        handles.push(handle);
    }

    // Wait for all threads
    for handle in handles {
        handle.join().unwrap();
    }

    // Collect all IDs
    let mut ids = std::collections::HashSet::new();
    for _ in 0..100 {
        let id = rx.recv().unwrap();
        ids.insert(id);
    }

    // Then all IDs should be unique
    assert_eq!(ids.len(), 100, "All 100 concurrent run IDs should be unique");
}

#[test]
fn scenario_concurrent_agent_id_generation_is_unique() {
    // Given multiple threads generating agent IDs
    use std::sync::mpsc;
    use std::thread;

    let (tx, rx) = mpsc::channel();
    let mut handles = vec![];

    // When spawning threads to generate IDs
    for _ in 0..10 {
        let tx = tx.clone();
        let handle = thread::spawn(move || {
            for _ in 0..10 {
                let agent = AgentId::new();
                tx.send(agent.as_str().to_string()).unwrap();
            }
        });
        handles.push(handle);
    }

    // Wait for all threads
    for handle in handles {
        handle.join().unwrap();
    }

    // Collect all IDs
    let mut ids = std::collections::HashSet::new();
    for _ in 0..100 {
        let id = rx.recv().unwrap();
        ids.insert(id);
    }

    // Then all IDs should be unique
    assert_eq!(ids.len(), 100, "All 100 concurrent agent IDs should be unique");
}

// =============================================================================
// STATE MACHINE VALIDATION SCENARIOS
// =============================================================================

#[test]
fn scenario_run_state_machine_enforces_valid_transitions() {
    // Given a pending run
    let bead_id = BeadId::new("state-machine-test");
    let run = Run::new(bead_id);
    assert!(matches!(run.state, RunState::Pending));

    // When starting the run
    let run = match run.start() {
        Ok(r) => r,
        Err(_) => {
            assert!(false, "Should be able to start pending run");
            unreachable!()
        }
    };

    // Then it should be running
    assert!(matches!(run.state, RunState::Running { .. }));

    // When trying to start again, it should fail
    let result = run.start();
    assert!(result.is_err(), "Should not be able to start already running run");
}

#[test]
fn scenario_agent_invariants_prevent_invalid_states() {
    // Given an agent in Working state
    let mut agent = AgentState::new(
        AgentId::new(),
        Some(BeadId::new("test")),
        Some(Stage::Contract),
        AgentStatus::Working,
        1,
    );

    // Then invariants should pass
    assert!(agent.validate_invariants().is_ok());

    // When setting to Done but keeping bead
    agent.status = AgentStatus::Done;
    // bead_id is still Some

    // Then invariants should fail
    let result = agent.validate_invariants();
    assert!(result.is_err(), "Done agent with bead should fail invariant");
    match result {
        Err(msg) => assert!(msg.contains("must not have a bead")),
        Ok(_) => assert!(false, "Should have failed"),
    }
}

#[test]
fn scenario_invalid_stage_transition_is_rejected() {
    // Given a pending run
    let run = Run::new(BeadId::new("invalid-transition"));
    assert!(matches!(run.state, RunState::Pending));

    // When attempting to transition to an invalid stage
    let result = validate_transition(&run.state, Stage::ShipGate);

    // Then it should fail
    assert!(result.is_err(), "Should not allow transition from Pending to ShipGate");
    match result {
        Err(WorkflowError::InvalidTransition { from, to }) => {
            assert!(from.contains("Pending"));
            assert!(to.contains("ShipGate"));
        }
        _ => assert!(false, "Expected InvalidTransition error"),
    }
}

// =============================================================================
// BOUNDARY AND EDGE CASE SCENARIOS
// =============================================================================

#[test]
fn scenario_backoff_capped_at_maximum() {
    // Given very high attempt numbers
    let high_attempts = [10, 20, 50, 100];

    // When calculating backoff
    for attempt in high_attempts {
        let backoff = calculate_backoff(attempt);

        // Then backoff should be capped at 300 seconds (5 minutes)
        assert_eq!(
            backoff,
            Duration::from_secs(300),
            "Backoff for attempt {} should be capped at 300s",
            attempt
        );
    }
}

#[test]
fn scenario_health_metrics_with_zero_operations() {
    // Given default metrics (zero operations)
    let metrics = HealthMetrics::default();

    // Then success rate should be 100%
    assert_eq!(metrics.success_rate(), 100);
}

#[test]
fn scenario_stage_name_try_from_invalid() {
    // Given invalid stage name strings
    let invalid_names = ["", "invalid", "unknown", "contractt"];

    // When attempting to parse
    for name in invalid_names {
        let result = Stage::try_from(name);
        if name.is_empty() || name != "contract" {
            assert!(result.is_err(), "Should fail for invalid input: {}", name);
        }
    }
}

#[test]
fn scenario_all_failure_categories_classified() {
    // Given all failure categories
    let all_categories = [
        FailureCategory::TestFailed,
        FailureCategory::TestInfraFailed,
        FailureCategory::CompileFailed,
        FailureCategory::LintFailed,
        FailureCategory::MergeConflict,
        FailureCategory::RateLimited,
        FailureCategory::AuthFailed,
        FailureCategory::ContextOverflow,
        FailureCategory::ProviderUnavailable,
        FailureCategory::OutputParseFailure,
        FailureCategory::MaxAttemptsExceeded,
    ];

    // When classifying each
    for category in all_categories {
        let retryable = is_retryable_failure(&category);

        // Then each should be classified correctly
        match category {
            FailureCategory::AuthFailed
            | FailureCategory::ContextOverflow
            | FailureCategory::ProviderUnavailable
            | FailureCategory::OutputParseFailure
            | FailureCategory::MaxAttemptsExceeded => {
                assert!(!retryable, "{:?} should not be retryable", category);
            }
            _ => {
                assert!(retryable, "{:?} should be retryable", category);
            }
        }
    }
}

// =============================================================================
// ADVERSARIAL EDGE CASE TESTS - Breaking the system to find bugs
// =============================================================================

#[test]
fn scenario_backoff_boundary_values() {
    // Test attempt 0 - should return 2^0 = 1 second
    let backoff_0 = calculate_backoff(0);
    assert_eq!(backoff_0.as_secs(), 1, "Attempt 0 should have 1s backoff");

    // Test attempt 1 - should return 2^1 = 2 seconds
    let backoff_1 = calculate_backoff(1);
    assert_eq!(backoff_1.as_secs(), 2, "Attempt 1 should have 2s backoff");

    // Test attempt 8 - should cap at 256 seconds (2^8 = 256)
    let backoff_8 = calculate_backoff(8);
    assert_eq!(backoff_8.as_secs(), 256, "Attempt 8 should have 256s backoff");

    // Test attempt 9+ - should cap at MAX_BACKOFF_SECS (300)
    let backoff_9 = calculate_backoff(9);
    assert_eq!(backoff_9.as_secs(), 300, "Attempt 9+ should cap at 300s");

    // Test large attempt number - should not panic
    let backoff_huge = calculate_backoff(100);
    assert_eq!(backoff_huge.as_secs(), 300, "Huge attempt should cap at 300s");
}

#[test]
fn scenario_invalid_bead_id_handling() {
    // Test empty string BeadId
    let empty_bead = BeadId::new("");
    assert_eq!(empty_bead.as_str(), "", "Empty string should be allowed");

    // Test very long BeadId
    let long_string = "x".repeat(10000);
    let long_bead = BeadId::new(long_string.clone());
    assert_eq!(long_bead.as_str(), &long_string, "Long string should be allowed");

    // Test special characters in BeadId
    let special_bead = BeadId::new("bead-123_abc!@#");
    assert_eq!(special_bead.as_str(), "bead-123_abc!@#", "Special chars should be allowed");
}

#[test]
fn scenario_stage_name_serialization_roundtrip() {
    // Test all stage names serialize and deserialize correctly
    let stages = [
        Stage::Contract,
        Stage::Tdd15,
        Stage::Qa,
        Stage::RedQueen,
        Stage::GptReview,
        Stage::ShipGate,
    ];

    for stage in stages {
        let serialized = serde_json::to_string(&stage).expect("Should serialize");
        let deserialized: Stage = serde_json::from_str(&serialized).expect("Should deserialize");
        assert_eq!(stage, deserialized, "Stage {:?} should roundtrip", stage);
    }
}

#[test]
fn scenario_failure_category_serialization_roundtrip() {
    // Test all failure categories serialize and deserialize correctly
    let categories = [
        FailureCategory::TestFailed,
        FailureCategory::TestInfraFailed,
        FailureCategory::CompileFailed,
        FailureCategory::LintFailed,
        FailureCategory::MergeConflict,
        FailureCategory::RateLimited,
        FailureCategory::AuthFailed,
        FailureCategory::ContextOverflow,
        FailureCategory::ProviderUnavailable,
        FailureCategory::OutputParseFailure,
        FailureCategory::MaxAttemptsExceeded,
    ];

    for category in categories {
        let serialized = serde_json::to_string(&category).expect("Should serialize");
        let deserialized: FailureCategory =
            serde_json::from_str(&serialized).expect("Should deserialize");
        assert_eq!(category, deserialized, "Category {:?} should roundtrip", category);
    }
}

#[test]
fn scenario_concurrent_transition_validation() {
    use std::sync::mpsc;
    use std::thread;

    // Given a run in Running state at Contract stage
    let run = Run::new(BeadId::new("concurrent-test"));
    let run = run.start().expect("Should start");
    let run_state = run.state.clone();

    let (tx, rx) = mpsc::channel();

    // When multiple threads try to validate transitions concurrently
    let handles: Vec<_> = (0..10)
        .map(|_| {
            let tx = tx.clone();
            let state = run_state.clone();
            thread::spawn(move || {
                let result = validate_transition(&state, Stage::Tdd15);
                let _ = tx.send(result);
            })
        })
        .collect();

    drop(tx);

    // Then all should succeed (no race condition in pure function)
    let mut successes = 0;
    let mut failures = 0;
    for result in rx {
        if result.is_ok() {
            successes += 1;
        } else {
            failures += 1;
        }
    }

    for handle in handles {
        let _ = handle.join();
    }

    assert_eq!(successes, 10, "All 10 concurrent validations should succeed");
    assert_eq!(failures, 0, "No failures should occur");
}

#[test]
fn scenario_terminal_state_blocks_all_transitions() {
    // Given a Shipped run
    let mut run = Run::new(BeadId::new("terminal-test"));
    run.state = RunState::Shipped { completed_at: chrono::Utc::now() };

    // When trying to transition from terminal state
    let result = validate_transition(&run.state, Stage::Tdd15);

    // Then it should fail
    assert!(result.is_err(), "Terminal state should reject transitions");

    // Given a Failed run
    run.state =
        RunState::Failed { reason: "Tests failed".to_string(), failed_at: chrono::Utc::now() };

    let result = validate_transition(&run.state, Stage::Contract);
    assert!(result.is_err(), "Failed state should reject transitions");

    // Given an Aborted run
    run.state =
        RunState::Aborted { reason: "User aborted".to_string(), aborted_at: chrono::Utc::now() };

    let result = validate_transition(&run.state, Stage::Tdd15);
    assert!(result.is_err(), "Aborted state should reject transitions");
}

#[test]
fn scenario_pending_only_transitions_to_contract() {
    // Given a Pending run
    let run = Run::new(BeadId::new("pending-test"));

    // When trying to transition to non-Contract stage
    let result = validate_transition(&run.state, Stage::Tdd15);
    assert!(result.is_err(), "Pending should not transition to Tdd15");

    let result = validate_transition(&run.state, Stage::Qa);
    assert!(result.is_err(), "Pending should not transition to Qa");

    let result = validate_transition(&run.state, Stage::ShipGate);
    assert!(result.is_err(), "Pending should not transition to ShipGate");

    // When trying to transition to Contract
    let result = validate_transition(&run.state, Stage::Contract);
    assert!(result.is_ok(), "Pending should transition to Contract");
}

#[test]
fn scenario_circuit_breaker_invalid_state_transitions() {
    let config = CircuitConfig::new(3, 2, 60);

    let cb = CircuitBreaker::new(config.clone());
    assert_eq!(cb.state, CircuitState::Closed, "Should start closed");

    let mut cb = CircuitBreaker::new(config.clone());
    cb.state = CircuitState::HalfOpen;
    cb.record_success();
    assert_eq!(cb.state, CircuitState::Closed, "HalfOpen + success -> Closed");

    let mut cb = CircuitBreaker::new(config);
    cb.state = CircuitState::HalfOpen;
    cb.record_failure();
    assert_eq!(cb.state, CircuitState::Open, "HalfOpen + failure -> Open");

    let mut cb = CircuitBreaker::new(CircuitConfig::new(3, 2, 60));
    cb.record_success();
    assert_eq!(cb.state, CircuitState::Closed, "Closed + success -> Closed");
}

#[test]
fn scenario_health_metrics_extreme_values() {
    let hm = HealthMetrics::default();
    assert_eq!(hm.success_rate(), 100.0, "Zero ops should return 100% success rate");

    let mut hm = HealthMetrics::default();
    for _ in 0..10 {
        hm = hm.record_operation(false);
    }
    assert_eq!(hm.success_rate(), 0.0, "All failures should return 0% success rate");

    let mut hm = HealthMetrics::default();
    for _ in 0..10 {
        hm = hm.record_operation(true);
    }
    assert_eq!(hm.success_rate(), 100.0, "All successes should return 100% success rate");

    let mut hm = HealthMetrics::default();
    for i in 0..10 {
        hm = hm.record_operation(i < 6);
    }
    assert_eq!(hm.success_rate(), 60.0, "6/10 should return 60% success rate");
}

#[test]
fn scenario_behavioral_fingerprint_edge_cases() {
    let fp = BehavioralFingerprint::default();
    assert_eq!(fp.consecutive_failures, 0, "Default should have 0 consecutive failures");
    assert_eq!(fp.total_operations, 0, "Default should have 0 total operations");

    let fp = BehavioralFingerprint::default();
    let fp2 = fp.record_failure();
    assert_ne!(fp.consecutive_failures, fp2.consecutive_failures, "Original should be unchanged");

    let mut fp = BehavioralFingerprint::default();
    for _ in 0..15 {
        fp = fp.record_failure();
    }
    assert!(fp.consecutive_failures >= 10, "Should have many consecutive failures");
}

#[test]
fn scenario_stage_result_serialization_stress() {
    let result = StageResult {
        run_id: "test-run".to_string(),
        stage: Stage::Contract,
        attempt: 1,
        passed: true,
        output: serde_json::json!({"key": "value", "nested": {"a": 1, "b": 2}}),
        failure_category: None,
        next_stage: Some(Stage::Tdd15),
    };

    let serialized = serde_json::to_string(&result).expect("Should serialize");
    let deserialized: StageResult = serde_json::from_str(&serialized).expect("Should deserialize");

    assert_eq!(result.run_id, deserialized.run_id);
    assert_eq!(result.stage, deserialized.stage);
    assert_eq!(result.attempt, deserialized.attempt);
    assert_eq!(result.passed, deserialized.passed);
}

#[test]
fn scenario_run_history_accumulation() {
    let bead_id = BeadId::new("history-test");
    let mut run = Run::new(bead_id);

    assert!(run.history.is_empty(), "Initial history should be empty");

    let attempt1 = StageAttempt {
        run_id: "test-run".to_string(),
        stage: Stage::Contract,
        attempt: 1,
        session_id: None,
        state: StageState::Passed,
        started_at: chrono::Utc::now(),
        completed_at: Some(chrono::Utc::now()),
    };
    run.history.push(attempt1);

    let attempt2 = StageAttempt {
        run_id: "test-run".to_string(),
        stage: Stage::Tdd15,
        attempt: 1,
        session_id: None,
        state: StageState::Passed,
        started_at: chrono::Utc::now(),
        completed_at: Some(chrono::Utc::now()),
    };
    run.history.push(attempt2);

    assert_eq!(run.history.len(), 2, "Should have 2 attempts");
    assert_eq!(run.history[0].stage, Stage::Contract, "First should be Contract");
    assert_eq!(run.history[1].stage, Stage::Tdd15, "Second should be Tdd15");
}

#[test]
fn scenario_waiting_only_transitions_to_tdd15() {
    // Given a Waiting state - test the design constraint
    // The Waiting state should only allow Tdd15 transitions
    // This is validated via validate_transition function

    // Test that Waiting -> Contract is rejected
    let waiting_state = RunState::Waiting { reason: "retry".to_string() };
    let result = validate_transition(&waiting_state, Stage::Contract);
    assert!(result.is_err(), "Waiting should not transition to Contract");

    // Test that Waiting -> Tdd15 is allowed
    let result = validate_transition(&waiting_state, Stage::Tdd15);
    assert!(result.is_ok(), "Waiting should transition to Tdd15");

    // Test that Waiting -> Qa is rejected
    let result = validate_transition(&waiting_state, Stage::Qa);
    assert!(result.is_err(), "Waiting should not transition to Qa");
}

#[test]
fn scenario_deterministic_stage_next_is_consistent() {
    // Given canonical stage progression
    // When calling next() multiple times
    // Then it should always return the same result

    for _ in 0..100 {
        assert_eq!(Stage::Contract.next(), Some(Stage::Tdd15));
        assert_eq!(Stage::Tdd15.next(), Some(Stage::Qa));
        assert_eq!(Stage::Qa.next(), Some(Stage::RedQueen));
        assert_eq!(Stage::RedQueen.next(), Some(Stage::GptReview));
        assert_eq!(Stage::GptReview.next(), Some(Stage::ShipGate));
        assert_eq!(Stage::ShipGate.next(), None);
    }
}

#[test]
fn scenario_run_id_uniqueness_stress() {
    use std::collections::HashSet;

    // Generate 10000 RunIds and verify uniqueness
    let mut ids: HashSet<String> = HashSet::new();

    for _ in 0..10000 {
        let id = RunId::new();
        let inserted = ids.insert(id.as_str().to_string());
        assert!(inserted, "RunId should be unique, found duplicate");
    }

    assert_eq!(ids.len(), 10000, "All 10000 IDs should be unique");
}

#[test]
fn scenario_agent_id_uniqueness_stress() {
    use std::collections::HashSet;

    // Generate 10000 AgentIds and verify uniqueness
    let mut ids: HashSet<String> = HashSet::new();

    for _ in 0..10000 {
        let id = AgentId::new();
        let inserted = ids.insert(id.as_str().to_string());
        assert!(inserted, "AgentId should be unique, found duplicate");
    }

    assert_eq!(ids.len(), 10000, "All 10000 IDs should be unique");
}
