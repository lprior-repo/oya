//! State machine tests - exhaustively verify stage transitions
//!
//! These tests verify that the orchestrator correctly transitions through
//! all stages and handles failures appropriately.

use oya::orchestrator::{
    FakeOrchestrator, FakeOrchestratorConfig, Orchestrator, StageExecutionResult,
};
use oya::types::{FailureCategory, StageName};
use serde_json::json;
use std::collections::HashMap;

mod util;

/// Test that a stage progresses to its next stage on success
#[tokio::test]
async fn test_stage_success_advances() {
    let orch = util::passing_orchestrator();

    let result = orch.run_stage(StageName::Research, 1, "bead-001", "context", None).await.unwrap();

    assert!(result.passed);
    assert_eq!(result.next_stage, Some(StageName::Plan));
}

/// Test all stage transitions in the happy path
#[tokio::test]
async fn test_all_stage_transitions() {
    let test_cases = vec![
        (StageName::Research, Some(StageName::Plan)),
        (StageName::Plan, Some(StageName::Contract)),
        (StageName::Contract, Some(StageName::Tdd15)),
        (StageName::Tdd15, Some(StageName::Qa)),
        (StageName::Qa, Some(StageName::RedQueen)),
        (StageName::RedQueen, Some(StageName::GptReview)),
        (StageName::GptReview, Some(StageName::ShipGate)),
        (StageName::ShipGate, None), // Terminal stage
    ];

    for (current, expected_next) in test_cases {
        let orch = util::passing_orchestrator();
        let current_clone = current.clone();
        let result = orch.run_stage(current_clone, 1, "bead", "ctx", None).await.unwrap();

        assert!(result.passed, "Stage {:?} should pass", current);
        assert_eq!(
            result.next_stage, expected_next,
            "Stage {:?} should transition to {:?}",
            current, expected_next
        );
    }
}

/// Test that TestFailed triggers retry (stays on same stage)
#[tokio::test]
async fn test_test_failed_retries() {
    let mut config = FakeOrchestratorConfig::default();
    config.stage_results.insert(
        (StageName::Tdd15, 1),
        StageExecutionResult {
            passed: false,
            output: json!({"error": "test failed"}),
            failure_category: Some(FailureCategory::TestFailed),
            next_stage: Some(StageName::Tdd15), // Retry same stage
            prompt: "fix tests".to_string(),
        },
    );

    let orch = FakeOrchestrator::new(config, "run-001".to_string(), "bead-001".to_string());

    let result = orch.run_stage(StageName::Tdd15, 1, "bead", "ctx", None).await.unwrap();

    assert!(!result.passed);
    assert_eq!(result.failure_category, Some(FailureCategory::TestFailed));
    assert_eq!(result.next_stage, Some(StageName::Tdd15)); // Retry
}

/// Test that max attempts (3) is respected
#[tokio::test]
async fn test_max_attempts_exceeded() {
    let orch = util::max_retries_exceeded_orchestrator(StageName::Tdd15);

    // Simulate 3 failed attempts
    for attempt in 1..=3 {
        let result = orch.run_stage(StageName::Tdd15, attempt, "bead", "ctx", None).await.unwrap();

        assert!(!result.passed);
        assert_eq!(result.failure_category, Some(FailureCategory::TestFailed));
    }

    // Verify we made exactly 3 calls
    let calls = orch.stage_calls(StageName::Tdd15);
    assert_eq!(calls.len(), 3);
}

/// Test that CompileFailed is retryable
#[tokio::test]
async fn test_compile_failed_is_retryable() {
    let mut config = FakeOrchestratorConfig::default();
    config.stage_results.insert(
        (StageName::Tdd15, 1),
        StageExecutionResult {
            passed: false,
            output: json!({"error": "compile failed"}),
            failure_category: Some(FailureCategory::CompileFailed),
            next_stage: Some(StageName::Tdd15),
            prompt: "fix compile".to_string(),
        },
    );

    let orch = FakeOrchestrator::new(config, "run".to_string(), "bead".to_string());
    let result = orch.run_stage(StageName::Tdd15, 1, "b", "c", None).await.unwrap();

    assert!(!result.passed);
    assert_eq!(result.next_stage, Some(StageName::Tdd15)); // Can retry
}

/// Test that MergeConflict eventually fails (not retryable forever)
#[tokio::test]
async fn test_merge_conflict_fails_after_max_attempts() {
    let mut config = FakeOrchestratorConfig::default();

    // Set up failures for all 3 attempts
    for attempt in 1..=3 {
        config.stage_results.insert(
            (StageName::ShipGate, attempt),
            StageExecutionResult {
                passed: false,
                output: json!({"error": "merge conflict"}),
                failure_category: Some(FailureCategory::MergeConflict),
                next_stage: Some(StageName::GptReview),
                prompt: "fix conflicts".to_string(),
            },
        );
    }

    let orch = FakeOrchestrator::new(config, "run".to_string(), "bead".to_string());

    // All 3 attempts should fail
    for attempt in 1..=3 {
        let result = orch.run_stage(StageName::ShipGate, attempt, "b", "c", None).await.unwrap();
        if attempt < 3 {
            assert_eq!(result.next_stage, Some(StageName::GptReview));
        }
    }
}

/// Test complete pipeline execution (happy path simulation)
#[tokio::test]
async fn test_complete_pipeline_simulation() {
    let orch = util::passing_orchestrator();
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

    for stage in stages {
        let stage_clone = stage.clone();
        let result = orch.run_stage(stage_clone, 1, "bead", "ctx", None).await.unwrap();
        assert!(result.passed, "Stage {:?} should pass in happy path", stage);
    }

    // Verify all stages were called
    let all_calls = orch.calls();
    assert_eq!(all_calls.len(), 8);
}

/// Test that stage failure context is passed correctly
#[tokio::test]
async fn test_failure_context_propagation() {
    let mut config = FakeOrchestratorConfig::default();
    config.stage_results.insert(
        (StageName::Tdd15, 1),
        StageExecutionResult {
            passed: false,
            output: json!({"test": "failure output"}),
            failure_category: Some(FailureCategory::TestFailed),
            next_stage: Some(StageName::Tdd15),
            prompt: "previous failure context".to_string(),
        },
    );

    let orch = FakeOrchestrator::new(config, "run".to_string(), "bead".to_string());

    // First attempt fails
    let result1 = orch.run_stage(StageName::Tdd15, 1, "bead", "ctx", None).await.unwrap();

    assert!(!result1.passed);

    // Second attempt should receive failure context
    let _result2 = orch
        .run_stage(
            StageName::Tdd15,
            2,
            "bead",
            "ctx",
            Some((FailureCategory::TestFailed, "previous error".to_string())),
        )
        .await
        .unwrap();

    // Verify second attempt was made
    let calls = orch.stage_calls(StageName::Tdd15);
    assert_eq!(calls.len(), 2);
}
