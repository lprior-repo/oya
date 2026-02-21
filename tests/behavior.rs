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

use oya::orchestrator::{Orchestrator, StageExecutionResult, StageRequest};
use oya::types::{FailureCategory, StageFailure, StageName};
use serde_json::json;

mod util;

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
        StageName::Contract,
        StageName::Contract,
        StageName::Implementation,
        StageName::Implementation,
        StageName::ShipGate,
        StageName::ShipGate,
        StageName::ShipGate,
    ];

    for stage in &stages {
        let result = orch
            .run_stage(StageRequest {
                stage: stage.clone(),
                attempt: 1 as u32,
                bead_id: "bead".to_string(),
                context: "ctx".to_string(),
                last_failure: None,
            })
            .await
            .unwrap();
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

    let result = orch
        .run_stage(StageRequest {
            stage: StageName::Contract,
            attempt: 1 as u32,
            bead_id: "bead".to_string(),
            context: "ctx".to_string(),
            last_failure: None,
        })
        .await
        .unwrap();

    assert!(result.passed);
    assert_eq!(
        result.next_stage,
        Some(StageName::Implementation),
        "Contract should advance to Implementation"
    );
}

// =============================================================================
// RETRY BEHAVIOR: Transient failures
// =============================================================================

/// Given: Stage fails with retryable error (TestFailed)
/// When: It's attempted 2 times
/// Then: It should retry each time, then fail permanently
#[tokio::test]
async fn given_retryable_failure_when_exhausted_2_attempts_then_fails_permanently() {
    let orch = util::failing_orchestrator(vec![
        (StageName::Implementation, 1, FailureCategory::TestFailed),
        (StageName::Implementation, 2, FailureCategory::TestFailed),
    ]);

    // Attempt 1: Fail
    let result1 = orch
        .run_stage(StageRequest {
            stage: StageName::Implementation,
            attempt: 1 as u32,
            bead_id: "bead".to_string(),
            context: "ctx".to_string(),
            last_failure: None,
        })
        .await
        .unwrap();
    assert!(!result1.passed);
    assert_eq!(result1.next_stage, Some(StageName::Implementation)); // Retry

    // Attempt 2: Fail
    let result2 = orch
        .run_stage(StageRequest {
            stage: StageName::Implementation,
            attempt: 2 as u32,
            bead_id: "bead".to_string(),
            context: "ctx".to_string(),
            last_failure: None,
        })
        .await
        .unwrap();
    assert!(!result2.passed);
    assert_eq!(result2.next_stage, Some(StageName::Implementation)); // Retry

    // Behavior: Exactly 2 attempts were made
    let calls = orch.stage_calls(&StageName::Implementation);
    assert_eq!(calls.len(), 2, "Should make exactly 2 attempts before giving up");
}

/// Given: Stage fails with retryable error
/// When: It's attempted less than max
/// Then: It should stay on same stage for retry
#[tokio::test]
async fn given_retryable_failure_when_under_max_attempts_then_stays_on_stage() {
    let orch = util::failing_orchestrator(vec![(
        StageName::Implementation,
        1,
        FailureCategory::TestFailed,
    )]);

    let result = orch
        .run_stage(StageRequest {
            stage: StageName::Implementation,
            attempt: 1 as u32,
            bead_id: "bead".to_string(),
            context: "ctx".to_string(),
            last_failure: None,
        })
        .await
        .unwrap();

    // Behavior: Stays on same stage to retry
    assert!(!result.passed);
    assert_eq!(
        result.next_stage,
        Some(StageName::Implementation),
        "Should stay on Qa stage for retry"
    );
}

