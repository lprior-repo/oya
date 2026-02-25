//! Behavior-driven tests for the orchestrator pipeline
//!
//! These tests use Martin Fowler style - testing behavior, not implementation.
//! They verify WHAT the system does, not HOW it does it.

use anyhow::Result;
use oya::orchestrator::{Orchestrator, StageExecutionResult, StageRequest};
use oya::types::{FailureCategory, ModelId, StageFailure, StageName};
use serde_json::json;

mod util;

#[tokio::test]
async fn given_all_stages_pass_when_pipeline_runs_then_status_is_shipped() -> Result<()> {
    let orch = util::passing_orchestrator();
    let model = ModelId::new("test-model").map_err(|e| anyhow::anyhow!(e))?;
    let stages = vec![StageName::JjWorkspace, StageName::Implementation, StageName::Main];

    for stage in &stages {
        let result = orch
            .run_stage(StageRequest {
                stage: stage.clone(),
                attempt: 1,
                bead_id: "bead".to_string(),
                context: "ctx".to_string(),
                model: model.clone(),
                last_failure: None,
            })
            .await
            .map_err(|e| anyhow::anyhow!(e))?;
        assert!(result.passed, "Stage {:?} should pass", stage);
    }

    let calls = orch.calls();
    assert_eq!(calls.len(), 3, "All 3 stages should execute exactly once");
    Ok(())
}

#[tokio::test]
async fn given_stage_succeeds_when_it_completes_then_advances_to_next() -> Result<()> {
    let orch = util::passing_orchestrator();
    let model = ModelId::new("test-model").map_err(|e| anyhow::anyhow!(e))?;

    let result = orch
        .run_stage(StageRequest {
            stage: StageName::Implementation,
            attempt: 1,
            bead_id: "bead".to_string(),
            context: "ctx".to_string(),
            model,
            last_failure: None,
        })
        .await
        .map_err(|e| anyhow::anyhow!(e))?;

    assert!(result.passed);
    assert_eq!(result.next_stage, Some(StageName::Main), "Implementation should advance to Main");
    Ok(())
}

#[tokio::test]
async fn given_retryable_failure_when_exhausted_2_attempts_then_fails_permanently() -> Result<()> {
    let orch = util::failing_orchestrator(vec![
        (StageName::Implementation, 1, FailureCategory::TestFailed),
        (StageName::Implementation, 2, FailureCategory::TestFailed),
    ]);
    let model = ModelId::new("test-model").map_err(|e| anyhow::anyhow!(e))?;

    // Attempt 1: Fail
    let result1 = orch
        .run_stage(StageRequest {
            stage: StageName::Implementation,
            attempt: 1,
            bead_id: "bead".to_string(),
            context: "ctx".to_string(),
            model: model.clone(),
            last_failure: None,
        })
        .await
        .map_err(|e| anyhow::anyhow!(e))?;
    assert!(!result1.passed);
    assert_eq!(result1.next_stage, Some(StageName::Implementation)); // Retry

    // Attempt 2: Fail
    let result2 = orch
        .run_stage(StageRequest {
            stage: StageName::Implementation,
            attempt: 2,
            bead_id: "bead".to_string(),
            context: "ctx".to_string(),
            model,
            last_failure: None,
        })
        .await
        .map_err(|e| anyhow::anyhow!(e))?;
    assert!(!result2.passed);
    assert_eq!(result2.next_stage, Some(StageName::Implementation)); // Retry

    let calls = orch.stage_calls(&StageName::Implementation);
    assert_eq!(calls.len(), 2, "Should make exactly 2 attempts before giving up");
    Ok(())
}

#[tokio::test]
async fn given_retryable_failure_when_under_max_attempts_then_stays_on_stage() -> Result<()> {
    let orch = util::failing_orchestrator(vec![(
        StageName::Implementation,
        1,
        FailureCategory::TestFailed,
    )]);
    let model = ModelId::new("test-model").map_err(|e| anyhow::anyhow!(e))?;

    let result = orch
        .run_stage(StageRequest {
            stage: StageName::Implementation,
            attempt: 1,
            bead_id: "bead".to_string(),
            context: "ctx".to_string(),
            model,
            last_failure: None,
        })
        .await
        .map_err(|e| anyhow::anyhow!(e))?;

    assert!(!result.passed);
    assert_eq!(
        result.next_stage,
        Some(StageName::Implementation),
        "Should stay on Implementation stage for retry"
    );
    Ok(())
}

