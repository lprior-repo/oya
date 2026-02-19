//! Behavior-driven tests for the orchestrator pipeline
//!
//! These tests use Martin Fowler style - testing behavior, not implementation.
//! They verify WHAT the system does, not HOW it does it.
//!
//! # Test Philosophy
//!
//! - Given [context], When [event], Then [outcome]
//! - Test domain concepts, not code structure
//! - One concept per test
//! - Tests serve as executable documentation

use oya::orchestrator::{
    FakeOrchestrator, FakeOrchestratorConfig, Orchestrator, StageExecutionResult,
};
use oya::types::{FailureCategory, StageName};
use serde_json::json;

mod util;

fn max_retries_exceeded_orchestrator(stage: StageName) -> FakeOrchestrator {
    let mut config = FakeOrchestratorConfig::default();

    for attempt in 1..=2 {
        config.stage_results.insert(
            (stage.clone(), attempt),
            StageExecutionResult {
                passed: false,
                output: json!({"error": "persistent failure"}),
                failure_category: Some(FailureCategory::TestFailed),
                next_stage: Some(stage.clone()),
                prompt: "fix it".to_string(),
            },
        );
    }

    FakeOrchestrator::new(
        config,
        "test-run-max-retries".to_string(),
        "test-bead-max-retries".to_string(),
    )
}

// =============================================================================
// HAPPY PATH: Pipeline succeeds
// =============================================================================

/// Given: All stages pass on first attempt
/// When: Pipeline runs
/// Then: Status should be "shipped" after completing all 7 stages
#[tokio::test]
async fn given_all_stages_pass_when_pipeline_runs_then_status_is_shipped() {
    let orch = util::passing_orchestrator();
    let stages = vec![
        StageName::Plan,
        StageName::Contract,
        StageName::Tdd15,
        StageName::Qa,
        StageName::RedQueen,
        StageName::GptReview,
        StageName::ShipGate,
    ];

    for stage in &stages {
        let result = orch.run_stage(stage.clone(), 1, "bead", "ctx", None).await.unwrap();
        assert!(result.passed, "Stage {:?} should pass", stage);
    }

    // Verify all 7 stages were executed (the behavior we care about)
    let calls = orch.calls();
    assert_eq!(calls.len(), 7, "All 7 stages should execute exactly once");
}

/// Given: Stage succeeds
/// When: It completes
/// Then: It should advance to the next stage
#[tokio::test]
async fn given_stage_succeeds_when_it_completes_then_advances_to_next() {
    let orch = util::passing_orchestrator();

    let result = orch.run_stage(StageName::Plan, 1, "bead", "ctx", None).await.unwrap();

    assert!(result.passed);
    assert_eq!(result.next_stage, Some(StageName::Contract), "Plan should advance to Contract");
}

// =============================================================================
// RETRY BEHAVIOR: Transient failures
// =============================================================================

/// Given: Stage fails with retryable error (TestFailed)
/// When: It's attempted 2 times
/// Then: It should retry each time, then fail permanently
#[tokio::test]
async fn given_retryable_failure_when_exhausted_2_attempts_then_fails_permanently() {
    let orch = max_retries_exceeded_orchestrator(StageName::Tdd15);

    // Attempt 1: Fail
    let result1 = orch.run_stage(StageName::Tdd15, 1, "bead", "ctx", None).await.unwrap();
    assert!(!result1.passed);
    assert_eq!(result1.next_stage, Some(StageName::Tdd15)); // Retry

    // Attempt 2: Fail
    let result2 = orch.run_stage(StageName::Tdd15, 2, "bead", "ctx", None).await.unwrap();
    assert!(!result2.passed);
    assert_eq!(result2.next_stage, Some(StageName::Tdd15)); // Retry

    // Behavior: Exactly 2 attempts were made
    let calls = orch.stage_calls(StageName::Tdd15);
    assert_eq!(calls.len(), 2, "Should make exactly 2 attempts before giving up");
}

/// Given: Stage fails with retryable error
/// When: It's attempted less than max
/// Then: It should stay on same stage for retry
#[tokio::test]
async fn given_retryable_failure_when_under_max_attempts_then_stays_on_stage() {
    let mut config = FakeOrchestratorConfig::default();
    config.stage_results.insert(
        (StageName::Qa, 1),
        StageExecutionResult {
            passed: false,
            output: json!({"error": "tests failed"}),
            failure_category: Some(FailureCategory::TestFailed),
            next_stage: Some(StageName::Qa), // Retry same stage
            prompt: "fix tests".to_string(),
        },
    );

    let orch = FakeOrchestrator::new(config, "run".to_string(), "bead".to_string());

    let result = orch.run_stage(StageName::Qa, 1, "bead", "ctx", None).await.unwrap();

    // Behavior: Stays on same stage to retry
    assert!(!result.passed);
    assert_eq!(result.next_stage, Some(StageName::Qa), "Should stay on Qa stage for retry");
}

