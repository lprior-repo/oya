use std::collections::HashMap;
use std::fs;
use std::path::Path;

use super::types::{
    ActionResult, Assertion, CategoryResult, Extraction, Scenario, ScenarioError, ScenarioResult,
    ScenarioStep, StepAction, StepResult, ValidationReport,
};

pub struct ScenarioRunner<S = std::hash::RandomState> {
    http_client: reqwest::Client,
    application_endpoint: String,
    #[allow(dead_code)]
    twin_endpoints: HashMap<String, String, S>,
    extracted_values: HashMap<String, serde_json::Value>,
}

impl<S: std::hash::BuildHasher + Send + Sync> ScenarioRunner<S> {
    #[must_use]
    pub fn new(application_endpoint: &str, twins: HashMap<String, String, S>) -> Self {
        Self {
            http_client: reqwest::Client::new(),
            application_endpoint: application_endpoint.to_string(),
            twin_endpoints: twins,
            extracted_values: HashMap::new(),
        }
    }

    pub async fn run_scenario(&mut self, scenario: &Scenario) -> ScenarioResult {
        let start = std::time::Instant::now();
        let mut step_results = Vec::new();
        let mut passed = true;

        for step in &scenario.steps {
            let step_result = self.execute_step(step).await;
            if !step_result.passed {
                passed = false;
                step_results.push(step_result);
                break;
            }
            step_results.push(step_result);
        }

        let duration = u64::try_from(start.elapsed().as_millis()).map_or(u64::MAX, |value| value);
        ScenarioResult {
            scenario_id: scenario.scenario.id.clone(),
            spec_ref: scenario.scenario.spec_ref.clone(),
            category: scenario.scenario.category.clone(),
            passed,
            steps: step_results,
            total_duration_ms: duration,
            error: None,
        }
    }

    async fn execute_step(&mut self, step: &ScenarioStep) -> StepResult {
        let start = std::time::Instant::now();
        let mut assertions_passed = 0;
        let mut assertions_failed = 0;
        let mut error = None;

        let action_result = self.execute_action(&step.action).await;

        for assertion in &step.assertions {
            match Self::check_assertion(&action_result, assertion) {
                Ok(()) => assertions_passed += 1,
                Err(e) => {
                    assertions_failed += 1;
                    error = Some(e);
                }
            }
        }

        for extraction in &step.extractions {
            self.extract_value(&action_result, extraction);
        }

        let duration = u64::try_from(start.elapsed().as_millis()).map_or(u64::MAX, |value| value);
        StepResult {
            step_id: step.id.clone(),
            passed: assertions_failed == 0,
            duration_ms: duration,
            assertions_passed,
            assertions_failed,
            error,
        }
    }

    async fn execute_action(&self, action: &StepAction) -> ActionResult {
        match action.action_type.as_str() {
            "http" => {
                let client = &self.http_client;
                let url = action.url.as_ref().map_or_else(String::new, |value| {
                    value.replace("${application.endpoint}", &self.application_endpoint)
                });

                if url.is_empty() {
                    return ActionResult {
                        status: 0,
                        body: "Missing URL for http action".to_string(),
                        response_time_ms: 0,
                    };
                }

                let method = action.method.as_deref().map_or("GET", |value| value);

                let mut req = match method {
                    "POST" => client.post(&url),
                    "PUT" => client.put(&url),
                    "DELETE" => client.delete(&url),
                    _ => client.get(&url),
                };

                if let Some(headers) = &action.headers {
                    for (key, value) in headers {
                        req = req.header(key, value);
                    }
                }

                if let Some(body) = &action.body {
                    req = req.json(body);
                }

                match req.send().await {
                    Ok(response) => {
                        let status = response.status().as_u16();
                        let body = match response.text().await {
                            Ok(text) => text,
                            Err(e) => format!("<failed to read response body: {e}>"),
                        };
                        ActionResult { status, body, response_time_ms: 0 }
                    }
                    Err(e) => ActionResult { status: 0, body: e.to_string(), response_time_ms: 0 },
                }
            }
            _ => ActionResult {
                status: 0,
                body: format!("Unknown action type: {}", action.action_type),
                response_time_ms: 0,
            },
        }
    }

    fn check_assertion(result: &ActionResult, assertion: &Assertion) -> Result<(), String> {
        match assertion.assertion_type.as_str() {
            "status" => {
                let Some(expected) = assertion.expected.as_ref() else {
                    return Err("Missing expected value for status assertion".to_string());
                };
                let expected_status =
                    expected.as_u64().map_or(0, |v| u16::try_from(v).map_or(0, |status| status));
                if result.status != expected_status {
                    return Err(format!(
                        "Expected status {expected_status}, got {}",
                        result.status
                    ));
                }
            }
            "body_json" => {
                if let Ok(json) = serde_json::from_str::<serde_json::Value>(&result.body) {
                    if let Some(path) = &assertion.path {
                        let value = json.pointer(path);
                        if let Some(expected) = &assertion.expected {
                            if let Some(actual) = value {
                                if actual != expected {
                                    return Err(format!(
                                        "Path {path}: expected {expected}, got {actual}"
                                    ));
                                }
                            }
                        }
                    }
                }
            }
            _ => {}
        }
        Ok(())
    }

    fn extract_value(&mut self, result: &ActionResult, extraction: &Extraction) {
        if let Ok(json) = serde_json::from_str::<serde_json::Value>(&result.body) {
            if let Some(path) = &extraction.path {
                if let Some(value) = json.pointer(path) {
                    self.extracted_values.insert(extraction.name.clone(), value.clone());
                }
            }
        }
    }
}

/// Run validation on a directory of scenarios.
///
/// # Errors
/// Returns an error if reading directory or files fails.
pub async fn run_validation<S: std::hash::BuildHasher + Send + Sync>(
    scenario_dir: &Path,
    application_endpoint: &str,
    twins: HashMap<String, String, S>,
) -> Result<ValidationReport, ScenarioError> {
    let mut results = Vec::new();
    let mut runner = ScenarioRunner::new(application_endpoint, twins);

    let entries = fs::read_dir(scenario_dir)?;
    for entry in entries.flatten() {
        let path = entry.path();
        if path.extension().is_some_and(|ext| ext == "yaml") {
            let content = fs::read_to_string(&path)?;
            let scenario: Scenario = serde_yaml::from_str(&content)?;
            let result = runner.run_scenario(&scenario).await;
            results.push(result);
        }
    }

    let (passed, failed) = results.iter().fold((0, 0), |(passed, failed), result| {
        if result.passed {
            (passed + 1, failed)
        } else {
            (passed, failed + 1)
        }
    });
    let total = passed + failed;

    let category_breakdown: HashMap<_, _> =
        results.iter().fold(HashMap::new(), |mut acc, result| {
            let entry = acc.entry(result.category.clone()).or_insert(CategoryResult {
                total: 0,
                passed: 0,
                failed: 0,
            });
            entry.total += 1;
            if result.passed {
                entry.passed += 1;
            } else {
                entry.failed += 1;
            }
            acc
        });

    Ok(ValidationReport {
        spec_id: "flow-wasm-v1".to_string(),
        total_scenarios: total,
        passed_scenarios: passed,
        failed_scenarios: failed,
        results,
        category_breakdown,
    })
}
