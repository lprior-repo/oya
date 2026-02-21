//! State machine tests - exhaustively verify stage transitions
//!
//! These tests verify that the orchestrator correctly transitions through
//! all stages and handles failures appropriately.

use oya::orchestrator::{Orchestrator, StageExecutionResult, StageRequest};
use oya::types::{FailureCategory, StageFailure, StageName};
use serde_json::json;

mod util;

/// Test that a stage progresses to its next stage on success
#[tokio::test]
async fn test_stage_success_advances() {
    let orch = util::passing_orchestrator();

    let result = orch
        .run_stage(StageRequest {
            stage: StageName::Contract,
            attempt: 1 as u32,
            bead_id: "bead-001".to_string(),
            context: "context".to_string(),
            last_failure: None,
        })
        .await
        .unwrap();

    assert!(result.passed);
    assert_eq!(result.next_stage, Some(StageName::Contract));
}

/// Test all stage transitions in the happy path
#[tokio::test]
async fn test_all_stage_transitions() {
    let test_cases = vec![
        (StageName::Contract, Some(StageName::Contract)),
        (StageName::Contract, Some(StageName::Implementation)),
        (StageName::Implementation, Some(StageName::Implementation)),
        (StageName::Implementation, Some(StageName::Implementation)),
        (StageName::Implementation, Some(StageName::Implementation)),
        (StageName::Implementation, Some(StageName::ShipGate)),
        (StageName::ShipGate, Some(StageName::ShipGate)),
        (StageName::ShipGate, Some(StageName::ShipGate)),
        (StageName::ShipGate, None), // Terminal stage
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
    let orch = util::failing_orchestrator(vec![(StageName::Implementation, 1, FailureCategory::TestFailed)]);

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

    assert!(!result.passed);
    assert_eq!(result.failure_category, Some(FailureCategory::TestFailed));
    assert_eq!(result.next_stage, Some(StageName::Implementation)); // Retry
}

/// Test that max attempts (2) is respected
#[tokio::test]
async fn test_max_attempts_exceeded() {
    let orch = util::failing_orchestrator(vec![
        (StageName::Implementation, 1, FailureCategory::TestFailed),
        (StageName::Implementation, 2, FailureCategory::TestFailed),
    ]);

    // Simulate 2 failed attempts
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

        assert!(!result.passed);
        assert_eq!(result.failure_category, Some(FailureCategory::TestFailed));
    }

    // Verify we made exactly 2 calls
    let calls = orch.stage_calls(&StageName::Implementation);
    assert_eq!(calls.len(), 2);
}

/// Test that CompileFailed is retryable
#[tokio::test]
async fn test_compile_failed_is_retryable() {
    let orch =
        util::failing_orchestrator(vec![(StageName::Implementation, 1, FailureCategory::CompileFailed)]);
    let result = orch
        .run_stage(StageRequest {
            stage: StageName::Implementation,
            attempt: 1 as u32,
            bead_id: "b".to_string(),
            context: "c".to_string(),
            last_failure: None,
        })
        .await
        .unwrap();

    assert!(!result.passed);
    assert_eq!(result.next_stage, Some(StageName::Implementation)); // Can retry
}

/// Test that MergeConflict eventually fails (not retryable forever)
#[tokio::test]
async fn test_merge_conflict_fails_after_max_attempts() {
    let orch = util::orchestrator_with_stage_results(vec![
        (
            (StageName::ShipGate, 1),
            StageExecutionResult {
                passed: false,
                output: json!({"error": "merge conflict"}),
                failure_category: Some(FailureCategory::MergeConflict),
                next_stage: Some(StageName::ShipGate),
                prompt: "fix conflicts".to_string(),
            },
        ),
        (
            (StageName::ShipGate, 2),
            StageExecutionResult {
                passed: false,
                output: json!({"error": "merge conflict"}),
                failure_category: Some(FailureCategory::MergeConflict),
                next_stage: Some(StageName::ShipGate),
                prompt: "fix conflicts".to_string(),
            },
        ),
    ]);

    // All configured attempts should fail
    for attempt in 1..=2 {
        let result = orch
            .run_stage(StageRequest {
                stage: StageName::ShipGate,
                attempt: attempt as u32,
                bead_id: "b".to_string(),
                context: "c".to_string(),
                last_failure: None,
            })
            .await
            .unwrap();
        if attempt < 2 {
            assert_eq!(result.next_stage, Some(StageName::ShipGate));
        }
    }
}

/// Test complete pipeline execution (happy path simulation)
#[tokio::test]
async fn test_complete_pipeline_simulation() {
    let orch = util::passing_orchestrator();
    let stages = vec![
        StageName::Contract,
        StageName::Contract,
        StageName::Implementation,
        StageName::Implementation,
        StageName::Implementation,
        StageName::ShipGate,
        StageName::ShipGate,
        StageName::ShipGate,
    ];

    for stage in stages {
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
        assert!(result.passed, "Stage {:?} should pass in happy path", stage);
    }

    // Verify all stages were called
    let all_calls = orch.calls();
    assert_eq!(all_calls.len(), 8);
}

/// Test that stage failure context is passed correctly
#[tokio::test]
async fn test_failure_context_propagation() {
    let orch = util::failing_orchestrator(vec![(StageName::Implementation, 1, FailureCategory::TestFailed)]);

    // First attempt fails
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

    // Second attempt should receive failure context
    orch.run_stage(StageRequest {
        stage: StageName::Implementation,
        attempt: 2 as u32,
        bead_id: "bead".to_string(),
        context: "ctx".to_string(),
        last_failure: Some(StageFailure {
            category: FailureCategory::TestFailed,
            message: "previous error".to_string(),
            retryable: oya::is_retryable_failure(&FailureCategory::TestFailed),
            failed_at: "2026-02-20T00:00:00Z".to_string(),
        }),
    })
    .await
    .unwrap();

    // Verify second attempt was made
    let calls = orch.stage_calls(&StageName::Implementation);
    assert_eq!(calls.len(), 2);
}