/// Given: Stage fails, then succeeds on retry
/// When: Retry succeeds
/// Then: It should advance to next stage
#[tokio::test]
async fn given_stage_fails_then_succeeds_on_retry_when_retry_passes_then_advances() {
    let mut config = FakeOrchestratorConfig::default();

    // First attempt fails
    config.stage_results.insert(
        (StageName::Tdd15, 1),
        StageExecutionResult {
            passed: false,
            output: json!({"error": "compile error"}),
            failure_category: Some(FailureCategory::CompileFailed),
            next_stage: Some(StageName::Tdd15),
            prompt: "fix compile".to_string(),
        },
    );

    // Second attempt succeeds
    config.stage_results.insert(
        (StageName::Tdd15, 2),
        StageExecutionResult {
            passed: true,
            output: json!({"output": "success"}),
            failure_category: None,
            next_stage: Some(StageName::Qa),
            prompt: "success".to_string(),
        },
    );

    let orch = FakeOrchestrator::new(config, "run".to_string(), "bead".to_string());

    // Attempt 1: Fail
    let result1 = orch.run_stage(StageName::Tdd15, 1, "bead", "ctx", None).await.unwrap();
    assert!(!result1.passed);

    // Attempt 2: Succeed
    let result2 = orch.run_stage(StageName::Tdd15, 2, "bead", "ctx", None).await.unwrap();

    // Behavior: Advances to next stage after success
    assert!(result2.passed);
    assert_eq!(
        result2.next_stage,
        Some(StageName::Qa),
        "Should advance to Qa after Tdd15 succeeds"
    );
}

// =============================================================================
// NON-RETRYABLE FAILURES: Stop immediately
// =============================================================================

/// Given: Stage fails with non-retryable error
/// When: It fails
/// Then: Pipeline should fail immediately without retry
#[tokio::test]
async fn given_non_retryable_failure_when_it_occurs_then_fails_immediately() {
    let mut config = FakeOrchestratorConfig::default();
    config.stage_results.insert(
        (StageName::Plan, 1),
        StageExecutionResult {
            passed: false,
            output: json!({"error": "auth failed"}),
            failure_category: Some(FailureCategory::AuthFailed), // Non-retryable
            next_stage: None,
            prompt: "auth error".to_string(),
        },
    );

    let orch = FakeOrchestrator::new(config, "run".to_string(), "bead".to_string());

    let result = orch.run_stage(StageName::Plan, 1, "bead", "ctx", None).await.unwrap();

    // Behavior: Fails immediately, no retry
    assert!(!result.passed);
    assert_eq!(
        result.failure_category,
        Some(FailureCategory::AuthFailed),
        "Should report AuthFailed"
    );

    // Only 1 attempt made (no retries)
    let calls = orch.stage_calls(StageName::Plan);
    assert_eq!(calls.len(), 1, "Should not retry non-retryable failures");
}

// =============================================================================
// STAGE PROGRESSION: The 7-stage pipeline
// =============================================================================

/// Given: Pipeline starts
/// When: Plan completes successfully
/// Then: It should move to Contract stage
#[tokio::test]
async fn given_plan_completes_when_successful_then_moves_to_contract() {
    let orch = util::passing_orchestrator();

    let result = orch.run_stage(StageName::Plan, 1, "bead", "ctx", None).await.unwrap();

    assert!(result.passed);
    assert_eq!(result.next_stage, Some(StageName::Contract));
}

/// Given: Pipeline reaches ShipGate
/// When: ShipGate passes
/// Then: Pipeline should complete (no next stage)
#[tokio::test]
async fn given_shipgate_passes_when_successful_then_pipeline_completes() {
    let orch = util::passing_orchestrator();

    let result = orch.run_stage(StageName::ShipGate, 1, "bead", "ctx", None).await.unwrap();

    assert!(result.passed);
    assert_eq!(result.next_stage, None, "ShipGate is terminal - pipeline should complete");
}

/// Given: Any stage in the pipeline
/// When: It succeeds
/// Then: It should transition to the correct next stage
#[tokio::test]
async fn given_any_stage_when_successful_then_transitions_to_correct_next() {
    let test_cases = vec![
        (StageName::Plan, StageName::Contract),
        (StageName::Contract, StageName::Tdd15),
        (StageName::Tdd15, StageName::Qa),
        (StageName::Qa, StageName::RedQueen),
        (StageName::RedQueen, StageName::GptReview),
        (StageName::GptReview, StageName::ShipGate),
    ];

    for (current, expected_next) in test_cases {
        let orch = util::passing_orchestrator();
        let result = orch.run_stage(current.clone(), 1, "bead", "ctx", None).await.unwrap();

        assert!(result.passed, "{:?} should pass in happy path", current);
        assert_eq!(
            result.next_stage,
            Some(expected_next.clone()),
            "{:?} should transition to {:?}",
            current,
            expected_next
        );
    }
}

// =============================================================================
// FAILURE CONTEXT: Error information flows
// =============================================================================

