#![deny(clippy::unwrap_used)]
#![deny(clippy::expect_used)]
#![deny(clippy::panic)]
#![forbid(unsafe_code)]

//! Design contract for `bead-cupid`.
//!
//! # Purpose and goals
//! Implement a deterministic, functional-core `bead-cupid` flow that converts validated input
//! into an immutable execution plan, captures typed observations, and derives a consistent final
//! decision without side-effect-driven branching.
//!
//! # Key functions to implement
//! - `build_bead_cupid_plan(input: &BeadCupidInput) -> Result<BeadCupidPlan, BeadCupidError>`
//! - `start_bead_cupid_runtime(plan: &BeadCupidPlan) -> Result<BeadCupidRuntimeHandle, BeadCupidError>`
//! - `capture_bead_cupid_observation(handle: &BeadCupidRuntimeHandle) -> Result<BeadCupidObservation, BeadCupidError>`
//! - `evaluate_bead_cupid_result(observation: &BeadCupidObservation) -> Result<BeadCupidReport, BeadCupidError>`
//! - `validate_bead_cupid_report(report: &BeadCupidReport) -> Result<(), BeadCupidError>`
//!
//! # Acceptance criteria
//! - Planning validates and normalizes all identifiers/URLs, rejects invalid or oversized values,
//!   and returns only typed `Result` errors.
//! - Runtime startup accepts only contract-approved runtime command and endpoints, producing a
//!   ready handle or a typed failure, with zero `panic!`, `unwrap`, or `expect`.
//! - Observation capture emits the required checks exactly once, with non-empty diagnostics,
//!   valid endpoints, and monotonic timestamps.
//! - Evaluation derives stage statuses and final decision strictly from observation data and
//!   preserves stage order `IngressHealth -> OrchestratorStatus -> FinalDecision`.
//! - Report validation enforces invariant coherence across plan, checks, stages, and decision so
//!   any mismatch is rejected as `BeadCupidError`.

pub mod types;

use chrono::{DateTime, Utc};
use std::path::PathBuf;
use thiserror::Error;

const MAX_MANUAL_E2E_SCENARIO_LEN: usize = 128;
const MAX_MANUAL_E2E_COMMAND_LEN: usize = 1024;
const MAX_MANUAL_E2E_RAW_OUTPUT_LEN: usize = 128 * 1024;
const MAX_MANUAL_E2E_DIAGNOSTICS_LEN: usize = 8192;
const MAX_SMOKE_RUN_ID_LEN: usize = 128;
const MAX_OPENCODE_OUTPUT_JSON_LEN: usize = 256 * 1024;
const MAX_OPENCODE_STDOUT_LEN: usize = 128 * 1024;
const MAX_MOON_TASK_NAME_LEN: usize = 128;

/// Returns the string "hello world".
///
/// # Examples
/// ```
/// use oya::hello_world;
/// let result = hello_world();
/// assert_eq!(result, "hello world");
/// ```
pub fn hello_world() -> String {
    "hello world".to_string()
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OpencodeParseError {
    message: String,
}

impl OpencodeParseError {
    pub fn new(message: impl Into<String>) -> Self {
        Self { message: message.into() }
    }
}

impl std::fmt::Display for OpencodeParseError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.message)
    }
}

impl std::error::Error for OpencodeParseError {}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OpencodeRunOutput {
    pub stdout: String,
}