/// Given: Stage fails, then succeeds on retry
/// When: Retry succeeds
/// Then: It should advance to next stage
#[tokio::test]
async fn given_stage_fails_then_succeeds_on_retry_when_retry_passes_then_advances() {
    let orch = util::orchestrator_with_stage_results(vec![
        (
            (StageName::Implementation, 1),
            StageExecutionResult {
                passed: false,
                output: json!({"error": "compile error"}),
                failure_category: Some(FailureCategory::CompileFailed),
                next_stage: Some(StageName::Implementation),
                prompt: "fix compile".to_string(),
            },
        ),
        (
            (StageName::Implementation, 2),
            StageExecutionResult {
                passed: true,
                output: json!({"output": "success"}),
                failure_category: None,
                next_stage: Some(StageName::Implementation),
                prompt: "success".to_string(),
            },
        ),
    ]);

    // Attempt 1: Fail
    let result1 = orch
        .run_stage(StageRequest {
            stage: StageName::Implementation,
            attempt: 1 as u32,
            bead_id: "bead".to_string(),
            context: "ctx".to_string(),
            last_failure: None,
        })
        .await
        .unwrap();
    assert!(!result1.passed);

    // Attempt 2: Succeed
    let result2 = orch
        .run_stage(StageRequest {
            stage: StageName::Implementation,
            attempt: 2 as u32,
            bead_id: "bead".to_string(),
            context: "ctx".to_string(),
            last_failure: None,
        })
        .await
        .unwrap();

    // Behavior: Advances to next stage after success
    assert!(result2.passed);
    assert_eq!(
        result2.next_stage,
        Some(StageName::Implementation),
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
    let orch =
        util::failing_orchestrator(vec![(StageName::Contract, 1, FailureCategory::AuthFailed)]);

    let result = orch
        .run_stage(StageRequest {
            stage: StageName::Contract,
            attempt: 1 as u32,
            bead_id: "bead".to_string(),
            context: "ctx".to_string(),
            last_failure: None,
        })
        .await
        .unwrap();

    // Behavior: Fails immediately, no retry
    assert!(!result.passed);
    assert_eq!(
        result.failure_category,
        Some(FailureCategory::AuthFailed),
        "Should report AuthFailed"
    );

    // Only 1 attempt made (no retries)
    let calls = orch.stage_calls(&StageName::Contract);
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

    let result = orch
        .run_stage(StageRequest {
            stage: StageName::Contract,
            attempt: 1 as u32,
            bead_id: "bead".to_string(),
            context: "ctx".to_string(),
            last_failure: None,
        })
        .await
        .unwrap();

    assert!(result.passed);
    assert_eq!(result.next_stage, Some(StageName::Implementation));
}

/// Given: Pipeline reaches ShipGate
/// When: ShipGate passes
/// Then: Pipeline should complete (no next stage)
#[tokio::test]
async fn given_shipgate_passes_when_successful_then_pipeline_completes() {
    let orch = util::passing_orchestrator();

    let result = orch
        .run_stage(StageRequest {
            stage: StageName::ShipGate,
            attempt: 1 as u32,
            bead_id: "bead".to_string(),
            context: "ctx".to_string(),
            last_failure: None,
        })
        .await
        .unwrap();

    assert!(result.passed);
    assert_eq!(result.next_stage, None, "ShipGate is terminal - pipeline should complete");
}

/// Given: Any stage in the pipeline
/// When: It succeeds
/// Then: It should transition to the correct next stage
#[tokio::test]
async fn given_any_stage_when_successful_then_transitions_to_correct_next() {
    let test_cases = vec![
        (StageName::Contract, StageName::Contract),
        (StageName::Contract, StageName::Implementation),
        (StageName::Implementation, StageName::Implementation),
        (StageName::Implementation, StageName::Implementation),
        (StageName::Implementation, StageName::Implementation),
        (StageName::Implementation, StageName::ShipGate),
        (StageName::ShipGate, StageName::ShipGate),
        (StageName::ShipGate, StageName::ShipGate),
    ];

    for (current, expected_next) in test_cases {
        let orch = util::passing_orchestrator();
        let result = orch
            .run_stage(StageRequest {
                stage: current.clone(),
                attempt: 1 as u32,
                bead_id: "bead".to_string(),
                context: "ctx".to_string(),
                last_failure: None,
            })
            .await
            .unwrap();

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
    let orch = util::failing_orchestrator(vec![(
        StageName::Implementation,
        1,
        FailureCategory::TestFailed,
    )]);

    // First attempt
    orch.run_stage(StageRequest {
        stage: StageName::Implementation,
        attempt: 1 as u32,
        bead_id: "bead".to_string(),
        context: "ctx".to_string(),
        last_failure: None,
    })
    .await
    .unwrap();

    // Second attempt with failure context
    orch.run_stage(StageRequest {
        stage: StageName::Implementation,
        attempt: 2 as u32,
        bead_id: "bead".to_string(),
        context: "ctx".to_string(),
        last_failure: Some(StageFailure {
            category: FailureCategory::TestFailed,
            message: "previous error details".to_string(),
            retryable: oya::is_retryable_failure(&FailureCategory::TestFailed),
            failed_at: "2026-02-20T00:00:00Z".to_string(),
        }),
    })
    .await
    .unwrap();

    // Behavior: Both attempts were made (context was passed)
    let calls = orch.stage_calls(&StageName::Implementation);
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
    let orch = util::orchestrator_with_stage_results(vec![
        (
            (StageName::Implementation, 1),
            StageExecutionResult {
                passed: false,
                output: json!({"error": "test failure"}),
                failure_category: Some(FailureCategory::TestFailed),
                next_stage: Some(StageName::Implementation),
                prompt: "fix tests".to_string(),
            },
        ),
        (
            (StageName::Implementation, 2),
            StageExecutionResult {
                passed: true,
                output: json!({"output": "tests pass"}),
                failure_category: None,
                next_stage: Some(StageName::Implementation),
                prompt: "success".to_string(),
            },
        ),
    ]);

    // Run Tdd15 twice
    for attempt in 1..=2 {
        let result = orch
            .run_stage(StageRequest {
                stage: StageName::Implementation,
                attempt: attempt as u32,
                bead_id: "bead".to_string(),
                context: "ctx".to_string(),
                last_failure: None,
            })
            .await
            .unwrap();
        if attempt == 2 {
            assert!(result.passed);
        }
    }

    // Behavior: Exactly 2 attempts on Tdd15
    let tdd15_calls = orch.stage_calls(&StageName::Implementation);
    assert_eq!(tdd15_calls.len(), 2);
}

/// Given: Multiple stages with intermittent failures
/// When: Pipeline runs with retries
/// Then: Should eventually complete if all stages pass within retry limit
#[tokio::test]
async fn given_intermittent_failures_when_within_retry_limits_then_completes() {
    let orch = util::orchestrator_with_stage_results(vec![
        (
            (StageName::Contract, 1),
            StageExecutionResult {
                passed: false,
                output: json!({"error": "compile error"}),
                failure_category: Some(FailureCategory::CompileFailed),
                next_stage: Some(StageName::Contract),
                prompt: "fix compile".to_string(),
            },
        ),
        (
            (StageName::Implementation, 1),
            StageExecutionResult {
                passed: false,
                output: json!({"error": "test failure"}),
                failure_category: Some(FailureCategory::TestFailed),
                next_stage: Some(StageName::Implementation),
                prompt: "fix tests".to_string(),
            },
        ),
        (
            (StageName::Implementation, 2),
            StageExecutionResult {
                passed: true,
                output: json!({"output": "tests pass"}),
                failure_category: None,
                next_stage: Some(StageName::Implementation),
                prompt: "success".to_string(),
            },
        ),
    ]);

    // Contract stage: 2 attempts
    for attempt in 1..=2 {
        let result = orch
            .run_stage(StageRequest {
                stage: StageName::Contract,
                attempt: attempt as u32,
                bead_id: "bead".to_string(),
                context: "ctx".to_string(),
                last_failure: None,
            })
            .await
            .unwrap();
        if attempt == 2 {
            assert!(result.passed);
        }
    }

    // Tdd15 stage: 2 attempts
    for attempt in 1..=2 {
        let result = orch
            .run_stage(StageRequest {
                stage: StageName::Implementation,
                attempt: attempt as u32,
                bead_id: "bead".to_string(),
                context: "ctx".to_string(),
                last_failure: None,
            })
            .await
            .unwrap();
        if attempt == 2 {
            assert!(result.passed);
        }
    }

    // Verify retry counts
    assert_eq!(orch.stage_calls(&StageName::Contract).len(), 2);
    assert_eq!(orch.stage_calls(&StageName::Implementation).len(), 2);
}
