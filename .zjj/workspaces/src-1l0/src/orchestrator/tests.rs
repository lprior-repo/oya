use super::*;

fn fake_orch() -> FakeOrchestrator {
    FakeOrchestrator::new(
        FakeOrchestratorConfig::default(),
        "test-run-001".to_string(),
        "test-bead-001".to_string(),
    )
}

#[tokio::test]
async fn fake_orch_returns_default_success() {
    let orch = fake_orch();
    let result = orch
        .run_stage(StageRequest {
            stage: StageName::Contract,
            attempt: 1,
            bead_id: "test-bead".to_string(),
            context: "test context".to_string(),
            last_failure: None,
        })
        .await
        .unwrap();

    assert!(result.passed);
    assert_eq!(result.next_stage, Some(StageName::Red));
}

#[tokio::test]
async fn fake_orch_records_calls() {
    let orch = fake_orch();

    orch.run_stage(StageRequest {
        stage: StageName::Contract,
        attempt: 1,
        bead_id: "b".to_string(),
        context: "c".to_string(),
        last_failure: None,
    })
    .await
    .unwrap();
    orch.run_stage(StageRequest {
        stage: StageName::Contract,
        attempt: 2,
        bead_id: "b".to_string(),
        context: "c".to_string(),
        last_failure: None,
    })
    .await
    .unwrap();

    let calls = orch.stage_calls(&StageName::Contract);
    assert_eq!(calls.len(), 2);
    assert_eq!(calls[0].attempt, Some(1));
    assert_eq!(calls[1].attempt, Some(2));
}

#[tokio::test]
async fn fake_orch_uses_configured_results() {
    let mut config = FakeOrchestratorConfig::default();
    config.stage_results.insert(
        (StageName::Implementation, 1),
        StageExecutionResult {
            passed: false,
            output: serde_json::json!({"error": "test failed"}),
            failure_category: Some(FailureCategory::TestFailed),
            next_stage: Some(StageName::Implementation),
            prompt: "fix the tests".to_string(),
        },
    );

    let orch = FakeOrchestrator::new(config, "run-001".to_string(), "bead-001".to_string());

    let result = orch
        .run_stage(StageRequest {
            stage: StageName::Implementation,
            attempt: 1,
            bead_id: "b".to_string(),
            context: "c".to_string(),
            last_failure: None,
        })
        .await
        .unwrap();

    assert!(!result.passed);
    assert_eq!(result.failure_category, Some(FailureCategory::TestFailed));
}
