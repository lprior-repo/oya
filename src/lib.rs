#![deny(clippy::unwrap_used)]
#![deny(clippy::expect_used)]
#![deny(clippy::panic)]
#![deny(clippy::too_many_lines)]
#![deny(clippy::too_many_arguments)]
#![forbid(unsafe_code)]

//! Oya - Workflow orchestration and testing framework
//!
//! This crate provides the Oya orchestrator for managing development workflows
//! with Restate durable execution, quality gates, and bead tracking.
//!
//! # Design Contract: `test-trace-final`
//!
//! ## Purpose and goals
//! - Define a deterministic final-stage trace contract for planning, trace collection,
//!   report evaluation, and final gate validation.
//! - Guarantee reproducible outcomes for identical inputs by enforcing strict validation,
//!   stable stage ordering, and explicit decision derivation.
//! - Preserve auditability through structured diagnostics and monotonic event timestamps.
//!
//! ## Key functions to implement
//! - `build_test_trace_final_plan(input: &TestTraceFinalInput) -> Result<TestTraceFinalPlan, TestTraceFinalError>`
//! - `collect_test_trace_final_observation(plan: &TestTraceFinalPlan) -> Result<TestTraceFinalObservation, TestTraceFinalError>`
//! - `evaluate_test_trace_final_report(observation: &TestTraceFinalObservation) -> Result<TestTraceFinalReport, TestTraceFinalError>`
//! - `derive_test_trace_final_decision(report: &TestTraceFinalReport) -> TestTraceFinalDecision`
//! - `validate_test_trace_final_report(report: &TestTraceFinalReport) -> Result<(), TestTraceFinalError>`
//!
//! ## Acceptance criteria
//! - Plan creation rejects empty fields, over-limit inputs, and invalid control characters.
//! - Observation collection emits ordered checks with non-empty diagnostics and valid timestamps.
//! - Report evaluation preserves contract stage order and enforces monotonic timestamps.
//! - Final decision is derived only from trace/check outcomes and matches validation results.
//! - Re-running with equivalent inputs yields equivalent report structure and decisions.
//!
//! # Design Contract: `src-2nw`
//!
//! ## Purpose and goals
//! - Fix critical determinism bug in Restate workflow execution by ensuring `spawn_blocking`
//!   operations are properly journaled and not re-executed on workflow replay.
//! - Maintain Restate's determinism guarantee by separating non-deterministic operations
//!   from deterministic journaling in the workflow execution context.
//! - Ensure workflow state consistency across executions and replays by following the
//!   correct execution pattern for blocking operations.
//!
//! ## Key functions to implement
//! - `execute_stage_real(ctx: &WorkflowContext<'_>, request: StageExecutionRequest, merge_queue_policy: MergeQueuePolicy, repo_root: PathBuf) -> Result<(StageResult, String), OyaError>`
//!   - Fixed implementation with `spawn_blocking` outside `ctx.run()`
//! - `execute_stage_blocking(input: StageBlockingInput) -> Result<StageExecution, OyaError>`
//!   - Existing synchronous blocking execution (no changes needed)
//! - `test_execute_stage_real_deterministic_replay()`
//!   - New test to verify spawn_blocking is not called on replay
//!
//! ## Acceptance criteria
//! - `spawn_blocking` is called OUTSIDE of `ctx.run()` (non-deterministic part)
//! - Only result mapping is inside `ctx.run()` (deterministic journaling)
//! - Error handling properly separates OyaError (outer) from HandlerError (inner)
//! - Test added that verifies spawn_blocking is not called on replay
//! - Documentation explains the determinism pattern with proper doc comments
//! - `moon run :clippy` passes with no unwrap/expect/panic violations
//! - `moon run :test` passes with all tests green
//! - Code review confirms no other functions have this anti-pattern

pub mod beads;
pub mod config;
pub mod orchestrator;
pub mod telemetry;
pub mod types;
pub mod usage;

use chrono::{DateTime, Utc};
use std::path::PathBuf;
use thiserror::Error;
use types::FailureCategory;

/// Determine if a failure category is retryable
///
/// Retryable: code-level failures the AI can fix (test, lint, parse errors)
/// Non-retryable: provider/environment failures that require external intervention
pub fn is_retryable_failure(category: &FailureCategory) -> bool {
    matches!(
        category,
        FailureCategory::TestFailed
            | FailureCategory::LintFailed
            | FailureCategory::OutputParseFailure
            | FailureCategory::CompileFailed
    )
}

