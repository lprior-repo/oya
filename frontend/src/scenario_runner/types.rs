use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use thiserror::Error;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ScenarioIdentity {
    pub id: String,
    #[serde(rename = "spec_ref")]
    pub spec_ref: String,
    #[serde(rename = "spec_version")]
    pub spec_version: String,
    pub category: String,
    pub visibility: String,
    pub priority: String,
    pub description: String,
    pub rationale: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ScenarioSetup {
    pub universe: String,
    #[serde(rename = "initial_state")]
    pub initial_state: String,
    pub preconditions: Vec<Precondition>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Precondition {
    pub description: String,
    pub check: serde_json::Value,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct StepAction {
    #[serde(rename = "type")]
    pub action_type: String,
    pub method: Option<String>,
    pub url: Option<String>,
    pub headers: Option<HashMap<String, String>>,
    pub body: Option<serde_json::Value>,
    pub params: Option<HashMap<String, String>>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Assertion {
    #[serde(rename = "type")]
    pub assertion_type: String,
    pub path: Option<String>,
    pub expected: Option<serde_json::Value>,
    pub operator: Option<String>,
    pub message: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Extraction {
    pub name: String,
    pub from: String,
    pub path: Option<String>,
    pub regex: Option<String>,
    #[serde(rename = "group")]
    pub extract_group: Option<usize>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ScenarioStep {
    pub id: String,
    pub description: String,
    pub action: StepAction,
    pub assertions: Vec<Assertion>,
    pub extractions: Vec<Extraction>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ScenarioTeardown {
    #[serde(rename = "reset_universe")]
    pub reset_universe: bool,
    #[serde(rename = "custom_cleanup")]
    pub custom_cleanup: Option<Vec<serde_json::Value>>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Scenario {
    pub scenario: ScenarioIdentity,
    pub setup: ScenarioSetup,
    pub steps: Vec<ScenarioStep>,
    pub teardown: ScenarioTeardown,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct StepResult {
    pub step_id: String,
    pub passed: bool,
    pub duration_ms: u64,
    pub assertions_passed: usize,
    pub assertions_failed: usize,
    pub error: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ScenarioResult {
    pub scenario_id: String,
    pub spec_ref: String,
    pub category: String,
    pub passed: bool,
    pub steps: Vec<StepResult>,
    pub total_duration_ms: u64,
    pub error: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ValidationReport {
    pub spec_id: String,
    pub total_scenarios: usize,
    pub passed_scenarios: usize,
    pub failed_scenarios: usize,
    pub results: Vec<ScenarioResult>,
    pub category_breakdown: HashMap<String, CategoryResult>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CategoryResult {
    pub total: usize,
    pub passed: usize,
    pub failed: usize,
}

#[derive(Debug, Clone)]
pub struct ActionResult {
    pub status: u16,
    pub body: String,
    pub response_time_ms: u64,
}

#[derive(Debug, Error)]
pub enum ScenarioError {
    #[error("Failed to read scenario file: {0}")]
    ReadError(#[from] std::io::Error),
    #[error("Failed to parse YAML: {0}")]
    ParseError(#[from] serde_yaml::Error),
    #[error("HTTP request failed: {0}")]
    HttpError(#[from] reqwest::Error),
    #[error("Assertion failed: {0}")]
    AssertionFailed(String),
    #[error("Setup failed: {0}")]
    SetupFailed(String),
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[allow(dead_code)]
pub struct ScenarioCategory(String);

#[allow(dead_code)]
impl ScenarioCategory {
    pub fn new(cat: impl Into<String>) -> Self {
        Self(cat.into())
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }

    pub fn security() -> Self {
        Self("security".to_string())
    }

    pub fn error_handling() -> Self {
        Self("error-handling".to_string())
    }

    pub fn happy_path() -> Self {
        Self("happy-path".to_string())
    }
}

impl std::fmt::Display for ScenarioCategory {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[allow(dead_code)]
pub struct HttpMethod(String);

#[allow(dead_code)]
impl HttpMethod {
    pub fn new(method: impl Into<String>) -> Self {
        Self(method.into())
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }

    pub fn get() -> Self {
        Self("GET".to_string())
    }

    pub fn post() -> Self {
        Self("POST".to_string())
    }

    pub fn put() -> Self {
        Self("PUT".to_string())
    }

    pub fn delete() -> Self {
        Self("DELETE".to_string())
    }
}

impl std::fmt::Display for HttpMethod {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}