#[tokio::test]
async fn given_stage_fails_then_succeeds_on_retry_when_retry_passes_then_advances() -> Result<()> {
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
    let model = ModelId::new("test-model").map_err(|e| anyhow::anyhow!(e))?;

    // Attempt 1: Fail
    let result1 = orch
        .run_stage(StageRequest {
            stage: StageName::Implementation,
            attempt: 1,
            bead_id: "bead".to_string(),
            context: "ctx".to_string(),
            model: model.clone(),
            last_failure: None,
        })
        .await
        .map_err(|e| anyhow::anyhow!(e))?;
    assert!(!result1.passed);

    // Attempt 2: Succeed
    let result2 = orch
        .run_stage(StageRequest {
            stage: StageName::Implementation,
            attempt: 2,
            bead_id: "bead".to_string(),
            context: "ctx".to_string(),
            model,
            last_failure: None,
        })
        .await
        .map_err(|e| anyhow::anyhow!(e))?;

    assert!(result2.passed);
    assert_eq!(result2.next_stage, Some(StageName::Implementation), "Should advance after success");
    Ok(())
}

#[tokio::test]
async fn given_non_retryable_failure_when_it_occurs_then_fails_immediately() -> Result<()> {
    let orch = util::failing_orchestrator(vec![(
        StageName::Implementation,
        1,
        FailureCategory::AuthFailed,
    )]);
    let model = ModelId::new("test-model").map_err(|e| anyhow::anyhow!(e))?;

    let result = orch
        .run_stage(StageRequest {
            stage: StageName::Implementation,
            attempt: 1,
            bead_id: "bead".to_string(),
            context: "ctx".to_string(),
            model,
            last_failure: None,
        })
        .await
        .map_err(|e| anyhow::anyhow!(e))?;

    assert!(!result.passed);
    assert_eq!(
        result.failure_category,
        Some(FailureCategory::AuthFailed),
        "Should report AuthFailed"
    );

    let calls = orch.stage_calls(&StageName::Implementation);
    assert_eq!(calls.len(), 1, "Should not retry non-retryable failures");
    Ok(())
}

#[tokio::test]
async fn given_plan_completes_when_successful_then_moves_to_contract() -> Result<()> {
    let orch = util::passing_orchestrator();
    let model = ModelId::new("test-model").map_err(|e| anyhow::anyhow!(e))?;

    let result = orch
        .run_stage(StageRequest {
            stage: StageName::JjWorkspace,
            attempt: 1,
            bead_id: "bead".to_string(),
            context: "ctx".to_string(),
            model,
            last_failure: None,
        })
        .await
        .map_err(|e| anyhow::anyhow!(e))?;

    assert!(result.passed);
    assert_eq!(result.next_stage, Some(StageName::Implementation));
    Ok(())
}

#[tokio::test]
async fn given_shipgate_passes_when_successful_then_pipeline_completes() -> Result<()> {
    let orch = util::passing_orchestrator();
    let model = ModelId::new("test-model").map_err(|e| anyhow::anyhow!(e))?;

    let result = orch
        .run_stage(StageRequest {
            stage: StageName::Main,
            attempt: 1,
            bead_id: "bead".to_string(),
            context: "ctx".to_string(),
            model,
            last_failure: None,
        })
        .await
        .map_err(|e| anyhow::anyhow!(e))?;

    assert!(result.passed);
    assert_eq!(result.next_stage, None, "ShipGate is terminal - pipeline should complete");
    Ok(())
}

