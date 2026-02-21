use crate::types::FailureCategory;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::path::PathBuf;
use std::process::{Command, Output};

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
            message: scenario_failure_message(&scenarios),
        }
    }

    fn validate_spec(&self) -> Result<SpecValidationResult, Box<dyn std::error::Error>> {
        let output = run_moon_command(["run", ":check"])?;
        let score = spec_score_from_output(&output);
        Ok(SpecValidationResult {
            passed: output.status.success() && score >= self.config.spec_threshold,
            score,
        })
    }

    fn validate_scenarios(&self) -> Result<ScenarioValidationResult, Box<dyn std::error::Error>> {
        let output = run_moon_command(["run", ":holdout"])?;
        Ok(scenario_validation_from_output(&output))
    }
}

fn run_moon_command<const N: usize>(
    args: [&str; N],
) -> Result<std::process::Output, std::io::Error> {
    Command::new("moon").args(args).output()
}

fn command_output(output: &Output) -> String {
    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);

    if stdout.is_empty() {
        return stderr.into_owned();
    }
    if stderr.is_empty() {
        return stdout.into_owned();
    }

    format!("{}\n{}", stdout, stderr)
}

fn occurrence_count(haystack: &str, needle: &str) -> u32 {
    let count = haystack.match_indices(needle).count();
    u32::try_from(count).map_or(u32::MAX, |value| value)
}

fn spec_score_from_output(output: &Output) -> u32 {
    spec_score_from_text(command_output(output).as_str(), output.status.success())
}

fn spec_score_from_text(raw: &str, command_succeeded: bool) -> u32 {
    let lower = raw.to_ascii_lowercase();
    let warnings = occurrence_count(lower.as_str(), "warning:");
    let errors = occurrence_count(lower.as_str(), "error:");
    let penalty = warnings.saturating_mul(5).saturating_add(errors.saturating_mul(20)).min(100);

    if !command_succeeded && warnings == 0 && errors == 0 {
        return 0;
    }

    100_u32.saturating_sub(penalty)
}

fn scenario_validation_from_output(output: &Output) -> ScenarioValidationResult {
    scenario_validation_from_text(command_output(output).as_str(), output.status.success())
}

fn scenario_validation_from_text(raw: &str, command_succeeded: bool) -> ScenarioValidationResult {
    let (passed_count, total_count) = parse_scenario_counts_from_json(raw)
        .or_else(|| parse_scenario_counts_from_test_output(raw))
        .unwrap_or((0, 0));
    let normalized_passed = passed_count.min(total_count);
    let failed_count = total_count.saturating_sub(normalized_passed);
    let passed = command_succeeded && total_count > 0 && failed_count == 0;

    ScenarioValidationResult { passed, passed_count: normalized_passed, total_count, failed_count }
}

fn parse_scenario_counts_from_json(raw: &str) -> Option<(usize, usize)> {
    serde_json::from_str::<Value>(raw)
        .ok()
        .and_then(|value| parse_scenario_counts_from_value(&value))
        .or_else(|| {
            raw.lines().rev().find_map(|line| {
                serde_json::from_str::<Value>(line.trim())
                    .ok()
                    .and_then(|value| parse_scenario_counts_from_value(&value))
            })
        })
}

fn parse_scenario_counts_from_value(value: &Value) -> Option<(usize, usize)> {
    let passed = usize::try_from(value.get("passed_scenarios")?.as_u64()?).ok()?;
    let total = usize::try_from(value.get("total_scenarios")?.as_u64()?).ok()?;
    Some((passed.min(total), total))
}

fn parse_scenario_counts_from_test_output(raw: &str) -> Option<(usize, usize)> {
    let (passed_count, total_count) = raw.lines().filter_map(parse_test_result_counts).fold(
        (0usize, 0usize),
        |(passed, total), (line_passed, line_total)| {
            (passed.saturating_add(line_passed), total.saturating_add(line_total))
        },
    );

    if total_count == 0 {
        None
    } else {
        Some((passed_count.min(total_count), total_count))
    }
}

fn parse_test_result_counts(line: &str) -> Option<(usize, usize)> {
    if !line.to_ascii_lowercase().contains("test result:") {
        return None;
    }

    let segments = line.split(';').collect::<Vec<_>>();
    let passed = parse_metric_count(segments.as_slice(), "passed")?;
    let failed = parse_metric_count(segments.as_slice(), "failed")?;
    Some((passed, passed.saturating_add(failed)))
}

fn parse_metric_count(segments: &[&str], metric: &str) -> Option<usize> {
    segments.iter().find_map(|segment| {
        let tokens = segment.trim().trim_end_matches('.').split_whitespace().collect::<Vec<_>>();

        if tokens.len() < 2 {
            return None;
        }

        let metric_token = tokens[tokens.len().saturating_sub(1)];
        let count_token = tokens[tokens.len().saturating_sub(2)];
        if metric_token != metric {
            return None;
        }

        count_token.parse::<usize>().ok()
    })
}

fn scenario_failure_message(scenarios: &ScenarioValidationResult) -> String {
    if scenarios.total_count == 0 {
        return "holdout scenarios produced no executable results (details redacted)".to_string();
    }

    format!(
        "{} of {} holdout scenarios failed (details redacted)",
        scenarios.failed_count, scenarios.total_count
    )
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

    #[test]
    fn spec_score_penalizes_warning_and_error_lines() {
        let raw = "warning: deprecated config\nerror: parse failed\nwarning: unused";
        assert_eq!(spec_score_from_text(raw, false), 70);
    }

    #[test]
    fn spec_score_returns_zero_on_silent_failure() {
        assert_eq!(spec_score_from_text("", false), 0);
    }

    #[test]
    fn parse_scenario_counts_from_json_accepts_json_line() {
        let raw = "noise\n{\"passed_scenarios\":2,\"total_scenarios\":3}";
        assert_eq!(parse_scenario_counts_from_json(raw), Some((2, 3)));
    }

    #[test]
    fn parse_scenario_counts_from_test_output_aggregates_lines() {
        let raw = "test result: ok. 2 passed; 0 failed;\ntest result: FAILED. 1 passed; 1 failed;";
        assert_eq!(parse_scenario_counts_from_test_output(raw), Some((3, 4)));
    }

    #[test]
    fn scenario_validation_requires_executed_scenarios() {
        let validation = scenario_validation_from_text("running 0 tests", true);
        assert!(!validation.passed);
        assert_eq!(validation.total_count, 0);
    }

    #[test]
    fn scenario_failure_message_redacts_details() {
        let scenarios = ScenarioValidationResult {
            passed: false,
            passed_count: 1,
            total_count: 2,
            failed_count: 1,
        };
        assert_eq!(
            scenario_failure_message(&scenarios),
            "1 of 2 holdout scenarios failed (details redacted)"
        );
    }
}
