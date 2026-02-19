use crate::orchestrator::StageExecutionResult;
use crate::types::{FailureCategory, StageName};
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
    pub failure_category: Option<FailureCategory>,
    pub message: String,
}

impl QualityGateResult {
    pub fn passed(&self) -> bool {
        self.overall_passed
    }

    pub fn failed(&self) -> bool {
        !self.overall_passed
    }

    pub fn summary(&self) -> String {
        if self.overall_passed {
            format!(
                "✅ Quality gate passed - iteration {}/{}",
                self.iteration,
                self.max_iterations()
            )
        } else {
            format!(
                "❌ Quality gate failed - iteration {}/{} - {}",
                self.iteration,
                self.max_iterations(),
                self.message
            )
        }
    }

    pub fn should_retry(&self) -> bool {
        self.iteration < 5 && !self.overall_passed
    }

    pub fn next_iteration(&self) -> QualityGateResult {
        let mut result = self.clone();
        result.iteration += 1;
        result
    }

    pub fn max_iterations(&self) -> u32 {
        5
    }
}

pub struct QualityGate {
    config: QualityGateConfig,
    iteration: u32,
}

impl QualityGate {
    pub fn new(config: QualityGateConfig) -> Self {
        Self { config, iteration: 0 }
    }

    pub fn run(&mut self) -> Result<QualityGateResult, Box<dyn std::error::Error>> {
        println!("\n━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
        println!("  QUALITY GATE - Iteration {}", self.iteration);
        println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");

        let mut result = QualityGateResult {
            spec_passed: false,
            spec_score: 0,
            scenarios_passed: false,
            scenarios_passed_count: 0,
            scenarios_total_count: 0,
            overall_passed: false,
            iteration: self.iteration,
            failure_category: None,
            message: String::new(),
        };

        self.iteration += 1;

        // Phase 1: Spec validation
        println!("\n  PHASE 1: SPEC VALIDATION");
        let spec_result = self.validate_spec()?;

        result.spec_passed = spec_result.passed;
        result.spec_score = spec_result.score;

        if !spec_result.passed {
            result.overall_passed = false;
            result.failure_category = Some(FailureCategory::Spec);
            result.message = format!(
                "Spec quality score {}/{} below threshold {}",
                spec_result.score, self.config.spec_threshold
            );
            return Ok(result);
        }

        println!("  ✅ Spec validation passed (score: {}/100)", spec_result.score);

        // Phase 2: Scenario validation
        println!("\n  PHASE 2: SCENARIO VALIDATION");
        let scenario_result = self.validate_scenarios()?;

        result.scenarios_passed = scenario_result.passed;
        result.scenarios_passed_count = scenario_result.passed;
        result.scenarios_total_count = scenario_result.total;

        if !scenario_result.passed {
            result.overall_passed = false;
            result.failure_category = Some(FailureCategory::Validation);
            result.message =
                format!("{} of {} scenarios failed", scenario_result.failed, scenario_result.total);

            // Determine category based on failures
            if result.scenarios_total_count - result.scenarios_passed_count > 3 {
                result.failure_category = Some(FailureCategory::Multiple);
            }

            return Ok(result);
        }

        println!(
            "  ✅ Scenario validation passed ({}/{} passed)",
            scenario_result.passed, scenario_result.total
        );

        result.overall_passed = true;
        result.message = "All quality checks passed".to_string();

        Ok(result)
    }

    fn validate_spec(&self) -> Result<SpecValidationResult, Box<dyn std::error::Error>> {
        let spec_linter_path = "cargo run --bin spec-linter".to_string();
        let spec_arg = format!("-- {}", self.config.spec_path.display());

        let output = Command::new(spec_linter_path)
            .args(["--format", "json", &spec_arg])
            .current_dir(&std::env::var("PROJECT_ROOT").unwrap_or_else(|_| PathBuf::from(".")))
            .output()?;

        let json_str = String::from_utf8_lossy(&output.stdout);
        let report: Value = serde_json::from_str(&json_str)?;

        Ok(SpecValidationResult {
            passed: report["passed"].as_bool().unwrap_or(false),
            score: report["overall_score"].as_u64().unwrap_or(0) as u32,
        })
    }

    fn validate_scenarios(&self) -> Result<ScenarioValidationResult, Box<dyn std::error::Error>> {
        let scenario_runner_path = "cargo run --bin scenario-runner".to_string();
        let scenarios_arg = format!("-- {}", self.config.scenarios_path.display());
        let app_arg = format!("--app-endpoint {}", self.config.app_endpoint);
        let level_arg = format!("--level {}", self.config.feedback_level);

        let output = Command::new(scenario_runner_path)
            .args(["--format", "json", &scenarios_arg, &app_arg, &level_arg])
            .current_dir(&std::env::var("PROJECT_ROOT").unwrap_or_else(|_| PathBuf::from(".")))
            .output()?;

        let json_str = String::from_utf8_lossy(&output.stdout);
        let report: Value = serde_json::from_str(&json_str)?;

        let passed = report["passed_scenarios"].as_u64().unwrap_or(0) as usize;
        let total = report["total_scenarios"].as_u64().unwrap_or(0) as usize;
        let failed = total - passed;

        Ok(ScenarioValidationResult { passed: failed == 0, passed, total, failed })
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct SpecValidationResult {
    passed: bool,
    score: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct ScenarioValidationResult {
    passed: bool,
    passed: usize,
    total: usize,
    failed: usize,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_quality_gate_default_config() {
        let config = QualityGateConfig::default();
        assert_eq!(config.spec_threshold, 80);
        assert_eq!(config.feedback_level, 3);
        assert_eq!(config.max_iterations, 5);
    }

    #[test]
    fn test_quality_gate_result_passed() {
        let result = QualityGateResult {
            spec_passed: true,
            spec_score: 90,
            scenarios_passed: true,
            scenarios_passed_count: 10,
            scenarios_total_count: 10,
            overall_passed: true,
            iteration: 1,
            failure_category: None,
            message: "All good".to_string(),
        };

        assert!(result.passed());
        assert!(!result.failed());
        assert!(result.overall_passed);
        assert!(!result.should_retry());
    }

    #[test]
    fn test_quality_gate_result_failed() {
        let result = QualityGateResult {
            spec_passed: false,
            spec_score: 70,
            scenarios_passed: false,
            scenarios_passed_count: 5,
            scenarios_total_count: 10,
            overall_passed: false,
            iteration: 1,
            failure_category: Some(FailureCategory::Spec),
            message: "Spec failed".to_string(),
        };

        assert!(!result.passed());
        assert!(result.failed());
        assert!(!result.overall_passed);
        assert!(result.should_retry());
    }

    #[test]
    fn test_quality_gate_next_iteration() {
        let result = QualityGateResult {
            spec_passed: false,
            spec_score: 70,
            scenarios_passed: false,
            scenarios_passed_count: 5,
            scenarios_total_count: 10,
            overall_passed: false,
            iteration: 1,
            failure_category: Some(FailureCategory::Spec),
            message: "Spec failed".to_string(),
        };

        let next = result.next_iteration();
        assert_eq!(next.iteration, 2);
    }

    #[test]
    fn test_quality_gate_max_iterations_reached() {
        let result = QualityGateResult {
            spec_passed: false,
            spec_score: 70,
            scenarios_passed: false,
            scenarios_passed_count: 5,
            scenarios_total_count: 10,
            overall_passed: false,
            iteration: 5,
            failure_category: Some(FailureCategory::Spec),
            message: "Spec failed".to_string(),
        };

        assert!(!result.should_retry());
        assert_eq!(result.max_iterations(), 5);
    }
}
