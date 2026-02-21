use crate::types::FailureCategory;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::path::PathBuf;
use std::process::Command;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QualityGateConfig {
    pub spec_path: PathBuf,
    pub scenarios_path: PathBuf,
    pub app_endpoint: String,
    pub feedback_level: u8,
    pub spec_threshold: u32,
    pub max_iterations: u32,
}

impl Default for QualityGateConfig {
    fn default() -> Self {
        Self {
            spec_path: PathBuf::from("specs/flow-wasm-v1.yaml"),
            scenarios_path: PathBuf::from("../scenarios-vault/flow-wasm"),
            app_endpoint: "http://localhost:8080".to_string(),
            feedback_level: 3,
            spec_threshold: 80,
            max_iterations: 5,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QualityGateResult {
    pub spec_passed: bool,
    pub spec_score: u32,
    pub scenarios_passed: bool,
    pub scenarios_passed_count: usize,
    pub scenarios_total_count: usize,
    pub overall_passed: bool,
    pub iteration: u32,
    pub max_iterations: u32,
    pub failure_category: Option<FailureCategory>,
    pub message: String,
}

impl QualityGateResult {
    #[must_use]
    pub const fn passed(&self) -> bool {
        self.overall_passed
    }

    #[must_use]
    pub const fn failed(&self) -> bool {
        !self.overall_passed
    }

    #[must_use]
    pub fn should_retry(&self) -> bool {
        self.iteration < self.max_iterations && !self.overall_passed
    }

    #[must_use]
    pub fn next_iteration(&self) -> Self {
        let mut next = self.clone();
        next.iteration = next.iteration.saturating_add(1);
        next
    }
}

pub struct QualityGate {
    config: QualityGateConfig,
}

impl QualityGate {
    #[must_use]
    pub fn new(config: QualityGateConfig) -> Self {
        Self { config }
    }

    pub fn run(&self) -> Result<QualityGateResult, Box<dyn std::error::Error>> {
        self.run_iteration(1)
    }

    pub fn run_iteration(
        &self,
        iteration: u32,
    ) -> Result<QualityGateResult, Box<dyn std::error::Error>> {
        let spec = self.validate_spec()?;
        if !spec.passed {
            return Ok(self.spec_failure(spec.score, iteration));
        }

        let scenarios = self.validate_scenarios()?;
        if !scenarios.passed {
            return Ok(self.scenario_failure(scenarios, iteration));
        }

        Ok(self.success_result(scenarios, iteration))
    }

    fn success_result(
        &self,
        scenarios: ScenarioValidationResult,
        iteration: u32,
    ) -> QualityGateResult {
        QualityGateResult {
            spec_passed: true,
            spec_score: 100,
            scenarios_passed: true,
            scenarios_passed_count: scenarios.passed_count,
            scenarios_total_count: scenarios.total_count,
            overall_passed: true,
            iteration,
            max_iterations: self.config.max_iterations,
            failure_category: None,
            message: "all quality checks passed".to_string(),
        }
    }

    fn spec_failure(&self, score: u32, iteration: u32) -> QualityGateResult {
        QualityGateResult {
            spec_passed: false,
            spec_score: score,
            scenarios_passed: false,
            scenarios_passed_count: 0,
            scenarios_total_count: 0,
            overall_passed: false,
            iteration,
            max_iterations: self.config.max_iterations,
            failure_category: Some(FailureCategory::CompileFailed),
            message: format!(
                "spec quality score {}/100 below threshold {}",
                score, self.config.spec_threshold
            ),
        }
    }

    fn scenario_failure(
        &self,
        scenarios: ScenarioValidationResult,
        iteration: u32,
    ) -> QualityGateResult {
        QualityGateResult {
            spec_passed: true,
            spec_score: 100,
            scenarios_passed: false,
            scenarios_passed_count: scenarios.passed_count,
            scenarios_total_count: scenarios.total_count,
            overall_passed: false,
            iteration,
            max_iterations: self.config.max_iterations,
            failure_category: Some(FailureCategory::TestFailed),
            message: format!(
                "{} of {} scenarios failed",
                scenarios.failed_count, scenarios.total_count
            ),
        }
    }

    fn validate_spec(&self) -> Result<SpecValidationResult, Box<dyn std::error::Error>> {
        let output = run_moon_command(["run", ":check"])?;
        let score = if output.status.success() { 100 } else { 0 };
        Ok(SpecValidationResult { passed: score >= self.config.spec_threshold, score })
    }

    fn validate_scenarios(&self) -> Result<ScenarioValidationResult, Box<dyn std::error::Error>> {
        let output = run_moon_command(["run", ":holdout"])?;

        let stdout = String::from_utf8_lossy(&output.stdout).to_string();
        let parsed = serde_json::from_str::<Value>(stdout.as_str()).ok();
        let passed_count = parsed
            .as_ref()
            .and_then(|value| value.get("passed_scenarios"))
            .and_then(Value::as_u64)
            .map_or(usize::from(output.status.success()), |value| value as usize);
        let total_count = parsed
            .as_ref()
            .and_then(|value| value.get("total_scenarios"))
            .and_then(Value::as_u64)
            .map_or(1, |value| value as usize);
        let failed_count = total_count.saturating_sub(passed_count);

        Ok(ScenarioValidationResult {
            passed: failed_count == 0 && output.status.success(),
            passed_count,
            total_count,
            failed_count,
        })
    }
}

fn run_moon_command<const N: usize>(
    args: [&str; N],
) -> Result<std::process::Output, std::io::Error> {
    Command::new("moon").args(args).output()
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct SpecValidationResult {
    passed: bool,
    score: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct ScenarioValidationResult {
    passed: bool,
    passed_count: usize,
    total_count: usize,
    failed_count: usize,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_config_matches_contract() {
        let config = QualityGateConfig::default();
        assert_eq!(config.spec_threshold, 80);
        assert_eq!(config.feedback_level, 3);
        assert_eq!(config.max_iterations, 5);
    }

    #[test]
    fn retry_respects_configured_max_iterations() {
        let result = QualityGateResult {
            spec_passed: false,
            spec_score: 70,
            scenarios_passed: false,
            scenarios_passed_count: 0,
            scenarios_total_count: 1,
            overall_passed: false,
            iteration: 5,
            max_iterations: 5,
            failure_category: Some(FailureCategory::CompileFailed),
            message: "spec failed".to_string(),
        };
        assert!(!result.should_retry());
    }

    #[test]
    fn next_iteration_increments_once() {
        let result = QualityGateResult {
            spec_passed: true,
            spec_score: 100,
            scenarios_passed: true,
            scenarios_passed_count: 1,
            scenarios_total_count: 1,
            overall_passed: true,
            iteration: 1,
            max_iterations: 5,
            failure_category: None,
            message: "ok".to_string(),
        };
        assert_eq!(result.next_iteration().iteration, 2);
    }
}