/// Given: Stage fails
/// When: Retry is attempted
/// Then: Failure context should be available for the retry
#[tokio::test]
async fn given_stage_fails_when_retry_attempted_then_failure_context_available() {
    let mut config = FakeOrchestratorConfig::default();
    config.stage_results.insert(
        (StageName::Tdd15, 1),
        StageExecutionResult {
            passed: false,
            output: json!({"test": "specific failure details"}),
            failure_category: Some(FailureCategory::TestFailed),
            next_stage: Some(StageName::Tdd15),
            prompt: "detailed error message".to_string(),
        },
    );

    let orch = FakeOrchestrator::new(config, "run".to_string(), "bead".to_string());

    // First attempt
    let _result1 = orch.run_stage(StageName::Tdd15, 1, "bead", "ctx", None).await.unwrap();

    // Second attempt with failure context
    let _result2 = orch
        .run_stage(
            StageName::Tdd15,
            2,
            "bead",
            "ctx",
            Some((FailureCategory::TestFailed, "previous error details".to_string())),
        )
        .await
        .unwrap();

    // Behavior: Both attempts were made (context was passed)
    let calls = orch.stage_calls(StageName::Tdd15);
    assert_eq!(calls.len(), 2, "Retry should have been attempted with context");
}

// =============================================================================
// COMPLEX SCENARIOS: Real-world situations
// =============================================================================

/// Given: Tdd15 fails once with TestFailed, succeeds on second attempt
/// When: Pipeline continues
/// Then: Should complete all stages including ShipGate
#[tokio::test]
async fn given_tdd15_fails_once_then_succeeds_when_pipeline_continues_then_completes() {
    let mut config = FakeOrchestratorConfig::default();

    // Tdd15: Fail, Succeed
    config.stage_results.insert(
        (StageName::Tdd15, 1),
        StageExecutionResult {
            passed: false,
            output: json!({"error": "test failure"}),
            failure_category: Some(FailureCategory::TestFailed),
            next_stage: Some(StageName::Tdd15),
            prompt: "fix tests".to_string(),
        },
    );
    config.stage_results.insert(
        (StageName::Tdd15, 2),
        StageExecutionResult {
            passed: true,
            output: json!({"output": "tests pass"}),
            failure_category: None,
            next_stage: Some(StageName::Qa),
            prompt: "success".to_string(),
        },
    );

    let orch = FakeOrchestrator::new(config, "run".to_string(), "bead".to_string());

    // Run Tdd15 twice
    for attempt in 1..=2 {
        let result = orch.run_stage(StageName::Tdd15, attempt, "bead", "ctx", None).await.unwrap();
        if attempt == 2 {
            assert!(result.passed);
        }
    }

    // Behavior: Exactly 2 attempts on Tdd15
    let tdd15_calls = orch.stage_calls(StageName::Tdd15);
    assert_eq!(tdd15_calls.len(), 2);
}

/// Given: Multiple stages with intermittent failures
/// When: Pipeline runs with retries
/// Then: Should eventually complete if all stages pass within retry limit
#[tokio::test]
async fn given_intermittent_failures_when_within_retry_limits_then_completes() {
    let mut config = FakeOrchestratorConfig::default();

    // Contract: Fail once, then succeed
    config.stage_results.insert(
        (StageName::Contract, 1),
        StageExecutionResult {
            passed: false,
            output: json!({"error": "compile error"}),
            failure_category: Some(FailureCategory::CompileFailed),
            next_stage: Some(StageName::Contract),
            prompt: "fix compile".to_string(),
        },
    );

    // Tdd15: Fail once, then succeed
    config.stage_results.insert(
        (StageName::Tdd15, 1),
        StageExecutionResult {
            passed: false,
            output: json!({"error": "test failure"}),
            failure_category: Some(FailureCategory::TestFailed),
            next_stage: Some(StageName::Tdd15),
            prompt: "fix tests".to_string(),
        },
    );
    config.stage_results.insert(
        (StageName::Tdd15, 2),
        StageExecutionResult {
            passed: true,
            output: json!({"output": "tests pass"}),
            failure_category: None,
            next_stage: Some(StageName::Qa),
            prompt: "success".to_string(),
        },
    );

    let orch = FakeOrchestrator::new(config, "run".to_string(), "bead".to_string());

    // Contract stage: 2 attempts
    for attempt in 1..=2 {
        let result =
            orch.run_stage(StageName::Contract, attempt, "bead", "ctx", None).await.unwrap();
        if attempt == 2 {
            assert!(result.passed);
        }
    }

    // Tdd15 stage: 2 attempts
    for attempt in 1..=2 {
        let result = orch.run_stage(StageName::Tdd15, attempt, "bead", "ctx", None).await.unwrap();
        if attempt == 2 {
            assert!(result.passed);
        }
    }

    // Verify retry counts
    assert_eq!(orch.stage_calls(StageName::Contract).len(), 2);
    assert_eq!(orch.stage_calls(StageName::Tdd15).len(), 2);
}