/// Classify opencode CLI error output
pub fn classify_opencode_error(stderr: &str) -> Option<FailureCategory> {
    if stderr.contains("429")
        || stderr.contains("Rate Limited")
        || stderr.contains("Too Many Requests")
        || stderr.contains("Quota exceeded")
        || stderr.contains("Provider is overloaded")
        || stderr.contains("Provider unavailable")
    {
        return Some(FailureCategory::RateLimited);
    }

    if stderr.contains("OpenCode provider unavailable")
        || stderr.contains("OpenCode HTTP request failed")
        || stderr.contains("Connection refused")
        || stderr.contains("error sending request for url")
    {
        return Some(FailureCategory::ProviderUnavailable);
    }

    None
}

const MAX_MANUAL_E2E_SCENARIO_LEN: usize = 128;
const MAX_MANUAL_E2E_COMMAND_LEN: usize = 1024;
const MAX_MANUAL_E2E_RAW_OUTPUT_LEN: usize = 128 * 1024;
const MAX_MANUAL_E2E_DIAGNOSTICS_LEN: usize = 8192;
const MAX_SMOKE_RUN_ID_LEN: usize = 128;
const MAX_OPENCODE_OUTPUT_JSON_LEN: usize = 256 * 1024;
const MAX_OPENCODE_STDOUT_LEN: usize = 128 * 1024;
const MAX_MOON_TASK_NAME_LEN: usize = 128;
const MAX_ZJJ_WORKSPACE_NAME_LEN: usize = 64;
const MAX_OPENCODE_SSE_RAW_CHUNK_LEN: usize = 256 * 1024;
const MAX_OPENCODE_SSE_EVENT_PAYLOAD_LEN: usize = 16 * 1024;

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

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OpencodePollSnapshot {
    pub busy_sessions: Vec<String>,
    pub pending_permissions: usize,
    pub pending_questions: usize,
}