#[tokio::test]
async fn given_any_stage_when_successful_then_transitions_to_correct_next() -> Result<()> {
    let test_cases = vec![
        (StageName::JjWorkspace, StageName::Implementation),
        (StageName::Implementation, StageName::Main),
    ];
    let model = ModelId::new("test-model").map_err(|e| anyhow::anyhow!(e))?;

    for (current, expected_next) in test_cases {
        let orch = util::passing_orchestrator();
        let result = orch
            .run_stage(StageRequest {
                stage: current.clone(),
                attempt: 1,
                bead_id: "bead".to_string(),
                context: "ctx".to_string(),
                model: model.clone(),
                last_failure: None,
            })
            .await
            .map_err(|e| anyhow::anyhow!(e))?;

        assert!(result.passed, "{:?} should pass in happy path", current);
        assert_eq!(
            result.next_stage,
            Some(expected_next.clone()),
            "{:?} should transition correctly to {:?}",
            current,
            expected_next
        );
    }
    Ok(())
}

#[tokio::test]
async fn given_stage_fails_when_retry_attempted_then_failure_context_available() -> Result<()> {
    let orch = util::failing_orchestrator(vec![(
        StageName::Implementation,
        1,
        FailureCategory::TestFailed,
    )]);
    let model = ModelId::new("test-model").map_err(|e| anyhow::anyhow!(e))?;

    // First attempt
    orch.run_stage(StageRequest {
        stage: StageName::Implementation,
        attempt: 1,
        bead_id: "bead".to_string(),
        context: "ctx".to_string(),
        model: model.clone(),
        last_failure: None,
    })
    .await
    .map_err(|e| anyhow::anyhow!(e))?;

    // Second attempt with failure context
    orch.run_stage(StageRequest {
        stage: StageName::Implementation,
        attempt: 2,
        bead_id: "bead".to_string(),
        context: "ctx".to_string(),
        model,
        last_failure: Some(StageFailure {
            category: FailureCategory::TestFailed,
            message: "previous error details".to_string(),
            retryable: true,
            failed_at: "2026-02-20T00:00:00Z".to_string(),
        }),
    })
    .await
    .map_err(|e| anyhow::anyhow!(e))?;

    let calls = orch.stage_calls(&StageName::Implementation);
    assert_eq!(calls.len(), 2, "Retry should have been attempted with context");
    Ok(())
}

#[tokio::test]
async fn given_tdd15_fails_once_then_succeeds_when_pipeline_continues_then_completes() -> Result<()>
{
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
    let model = ModelId::new("test-model").map_err(|e| anyhow::anyhow!(e))?;

    for attempt in 1..=2 {
        let result = orch
            .run_stage(StageRequest {
                stage: StageName::Implementation,
                attempt: attempt as u32,
                bead_id: "bead".to_string(),
                context: "ctx".to_string(),
                model: model.clone(),
                last_failure: None,
            })
            .await
            .map_err(|e| anyhow::anyhow!(e))?;
        if attempt == 2 {
            assert!(result.passed);
        }
    }

    let tdd15_calls = orch.stage_calls(&StageName::Implementation);
    assert_eq!(tdd15_calls.len(), 2);
    Ok(())
}

#[tokio::test]
async fn given_intermittent_failures_when_within_retry_limits_then_completes() -> Result<()> {
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
    let model = ModelId::new("test-model").map_err(|e| anyhow::anyhow!(e))?;

    // Contract stage: 2 attempts
    for attempt in 1..=2 {
        let result = orch
            .run_stage(StageRequest {
                stage: StageName::Implementation,
                attempt: attempt as u32,
                bead_id: "bead".to_string(),
                context: "ctx".to_string(),
                model: model.clone(),
                last_failure: None,
            })
            .await
            .map_err(|e| anyhow::anyhow!(e))?;
        if attempt == 2 {
            assert!(result.passed);
        }
    }

    // Implementation stage: 2 attempts
    for attempt in 1..=2 {
        let result = orch
            .run_stage(StageRequest {
                stage: StageName::Implementation,
                attempt: attempt as u32,
                bead_id: "bead".to_string(),
                context: "ctx".to_string(),
                model: model.clone(),
                last_failure: None,
            })
            .await
            .map_err(|e| anyhow::anyhow!(e))?;
        if attempt == 2 {
            assert!(result.passed);
        }
    }

    assert_eq!(orch.stage_calls(&StageName::Implementation).len(), 4);
    Ok(())
}
