use super::*;
use anyhow::Result;

fn fake_orch() -> FakeOrchestrator {
    FakeOrchestrator::new(
        FakeOrchestratorConfig::default(),
        "test-run-001".to_string(),
        "test-bead-001".to_string(),
    )
}

#[tokio::test]
async fn fake_orch_returns_default_success() -> Result<()> {
    let orch = fake_orch();
    let model = ModelId::new("test-model").map_err(|e| anyhow::anyhow!(e))?;
    let result = orch
        .run_stage(StageRequest {
            stage: StageName::Implementation,
            attempt: 1,
            bead_id: "test-bead".to_string(),
            context: "test context".to_string(),
            model,
            last_failure: None,
        })
        .await
        .map_err(|e| anyhow::anyhow!(e))?;

    assert!(result.passed);
    assert_eq!(result.next_stage, Some(StageName::Main));
    Ok(())
}

#[tokio::test]
async fn fake_orch_records_calls() -> Result<()> {
    let orch = fake_orch();
    let model = ModelId::new("test-model").map_err(|e| anyhow::anyhow!(e))?;

    orch.run_stage(StageRequest {
        stage: StageName::Implementation,
        attempt: 1,
        bead_id: "b".to_string(),
        context: "c".to_string(),
        model: model.clone(),
        last_failure: None,
    })
    .await
    .map_err(|e| anyhow::anyhow!(e))?;
    orch.run_stage(StageRequest {
        stage: StageName::Implementation,
        attempt: 2,
        bead_id: "b".to_string(),
        context: "c".to_string(),
        model,
        last_failure: None,
    })
    .await
    .map_err(|e| anyhow::anyhow!(e))?;

    let calls = orch.stage_calls(&StageName::Implementation);
    assert_eq!(calls.len(), 2);
    assert_eq!(calls[0].attempt, Some(1));
    assert_eq!(calls[1].attempt, Some(2));
    Ok(())
}

#[tokio::test]
async fn fake_orch_uses_configured_results() -> Result<()> {
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
    let model = ModelId::new("test-model").map_err(|e| anyhow::anyhow!(e))?;

    let result = orch
        .run_stage(StageRequest {
            stage: StageName::Implementation,
            attempt: 1,
            bead_id: "b".to_string(),
            context: "c".to_string(),
            model,
            last_failure: None,
        })
        .await
        .map_err(|e| anyhow::anyhow!(e))?;

    assert!(!result.passed);
    assert_eq!(result.failure_category, Some(FailureCategory::TestFailed));
    Ok(())
}
