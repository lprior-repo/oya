//! Test utilities and infrastructure
//!
//! Provides:
//! - FakeOrchestrator for fast unit tests
//! - Testcontainers setup for Restate integration tests
//! - Wiremock helpers for OpenCode HTTP mocking
//! - Property generators for fuzzing

use oya::orchestrator::{
    FakeOrchestrator, FakeOrchestratorConfig, Orchestrator, StageExecutionResult,
};
use oya::types::{FailureCategory, StageName};
use serde_json::json;

/// Create a fake orchestrator that passes all stages
pub fn passing_orchestrator() -> FakeOrchestrator {
    FakeOrchestrator::new(
        FakeOrchestratorConfig::default(),
        "test-run-001".to_string(),
        "test-bead-001".to_string(),
    )
}

/// Create a fake orchestrator with specific stage failures
pub fn failing_orchestrator(failures: Vec<(StageName, u32, FailureCategory)>) -> FakeOrchestrator {
    let mut config = FakeOrchestratorConfig::default();

    for (stage, attempt, category) in failures {
        config.stage_results.insert(
            (stage.clone(), attempt),
            StageExecutionResult {
                passed: false,
                output: json!({"error": format!("{:?} failed", category)}),
                failure_category: Some(category),
                next_stage: Some(stage.clone()), // Retry same stage
                prompt: "fix it".to_string(),
            },
        );
    }

    FakeOrchestrator::new(config, "test-run-fail".to_string(), "test-bead-fail".to_string())
}

/// Create a fake orchestrator that fails with max retries
pub fn max_retries_exceeded_orchestrator(stage: StageName) -> FakeOrchestrator {
    let mut config = FakeOrchestratorConfig::default();

    // Fail all 3 attempts
    for attempt in 1..=3 {
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

/// Assert that calls were made in expected order
pub fn assert_call_sequence(calls: &[oya::orchestrator::CallRecord], expected: &[&str]) {
    let actual: Vec<String> = calls.iter().map(|c| c.method.clone()).collect();
    assert_eq!(actual, expected, "Expected call sequence {:?}, got {:?}", expected, actual);
}

/// Assert stage was attempted N times
pub fn assert_stage_attempts(
    calls: &[oya::orchestrator::CallRecord],
    stage: StageName,
    expected_attempts: u32,
) {
    let attempts: u32 = calls.iter().filter(|c| c.stage == Some(stage.clone())).count() as u32;
    assert_eq!(
        attempts, expected_attempts,
        "Expected stage {:?} to be attempted {} times, got {}",
        stage, expected_attempts, attempts
    );
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_failing_orchestrator_config() {
        let orch = failing_orchestrator(vec![(StageName::Tdd15, 1, FailureCategory::TestFailed)]);

        // Just verify it builds without panicking
        assert_eq!(orch.run_id(), "test-run-fail");
    }
}