#[derive(Debug, Error, Clone, PartialEq, Eq)]
pub enum OpsMonitorError {
    #[error("ops monitor field is empty: {0}")]
    EmptyField(&'static str),
    #[error("ops monitor field exceeds max length: {0} > {1}")]
    FieldTooLong(&'static str, usize),
    #[error("ops monitor field has invalid control characters: {0}")]
    InvalidFieldContent(&'static str),
    #[error("ops monitor field has invalid format: {0}")]
    InvalidFieldFormat(&'static str),
    #[error("ops monitor json parse failed: {0}")]
    InvalidJson(String),
}

pub fn build_zjj_workspace_name(
    run_id: &str,
    stage: &str,
    attempt: u32,
) -> Result<String, OpsMonitorError> {
    let normalized_run_id = normalize_workspace_segment(run_id, "run_id")?;
    let normalized_stage = normalize_workspace_segment(stage, "stage")?;
    if attempt == 0 {
        return Err(OpsMonitorError::InvalidFieldFormat("attempt"));
    }

    let workspace = format!("oya-{}-{}-a{}", normalized_run_id, normalized_stage, attempt);
    if workspace.len() > MAX_ZJJ_WORKSPACE_NAME_LEN {
        return Err(OpsMonitorError::FieldTooLong("workspace", MAX_ZJJ_WORKSPACE_NAME_LEN));
    }

    Ok(workspace)
}

pub fn parse_opencode_busy_sessions(raw: &str) -> Result<Vec<String>, OpsMonitorError> {
    if raw.trim().is_empty() {
        return Ok(Vec::new());
    }

    let value: serde_json::Value = serde_json::from_str(raw)
        .map_err(|error| OpsMonitorError::InvalidJson(error.to_string()))?;
    let object = value.as_object().ok_or(OpsMonitorError::InvalidFieldFormat("session_status"))?;

    Ok(object
        .iter()
        .filter(|(_, value)| value.get("type").and_then(serde_json::Value::as_str) == Some("busy"))
        .map(|(session_id, _)| session_id.to_string())
        .collect::<std::collections::BTreeSet<_>>()
        .into_iter()
        .collect())
}

pub fn parse_opencode_pending_count(
    raw: &str,
    field: &'static str,
) -> Result<usize, OpsMonitorError> {
    if raw.trim().is_empty() {
        return Ok(0);
    }

    let value: serde_json::Value = serde_json::from_str(raw)
        .map_err(|error| OpsMonitorError::InvalidJson(error.to_string()))?;

    match value {
        serde_json::Value::Null => Ok(0),
        serde_json::Value::Array(items) => Ok(items.len()),
        serde_json::Value::Object(object) => object
            .get("items")
            .and_then(serde_json::Value::as_array)
            .map(std::vec::Vec::len)
            .or_else(|| {
                object.get("requests").and_then(serde_json::Value::as_array).map(std::vec::Vec::len)
            })
            .or_else(|| {
                object.get("rows").and_then(serde_json::Value::as_array).map(std::vec::Vec::len)
            })
            .map_or_else(|| Ok(object.len()), Ok),
        _ => Err(OpsMonitorError::InvalidFieldFormat(field)),
    }
}

pub fn parse_opencode_sse_events(
    raw_chunk: &str,
    max_events: usize,
) -> Result<Vec<String>, OpsMonitorError> {
    if raw_chunk.trim().is_empty() {
        return Ok(Vec::new());
    }
    if raw_chunk.len() > MAX_OPENCODE_SSE_RAW_CHUNK_LEN {
        return Err(OpsMonitorError::FieldTooLong("event_chunk", MAX_OPENCODE_SSE_RAW_CHUNK_LEN));
    }

    // Normalize line endings first for consistent validation
    let normalized = raw_chunk.replace("\r\n", "\n").replace('\r', "\n");

    if contains_forbidden_control_chars(&normalized) {
        return Err(OpsMonitorError::InvalidFieldContent("event_chunk"));
    }

    normalized
        .split("\n\n")
        .map(parse_sse_payload_block)
        .filter(|payload| !payload.trim().is_empty())
        .take(max_events)
        .map(|payload| {
            if payload.len() > MAX_OPENCODE_SSE_EVENT_PAYLOAD_LEN {
                Err(OpsMonitorError::FieldTooLong(
                    "event_payload",
                    MAX_OPENCODE_SSE_EVENT_PAYLOAD_LEN,
                ))
            } else if contains_forbidden_control_chars(payload.as_str()) {
                Err(OpsMonitorError::InvalidFieldContent("event_payload"))
            } else {
                Ok(payload)
            }
        })
        .collect()
}

pub fn build_opencode_poll_snapshot(
    session_status_json: &str,
    permission_json: &str,
    question_json: &str,
) -> Result<OpencodePollSnapshot, OpsMonitorError> {
    Ok(OpencodePollSnapshot {
        busy_sessions: parse_opencode_busy_sessions(session_status_json)?,
        pending_permissions: parse_opencode_pending_count(permission_json, "permission")?,
        pending_questions: parse_opencode_pending_count(question_json, "question")?,
    })
}

fn normalize_workspace_segment(
    value: &str,
    field: &'static str,
) -> Result<String, OpsMonitorError> {
    let trimmed = value.trim();
    if trimmed.is_empty() {
        return Err(OpsMonitorError::EmptyField(field));
    }
    if contains_forbidden_control_chars(trimmed) {
        return Err(OpsMonitorError::InvalidFieldContent(field));
    }

    let normalized =
        trimmed
            .to_ascii_lowercase()
            .chars()
            .map(|char| {
                if char.is_ascii_alphanumeric() || char == '-' || char == '_' {
                    char
                } else {
                    '-'
                }
            })
            .collect::<String>()
            .split('-')
            .filter(|segment| !segment.is_empty())
            .collect::<Vec<_>>()
            .join("-");

    if normalized.is_empty() {
        return Err(OpsMonitorError::InvalidFieldFormat(field));
    }

    Ok(normalized)
}

fn parse_sse_payload_block(block: &str) -> String {
    block
        .lines()
        .filter_map(|line| {
            line.strip_prefix("data:").map(str::trim_start).filter(|payload| !payload.is_empty())
        })
        .collect::<Vec<_>>()
        .join("\n")
}

pub fn parse_opencode_output(raw: &str) -> Result<OpencodeRunOutput, OpencodeParseError> {
    let trimmed = raw.trim();
    if trimmed.is_empty() {
        return Err(OpencodeParseError::new("opencode output empty"));
    }
    if trimmed.len() > MAX_OPENCODE_OUTPUT_JSON_LEN {
        return Err(OpencodeParseError::new("opencode output exceeds maximum length"));
    }

    if let Some(output) = parse_opencode_json_payload(trimmed)? {
        return Ok(output);
    }

    let extracted = parse_opencode_output_text_events(trimmed)?;
    parse_opencode_output_text(&serde_json::Value::String(extracted))
}

fn parse_opencode_json_payload(
    trimmed: &str,
) -> Result<Option<OpencodeRunOutput>, OpencodeParseError> {
    let Some(value) = serde_json::from_str::<serde_json::Value>(trimmed).ok() else {
        return Ok(None);
    };
    match value.get("stdout") {
        Some(stdout) => parse_opencode_output_stdout(stdout).map(Some),
        None => Err(OpencodeParseError::new("opencode json missing stdout field")),
    }
}

fn parse_opencode_text_events(raw: &str) -> String {
    raw.lines().filter_map(parse_opencode_text_event_piece).collect::<Vec<String>>().join("")
}

fn parse_opencode_text_event_piece(raw_line: &str) -> Option<String> {
    let line = raw_line.trim();
    let mut payload = line;
    if let Some(rest) = line.strip_prefix("data:") {
        payload = rest.trim_start();
    }

    serde_json::from_str::<serde_json::Value>(payload).ok().and_then(|value| {
        let type_is_text = value.get("type").and_then(serde_json::Value::as_str) == Some("text");
        if let Some(stdout) = value.get("stdout").and_then(serde_json::Value::as_str) {
            Some(stdout.to_string())
        } else if type_is_text {
            value
                .get("part")
                .and_then(|part| part.get("text"))
                .and_then(serde_json::Value::as_str)
                .map(str::to_string)
        } else {
            None
        }
    })
}

fn parse_opencode_output_text_events(raw: &str) -> Result<String, OpencodeParseError> {
    let extracted = parse_opencode_text_events(raw);
    if extracted.is_empty() {
        return Err(OpencodeParseError::new("opencode json missing stdout field"));
    }
    Ok(extracted)
}

fn parse_opencode_output_stdout(
    value: &serde_json::Value,
) -> Result<OpencodeRunOutput, OpencodeParseError> {
    parse_opencode_output_text(value)
}

fn parse_opencode_output_text(
    value: &serde_json::Value,
) -> Result<OpencodeRunOutput, OpencodeParseError> {
    let Some(stdout) = value.as_str() else {
        return Err(OpencodeParseError::new("opencode json stdout is not a string"));
    };

    if stdout.len() > MAX_OPENCODE_STDOUT_LEN {
        return Err(OpencodeParseError::new("opencode stdout exceeds maximum length"));
    }
    if contains_forbidden_control_chars(stdout) {
        return Err(OpencodeParseError::new("opencode stdout contains invalid control characters"));
    }
    Ok(OpencodeRunOutput { stdout: stdout.to_string() })
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
    validate_manual_e2e_stage_contract(report)?;
    validate_manual_e2e_stage_diagnostics(report)?;
    validate_manual_e2e_timestamps_and_decision(report)
}

fn validate_manual_e2e_stage_contract(report: &ManualE2eReport) -> Result<(), ManualE2eError> {
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

    Ok(())
}

fn validate_manual_e2e_stage_diagnostics(report: &ManualE2eReport) -> Result<(), ManualE2eError> {
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

    Ok(())
}

fn validate_manual_e2e_timestamps_and_decision(
    report: &ManualE2eReport,
) -> Result<(), ManualE2eError> {
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

include!("lib_contracts_src_kes.rs");

const MAX_ONEWF_WORKFLOW_ID_LEN: usize = 128;
const MAX_ONEWF_BEAD_ID_LEN: usize = 128;
const MAX_ONEWF_ENDPOINT_LEN: usize = 2048;
const MAX_ONEWF_DIAGNOSTICS_LEN: usize = 4096;

include!("lib_contracts_mid.rs");
include!("lib_contracts_tail.rs");

#[cfg(test)]
mod lib_tests;