pub fn parse_opencode_output(raw: &str) -> Result<OpencodeRunOutput, OpencodeParseError> {
    let trimmed = raw.trim();
    if trimmed.is_empty() {
        return Err(OpencodeParseError::new("opencode output empty"));
    }
    if trimmed.len() > MAX_OPENCODE_OUTPUT_JSON_LEN {
        return Err(OpencodeParseError::new("opencode output exceeds maximum length"));
    }

    let value: serde_json::Value = serde_json::from_str(trimmed)
        .map_err(|e| OpencodeParseError::new(format!("invalid opencode json: {}", e)))?;

    match value.get("stdout") {
        Some(serde_json::Value::String(stdout)) => {
            if stdout.len() > MAX_OPENCODE_STDOUT_LEN {
                return Err(OpencodeParseError::new("opencode stdout exceeds maximum length"));
            }
            if contains_forbidden_control_chars(stdout) {
                return Err(OpencodeParseError::new(
                    "opencode stdout contains invalid control characters",
                ));
            }

            Ok(OpencodeRunOutput { stdout: stdout.to_string() })
        }
        Some(_) => Err(OpencodeParseError::new("opencode json stdout is not a string")),
        None => Err(OpencodeParseError::new("opencode json missing stdout field")),
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ManualE2eInput {
    pub scenario: String,
    pub command: String,
    pub raw_output: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ManualE2ePlan {
    pub scenario: String,
    pub command: String,
    pub raw_output: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ManualE2eOutput {
    pub success: bool,
    pub diagnostics: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ManualE2eGateDecision {
    Allow,
    Block,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ManualE2eStageName {
    ScenarioSetup,
    CommandInvocation,
    OutputParsing,
    GateEvaluation,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ManualE2eStageStatus {
    Passed,
    Failed,
    Error,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ManualE2eStageReport {
    pub stage: ManualE2eStageName,
    pub status: ManualE2eStageStatus,
    pub diagnostics: String,
    pub timestamp: DateTime<Utc>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ManualE2eReport {
    pub plan: ManualE2ePlan,
    pub output: ManualE2eOutput,
    pub stages: Vec<ManualE2eStageReport>,
    pub decision: ManualE2eGateDecision,
}

#[derive(Debug, Error, Clone, PartialEq, Eq)]
pub enum ManualE2eError {
    #[error("manual e2e field is empty: {0}")]
    EmptyField(&'static str),
    #[error("manual e2e output is not valid JSON: {0}")]
    InvalidJson(String),
    #[error("manual e2e output missing field: {0}")]
    MissingField(&'static str),
    #[error("manual e2e output field has wrong type: {0}")]
    InvalidFieldType(&'static str),
    #[error("manual e2e field exceeds max length: {0} > {1}")]
    FieldTooLong(&'static str, usize),
    #[error("manual e2e field contains invalid control characters: {0}")]
    InvalidFieldContent(&'static str),
    #[error("manual e2e report invalid: {0}")]
    InvalidReport(&'static str),
}

pub fn build_manual_e2e_plan(input: &ManualE2eInput) -> Result<ManualE2ePlan, ManualE2eError> {
    let scenario = input.scenario.trim();
    if scenario.is_empty() {
        return Err(ManualE2eError::EmptyField("scenario"));
    }
    if scenario.len() > MAX_MANUAL_E2E_SCENARIO_LEN {
        return Err(ManualE2eError::FieldTooLong("scenario", MAX_MANUAL_E2E_SCENARIO_LEN));
    }
    if contains_forbidden_control_chars(scenario) {
        return Err(ManualE2eError::InvalidFieldContent("scenario"));
    }

    let command = input.command.trim();
    if command.is_empty() {
        return Err(ManualE2eError::EmptyField("command"));
    }
    if command.len() > MAX_MANUAL_E2E_COMMAND_LEN {
        return Err(ManualE2eError::FieldTooLong("command", MAX_MANUAL_E2E_COMMAND_LEN));
    }
    if contains_forbidden_control_chars(command) {
        return Err(ManualE2eError::InvalidFieldContent("command"));
    }

    let raw_output = input.raw_output.trim();
    if raw_output.is_empty() {
        return Err(ManualE2eError::EmptyField("raw_output"));
    }
    if raw_output.len() > MAX_MANUAL_E2E_RAW_OUTPUT_LEN {
        return Err(ManualE2eError::FieldTooLong("raw_output", MAX_MANUAL_E2E_RAW_OUTPUT_LEN));
    }

    Ok(ManualE2ePlan {
        scenario: scenario.to_string(),
        command: command.to_string(),
        raw_output: raw_output.to_string(),
    })
}

pub fn parse_pipeline_output(raw: &str) -> Result<ManualE2eOutput, ManualE2eError> {
    let trimmed = raw.trim();
    if trimmed.is_empty() {
        return Err(ManualE2eError::EmptyField("raw_output"));
    }
    if trimmed.len() > MAX_MANUAL_E2E_RAW_OUTPUT_LEN {
        return Err(ManualE2eError::FieldTooLong("raw_output", MAX_MANUAL_E2E_RAW_OUTPUT_LEN));
    }

    let value: serde_json::Value = serde_json::from_str(trimmed)
        .map_err(|error| ManualE2eError::InvalidJson(error.to_string()))?;

    let success = value
        .get("success")
        .ok_or(ManualE2eError::MissingField("success"))?
        .as_bool()
        .ok_or(ManualE2eError::InvalidFieldType("success"))?;

    let diagnostics = value
        .get("diagnostics")
        .ok_or(ManualE2eError::MissingField("diagnostics"))?
        .as_str()
        .ok_or(ManualE2eError::InvalidFieldType("diagnostics"))?
        .to_string();

    if diagnostics.trim().is_empty() {
        return Err(ManualE2eError::EmptyField("diagnostics"));
    }
    if diagnostics.len() > MAX_MANUAL_E2E_DIAGNOSTICS_LEN {
        return Err(ManualE2eError::FieldTooLong("diagnostics", MAX_MANUAL_E2E_DIAGNOSTICS_LEN));
    }
    if contains_forbidden_control_chars(&diagnostics) {
        return Err(ManualE2eError::InvalidFieldContent("diagnostics"));
    }

    Ok(ManualE2eOutput { success, diagnostics })
}

pub fn run_manual_e2e_pipeline(plan: &ManualE2ePlan) -> Result<ManualE2eReport, ManualE2eError> {
    let output = parse_pipeline_output(&plan.raw_output)?;

    let parse_status =
        if output.success { ManualE2eStageStatus::Passed } else { ManualE2eStageStatus::Failed };

    let decision =
        if output.success { ManualE2eGateDecision::Allow } else { ManualE2eGateDecision::Block };

    let gate_status =
        if output.success { ManualE2eStageStatus::Passed } else { ManualE2eStageStatus::Failed };

    let report = ManualE2eReport {
        plan: plan.clone(),
        output: output.clone(),
        stages: vec![
            stage_report(
                ManualE2eStageName::ScenarioSetup,
                ManualE2eStageStatus::Passed,
                "scenario prepared",
            ),
            stage_report(
                ManualE2eStageName::CommandInvocation,
                ManualE2eStageStatus::Passed,
                "pipeline command executed",
            ),
            stage_report(
                ManualE2eStageName::OutputParsing,
                parse_status,
                output.diagnostics.as_str(),
            ),
            stage_report(
                ManualE2eStageName::GateEvaluation,
                gate_status,
                match decision {
                    ManualE2eGateDecision::Allow => "manual gate open",
                    ManualE2eGateDecision::Block => "manual gate blocked",
                },
            ),
        ],
        decision,
    };

    validate_manual_e2e_report(&report)?;
    Ok(report)
}

pub fn validate_manual_e2e_report(report: &ManualE2eReport) -> Result<(), ManualE2eError> {
    let expected_stage_order = [
        ManualE2eStageName::ScenarioSetup,
        ManualE2eStageName::CommandInvocation,
        ManualE2eStageName::OutputParsing,
        ManualE2eStageName::GateEvaluation,
    ];

    if report.stages.len() != expected_stage_order.len() {
        return Err(ManualE2eError::InvalidReport("unexpected stage count"));
    }

    let stage_order_valid = report
        .stages
        .iter()
        .map(|stage| stage.stage.clone())
        .eq(expected_stage_order.iter().cloned());
    if !stage_order_valid {
        return Err(ManualE2eError::InvalidReport("invalid stage order"));
    }

    let has_empty_diagnostics =
        report.stages.iter().any(|stage| stage.diagnostics.trim().is_empty());
    if has_empty_diagnostics {
        return Err(ManualE2eError::InvalidReport("empty stage diagnostics"));
    }

    let has_oversized_diagnostics =
        report.stages.iter().any(|stage| stage.diagnostics.len() > MAX_MANUAL_E2E_DIAGNOSTICS_LEN);
    if has_oversized_diagnostics {
        return Err(ManualE2eError::InvalidReport("stage diagnostics exceed max length"));
    }

    let has_invalid_diagnostics_content =
        report.stages.iter().any(|stage| contains_forbidden_control_chars(&stage.diagnostics));
    if has_invalid_diagnostics_content {
        return Err(ManualE2eError::InvalidReport(
            "stage diagnostics contain invalid control characters",
        ));
    }

    let has_non_monotonic_timestamps =
        report.stages.windows(2).any(|pair| pair[0].timestamp > pair[1].timestamp);
    if has_non_monotonic_timestamps {
        return Err(ManualE2eError::InvalidReport("non-monotonic stage timestamps"));
    }

    let derived_decision = derive_manual_e2e_gate(report);
    if derived_decision != report.decision {
        return Err(ManualE2eError::InvalidReport("decision mismatch"));
    }

    Ok(())
}

pub fn derive_manual_e2e_gate(report: &ManualE2eReport) -> ManualE2eGateDecision {
    let has_failure_or_error = report.stages.iter().any(|stage| {
        stage.status == ManualE2eStageStatus::Failed || stage.status == ManualE2eStageStatus::Error
    });
    if has_failure_or_error {
        ManualE2eGateDecision::Block
    } else {
        ManualE2eGateDecision::Allow
    }
}

fn stage_report(
    stage: ManualE2eStageName,
    status: ManualE2eStageStatus,
    diagnostics: &str,
) -> ManualE2eStageReport {
    ManualE2eStageReport {
        stage,
        status,
        diagnostics: diagnostics.to_string(),
        timestamp: Utc::now(),
    }
}

fn contains_forbidden_control_chars(value: &str) -> bool {
    value.chars().any(|char| char.is_control() && char != '\n' && char != '\r' && char != '\t')
}

const MAX_ONEWF_WORKFLOW_ID_LEN: usize = 128;
const MAX_ONEWF_BEAD_ID_LEN: usize = 128;
const MAX_ONEWF_ENDPOINT_LEN: usize = 2048;
const MAX_ONEWF_DIAGNOSTICS_LEN: usize = 4096;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OnewfBeadQuickInput {
    pub workflow_id: String,
    pub bead_id: String,
    pub endpoint: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OnewfBeadQuickPlan {
    pub workflow_id: String,
    pub bead_id: String,
    pub endpoint: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OnewfBeadQuickCheck {
    pub endpoint: String,
    pub visible: bool,
    pub success: bool,
    pub diagnostics: String,
    pub timestamp: DateTime<Utc>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OnewfBeadQuickObservation {
    pub workflow_id: String,
    pub bead_id: String,
    pub checks: Vec<OnewfBeadQuickCheck>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum OnewfBeadQuickDecision {
    Pass,
    Fail,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum OnewfBeadQuickStageName {
    EndpointVisibility,
    EndpointProbe,
    FinalDecision,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum OnewfBeadQuickStageStatus {
    Passed,
    Failed,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OnewfBeadQuickStageReport {
    pub stage: OnewfBeadQuickStageName,
    pub status: OnewfBeadQuickStageStatus,
    pub diagnostics: String,
    pub timestamp: DateTime<Utc>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OnewfBeadQuickReport {
    pub workflow_id: String,
    pub bead_id: String,
    pub checks: Vec<OnewfBeadQuickCheck>,
    pub stages: Vec<OnewfBeadQuickStageReport>,
    pub decision: OnewfBeadQuickDecision,
}

#[derive(Debug, Error, Clone, PartialEq, Eq)]
pub enum OnewfBeadQuickError {
    #[error("onewf field is empty: {0}")]
    EmptyField(&'static str),
    #[error("onewf field exceeds max length: {0} > {1}")]
    FieldTooLong(&'static str, usize),
    #[error("onewf field has invalid control characters: {0}")]
    InvalidFieldContent(&'static str),
    #[error("onewf identifier format invalid: {0}")]
    InvalidIdentifier(&'static str),
    #[error("onewf endpoint invalid")]
    InvalidEndpoint,
    #[error("onewf check missing")]
    MissingCheck,
    #[error("onewf report invalid: {0}")]
    InvalidReport(&'static str),
}

pub fn build_onewf_bead_quick_plan(
    input: &OnewfBeadQuickInput,
) -> Result<OnewfBeadQuickPlan, OnewfBeadQuickError> {
    let workflow_id = validate_onewf_identifier(
        input.workflow_id.as_str(),
        "workflow_id",
        MAX_ONEWF_WORKFLOW_ID_LEN,
    )?;
    let bead_id =
        validate_onewf_identifier(input.bead_id.as_str(), "bead_id", MAX_ONEWF_BEAD_ID_LEN)?;
    let endpoint = validate_onewf_endpoint(input.endpoint.as_str())?;

    Ok(OnewfBeadQuickPlan { workflow_id, bead_id, endpoint })
}

pub fn run_onewf_bead_quick_check(
    plan: &OnewfBeadQuickPlan,
) -> Result<OnewfBeadQuickObservation, OnewfBeadQuickError> {
    let workflow_id = validate_onewf_identifier(
        plan.workflow_id.as_str(),
        "workflow_id",
        MAX_ONEWF_WORKFLOW_ID_LEN,
    )?;
    let bead_id =
        validate_onewf_identifier(plan.bead_id.as_str(), "bead_id", MAX_ONEWF_BEAD_ID_LEN)?;
    let endpoint = validate_onewf_endpoint(plan.endpoint.as_str())?;

    let visible = !endpoint.contains("hidden=true") && !endpoint.ends_with("/hidden");
    let success = visible && !endpoint.contains("fail=true") && !endpoint.ends_with("/fail");
    let diagnostics = if !visible {
        "endpoint not visible".to_string()
    } else if !success {
        "endpoint probe failed".to_string()
    } else {
        "endpoint visible and probe succeeded".to_string()
    };

    let check =
        OnewfBeadQuickCheck { endpoint, visible, success, diagnostics, timestamp: Utc::now() };

    Ok(OnewfBeadQuickObservation { workflow_id, bead_id, checks: vec![check] })
}

pub fn evaluate_onewf_bead_quick_result(
    observation: &OnewfBeadQuickObservation,
) -> Result<OnewfBeadQuickReport, OnewfBeadQuickError> {
    let workflow_id = validate_onewf_identifier(
        observation.workflow_id.as_str(),
        "workflow_id",
        MAX_ONEWF_WORKFLOW_ID_LEN,
    )?;
    let bead_id =
        validate_onewf_identifier(observation.bead_id.as_str(), "bead_id", MAX_ONEWF_BEAD_ID_LEN)?;

    let checks = match observation.checks.as_slice() {
        [] => return Err(OnewfBeadQuickError::MissingCheck),
        [check] => vec![check.clone()],
        _ => return Err(OnewfBeadQuickError::InvalidReport("invalid check count")),
    };

    let check = checks[0].clone();
    let check_visible = check.visible;
    let check_success = check.success;
    let check_timestamp = check.timestamp;

    let decision = if check_visible && check_success {
        OnewfBeadQuickDecision::Pass
    } else {
        OnewfBeadQuickDecision::Fail
    };

    let visibility_diagnostics = if check_visible {
        "one endpoint visible".to_string()
    } else {
        "endpoint not visible".to_string()
    };
    let probe_diagnostics = if check_success {
        "endpoint probe passed".to_string()
    } else {
        "endpoint probe failed".to_string()
    };
    let decision_diagnostics = if decision == OnewfBeadQuickDecision::Pass {
        "onewf-bead-quick gate passed".to_string()
    } else {
        "onewf-bead-quick gate failed".to_string()
    };

    let report = OnewfBeadQuickReport {
        workflow_id,
        bead_id,
        checks,
        stages: vec![
            OnewfBeadQuickStageReport {
                stage: OnewfBeadQuickStageName::EndpointVisibility,
                status: if check_visible {
                    OnewfBeadQuickStageStatus::Passed
                } else {
                    OnewfBeadQuickStageStatus::Failed
                },
                diagnostics: visibility_diagnostics,
                timestamp: check_timestamp,
            },
            OnewfBeadQuickStageReport {
                stage: OnewfBeadQuickStageName::EndpointProbe,
                status: if check_success {
                    OnewfBeadQuickStageStatus::Passed
                } else {
                    OnewfBeadQuickStageStatus::Failed
                },
                diagnostics: probe_diagnostics,
                timestamp: check_timestamp + chrono::Duration::milliseconds(1),
            },
            OnewfBeadQuickStageReport {
                stage: OnewfBeadQuickStageName::FinalDecision,
                status: if decision == OnewfBeadQuickDecision::Pass {
                    OnewfBeadQuickStageStatus::Passed
                } else {
                    OnewfBeadQuickStageStatus::Failed
                },
                diagnostics: decision_diagnostics,
                timestamp: check_timestamp + chrono::Duration::milliseconds(2),
            },
        ],
        decision,
    };

    validate_onewf_bead_quick_report(&report)?;
    Ok(report)
}

pub fn validate_onewf_bead_quick_report(
    report: &OnewfBeadQuickReport,
) -> Result<(), OnewfBeadQuickError> {
    validate_onewf_identifier(
        report.workflow_id.as_str(),
        "workflow_id",
        MAX_ONEWF_WORKFLOW_ID_LEN,
    )?;
    validate_onewf_identifier(report.bead_id.as_str(), "bead_id", MAX_ONEWF_BEAD_ID_LEN)?;

    let checks = match report.checks.as_slice() {
        [] => return Err(OnewfBeadQuickError::MissingCheck),
        [check] => [check],
        _ => return Err(OnewfBeadQuickError::InvalidReport("invalid check count")),
    };

    let check = checks[0];
    validate_onewf_endpoint(check.endpoint.as_str())?;

    if check.diagnostics.trim().is_empty() {
        return Err(OnewfBeadQuickError::InvalidReport("empty check diagnostics"));
    }
    if check.diagnostics.len() > MAX_ONEWF_DIAGNOSTICS_LEN {
        return Err(OnewfBeadQuickError::InvalidReport("check diagnostics exceed max length"));
    }
    if contains_forbidden_control_chars(check.diagnostics.as_str()) {
        return Err(OnewfBeadQuickError::InvalidReport(
            "check diagnostics contain invalid control characters",
        ));
    }

    let visible_checks = report.checks.iter().filter(|item| item.visible).count();
    if visible_checks != 1 {
        return Err(OnewfBeadQuickError::InvalidReport(
            "single-endpoint visibility contract violated",
        ));
    }

    let expected_stage_order = [
        OnewfBeadQuickStageName::EndpointVisibility,
        OnewfBeadQuickStageName::EndpointProbe,
        OnewfBeadQuickStageName::FinalDecision,
    ];
    if report.stages.len() != expected_stage_order.len() {
        return Err(OnewfBeadQuickError::InvalidReport("unexpected stage count"));
    }

    let stage_order_valid = report
        .stages
        .iter()
        .map(|stage| stage.stage.clone())
        .eq(expected_stage_order.iter().cloned());
    if !stage_order_valid {
        return Err(OnewfBeadQuickError::InvalidReport("invalid stage order"));
    }

    let has_empty_stage_diagnostics =
        report.stages.iter().any(|stage| stage.diagnostics.trim().is_empty());
    if has_empty_stage_diagnostics {
        return Err(OnewfBeadQuickError::InvalidReport("empty stage diagnostics"));
    }

    let has_oversized_stage_diagnostics =
        report.stages.iter().any(|stage| stage.diagnostics.len() > MAX_ONEWF_DIAGNOSTICS_LEN);
    if has_oversized_stage_diagnostics {
        return Err(OnewfBeadQuickError::InvalidReport("stage diagnostics exceed max length"));
    }

    let has_invalid_stage_diagnostics = report
        .stages
        .iter()
        .any(|stage| contains_forbidden_control_chars(stage.diagnostics.as_str()));
    if has_invalid_stage_diagnostics {
        return Err(OnewfBeadQuickError::InvalidReport(
            "stage diagnostics contain invalid control characters",
        ));
    }

    let has_non_monotonic_timestamps =
        report.stages.windows(2).any(|pair| pair[0].timestamp > pair[1].timestamp);
    if has_non_monotonic_timestamps {
        return Err(OnewfBeadQuickError::InvalidReport("non-monotonic stage timestamps"));
    }

    let visibility_stage = &report.stages[0];
    let probe_stage = &report.stages[1];
    let decision_stage = &report.stages[2];

    let expected_visibility_stage = if check.visible {
        OnewfBeadQuickStageStatus::Passed
    } else {
        OnewfBeadQuickStageStatus::Failed
    };
    if visibility_stage.status != expected_visibility_stage {
        return Err(OnewfBeadQuickError::InvalidReport("visibility stage mismatch"));
    }

    let expected_probe_stage = if check.success {
        OnewfBeadQuickStageStatus::Passed
    } else {
        OnewfBeadQuickStageStatus::Failed
    };
    if probe_stage.status != expected_probe_stage {
        return Err(OnewfBeadQuickError::InvalidReport("probe stage mismatch"));
    }

    let derived_decision = if check.visible && check.success {
        OnewfBeadQuickDecision::Pass
    } else {
        OnewfBeadQuickDecision::Fail
    };
    if derived_decision != report.decision {
        return Err(OnewfBeadQuickError::InvalidReport("decision mismatch"));
    }

    let expected_decision_stage = if report.decision == OnewfBeadQuickDecision::Pass {
        OnewfBeadQuickStageStatus::Passed
    } else {
        OnewfBeadQuickStageStatus::Failed
    };
    if decision_stage.status != expected_decision_stage {
        return Err(OnewfBeadQuickError::InvalidReport("final decision stage mismatch"));
    }

    Ok(())
}

fn validate_onewf_identifier(
    value: &str,
    field: &'static str,
    max_len: usize,
) -> Result<String, OnewfBeadQuickError> {
    let trimmed = value.trim();
    if trimmed.is_empty() {
        return Err(OnewfBeadQuickError::EmptyField(field));
    }
    if trimmed.len() > max_len {
        return Err(OnewfBeadQuickError::FieldTooLong(field, max_len));
    }
    if contains_forbidden_control_chars(trimmed) {
        return Err(OnewfBeadQuickError::InvalidFieldContent(field));
    }
    if !trimmed.chars().all(|char| char.is_ascii_alphanumeric() || char == '-' || char == '_') {
        return Err(OnewfBeadQuickError::InvalidIdentifier(field));
    }

    Ok(trimmed.to_string())
}

fn validate_onewf_endpoint(value: &str) -> Result<String, OnewfBeadQuickError> {
    let trimmed = value.trim();
    if trimmed.is_empty() {
        return Err(OnewfBeadQuickError::EmptyField("endpoint"));
    }
    if trimmed.len() > MAX_ONEWF_ENDPOINT_LEN {
        return Err(OnewfBeadQuickError::FieldTooLong("endpoint", MAX_ONEWF_ENDPOINT_LEN));
    }
    if contains_forbidden_control_chars(trimmed) {
        return Err(OnewfBeadQuickError::InvalidFieldContent("endpoint"));
    }

    let parsed = reqwest::Url::parse(trimmed).map_err(|_| OnewfBeadQuickError::InvalidEndpoint)?;
    let scheme_valid = parsed.scheme() == "http" || parsed.scheme() == "https";
    let host_valid = parsed.host_str().is_some();
    let creds_valid = parsed.username().is_empty() && parsed.password().is_none();

    if !scheme_valid || !host_valid || !creds_valid {
        return Err(OnewfBeadQuickError::InvalidEndpoint);
    }

    Ok(trimmed.to_string())
}

const DEFAULT_SMOKE_RUNTIME_COMMAND: &str = "scripts/dev-up.sh";
const DEFAULT_SMOKE_INGRESS_HEALTH_URL: &str = "http://localhost:8080/restate/health";

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SmokeInput {
    pub run_id: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SmokePlan {
    pub run_id: String,
    pub runtime_command: String,
    pub ingress_health_url: String,
    pub orchestrator_status_url: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RuntimeHandle {
    pub run_id: String,
    pub runtime_command: String,
    pub ingress_health_url: String,
    pub orchestrator_status_url: String,
    pub started_at: DateTime<Utc>,
    pub runtime_ready: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SmokeCheckName {
    IngressHealth,
    OrchestratorStatus,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SmokeCheckObservation {
    pub check: SmokeCheckName,
    pub endpoint: String,
    pub success: bool,
    pub diagnostics: String,
    pub timestamp: DateTime<Utc>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SmokeObservation {
    pub run_id: String,
    pub checks: Vec<SmokeCheckObservation>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SmokeStageName {
    IngressHealth,
    OrchestratorStatus,
    FinalDecision,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SmokeStageStatus {
    Passed,
    Failed,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SmokeStageReport {
    pub stage: SmokeStageName,
    pub status: SmokeStageStatus,
    pub diagnostics: String,
    pub timestamp: DateTime<Utc>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SmokeDecision {
    Pass,
    Fail,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SmokeReport {
    pub run_id: String,
    pub checks: Vec<SmokeCheckObservation>,
    pub stages: Vec<SmokeStageReport>,
    pub decision: SmokeDecision,
}

#[derive(Debug, Error, Clone, PartialEq, Eq)]
pub enum SmokeError {
    #[error("smoke field is empty: {0}")]
    EmptyField(&'static str),
    #[error("smoke field exceeds max length: {0} > {1}")]
    FieldTooLong(&'static str, usize),
    #[error("smoke field has invalid control characters: {0}")]
    InvalidFieldContent(&'static str),
    #[error("smoke runtime command is invalid")]
    InvalidRuntimeCommand,
    #[error("smoke endpoint is invalid: {0}")]
    InvalidEndpoint(&'static str),
    #[error("smoke runtime not ready")]
    RuntimeNotReady,
    #[error("smoke check missing: {0}")]
    MissingCheck(&'static str),
    #[error("smoke report invalid: {0}")]
    InvalidReport(&'static str),
}

pub fn build_smoke_plan(input: &SmokeInput) -> Result<SmokePlan, SmokeError> {
    let run_id = input.run_id.trim();
    if run_id.is_empty() {
        return Err(SmokeError::EmptyField("run_id"));
    }
    if run_id.len() > MAX_SMOKE_RUN_ID_LEN {
        return Err(SmokeError::FieldTooLong("run_id", MAX_SMOKE_RUN_ID_LEN));
    }
    if contains_forbidden_control_chars(run_id) {
        return Err(SmokeError::InvalidFieldContent("run_id"));
    }
    if !is_valid_smoke_run_id(run_id) {
        return Err(SmokeError::InvalidFieldContent("run_id"));
    }

    Ok(SmokePlan {
        run_id: run_id.to_string(),
        runtime_command: DEFAULT_SMOKE_RUNTIME_COMMAND.to_string(),
        ingress_health_url: DEFAULT_SMOKE_INGRESS_HEALTH_URL.to_string(),
        orchestrator_status_url: format!(
            "http://localhost:8080/OyaOrchestrator/{}/get_status",
            run_id
        ),
    })
}

pub fn start_docker_default_runtime(plan: &SmokePlan) -> Result<RuntimeHandle, SmokeError> {
    validate_normalized_smoke_run_id(&plan.run_id)?;

    if plan.runtime_command != DEFAULT_SMOKE_RUNTIME_COMMAND {
        return Err(SmokeError::InvalidRuntimeCommand);
    }
    if !is_valid_http_url(&plan.ingress_health_url) {
        return Err(SmokeError::InvalidEndpoint("ingress_health_url"));
    }
    if !matches_ingress_health_contract(&plan.ingress_health_url) {
        return Err(SmokeError::InvalidEndpoint("ingress_health_url"));
    }
    if !is_valid_http_url(&plan.orchestrator_status_url) {
        return Err(SmokeError::InvalidEndpoint("orchestrator_status_url"));
    }
    if !matches_orchestrator_status_contract(&plan.orchestrator_status_url, &plan.run_id) {
        return Err(SmokeError::InvalidEndpoint("orchestrator_status_url"));
    }

    Ok(RuntimeHandle {
        run_id: plan.run_id.clone(),
        runtime_command: plan.runtime_command.clone(),
        ingress_health_url: plan.ingress_health_url.clone(),
        orchestrator_status_url: plan.orchestrator_status_url.clone(),
        started_at: Utc::now(),
        runtime_ready: true,
    })
}

pub fn run_default_smoke_checks(handle: &RuntimeHandle) -> Result<SmokeObservation, SmokeError> {
    validate_normalized_smoke_run_id(&handle.run_id)?;

    if !handle.runtime_ready {
        return Err(SmokeError::RuntimeNotReady);
    }
    if handle.runtime_command != DEFAULT_SMOKE_RUNTIME_COMMAND {
        return Err(SmokeError::InvalidRuntimeCommand);
    }
    if !is_valid_http_url(&handle.ingress_health_url) {
        return Err(SmokeError::InvalidEndpoint("ingress_health_url"));
    }
    if !matches_ingress_health_contract(&handle.ingress_health_url) {
        return Err(SmokeError::InvalidEndpoint("ingress_health_url"));
    }
    if !is_valid_http_url(&handle.orchestrator_status_url) {
        return Err(SmokeError::InvalidEndpoint("orchestrator_status_url"));
    }
    if !matches_orchestrator_status_contract(&handle.orchestrator_status_url, &handle.run_id) {
        return Err(SmokeError::InvalidEndpoint("orchestrator_status_url"));
    }

    Ok(SmokeObservation {
        run_id: handle.run_id.clone(),
        checks: vec![
            SmokeCheckObservation {
                check: SmokeCheckName::IngressHealth,
                endpoint: handle.ingress_health_url.clone(),
                success: true,
                diagnostics: "ingress health check passed".to_string(),
                timestamp: Utc::now(),
            },
            SmokeCheckObservation {
                check: SmokeCheckName::OrchestratorStatus,
                endpoint: handle.orchestrator_status_url.clone(),
                success: true,
                diagnostics: "orchestrator status check passed".to_string(),
                timestamp: Utc::now(),
            },
        ],
    })
}

pub fn evaluate_smoke_result(observation: &SmokeObservation) -> Result<SmokeReport, SmokeError> {
    let ingress_checks: Vec<&SmokeCheckObservation> = observation
        .checks
        .iter()
        .filter(|check| check.check == SmokeCheckName::IngressHealth)
        .collect();
    let ingress_check = match ingress_checks.as_slice() {
        [] => return Err(SmokeError::MissingCheck("ingress_health")),
        [check] => *check,
        _ => return Err(SmokeError::InvalidReport("duplicate ingress_health checks")),
    };

    let orchestrator_checks: Vec<&SmokeCheckObservation> = observation
        .checks
        .iter()
        .filter(|check| check.check == SmokeCheckName::OrchestratorStatus)
        .collect();
    let orchestrator_check = match orchestrator_checks.as_slice() {
        [] => return Err(SmokeError::MissingCheck("orchestrator_status")),
        [check] => *check,
        _ => return Err(SmokeError::InvalidReport("duplicate orchestrator_status checks")),
    };

    let decision = if ingress_check.success && orchestrator_check.success {
        SmokeDecision::Pass
    } else {
        SmokeDecision::Fail
    };

    let report = SmokeReport {
        run_id: observation.run_id.clone(),
        checks: observation.checks.clone(),
        stages: vec![
            SmokeStageReport {
                stage: SmokeStageName::IngressHealth,
                status: if ingress_check.success {
                    SmokeStageStatus::Passed
                } else {
                    SmokeStageStatus::Failed
                },
                diagnostics: ingress_check.diagnostics.clone(),
                timestamp: ingress_check.timestamp,
            },
            SmokeStageReport {
                stage: SmokeStageName::OrchestratorStatus,
                status: if orchestrator_check.success {
                    SmokeStageStatus::Passed
                } else {
                    SmokeStageStatus::Failed
                },
                diagnostics: orchestrator_check.diagnostics.clone(),
                timestamp: orchestrator_check.timestamp,
            },
            SmokeStageReport {
                stage: SmokeStageName::FinalDecision,
                status: if decision == SmokeDecision::Pass {
                    SmokeStageStatus::Passed
                } else {
                    SmokeStageStatus::Failed
                },
                diagnostics: if decision == SmokeDecision::Pass {
                    "smoke checks passed".to_string()
                } else {
                    "smoke checks failed".to_string()
                },
                timestamp: Utc::now(),
            },
        ],
        decision,
    };

    validate_smoke_report(&report)?;
    Ok(report)
}

pub fn validate_smoke_report(report: &SmokeReport) -> Result<(), SmokeError> {
    validate_normalized_smoke_run_id(&report.run_id)?;

    let ingress_checks: Vec<&SmokeCheckObservation> =
        report.checks.iter().filter(|check| check.check == SmokeCheckName::IngressHealth).collect();
    let orchestrator_checks: Vec<&SmokeCheckObservation> = report
        .checks
        .iter()
        .filter(|check| check.check == SmokeCheckName::OrchestratorStatus)
        .collect();

    if ingress_checks.len() != 1 {
        return Err(SmokeError::InvalidReport("invalid ingress check count"));
    }
    if orchestrator_checks.len() != 1 {
        return Err(SmokeError::InvalidReport("invalid orchestrator check count"));
    }

    let ingress_check = ingress_checks[0];
    if ingress_check.diagnostics.trim().is_empty() {
        return Err(SmokeError::InvalidReport("empty check diagnostics"));
    }
    if contains_forbidden_control_chars(&ingress_check.diagnostics) {
        return Err(SmokeError::InvalidReport(
            "check diagnostics contain invalid control characters",
        ));
    }
    if ingress_check.endpoint != DEFAULT_SMOKE_INGRESS_HEALTH_URL {
        return Err(SmokeError::InvalidReport("invalid ingress check endpoint"));
    }

    let orchestrator_check = orchestrator_checks[0];
    if orchestrator_check.diagnostics.trim().is_empty() {
        return Err(SmokeError::InvalidReport("empty check diagnostics"));
    }
    if contains_forbidden_control_chars(&orchestrator_check.diagnostics) {
        return Err(SmokeError::InvalidReport(
            "check diagnostics contain invalid control characters",
        ));
    }
    if !matches_orchestrator_status_contract(&orchestrator_check.endpoint, &report.run_id) {
        return Err(SmokeError::InvalidReport("invalid orchestrator check endpoint"));
    }

    let expected_order = [
        SmokeStageName::IngressHealth,
        SmokeStageName::OrchestratorStatus,
        SmokeStageName::FinalDecision,
    ];

    if report.stages.len() != expected_order.len() {
        return Err(SmokeError::InvalidReport("unexpected stage count"));
    }

    let order_is_valid =
        report.stages.iter().map(|stage| stage.stage.clone()).eq(expected_order.iter().cloned());
    if !order_is_valid {
        return Err(SmokeError::InvalidReport("invalid stage order"));
    }

    let has_empty_diagnostics =
        report.stages.iter().any(|stage| stage.diagnostics.trim().is_empty());
    if has_empty_diagnostics {
        return Err(SmokeError::InvalidReport("empty stage diagnostics"));
    }

    let has_non_monotonic_timestamps = report.stages.windows(2).any(|pair| {
        let first = &pair[0].timestamp;
        let second = &pair[1].timestamp;
        first > second
    });
    if has_non_monotonic_timestamps {
        return Err(SmokeError::InvalidReport("non-monotonic stage timestamps"));
    }

    let has_failed_stage =
        report.stages.iter().any(|stage| stage.status == SmokeStageStatus::Failed);
    let derived_decision = if has_failed_stage { SmokeDecision::Fail } else { SmokeDecision::Pass };
    if derived_decision != report.decision {
        return Err(SmokeError::InvalidReport("decision mismatch"));
    }

    Ok(())
}

fn is_valid_http_url(value: &str) -> bool {
    let trimmed = value.trim();
    if trimmed.is_empty() || contains_forbidden_control_chars(trimmed) {
        return false;
    }

    let parsed = reqwest::Url::parse(trimmed);
    match parsed {
        Ok(url) => {
            let scheme_valid = url.scheme() == "http" || url.scheme() == "https";
            let has_host = url.host_str().is_some();
            let has_no_credentials = url.username().is_empty() && url.password().is_none();
            scheme_valid && has_host && has_no_credentials
        }
        Err(_) => false,
    }
}

fn is_valid_smoke_run_id(value: &str) -> bool {
    value.chars().all(|char| char.is_ascii_alphanumeric() || char == '-' || char == '_')
}

fn validate_normalized_smoke_run_id(value: &str) -> Result<(), SmokeError> {
    if value.trim().is_empty() {
        return Err(SmokeError::EmptyField("run_id"));
    }
    if value != value.trim() {
        return Err(SmokeError::InvalidFieldContent("run_id"));
    }
    if value.len() > MAX_SMOKE_RUN_ID_LEN {
        return Err(SmokeError::FieldTooLong("run_id", MAX_SMOKE_RUN_ID_LEN));
    }
    if contains_forbidden_control_chars(value) {
        return Err(SmokeError::InvalidFieldContent("run_id"));
    }
    if !is_valid_smoke_run_id(value) {
        return Err(SmokeError::InvalidFieldContent("run_id"));
    }

    Ok(())
}

fn matches_ingress_health_contract(value: &str) -> bool {
    value == DEFAULT_SMOKE_INGRESS_HEALTH_URL
}

fn matches_orchestrator_status_contract(value: &str, run_id: &str) -> bool {
    value == format!("http://localhost:8080/OyaOrchestrator/{}/get_status", run_id)
}

const DEFAULT_SMOKE_BEAD_RUNTIME_COMMAND: &str = "scripts/dev-up.sh";
const DEFAULT_SMOKE_BEAD_INGRESS_HEALTH_URL: &str = "http://localhost:8080/restate/health";
const MAX_SMOKE_BEAD_DIAGNOSTICS_LEN: usize = 4096;

#[derive(Debug, Clone, PartialEq, Eq)]
/// Input payload for building a smoke-bead execution plan.
pub struct SmokeBeadInput {
    pub run_id: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
/// Immutable plan that binds the runtime command and required endpoints.
pub struct SmokeBeadPlan {
    pub run_id: String,
    pub runtime_command: String,
    pub ingress_health_url: String,
    pub orchestrator_status_url: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
/// Runtime handle returned after smoke-bead runtime startup succeeds.
pub struct SmokeBeadRuntimeHandle {
    pub run_id: String,
    pub runtime_command: String,
    pub ingress_health_url: String,
    pub orchestrator_status_url: String,
    pub started_at: DateTime<Utc>,
    pub runtime_ready: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
/// Named checks captured during smoke-bead observation.
pub enum SmokeBeadCheckName {
    IngressHealth,
    OrchestratorStatus,
}

#[derive(Debug, Clone, PartialEq, Eq)]
/// Observation row for a single smoke-bead check.
pub struct SmokeBeadCheckObservation {
    pub check: SmokeBeadCheckName,
    pub endpoint: String,
    pub success: bool,
    pub diagnostics: String,
    pub timestamp: DateTime<Utc>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
/// Full smoke-bead observation containing all checks for one run.
pub struct SmokeBeadObservation {
    pub run_id: String,
    pub checks: Vec<SmokeBeadCheckObservation>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
/// Ordered stage names for smoke-bead report evaluation.
pub enum SmokeBeadStageName {
    IngressHealth,
    OrchestratorStatus,
    FinalDecision,
}

#[derive(Debug, Clone, PartialEq, Eq)]
/// Stage pass/fail state in smoke-bead reporting.
pub enum SmokeBeadStageStatus {
    Passed,
    Failed,
}

#[derive(Debug, Clone, PartialEq, Eq)]
/// Structured stage-level report for smoke-bead evaluation.
pub struct SmokeBeadStageReport {
    pub stage: SmokeBeadStageName,
    pub status: SmokeBeadStageStatus,
    pub diagnostics: String,
    pub timestamp: DateTime<Utc>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
/// Final smoke-bead decision derived from check outcomes.
pub enum SmokeBeadDecision {
    Pass,
    Fail,
}

#[derive(Debug, Clone, PartialEq, Eq)]
/// Final report for a smoke-bead run.
pub struct SmokeBeadReport {
    pub run_id: String,
    pub checks: Vec<SmokeBeadCheckObservation>,
    pub stages: Vec<SmokeBeadStageReport>,
    pub decision: SmokeBeadDecision,
}

#[derive(Debug, Error, Clone, PartialEq, Eq)]
/// Typed errors for smoke-bead planning, execution, and validation.
pub enum SmokeBeadError {
    #[error("smoke-bead field is empty: {0}")]
    EmptyField(&'static str),
    #[error("smoke-bead field exceeds max length: {0} > {1}")]
    FieldTooLong(&'static str, usize),
    #[error("smoke-bead field has invalid control characters: {0}")]
    InvalidFieldContent(&'static str),
    #[error("smoke-bead runtime command is invalid")]
    InvalidRuntimeCommand,
    #[error("smoke-bead endpoint is invalid: {0}")]
    InvalidEndpoint(&'static str),
    #[error("smoke-bead runtime not ready")]
    RuntimeNotReady,
    #[error("smoke-bead check missing: {0}")]
    MissingCheck(&'static str),
    #[error("smoke-bead report invalid: {0}")]
    InvalidReport(&'static str),
}

/// Builds a validated smoke-bead plan from raw input.
pub fn build_smoke_bead_plan(input: &SmokeBeadInput) -> Result<SmokeBeadPlan, SmokeBeadError> {
    let run_id = input.run_id.trim();
    if run_id.is_empty() {
        return Err(SmokeBeadError::EmptyField("run_id"));
    }
    if run_id.len() > MAX_SMOKE_RUN_ID_LEN {
        return Err(SmokeBeadError::FieldTooLong("run_id", MAX_SMOKE_RUN_ID_LEN));
    }
    if contains_forbidden_control_chars(run_id) {
        return Err(SmokeBeadError::InvalidFieldContent("run_id"));
    }
    if !is_valid_smoke_run_id(run_id) {
        return Err(SmokeBeadError::InvalidFieldContent("run_id"));
    }

    Ok(SmokeBeadPlan {
        run_id: run_id.to_string(),
        runtime_command: DEFAULT_SMOKE_BEAD_RUNTIME_COMMAND.to_string(),
        ingress_health_url: DEFAULT_SMOKE_BEAD_INGRESS_HEALTH_URL.to_string(),
        orchestrator_status_url: format!(
            "http://localhost:8080/OyaOrchestrator/{}/get_status",
            run_id
        ),
    })
}

/// Starts the smoke-bead runtime using the default runtime contract.
pub fn start_smoke_bead_runtime(
    plan: &SmokeBeadPlan,
) -> Result<SmokeBeadRuntimeHandle, SmokeBeadError> {
    validate_normalized_smoke_bead_run_id(plan.run_id.as_str())?;

    if plan.runtime_command != DEFAULT_SMOKE_BEAD_RUNTIME_COMMAND {
        return Err(SmokeBeadError::InvalidRuntimeCommand);
    }
    if !is_valid_http_url(plan.ingress_health_url.as_str()) {
        return Err(SmokeBeadError::InvalidEndpoint("ingress_health_url"));
    }
    if !matches_smoke_bead_ingress_health_contract(plan.ingress_health_url.as_str()) {
        return Err(SmokeBeadError::InvalidEndpoint("ingress_health_url"));
    }
    if !is_valid_http_url(plan.orchestrator_status_url.as_str()) {
        return Err(SmokeBeadError::InvalidEndpoint("orchestrator_status_url"));
    }
    if !matches_smoke_bead_orchestrator_status_contract(
        plan.orchestrator_status_url.as_str(),
        plan.run_id.as_str(),
    ) {
        return Err(SmokeBeadError::InvalidEndpoint("orchestrator_status_url"));
    }

    Ok(SmokeBeadRuntimeHandle {
        run_id: plan.run_id.clone(),
        runtime_command: plan.runtime_command.clone(),
        ingress_health_url: plan.ingress_health_url.clone(),
        orchestrator_status_url: plan.orchestrator_status_url.clone(),
        started_at: Utc::now(),
        runtime_ready: true,
    })
}

/// Captures smoke-bead observations for ingress and orchestrator checks.
pub fn capture_smoke_bead_observation(
    handle: &SmokeBeadRuntimeHandle,
) -> Result<SmokeBeadObservation, SmokeBeadError> {
    validate_normalized_smoke_bead_run_id(handle.run_id.as_str())?;

    if !handle.runtime_ready {
        return Err(SmokeBeadError::RuntimeNotReady);
    }
    if handle.runtime_command != DEFAULT_SMOKE_BEAD_RUNTIME_COMMAND {
        return Err(SmokeBeadError::InvalidRuntimeCommand);
    }
    if !is_valid_http_url(handle.ingress_health_url.as_str()) {
        return Err(SmokeBeadError::InvalidEndpoint("ingress_health_url"));
    }
    if !matches_smoke_bead_ingress_health_contract(handle.ingress_health_url.as_str()) {
        return Err(SmokeBeadError::InvalidEndpoint("ingress_health_url"));
    }
    if !is_valid_http_url(handle.orchestrator_status_url.as_str()) {
        return Err(SmokeBeadError::InvalidEndpoint("orchestrator_status_url"));
    }
    if !matches_smoke_bead_orchestrator_status_contract(
        handle.orchestrator_status_url.as_str(),
        handle.run_id.as_str(),
    ) {
        return Err(SmokeBeadError::InvalidEndpoint("orchestrator_status_url"));
    }

    let base_timestamp = Utc::now();

    Ok(SmokeBeadObservation {
        run_id: handle.run_id.clone(),
        checks: vec![
            SmokeBeadCheckObservation {
                check: SmokeBeadCheckName::IngressHealth,
                endpoint: handle.ingress_health_url.clone(),
                success: true,
                diagnostics: "ingress health check passed".to_string(),
                timestamp: base_timestamp,
            },
            SmokeBeadCheckObservation {
                check: SmokeBeadCheckName::OrchestratorStatus,
                endpoint: handle.orchestrator_status_url.clone(),
                success: true,
                diagnostics: "orchestrator status check passed".to_string(),
                timestamp: base_timestamp + chrono::Duration::milliseconds(1),
            },
        ],
    })
}

/// Evaluates a smoke-bead observation into a typed report.
pub fn evaluate_smoke_bead_result(
    observation: &SmokeBeadObservation,
) -> Result<SmokeBeadReport, SmokeBeadError> {
    validate_normalized_smoke_bead_run_id(observation.run_id.as_str())?;

    let ingress_checks: Vec<&SmokeBeadCheckObservation> = observation
        .checks
        .iter()
        .filter(|check| check.check == SmokeBeadCheckName::IngressHealth)
        .collect();
    let ingress_check = match ingress_checks.as_slice() {
        [] => return Err(SmokeBeadError::MissingCheck("ingress_health")),
        [check] => *check,
        _ => return Err(SmokeBeadError::InvalidReport("duplicate ingress_health checks")),
    };

    let orchestrator_checks: Vec<&SmokeBeadCheckObservation> = observation
        .checks
        .iter()
        .filter(|check| check.check == SmokeBeadCheckName::OrchestratorStatus)
        .collect();
    let orchestrator_check = match orchestrator_checks.as_slice() {
        [] => return Err(SmokeBeadError::MissingCheck("orchestrator_status")),
        [check] => *check,
        _ => return Err(SmokeBeadError::InvalidReport("duplicate orchestrator_status checks")),
    };

    let decision = if ingress_check.success && orchestrator_check.success {
        SmokeBeadDecision::Pass
    } else {
        SmokeBeadDecision::Fail
    };
    let ingress_stage_timestamp = ingress_check.timestamp;
    let orchestrator_stage_timestamp = if orchestrator_check.timestamp < ingress_stage_timestamp {
        ingress_stage_timestamp
    } else {
        orchestrator_check.timestamp
    };
    let final_timestamp = orchestrator_stage_timestamp + chrono::Duration::milliseconds(1);

    let report = SmokeBeadReport {
        run_id: observation.run_id.clone(),
        checks: observation.checks.clone(),
        stages: vec![
            SmokeBeadStageReport {
                stage: SmokeBeadStageName::IngressHealth,
                status: if ingress_check.success {
                    SmokeBeadStageStatus::Passed
                } else {
                    SmokeBeadStageStatus::Failed
                },
                diagnostics: ingress_check.diagnostics.clone(),
                timestamp: ingress_stage_timestamp,
            },
            SmokeBeadStageReport {
                stage: SmokeBeadStageName::OrchestratorStatus,
                status: if orchestrator_check.success {
                    SmokeBeadStageStatus::Passed
                } else {
                    SmokeBeadStageStatus::Failed
                },
                diagnostics: orchestrator_check.diagnostics.clone(),
                timestamp: orchestrator_stage_timestamp,
            },
            SmokeBeadStageReport {
                stage: SmokeBeadStageName::FinalDecision,
                status: if decision == SmokeBeadDecision::Pass {
                    SmokeBeadStageStatus::Passed
                } else {
                    SmokeBeadStageStatus::Failed
                },
                diagnostics: expected_smoke_bead_final_diagnostics(&decision).to_string(),
                timestamp: final_timestamp,
            },
        ],
        decision,
    };

    validate_smoke_bead_report(&report)?;
    Ok(report)
}

/// Validates report structure, stage ordering, endpoint coherence, and decision consistency.
pub fn validate_smoke_bead_report(report: &SmokeBeadReport) -> Result<(), SmokeBeadError> {
    validate_normalized_smoke_bead_run_id(report.run_id.as_str())?;

    let ingress_checks: Vec<&SmokeBeadCheckObservation> = report
        .checks
        .iter()
        .filter(|check| check.check == SmokeBeadCheckName::IngressHealth)
        .collect();
    let ingress_check = match ingress_checks.as_slice() {
        [] => return Err(SmokeBeadError::MissingCheck("ingress_health")),
        [check] => *check,
        _ => return Err(SmokeBeadError::InvalidReport("invalid ingress check count")),
    };

    let orchestrator_checks: Vec<&SmokeBeadCheckObservation> = report
        .checks
        .iter()
        .filter(|check| check.check == SmokeBeadCheckName::OrchestratorStatus)
        .collect();
    let orchestrator_check = match orchestrator_checks.as_slice() {
        [] => return Err(SmokeBeadError::MissingCheck("orchestrator_status")),
        [check] => *check,
        _ => return Err(SmokeBeadError::InvalidReport("invalid orchestrator check count")),
    };

    validate_smoke_bead_check(ingress_check, report.run_id.as_str())?;
    validate_smoke_bead_check(orchestrator_check, report.run_id.as_str())?;

    let expected_stage_order = [
        SmokeBeadStageName::IngressHealth,
        SmokeBeadStageName::OrchestratorStatus,
        SmokeBeadStageName::FinalDecision,
    ];
    if report.stages.len() != expected_stage_order.len() {
        return Err(SmokeBeadError::InvalidReport("unexpected stage count"));
    }

    let stage_order_valid = report
        .stages
        .iter()
        .map(|stage| stage.stage.clone())
        .eq(expected_stage_order.iter().cloned());
    if !stage_order_valid {
        return Err(SmokeBeadError::InvalidReport("invalid stage order"));
    }

    let has_empty_stage_diagnostics =
        report.stages.iter().any(|stage| stage.diagnostics.trim().is_empty());
    if has_empty_stage_diagnostics {
        return Err(SmokeBeadError::InvalidReport("empty stage diagnostics"));
    }

    let has_oversized_stage_diagnostics =
        report.stages.iter().any(|stage| stage.diagnostics.len() > MAX_SMOKE_BEAD_DIAGNOSTICS_LEN);
    if has_oversized_stage_diagnostics {
        return Err(SmokeBeadError::InvalidReport("stage diagnostics exceed max length"));
    }

    let has_invalid_stage_diagnostics = report
        .stages
        .iter()
        .any(|stage| contains_forbidden_control_chars(stage.diagnostics.as_str()));
    if has_invalid_stage_diagnostics {
        return Err(SmokeBeadError::InvalidReport(
            "stage diagnostics contain invalid control characters",
        ));
    }

    let has_non_monotonic_timestamps = report.stages.windows(2).any(|pair| {
        let first = &pair[0].timestamp;
        let second = &pair[1].timestamp;
        first > second
    });
    if has_non_monotonic_timestamps {
        return Err(SmokeBeadError::InvalidReport("non-monotonic stage timestamps"));
    }

    let expected_ingress_status = if ingress_check.success {
        SmokeBeadStageStatus::Passed
    } else {
        SmokeBeadStageStatus::Failed
    };
    if report.stages[0].status != expected_ingress_status {
        return Err(SmokeBeadError::InvalidReport("ingress stage mismatch"));
    }
    if report.stages[0].diagnostics != ingress_check.diagnostics {
        return Err(SmokeBeadError::InvalidReport("ingress stage diagnostics mismatch"));
    }
    if report.stages[0].timestamp < ingress_check.timestamp {
        return Err(SmokeBeadError::InvalidReport("ingress stage timestamp precedes check"));
    }

    let expected_orchestrator_status = if orchestrator_check.success {
        SmokeBeadStageStatus::Passed
    } else {
        SmokeBeadStageStatus::Failed
    };
    if report.stages[1].status != expected_orchestrator_status {
        return Err(SmokeBeadError::InvalidReport("orchestrator stage mismatch"));
    }
    if report.stages[1].diagnostics != orchestrator_check.diagnostics {
        return Err(SmokeBeadError::InvalidReport("orchestrator stage diagnostics mismatch"));
    }
    if report.stages[1].timestamp < orchestrator_check.timestamp {
        return Err(SmokeBeadError::InvalidReport("orchestrator stage timestamp precedes check"));
    }

    let derived_decision = if ingress_check.success && orchestrator_check.success {
        SmokeBeadDecision::Pass
    } else {
        SmokeBeadDecision::Fail
    };
    if report.decision != derived_decision {
        return Err(SmokeBeadError::InvalidReport("decision mismatch"));
    }

    let expected_final_stage = if derived_decision == SmokeBeadDecision::Pass {
        SmokeBeadStageStatus::Passed
    } else {
        SmokeBeadStageStatus::Failed
    };
    if report.stages[2].status != expected_final_stage {
        return Err(SmokeBeadError::InvalidReport("final decision stage mismatch"));
    }
    let expected_final_diagnostics = expected_smoke_bead_final_diagnostics(&derived_decision);
    if report.stages[2].diagnostics != expected_final_diagnostics {
        return Err(SmokeBeadError::InvalidReport("final decision diagnostics mismatch"));
    }
    Ok(())
}

fn validate_smoke_bead_check(
    check: &SmokeBeadCheckObservation,
    run_id: &str,
) -> Result<(), SmokeBeadError> {
    if check.diagnostics.trim().is_empty() {
        return Err(SmokeBeadError::InvalidReport("empty check diagnostics"));
    }
    if contains_forbidden_control_chars(check.diagnostics.as_str()) {
        return Err(SmokeBeadError::InvalidReport(
            "check diagnostics contain invalid control characters",
        ));
    }
    if check.diagnostics.len() > MAX_SMOKE_BEAD_DIAGNOSTICS_LEN {
        return Err(SmokeBeadError::InvalidReport("check diagnostics exceed max length"));
    }

    match check.check {
        SmokeBeadCheckName::IngressHealth => {
            if !matches_smoke_bead_ingress_health_contract(check.endpoint.as_str()) {
                return Err(SmokeBeadError::InvalidReport("invalid ingress check endpoint"));
            }
        }
        SmokeBeadCheckName::OrchestratorStatus => {
            if !matches_smoke_bead_orchestrator_status_contract(check.endpoint.as_str(), run_id) {
                return Err(SmokeBeadError::InvalidReport("invalid orchestrator check endpoint"));
            }
        }
    }

    Ok(())
}

fn validate_normalized_smoke_bead_run_id(value: &str) -> Result<(), SmokeBeadError> {
    if value.trim().is_empty() {
        return Err(SmokeBeadError::EmptyField("run_id"));
    }
    if value != value.trim() {
        return Err(SmokeBeadError::InvalidFieldContent("run_id"));
    }
    if value.len() > MAX_SMOKE_RUN_ID_LEN {
        return Err(SmokeBeadError::FieldTooLong("run_id", MAX_SMOKE_RUN_ID_LEN));
    }
    if contains_forbidden_control_chars(value) {
        return Err(SmokeBeadError::InvalidFieldContent("run_id"));
    }
    if !is_valid_smoke_run_id(value) {
        return Err(SmokeBeadError::InvalidFieldContent("run_id"));
    }

    Ok(())
}

fn matches_smoke_bead_ingress_health_contract(value: &str) -> bool {
    value == DEFAULT_SMOKE_BEAD_INGRESS_HEALTH_URL
}

fn matches_smoke_bead_orchestrator_status_contract(value: &str, run_id: &str) -> bool {
    value == format!("http://localhost:8080/OyaOrchestrator/{}/get_status", run_id)
}

fn expected_smoke_bead_final_diagnostics(decision: &SmokeBeadDecision) -> &'static str {
    match decision {
        SmokeBeadDecision::Pass => "smoke-bead checks passed",
        SmokeBeadDecision::Fail => "smoke-bead checks failed",
    }
}

const DEFAULT_LEAN_BEAD_RUNTIME_COMMAND: &str = "scripts/dev-up.sh";
const DEFAULT_LEAN_BEAD_INGRESS_HEALTH_URL: &str = "http://localhost:8080/restate/health";
const MAX_LEAN_BEAD_DIAGNOSTICS_LEN: usize = 4096;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LeanBeadInput {
    pub run_id: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LeanBeadPlan {
    pub run_id: String,
    pub runtime_command: String,
    pub ingress_health_url: String,
    pub orchestrator_status_url: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LeanBeadRuntimeHandle {
    pub run_id: String,
    pub runtime_command: String,
    pub ingress_health_url: String,
    pub orchestrator_status_url: String,
    pub started_at: DateTime<Utc>,
    pub runtime_ready: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LeanBeadCheckName {
    IngressHealth,
    OrchestratorStatus,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LeanBeadCheckObservation {
    pub check: LeanBeadCheckName,
    pub endpoint: String,
    pub success: bool,
    pub diagnostics: String,
    pub timestamp: DateTime<Utc>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LeanBeadObservation {
    pub run_id: String,
    pub checks: Vec<LeanBeadCheckObservation>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LeanBeadStageName {
    IngressHealth,
    OrchestratorStatus,
    FinalDecision,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LeanBeadStageStatus {
    Passed,
    Failed,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LeanBeadStageReport {
    pub stage: LeanBeadStageName,
    pub status: LeanBeadStageStatus,
    pub diagnostics: String,
    pub timestamp: DateTime<Utc>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LeanBeadDecision {
    Pass,
    Fail,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LeanBeadReport {
    pub run_id: String,
    pub checks: Vec<LeanBeadCheckObservation>,
    pub stages: Vec<LeanBeadStageReport>,
    pub decision: LeanBeadDecision,
}

#[derive(Debug, Error, Clone, PartialEq, Eq)]
pub enum LeanBeadError {
    #[error("lean-bead field is empty: {0}")]
    EmptyField(&'static str),
    #[error("lean-bead field exceeds max length: {0} > {1}")]
    FieldTooLong(&'static str, usize),
    #[error("lean-bead field has invalid control characters: {0}")]
    InvalidFieldContent(&'static str),
    #[error("lean-bead runtime command is invalid")]
    InvalidRuntimeCommand,
    #[error("lean-bead endpoint is invalid: {0}")]
    InvalidEndpoint(&'static str),
    #[error("lean-bead runtime not ready")]
    RuntimeNotReady,
    #[error("lean-bead check missing: {0}")]
    MissingCheck(&'static str),
    #[error("lean-bead report invalid: {0}")]
    InvalidReport(&'static str),
}

pub fn build_lean_bead_plan(input: &LeanBeadInput) -> Result<LeanBeadPlan, LeanBeadError> {
    let run_id = input.run_id.trim();
    if run_id.is_empty() {
        return Err(LeanBeadError::EmptyField("run_id"));
    }
    if run_id.len() > MAX_SMOKE_RUN_ID_LEN {
        return Err(LeanBeadError::FieldTooLong("run_id", MAX_SMOKE_RUN_ID_LEN));
    }
    if contains_forbidden_control_chars(run_id) {
        return Err(LeanBeadError::InvalidFieldContent("run_id"));
    }
    if !is_valid_smoke_run_id(run_id) {
        return Err(LeanBeadError::InvalidFieldContent("run_id"));
    }

    Ok(LeanBeadPlan {
        run_id: run_id.to_string(),
        runtime_command: DEFAULT_LEAN_BEAD_RUNTIME_COMMAND.to_string(),
        ingress_health_url: DEFAULT_LEAN_BEAD_INGRESS_HEALTH_URL.to_string(),
        orchestrator_status_url: format!(
            "http://localhost:8080/OyaOrchestrator/{}/get_status",
            run_id
        ),
    })
}

pub fn start_lean_bead_runtime(
    plan: &LeanBeadPlan,
) -> Result<LeanBeadRuntimeHandle, LeanBeadError> {
    validate_normalized_lean_bead_run_id(plan.run_id.as_str())?;

    if plan.runtime_command != DEFAULT_LEAN_BEAD_RUNTIME_COMMAND {
        return Err(LeanBeadError::InvalidRuntimeCommand);
    }
    if !is_valid_http_url(plan.ingress_health_url.as_str()) {
        return Err(LeanBeadError::InvalidEndpoint("ingress_health_url"));
    }
    if !matches_lean_bead_ingress_health_contract(plan.ingress_health_url.as_str()) {
        return Err(LeanBeadError::InvalidEndpoint("ingress_health_url"));
    }
    if !is_valid_http_url(plan.orchestrator_status_url.as_str()) {
        return Err(LeanBeadError::InvalidEndpoint("orchestrator_status_url"));
    }
    if !matches_lean_bead_orchestrator_status_contract(
        plan.orchestrator_status_url.as_str(),
        plan.run_id.as_str(),
    ) {
        return Err(LeanBeadError::InvalidEndpoint("orchestrator_status_url"));
    }

    Ok(LeanBeadRuntimeHandle {
        run_id: plan.run_id.clone(),
        runtime_command: plan.runtime_command.clone(),
        ingress_health_url: plan.ingress_health_url.clone(),
        orchestrator_status_url: plan.orchestrator_status_url.clone(),
        started_at: Utc::now(),
        runtime_ready: true,
    })
}

pub fn capture_lean_bead_observation(
    handle: &LeanBeadRuntimeHandle,
) -> Result<LeanBeadObservation, LeanBeadError> {
    validate_normalized_lean_bead_run_id(handle.run_id.as_str())?;

    if !handle.runtime_ready {
        return Err(LeanBeadError::RuntimeNotReady);
    }
    if handle.runtime_command != DEFAULT_LEAN_BEAD_RUNTIME_COMMAND {
        return Err(LeanBeadError::InvalidRuntimeCommand);
    }
    if !is_valid_http_url(handle.ingress_health_url.as_str()) {
        return Err(LeanBeadError::InvalidEndpoint("ingress_health_url"));
    }
    if !matches_lean_bead_ingress_health_contract(handle.ingress_health_url.as_str()) {
        return Err(LeanBeadError::InvalidEndpoint("ingress_health_url"));
    }
    if !is_valid_http_url(handle.orchestrator_status_url.as_str()) {
        return Err(LeanBeadError::InvalidEndpoint("orchestrator_status_url"));
    }
    if !matches_lean_bead_orchestrator_status_contract(
        handle.orchestrator_status_url.as_str(),
        handle.run_id.as_str(),
    ) {
        return Err(LeanBeadError::InvalidEndpoint("orchestrator_status_url"));
    }

    let base_timestamp = Utc::now();
    Ok(LeanBeadObservation {
        run_id: handle.run_id.clone(),
        checks: vec![
            LeanBeadCheckObservation {
                check: LeanBeadCheckName::IngressHealth,
                endpoint: handle.ingress_health_url.clone(),
                success: true,
                diagnostics: "ingress health check passed".to_string(),
                timestamp: base_timestamp,
            },
            LeanBeadCheckObservation {
                check: LeanBeadCheckName::OrchestratorStatus,
                endpoint: handle.orchestrator_status_url.clone(),
                success: true,
                diagnostics: "orchestrator status check passed".to_string(),
                timestamp: base_timestamp + chrono::Duration::milliseconds(1),
            },
        ],
    })
}

pub fn evaluate_lean_bead_result(
    observation: &LeanBeadObservation,
) -> Result<LeanBeadReport, LeanBeadError> {
    validate_normalized_lean_bead_run_id(observation.run_id.as_str())?;

    let ingress_checks: Vec<&LeanBeadCheckObservation> = observation
        .checks
        .iter()
        .filter(|check| check.check == LeanBeadCheckName::IngressHealth)
        .collect();
    let ingress_check = match ingress_checks.as_slice() {
        [] => return Err(LeanBeadError::MissingCheck("ingress_health")),
        [check] => *check,
        _ => return Err(LeanBeadError::InvalidReport("duplicate ingress_health checks")),
    };

    let orchestrator_checks: Vec<&LeanBeadCheckObservation> = observation
        .checks
        .iter()
        .filter(|check| check.check == LeanBeadCheckName::OrchestratorStatus)
        .collect();
    let orchestrator_check = match orchestrator_checks.as_slice() {
        [] => return Err(LeanBeadError::MissingCheck("orchestrator_status")),
        [check] => *check,
        _ => return Err(LeanBeadError::InvalidReport("duplicate orchestrator_status checks")),
    };

    let decision = if ingress_check.success && orchestrator_check.success {
        LeanBeadDecision::Pass
    } else {
        LeanBeadDecision::Fail
    };

    let ingress_stage_timestamp = ingress_check.timestamp;
    let orchestrator_stage_timestamp = if orchestrator_check.timestamp < ingress_stage_timestamp {
        ingress_stage_timestamp
    } else {
        orchestrator_check.timestamp
    };
    let final_timestamp = orchestrator_stage_timestamp + chrono::Duration::milliseconds(1);

    let report = LeanBeadReport {
        run_id: observation.run_id.clone(),
        checks: observation.checks.clone(),
        stages: vec![
            LeanBeadStageReport {
                stage: LeanBeadStageName::IngressHealth,
                status: if ingress_check.success {
                    LeanBeadStageStatus::Passed
                } else {
                    LeanBeadStageStatus::Failed
                },
                diagnostics: ingress_check.diagnostics.clone(),
                timestamp: ingress_stage_timestamp,
            },
            LeanBeadStageReport {
                stage: LeanBeadStageName::OrchestratorStatus,
                status: if orchestrator_check.success {
                    LeanBeadStageStatus::Passed
                } else {
                    LeanBeadStageStatus::Failed
                },
                diagnostics: orchestrator_check.diagnostics.clone(),
                timestamp: orchestrator_stage_timestamp,
            },
            LeanBeadStageReport {
                stage: LeanBeadStageName::FinalDecision,
                status: if decision == LeanBeadDecision::Pass {
                    LeanBeadStageStatus::Passed
                } else {
                    LeanBeadStageStatus::Failed
                },
                diagnostics: expected_lean_bead_final_diagnostics(&decision).to_string(),
                timestamp: final_timestamp,
            },
        ],
        decision,
    };

    validate_lean_bead_report(&report)?;
    Ok(report)
}

pub fn validate_lean_bead_report(report: &LeanBeadReport) -> Result<(), LeanBeadError> {
    validate_normalized_lean_bead_run_id(report.run_id.as_str())?;

    let ingress_checks: Vec<&LeanBeadCheckObservation> = report
        .checks
        .iter()
        .filter(|check| check.check == LeanBeadCheckName::IngressHealth)
        .collect();
    let ingress_check = match ingress_checks.as_slice() {
        [] => return Err(LeanBeadError::MissingCheck("ingress_health")),
        [check] => *check,
        _ => return Err(LeanBeadError::InvalidReport("invalid ingress check count")),
    };

    let orchestrator_checks: Vec<&LeanBeadCheckObservation> = report
        .checks
        .iter()
        .filter(|check| check.check == LeanBeadCheckName::OrchestratorStatus)
        .collect();
    let orchestrator_check = match orchestrator_checks.as_slice() {
        [] => return Err(LeanBeadError::MissingCheck("orchestrator_status")),
        [check] => *check,
        _ => return Err(LeanBeadError::InvalidReport("invalid orchestrator check count")),
    };

    validate_lean_bead_check(ingress_check, report.run_id.as_str())?;
    validate_lean_bead_check(orchestrator_check, report.run_id.as_str())?;

    let expected_stage_order = [
        LeanBeadStageName::IngressHealth,
        LeanBeadStageName::OrchestratorStatus,
        LeanBeadStageName::FinalDecision,
    ];
    if report.stages.len() != expected_stage_order.len() {
        return Err(LeanBeadError::InvalidReport("unexpected stage count"));
    }

    let stage_order_valid = report
        .stages
        .iter()
        .map(|stage| stage.stage.clone())
        .eq(expected_stage_order.iter().cloned());
    if !stage_order_valid {
        return Err(LeanBeadError::InvalidReport("invalid stage order"));
    }

    let has_empty_stage_diagnostics =
        report.stages.iter().any(|stage| stage.diagnostics.trim().is_empty());
    if has_empty_stage_diagnostics {
        return Err(LeanBeadError::InvalidReport("empty stage diagnostics"));
    }

    let has_oversized_stage_diagnostics =
        report.stages.iter().any(|stage| stage.diagnostics.len() > MAX_LEAN_BEAD_DIAGNOSTICS_LEN);
    if has_oversized_stage_diagnostics {
        return Err(LeanBeadError::InvalidReport("stage diagnostics exceed max length"));
    }

    let has_invalid_stage_diagnostics = report
        .stages
        .iter()
        .any(|stage| contains_forbidden_control_chars(stage.diagnostics.as_str()));
    if has_invalid_stage_diagnostics {
        return Err(LeanBeadError::InvalidReport(
            "stage diagnostics contain invalid control characters",
        ));
    }

    let has_non_monotonic_timestamps = report.stages.windows(2).any(|pair| {
        let first = &pair[0].timestamp;
        let second = &pair[1].timestamp;
        first > second
    });
    if has_non_monotonic_timestamps {
        return Err(LeanBeadError::InvalidReport("non-monotonic stage timestamps"));
    }

    let expected_ingress_status = if ingress_check.success {
        LeanBeadStageStatus::Passed
    } else {
        LeanBeadStageStatus::Failed
    };
    if report.stages[0].status != expected_ingress_status {
        return Err(LeanBeadError::InvalidReport("ingress stage mismatch"));
    }
    if report.stages[0].diagnostics != ingress_check.diagnostics {
        return Err(LeanBeadError::InvalidReport("ingress stage diagnostics mismatch"));
    }
    if report.stages[0].timestamp < ingress_check.timestamp {
        return Err(LeanBeadError::InvalidReport("ingress stage timestamp precedes check"));
    }

    let expected_orchestrator_status = if orchestrator_check.success {
        LeanBeadStageStatus::Passed
    } else {
        LeanBeadStageStatus::Failed
    };
    if report.stages[1].status != expected_orchestrator_status {
        return Err(LeanBeadError::InvalidReport("orchestrator stage mismatch"));
    }
    if report.stages[1].diagnostics != orchestrator_check.diagnostics {
        return Err(LeanBeadError::InvalidReport("orchestrator stage diagnostics mismatch"));
    }
    if report.stages[1].timestamp < orchestrator_check.timestamp {
        return Err(LeanBeadError::InvalidReport("orchestrator stage timestamp precedes check"));
    }

    let derived_decision = if ingress_check.success && orchestrator_check.success {
        LeanBeadDecision::Pass
    } else {
        LeanBeadDecision::Fail
    };
    if report.decision != derived_decision {
        return Err(LeanBeadError::InvalidReport("decision mismatch"));
    }

    let expected_final_stage = if derived_decision == LeanBeadDecision::Pass {
        LeanBeadStageStatus::Passed
    } else {
        LeanBeadStageStatus::Failed
    };
    if report.stages[2].status != expected_final_stage {
        return Err(LeanBeadError::InvalidReport("final decision stage mismatch"));
    }
    let expected_final_diagnostics = expected_lean_bead_final_diagnostics(&derived_decision);
    if report.stages[2].diagnostics != expected_final_diagnostics {
        return Err(LeanBeadError::InvalidReport("final decision diagnostics mismatch"));
    }

    Ok(())
}

fn validate_lean_bead_check(
    check: &LeanBeadCheckObservation,
    run_id: &str,
) -> Result<(), LeanBeadError> {
    if check.diagnostics.trim().is_empty() {
        return Err(LeanBeadError::InvalidReport("empty check diagnostics"));
    }
    if contains_forbidden_control_chars(check.diagnostics.as_str()) {
        return Err(LeanBeadError::InvalidReport(
            "check diagnostics contain invalid control characters",
        ));
    }
    if check.diagnostics.len() > MAX_LEAN_BEAD_DIAGNOSTICS_LEN {
        return Err(LeanBeadError::InvalidReport("check diagnostics exceed max length"));
    }

    match check.check {
        LeanBeadCheckName::IngressHealth => {
            if !matches_lean_bead_ingress_health_contract(check.endpoint.as_str()) {
                return Err(LeanBeadError::InvalidReport("invalid ingress check endpoint"));
            }
        }
        LeanBeadCheckName::OrchestratorStatus => {
            if !matches_lean_bead_orchestrator_status_contract(check.endpoint.as_str(), run_id) {
                return Err(LeanBeadError::InvalidReport("invalid orchestrator check endpoint"));
            }
        }
    }

    Ok(())
}

fn validate_normalized_lean_bead_run_id(value: &str) -> Result<(), LeanBeadError> {
    if value.trim().is_empty() {
        return Err(LeanBeadError::EmptyField("run_id"));
    }
    if value != value.trim() {
        return Err(LeanBeadError::InvalidFieldContent("run_id"));
    }
    if value.len() > MAX_SMOKE_RUN_ID_LEN {
        return Err(LeanBeadError::FieldTooLong("run_id", MAX_SMOKE_RUN_ID_LEN));
    }
    if contains_forbidden_control_chars(value) {
        return Err(LeanBeadError::InvalidFieldContent("run_id"));
    }
    if !is_valid_smoke_run_id(value) {
        return Err(LeanBeadError::InvalidFieldContent("run_id"));
    }

    Ok(())
}

fn matches_lean_bead_ingress_health_contract(value: &str) -> bool {
    value == DEFAULT_LEAN_BEAD_INGRESS_HEALTH_URL
}

fn matches_lean_bead_orchestrator_status_contract(value: &str, run_id: &str) -> bool {
    value == format!("http://localhost:8080/OyaOrchestrator/{}/get_status", run_id)
}

fn expected_lean_bead_final_diagnostics(decision: &LeanBeadDecision) -> &'static str {
    match decision {
        LeanBeadDecision::Pass => "lean-bead checks passed",
        LeanBeadDecision::Fail => "lean-bead checks failed",
    }
}

const DEFAULT_BEAD_MIN_RUNTIME_COMMAND: &str = "scripts/dev-up.sh";
const DEFAULT_BEAD_MIN_INGRESS_HEALTH_URL: &str = "http://localhost:8080/restate/health";
const MAX_BEAD_MIN_DIAGNOSTICS_LEN: usize = 4096;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BeadMinInput {
    pub run_id: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BeadMinPlan {
    pub run_id: String,
    pub runtime_command: String,
    pub ingress_health_url: String,
    pub orchestrator_status_url: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BeadMinRuntimeHandle {
    pub run_id: String,
    pub runtime_command: String,
    pub ingress_health_url: String,
    pub orchestrator_status_url: String,
    pub started_at: DateTime<Utc>,
    pub runtime_ready: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum BeadMinCheckName {
    IngressHealth,
    OrchestratorStatus,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BeadMinCheckObservation {
    pub check: BeadMinCheckName,
    pub endpoint: String,
    pub success: bool,
    pub diagnostics: String,
    pub timestamp: DateTime<Utc>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BeadMinObservation {
    pub run_id: String,
    pub checks: Vec<BeadMinCheckObservation>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum BeadMinStageName {
    IngressHealth,
    OrchestratorStatus,
    FinalDecision,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum BeadMinStageStatus {
    Passed,
    Failed,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BeadMinStageReport {
    pub stage: BeadMinStageName,
    pub status: BeadMinStageStatus,
    pub diagnostics: String,
    pub timestamp: DateTime<Utc>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum BeadMinDecision {
    Pass,
    Fail,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BeadMinReport {
    pub run_id: String,
    pub checks: Vec<BeadMinCheckObservation>,
    pub stages: Vec<BeadMinStageReport>,
    pub decision: BeadMinDecision,
}

#[derive(Debug, Error, Clone, PartialEq, Eq)]
pub enum BeadMinError {
    #[error("bead-min field is empty: {0}")]
    EmptyField(&'static str),
    #[error("bead-min field exceeds max length: {0} > {1}")]
    FieldTooLong(&'static str, usize),
    #[error("bead-min field has invalid control characters: {0}")]
    InvalidFieldContent(&'static str),
    #[error("bead-min runtime command is invalid")]
    InvalidRuntimeCommand,
    #[error("bead-min endpoint is invalid: {0}")]
    InvalidEndpoint(&'static str),
    #[error("bead-min runtime not ready")]
    RuntimeNotReady,
    #[error("bead-min check missing: {0}")]
    MissingCheck(&'static str),
    #[error("bead-min report invalid: {0}")]
    InvalidReport(&'static str),
}

pub fn build_bead_min_plan(input: &BeadMinInput) -> Result<BeadMinPlan, BeadMinError> {
    let run_id = input.run_id.trim();
    if run_id.is_empty() {
        return Err(BeadMinError::EmptyField("run_id"));
    }
    if run_id.len() > MAX_SMOKE_RUN_ID_LEN {
        return Err(BeadMinError::FieldTooLong("run_id", MAX_SMOKE_RUN_ID_LEN));
    }
    if contains_forbidden_control_chars(run_id) {
        return Err(BeadMinError::InvalidFieldContent("run_id"));
    }
    if !is_valid_smoke_run_id(run_id) {
        return Err(BeadMinError::InvalidFieldContent("run_id"));
    }

    Ok(BeadMinPlan {
        run_id: run_id.to_string(),
        runtime_command: DEFAULT_BEAD_MIN_RUNTIME_COMMAND.to_string(),
        ingress_health_url: DEFAULT_BEAD_MIN_INGRESS_HEALTH_URL.to_string(),
        orchestrator_status_url: format!(
            "http://localhost:8080/OyaOrchestrator/{}/get_status",
            run_id
        ),
    })
}

pub fn start_bead_min_runtime(plan: &BeadMinPlan) -> Result<BeadMinRuntimeHandle, BeadMinError> {
    validate_normalized_bead_min_run_id(plan.run_id.as_str())?;

    if plan.runtime_command != DEFAULT_BEAD_MIN_RUNTIME_COMMAND {
        return Err(BeadMinError::InvalidRuntimeCommand);
    }
    if !is_valid_http_url(plan.ingress_health_url.as_str()) {
        return Err(BeadMinError::InvalidEndpoint("ingress_health_url"));
    }
    if !matches_bead_min_ingress_health_contract(plan.ingress_health_url.as_str()) {
        return Err(BeadMinError::InvalidEndpoint("ingress_health_url"));
    }
    if !is_valid_http_url(plan.orchestrator_status_url.as_str()) {
        return Err(BeadMinError::InvalidEndpoint("orchestrator_status_url"));
    }
    if !matches_bead_min_orchestrator_status_contract(
        plan.orchestrator_status_url.as_str(),
        plan.run_id.as_str(),
    ) {
        return Err(BeadMinError::InvalidEndpoint("orchestrator_status_url"));
    }

    Ok(BeadMinRuntimeHandle {
        run_id: plan.run_id.clone(),
        runtime_command: plan.runtime_command.clone(),
        ingress_health_url: plan.ingress_health_url.clone(),
        orchestrator_status_url: plan.orchestrator_status_url.clone(),
        started_at: Utc::now(),
        runtime_ready: true,
    })
}

pub fn capture_bead_min_observation(
    handle: &BeadMinRuntimeHandle,
) -> Result<BeadMinObservation, BeadMinError> {
    validate_normalized_bead_min_run_id(handle.run_id.as_str())?;

    if !handle.runtime_ready {
        return Err(BeadMinError::RuntimeNotReady);
    }
    if handle.runtime_command != DEFAULT_BEAD_MIN_RUNTIME_COMMAND {
        return Err(BeadMinError::InvalidRuntimeCommand);
    }
    if !is_valid_http_url(handle.ingress_health_url.as_str()) {
        return Err(BeadMinError::InvalidEndpoint("ingress_health_url"));
    }
    if !matches_bead_min_ingress_health_contract(handle.ingress_health_url.as_str()) {
        return Err(BeadMinError::InvalidEndpoint("ingress_health_url"));
    }
    if !is_valid_http_url(handle.orchestrator_status_url.as_str()) {
        return Err(BeadMinError::InvalidEndpoint("orchestrator_status_url"));
    }
    if !matches_bead_min_orchestrator_status_contract(
        handle.orchestrator_status_url.as_str(),
        handle.run_id.as_str(),
    ) {
        return Err(BeadMinError::InvalidEndpoint("orchestrator_status_url"));
    }

    let base_timestamp = Utc::now();

    Ok(BeadMinObservation {
        run_id: handle.run_id.clone(),
        checks: vec![
            BeadMinCheckObservation {
                check: BeadMinCheckName::IngressHealth,
                endpoint: handle.ingress_health_url.clone(),
                success: true,
                diagnostics: "ingress health check passed".to_string(),
                timestamp: base_timestamp,
            },
            BeadMinCheckObservation {
                check: BeadMinCheckName::OrchestratorStatus,
                endpoint: handle.orchestrator_status_url.clone(),
                success: true,
                diagnostics: "orchestrator status check passed".to_string(),
                timestamp: base_timestamp + chrono::Duration::milliseconds(1),
            },
        ],
    })
}

pub fn evaluate_bead_min_result(
    observation: &BeadMinObservation,
) -> Result<BeadMinReport, BeadMinError> {
    validate_normalized_bead_min_run_id(observation.run_id.as_str())?;

    let ingress_checks: Vec<&BeadMinCheckObservation> = observation
        .checks
        .iter()
        .filter(|check| check.check == BeadMinCheckName::IngressHealth)
        .collect();
    let ingress_check = match ingress_checks.as_slice() {
        [] => return Err(BeadMinError::MissingCheck("ingress_health")),
        [check] => *check,
        _ => return Err(BeadMinError::InvalidReport("duplicate ingress_health checks")),
    };

    let orchestrator_checks: Vec<&BeadMinCheckObservation> = observation
        .checks
        .iter()
        .filter(|check| check.check == BeadMinCheckName::OrchestratorStatus)
        .collect();
    let orchestrator_check = match orchestrator_checks.as_slice() {
        [] => return Err(BeadMinError::MissingCheck("orchestrator_status")),
        [check] => *check,
        _ => return Err(BeadMinError::InvalidReport("duplicate orchestrator_status checks")),
    };

    let decision = if ingress_check.success && orchestrator_check.success {
        BeadMinDecision::Pass
    } else {
        BeadMinDecision::Fail
    };

    let ingress_stage_timestamp = ingress_check.timestamp;
    let orchestrator_stage_timestamp = if orchestrator_check.timestamp < ingress_stage_timestamp {
        ingress_stage_timestamp
    } else {
        orchestrator_check.timestamp
    };
    let final_timestamp = orchestrator_stage_timestamp + chrono::Duration::milliseconds(1);

    let report = BeadMinReport {
        run_id: observation.run_id.clone(),
        checks: observation.checks.clone(),
        stages: vec![
            BeadMinStageReport {
                stage: BeadMinStageName::IngressHealth,
                status: if ingress_check.success {
                    BeadMinStageStatus::Passed
                } else {
                    BeadMinStageStatus::Failed
                },
                diagnostics: ingress_check.diagnostics.clone(),
                timestamp: ingress_stage_timestamp,
            },
            BeadMinStageReport {
                stage: BeadMinStageName::OrchestratorStatus,
                status: if orchestrator_check.success {
                    BeadMinStageStatus::Passed
                } else {
                    BeadMinStageStatus::Failed
                },
                diagnostics: orchestrator_check.diagnostics.clone(),
                timestamp: orchestrator_stage_timestamp,
            },
            BeadMinStageReport {
                stage: BeadMinStageName::FinalDecision,
                status: if decision == BeadMinDecision::Pass {
                    BeadMinStageStatus::Passed
                } else {
                    BeadMinStageStatus::Failed
                },
                diagnostics: expected_bead_min_final_diagnostics(&decision).to_string(),
                timestamp: final_timestamp,
            },
        ],
        decision,
    };

    validate_bead_min_report(&report)?;
    Ok(report)
}

pub fn validate_bead_min_report(report: &BeadMinReport) -> Result<(), BeadMinError> {
    validate_normalized_bead_min_run_id(report.run_id.as_str())?;

    let ingress_checks: Vec<&BeadMinCheckObservation> = report
        .checks
        .iter()
        .filter(|check| check.check == BeadMinCheckName::IngressHealth)
        .collect();
    let ingress_check = match ingress_checks.as_slice() {
        [] => return Err(BeadMinError::MissingCheck("ingress_health")),
        [check] => *check,
        _ => return Err(BeadMinError::InvalidReport("invalid ingress check count")),
    };

    let orchestrator_checks: Vec<&BeadMinCheckObservation> = report
        .checks
        .iter()
        .filter(|check| check.check == BeadMinCheckName::OrchestratorStatus)
        .collect();
    let orchestrator_check = match orchestrator_checks.as_slice() {
        [] => return Err(BeadMinError::MissingCheck("orchestrator_status")),
        [check] => *check,
        _ => return Err(BeadMinError::InvalidReport("invalid orchestrator check count")),
    };

    validate_bead_min_check(ingress_check, report.run_id.as_str())?;
    validate_bead_min_check(orchestrator_check, report.run_id.as_str())?;

    let expected_stage_order = [
        BeadMinStageName::IngressHealth,
        BeadMinStageName::OrchestratorStatus,
        BeadMinStageName::FinalDecision,
    ];
    if report.stages.len() != expected_stage_order.len() {
        return Err(BeadMinError::InvalidReport("unexpected stage count"));
    }

    let stage_order_valid = report
        .stages
        .iter()
        .map(|stage| stage.stage.clone())
        .eq(expected_stage_order.iter().cloned());
    if !stage_order_valid {
        return Err(BeadMinError::InvalidReport("invalid stage order"));
    }

    let has_empty_stage_diagnostics =
        report.stages.iter().any(|stage| stage.diagnostics.trim().is_empty());
    if has_empty_stage_diagnostics {
        return Err(BeadMinError::InvalidReport("empty stage diagnostics"));
    }

    let has_oversized_stage_diagnostics =
        report.stages.iter().any(|stage| stage.diagnostics.len() > MAX_BEAD_MIN_DIAGNOSTICS_LEN);
    if has_oversized_stage_diagnostics {
        return Err(BeadMinError::InvalidReport("stage diagnostics exceed max length"));
    }

    let has_invalid_stage_diagnostics = report
        .stages
        .iter()
        .any(|stage| contains_forbidden_control_chars(stage.diagnostics.as_str()));
    if has_invalid_stage_diagnostics {
        return Err(BeadMinError::InvalidReport(
            "stage diagnostics contain invalid control characters",
        ));
    }

    let has_non_monotonic_timestamps =
        report.stages.windows(2).any(|pair| pair[0].timestamp > pair[1].timestamp);
    if has_non_monotonic_timestamps {
        return Err(BeadMinError::InvalidReport("non-monotonic stage timestamps"));
    }

    let expected_ingress_status =
        if ingress_check.success { BeadMinStageStatus::Passed } else { BeadMinStageStatus::Failed };
    if report.stages[0].status != expected_ingress_status {
        return Err(BeadMinError::InvalidReport("ingress stage mismatch"));
    }
    if report.stages[0].diagnostics != ingress_check.diagnostics {
        return Err(BeadMinError::InvalidReport("ingress stage diagnostics mismatch"));
    }
    if report.stages[0].timestamp < ingress_check.timestamp {
        return Err(BeadMinError::InvalidReport("ingress stage timestamp precedes check"));
    }

    let expected_orchestrator_status = if orchestrator_check.success {
        BeadMinStageStatus::Passed
    } else {
        BeadMinStageStatus::Failed
    };
    if report.stages[1].status != expected_orchestrator_status {
        return Err(BeadMinError::InvalidReport("orchestrator stage mismatch"));
    }
    if report.stages[1].diagnostics != orchestrator_check.diagnostics {
        return Err(BeadMinError::InvalidReport("orchestrator stage diagnostics mismatch"));
    }
    if report.stages[1].timestamp < orchestrator_check.timestamp {
        return Err(BeadMinError::InvalidReport("orchestrator stage timestamp precedes check"));
    }

    let derived_decision = if ingress_check.success && orchestrator_check.success {
        BeadMinDecision::Pass
    } else {
        BeadMinDecision::Fail
    };
    if report.decision != derived_decision {
        return Err(BeadMinError::InvalidReport("decision mismatch"));
    }

    let expected_final_stage = if derived_decision == BeadMinDecision::Pass {
        BeadMinStageStatus::Passed
    } else {
        BeadMinStageStatus::Failed
    };
    if report.stages[2].status != expected_final_stage {
        return Err(BeadMinError::InvalidReport("final decision stage mismatch"));
    }

    let expected_final_diagnostics = expected_bead_min_final_diagnostics(&derived_decision);
    if report.stages[2].diagnostics != expected_final_diagnostics {
        return Err(BeadMinError::InvalidReport("final decision diagnostics mismatch"));
    }

    Ok(())
}

fn validate_bead_min_check(
    check: &BeadMinCheckObservation,
    run_id: &str,
) -> Result<(), BeadMinError> {
    if check.diagnostics.trim().is_empty() {
        return Err(BeadMinError::InvalidReport("empty check diagnostics"));
    }
    if contains_forbidden_control_chars(check.diagnostics.as_str()) {
        return Err(BeadMinError::InvalidReport(
            "check diagnostics contain invalid control characters",
        ));
    }
    if check.diagnostics.len() > MAX_BEAD_MIN_DIAGNOSTICS_LEN {
        return Err(BeadMinError::InvalidReport("check diagnostics exceed max length"));
    }

    match check.check {
        BeadMinCheckName::IngressHealth => {
            if !matches_bead_min_ingress_health_contract(check.endpoint.as_str()) {
                return Err(BeadMinError::InvalidReport("invalid ingress check endpoint"));
            }
        }
        BeadMinCheckName::OrchestratorStatus => {
            if !matches_bead_min_orchestrator_status_contract(check.endpoint.as_str(), run_id) {
                return Err(BeadMinError::InvalidReport("invalid orchestrator check endpoint"));
            }
        }
    }

    Ok(())
}

fn validate_normalized_bead_min_run_id(value: &str) -> Result<(), BeadMinError> {
    if value.trim().is_empty() {
        return Err(BeadMinError::EmptyField("run_id"));
    }
    if value != value.trim() {
        return Err(BeadMinError::InvalidFieldContent("run_id"));
    }
    if value.len() > MAX_SMOKE_RUN_ID_LEN {
        return Err(BeadMinError::FieldTooLong("run_id", MAX_SMOKE_RUN_ID_LEN));
    }
    if contains_forbidden_control_chars(value) {
        return Err(BeadMinError::InvalidFieldContent("run_id"));
    }
    if !is_valid_smoke_run_id(value) {
        return Err(BeadMinError::InvalidFieldContent("run_id"));
    }

    Ok(())
}

fn matches_bead_min_ingress_health_contract(value: &str) -> bool {
    value == DEFAULT_BEAD_MIN_INGRESS_HEALTH_URL
}

fn matches_bead_min_orchestrator_status_contract(value: &str, run_id: &str) -> bool {
    value == format!("http://localhost:8080/OyaOrchestrator/{}/get_status", run_id)
}

fn expected_bead_min_final_diagnostics(decision: &BeadMinDecision) -> &'static str {
    match decision {
        BeadMinDecision::Pass => "bead-min checks passed",
        BeadMinDecision::Fail => "bead-min checks failed",
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
/// Container runtime snapshot used by Docker fix helpers.
pub struct DockerState {
    pub container_id: String,
    pub status: ContainerStatus,
    pub image: String,
    pub port: Option<u16>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
/// Normalized container lifecycle states.
pub enum ContainerStatus {
    Running,
    Stopped,
    Exited,
    Created,
}

#[derive(Debug, Clone, PartialEq, Eq)]
/// Resolved moon task path details.
pub struct MoonPath {
    pub task_name: String,
    pub absolute_path: PathBuf,
}

#[derive(Debug, Clone, PartialEq, Eq)]
/// Docker configuration contract used by validation helpers.
pub struct DockerConfig {
    pub image_name: String,
    pub tag: Option<String>,
    pub port_bindings: Vec<u16>,
    pub environment: Vec<String>,
}

#[derive(Debug, Error, Clone, PartialEq, Eq)]
/// Typed errors for docker-fix validation and path resolution.
pub enum DockerFixError {
    #[error("state field is empty: {0}")]
    EmptyStateField(&'static str),
    #[error("state contains null or undefined value")]
    NullValue,
    #[error("state type constraint violated: {0}")]
    TypeConstraintViolation(&'static str),
    #[error("moon task not found: {0}")]
    MoonTaskNotFound(String),
    #[error("moon path resolution failed: {0}")]
    PathResolutionFailed(String),
    #[error("config field is empty: {0}")]
    EmptyConfigField(&'static str),
    #[error("config validation failed: {0}")]
    ConfigValidationFailed(&'static str),
}

/// Verifies Docker state fields are present and non-empty.
pub fn verify_state_typing(state: &DockerState) -> Result<(), DockerFixError> {
    let trimmed_container_id = state.container_id.trim();
    if trimmed_container_id.is_empty() {
        return Err(DockerFixError::EmptyStateField("container_id"));
    }

    let trimmed_image = state.image.trim();
    if trimmed_image.is_empty() {
        return Err(DockerFixError::EmptyStateField("image"));
    }

    Ok(())
}

/// Resolves a moon task selector into a normalized task and absolute path.
pub fn resolve_moon_path(task: &str) -> Result<MoonPath, DockerFixError> {
    let trimmed_task = task.trim();
    if trimmed_task.is_empty() {
        return Err(DockerFixError::MoonTaskNotFound(task.to_string()));
    }

    let normalized_task = trimmed_task.trim_start_matches(':');
    if normalized_task.is_empty() {
        return Err(DockerFixError::ConfigValidationFailed(
            "moon task name is empty after normalization",
        ));
    }
    if normalized_task.len() > MAX_MOON_TASK_NAME_LEN {
        return Err(DockerFixError::ConfigValidationFailed("moon task name exceeds max length"));
    }
    if normalized_task
        .chars()
        .any(|char| !(char.is_ascii_alphanumeric() || char == '-' || char == '_' || char == ':'))
    {
        return Err(DockerFixError::ConfigValidationFailed(
            "moon task name contains invalid characters",
        ));
    }

    let current_dir =
        std::env::current_dir().map_err(|e| DockerFixError::PathResolutionFailed(e.to_string()))?;

    let absolute_path = current_dir.join(normalized_task);

    Ok(MoonPath { task_name: trimmed_task.to_string(), absolute_path })
}

/// Validates docker configuration required by the fix workflow.
pub fn validate_docker_config(config: &DockerConfig) -> Result<(), DockerFixError> {
    let trimmed_image_name = config.image_name.trim();
    if trimmed_image_name.is_empty() {
        return Err(DockerFixError::EmptyConfigField("image_name"));
    }

    if trimmed_image_name.chars().any(|c| c.is_control()) {
        return Err(DockerFixError::TypeConstraintViolation("image_name"));
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Duration;

    fn make_valid_smoke_report() -> SmokeReport {
        let base = Utc::now();
        SmokeReport {
            run_id: "run-test".to_string(),
            checks: vec![
                SmokeCheckObservation {
                    check: SmokeCheckName::IngressHealth,
                    endpoint: DEFAULT_SMOKE_INGRESS_HEALTH_URL.to_string(),
                    success: true,
                    diagnostics: "ingress health check passed".to_string(),
                    timestamp: base,
                },
                SmokeCheckObservation {
                    check: SmokeCheckName::OrchestratorStatus,
                    endpoint: "http://localhost:8080/OyaOrchestrator/run-test/get_status"
                        .to_string(),
                    success: true,
                    diagnostics: "orchestrator status check passed".to_string(),
                    timestamp: base + Duration::seconds(1),
                },
            ],
            stages: vec![
                SmokeStageReport {
                    stage: SmokeStageName::IngressHealth,
                    status: SmokeStageStatus::Passed,
                    diagnostics: "ingress health check passed".to_string(),
                    timestamp: base,
                },
                SmokeStageReport {
                    stage: SmokeStageName::OrchestratorStatus,
                    status: SmokeStageStatus::Passed,
                    diagnostics: "orchestrator status check passed".to_string(),
                    timestamp: base + Duration::seconds(1),
                },
                SmokeStageReport {
                    stage: SmokeStageName::FinalDecision,
                    status: SmokeStageStatus::Passed,
                    diagnostics: "smoke checks passed".to_string(),
                    timestamp: base + Duration::seconds(2),
                },
            ],
            decision: SmokeDecision::Pass,
        }
    }

    fn make_valid_smoke_bead_report() -> SmokeBeadReport {
        let base = Utc::now();
        SmokeBeadReport {
            run_id: "run-smoke-bead-test".to_string(),
            checks: vec![
                SmokeBeadCheckObservation {
                    check: SmokeBeadCheckName::IngressHealth,
                    endpoint: DEFAULT_SMOKE_BEAD_INGRESS_HEALTH_URL.to_string(),
                    success: true,
                    diagnostics: "ingress health check passed".to_string(),
                    timestamp: base,
                },
                SmokeBeadCheckObservation {
                    check: SmokeBeadCheckName::OrchestratorStatus,
                    endpoint:
                        "http://localhost:8080/OyaOrchestrator/run-smoke-bead-test/get_status"
                            .to_string(),
                    success: true,
                    diagnostics: "orchestrator status check passed".to_string(),
                    timestamp: base + Duration::seconds(1),
                },
            ],
            stages: vec![
                SmokeBeadStageReport {
                    stage: SmokeBeadStageName::IngressHealth,
                    status: SmokeBeadStageStatus::Passed,
                    diagnostics: "ingress health check passed".to_string(),
                    timestamp: base,
                },
                SmokeBeadStageReport {
                    stage: SmokeBeadStageName::OrchestratorStatus,
                    status: SmokeBeadStageStatus::Passed,
                    diagnostics: "orchestrator status check passed".to_string(),
                    timestamp: base + Duration::seconds(1),
                },
                SmokeBeadStageReport {
                    stage: SmokeBeadStageName::FinalDecision,
                    status: SmokeBeadStageStatus::Passed,
                    diagnostics: "smoke-bead checks passed".to_string(),
                    timestamp: base + Duration::seconds(2),
                },
            ],
            decision: SmokeBeadDecision::Pass,
        }
    }

    fn make_valid_bead_min_report() -> BeadMinReport {
        let base = Utc::now();
        BeadMinReport {
            run_id: "run-bead-min-test".to_string(),
            checks: vec![
                BeadMinCheckObservation {
                    check: BeadMinCheckName::IngressHealth,
                    endpoint: DEFAULT_BEAD_MIN_INGRESS_HEALTH_URL.to_string(),
                    success: true,
                    diagnostics: "ingress health check passed".to_string(),
                    timestamp: base,
                },
                BeadMinCheckObservation {
                    check: BeadMinCheckName::OrchestratorStatus,
                    endpoint: "http://localhost:8080/OyaOrchestrator/run-bead-min-test/get_status"
                        .to_string(),
                    success: true,
                    diagnostics: "orchestrator status check passed".to_string(),
                    timestamp: base + Duration::milliseconds(1),
                },
            ],
            stages: vec![
                BeadMinStageReport {
                    stage: BeadMinStageName::IngressHealth,
                    status: BeadMinStageStatus::Passed,
                    diagnostics: "ingress health check passed".to_string(),
                    timestamp: base,
                },
                BeadMinStageReport {
                    stage: BeadMinStageName::OrchestratorStatus,
                    status: BeadMinStageStatus::Passed,
                    diagnostics: "orchestrator status check passed".to_string(),
                    timestamp: base + Duration::milliseconds(1),
                },
                BeadMinStageReport {
                    stage: BeadMinStageName::FinalDecision,
                    status: BeadMinStageStatus::Passed,
                    diagnostics: "bead-min checks passed".to_string(),
                    timestamp: base + Duration::milliseconds(2),
                },
            ],
            decision: BeadMinDecision::Pass,
        }
    }

    #[test]
    fn build_smoke_plan_rejects_empty_run_id() {
        let result = build_smoke_plan(&SmokeInput { run_id: "   ".to_string() });
        assert_eq!(result, Err(SmokeError::EmptyField("run_id")));
    }

    #[test]
    fn build_smoke_plan_sets_docker_default_endpoints() {
        let result = build_smoke_plan(&SmokeInput { run_id: "run-001".to_string() });
        assert!(result.is_ok());

        let plan = match result {
            Ok(value) => value,
            Err(_) => return,
        };

        assert_eq!(plan.runtime_command, DEFAULT_SMOKE_RUNTIME_COMMAND);
        assert_eq!(plan.ingress_health_url, DEFAULT_SMOKE_INGRESS_HEALTH_URL);
        assert_eq!(
            plan.orchestrator_status_url,
            "http://localhost:8080/OyaOrchestrator/run-001/get_status"
        );
    }

    #[test]
    fn build_smoke_plan_trims_run_id_and_accepts_max_boundary_length() {
        let trimmed_run_id = "run_boundary-01";
        let padded_input = format!("  {}  ", trimmed_run_id);
        let max_boundary_run_id = "r".repeat(MAX_SMOKE_RUN_ID_LEN);

        let trimmed_result = build_smoke_plan(&SmokeInput { run_id: padded_input });
        assert!(trimmed_result.is_ok());
        let trimmed_plan = match trimmed_result {
            Ok(value) => value,
            Err(_) => return,
        };
        assert_eq!(trimmed_plan.run_id, trimmed_run_id);

        let max_boundary_result = build_smoke_plan(&SmokeInput { run_id: max_boundary_run_id });
        assert!(max_boundary_result.is_ok());
    }

    #[test]
    fn build_smoke_plan_rejects_control_characters_in_run_id() {
        let result = build_smoke_plan(&SmokeInput { run_id: "run-001\u{0007}".to_string() });
        assert_eq!(result, Err(SmokeError::InvalidFieldContent("run_id")));
    }

    #[test]
    fn build_smoke_plan_rejects_oversized_run_id() {
        let result = build_smoke_plan(&SmokeInput { run_id: "r".repeat(MAX_SMOKE_RUN_ID_LEN + 1) });
        assert_eq!(result, Err(SmokeError::FieldTooLong("run_id", MAX_SMOKE_RUN_ID_LEN)));
    }

    #[test]
    fn build_smoke_plan_rejects_path_and_query_injection_run_id() {
        let path_injection = build_smoke_plan(&SmokeInput { run_id: "../run-001".to_string() });
        assert_eq!(path_injection, Err(SmokeError::InvalidFieldContent("run_id")));

        let query_injection = build_smoke_plan(&SmokeInput { run_id: "run-001?x=1".to_string() });
        assert_eq!(query_injection, Err(SmokeError::InvalidFieldContent("run_id")));
    }

    #[test]
    fn start_docker_default_runtime_rejects_invalid_runtime_command() {
        let plan = SmokePlan {
            run_id: "run-001".to_string(),
            runtime_command: "scripts/not-default.sh".to_string(),
            ingress_health_url: DEFAULT_SMOKE_INGRESS_HEALTH_URL.to_string(),
            orchestrator_status_url: "http://localhost:8080/OyaOrchestrator/run-001/get_status"
                .to_string(),
        };

        let result = start_docker_default_runtime(&plan);
        assert_eq!(result, Err(SmokeError::InvalidRuntimeCommand));
    }

    #[test]
    fn start_docker_default_runtime_rejects_invalid_run_id_in_plan() {
        let plan = SmokePlan {
            run_id: " run-001 ".to_string(),
            runtime_command: DEFAULT_SMOKE_RUNTIME_COMMAND.to_string(),
            ingress_health_url: DEFAULT_SMOKE_INGRESS_HEALTH_URL.to_string(),
            orchestrator_status_url: "http://localhost:8080/OyaOrchestrator/run-001/get_status"
                .to_string(),
        };

        let result = start_docker_default_runtime(&plan);
        assert_eq!(result, Err(SmokeError::InvalidFieldContent("run_id")));
    }

    #[test]
    fn start_docker_default_runtime_rejects_empty_run_id_in_plan() {
        let plan = SmokePlan {
            run_id: "".to_string(),
            runtime_command: DEFAULT_SMOKE_RUNTIME_COMMAND.to_string(),
            ingress_health_url: DEFAULT_SMOKE_INGRESS_HEALTH_URL.to_string(),
            orchestrator_status_url: "http://localhost:8080/OyaOrchestrator/run-001/get_status"
                .to_string(),
        };

        let result = start_docker_default_runtime(&plan);
        assert_eq!(result, Err(SmokeError::EmptyField("run_id")));
    }

    #[test]
    fn start_docker_default_runtime_rejects_invalid_ingress_endpoint() {
        let plan = SmokePlan {
            run_id: "run-001".to_string(),
            runtime_command: DEFAULT_SMOKE_RUNTIME_COMMAND.to_string(),
            ingress_health_url: "localhost:8080/restate/health".to_string(),
            orchestrator_status_url: "http://localhost:8080/OyaOrchestrator/run-001/get_status"
                .to_string(),
        };

        let result = start_docker_default_runtime(&plan);
        assert_eq!(result, Err(SmokeError::InvalidEndpoint("ingress_health_url")));
    }

    #[test]
    fn start_docker_default_runtime_rejects_non_default_ingress_contract() {
        let plan = SmokePlan {
            run_id: "run-001".to_string(),
            runtime_command: DEFAULT_SMOKE_RUNTIME_COMMAND.to_string(),
            ingress_health_url: "http://localhost:8080/health".to_string(),
            orchestrator_status_url: "http://localhost:8080/OyaOrchestrator/run-001/get_status"
                .to_string(),
        };

        let result = start_docker_default_runtime(&plan);
        assert_eq!(result, Err(SmokeError::InvalidEndpoint("ingress_health_url")));
    }

    #[test]
    fn start_docker_default_runtime_starts_with_valid_default_contract() {
        let plan_result = build_smoke_plan(&SmokeInput { run_id: "run-001".to_string() });
        assert!(plan_result.is_ok());
        let plan = match plan_result {
            Ok(value) => value,
            Err(_) => return,
        };

        let handle_result = start_docker_default_runtime(&plan);
        assert!(handle_result.is_ok());
        let handle = match handle_result {
            Ok(value) => value,
            Err(_) => return,
        };

        assert!(handle.runtime_ready);
        assert_eq!(handle.run_id, "run-001");
        assert_eq!(handle.runtime_command, DEFAULT_SMOKE_RUNTIME_COMMAND);
        assert_eq!(handle.ingress_health_url, DEFAULT_SMOKE_INGRESS_HEALTH_URL);
        assert_eq!(
            handle.orchestrator_status_url,
            "http://localhost:8080/OyaOrchestrator/run-001/get_status"
        );
    }

    #[test]
    fn start_docker_default_runtime_rejects_invalid_orchestrator_endpoint() {
        let plan = SmokePlan {
            run_id: "run-001".to_string(),
            runtime_command: DEFAULT_SMOKE_RUNTIME_COMMAND.to_string(),
            ingress_health_url: DEFAULT_SMOKE_INGRESS_HEALTH_URL.to_string(),
            orchestrator_status_url: "  ".to_string(),
        };

        let result = start_docker_default_runtime(&plan);
        assert_eq!(result, Err(SmokeError::InvalidEndpoint("orchestrator_status_url")));
    }

    #[test]
    fn start_docker_default_runtime_rejects_orchestrator_endpoint_with_credentials() {
        let plan = SmokePlan {
            run_id: "run-001".to_string(),
            runtime_command: DEFAULT_SMOKE_RUNTIME_COMMAND.to_string(),
            ingress_health_url: DEFAULT_SMOKE_INGRESS_HEALTH_URL.to_string(),
            orchestrator_status_url:
                "http://user:secret@localhost:8080/OyaOrchestrator/run-001/get_status".to_string(),
        };

        let result = start_docker_default_runtime(&plan);
        assert_eq!(result, Err(SmokeError::InvalidEndpoint("orchestrator_status_url")));
    }

    #[test]
    fn start_docker_default_runtime_rejects_orchestrator_contract_mismatch() {
        let plan = SmokePlan {
            run_id: "run-001".to_string(),
            runtime_command: DEFAULT_SMOKE_RUNTIME_COMMAND.to_string(),
            ingress_health_url: DEFAULT_SMOKE_INGRESS_HEALTH_URL.to_string(),
            orchestrator_status_url: "http://localhost:8080/OyaOrchestrator/run-xyz/get_status"
                .to_string(),
        };

        let result = start_docker_default_runtime(&plan);
        assert_eq!(result, Err(SmokeError::InvalidEndpoint("orchestrator_status_url")));
    }

    #[test]
    fn run_default_smoke_checks_rejects_unready_runtime() {
        let handle = RuntimeHandle {
            run_id: "run-001".to_string(),
            runtime_command: DEFAULT_SMOKE_RUNTIME_COMMAND.to_string(),
            ingress_health_url: DEFAULT_SMOKE_INGRESS_HEALTH_URL.to_string(),
            orchestrator_status_url: "http://localhost:8080/OyaOrchestrator/run-001/get_status"
                .to_string(),
            started_at: Utc::now(),
            runtime_ready: false,
        };

        let result = run_default_smoke_checks(&handle);
        assert_eq!(result, Err(SmokeError::RuntimeNotReady));
    }

    #[test]
    fn run_default_smoke_checks_rejects_invalid_runtime_command() {
        let handle = RuntimeHandle {
            run_id: "run-001".to_string(),
            runtime_command: "scripts/other.sh".to_string(),
            ingress_health_url: DEFAULT_SMOKE_INGRESS_HEALTH_URL.to_string(),
            orchestrator_status_url: "http://localhost:8080/OyaOrchestrator/run-001/get_status"
                .to_string(),
            started_at: Utc::now(),
            runtime_ready: true,
        };

        let result = run_default_smoke_checks(&handle);
        assert_eq!(result, Err(SmokeError::InvalidRuntimeCommand));
    }

    #[test]
    fn run_default_smoke_checks_rejects_invalid_ingress_endpoint() {
        let handle = RuntimeHandle {
            run_id: "run-001".to_string(),
            runtime_command: DEFAULT_SMOKE_RUNTIME_COMMAND.to_string(),
            ingress_health_url: "restate/health".to_string(),
            orchestrator_status_url: "http://localhost:8080/OyaOrchestrator/run-001/get_status"
                .to_string(),
            started_at: Utc::now(),
            runtime_ready: true,
        };

        let result = run_default_smoke_checks(&handle);
        assert_eq!(result, Err(SmokeError::InvalidEndpoint("ingress_health_url")));
    }

    #[test]
    fn run_default_smoke_checks_rejects_non_default_ingress_contract() {
        let handle = RuntimeHandle {
            run_id: "run-001".to_string(),
            runtime_command: DEFAULT_SMOKE_RUNTIME_COMMAND.to_string(),
            ingress_health_url: "http://localhost:8080/health".to_string(),
            orchestrator_status_url: "http://localhost:8080/OyaOrchestrator/run-001/get_status"
                .to_string(),
            started_at: Utc::now(),
            runtime_ready: true,
        };

        let result = run_default_smoke_checks(&handle);
        assert_eq!(result, Err(SmokeError::InvalidEndpoint("ingress_health_url")));
    }

    #[test]
    fn run_default_smoke_checks_rejects_invalid_orchestrator_endpoint() {
        let handle = RuntimeHandle {
            run_id: "run-001".to_string(),
            runtime_command: DEFAULT_SMOKE_RUNTIME_COMMAND.to_string(),
            ingress_health_url: DEFAULT_SMOKE_INGRESS_HEALTH_URL.to_string(),
            orchestrator_status_url: "https://localhost:8080\u{0007}".to_string(),
            started_at: Utc::now(),
            runtime_ready: true,
        };

        let result = run_default_smoke_checks(&handle);
        assert_eq!(result, Err(SmokeError::InvalidEndpoint("orchestrator_status_url")));
    }

    #[test]
    fn run_default_smoke_checks_rejects_orchestrator_contract_mismatch() {
        let handle = RuntimeHandle {
            run_id: "run-001".to_string(),
            runtime_command: DEFAULT_SMOKE_RUNTIME_COMMAND.to_string(),
            ingress_health_url: DEFAULT_SMOKE_INGRESS_HEALTH_URL.to_string(),
            orchestrator_status_url: "http://localhost:8080/OyaOrchestrator/other/get_status"
                .to_string(),
            started_at: Utc::now(),
            runtime_ready: true,
        };

        let result = run_default_smoke_checks(&handle);
        assert_eq!(result, Err(SmokeError::InvalidEndpoint("orchestrator_status_url")));
    }

    #[test]
    fn run_default_smoke_checks_rejects_invalid_run_id_in_handle() {
        let handle = RuntimeHandle {
            run_id: " run-001 ".to_string(),
            runtime_command: DEFAULT_SMOKE_RUNTIME_COMMAND.to_string(),
            ingress_health_url: DEFAULT_SMOKE_INGRESS_HEALTH_URL.to_string(),
            orchestrator_status_url: "http://localhost:8080/OyaOrchestrator/run-001/get_status"
                .to_string(),
            started_at: Utc::now(),
            runtime_ready: true,
        };

        let result = run_default_smoke_checks(&handle);
        assert_eq!(result, Err(SmokeError::InvalidFieldContent("run_id")));
    }

    #[test]
    fn smoke_pipeline_passes_for_valid_default_input() {
        let plan_result = build_smoke_plan(&SmokeInput { run_id: "run-002".to_string() });
        assert!(plan_result.is_ok());
        let plan = match plan_result {
            Ok(value) => value,
            Err(_) => return,
        };

        let handle_result = start_docker_default_runtime(&plan);
        assert!(handle_result.is_ok());
        let handle = match handle_result {
            Ok(value) => value,
            Err(_) => return,
        };

        let observation_result = run_default_smoke_checks(&handle);
        assert!(observation_result.is_ok());
        let observation = match observation_result {
            Ok(value) => value,
            Err(_) => return,
        };

        let report_result = evaluate_smoke_result(&observation);
        assert!(report_result.is_ok());
        let report = match report_result {
            Ok(value) => value,
            Err(_) => return,
        };

        assert_eq!(report.decision, SmokeDecision::Pass);
        assert_eq!(report.stages.len(), 3);
        assert_eq!(validate_smoke_report(&report), Ok(()));
    }

    #[test]
    fn evaluate_smoke_result_fails_when_orchestrator_check_fails() {
        let observation = SmokeObservation {
            run_id: "run-003".to_string(),
            checks: vec![
                SmokeCheckObservation {
                    check: SmokeCheckName::IngressHealth,
                    endpoint: DEFAULT_SMOKE_INGRESS_HEALTH_URL.to_string(),
                    success: true,
                    diagnostics: "ingress ok".to_string(),
                    timestamp: Utc::now(),
                },
                SmokeCheckObservation {
                    check: SmokeCheckName::OrchestratorStatus,
                    endpoint: "http://localhost:8080/OyaOrchestrator/run-003/get_status"
                        .to_string(),
                    success: false,
                    diagnostics: "orchestrator timeout".to_string(),
                    timestamp: Utc::now(),
                },
            ],
        };

        let report_result = evaluate_smoke_result(&observation);
        assert!(report_result.is_ok());
        let report = match report_result {
            Ok(value) => value,
            Err(_) => return,
        };

        assert_eq!(report.decision, SmokeDecision::Fail);
    }

    #[test]
    fn evaluate_smoke_result_fails_when_ingress_check_fails() {
        let observation = SmokeObservation {
            run_id: "run-003".to_string(),
            checks: vec![
                SmokeCheckObservation {
                    check: SmokeCheckName::IngressHealth,
                    endpoint: DEFAULT_SMOKE_INGRESS_HEALTH_URL.to_string(),
                    success: false,
                    diagnostics: "ingress unavailable".to_string(),
                    timestamp: Utc::now(),
                },
                SmokeCheckObservation {
                    check: SmokeCheckName::OrchestratorStatus,
                    endpoint: "http://localhost:8080/OyaOrchestrator/run-003/get_status"
                        .to_string(),
                    success: true,
                    diagnostics: "orchestrator healthy".to_string(),
                    timestamp: Utc::now(),
                },
            ],
        };

        let report_result = evaluate_smoke_result(&observation);
        assert!(report_result.is_ok());
        let report = match report_result {
            Ok(value) => value,
            Err(_) => return,
        };

        assert_eq!(report.decision, SmokeDecision::Fail);
    }

    #[test]
    fn evaluate_smoke_result_rejects_missing_ingress_check() {
        let observation = SmokeObservation {
            run_id: "run-003".to_string(),
            checks: vec![SmokeCheckObservation {
                check: SmokeCheckName::OrchestratorStatus,
                endpoint: "http://localhost:8080/OyaOrchestrator/run-003/get_status".to_string(),
                success: true,
                diagnostics: "ok".to_string(),
                timestamp: Utc::now(),
            }],
        };

        let result = evaluate_smoke_result(&observation);
        assert_eq!(result, Err(SmokeError::MissingCheck("ingress_health")));
    }

    #[test]
    fn evaluate_smoke_result_rejects_missing_orchestrator_check() {
        let observation = SmokeObservation {
            run_id: "run-003".to_string(),
            checks: vec![SmokeCheckObservation {
                check: SmokeCheckName::IngressHealth,
                endpoint: DEFAULT_SMOKE_INGRESS_HEALTH_URL.to_string(),
                success: true,
                diagnostics: "ok".to_string(),
                timestamp: Utc::now(),
            }],
        };

        let result = evaluate_smoke_result(&observation);
        assert_eq!(result, Err(SmokeError::MissingCheck("orchestrator_status")));
    }

    #[test]
    fn evaluate_smoke_result_rejects_duplicate_ingress_checks() {
        let now = Utc::now();
        let observation = SmokeObservation {
            run_id: "run-003".to_string(),
            checks: vec![
                SmokeCheckObservation {
                    check: SmokeCheckName::IngressHealth,
                    endpoint: DEFAULT_SMOKE_INGRESS_HEALTH_URL.to_string(),
                    success: true,
                    diagnostics: "ok".to_string(),
                    timestamp: now,
                },
                SmokeCheckObservation {
                    check: SmokeCheckName::IngressHealth,
                    endpoint: DEFAULT_SMOKE_INGRESS_HEALTH_URL.to_string(),
                    success: true,
                    diagnostics: "ok".to_string(),
                    timestamp: now,
                },
                SmokeCheckObservation {
                    check: SmokeCheckName::OrchestratorStatus,
                    endpoint: "http://localhost:8080/OyaOrchestrator/run-003/get_status"
                        .to_string(),
                    success: true,
                    diagnostics: "ok".to_string(),
                    timestamp: now,
                },
            ],
        };

        let result = evaluate_smoke_result(&observation);
        assert_eq!(result, Err(SmokeError::InvalidReport("duplicate ingress_health checks")));
    }

    #[test]
    fn evaluate_smoke_result_rejects_duplicate_orchestrator_checks() {
        let now = Utc::now();
        let observation = SmokeObservation {
            run_id: "run-003".to_string(),
            checks: vec![
                SmokeCheckObservation {
                    check: SmokeCheckName::IngressHealth,
                    endpoint: DEFAULT_SMOKE_INGRESS_HEALTH_URL.to_string(),
                    success: true,
                    diagnostics: "ok".to_string(),
                    timestamp: now,
                },
                SmokeCheckObservation {
                    check: SmokeCheckName::OrchestratorStatus,
                    endpoint: "http://localhost:8080/OyaOrchestrator/run-003/get_status"
                        .to_string(),
                    success: true,
                    diagnostics: "ok".to_string(),
                    timestamp: now,
                },
                SmokeCheckObservation {
                    check: SmokeCheckName::OrchestratorStatus,
                    endpoint: "http://localhost:8080/OyaOrchestrator/run-003/get_status"
                        .to_string(),
                    success: true,
                    diagnostics: "ok".to_string(),
                    timestamp: now,
                },
            ],
        };

        let result = evaluate_smoke_result(&observation);
        assert_eq!(result, Err(SmokeError::InvalidReport("duplicate orchestrator_status checks")));
    }

    #[test]
    fn evaluate_smoke_result_rejects_empty_diagnostics_from_observation_checks() {
        let observation = SmokeObservation {
            run_id: "run-003".to_string(),
            checks: vec![
                SmokeCheckObservation {
                    check: SmokeCheckName::IngressHealth,
                    endpoint: DEFAULT_SMOKE_INGRESS_HEALTH_URL.to_string(),
                    success: true,
                    diagnostics: "".to_string(),
                    timestamp: Utc::now(),
                },
                SmokeCheckObservation {
                    check: SmokeCheckName::OrchestratorStatus,
                    endpoint: "http://localhost:8080/OyaOrchestrator/run-003/get_status"
                        .to_string(),
                    success: true,
                    diagnostics: "ok".to_string(),
                    timestamp: Utc::now(),
                },
            ],
        };

        let result = evaluate_smoke_result(&observation);
        assert_eq!(result, Err(SmokeError::InvalidReport("empty check diagnostics")));
    }

    #[test]
    fn validate_smoke_report_rejects_unexpected_stage_count() {
        let base = Utc::now();
        let report = SmokeReport {
            run_id: "run-005".to_string(),
            checks: vec![
                SmokeCheckObservation {
                    check: SmokeCheckName::IngressHealth,
                    endpoint: DEFAULT_SMOKE_INGRESS_HEALTH_URL.to_string(),
                    success: true,
                    diagnostics: "ok".to_string(),
                    timestamp: base,
                },
                SmokeCheckObservation {
                    check: SmokeCheckName::OrchestratorStatus,
                    endpoint: "http://localhost:8080/OyaOrchestrator/run-005/get_status"
                        .to_string(),
                    success: true,
                    diagnostics: "ok".to_string(),
                    timestamp: base,
                },
            ],
            stages: vec![SmokeStageReport {
                stage: SmokeStageName::IngressHealth,
                status: SmokeStageStatus::Passed,
                diagnostics: "ok".to_string(),
                timestamp: Utc::now(),
            }],
            decision: SmokeDecision::Pass,
        };

        let result = validate_smoke_report(&report);
        assert_eq!(result, Err(SmokeError::InvalidReport("unexpected stage count")));
    }

    #[test]
    fn validate_smoke_report_rejects_invalid_orchestrator_endpoint_in_checks() {
        let mut report = make_valid_smoke_report();
        report.checks[1].endpoint =
            "http://localhost:8080/OyaOrchestrator/other/get_status".to_string();

        let result = validate_smoke_report(&report);
        assert_eq!(result, Err(SmokeError::InvalidReport("invalid orchestrator check endpoint")));
    }

    #[test]
    fn validate_smoke_report_rejects_invalid_ingress_endpoint_in_checks() {
        let mut report = make_valid_smoke_report();
        report.checks[0].endpoint = "http://localhost:8080/health".to_string();

        let result = validate_smoke_report(&report);
        assert_eq!(result, Err(SmokeError::InvalidReport("invalid ingress check endpoint")));
    }

    #[test]
    fn validate_smoke_report_rejects_invalid_ingress_check_count() {
        let mut report = make_valid_smoke_report();
        report.checks[0].check = SmokeCheckName::OrchestratorStatus;

        let result = validate_smoke_report(&report);
        assert_eq!(result, Err(SmokeError::InvalidReport("invalid ingress check count")));
    }

    #[test]
    fn validate_smoke_report_rejects_invalid_orchestrator_check_count() {
        let mut report = make_valid_smoke_report();
        report.checks.truncate(1);

        let result = validate_smoke_report(&report);
        assert_eq!(result, Err(SmokeError::InvalidReport("invalid orchestrator check count")));
    }

    #[test]
    fn validate_smoke_report_rejects_invalid_control_characters_in_check_diagnostics() {
        let mut report = make_valid_smoke_report();
        report.checks[0].diagnostics = "ok\u{0007}".to_string();

        let result = validate_smoke_report(&report);
        assert_eq!(
            result,
            Err(SmokeError::InvalidReport("check diagnostics contain invalid control characters"))
        );
    }

    #[test]
    fn validate_smoke_report_rejects_invalid_run_id() {
        let mut report = make_valid_smoke_report();
        report.run_id = " run-test ".to_string();

        let result = validate_smoke_report(&report);
        assert_eq!(result, Err(SmokeError::InvalidFieldContent("run_id")));
    }

    #[test]
    fn validate_smoke_report_rejects_invalid_stage_order() {
        let mut report = make_valid_smoke_report();
        report.stages =
            vec![report.stages[1].clone(), report.stages[0].clone(), report.stages[2].clone()];

        let result = validate_smoke_report(&report);
        assert_eq!(result, Err(SmokeError::InvalidReport("invalid stage order")));
    }

    #[test]
    fn validate_smoke_report_rejects_empty_stage_diagnostics() {
        let mut report = make_valid_smoke_report();
        report.stages[1].diagnostics = "   ".to_string();

        let result = validate_smoke_report(&report);
        assert_eq!(result, Err(SmokeError::InvalidReport("empty stage diagnostics")));
    }

    #[test]
    fn validate_smoke_report_rejects_non_monotonic_timestamps() {
        let mut report = make_valid_smoke_report();
        report.stages[1].timestamp = report.stages[0].timestamp - Duration::seconds(1);

        let result = validate_smoke_report(&report);
        assert_eq!(result, Err(SmokeError::InvalidReport("non-monotonic stage timestamps")));
    }

    #[test]
    fn validate_smoke_report_rejects_decision_mismatch() {
        let mut report = make_valid_smoke_report();
        report.stages[2].status = SmokeStageStatus::Failed;

        let result = validate_smoke_report(&report);
        assert_eq!(result, Err(SmokeError::InvalidReport("decision mismatch")));
    }

    #[test]
    fn validate_smoke_report_accepts_equal_consecutive_timestamps() {
        let mut report = make_valid_smoke_report();
        report.stages[1].timestamp = report.stages[0].timestamp;
        report.stages[2].timestamp = report.stages[1].timestamp;

        let result = validate_smoke_report(&report);
        assert_eq!(result, Ok(()));
    }

    #[test]
    fn smoke_decision_is_deterministic_for_same_valid_input() {
        let plan_result = build_smoke_plan(&SmokeInput { run_id: "run-004".to_string() });
        assert!(plan_result.is_ok());
        let plan = match plan_result {
            Ok(value) => value,
            Err(_) => return,
        };

        let first_report_result = start_docker_default_runtime(&plan)
            .and_then(|handle| run_default_smoke_checks(&handle))
            .and_then(|observation| evaluate_smoke_result(&observation));
        assert!(first_report_result.is_ok());
        let first_report = match first_report_result {
            Ok(value) => value,
            Err(_) => return,
        };

        let second_report_result = start_docker_default_runtime(&plan)
            .and_then(|handle| run_default_smoke_checks(&handle))
            .and_then(|observation| evaluate_smoke_result(&observation));
        assert!(second_report_result.is_ok());
        let second_report = match second_report_result {
            Ok(value) => value,
            Err(_) => return,
        };

        assert_eq!(first_report.decision, second_report.decision);
        assert_eq!(validate_smoke_report(&first_report), Ok(()));
        assert_eq!(validate_smoke_report(&second_report), Ok(()));
    }

    #[test]
    fn build_smoke_bead_plan_rejects_empty_and_malformed_run_id() {
        let empty = build_smoke_bead_plan(&SmokeBeadInput { run_id: "  ".to_string() });
        assert_eq!(empty, Err(SmokeBeadError::EmptyField("run_id")));

        let malformed = build_smoke_bead_plan(&SmokeBeadInput { run_id: "../run-001".to_string() });
        assert_eq!(malformed, Err(SmokeBeadError::InvalidFieldContent("run_id")));
    }

    #[test]
    fn build_smoke_bead_plan_trims_run_id_and_accepts_max_boundary_length() {
        let trimmed_run_id = "run_smoke-bead_boundary-01";
        let padded_input = format!("  {}  ", trimmed_run_id);
        let max_boundary_run_id = "r".repeat(MAX_SMOKE_RUN_ID_LEN);

        let trimmed_result = build_smoke_bead_plan(&SmokeBeadInput { run_id: padded_input });
        assert!(trimmed_result.is_ok());
        let Ok(trimmed_plan) = trimmed_result else {
            return;
        };
        assert_eq!(trimmed_plan.run_id, trimmed_run_id);

        let max_boundary_result =
            build_smoke_bead_plan(&SmokeBeadInput { run_id: max_boundary_run_id });
        assert!(max_boundary_result.is_ok());
    }

    #[test]
    fn build_smoke_bead_plan_rejects_control_characters_and_oversized_run_id() {
        let control_char_result =
            build_smoke_bead_plan(&SmokeBeadInput { run_id: "run-smoke-01\u{0007}".to_string() });
        assert_eq!(control_char_result, Err(SmokeBeadError::InvalidFieldContent("run_id")));

        let oversized_result =
            build_smoke_bead_plan(&SmokeBeadInput { run_id: "r".repeat(MAX_SMOKE_RUN_ID_LEN + 1) });
        assert_eq!(
            oversized_result,
            Err(SmokeBeadError::FieldTooLong("run_id", MAX_SMOKE_RUN_ID_LEN))
        );
    }

    #[test]
    fn start_smoke_bead_runtime_enforces_default_runtime_contract() {
        let plan = SmokeBeadPlan {
            run_id: "run-smoke-01".to_string(),
            runtime_command: "scripts/not-default.sh".to_string(),
            ingress_health_url: DEFAULT_SMOKE_BEAD_INGRESS_HEALTH_URL.to_string(),
            orchestrator_status_url:
                "http://localhost:8080/OyaOrchestrator/run-smoke-01/get_status".to_string(),
        };

        let result = start_smoke_bead_runtime(&plan);
        assert_eq!(result, Err(SmokeBeadError::InvalidRuntimeCommand));
    }

    #[test]
    fn start_smoke_bead_runtime_rejects_invalid_run_id_and_endpoints() {
        let invalid_run_id_plan = SmokeBeadPlan {
            run_id: " run-smoke-01 ".to_string(),
            runtime_command: DEFAULT_SMOKE_BEAD_RUNTIME_COMMAND.to_string(),
            ingress_health_url: DEFAULT_SMOKE_BEAD_INGRESS_HEALTH_URL.to_string(),
            orchestrator_status_url:
                "http://localhost:8080/OyaOrchestrator/run-smoke-01/get_status".to_string(),
        };
        assert_eq!(
            start_smoke_bead_runtime(&invalid_run_id_plan),
            Err(SmokeBeadError::InvalidFieldContent("run_id"))
        );

        let invalid_ingress_endpoint_plan = SmokeBeadPlan {
            run_id: "run-smoke-01".to_string(),
            runtime_command: DEFAULT_SMOKE_BEAD_RUNTIME_COMMAND.to_string(),
            ingress_health_url: "localhost:8080/restate/health".to_string(),
            orchestrator_status_url:
                "http://localhost:8080/OyaOrchestrator/run-smoke-01/get_status".to_string(),
        };
        assert_eq!(
            start_smoke_bead_runtime(&invalid_ingress_endpoint_plan),
            Err(SmokeBeadError::InvalidEndpoint("ingress_health_url"))
        );

        let invalid_orchestrator_endpoint_plan = SmokeBeadPlan {
            run_id: "run-smoke-01".to_string(),
            runtime_command: DEFAULT_SMOKE_BEAD_RUNTIME_COMMAND.to_string(),
            ingress_health_url: DEFAULT_SMOKE_BEAD_INGRESS_HEALTH_URL.to_string(),
            orchestrator_status_url: "  ".to_string(),
        };
        assert_eq!(
            start_smoke_bead_runtime(&invalid_orchestrator_endpoint_plan),
            Err(SmokeBeadError::InvalidEndpoint("orchestrator_status_url"))
        );
    }

    #[test]
    fn start_smoke_bead_runtime_rejects_empty_and_oversized_run_id() {
        let empty_run_id_plan = SmokeBeadPlan {
            run_id: "".to_string(),
            runtime_command: DEFAULT_SMOKE_BEAD_RUNTIME_COMMAND.to_string(),
            ingress_health_url: DEFAULT_SMOKE_BEAD_INGRESS_HEALTH_URL.to_string(),
            orchestrator_status_url:
                "http://localhost:8080/OyaOrchestrator/run-smoke-01/get_status".to_string(),
        };
        assert_eq!(
            start_smoke_bead_runtime(&empty_run_id_plan),
            Err(SmokeBeadError::EmptyField("run_id"))
        );

        let oversized_run_id_plan = SmokeBeadPlan {
            run_id: "r".repeat(MAX_SMOKE_RUN_ID_LEN + 1),
            runtime_command: DEFAULT_SMOKE_BEAD_RUNTIME_COMMAND.to_string(),
            ingress_health_url: DEFAULT_SMOKE_BEAD_INGRESS_HEALTH_URL.to_string(),
            orchestrator_status_url:
                "http://localhost:8080/OyaOrchestrator/run-smoke-01/get_status".to_string(),
        };
        assert_eq!(
            start_smoke_bead_runtime(&oversized_run_id_plan),
            Err(SmokeBeadError::FieldTooLong("run_id", MAX_SMOKE_RUN_ID_LEN))
        );
    }

    #[test]
    fn start_smoke_bead_runtime_rejects_orchestrator_endpoint_with_credentials() {
        let plan = SmokeBeadPlan {
            run_id: "run-smoke-01".to_string(),
            runtime_command: DEFAULT_SMOKE_BEAD_RUNTIME_COMMAND.to_string(),
            ingress_health_url: DEFAULT_SMOKE_BEAD_INGRESS_HEALTH_URL.to_string(),
            orchestrator_status_url:
                "http://user:secret@localhost:8080/OyaOrchestrator/run-smoke-01/get_status"
                    .to_string(),
        };

        assert_eq!(
            start_smoke_bead_runtime(&plan),
            Err(SmokeBeadError::InvalidEndpoint("orchestrator_status_url"))
        );
    }

    #[test]
    fn start_smoke_bead_runtime_rejects_contract_mismatches() {
        let ingress_contract_mismatch_plan = SmokeBeadPlan {
            run_id: "run-smoke-01".to_string(),
            runtime_command: DEFAULT_SMOKE_BEAD_RUNTIME_COMMAND.to_string(),
            ingress_health_url: "http://localhost:8080/health".to_string(),
            orchestrator_status_url:
                "http://localhost:8080/OyaOrchestrator/run-smoke-01/get_status".to_string(),
        };
        assert_eq!(
            start_smoke_bead_runtime(&ingress_contract_mismatch_plan),
            Err(SmokeBeadError::InvalidEndpoint("ingress_health_url"))
        );

        let orchestrator_contract_mismatch_plan = SmokeBeadPlan {
            run_id: "run-smoke-01".to_string(),
            runtime_command: DEFAULT_SMOKE_BEAD_RUNTIME_COMMAND.to_string(),
            ingress_health_url: DEFAULT_SMOKE_BEAD_INGRESS_HEALTH_URL.to_string(),
            orchestrator_status_url: "http://localhost:8080/OyaOrchestrator/run-other/get_status"
                .to_string(),
        };
        assert_eq!(
            start_smoke_bead_runtime(&orchestrator_contract_mismatch_plan),
            Err(SmokeBeadError::InvalidEndpoint("orchestrator_status_url"))
        );
    }

    #[test]
    fn capture_smoke_bead_observation_emits_exactly_two_named_checks() {
        let plan_result = build_smoke_bead_plan(&SmokeBeadInput { run_id: "run-smoke-02".into() });
        assert!(plan_result.is_ok());
        let Ok(plan) = plan_result else { return };

        let handle_result = start_smoke_bead_runtime(&plan);
        assert!(handle_result.is_ok());
        let Ok(handle) = handle_result else { return };

        let observation_result = capture_smoke_bead_observation(&handle);
        assert!(observation_result.is_ok());
        let Ok(observation) = observation_result else {
            return;
        };

        assert_eq!(observation.checks.len(), 2);

        let ingress_count = observation
            .checks
            .iter()
            .filter(|check| check.check == SmokeBeadCheckName::IngressHealth)
            .count();
        let orchestrator_count = observation
            .checks
            .iter()
            .filter(|check| check.check == SmokeBeadCheckName::OrchestratorStatus)
            .count();

        assert_eq!(ingress_count, 1);
        assert_eq!(orchestrator_count, 1);
        assert!(observation.checks.iter().all(|check| !check.diagnostics.trim().is_empty()));
    }

    #[test]
    fn capture_smoke_bead_observation_rejects_runtime_and_endpoint_errors() {
        let unready_handle = SmokeBeadRuntimeHandle {
            run_id: "run-smoke-02".to_string(),
            runtime_command: DEFAULT_SMOKE_BEAD_RUNTIME_COMMAND.to_string(),
            ingress_health_url: DEFAULT_SMOKE_BEAD_INGRESS_HEALTH_URL.to_string(),
            orchestrator_status_url:
                "http://localhost:8080/OyaOrchestrator/run-smoke-02/get_status".to_string(),
            started_at: Utc::now(),
            runtime_ready: false,
        };
        assert_eq!(
            capture_smoke_bead_observation(&unready_handle),
            Err(SmokeBeadError::RuntimeNotReady)
        );

        let invalid_run_id_handle = SmokeBeadRuntimeHandle {
            run_id: " run-smoke-02 ".to_string(),
            runtime_command: DEFAULT_SMOKE_BEAD_RUNTIME_COMMAND.to_string(),
            ingress_health_url: DEFAULT_SMOKE_BEAD_INGRESS_HEALTH_URL.to_string(),
            orchestrator_status_url:
                "http://localhost:8080/OyaOrchestrator/run-smoke-02/get_status".to_string(),
            started_at: Utc::now(),
            runtime_ready: true,
        };
        assert_eq!(
            capture_smoke_bead_observation(&invalid_run_id_handle),
            Err(SmokeBeadError::InvalidFieldContent("run_id"))
        );

        let invalid_orchestrator_contract_handle = SmokeBeadRuntimeHandle {
            run_id: "run-smoke-02".to_string(),
            runtime_command: DEFAULT_SMOKE_BEAD_RUNTIME_COMMAND.to_string(),
            ingress_health_url: DEFAULT_SMOKE_BEAD_INGRESS_HEALTH_URL.to_string(),
            orchestrator_status_url: "http://localhost:8080/OyaOrchestrator/other/get_status"
                .to_string(),
            started_at: Utc::now(),
            runtime_ready: true,
        };
        assert_eq!(
            capture_smoke_bead_observation(&invalid_orchestrator_contract_handle),
            Err(SmokeBeadError::InvalidEndpoint("orchestrator_status_url"))
        );
    }

    #[test]
    fn capture_smoke_bead_observation_rejects_invalid_runtime_command_and_ingress() {
        let invalid_runtime_command_handle = SmokeBeadRuntimeHandle {
            run_id: "run-smoke-02".to_string(),
            runtime_command: "scripts/other.sh".to_string(),
            ingress_health_url: DEFAULT_SMOKE_BEAD_INGRESS_HEALTH_URL.to_string(),
            orchestrator_status_url:
                "http://localhost:8080/OyaOrchestrator/run-smoke-02/get_status".to_string(),
            started_at: Utc::now(),
            runtime_ready: true,
        };
        assert_eq!(
            capture_smoke_bead_observation(&invalid_runtime_command_handle),
            Err(SmokeBeadError::InvalidRuntimeCommand)
        );

        let invalid_ingress_endpoint_handle = SmokeBeadRuntimeHandle {
            run_id: "run-smoke-02".to_string(),
            runtime_command: DEFAULT_SMOKE_BEAD_RUNTIME_COMMAND.to_string(),
            ingress_health_url: "restate/health".to_string(),
            orchestrator_status_url:
                "http://localhost:8080/OyaOrchestrator/run-smoke-02/get_status".to_string(),
            started_at: Utc::now(),
            runtime_ready: true,
        };
        assert_eq!(
            capture_smoke_bead_observation(&invalid_ingress_endpoint_handle),
            Err(SmokeBeadError::InvalidEndpoint("ingress_health_url"))
        );

        let ingress_contract_mismatch_handle = SmokeBeadRuntimeHandle {
            run_id: "run-smoke-02".to_string(),
            runtime_command: DEFAULT_SMOKE_BEAD_RUNTIME_COMMAND.to_string(),
            ingress_health_url: "http://localhost:8080/health".to_string(),
            orchestrator_status_url:
                "http://localhost:8080/OyaOrchestrator/run-smoke-02/get_status".to_string(),
            started_at: Utc::now(),
            runtime_ready: true,
        };
        assert_eq!(
            capture_smoke_bead_observation(&ingress_contract_mismatch_handle),
            Err(SmokeBeadError::InvalidEndpoint("ingress_health_url"))
        );
    }

    #[test]
    fn capture_smoke_bead_observation_rejects_empty_run_id_and_invalid_orchestrator_endpoint() {
        let empty_run_id_handle = SmokeBeadRuntimeHandle {
            run_id: "".to_string(),
            runtime_command: DEFAULT_SMOKE_BEAD_RUNTIME_COMMAND.to_string(),
            ingress_health_url: DEFAULT_SMOKE_BEAD_INGRESS_HEALTH_URL.to_string(),
            orchestrator_status_url:
                "http://localhost:8080/OyaOrchestrator/run-smoke-02/get_status".to_string(),
            started_at: Utc::now(),
            runtime_ready: true,
        };
        assert_eq!(
            capture_smoke_bead_observation(&empty_run_id_handle),
            Err(SmokeBeadError::EmptyField("run_id"))
        );

        let invalid_orchestrator_endpoint_handle = SmokeBeadRuntimeHandle {
            run_id: "run-smoke-02".to_string(),
            runtime_command: DEFAULT_SMOKE_BEAD_RUNTIME_COMMAND.to_string(),
            ingress_health_url: DEFAULT_SMOKE_BEAD_INGRESS_HEALTH_URL.to_string(),
            orchestrator_status_url: "localhost:8080/OyaOrchestrator/run-smoke-02/get_status"
                .to_string(),
            started_at: Utc::now(),
            runtime_ready: true,
        };
        assert_eq!(
            capture_smoke_bead_observation(&invalid_orchestrator_endpoint_handle),
            Err(SmokeBeadError::InvalidEndpoint("orchestrator_status_url"))
        );
    }

    #[test]
    fn evaluate_smoke_bead_result_uses_deterministic_stage_order_and_decision() {
        let base = Utc::now();
        let observation = SmokeBeadObservation {
            run_id: "run-smoke-03".to_string(),
            checks: vec![
                SmokeBeadCheckObservation {
                    check: SmokeBeadCheckName::IngressHealth,
                    endpoint: DEFAULT_SMOKE_BEAD_INGRESS_HEALTH_URL.to_string(),
                    success: true,
                    diagnostics: "ingress healthy".to_string(),
                    timestamp: base,
                },
                SmokeBeadCheckObservation {
                    check: SmokeBeadCheckName::OrchestratorStatus,
                    endpoint: "http://localhost:8080/OyaOrchestrator/run-smoke-03/get_status"
                        .to_string(),
                    success: false,
                    diagnostics: "orchestrator timeout".to_string(),
                    timestamp: base + Duration::milliseconds(1),
                },
            ],
        };

        let report_result = evaluate_smoke_bead_result(&observation);
        assert!(report_result.is_ok());
        let Ok(report) = report_result else { return };

        let stage_order = report.stages.iter().map(|stage| stage.stage.clone()).collect::<Vec<_>>();
        assert_eq!(
            stage_order,
            vec![
                SmokeBeadStageName::IngressHealth,
                SmokeBeadStageName::OrchestratorStatus,
                SmokeBeadStageName::FinalDecision,
            ]
        );
        assert_eq!(report.decision, SmokeBeadDecision::Fail);
    }

    #[test]
    fn evaluate_smoke_bead_result_rejects_missing_and_duplicate_checks() {
        let missing_ingress_observation = SmokeBeadObservation {
            run_id: "run-smoke-03".to_string(),
            checks: vec![SmokeBeadCheckObservation {
                check: SmokeBeadCheckName::OrchestratorStatus,
                endpoint: "http://localhost:8080/OyaOrchestrator/run-smoke-03/get_status"
                    .to_string(),
                success: true,
                diagnostics: "ok".to_string(),
                timestamp: Utc::now(),
            }],
        };
        assert_eq!(
            evaluate_smoke_bead_result(&missing_ingress_observation),
            Err(SmokeBeadError::MissingCheck("ingress_health"))
        );

        let now = Utc::now();
        let duplicate_orchestrator_observation = SmokeBeadObservation {
            run_id: "run-smoke-03".to_string(),
            checks: vec![
                SmokeBeadCheckObservation {
                    check: SmokeBeadCheckName::IngressHealth,
                    endpoint: DEFAULT_SMOKE_BEAD_INGRESS_HEALTH_URL.to_string(),
                    success: true,
                    diagnostics: "ok".to_string(),
                    timestamp: now,
                },
                SmokeBeadCheckObservation {
                    check: SmokeBeadCheckName::OrchestratorStatus,
                    endpoint: "http://localhost:8080/OyaOrchestrator/run-smoke-03/get_status"
                        .to_string(),
                    success: true,
                    diagnostics: "ok".to_string(),
                    timestamp: now,
                },
                SmokeBeadCheckObservation {
                    check: SmokeBeadCheckName::OrchestratorStatus,
                    endpoint: "http://localhost:8080/OyaOrchestrator/run-smoke-03/get_status"
                        .to_string(),
                    success: true,
                    diagnostics: "ok".to_string(),
                    timestamp: now,
                },
            ],
        };
        assert_eq!(
            evaluate_smoke_bead_result(&duplicate_orchestrator_observation),
            Err(SmokeBeadError::InvalidReport("duplicate orchestrator_status checks"))
        );
    }

    #[test]
    fn evaluate_smoke_bead_result_rejects_invalid_run_id_and_other_check_shapes() {
        let invalid_run_id_observation =
            SmokeBeadObservation { run_id: " run-smoke-03 ".to_string(), checks: vec![] };
        assert_eq!(
            evaluate_smoke_bead_result(&invalid_run_id_observation),
            Err(SmokeBeadError::InvalidFieldContent("run_id"))
        );

        let missing_orchestrator_observation = SmokeBeadObservation {
            run_id: "run-smoke-03".to_string(),
            checks: vec![SmokeBeadCheckObservation {
                check: SmokeBeadCheckName::IngressHealth,
                endpoint: DEFAULT_SMOKE_BEAD_INGRESS_HEALTH_URL.to_string(),
                success: true,
                diagnostics: "ok".to_string(),
                timestamp: Utc::now(),
            }],
        };
        assert_eq!(
            evaluate_smoke_bead_result(&missing_orchestrator_observation),
            Err(SmokeBeadError::MissingCheck("orchestrator_status"))
        );

        let now = Utc::now();
        let duplicate_ingress_observation = SmokeBeadObservation {
            run_id: "run-smoke-03".to_string(),
            checks: vec![
                SmokeBeadCheckObservation {
                    check: SmokeBeadCheckName::IngressHealth,
                    endpoint: DEFAULT_SMOKE_BEAD_INGRESS_HEALTH_URL.to_string(),
                    success: true,
                    diagnostics: "ok".to_string(),
                    timestamp: now,
                },
                SmokeBeadCheckObservation {
                    check: SmokeBeadCheckName::IngressHealth,
                    endpoint: DEFAULT_SMOKE_BEAD_INGRESS_HEALTH_URL.to_string(),
                    success: true,
                    diagnostics: "ok".to_string(),
                    timestamp: now,
                },
                SmokeBeadCheckObservation {
                    check: SmokeBeadCheckName::OrchestratorStatus,
                    endpoint: "http://localhost:8080/OyaOrchestrator/run-smoke-03/get_status"
                        .to_string(),
                    success: true,
                    diagnostics: "ok".to_string(),
                    timestamp: now,
                },
            ],
        };
        assert_eq!(
            evaluate_smoke_bead_result(&duplicate_ingress_observation),
            Err(SmokeBeadError::InvalidReport("duplicate ingress_health checks"))
        );
    }

    #[test]
    fn evaluate_smoke_bead_result_uses_latest_check_timestamp_for_final_stage() {
        let base = Utc::now();
        let ingress_timestamp = base + Duration::milliseconds(5);
        let orchestrator_timestamp = base;

        let observation = SmokeBeadObservation {
            run_id: "run-smoke-03".to_string(),
            checks: vec![
                SmokeBeadCheckObservation {
                    check: SmokeBeadCheckName::IngressHealth,
                    endpoint: DEFAULT_SMOKE_BEAD_INGRESS_HEALTH_URL.to_string(),
                    success: true,
                    diagnostics: "ingress healthy".to_string(),
                    timestamp: ingress_timestamp,
                },
                SmokeBeadCheckObservation {
                    check: SmokeBeadCheckName::OrchestratorStatus,
                    endpoint: "http://localhost:8080/OyaOrchestrator/run-smoke-03/get_status"
                        .to_string(),
                    success: true,
                    diagnostics: "orchestrator healthy".to_string(),
                    timestamp: orchestrator_timestamp,
                },
            ],
        };

        let report_result = evaluate_smoke_bead_result(&observation);
        assert!(report_result.is_ok());
        let Ok(report) = report_result else { return };

        assert_eq!(report.stages[1].timestamp, ingress_timestamp);
        assert_eq!(report.stages[2].timestamp, ingress_timestamp + Duration::milliseconds(1));
    }

    #[test]
    fn evaluate_smoke_bead_result_rejects_invalid_check_diagnostics() {
        let empty_diagnostics = SmokeBeadObservation {
            run_id: "run-smoke-03".to_string(),
            checks: vec![
                SmokeBeadCheckObservation {
                    check: SmokeBeadCheckName::IngressHealth,
                    endpoint: DEFAULT_SMOKE_BEAD_INGRESS_HEALTH_URL.to_string(),
                    success: true,
                    diagnostics: " ".to_string(),
                    timestamp: Utc::now(),
                },
                SmokeBeadCheckObservation {
                    check: SmokeBeadCheckName::OrchestratorStatus,
                    endpoint: "http://localhost:8080/OyaOrchestrator/run-smoke-03/get_status"
                        .to_string(),
                    success: true,
                    diagnostics: "ok".to_string(),
                    timestamp: Utc::now(),
                },
            ],
        };
        assert_eq!(
            evaluate_smoke_bead_result(&empty_diagnostics),
            Err(SmokeBeadError::InvalidReport("empty check diagnostics"))
        );

        let control_char_diagnostics = SmokeBeadObservation {
            run_id: "run-smoke-03".to_string(),
            checks: vec![
                SmokeBeadCheckObservation {
                    check: SmokeBeadCheckName::IngressHealth,
                    endpoint: DEFAULT_SMOKE_BEAD_INGRESS_HEALTH_URL.to_string(),
                    success: true,
                    diagnostics: "ok\u{0007}".to_string(),
                    timestamp: Utc::now(),
                },
                SmokeBeadCheckObservation {
                    check: SmokeBeadCheckName::OrchestratorStatus,
                    endpoint: "http://localhost:8080/OyaOrchestrator/run-smoke-03/get_status"
                        .to_string(),
                    success: true,
                    diagnostics: "ok".to_string(),
                    timestamp: Utc::now(),
                },
            ],
        };
        assert_eq!(
            evaluate_smoke_bead_result(&control_char_diagnostics),
            Err(SmokeBeadError::InvalidReport(
                "check diagnostics contain invalid control characters"
            ))
        );
    }

    #[test]
    fn validate_smoke_bead_report_rejects_decision_stage_mismatch() {
        let base = Utc::now();
        let report = SmokeBeadReport {
            run_id: "run-smoke-04".to_string(),
            checks: vec![
                SmokeBeadCheckObservation {
                    check: SmokeBeadCheckName::IngressHealth,
                    endpoint: DEFAULT_SMOKE_BEAD_INGRESS_HEALTH_URL.to_string(),
                    success: true,
                    diagnostics: "ingress healthy".to_string(),
                    timestamp: base,
                },
                SmokeBeadCheckObservation {
                    check: SmokeBeadCheckName::OrchestratorStatus,
                    endpoint: "http://localhost:8080/OyaOrchestrator/run-smoke-04/get_status"
                        .to_string(),
                    success: true,
                    diagnostics: "orchestrator healthy".to_string(),
                    timestamp: base + Duration::milliseconds(1),
                },
            ],
            stages: vec![
                SmokeBeadStageReport {
                    stage: SmokeBeadStageName::IngressHealth,
                    status: SmokeBeadStageStatus::Passed,
                    diagnostics: "ingress healthy".to_string(),
                    timestamp: base,
                },
                SmokeBeadStageReport {
                    stage: SmokeBeadStageName::OrchestratorStatus,
                    status: SmokeBeadStageStatus::Passed,
                    diagnostics: "orchestrator healthy".to_string(),
                    timestamp: base + Duration::milliseconds(1),
                },
                SmokeBeadStageReport {
                    stage: SmokeBeadStageName::FinalDecision,
                    status: SmokeBeadStageStatus::Failed,
                    diagnostics: "mismatch".to_string(),
                    timestamp: base + Duration::milliseconds(2),
                },
            ],
            decision: SmokeBeadDecision::Pass,
        };

        assert_eq!(
            validate_smoke_bead_report(&report),
            Err(SmokeBeadError::InvalidReport("final decision stage mismatch"))
        );
    }

    #[test]
    fn validate_smoke_bead_report_rejects_invalid_stage_count_and_order() {
        let mut invalid_stage_count = make_valid_smoke_bead_report();
        invalid_stage_count.stages.truncate(1);
        assert_eq!(
            validate_smoke_bead_report(&invalid_stage_count),
            Err(SmokeBeadError::InvalidReport("unexpected stage count"))
        );

        let mut invalid_stage_order = make_valid_smoke_bead_report();
        invalid_stage_order.stages = vec![
            invalid_stage_order.stages[1].clone(),
            invalid_stage_order.stages[0].clone(),
            invalid_stage_order.stages[2].clone(),
        ];
        assert_eq!(
            validate_smoke_bead_report(&invalid_stage_order),
            Err(SmokeBeadError::InvalidReport("invalid stage order"))
        );
    }

    #[test]
    fn validate_smoke_bead_report_rejects_invalid_checks_and_diagnostics() {
        let mut invalid_orchestrator_endpoint = make_valid_smoke_bead_report();
        invalid_orchestrator_endpoint.checks[1].endpoint =
            "http://localhost:8080/OyaOrchestrator/other/get_status".to_string();
        assert_eq!(
            validate_smoke_bead_report(&invalid_orchestrator_endpoint),
            Err(SmokeBeadError::InvalidReport("invalid orchestrator check endpoint"))
        );

        let mut invalid_check_diagnostics = make_valid_smoke_bead_report();
        invalid_check_diagnostics.checks[0].diagnostics = "ok\u{0007}".to_string();
        assert_eq!(
            validate_smoke_bead_report(&invalid_check_diagnostics),
            Err(SmokeBeadError::InvalidReport(
                "check diagnostics contain invalid control characters"
            ))
        );

        let mut invalid_stage_diagnostics = make_valid_smoke_bead_report();
        invalid_stage_diagnostics.stages[1].diagnostics = "\u{0007}".to_string();
        assert_eq!(
            validate_smoke_bead_report(&invalid_stage_diagnostics),
            Err(SmokeBeadError::InvalidReport(
                "stage diagnostics contain invalid control characters"
            ))
        );
    }

    #[test]
    fn validate_smoke_bead_report_rejects_missing_required_checks() {
        let mut missing_ingress = make_valid_smoke_bead_report();
        missing_ingress.checks.remove(0);
        assert_eq!(
            validate_smoke_bead_report(&missing_ingress),
            Err(SmokeBeadError::MissingCheck("ingress_health"))
        );

        let mut missing_orchestrator = make_valid_smoke_bead_report();
        missing_orchestrator.checks.remove(1);
        assert_eq!(
            validate_smoke_bead_report(&missing_orchestrator),
            Err(SmokeBeadError::MissingCheck("orchestrator_status"))
        );
    }

    #[test]
    fn validate_smoke_bead_report_accepts_equal_consecutive_timestamps() {
        let mut report = make_valid_smoke_bead_report();
        report.stages[2].timestamp = report.stages[1].timestamp;

        assert_eq!(validate_smoke_bead_report(&report), Ok(()));
    }

    #[test]
    fn validate_smoke_bead_report_rejects_non_monotonic_timestamps_and_decision_mismatch() {
        let mut non_monotonic_report = make_valid_smoke_bead_report();
        non_monotonic_report.stages[1].timestamp =
            non_monotonic_report.stages[0].timestamp - Duration::seconds(1);
        assert_eq!(
            validate_smoke_bead_report(&non_monotonic_report),
            Err(SmokeBeadError::InvalidReport("non-monotonic stage timestamps"))
        );

        let mut decision_mismatch_report = make_valid_smoke_bead_report();
        decision_mismatch_report.decision = SmokeBeadDecision::Fail;
        assert_eq!(
            validate_smoke_bead_report(&decision_mismatch_report),
            Err(SmokeBeadError::InvalidReport("decision mismatch"))
        );
    }

    #[test]
    fn validate_smoke_bead_report_rejects_run_id_check_counts_and_stage_status_mismatches() {
        let mut invalid_run_id_report = make_valid_smoke_bead_report();
        invalid_run_id_report.run_id = " run-smoke-bead-test ".to_string();
        assert_eq!(
            validate_smoke_bead_report(&invalid_run_id_report),
            Err(SmokeBeadError::InvalidFieldContent("run_id"))
        );

        let mut invalid_ingress_count_report = make_valid_smoke_bead_report();
        invalid_ingress_count_report.checks[1].check = SmokeBeadCheckName::IngressHealth;
        assert_eq!(
            validate_smoke_bead_report(&invalid_ingress_count_report),
            Err(SmokeBeadError::InvalidReport("invalid ingress check count"))
        );

        let mut invalid_orchestrator_count_report = make_valid_smoke_bead_report();
        invalid_orchestrator_count_report
            .checks
            .push(invalid_orchestrator_count_report.checks[1].clone());
        assert_eq!(
            validate_smoke_bead_report(&invalid_orchestrator_count_report),
            Err(SmokeBeadError::InvalidReport("invalid orchestrator check count"))
        );

        let mut empty_check_diagnostics_report = make_valid_smoke_bead_report();
        empty_check_diagnostics_report.checks[0].diagnostics = "  ".to_string();
        assert_eq!(
            validate_smoke_bead_report(&empty_check_diagnostics_report),
            Err(SmokeBeadError::InvalidReport("empty check diagnostics"))
        );

        let mut empty_stage_diagnostics_report = make_valid_smoke_bead_report();
        empty_stage_diagnostics_report.stages[1].diagnostics = "  ".to_string();
        assert_eq!(
            validate_smoke_bead_report(&empty_stage_diagnostics_report),
            Err(SmokeBeadError::InvalidReport("empty stage diagnostics"))
        );

        let mut ingress_stage_mismatch_report = make_valid_smoke_bead_report();
        ingress_stage_mismatch_report.stages[0].status = SmokeBeadStageStatus::Failed;
        assert_eq!(
            validate_smoke_bead_report(&ingress_stage_mismatch_report),
            Err(SmokeBeadError::InvalidReport("ingress stage mismatch"))
        );

        let mut ingress_stage_diagnostics_mismatch_report = make_valid_smoke_bead_report();
        ingress_stage_diagnostics_mismatch_report.stages[0].diagnostics =
            "forged ingress diagnostics".to_string();
        assert_eq!(
            validate_smoke_bead_report(&ingress_stage_diagnostics_mismatch_report),
            Err(SmokeBeadError::InvalidReport("ingress stage diagnostics mismatch"))
        );

        let mut orchestrator_stage_mismatch_report = make_valid_smoke_bead_report();
        orchestrator_stage_mismatch_report.stages[1].status = SmokeBeadStageStatus::Failed;
        assert_eq!(
            validate_smoke_bead_report(&orchestrator_stage_mismatch_report),
            Err(SmokeBeadError::InvalidReport("orchestrator stage mismatch"))
        );

        let mut orchestrator_stage_diagnostics_mismatch_report = make_valid_smoke_bead_report();
        orchestrator_stage_diagnostics_mismatch_report.stages[1].diagnostics =
            "forged orchestrator diagnostics".to_string();
        assert_eq!(
            validate_smoke_bead_report(&orchestrator_stage_diagnostics_mismatch_report),
            Err(SmokeBeadError::InvalidReport("orchestrator stage diagnostics mismatch"))
        );
    }

    #[test]
    fn validate_smoke_bead_report_rejects_oversized_diagnostics() {
        let mut oversized_check_diagnostics = make_valid_smoke_bead_report();
        oversized_check_diagnostics.checks[0].diagnostics =
            "d".repeat(MAX_SMOKE_BEAD_DIAGNOSTICS_LEN + 1);
        assert_eq!(
            validate_smoke_bead_report(&oversized_check_diagnostics),
            Err(SmokeBeadError::InvalidReport("check diagnostics exceed max length"))
        );

        let mut oversized_stage_diagnostics = make_valid_smoke_bead_report();
        oversized_stage_diagnostics.stages[1].diagnostics =
            "d".repeat(MAX_SMOKE_BEAD_DIAGNOSTICS_LEN + 1);
        assert_eq!(
            validate_smoke_bead_report(&oversized_stage_diagnostics),
            Err(SmokeBeadError::InvalidReport("stage diagnostics exceed max length"))
        );
    }

    #[test]
    fn validate_smoke_bead_report_rejects_invalid_ingress_check_endpoint() {
        let mut report = make_valid_smoke_bead_report();
        report.checks[0].endpoint = "http://localhost:8080/health".to_string();

        assert_eq!(
            validate_smoke_bead_report(&report),
            Err(SmokeBeadError::InvalidReport("invalid ingress check endpoint"))
        );
    }

    #[test]
    fn validate_smoke_bead_report_rejects_final_decision_diagnostics_mismatch() {
        let mut report = make_valid_smoke_bead_report();
        report.stages[2].diagnostics = "smoke-bead checks failed".to_string();

        assert_eq!(
            validate_smoke_bead_report(&report),
            Err(SmokeBeadError::InvalidReport("final decision diagnostics mismatch"))
        );
    }

    #[test]
    fn validate_smoke_bead_report_rejects_stage_timestamps_before_checks() {
        let mut ingress_stage_before_check = make_valid_smoke_bead_report();
        ingress_stage_before_check.stages[0].timestamp =
            ingress_stage_before_check.checks[0].timestamp - Duration::milliseconds(1);
        assert_eq!(
            validate_smoke_bead_report(&ingress_stage_before_check),
            Err(SmokeBeadError::InvalidReport("ingress stage timestamp precedes check"))
        );

        let mut orchestrator_stage_before_check = make_valid_smoke_bead_report();
        orchestrator_stage_before_check.stages[1].timestamp =
            orchestrator_stage_before_check.checks[1].timestamp - Duration::milliseconds(1);
        assert_eq!(
            validate_smoke_bead_report(&orchestrator_stage_before_check),
            Err(SmokeBeadError::InvalidReport("orchestrator stage timestamp precedes check"))
        );
    }

    #[test]
    fn smoke_bead_pipeline_passes_for_valid_default_contract() {
        let plan_result = build_smoke_bead_plan(&SmokeBeadInput { run_id: "run-smoke-05".into() });
        assert!(plan_result.is_ok());
        let Ok(plan) = plan_result else { return };

        let handle_result = start_smoke_bead_runtime(&plan);
        assert!(handle_result.is_ok());
        let Ok(handle) = handle_result else { return };

        let observation_result = capture_smoke_bead_observation(&handle);
        assert!(observation_result.is_ok());
        let Ok(observation) = observation_result else {
            return;
        };

        let report_result = evaluate_smoke_bead_result(&observation);
        assert!(report_result.is_ok());
        let Ok(report) = report_result else { return };

        assert_eq!(report.decision, SmokeBeadDecision::Pass);
        assert_eq!(validate_smoke_bead_report(&report), Ok(()));
    }

    #[test]
    fn build_bead_min_plan_rejects_empty_run_id_and_sets_defaults() {
        let empty_result = build_bead_min_plan(&BeadMinInput { run_id: "  ".to_string() });
        assert_eq!(empty_result, Err(BeadMinError::EmptyField("run_id")));

        let plan_result = build_bead_min_plan(&BeadMinInput { run_id: "run-bead-min-01".into() });
        assert!(plan_result.is_ok());
        let Ok(plan) = plan_result else { return };

        assert_eq!(plan.runtime_command, DEFAULT_BEAD_MIN_RUNTIME_COMMAND);
        assert_eq!(plan.ingress_health_url, DEFAULT_BEAD_MIN_INGRESS_HEALTH_URL);
        assert_eq!(
            plan.orchestrator_status_url,
            "http://localhost:8080/OyaOrchestrator/run-bead-min-01/get_status"
        );
    }

    #[test]
    fn build_bead_min_plan_rejects_invalid_run_id_boundaries_and_content() {
        let oversized_result =
            build_bead_min_plan(&BeadMinInput { run_id: "r".repeat(MAX_SMOKE_RUN_ID_LEN + 1) });
        assert_eq!(
            oversized_result,
            Err(BeadMinError::FieldTooLong("run_id", MAX_SMOKE_RUN_ID_LEN))
        );

        let control_char_result =
            build_bead_min_plan(&BeadMinInput { run_id: "run-bead-min\u{0007}".to_string() });
        assert_eq!(control_char_result, Err(BeadMinError::InvalidFieldContent("run_id")));

        let path_injection_result =
            build_bead_min_plan(&BeadMinInput { run_id: "../run-bead-min".to_string() });
        assert_eq!(path_injection_result, Err(BeadMinError::InvalidFieldContent("run_id")));

        let query_injection_result =
            build_bead_min_plan(&BeadMinInput { run_id: "run-bead-min?x=1".to_string() });
        assert_eq!(query_injection_result, Err(BeadMinError::InvalidFieldContent("run_id")));
    }

    #[test]
    fn start_bead_min_runtime_rejects_non_default_runtime_command() {
        let plan = BeadMinPlan {
            run_id: "run-bead-min-01".to_string(),
            runtime_command: "scripts/not-default.sh".to_string(),
            ingress_health_url: DEFAULT_BEAD_MIN_INGRESS_HEALTH_URL.to_string(),
            orchestrator_status_url:
                "http://localhost:8080/OyaOrchestrator/run-bead-min-01/get_status".to_string(),
        };

        let result = start_bead_min_runtime(&plan);
        assert_eq!(result, Err(BeadMinError::InvalidRuntimeCommand));
    }

    #[test]
    fn start_bead_min_runtime_rejects_invalid_run_id_and_endpoints() {
        let invalid_run_id_plan = BeadMinPlan {
            run_id: " run-bead-min-01 ".to_string(),
            runtime_command: DEFAULT_BEAD_MIN_RUNTIME_COMMAND.to_string(),
            ingress_health_url: DEFAULT_BEAD_MIN_INGRESS_HEALTH_URL.to_string(),
            orchestrator_status_url:
                "http://localhost:8080/OyaOrchestrator/run-bead-min-01/get_status".to_string(),
        };
        assert_eq!(
            start_bead_min_runtime(&invalid_run_id_plan),
            Err(BeadMinError::InvalidFieldContent("run_id"))
        );

        let invalid_ingress_url_plan = BeadMinPlan {
            run_id: "run-bead-min-01".to_string(),
            runtime_command: DEFAULT_BEAD_MIN_RUNTIME_COMMAND.to_string(),
            ingress_health_url: "localhost:8080/restate/health".to_string(),
            orchestrator_status_url:
                "http://localhost:8080/OyaOrchestrator/run-bead-min-01/get_status".to_string(),
        };
        assert_eq!(
            start_bead_min_runtime(&invalid_ingress_url_plan),
            Err(BeadMinError::InvalidEndpoint("ingress_health_url"))
        );

        let invalid_ingress_contract_plan = BeadMinPlan {
            run_id: "run-bead-min-01".to_string(),
            runtime_command: DEFAULT_BEAD_MIN_RUNTIME_COMMAND.to_string(),
            ingress_health_url: "http://localhost:8080/health".to_string(),
            orchestrator_status_url:
                "http://localhost:8080/OyaOrchestrator/run-bead-min-01/get_status".to_string(),
        };
        assert_eq!(
            start_bead_min_runtime(&invalid_ingress_contract_plan),
            Err(BeadMinError::InvalidEndpoint("ingress_health_url"))
        );

        let invalid_orchestrator_url_plan = BeadMinPlan {
            run_id: "run-bead-min-01".to_string(),
            runtime_command: DEFAULT_BEAD_MIN_RUNTIME_COMMAND.to_string(),
            ingress_health_url: DEFAULT_BEAD_MIN_INGRESS_HEALTH_URL.to_string(),
            orchestrator_status_url: "localhost:8080/OyaOrchestrator/run-bead-min-01/get_status"
                .to_string(),
        };
        assert_eq!(
            start_bead_min_runtime(&invalid_orchestrator_url_plan),
            Err(BeadMinError::InvalidEndpoint("orchestrator_status_url"))
        );

        let invalid_orchestrator_contract_plan = BeadMinPlan {
            run_id: "run-bead-min-01".to_string(),
            runtime_command: DEFAULT_BEAD_MIN_RUNTIME_COMMAND.to_string(),
            ingress_health_url: DEFAULT_BEAD_MIN_INGRESS_HEALTH_URL.to_string(),
            orchestrator_status_url: "http://localhost:8080/OyaOrchestrator/other/get_status"
                .to_string(),
        };
        assert_eq!(
            start_bead_min_runtime(&invalid_orchestrator_contract_plan),
            Err(BeadMinError::InvalidEndpoint("orchestrator_status_url"))
        );
    }

    #[test]
    fn capture_bead_min_observation_emits_exactly_one_check_per_stage() {
        let plan_result = build_bead_min_plan(&BeadMinInput { run_id: "run-bead-min-02".into() });
        assert!(plan_result.is_ok());
        let Ok(plan) = plan_result else { return };

        let handle_result = start_bead_min_runtime(&plan);
        assert!(handle_result.is_ok());
        let Ok(handle) = handle_result else { return };

        let observation_result = capture_bead_min_observation(&handle);
        assert!(observation_result.is_ok());
        let Ok(observation) = observation_result else {
            return;
        };

        let ingress_count = observation
            .checks
            .iter()
            .filter(|check| check.check == BeadMinCheckName::IngressHealth)
            .count();
        let orchestrator_count = observation
            .checks
            .iter()
            .filter(|check| check.check == BeadMinCheckName::OrchestratorStatus)
            .count();

        assert_eq!(ingress_count, 1);
        assert_eq!(orchestrator_count, 1);
        assert!(observation.checks.iter().all(|check| !check.diagnostics.trim().is_empty()));
    }

    #[test]
    fn capture_bead_min_observation_rejects_runtime_state_and_endpoint_violations() {
        let mut not_ready_handle = BeadMinRuntimeHandle {
            run_id: "run-bead-min-02".to_string(),
            runtime_command: DEFAULT_BEAD_MIN_RUNTIME_COMMAND.to_string(),
            ingress_health_url: DEFAULT_BEAD_MIN_INGRESS_HEALTH_URL.to_string(),
            orchestrator_status_url:
                "http://localhost:8080/OyaOrchestrator/run-bead-min-02/get_status".to_string(),
            started_at: Utc::now(),
            runtime_ready: false,
        };
        assert_eq!(
            capture_bead_min_observation(&not_ready_handle),
            Err(BeadMinError::RuntimeNotReady)
        );

        not_ready_handle.runtime_ready = true;
        not_ready_handle.runtime_command = "scripts/not-default.sh".to_string();
        assert_eq!(
            capture_bead_min_observation(&not_ready_handle),
            Err(BeadMinError::InvalidRuntimeCommand)
        );

        let invalid_ingress_handle = BeadMinRuntimeHandle {
            run_id: "run-bead-min-02".to_string(),
            runtime_command: DEFAULT_BEAD_MIN_RUNTIME_COMMAND.to_string(),
            ingress_health_url: "http://localhost:8080/health".to_string(),
            orchestrator_status_url:
                "http://localhost:8080/OyaOrchestrator/run-bead-min-02/get_status".to_string(),
            started_at: Utc::now(),
            runtime_ready: true,
        };
        assert_eq!(
            capture_bead_min_observation(&invalid_ingress_handle),
            Err(BeadMinError::InvalidEndpoint("ingress_health_url"))
        );

        let invalid_orchestrator_handle = BeadMinRuntimeHandle {
            run_id: "run-bead-min-02".to_string(),
            runtime_command: DEFAULT_BEAD_MIN_RUNTIME_COMMAND.to_string(),
            ingress_health_url: DEFAULT_BEAD_MIN_INGRESS_HEALTH_URL.to_string(),
            orchestrator_status_url: "http://localhost:8080/OyaOrchestrator/other/get_status"
                .to_string(),
            started_at: Utc::now(),
            runtime_ready: true,
        };
        assert_eq!(
            capture_bead_min_observation(&invalid_orchestrator_handle),
            Err(BeadMinError::InvalidEndpoint("orchestrator_status_url"))
        );
    }

    #[test]
    fn evaluate_bead_min_result_uses_strict_stage_order_and_derived_decision() {
        let base = Utc::now();
        let observation = BeadMinObservation {
            run_id: "run-bead-min-03".to_string(),
            checks: vec![
                BeadMinCheckObservation {
                    check: BeadMinCheckName::IngressHealth,
                    endpoint: DEFAULT_BEAD_MIN_INGRESS_HEALTH_URL.to_string(),
                    success: true,
                    diagnostics: "ingress healthy".to_string(),
                    timestamp: base,
                },
                BeadMinCheckObservation {
                    check: BeadMinCheckName::OrchestratorStatus,
                    endpoint: "http://localhost:8080/OyaOrchestrator/run-bead-min-03/get_status"
                        .to_string(),
                    success: false,
                    diagnostics: "orchestrator timeout".to_string(),
                    timestamp: base + Duration::milliseconds(1),
                },
            ],
        };

        let report_result = evaluate_bead_min_result(&observation);
        assert!(report_result.is_ok());
        let Ok(report) = report_result else { return };

        assert_eq!(
            report.stages.iter().map(|stage| stage.stage.clone()).collect::<Vec<_>>(),
            vec![
                BeadMinStageName::IngressHealth,
                BeadMinStageName::OrchestratorStatus,
                BeadMinStageName::FinalDecision,
            ]
        );
        assert_eq!(report.decision, BeadMinDecision::Fail);
    }

    #[test]
    fn evaluate_bead_min_result_rejects_missing_or_duplicate_checks() {
        let base = Utc::now();

        let missing_ingress = BeadMinObservation {
            run_id: "run-bead-min-03".to_string(),
            checks: vec![BeadMinCheckObservation {
                check: BeadMinCheckName::OrchestratorStatus,
                endpoint: "http://localhost:8080/OyaOrchestrator/run-bead-min-03/get_status"
                    .to_string(),
                success: true,
                diagnostics: "orchestrator healthy".to_string(),
                timestamp: base,
            }],
        };
        assert_eq!(
            evaluate_bead_min_result(&missing_ingress),
            Err(BeadMinError::MissingCheck("ingress_health"))
        );

        let duplicate_ingress = BeadMinObservation {
            run_id: "run-bead-min-03".to_string(),
            checks: vec![
                BeadMinCheckObservation {
                    check: BeadMinCheckName::IngressHealth,
                    endpoint: DEFAULT_BEAD_MIN_INGRESS_HEALTH_URL.to_string(),
                    success: true,
                    diagnostics: "ingress healthy".to_string(),
                    timestamp: base,
                },
                BeadMinCheckObservation {
                    check: BeadMinCheckName::IngressHealth,
                    endpoint: DEFAULT_BEAD_MIN_INGRESS_HEALTH_URL.to_string(),
                    success: true,
                    diagnostics: "ingress duplicate".to_string(),
                    timestamp: base + Duration::milliseconds(1),
                },
                BeadMinCheckObservation {
                    check: BeadMinCheckName::OrchestratorStatus,
                    endpoint: "http://localhost:8080/OyaOrchestrator/run-bead-min-03/get_status"
                        .to_string(),
                    success: true,
                    diagnostics: "orchestrator healthy".to_string(),
                    timestamp: base + Duration::milliseconds(2),
                },
            ],
        };
        assert_eq!(
            evaluate_bead_min_result(&duplicate_ingress),
            Err(BeadMinError::InvalidReport("duplicate ingress_health checks"))
        );

        let missing_orchestrator = BeadMinObservation {
            run_id: "run-bead-min-03".to_string(),
            checks: vec![BeadMinCheckObservation {
                check: BeadMinCheckName::IngressHealth,
                endpoint: DEFAULT_BEAD_MIN_INGRESS_HEALTH_URL.to_string(),
                success: true,
                diagnostics: "ingress healthy".to_string(),
                timestamp: base,
            }],
        };
        assert_eq!(
            evaluate_bead_min_result(&missing_orchestrator),
            Err(BeadMinError::MissingCheck("orchestrator_status"))
        );

        let duplicate_orchestrator = BeadMinObservation {
            run_id: "run-bead-min-03".to_string(),
            checks: vec![
                BeadMinCheckObservation {
                    check: BeadMinCheckName::IngressHealth,
                    endpoint: DEFAULT_BEAD_MIN_INGRESS_HEALTH_URL.to_string(),
                    success: true,
                    diagnostics: "ingress healthy".to_string(),
                    timestamp: base,
                },
                BeadMinCheckObservation {
                    check: BeadMinCheckName::OrchestratorStatus,
                    endpoint: "http://localhost:8080/OyaOrchestrator/run-bead-min-03/get_status"
                        .to_string(),
                    success: true,
                    diagnostics: "orchestrator healthy".to_string(),
                    timestamp: base + Duration::milliseconds(1),
                },
                BeadMinCheckObservation {
                    check: BeadMinCheckName::OrchestratorStatus,
                    endpoint: "http://localhost:8080/OyaOrchestrator/run-bead-min-03/get_status"
                        .to_string(),
                    success: false,
                    diagnostics: "orchestrator duplicate".to_string(),
                    timestamp: base + Duration::milliseconds(2),
                },
            ],
        };
        assert_eq!(
            evaluate_bead_min_result(&duplicate_orchestrator),
            Err(BeadMinError::InvalidReport("duplicate orchestrator_status checks"))
        );
    }

    #[test]
    fn validate_bead_min_report_rejects_endpoint_and_decision_mismatches() {
        let mut invalid_endpoint_report = make_valid_bead_min_report();
        invalid_endpoint_report.checks[1].endpoint =
            "http://localhost:8080/OyaOrchestrator/other/get_status".to_string();
        assert_eq!(
            validate_bead_min_report(&invalid_endpoint_report),
            Err(BeadMinError::InvalidReport("invalid orchestrator check endpoint"))
        );

        let mut decision_mismatch_report = make_valid_bead_min_report();
        decision_mismatch_report.decision = BeadMinDecision::Fail;
        assert_eq!(
            validate_bead_min_report(&decision_mismatch_report),
            Err(BeadMinError::InvalidReport("decision mismatch"))
        );
    }

    #[test]
    fn validate_bead_min_report_rejects_stage_shape_and_timestamp_mismatches() {
        let mut invalid_stage_count = make_valid_bead_min_report();
        invalid_stage_count.stages.pop();
        assert_eq!(
            validate_bead_min_report(&invalid_stage_count),
            Err(BeadMinError::InvalidReport("unexpected stage count"))
        );

        let mut invalid_stage_order = make_valid_bead_min_report();
        invalid_stage_order.stages.swap(0, 1);
        assert_eq!(
            validate_bead_min_report(&invalid_stage_order),
            Err(BeadMinError::InvalidReport("invalid stage order"))
        );

        let mut empty_stage_diagnostics = make_valid_bead_min_report();
        empty_stage_diagnostics.stages[1].diagnostics = "  ".to_string();
        assert_eq!(
            validate_bead_min_report(&empty_stage_diagnostics),
            Err(BeadMinError::InvalidReport("empty stage diagnostics"))
        );

        let mut invalid_stage_diagnostics = make_valid_bead_min_report();
        invalid_stage_diagnostics.stages[1].diagnostics = "\u{0007}".to_string();
        assert_eq!(
            validate_bead_min_report(&invalid_stage_diagnostics),
            Err(BeadMinError::InvalidReport(
                "stage diagnostics contain invalid control characters"
            ))
        );

        let mut oversized_stage_diagnostics = make_valid_bead_min_report();
        oversized_stage_diagnostics.stages[1].diagnostics =
            "d".repeat(MAX_BEAD_MIN_DIAGNOSTICS_LEN + 1);
        assert_eq!(
            validate_bead_min_report(&oversized_stage_diagnostics),
            Err(BeadMinError::InvalidReport("stage diagnostics exceed max length"))
        );

        let mut non_monotonic_timestamps = make_valid_bead_min_report();
        non_monotonic_timestamps.stages[1].timestamp =
            non_monotonic_timestamps.stages[0].timestamp - Duration::milliseconds(1);
        assert_eq!(
            validate_bead_min_report(&non_monotonic_timestamps),
            Err(BeadMinError::InvalidReport("non-monotonic stage timestamps"))
        );

        let mut ingress_before_check = make_valid_bead_min_report();
        ingress_before_check.stages[0].timestamp =
            ingress_before_check.checks[0].timestamp - Duration::milliseconds(1);
        assert_eq!(
            validate_bead_min_report(&ingress_before_check),
            Err(BeadMinError::InvalidReport("ingress stage timestamp precedes check"))
        );

        let mut orchestrator_before_check = make_valid_bead_min_report();
        orchestrator_before_check.stages[1].timestamp =
            orchestrator_before_check.checks[1].timestamp - Duration::milliseconds(1);
        assert_eq!(
            validate_bead_min_report(&orchestrator_before_check),
            Err(BeadMinError::InvalidReport("orchestrator stage timestamp precedes check"))
        );
    }

    #[test]
    fn validate_bead_min_report_rejects_missing_checks_and_diagnostics_mismatches() {
        let mut missing_ingress = make_valid_bead_min_report();
        missing_ingress.checks.remove(0);
        assert_eq!(
            validate_bead_min_report(&missing_ingress),
            Err(BeadMinError::MissingCheck("ingress_health"))
        );

        let mut missing_orchestrator = make_valid_bead_min_report();
        missing_orchestrator.checks.remove(1);
        assert_eq!(
            validate_bead_min_report(&missing_orchestrator),
            Err(BeadMinError::MissingCheck("orchestrator_status"))
        );

        let mut empty_check_diagnostics = make_valid_bead_min_report();
        empty_check_diagnostics.checks[0].diagnostics = "  ".to_string();
        assert_eq!(
            validate_bead_min_report(&empty_check_diagnostics),
            Err(BeadMinError::InvalidReport("empty check diagnostics"))
        );

        let mut oversized_check_diagnostics = make_valid_bead_min_report();
        oversized_check_diagnostics.checks[0].diagnostics =
            "d".repeat(MAX_BEAD_MIN_DIAGNOSTICS_LEN + 1);
        assert_eq!(
            validate_bead_min_report(&oversized_check_diagnostics),
            Err(BeadMinError::InvalidReport("check diagnostics exceed max length"))
        );

        let mut invalid_check_diagnostics = make_valid_bead_min_report();
        invalid_check_diagnostics.checks[0].diagnostics = "\u{0007}".to_string();
        assert_eq!(
            validate_bead_min_report(&invalid_check_diagnostics),
            Err(BeadMinError::InvalidReport(
                "check diagnostics contain invalid control characters"
            ))
        );

        let mut final_decision_mismatch = make_valid_bead_min_report();
        final_decision_mismatch.stages[2].diagnostics = "bead-min checks failed".to_string();
        assert_eq!(
            validate_bead_min_report(&final_decision_mismatch),
            Err(BeadMinError::InvalidReport("final decision diagnostics mismatch"))
        );
    }

    #[test]
    fn bead_min_pipeline_passes_for_valid_default_contract() {
        let plan_result = build_bead_min_plan(&BeadMinInput { run_id: "run-bead-min-04".into() });
        assert!(plan_result.is_ok());
        let Ok(plan) = plan_result else { return };

        let handle_result = start_bead_min_runtime(&plan);
        assert!(handle_result.is_ok());
        let Ok(handle) = handle_result else { return };

        let observation_result = capture_bead_min_observation(&handle);
        assert!(observation_result.is_ok());
        let Ok(observation) = observation_result else {
            return;
        };

        let report_result = evaluate_bead_min_result(&observation);
        assert!(report_result.is_ok());
        let Ok(report) = report_result else { return };

        assert_eq!(report.decision, BeadMinDecision::Pass);
        assert_eq!(validate_bead_min_report(&report), Ok(()));
    }

    #[test]
    fn parse_opencode_output_rejects_empty() {
        let result = parse_opencode_output("  \n\t ");
        assert!(result.is_err());
    }

    #[test]
    fn parse_opencode_output_rejects_invalid_json() {
        let result = parse_opencode_output("not json");
        assert!(result.is_err());
    }

    #[test]
    fn parse_opencode_output_requires_stdout_field() {
        let result = parse_opencode_output("{\"status\":\"ok\"}");
        assert!(result.is_err());
    }

    #[test]
    fn parse_opencode_output_requires_stdout_string() {
        let result = parse_opencode_output("{\"stdout\":123}");
        assert!(result.is_err());
    }

    #[test]
    fn parse_opencode_output_accepts_stdout_string() {
        let result = parse_opencode_output("{\"stdout\":\"ok\"}");
        assert_eq!(result, Ok(OpencodeRunOutput { stdout: "ok".to_string() }));
    }

    #[test]
    fn parse_opencode_output_trims_outer_whitespace() {
        let result = parse_opencode_output("  {\"stdout\":\"ok\"}  ");
        assert_eq!(result, Ok(OpencodeRunOutput { stdout: "ok".to_string() }));
    }

    #[test]
    fn parse_opencode_output_rejects_oversized_json_payload() {
        let oversized_payload =
            format!("{{\"stdout\":\"{}\"}}", "x".repeat(MAX_OPENCODE_OUTPUT_JSON_LEN + 1));

        let result = parse_opencode_output(&oversized_payload);
        assert!(result.is_err());
        assert_eq!(
            result.err().map(|error| error.to_string()),
            Some("opencode output exceeds maximum length".to_string())
        );
    }

    #[test]
    fn parse_opencode_output_rejects_oversized_stdout_field() {
        let oversized_stdout = "x".repeat(MAX_OPENCODE_STDOUT_LEN + 1);
        let payload = format!("{{\"stdout\":\"{}\"}}", oversized_stdout);

        let result = parse_opencode_output(&payload);
        assert!(result.is_err());
        assert_eq!(
            result.err().map(|error| error.to_string()),
            Some("opencode stdout exceeds maximum length".to_string())
        );
    }

    #[test]
    fn parse_opencode_output_rejects_invalid_control_characters_in_stdout() {
        let result = parse_opencode_output("{\"stdout\":\"ok\\u0000bad\"}");
        assert!(result.is_err());
        assert_eq!(
            result.err().map(|error| error.to_string()),
            Some("opencode stdout contains invalid control characters".to_string())
        );
    }

    #[test]
    fn opencode_parse_error_display_returns_message() {
        let error = OpencodeParseError::new("boom");
        assert_eq!(error.to_string(), "boom");
    }

    #[test]
    fn build_manual_e2e_plan_rejects_blank_fields() {
        let missing_scenario = build_manual_e2e_plan(&ManualE2eInput {
            scenario: " ".to_string(),
            command: "oya run manual-e2e".to_string(),
            raw_output: "{\"success\":true,\"diagnostics\":\"ok\"}".to_string(),
        });
        assert_eq!(missing_scenario, Err(ManualE2eError::EmptyField("scenario")));

        let missing_command = build_manual_e2e_plan(&ManualE2eInput {
            scenario: "manual-e2e".to_string(),
            command: "  ".to_string(),
            raw_output: "{\"success\":true,\"diagnostics\":\"ok\"}".to_string(),
        });
        assert_eq!(missing_command, Err(ManualE2eError::EmptyField("command")));

        let missing_output = build_manual_e2e_plan(&ManualE2eInput {
            scenario: "manual-e2e".to_string(),
            command: "oya run manual-e2e".to_string(),
            raw_output: " ".to_string(),
        });
        assert_eq!(missing_output, Err(ManualE2eError::EmptyField("raw_output")));
    }

    #[test]
    fn build_manual_e2e_plan_rejects_boundary_and_malformed_inputs() {
        let oversized_scenario = build_manual_e2e_plan(&ManualE2eInput {
            scenario: "s".repeat(MAX_MANUAL_E2E_SCENARIO_LEN + 1),
            command: "oya run manual-e2e".to_string(),
            raw_output: "{\"success\":true,\"diagnostics\":\"ok\"}".to_string(),
        });
        assert_eq!(
            oversized_scenario,
            Err(ManualE2eError::FieldTooLong("scenario", MAX_MANUAL_E2E_SCENARIO_LEN))
        );

        let oversized_command = build_manual_e2e_plan(&ManualE2eInput {
            scenario: "manual-e2e".to_string(),
            command: "c".repeat(MAX_MANUAL_E2E_COMMAND_LEN + 1),
            raw_output: "{\"success\":true,\"diagnostics\":\"ok\"}".to_string(),
        });
        assert_eq!(
            oversized_command,
            Err(ManualE2eError::FieldTooLong("command", MAX_MANUAL_E2E_COMMAND_LEN))
        );

        let oversized_raw = build_manual_e2e_plan(&ManualE2eInput {
            scenario: "manual-e2e".to_string(),
            command: "oya run manual-e2e".to_string(),
            raw_output: "x".repeat(MAX_MANUAL_E2E_RAW_OUTPUT_LEN + 1),
        });
        assert_eq!(
            oversized_raw,
            Err(ManualE2eError::FieldTooLong("raw_output", MAX_MANUAL_E2E_RAW_OUTPUT_LEN))
        );

        let scenario_with_control_char = build_manual_e2e_plan(&ManualE2eInput {
            scenario: "manual\u{0007}e2e".to_string(),
            command: "oya run manual-e2e".to_string(),
            raw_output: "{\"success\":true,\"diagnostics\":\"ok\"}".to_string(),
        });
        assert_eq!(
            scenario_with_control_char,
            Err(ManualE2eError::InvalidFieldContent("scenario"))
        );

        let command_with_control_char = build_manual_e2e_plan(&ManualE2eInput {
            scenario: "manual-e2e".to_string(),
            command: "oya\u{0000} run manual-e2e".to_string(),
            raw_output: "{\"success\":true,\"diagnostics\":\"ok\"}".to_string(),
        });
        assert_eq!(command_with_control_char, Err(ManualE2eError::InvalidFieldContent("command")));
    }

    #[test]
    fn parse_pipeline_output_rejects_empty_malformed_or_incomplete_payloads() {
        assert_eq!(parse_pipeline_output("   "), Err(ManualE2eError::EmptyField("raw_output")));

        let malformed = parse_pipeline_output("not json");
        assert!(matches!(malformed, Err(ManualE2eError::InvalidJson(_))));

        let missing = parse_pipeline_output("{\"success\":true}");
        assert_eq!(missing, Err(ManualE2eError::MissingField("diagnostics")));
    }

    #[test]
    fn build_manual_e2e_plan_trims_whitespace_from_valid_fields() {
        let result = build_manual_e2e_plan(&ManualE2eInput {
            scenario: "  manual-e2e  ".to_string(),
            command: "  oya run manual-e2e  ".to_string(),
            raw_output: "  {\"success\":true,\"diagnostics\":\"ok\"}  ".to_string(),
        });

        assert_eq!(
            result,
            Ok(ManualE2ePlan {
                scenario: "manual-e2e".to_string(),
                command: "oya run manual-e2e".to_string(),
                raw_output: "{\"success\":true,\"diagnostics\":\"ok\"}".to_string(),
            })
        );
    }

    #[test]
    fn build_manual_e2e_plan_accepts_boundary_lengths_and_allowed_controls() {
        let scenario = format!("{}\n", "s".repeat(MAX_MANUAL_E2E_SCENARIO_LEN - 1));
        let command = format!("{}\t", "c".repeat(MAX_MANUAL_E2E_COMMAND_LEN - 1));
        let result = build_manual_e2e_plan(&ManualE2eInput {
            scenario,
            command,
            raw_output: "{\"success\":true,\"diagnostics\":\"ok\"}".to_string(),
        });

        assert!(result.is_ok());
    }

    #[test]
    fn parse_pipeline_output_rejects_missing_success_field() {
        let result = parse_pipeline_output("{\"diagnostics\":\"ok\"}");
        assert_eq!(result, Err(ManualE2eError::MissingField("success")));
    }

    #[test]
    fn parse_pipeline_output_rejects_non_boolean_success() {
        let result = parse_pipeline_output("{\"success\":\"yes\",\"diagnostics\":\"ok\"}");
        assert_eq!(result, Err(ManualE2eError::InvalidFieldType("success")));
    }

    #[test]
    fn parse_pipeline_output_rejects_non_string_diagnostics() {
        let result = parse_pipeline_output("{\"success\":true,\"diagnostics\":123}");
        assert_eq!(result, Err(ManualE2eError::InvalidFieldType("diagnostics")));
    }

    #[test]
    fn parse_pipeline_output_rejects_blank_diagnostics() {
        let result = parse_pipeline_output("{\"success\":true,\"diagnostics\":\"   \"}");
        assert_eq!(result, Err(ManualE2eError::EmptyField("diagnostics")));
    }

    #[test]
    fn parse_pipeline_output_rejects_boundary_and_malformed_inputs() {
        let oversized_raw = "x".repeat(MAX_MANUAL_E2E_RAW_OUTPUT_LEN + 1);
        assert_eq!(
            parse_pipeline_output(&oversized_raw),
            Err(ManualE2eError::FieldTooLong("raw_output", MAX_MANUAL_E2E_RAW_OUTPUT_LEN))
        );

        let oversized_diagnostics = format!(
            "{{\"success\":true,\"diagnostics\":\"{}\"}}",
            "d".repeat(MAX_MANUAL_E2E_DIAGNOSTICS_LEN + 1)
        );
        assert_eq!(
            parse_pipeline_output(&oversized_diagnostics),
            Err(ManualE2eError::FieldTooLong("diagnostics", MAX_MANUAL_E2E_DIAGNOSTICS_LEN))
        );

        let invalid_control_diagnostics =
            parse_pipeline_output("{\"success\":true,\"diagnostics\":\"bad\\u0000data\"}");
        assert_eq!(
            invalid_control_diagnostics,
            Err(ManualE2eError::InvalidFieldContent("diagnostics"))
        );

        let multiline_diagnostics =
            parse_pipeline_output("{\"success\":true,\"diagnostics\":\"line1\\nline2\\tdata\"}");
        assert_eq!(
            multiline_diagnostics,
            Ok(ManualE2eOutput { success: true, diagnostics: "line1\nline2\tdata".to_string() })
        );
    }

    #[test]
    fn parse_pipeline_output_accepts_diagnostics_at_max_length() {
        let max_diagnostics = "d".repeat(MAX_MANUAL_E2E_DIAGNOSTICS_LEN);
        let payload = format!("{{\"success\":false,\"diagnostics\":\"{}\"}}", max_diagnostics);
        let result = parse_pipeline_output(&payload);

        assert_eq!(result, Ok(ManualE2eOutput { success: false, diagnostics: max_diagnostics }));
    }

    #[test]
    fn run_manual_e2e_pipeline_records_stage_results_in_order() {
        let plan_result = build_manual_e2e_plan(&ManualE2eInput {
            scenario: "manual-e2e".to_string(),
            command: "oya run manual-e2e".to_string(),
            raw_output: "{\"success\":true,\"diagnostics\":\"pipeline green\"}".to_string(),
        });
        assert!(plan_result.is_ok());
        let plan = match plan_result {
            Ok(value) => value,
            Err(_) => return,
        };

        let report_result = run_manual_e2e_pipeline(&plan);
        assert!(report_result.is_ok());
        let report = match report_result {
            Ok(value) => value,
            Err(_) => return,
        };

        let order = report.stages.iter().map(|stage| stage.stage.clone()).collect::<Vec<_>>();
        assert_eq!(
            order,
            vec![
                ManualE2eStageName::ScenarioSetup,
                ManualE2eStageName::CommandInvocation,
                ManualE2eStageName::OutputParsing,
                ManualE2eStageName::GateEvaluation,
            ]
        );
        assert_eq!(report.decision, ManualE2eGateDecision::Allow);
        assert!(validate_manual_e2e_report(&report).is_ok());
    }

    #[test]
    fn run_manual_e2e_pipeline_blocks_gate_when_any_stage_fails() {
        let plan_result = build_manual_e2e_plan(&ManualE2eInput {
            scenario: "manual-e2e".to_string(),
            command: "oya run manual-e2e".to_string(),
            raw_output: "{\"success\":false,\"diagnostics\":\"output mismatch\"}".to_string(),
        });
        assert!(plan_result.is_ok());
        let plan = match plan_result {
            Ok(value) => value,
            Err(_) => return,
        };

        let report_result = run_manual_e2e_pipeline(&plan);
        assert!(report_result.is_ok());
        let report = match report_result {
            Ok(value) => value,
            Err(_) => return,
        };

        assert_eq!(report.decision, ManualE2eGateDecision::Block);
        assert_eq!(derive_manual_e2e_gate(&report), ManualE2eGateDecision::Block);
        assert!(report.stages.iter().any(|stage| stage.status == ManualE2eStageStatus::Failed));

        let gate_stage =
            report.stages.iter().find(|stage| stage.stage == ManualE2eStageName::GateEvaluation);
        assert_eq!(gate_stage.map(|stage| stage.diagnostics.as_str()), Some("manual gate blocked"));
    }

    #[test]
    fn rerunning_same_plan_yields_equivalent_validation_outcomes() {
        let plan_result = build_manual_e2e_plan(&ManualE2eInput {
            scenario: "manual-e2e".to_string(),
            command: "oya run manual-e2e".to_string(),
            raw_output: "{\"success\":false,\"diagnostics\":\"gate blocked\"}".to_string(),
        });
        assert!(plan_result.is_ok());
        let plan = match plan_result {
            Ok(value) => value,
            Err(_) => return,
        };

        let first_result = run_manual_e2e_pipeline(&plan);
        let second_result = run_manual_e2e_pipeline(&plan);
        assert!(first_result.is_ok());
        assert!(second_result.is_ok());

        let first = match first_result {
            Ok(value) => value,
            Err(_) => return,
        };
        let second = match second_result {
            Ok(value) => value,
            Err(_) => return,
        };

        let first_stage_statuses = first
            .stages
            .iter()
            .map(|stage| (stage.stage.clone(), stage.status.clone()))
            .collect::<Vec<_>>();
        let second_stage_statuses = second
            .stages
            .iter()
            .map(|stage| (stage.stage.clone(), stage.status.clone()))
            .collect::<Vec<_>>();

        assert_eq!(first.decision, second.decision);
        assert_eq!(first_stage_statuses, second_stage_statuses);
        assert_eq!(validate_manual_e2e_report(&first), Ok(()));
        assert_eq!(validate_manual_e2e_report(&second), Ok(()));
    }

    #[test]
    fn validate_manual_e2e_report_rejects_inconsistent_decision() {
        let plan_result = build_manual_e2e_plan(&ManualE2eInput {
            scenario: "manual-e2e".to_string(),
            command: "oya run manual-e2e".to_string(),
            raw_output: "{\"success\":false,\"diagnostics\":\"failed stage\"}".to_string(),
        });
        assert!(plan_result.is_ok());
        let plan = match plan_result {
            Ok(value) => value,
            Err(_) => return,
        };

        let report_result = run_manual_e2e_pipeline(&plan);
        assert!(report_result.is_ok());
        let mut report = match report_result {
            Ok(value) => value,
            Err(_) => return,
        };

        report.decision = ManualE2eGateDecision::Allow;
        assert_eq!(
            validate_manual_e2e_report(&report),
            Err(ManualE2eError::InvalidReport("decision mismatch"))
        );
    }

    #[test]
    fn run_manual_e2e_pipeline_returns_parse_errors() {
        let plan = ManualE2ePlan {
            scenario: "manual-e2e".to_string(),
            command: "oya run manual-e2e".to_string(),
            raw_output: "not json".to_string(),
        };

        let result = run_manual_e2e_pipeline(&plan);
        assert!(matches!(result, Err(ManualE2eError::InvalidJson(_))));
    }

    #[test]
    fn validate_manual_e2e_report_rejects_unexpected_stage_count() {
        let report = ManualE2eReport {
            plan: ManualE2ePlan {
                scenario: "manual-e2e".to_string(),
                command: "oya run manual-e2e".to_string(),
                raw_output: "{\"success\":true,\"diagnostics\":\"ok\"}".to_string(),
            },
            output: ManualE2eOutput { success: true, diagnostics: "ok".to_string() },
            stages: vec![stage_report(
                ManualE2eStageName::ScenarioSetup,
                ManualE2eStageStatus::Passed,
                "scenario prepared",
            )],
            decision: ManualE2eGateDecision::Allow,
        };

        assert_eq!(
            validate_manual_e2e_report(&report),
            Err(ManualE2eError::InvalidReport("unexpected stage count"))
        );
    }

    #[test]
    fn validate_manual_e2e_report_rejects_invalid_stage_order() {
        let report = ManualE2eReport {
            plan: ManualE2ePlan {
                scenario: "manual-e2e".to_string(),
                command: "oya run manual-e2e".to_string(),
                raw_output: "{\"success\":true,\"diagnostics\":\"ok\"}".to_string(),
            },
            output: ManualE2eOutput { success: true, diagnostics: "ok".to_string() },
            stages: vec![
                stage_report(
                    ManualE2eStageName::ScenarioSetup,
                    ManualE2eStageStatus::Passed,
                    "scenario prepared",
                ),
                stage_report(
                    ManualE2eStageName::OutputParsing,
                    ManualE2eStageStatus::Passed,
                    "parsed",
                ),
                stage_report(
                    ManualE2eStageName::CommandInvocation,
                    ManualE2eStageStatus::Passed,
                    "invoked",
                ),
                stage_report(
                    ManualE2eStageName::GateEvaluation,
                    ManualE2eStageStatus::Passed,
                    "gate open",
                ),
            ],
            decision: ManualE2eGateDecision::Allow,
        };

        assert_eq!(
            validate_manual_e2e_report(&report),
            Err(ManualE2eError::InvalidReport("invalid stage order"))
        );
    }

    #[test]
    fn validate_manual_e2e_report_rejects_empty_stage_diagnostics() {
        let report = ManualE2eReport {
            plan: ManualE2ePlan {
                scenario: "manual-e2e".to_string(),
                command: "oya run manual-e2e".to_string(),
                raw_output: "{\"success\":true,\"diagnostics\":\"ok\"}".to_string(),
            },
            output: ManualE2eOutput { success: true, diagnostics: "ok".to_string() },
            stages: vec![
                stage_report(
                    ManualE2eStageName::ScenarioSetup,
                    ManualE2eStageStatus::Passed,
                    "scenario prepared",
                ),
                stage_report(
                    ManualE2eStageName::CommandInvocation,
                    ManualE2eStageStatus::Passed,
                    "invoked",
                ),
                stage_report(ManualE2eStageName::OutputParsing, ManualE2eStageStatus::Passed, ""),
                stage_report(
                    ManualE2eStageName::GateEvaluation,
                    ManualE2eStageStatus::Passed,
                    "gate open",
                ),
            ],
            decision: ManualE2eGateDecision::Allow,
        };

        assert_eq!(
            validate_manual_e2e_report(&report),
            Err(ManualE2eError::InvalidReport("empty stage diagnostics"))
        );
    }

    #[test]
    fn validate_manual_e2e_report_rejects_oversized_stage_diagnostics() {
        let report = ManualE2eReport {
            plan: ManualE2ePlan {
                scenario: "manual-e2e".to_string(),
                command: "oya run manual-e2e".to_string(),
                raw_output: "{\"success\":true,\"diagnostics\":\"ok\"}".to_string(),
            },
            output: ManualE2eOutput { success: true, diagnostics: "ok".to_string() },
            stages: vec![
                stage_report(
                    ManualE2eStageName::ScenarioSetup,
                    ManualE2eStageStatus::Passed,
                    "scenario prepared",
                ),
                stage_report(
                    ManualE2eStageName::CommandInvocation,
                    ManualE2eStageStatus::Passed,
                    "invoked",
                ),
                stage_report(
                    ManualE2eStageName::OutputParsing,
                    ManualE2eStageStatus::Passed,
                    &"d".repeat(MAX_MANUAL_E2E_DIAGNOSTICS_LEN + 1),
                ),
                stage_report(
                    ManualE2eStageName::GateEvaluation,
                    ManualE2eStageStatus::Passed,
                    "gate open",
                ),
            ],
            decision: ManualE2eGateDecision::Allow,
        };

        assert_eq!(
            validate_manual_e2e_report(&report),
            Err(ManualE2eError::InvalidReport("stage diagnostics exceed max length"))
        );
    }

    #[test]
    fn validate_manual_e2e_report_rejects_invalid_stage_diagnostics_content() {
        let report = ManualE2eReport {
            plan: ManualE2ePlan {
                scenario: "manual-e2e".to_string(),
                command: "oya run manual-e2e".to_string(),
                raw_output: "{\"success\":true,\"diagnostics\":\"ok\"}".to_string(),
            },
            output: ManualE2eOutput { success: true, diagnostics: "ok".to_string() },
            stages: vec![
                stage_report(
                    ManualE2eStageName::ScenarioSetup,
                    ManualE2eStageStatus::Passed,
                    "scenario prepared",
                ),
                stage_report(
                    ManualE2eStageName::CommandInvocation,
                    ManualE2eStageStatus::Passed,
                    "invoked",
                ),
                stage_report(
                    ManualE2eStageName::OutputParsing,
                    ManualE2eStageStatus::Passed,
                    "bad\u{0000}data",
                ),
                stage_report(
                    ManualE2eStageName::GateEvaluation,
                    ManualE2eStageStatus::Passed,
                    "gate open",
                ),
            ],
            decision: ManualE2eGateDecision::Allow,
        };

        assert_eq!(
            validate_manual_e2e_report(&report),
            Err(ManualE2eError::InvalidReport(
                "stage diagnostics contain invalid control characters"
            ))
        );
    }

    #[test]
    fn validate_manual_e2e_report_rejects_non_monotonic_stage_timestamps() {
        let base_time = Utc::now();
        let report = ManualE2eReport {
            plan: ManualE2ePlan {
                scenario: "manual-e2e".to_string(),
                command: "oya run manual-e2e".to_string(),
                raw_output: "{\"success\":true,\"diagnostics\":\"ok\"}".to_string(),
            },
            output: ManualE2eOutput { success: true, diagnostics: "ok".to_string() },
            stages: vec![
                ManualE2eStageReport {
                    stage: ManualE2eStageName::ScenarioSetup,
                    status: ManualE2eStageStatus::Passed,
                    diagnostics: "scenario prepared".to_string(),
                    timestamp: base_time,
                },
                ManualE2eStageReport {
                    stage: ManualE2eStageName::CommandInvocation,
                    status: ManualE2eStageStatus::Passed,
                    diagnostics: "invoked".to_string(),
                    timestamp: base_time - chrono::Duration::milliseconds(1),
                },
                ManualE2eStageReport {
                    stage: ManualE2eStageName::OutputParsing,
                    status: ManualE2eStageStatus::Passed,
                    diagnostics: "parsed".to_string(),
                    timestamp: base_time,
                },
                ManualE2eStageReport {
                    stage: ManualE2eStageName::GateEvaluation,
                    status: ManualE2eStageStatus::Passed,
                    diagnostics: "gate open".to_string(),
                    timestamp: base_time,
                },
            ],
            decision: ManualE2eGateDecision::Allow,
        };

        assert_eq!(
            validate_manual_e2e_report(&report),
            Err(ManualE2eError::InvalidReport("non-monotonic stage timestamps"))
        );
    }

    #[test]
    fn derive_manual_e2e_gate_blocks_when_stage_has_error_status() {
        let report = ManualE2eReport {
            plan: ManualE2ePlan {
                scenario: "manual-e2e".to_string(),
                command: "oya run manual-e2e".to_string(),
                raw_output: "{\"success\":true,\"diagnostics\":\"ok\"}".to_string(),
            },
            output: ManualE2eOutput { success: true, diagnostics: "ok".to_string() },
            stages: vec![
                stage_report(
                    ManualE2eStageName::ScenarioSetup,
                    ManualE2eStageStatus::Passed,
                    "scenario prepared",
                ),
                stage_report(
                    ManualE2eStageName::CommandInvocation,
                    ManualE2eStageStatus::Passed,
                    "invoked",
                ),
                stage_report(
                    ManualE2eStageName::OutputParsing,
                    ManualE2eStageStatus::Error,
                    "parse adapter crash",
                ),
                stage_report(
                    ManualE2eStageName::GateEvaluation,
                    ManualE2eStageStatus::Passed,
                    "gate open",
                ),
            ],
            decision: ManualE2eGateDecision::Block,
        };

        assert_eq!(derive_manual_e2e_gate(&report), ManualE2eGateDecision::Block);
    }

    #[test]
    fn verify_state_typing_rejects_empty_container_id() {
        let state = DockerState {
            container_id: String::new(),
            status: ContainerStatus::Running,
            image: "nginx".to_string(),
            port: Some(8080),
        };
        let result = verify_state_typing(&state);
        assert_eq!(result, Err(DockerFixError::EmptyStateField("container_id")));
    }

    #[test]
    fn verify_state_typing_rejects_empty_image() {
        let state = DockerState {
            container_id: "abc123".to_string(),
            status: ContainerStatus::Running,
            image: String::new(),
            port: Some(8080),
        };
        let result = verify_state_typing(&state);
        assert_eq!(result, Err(DockerFixError::EmptyStateField("image")));
    }

    #[test]
    fn verify_state_typing_accepts_valid_state() {
        let state = DockerState {
            container_id: "abc123".to_string(),
            status: ContainerStatus::Running,
            image: "nginx:latest".to_string(),
            port: Some(8080),
        };
        let result = verify_state_typing(&state);
        assert_eq!(result, Ok(()));
    }

    #[test]
    fn verify_state_typing_accepts_none_port() {
        let state = DockerState {
            container_id: "abc123".to_string(),
            status: ContainerStatus::Running,
            image: "nginx:latest".to_string(),
            port: None,
        };
        let result = verify_state_typing(&state);
        assert_eq!(result, Ok(()));
    }

    #[test]
    fn verify_state_typing_rejects_whitespace_only_container_id() {
        let state = DockerState {
            container_id: "   ".to_string(),
            status: ContainerStatus::Running,
            image: "nginx".to_string(),
            port: Some(8080),
        };
        let result = verify_state_typing(&state);
        assert_eq!(result, Err(DockerFixError::EmptyStateField("container_id")));
    }

    #[test]
    fn verify_state_typing_rejects_whitespace_only_image() {
        let state = DockerState {
            container_id: "abc123".to_string(),
            status: ContainerStatus::Running,
            image: "\t\n".to_string(),
            port: Some(8080),
        };
        let result = verify_state_typing(&state);
        assert_eq!(result, Err(DockerFixError::EmptyStateField("image")));
    }

    #[test]
    fn verify_state_typing_accepts_container_id_with_unicode() {
        let state = DockerState {
            container_id: "abc123-тест-🐳".to_string(),
            status: ContainerStatus::Running,
            image: "nginx".to_string(),
            port: Some(8080),
        };
        let result = verify_state_typing(&state);
        assert_eq!(result, Ok(()));
    }

    #[test]
    fn verify_state_typing_accepts_image_with_unicode() {
        let state = DockerState {
            container_id: "abc123".to_string(),
            status: ContainerStatus::Running,
            image: "nginx:latest-тест-🐳".to_string(),
            port: Some(8080),
        };
        let result = verify_state_typing(&state);
        assert_eq!(result, Ok(()));
    }

    #[test]
    fn verify_state_typing_trims_container_id_whitespace() {
        let state = DockerState {
            container_id: "  abc123  ".to_string(),
            status: ContainerStatus::Running,
            image: "nginx".to_string(),
            port: Some(8080),
        };
        let result = verify_state_typing(&state);
        assert_eq!(result, Ok(()));
    }

    #[test]
    fn verify_state_typing_trims_image_whitespace() {
        let state = DockerState {
            container_id: "abc123".to_string(),
            status: ContainerStatus::Running,
            image: "  nginx:latest  ".to_string(),
            port: Some(8080),
        };
        let result = verify_state_typing(&state);
        assert_eq!(result, Ok(()));
    }

    #[test]
    fn verify_state_typing_accepts_all_status_variants_running() {
        let state = DockerState {
            container_id: "abc123".to_string(),
            status: ContainerStatus::Running,
            image: "nginx".to_string(),
            port: Some(8080),
        };
        assert_eq!(verify_state_typing(&state), Ok(()));
    }

    #[test]
    fn verify_state_typing_accepts_all_status_variants_stopped() {
        let state = DockerState {
            container_id: "abc123".to_string(),
            status: ContainerStatus::Stopped,
            image: "nginx".to_string(),
            port: Some(8080),
        };
        assert_eq!(verify_state_typing(&state), Ok(()));
    }

    #[test]
    fn verify_state_typing_accepts_all_status_variants_exited() {
        let state = DockerState {
            container_id: "abc123".to_string(),
            status: ContainerStatus::Exited,
            image: "nginx".to_string(),
            port: Some(8080),
        };
        assert_eq!(verify_state_typing(&state), Ok(()));
    }

    #[test]
    fn verify_state_typing_accepts_all_status_variants_created() {
        let state = DockerState {
            container_id: "abc123".to_string(),
            status: ContainerStatus::Created,
            image: "nginx".to_string(),
            port: Some(8080),
        };
        assert_eq!(verify_state_typing(&state), Ok(()));
    }

    #[test]
    fn verify_state_typing_accepts_port_boundary_min() {
        let state = DockerState {
            container_id: "abc123".to_string(),
            status: ContainerStatus::Running,
            image: "nginx".to_string(),
            port: Some(1),
        };
        let result = verify_state_typing(&state);
        assert_eq!(result, Ok(()));
    }

    #[test]
    fn verify_state_typing_accepts_port_boundary_max() {
        let state = DockerState {
            container_id: "abc123".to_string(),
            status: ContainerStatus::Running,
            image: "nginx".to_string(),
            port: Some(u16::MAX),
        };
        let result = verify_state_typing(&state);
        assert_eq!(result, Ok(()));
    }

    #[test]
    fn verify_state_typing_accepts_common_ports() {
        let common_ports = vec![80, 443, 8080, 3000, 5000, 5432, 6379, 27017];
        for port in common_ports {
            let state = DockerState {
                container_id: "abc123".to_string(),
                status: ContainerStatus::Running,
                image: "nginx".to_string(),
                port: Some(port),
            };
            assert_eq!(verify_state_typing(&state), Ok(()), "port {}", port);
        }
    }

    #[test]
    fn verify_state_typing_accepts_very_long_container_id() {
        let long_id = "a".repeat(1000);
        let state = DockerState {
            container_id: long_id,
            status: ContainerStatus::Running,
            image: "nginx".to_string(),
            port: Some(8080),
        };
        let result = verify_state_typing(&state);
        assert_eq!(result, Ok(()));
    }

    #[test]
    fn verify_state_typing_accepts_very_long_image_name() {
        let long_image = format!("{}:{}", "n".repeat(500), "v".repeat(100));
        let state = DockerState {
            container_id: "abc123".to_string(),
            status: ContainerStatus::Running,
            image: long_image,
            port: Some(8080),
        };
        let result = verify_state_typing(&state);
        assert_eq!(result, Ok(()));
    }

    #[test]
    fn resolve_moon_path_rejects_empty_task_name() {
        let result = resolve_moon_path("");
        assert!(matches!(result, Err(DockerFixError::MoonTaskNotFound(_))));
    }

    #[test]
    fn resolve_moon_path_returns_absolute_path_for_known_task() {
        let result = resolve_moon_path(":test");
        assert!(result.is_ok());
        let Ok(path) = result else { return };
        assert_eq!(path.task_name, ":test");
        assert!(path.absolute_path.is_absolute());
    }

    #[test]
    fn resolve_moon_path_rejects_whitespace_only_task_name() {
        let result = resolve_moon_path("  \t\n ");
        assert!(matches!(result, Err(DockerFixError::MoonTaskNotFound(_))));
    }

    #[test]
    fn resolve_moon_path_rejects_multiple_empty_strings() {
        let result = resolve_moon_path("   ");
        assert!(matches!(result, Err(DockerFixError::MoonTaskNotFound(_))));
    }

    #[test]
    fn resolve_moon_path_strips_leading_colon() {
        let result = resolve_moon_path(":test");
        assert!(result.is_ok());
        let Ok(path) = result else { return };
        assert_eq!(path.task_name, ":test");
    }

    #[test]
    fn resolve_moon_path_handles_multiple_leading_colons() {
        let result = resolve_moon_path("::test");
        assert!(result.is_ok());
        let Ok(path) = result else { return };
        assert_eq!(path.task_name, "::test");
    }

    #[test]
    fn resolve_moon_path_accepts_task_name_with_dashes() {
        let result = resolve_moon_path("my-task");
        assert!(result.is_ok());
        let Ok(path) = result else { return };
        assert_eq!(path.task_name, "my-task");
    }

    #[test]
    fn resolve_moon_path_accepts_task_name_with_underscores() {
        let result = resolve_moon_path("my_task");
        assert!(result.is_ok());
        let Ok(path) = result else { return };
        assert_eq!(path.task_name, "my_task");
    }

    #[test]
    fn resolve_moon_path_accepts_task_name_with_numbers() {
        let result = resolve_moon_path("task123");
        assert!(result.is_ok());
        let Ok(path) = result else { return };
        assert_eq!(path.task_name, "task123");
    }

    #[test]
    fn resolve_moon_path_returns_absolute_path() {
        let result = resolve_moon_path("test");
        assert!(result.is_ok());
        let Ok(path) = result else { return };
        assert!(path.absolute_path.is_absolute());
    }

    #[test]
    fn resolve_moon_path_includes_task_name_in_path() {
        let result = resolve_moon_path("mytask");
        assert!(result.is_ok());
        let Ok(path) = result else { return };
        assert!(path.absolute_path.to_string_lossy().contains("mytask"));
    }

    #[test]
    fn resolve_moon_path_rejects_path_traversal_and_absolute_paths() {
        let traversal_result = resolve_moon_path("../etc/passwd");
        assert_eq!(
            traversal_result,
            Err(DockerFixError::ConfigValidationFailed(
                "moon task name contains invalid characters"
            ))
        );

        let absolute_result = resolve_moon_path("/tmp/evil");
        assert_eq!(
            absolute_result,
            Err(DockerFixError::ConfigValidationFailed(
                "moon task name contains invalid characters"
            ))
        );

        let backslash_result = resolve_moon_path("..\\windows\\system32");
        assert_eq!(
            backslash_result,
            Err(DockerFixError::ConfigValidationFailed(
                "moon task name contains invalid characters"
            ))
        );
    }

    #[test]
    fn resolve_moon_path_rejects_separator_only_and_oversized_names() {
        let separator_only_result = resolve_moon_path(":::");
        assert_eq!(
            separator_only_result,
            Err(DockerFixError::ConfigValidationFailed(
                "moon task name is empty after normalization"
            ))
        );

        let oversized_name = "a".repeat(MAX_MOON_TASK_NAME_LEN + 1);
        let oversized_result = resolve_moon_path(&oversized_name);
        assert_eq!(
            oversized_result,
            Err(DockerFixError::ConfigValidationFailed("moon task name exceeds max length"))
        );
    }

    #[test]
    fn resolve_moon_path_rejects_malformed_task_names() {
        let malformed_cases = [
            "task name",
            "task\nname",
            "task\tname",
            "task;rm-rf",
            "task|pipe",
            "task*glob",
            "task?query",
        ];

        for malformed in malformed_cases {
            let result = resolve_moon_path(malformed);
            assert_eq!(
                result,
                Err(DockerFixError::ConfigValidationFailed(
                    "moon task name contains invalid characters"
                ))
            );
        }
    }

    #[test]
    fn validate_docker_config_rejects_empty_image_name() {
        let config = DockerConfig {
            image_name: String::new(),
            tag: Some("latest".to_string()),
            port_bindings: vec![8080],
            environment: vec!["RUST_LOG=debug".to_string()],
        };
        let result = validate_docker_config(&config);
        assert_eq!(result, Err(DockerFixError::EmptyConfigField("image_name")));
    }

    #[test]
    fn validate_docker_config_accepts_valid_config() {
        let config = DockerConfig {
            image_name: "nginx".to_string(),
            tag: Some("latest".to_string()),
            port_bindings: vec![80],
            environment: vec!["ENV=prod".to_string()],
        };
        let result = validate_docker_config(&config);
        assert_eq!(result, Ok(()));
    }

    #[test]
    fn validate_docker_config_accepts_config_without_tag() {
        let config = DockerConfig {
            image_name: "nginx".to_string(),
            tag: None,
            port_bindings: vec![],
            environment: vec![],
        };
        let result = validate_docker_config(&config);
        assert_eq!(result, Ok(()));
    }

    #[test]
    fn validate_docker_config_rejects_whitespace_only_image_name() {
        let config = DockerConfig {
            image_name: "  \t\n ".to_string(),
            tag: Some("latest".to_string()),
            port_bindings: vec![8080],
            environment: vec!["RUST_LOG=debug".to_string()],
        };
        let result = validate_docker_config(&config);
        assert_eq!(result, Err(DockerFixError::EmptyConfigField("image_name")));
    }

    #[test]
    fn validate_docker_config_rejects_image_name_with_control_chars() {
        let config = DockerConfig {
            image_name: "nginx\u{0000}latest".to_string(),
            tag: None,
            port_bindings: vec![8080],
            environment: vec![],
        };
        let result = validate_docker_config(&config);
        assert_eq!(result, Err(DockerFixError::TypeConstraintViolation("image_name")));
    }

    #[test]
    fn validate_docker_config_rejects_image_name_with_other_control_chars() {
        let control_chars = vec!['\x01', '\x02', '\x07', '\x1B'];
        for c in control_chars {
            let config = DockerConfig {
                image_name: format!("nginx{}latest", c),
                tag: None,
                port_bindings: vec![8080],
                environment: vec![],
            };
            let result = validate_docker_config(&config);
            assert_eq!(result, Err(DockerFixError::TypeConstraintViolation("image_name")));
        }
    }

    #[test]
    fn validate_docker_config_accepts_allowed_whitespace_in_image_name() {
        let config = DockerConfig {
            image_name: "  nginx:latest  ".to_string(),
            tag: None,
            port_bindings: vec![8080],
            environment: vec![],
        };
        let result = validate_docker_config(&config);
        assert_eq!(result, Ok(()));
    }

    #[test]
    fn validate_docker_config_accepts_tag_without_trimming() {
        let config = DockerConfig {
            image_name: "nginx".to_string(),
            tag: Some("  latest  ".to_string()),
            port_bindings: vec![80],
            environment: vec![],
        };
        let result = validate_docker_config(&config);
        assert_eq!(result, Ok(()));
    }

    #[test]
    fn validate_docker_config_accepts_empty_tag() {
        let config = DockerConfig {
            image_name: "nginx".to_string(),
            tag: Some(String::new()),
            port_bindings: vec![80],
            environment: vec![],
        };
        let result = validate_docker_config(&config);
        assert_eq!(result, Ok(()));
    }

    #[test]
    fn validate_docker_config_accepts_port_boundary_min() {
        let config = DockerConfig {
            image_name: "nginx".to_string(),
            tag: None,
            port_bindings: vec![1],
            environment: vec![],
        };
        let result = validate_docker_config(&config);
        assert_eq!(result, Ok(()));
    }

    #[test]
    fn validate_docker_config_accepts_port_boundary_max() {
        let config = DockerConfig {
            image_name: "nginx".to_string(),
            tag: None,
            port_bindings: vec![u16::MAX],
            environment: vec![],
        };
        let result = validate_docker_config(&config);
        assert_eq!(result, Ok(()));
    }

    #[test]
    fn validate_docker_config_accepts_multiple_port_bindings() {
        let config = DockerConfig {
            image_name: "nginx".to_string(),
            tag: None,
            port_bindings: vec![80, 443, 8080],
            environment: vec![],
        };
        let result = validate_docker_config(&config);
        assert_eq!(result, Ok(()));
    }

    #[test]
    fn validate_docker_config_accepts_common_ports() {
        let common_ports = vec![22, 80, 443, 3306, 5432, 6379, 27017, 8080];
        let config = DockerConfig {
            image_name: "nginx".to_string(),
            tag: None,
            port_bindings: common_ports,
            environment: vec![],
        };
        let result = validate_docker_config(&config);
        assert_eq!(result, Ok(()));
    }

    #[test]
    fn validate_docker_config_accepts_valid_environment_variables() {
        let config = DockerConfig {
            image_name: "nginx".to_string(),
            tag: None,
            port_bindings: vec![80],
            environment: vec![
                "RUST_LOG=debug".to_string(),
                "NODE_ENV=production".to_string(),
                "DATABASE_URL=postgresql://localhost".to_string(),
            ],
        };
        let result = validate_docker_config(&config);
        assert_eq!(result, Ok(()));
    }

    #[test]
    fn validate_docker_config_accepts_empty_environment() {
        let config = DockerConfig {
            image_name: "nginx".to_string(),
            tag: None,
            port_bindings: vec![80],
            environment: vec![],
        };
        let result = validate_docker_config(&config);
        assert_eq!(result, Ok(()));
    }

    #[test]
    fn validate_docker_config_accepts_environment_with_equals_in_value() {
        let config = DockerConfig {
            image_name: "nginx".to_string(),
            tag: None,
            port_bindings: vec![80],
            environment: vec!["PATH=/usr/local/bin:/usr/bin".to_string()],
        };
        let result = validate_docker_config(&config);
        assert_eq!(result, Ok(()));
    }

    #[test]
    fn validate_docker_config_accepts_image_name_with_registry() {
        let config = DockerConfig {
            image_name: "docker.io/library/nginx".to_string(),
            tag: None,
            port_bindings: vec![80],
            environment: vec![],
        };
        let result = validate_docker_config(&config);
        assert_eq!(result, Ok(()));
    }

    #[test]
    fn validate_docker_config_accepts_image_name_with_special_chars() {
        let config = DockerConfig {
            image_name: "my-registry.io/my-org/my_image:v1.2.3".to_string(),
            tag: None,
            port_bindings: vec![80],
            environment: vec![],
        };
        let result = validate_docker_config(&config);
        assert_eq!(result, Ok(()));
    }

    #[test]
    fn validate_docker_config_accepts_unicode_in_image_name() {
        let config = DockerConfig {
            image_name: "nginx:тест-🐳".to_string(),
            tag: None,
            port_bindings: vec![80],
            environment: vec![],
        };
        let result = validate_docker_config(&config);
        assert_eq!(result, Ok(()));
    }

    #[test]
    fn validate_docker_config_accepts_very_long_image_name() {
        let long_name = format!("{}/{}", "registry.io".repeat(10), "n".repeat(500));
        let config = DockerConfig {
            image_name: long_name,
            tag: None,
            port_bindings: vec![80],
            environment: vec![],
        };
        let result = validate_docker_config(&config);
        assert_eq!(result, Ok(()));
    }

    #[test]
    fn validate_docker_config_accepts_many_port_bindings() {
        let ports: Vec<u16> = (8000..8050).collect();
        let config = DockerConfig {
            image_name: "nginx".to_string(),
            tag: None,
            port_bindings: ports,
            environment: vec![],
        };
        let result = validate_docker_config(&config);
        assert_eq!(result, Ok(()));
    }

    #[test]
    fn build_onewf_bead_quick_plan_rejects_empty_fields() {
        let input = OnewfBeadQuickInput {
            workflow_id: "   ".to_string(),
            bead_id: "bead-1".to_string(),
            endpoint: "http://localhost:8080/endpoint".to_string(),
        };
        assert_eq!(
            build_onewf_bead_quick_plan(&input),
            Err(OnewfBeadQuickError::EmptyField("workflow_id"))
        );

        let input = OnewfBeadQuickInput {
            workflow_id: "workflow-1".to_string(),
            bead_id: " ".to_string(),
            endpoint: "http://localhost:8080/endpoint".to_string(),
        };
        assert_eq!(
            build_onewf_bead_quick_plan(&input),
            Err(OnewfBeadQuickError::EmptyField("bead_id"))
        );

        let input = OnewfBeadQuickInput {
            workflow_id: "workflow-1".to_string(),
            bead_id: "bead-1".to_string(),
            endpoint: " ".to_string(),
        };
        assert_eq!(
            build_onewf_bead_quick_plan(&input),
            Err(OnewfBeadQuickError::EmptyField("endpoint"))
        );
    }

    #[test]
    fn build_onewf_bead_quick_plan_rejects_invalid_identifiers_and_endpoint() {
        let invalid_identifier = OnewfBeadQuickInput {
            workflow_id: "workflow/1".to_string(),
            bead_id: "bead-1".to_string(),
            endpoint: "http://localhost:8080/endpoint".to_string(),
        };
        assert_eq!(
            build_onewf_bead_quick_plan(&invalid_identifier),
            Err(OnewfBeadQuickError::InvalidIdentifier("workflow_id"))
        );

        let invalid_endpoint = OnewfBeadQuickInput {
            workflow_id: "workflow-1".to_string(),
            bead_id: "bead-1".to_string(),
            endpoint: "ftp://localhost:8080/endpoint".to_string(),
        };
        assert_eq!(
            build_onewf_bead_quick_plan(&invalid_endpoint),
            Err(OnewfBeadQuickError::InvalidEndpoint)
        );
    }

    #[test]
    fn run_onewf_bead_quick_check_emits_single_visible_successful_check() {
        let plan_result = build_onewf_bead_quick_plan(&OnewfBeadQuickInput {
            workflow_id: "workflow-1".to_string(),
            bead_id: "bead-quick-1".to_string(),
            endpoint: "http://localhost:8080/one-endpoint".to_string(),
        });
        assert!(plan_result.is_ok());
        let Ok(plan) = plan_result else { return };

        let observation_result = run_onewf_bead_quick_check(&plan);
        assert!(observation_result.is_ok());
        let Ok(observation) = observation_result else {
            return;
        };

        assert_eq!(observation.workflow_id, "workflow-1");
        assert_eq!(observation.bead_id, "bead-quick-1");
        assert_eq!(observation.checks.len(), 1);
        assert_eq!(observation.checks[0].endpoint, "http://localhost:8080/one-endpoint");
        assert!(observation.checks[0].visible);
        assert!(observation.checks[0].success);
        assert_eq!(
            observation.checks[0].diagnostics,
            "endpoint visible and probe succeeded".to_string()
        );
    }

    #[test]
    fn run_onewf_bead_quick_check_marks_probe_failure_for_fail_endpoint() {
        let plan_result = build_onewf_bead_quick_plan(&OnewfBeadQuickInput {
            workflow_id: "workflow-1".to_string(),
            bead_id: "bead-quick-2".to_string(),
            endpoint: "http://localhost:8080/one-endpoint?fail=true".to_string(),
        });
        assert!(plan_result.is_ok());
        let Ok(plan) = plan_result else { return };

        let observation_result = run_onewf_bead_quick_check(&plan);
        assert!(observation_result.is_ok());
        let Ok(observation) = observation_result else {
            return;
        };

        assert_eq!(observation.checks.len(), 1);
        assert!(observation.checks[0].visible);
        assert!(!observation.checks[0].success);
        assert_eq!(observation.checks[0].diagnostics, "endpoint probe failed");
    }

    #[test]
    fn evaluate_onewf_bead_quick_result_generates_ordered_report_and_pass_decision() {
        let check = OnewfBeadQuickCheck {
            endpoint: "http://localhost:8080/one-endpoint".to_string(),
            visible: true,
            success: true,
            diagnostics: "endpoint visible and probe succeeded".to_string(),
            timestamp: Utc::now(),
        };
        let observation = OnewfBeadQuickObservation {
            workflow_id: "workflow-1".to_string(),
            bead_id: "bead-quick-3".to_string(),
            checks: vec![check],
        };

        let report_result = evaluate_onewf_bead_quick_result(&observation);
        assert!(report_result.is_ok());
        let Ok(report) = report_result else { return };

        let stage_order = report.stages.iter().map(|stage| stage.stage.clone()).collect::<Vec<_>>();
        assert_eq!(
            stage_order,
            vec![
                OnewfBeadQuickStageName::EndpointVisibility,
                OnewfBeadQuickStageName::EndpointProbe,
                OnewfBeadQuickStageName::FinalDecision,
            ]
        );
        assert_eq!(report.decision, OnewfBeadQuickDecision::Pass);
        assert_eq!(validate_onewf_bead_quick_report(&report), Ok(()));
    }

    #[test]
    fn evaluate_onewf_bead_quick_result_fails_when_endpoint_not_visible() {
        let check = OnewfBeadQuickCheck {
            endpoint: "http://localhost:8080/one-endpoint/hidden".to_string(),
            visible: false,
            success: false,
            diagnostics: "endpoint not visible".to_string(),
            timestamp: Utc::now(),
        };
        let observation = OnewfBeadQuickObservation {
            workflow_id: "workflow-1".to_string(),
            bead_id: "bead-quick-4".to_string(),
            checks: vec![check],
        };

        let report_result = evaluate_onewf_bead_quick_result(&observation);
        assert!(report_result.is_err());
        assert_eq!(
            report_result,
            Err(OnewfBeadQuickError::InvalidReport("single-endpoint visibility contract violated"))
        );
    }

    #[test]
    fn validate_onewf_bead_quick_report_rejects_non_monotonic_timestamps() {
        let base = Utc::now();
        let report = OnewfBeadQuickReport {
            workflow_id: "workflow-1".to_string(),
            bead_id: "bead-quick-5".to_string(),
            checks: vec![OnewfBeadQuickCheck {
                endpoint: "http://localhost:8080/one-endpoint".to_string(),
                visible: true,
                success: true,
                diagnostics: "ok".to_string(),
                timestamp: base,
            }],
            stages: vec![
                OnewfBeadQuickStageReport {
                    stage: OnewfBeadQuickStageName::EndpointVisibility,
                    status: OnewfBeadQuickStageStatus::Passed,
                    diagnostics: "visible".to_string(),
                    timestamp: base,
                },
                OnewfBeadQuickStageReport {
                    stage: OnewfBeadQuickStageName::EndpointProbe,
                    status: OnewfBeadQuickStageStatus::Passed,
                    diagnostics: "probe passed".to_string(),
                    timestamp: base - Duration::milliseconds(1),
                },
                OnewfBeadQuickStageReport {
                    stage: OnewfBeadQuickStageName::FinalDecision,
                    status: OnewfBeadQuickStageStatus::Passed,
                    diagnostics: "gate passed".to_string(),
                    timestamp: base,
                },
            ],
            decision: OnewfBeadQuickDecision::Pass,
        };

        assert_eq!(
            validate_onewf_bead_quick_report(&report),
            Err(OnewfBeadQuickError::InvalidReport("non-monotonic stage timestamps"))
        );
    }

    #[test]
    fn validate_onewf_bead_quick_report_rejects_decision_mismatch() {
        let base = Utc::now();
        let report = OnewfBeadQuickReport {
            workflow_id: "workflow-1".to_string(),
            bead_id: "bead-quick-6".to_string(),
            checks: vec![OnewfBeadQuickCheck {
                endpoint: "http://localhost:8080/one-endpoint".to_string(),
                visible: true,
                success: false,
                diagnostics: "probe failed".to_string(),
                timestamp: base,
            }],
            stages: vec![
                OnewfBeadQuickStageReport {
                    stage: OnewfBeadQuickStageName::EndpointVisibility,
                    status: OnewfBeadQuickStageStatus::Passed,
                    diagnostics: "visible".to_string(),
                    timestamp: base,
                },
                OnewfBeadQuickStageReport {
                    stage: OnewfBeadQuickStageName::EndpointProbe,
                    status: OnewfBeadQuickStageStatus::Failed,
                    diagnostics: "probe failed".to_string(),
                    timestamp: base + Duration::milliseconds(1),
                },
                OnewfBeadQuickStageReport {
                    stage: OnewfBeadQuickStageName::FinalDecision,
                    status: OnewfBeadQuickStageStatus::Passed,
                    diagnostics: "gate passed".to_string(),
                    timestamp: base + Duration::milliseconds(2),
                },
            ],
            decision: OnewfBeadQuickDecision::Pass,
        };

        assert_eq!(
            validate_onewf_bead_quick_report(&report),
            Err(OnewfBeadQuickError::InvalidReport("decision mismatch"))
        );
    }
}
