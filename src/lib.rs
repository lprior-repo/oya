#![deny(clippy::unwrap_used)]
#![deny(clippy::expect_used)]
#![deny(clippy::panic)]
#![forbid(unsafe_code)]

//! Design contract for `manual-e2e-bead`.
//!
//! # Purpose and goals
//! Define a manual end-to-end pipeline test contract that verifies setup, invocation, parsing,
//! and gate evaluation as one deterministic operator-visible flow.
//! Ensure each phase is observable, validated, and safe for release readiness checks.
//!
//! # Key functions to implement
//! - `build_manual_e2e_plan(input: &ManualE2eInput) -> Result<ManualE2ePlan, ManualE2eError>`
//! - `parse_pipeline_output(raw: &str) -> Result<ManualE2eOutput, ManualE2eError>`
//! - `run_manual_e2e_pipeline(plan: &ManualE2ePlan) -> Result<ManualE2eReport, ManualE2eError>`
//! - `validate_manual_e2e_report(report: &ManualE2eReport) -> Result<(), ManualE2eError>`
//! - `derive_manual_e2e_gate(report: &ManualE2eReport) -> ManualE2eGateDecision`
//!
//! # Acceptance criteria
//! - A valid manual e2e run records exactly four ordered stages: setup, invocation, parsing,
//!   gate evaluation.
//! - Output parsing rejects empty input, malformed JSON, missing required fields, and invalid
//!   field types via `ManualE2eError`.
//! - The generated report includes per-stage status, diagnostics, timestamps, and a final
//!   gate decision.
//! - Any stage with `Failed` or `Error` status produces `ManualE2eGateDecision::Block`.
//! - All fallible public APIs return `Result<_, ManualE2eError>` and do not panic.
//! - Re-running the same valid plan yields equivalent validation and gate outcomes.

pub mod application;
pub mod domain;
pub mod infrastructure;

use chrono::{DateTime, Utc};
use thiserror::Error;

const MAX_MANUAL_E2E_SCENARIO_LEN: usize = 128;
const MAX_MANUAL_E2E_COMMAND_LEN: usize = 1024;
const MAX_MANUAL_E2E_RAW_OUTPUT_LEN: usize = 128 * 1024;
const MAX_MANUAL_E2E_DIAGNOSTICS_LEN: usize = 8192;

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

    let value: serde_json::Value = serde_json::from_str(trimmed)
        .map_err(|e| OpencodeParseError::new(format!("invalid opencode json: {}", e)))?;

    match value.get("stdout") {
        Some(serde_json::Value::String(stdout)) => {
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

#[cfg(test)]
mod tests {
    use super::*;

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
}
