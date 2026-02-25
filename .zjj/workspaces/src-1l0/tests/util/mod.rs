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

/// Create a fake orchestrator with explicit stage execution results
#[allow(dead_code)]
pub fn orchestrator_with_stage_results(
    stage_results: Vec<((StageName, u32), StageExecutionResult)>,
) -> FakeOrchestrator {
    let mut config = FakeOrchestratorConfig::default();

    for (key, result) in stage_results {
        config.stage_results.insert(key, result);
    }

    FakeOrchestrator::new(config, "test-run-custom".to_string(), "test-bead-custom".to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_failing_orchestrator_config() {
        let orch =
            failing_orchestrator(vec![(StageName::Implementation, 1, FailureCategory::TestFailed)]);

        // Just verify it builds without panicking
        assert_eq!(orch.run_id(), "test-run-fail");
    }
}
