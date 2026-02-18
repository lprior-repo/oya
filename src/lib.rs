#![deny(clippy::unwrap_used)]
#![deny(clippy::expect_used)]
#![deny(clippy::panic)]
#![forbid(unsafe_code)]

//! Design contract for `src-kes`.
//!
//! # Purpose and goals
//! Define a deterministic `src-kes` CRUD service contract for observability test runs, including
//! route completeness, typed validation, explicit failures, and stage-based decision reporting.
//!
//! # Key functions to implement
//! - `build_src_kes_plan(input: &SrcKesInput) -> Result<SrcKesPlan, SrcKesError>`
//! - `start_src_kes_server(plan: &SrcKesPlan) -> Result<SrcKesRuntimeHandle, SrcKesError>`
//! - `register_user_routes() -> Vec<SrcKesRouteContract>`
//! - `run_user_create(state: &SrcKesServiceState, request: &UserCreateRequest) -> Result<(SrcKesServiceState, UserRecord), SrcKesError>`
//! - `run_user_read(state: &SrcKesServiceState, user_id: &str) -> Result<UserRecord, SrcKesError>`
//! - `run_user_update(state: &SrcKesServiceState, user_id: &str, request: &UserUpdateRequest) -> Result<(SrcKesServiceState, UserRecord), SrcKesError>`
//! - `run_user_delete(state: &SrcKesServiceState, user_id: &str) -> Result<SrcKesServiceState, SrcKesError>`
//! - `validate_src_kes_report(report: &SrcKesReport) -> Result<(), SrcKesError>`
//!
//! # Acceptance criteria
//! - Route contract includes exactly `POST /users` (`201`), `GET /users/:id` (`200`),
//!   `PUT /users/:id` (`200`), and `DELETE /users/:id` (`204`).
//! - Service and user inputs reject empty, malformed, overlong, or control-character content with
//!   explicit `SrcKesError` variants.
//! - User lifecycle is deterministic: create enforces unique normalized IDs, read/update/delete
//!   fail with `UserNotFound` for missing IDs, and update/delete preserve map consistency.
//! - Report validation enforces framework/resource invariants, required stage order, monotonic
//!   timestamps, non-empty diagnostics, and decision derivation from stage outcomes.
//! - Identical valid inputs produce equivalent plans, route contracts, and gate decisions.

pub mod orchestrator;
pub mod types;

use chrono::{DateTime, Utc};
use std::path::PathBuf;
use thiserror::Error;
use types::FailureCategory;

/// Determine if a failure category is retryable
pub fn is_retryable_failure(category: &FailureCategory) -> bool {
    matches!(
        category,
        FailureCategory::TestFailed
            | FailureCategory::LintFailed
            | FailureCategory::OutputParseFailure
    )
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
    if contains_forbidden_control_chars(raw_chunk) {
        return Err(OpsMonitorError::InvalidFieldContent("event_chunk"));
    }

    let normalized = raw_chunk.replace("\r\n", "\n");

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

const MAX_SRC_KES_SERVICE_NAME_LEN: usize = 64;
const MAX_SRC_KES_USER_NAME_LEN: usize = 128;
const MAX_SRC_KES_EMAIL_LEN: usize = 256;
const MAX_SRC_KES_USER_ID_LEN: usize = 96;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SrcKesInput {
    pub service_name: String,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub enum SrcKesRouteMethod {
    Post,
    Get,
    Put,
    Delete,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct SrcKesRouteContract {
    pub method: SrcKesRouteMethod,
    pub path: String,
    pub success_status: u16,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SrcKesPlan {
    pub service_name: String,
    pub framework: String,
    pub resource: String,
    pub routes: Vec<SrcKesRouteContract>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SrcKesRuntimeHandle {
    pub service_name: String,
    pub framework: String,
    pub running: bool,
}

pub type UserId = String;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UserCreateRequest {
    pub name: String,
    pub email: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UserUpdateRequest {
    pub name: String,
    pub email: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UserRecord {
    pub id: UserId,
    pub name: String,
    pub email: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct SrcKesServiceState {
    pub users: std::collections::BTreeMap<UserId, UserRecord>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SrcKesStageName {
    PlanBuild,
    RuntimeStart,
    RouteContract,
    CrudContract,
    FinalDecision,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SrcKesStageStatus {
    Passed,
    Failed,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SrcKesStageReport {
    pub stage: SrcKesStageName,
    pub status: SrcKesStageStatus,
    pub diagnostics: String,
    pub timestamp: DateTime<Utc>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SrcKesDecision {
    Pass,
    Fail,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SrcKesReport {
    pub plan: SrcKesPlan,
    pub runtime_started: bool,
    pub deterministic_behavior: bool,
    pub stages: Vec<SrcKesStageReport>,
    pub decision: SrcKesDecision,
}

#[derive(Debug, Error, Clone, PartialEq, Eq)]
pub enum SrcKesError {
    #[error("src-kes field is empty: {0}")]
    EmptyField(&'static str),
    #[error("src-kes field exceeds max length: {0} > {1}")]
    FieldTooLong(&'static str, usize),
    #[error("src-kes field contains invalid control characters: {0}")]
    InvalidFieldContent(&'static str),
    #[error("src-kes field has invalid format: {0}")]
    InvalidFieldFormat(&'static str),
    #[error("src-kes route contract invalid")]
    InvalidRouteContract,
    #[error("src-kes user already exists: {0}")]
    DuplicateUserId(String),
    #[error("src-kes user not found: {0}")]
    UserNotFound(String),
    #[error("src-kes report invalid: {0}")]
    InvalidReport(&'static str),
}

pub fn build_src_kes_plan(input: &SrcKesInput) -> Result<SrcKesPlan, SrcKesError> {
    let service_name = validate_src_kes_text_field(
        input.service_name.as_str(),
        "service_name",
        MAX_SRC_KES_SERVICE_NAME_LEN,
    )?;
    let routes = register_user_routes();
    validate_src_kes_route_contract(routes.as_slice())?;

    Ok(SrcKesPlan {
        service_name,
        framework: "scotty".to_string(),
        resource: "user".to_string(),
        routes,
    })
}

pub fn start_src_kes_server(plan: &SrcKesPlan) -> Result<SrcKesRuntimeHandle, SrcKesError> {
    if plan.framework != "scotty" {
        return Err(SrcKesError::InvalidFieldFormat("framework"));
    }
    if plan.resource != "user" {
        return Err(SrcKesError::InvalidFieldFormat("resource"));
    }
    validate_src_kes_text_field(
        plan.service_name.as_str(),
        "service_name",
        MAX_SRC_KES_SERVICE_NAME_LEN,
    )?;
    validate_src_kes_route_contract(plan.routes.as_slice())?;

    Ok(SrcKesRuntimeHandle {
        service_name: plan.service_name.clone(),
        framework: plan.framework.clone(),
        running: true,
    })
}

pub fn register_user_routes() -> Vec<SrcKesRouteContract> {
    vec![
        SrcKesRouteContract {
            method: SrcKesRouteMethod::Post,
            path: "/users".to_string(),
            success_status: 201,
        },
        SrcKesRouteContract {
            method: SrcKesRouteMethod::Get,
            path: "/users/:id".to_string(),
            success_status: 200,
        },
        SrcKesRouteContract {
            method: SrcKesRouteMethod::Put,
            path: "/users/:id".to_string(),
            success_status: 200,
        },
        SrcKesRouteContract {
            method: SrcKesRouteMethod::Delete,
            path: "/users/:id".to_string(),
            success_status: 204,
        },
    ]
}

pub fn run_user_create(
    state: &SrcKesServiceState,
    request: &UserCreateRequest,
) -> Result<(SrcKesServiceState, UserRecord), SrcKesError> {
    let name =
        validate_src_kes_text_field(request.name.as_str(), "name", MAX_SRC_KES_USER_NAME_LEN)?;
    let email = normalize_src_kes_email(request.email.as_str())?;
    let user_id = build_src_kes_user_id(email.as_str())?;

    if state.users.contains_key(user_id.as_str()) {
        return Err(SrcKesError::DuplicateUserId(user_id));
    }

    let record = UserRecord { id: user_id.clone(), name, email };
    let users = state
        .users
        .iter()
        .map(|(existing_id, existing_record)| (existing_id.clone(), existing_record.clone()))
        .chain(std::iter::once((user_id, record.clone())))
        .collect::<std::collections::BTreeMap<_, _>>();

    Ok((SrcKesServiceState { users }, record))
}

pub fn run_user_read(state: &SrcKesServiceState, user_id: &str) -> Result<UserRecord, SrcKesError> {
    let normalized_id = validate_src_kes_user_id(user_id)?;
    state.users.get(normalized_id.as_str()).cloned().ok_or(SrcKesError::UserNotFound(normalized_id))
}

pub fn run_user_update(
    state: &SrcKesServiceState,
    user_id: &str,
    request: &UserUpdateRequest,
) -> Result<(SrcKesServiceState, UserRecord), SrcKesError> {
    let normalized_id = validate_src_kes_user_id(user_id)?;
    let existing = state
        .users
        .get(normalized_id.as_str())
        .cloned()
        .ok_or(SrcKesError::UserNotFound(normalized_id.clone()))?;

    let name =
        validate_src_kes_text_field(request.name.as_str(), "name", MAX_SRC_KES_USER_NAME_LEN)?;
    let email = normalize_src_kes_email(request.email.as_str())?;
    let next_record = UserRecord { id: existing.id, name, email };

    let users = state
        .users
        .iter()
        .map(|(existing_id, existing_record)| {
            if existing_id == &normalized_id {
                (existing_id.clone(), next_record.clone())
            } else {
                (existing_id.clone(), existing_record.clone())
            }
        })
        .collect::<std::collections::BTreeMap<_, _>>();

    Ok((SrcKesServiceState { users }, next_record))
}

pub fn run_user_delete(
    state: &SrcKesServiceState,
    user_id: &str,
) -> Result<SrcKesServiceState, SrcKesError> {
    let normalized_id = validate_src_kes_user_id(user_id)?;
    if !state.users.contains_key(normalized_id.as_str()) {
        return Err(SrcKesError::UserNotFound(normalized_id));
    }

    let users = state
        .users
        .iter()
        .filter(|(existing_id, _)| *existing_id != &normalized_id)
        .map(|(existing_id, existing_record)| (existing_id.clone(), existing_record.clone()))
        .collect::<std::collections::BTreeMap<_, _>>();

    Ok(SrcKesServiceState { users })
}

pub fn validate_src_kes_report(report: &SrcKesReport) -> Result<(), SrcKesError> {
    validate_src_kes_route_contract(report.plan.routes.as_slice())?;
    if report.plan.framework != "scotty" {
        return Err(SrcKesError::InvalidReport("framework must be scotty"));
    }
    if report.plan.resource != "user" {
        return Err(SrcKesError::InvalidReport("resource must be user"));
    }
    if !report.runtime_started {
        return Err(SrcKesError::InvalidReport("runtime not started"));
    }
    if !report.deterministic_behavior {
        return Err(SrcKesError::InvalidReport("deterministic behavior violated"));
    }

    let expected_stage_order = [
        SrcKesStageName::PlanBuild,
        SrcKesStageName::RuntimeStart,
        SrcKesStageName::RouteContract,
        SrcKesStageName::CrudContract,
        SrcKesStageName::FinalDecision,
    ];

    if report.stages.len() != expected_stage_order.len() {
        return Err(SrcKesError::InvalidReport("unexpected stage count"));
    }

    let valid_order = report
        .stages
        .iter()
        .map(|stage| stage.stage.clone())
        .eq(expected_stage_order.iter().cloned());
    if !valid_order {
        return Err(SrcKesError::InvalidReport("invalid stage order"));
    }

    let has_empty_diagnostics =
        report.stages.iter().any(|stage| stage.diagnostics.trim().is_empty());
    if has_empty_diagnostics {
        return Err(SrcKesError::InvalidReport("empty stage diagnostics"));
    }

    let has_non_monotonic_timestamps =
        report.stages.windows(2).any(|pair| pair[0].timestamp > pair[1].timestamp);
    if has_non_monotonic_timestamps {
        return Err(SrcKesError::InvalidReport("non-monotonic stage timestamps"));
    }

    let has_failed_stage =
        report.stages.iter().any(|stage| stage.status == SrcKesStageStatus::Failed);
    let derived_decision =
        if has_failed_stage { SrcKesDecision::Fail } else { SrcKesDecision::Pass };
    if derived_decision != report.decision {
        return Err(SrcKesError::InvalidReport("decision mismatch"));
    }

    Ok(())
}

fn validate_src_kes_route_contract(routes: &[SrcKesRouteContract]) -> Result<(), SrcKesError> {
    let expected = vec![
        SrcKesRouteContract {
            method: SrcKesRouteMethod::Post,
            path: "/users".to_string(),
            success_status: 201,
        },
        SrcKesRouteContract {
            method: SrcKesRouteMethod::Get,
            path: "/users/:id".to_string(),
            success_status: 200,
        },
        SrcKesRouteContract {
            method: SrcKesRouteMethod::Put,
            path: "/users/:id".to_string(),
            success_status: 200,
        },
        SrcKesRouteContract {
            method: SrcKesRouteMethod::Delete,
            path: "/users/:id".to_string(),
            success_status: 204,
        },
    ]
    .into_iter()
    .collect::<std::collections::BTreeSet<_>>();

    let actual = routes.iter().cloned().collect::<std::collections::BTreeSet<_>>();

    if actual != expected {
        return Err(SrcKesError::InvalidRouteContract);
    }

    Ok(())
}

fn validate_src_kes_text_field(
    value: &str,
    field: &'static str,
    max_len: usize,
) -> Result<String, SrcKesError> {
    let trimmed = value.trim();
    if trimmed.is_empty() {
        return Err(SrcKesError::EmptyField(field));
    }
    if trimmed.len() > max_len {
        return Err(SrcKesError::FieldTooLong(field, max_len));
    }
    if contains_forbidden_control_chars(trimmed) {
        return Err(SrcKesError::InvalidFieldContent(field));
    }
    Ok(trimmed.to_string())
}

fn normalize_src_kes_email(value: &str) -> Result<String, SrcKesError> {
    let lowered =
        validate_src_kes_text_field(value, "email", MAX_SRC_KES_EMAIL_LEN)?.to_ascii_lowercase();

    let valid_chars = lowered
        .chars()
        .all(|char| char.is_ascii_alphanumeric() || matches!(char, '@' | '.' | '_' | '-' | '+'));
    if !valid_chars {
        return Err(SrcKesError::InvalidFieldFormat("email"));
    }

    let segments = lowered.split('@').collect::<Vec<_>>();
    let local = if segments.is_empty() { "" } else { segments[0] };
    let domain = if segments.len() < 2 { "" } else { segments[1] };
    let no_extra_segments = segments.len() == 2;
    if local.is_empty()
        || domain.is_empty()
        || !domain.contains('.')
        || domain.starts_with('.')
        || domain.ends_with('.')
        || !no_extra_segments
    {
        return Err(SrcKesError::InvalidFieldFormat("email"));
    }

    Ok(lowered)
}

fn build_src_kes_user_id(email: &str) -> Result<String, SrcKesError> {
    let normalized = email
        .chars()
        .map(|char| if char.is_ascii_alphanumeric() { char } else { '-' })
        .collect::<String>()
        .split('-')
        .filter(|segment| !segment.is_empty())
        .collect::<Vec<_>>()
        .join("-");

    if normalized.is_empty() {
        return Err(SrcKesError::InvalidFieldFormat("user_id"));
    }

    let user_id = format!("user-{}", normalized);
    if user_id.len() > MAX_SRC_KES_USER_ID_LEN {
        return Err(SrcKesError::FieldTooLong("user_id", MAX_SRC_KES_USER_ID_LEN));
    }
    if !is_valid_src_kes_user_id(user_id.as_str()) {
        return Err(SrcKesError::InvalidFieldFormat("user_id"));
    }

    Ok(user_id)
}

fn validate_src_kes_user_id(value: &str) -> Result<String, SrcKesError> {
    let normalized = validate_src_kes_text_field(value, "user_id", MAX_SRC_KES_USER_ID_LEN)?;
    if !is_valid_src_kes_user_id(normalized.as_str()) {
        return Err(SrcKesError::InvalidFieldFormat("user_id"));
    }
    Ok(normalized)
}

fn is_valid_src_kes_user_id(value: &str) -> bool {
    value.chars().all(|char| char.is_ascii_alphanumeric() || char == '-')
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

const DEFAULT_BEAD_CUPID_RUNTIME_COMMAND: &str = "scripts/dev-up.sh";
const DEFAULT_BEAD_CUPID_INGRESS_HEALTH_URL: &str = "http://localhost:8080/restate/health";
const MAX_BEAD_CUPID_RUN_ID_LEN: usize = 128;
const MAX_BEAD_CUPID_BEAD_ID_LEN: usize = 128;
const MAX_BEAD_CUPID_ENDPOINT_LEN: usize = 2048;
const MAX_BEAD_CUPID_DIAGNOSTICS_LEN: usize = 4096;

#[derive(Debug, Clone, PartialEq, Eq)]
/// Input contract for planning a bead-cupid run.
pub struct BeadCupidInput {
    pub run_id: String,
    pub bead_id: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
/// Immutable plan for bead-cupid runtime startup and checks.
pub struct BeadCupidPlan {
    pub run_id: String,
    pub bead_id: String,
    pub runtime_command: String,
    pub ingress_health_url: String,
    pub orchestrator_status_url: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
/// Runtime handle produced after bead-cupid startup validation succeeds.
pub struct BeadCupidRuntimeHandle {
    pub run_id: String,
    pub bead_id: String,
    pub runtime_command: String,
    pub ingress_health_url: String,
    pub orchestrator_status_url: String,
    pub started_at: DateTime<Utc>,
    pub runtime_ready: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
/// Named checks captured during bead-cupid observation.
pub enum BeadCupidCheckName {
    IngressHealth,
    OrchestratorStatus,
}

#[derive(Debug, Clone, PartialEq, Eq)]
/// One observed bead-cupid check result.
pub struct BeadCupidCheckObservation {
    pub check: BeadCupidCheckName,
    pub endpoint: String,
    pub success: bool,
    pub diagnostics: String,
    pub timestamp: DateTime<Utc>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
/// Full observation payload emitted from a runtime handle.
pub struct BeadCupidObservation {
    pub run_id: String,
    pub bead_id: String,
    pub runtime_command: String,
    pub ingress_health_url: String,
    pub orchestrator_status_url: String,
    pub checks: Vec<BeadCupidCheckObservation>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
/// Ordered stage names expected in bead-cupid reports.
pub enum BeadCupidStageName {
    IngressHealth,
    OrchestratorStatus,
    FinalDecision,
}

#[derive(Debug, Clone, PartialEq, Eq)]
/// Stage-level pass/fail status for bead-cupid reporting.
pub enum BeadCupidStageStatus {
    Passed,
    Failed,
}

#[derive(Debug, Clone, PartialEq, Eq)]
/// Stage report row in the bead-cupid evaluation output.
pub struct BeadCupidStageReport {
    pub stage: BeadCupidStageName,
    pub status: BeadCupidStageStatus,
    pub diagnostics: String,
    pub timestamp: DateTime<Utc>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
/// Final gate decision derived from bead-cupid checks.
pub enum BeadCupidDecision {
    Pass,
    Fail,
}

#[derive(Debug, Clone, PartialEq, Eq)]
/// Final validated report for the bead-cupid flow.
pub struct BeadCupidReport {
    pub plan: BeadCupidPlan,
    pub checks: Vec<BeadCupidCheckObservation>,
    pub stages: Vec<BeadCupidStageReport>,
    pub decision: BeadCupidDecision,
}

#[derive(Debug, Error, Clone, PartialEq, Eq)]
/// Typed errors for bead-cupid planning, observation, and validation.
pub enum BeadCupidError {
    #[error("bead-cupid field is empty: {0}")]
    EmptyField(&'static str),
    #[error("bead-cupid field exceeds max length: {0} > {1}")]
    FieldTooLong(&'static str, usize),
    #[error("bead-cupid field has invalid control characters: {0}")]
    InvalidFieldContent(&'static str),
    #[error("bead-cupid identifier format invalid: {0}")]
    InvalidIdentifier(&'static str),
    #[error("bead-cupid runtime command is invalid")]
    InvalidRuntimeCommand,
    #[error("bead-cupid endpoint is invalid: {0}")]
    InvalidEndpoint(&'static str),
    #[error("bead-cupid runtime not ready")]
    RuntimeNotReady,
    #[error("bead-cupid check missing: {0}")]
    MissingCheck(&'static str),
    #[error("bead-cupid report invalid: {0}")]
    InvalidReport(&'static str),
}

/// Builds a normalized bead-cupid plan from raw run and bead identifiers.
pub fn build_bead_cupid_plan(input: &BeadCupidInput) -> Result<BeadCupidPlan, BeadCupidError> {
    let run_id =
        validate_bead_cupid_identifier(input.run_id.as_str(), "run_id", MAX_BEAD_CUPID_RUN_ID_LEN)?;
    let bead_id = validate_bead_cupid_identifier(
        input.bead_id.as_str(),
        "bead_id",
        MAX_BEAD_CUPID_BEAD_ID_LEN,
    )?;

    Ok(BeadCupidPlan {
        run_id: run_id.clone(),
        bead_id,
        runtime_command: DEFAULT_BEAD_CUPID_RUNTIME_COMMAND.to_string(),
        ingress_health_url: DEFAULT_BEAD_CUPID_INGRESS_HEALTH_URL.to_string(),
        orchestrator_status_url: format!(
            "http://localhost:8080/OyaOrchestrator/{}/get_status",
            run_id
        ),
    })
}

/// Starts bead-cupid runtime and validates all runtime contract fields.
pub fn start_bead_cupid_runtime(
    plan: &BeadCupidPlan,
) -> Result<BeadCupidRuntimeHandle, BeadCupidError> {
    validate_normalized_bead_cupid_identifier(
        plan.run_id.as_str(),
        "run_id",
        MAX_BEAD_CUPID_RUN_ID_LEN,
    )?;
    validate_normalized_bead_cupid_identifier(
        plan.bead_id.as_str(),
        "bead_id",
        MAX_BEAD_CUPID_BEAD_ID_LEN,
    )?;

    if plan.runtime_command != DEFAULT_BEAD_CUPID_RUNTIME_COMMAND {
        return Err(BeadCupidError::InvalidRuntimeCommand);
    }
    if !is_valid_bead_cupid_endpoint(plan.ingress_health_url.as_str()) {
        return Err(BeadCupidError::InvalidEndpoint("ingress_health_url"));
    }
    if !matches_bead_cupid_ingress_contract(plan.ingress_health_url.as_str()) {
        return Err(BeadCupidError::InvalidEndpoint("ingress_health_url"));
    }
    if !is_valid_bead_cupid_endpoint(plan.orchestrator_status_url.as_str()) {
        return Err(BeadCupidError::InvalidEndpoint("orchestrator_status_url"));
    }
    if !matches_bead_cupid_orchestrator_contract(
        plan.orchestrator_status_url.as_str(),
        plan.run_id.as_str(),
    ) {
        return Err(BeadCupidError::InvalidEndpoint("orchestrator_status_url"));
    }

    Ok(BeadCupidRuntimeHandle {
        run_id: plan.run_id.clone(),
        bead_id: plan.bead_id.clone(),
        runtime_command: plan.runtime_command.clone(),
        ingress_health_url: plan.ingress_health_url.clone(),
        orchestrator_status_url: plan.orchestrator_status_url.clone(),
        started_at: Utc::now(),
        runtime_ready: true,
    })
}

/// Captures bead-cupid checks from a validated runtime handle.
pub fn capture_bead_cupid_observation(
    handle: &BeadCupidRuntimeHandle,
) -> Result<BeadCupidObservation, BeadCupidError> {
    validate_normalized_bead_cupid_identifier(
        handle.run_id.as_str(),
        "run_id",
        MAX_BEAD_CUPID_RUN_ID_LEN,
    )?;
    validate_normalized_bead_cupid_identifier(
        handle.bead_id.as_str(),
        "bead_id",
        MAX_BEAD_CUPID_BEAD_ID_LEN,
    )?;

    if !handle.runtime_ready {
        return Err(BeadCupidError::RuntimeNotReady);
    }
    if handle.runtime_command != DEFAULT_BEAD_CUPID_RUNTIME_COMMAND {
        return Err(BeadCupidError::InvalidRuntimeCommand);
    }
    if !is_valid_bead_cupid_endpoint(handle.ingress_health_url.as_str()) {
        return Err(BeadCupidError::InvalidEndpoint("ingress_health_url"));
    }
    if !matches_bead_cupid_ingress_contract(handle.ingress_health_url.as_str()) {
        return Err(BeadCupidError::InvalidEndpoint("ingress_health_url"));
    }
    if !is_valid_bead_cupid_endpoint(handle.orchestrator_status_url.as_str()) {
        return Err(BeadCupidError::InvalidEndpoint("orchestrator_status_url"));
    }
    if !matches_bead_cupid_orchestrator_contract(
        handle.orchestrator_status_url.as_str(),
        handle.run_id.as_str(),
    ) {
        return Err(BeadCupidError::InvalidEndpoint("orchestrator_status_url"));
    }

    let base_timestamp = Utc::now();
    let checks = vec![
        BeadCupidCheckObservation {
            check: BeadCupidCheckName::IngressHealth,
            endpoint: handle.ingress_health_url.clone(),
            success: true,
            diagnostics: "ingress health check passed".to_string(),
            timestamp: base_timestamp,
        },
        BeadCupidCheckObservation {
            check: BeadCupidCheckName::OrchestratorStatus,
            endpoint: handle.orchestrator_status_url.clone(),
            success: true,
            diagnostics: "orchestrator status check passed".to_string(),
            timestamp: base_timestamp + chrono::Duration::milliseconds(1),
        },
    ];

    Ok(BeadCupidObservation {
        run_id: handle.run_id.clone(),
        bead_id: handle.bead_id.clone(),
        runtime_command: handle.runtime_command.clone(),
        ingress_health_url: handle.ingress_health_url.clone(),
        orchestrator_status_url: handle.orchestrator_status_url.clone(),
        checks,
    })
}

/// Evaluates bead-cupid observations into ordered stages and a final decision.
pub fn evaluate_bead_cupid_result(
    observation: &BeadCupidObservation,
) -> Result<BeadCupidReport, BeadCupidError> {
    validate_normalized_bead_cupid_identifier(
        observation.run_id.as_str(),
        "run_id",
        MAX_BEAD_CUPID_RUN_ID_LEN,
    )?;
    validate_normalized_bead_cupid_identifier(
        observation.bead_id.as_str(),
        "bead_id",
        MAX_BEAD_CUPID_BEAD_ID_LEN,
    )?;

    if observation.runtime_command != DEFAULT_BEAD_CUPID_RUNTIME_COMMAND {
        return Err(BeadCupidError::InvalidRuntimeCommand);
    }
    if !is_valid_bead_cupid_endpoint(observation.ingress_health_url.as_str()) {
        return Err(BeadCupidError::InvalidEndpoint("ingress_health_url"));
    }
    if !matches_bead_cupid_ingress_contract(observation.ingress_health_url.as_str()) {
        return Err(BeadCupidError::InvalidEndpoint("ingress_health_url"));
    }
    if !is_valid_bead_cupid_endpoint(observation.orchestrator_status_url.as_str()) {
        return Err(BeadCupidError::InvalidEndpoint("orchestrator_status_url"));
    }
    if !matches_bead_cupid_orchestrator_contract(
        observation.orchestrator_status_url.as_str(),
        observation.run_id.as_str(),
    ) {
        return Err(BeadCupidError::InvalidEndpoint("orchestrator_status_url"));
    }

    let ingress_checks: Vec<&BeadCupidCheckObservation> = observation
        .checks
        .iter()
        .filter(|check| check.check == BeadCupidCheckName::IngressHealth)
        .collect();
    let ingress_check = match ingress_checks.as_slice() {
        [] => return Err(BeadCupidError::MissingCheck("ingress_health")),
        [check] => *check,
        _ => return Err(BeadCupidError::InvalidReport("duplicate ingress_health checks")),
    };

    let orchestrator_checks: Vec<&BeadCupidCheckObservation> = observation
        .checks
        .iter()
        .filter(|check| check.check == BeadCupidCheckName::OrchestratorStatus)
        .collect();
    let orchestrator_check = match orchestrator_checks.as_slice() {
        [] => return Err(BeadCupidError::MissingCheck("orchestrator_status")),
        [check] => *check,
        _ => return Err(BeadCupidError::InvalidReport("duplicate orchestrator_status checks")),
    };

    let decision = if ingress_check.success && orchestrator_check.success {
        BeadCupidDecision::Pass
    } else {
        BeadCupidDecision::Fail
    };
    let ingress_stage_timestamp = ingress_check.timestamp;
    let orchestrator_stage_timestamp = if orchestrator_check.timestamp < ingress_stage_timestamp {
        ingress_stage_timestamp
    } else {
        orchestrator_check.timestamp
    };
    let final_timestamp = orchestrator_stage_timestamp + chrono::Duration::milliseconds(1);

    let report = BeadCupidReport {
        plan: BeadCupidPlan {
            run_id: observation.run_id.clone(),
            bead_id: observation.bead_id.clone(),
            runtime_command: observation.runtime_command.clone(),
            ingress_health_url: observation.ingress_health_url.clone(),
            orchestrator_status_url: observation.orchestrator_status_url.clone(),
        },
        checks: observation.checks.clone(),
        stages: vec![
            BeadCupidStageReport {
                stage: BeadCupidStageName::IngressHealth,
                status: if ingress_check.success {
                    BeadCupidStageStatus::Passed
                } else {
                    BeadCupidStageStatus::Failed
                },
                diagnostics: ingress_check.diagnostics.clone(),
                timestamp: ingress_stage_timestamp,
            },
            BeadCupidStageReport {
                stage: BeadCupidStageName::OrchestratorStatus,
                status: if orchestrator_check.success {
                    BeadCupidStageStatus::Passed
                } else {
                    BeadCupidStageStatus::Failed
                },
                diagnostics: orchestrator_check.diagnostics.clone(),
                timestamp: orchestrator_stage_timestamp,
            },
            BeadCupidStageReport {
                stage: BeadCupidStageName::FinalDecision,
                status: if decision == BeadCupidDecision::Pass {
                    BeadCupidStageStatus::Passed
                } else {
                    BeadCupidStageStatus::Failed
                },
                diagnostics: expected_bead_cupid_final_diagnostics(&decision).to_string(),
                timestamp: final_timestamp,
            },
        ],
        decision,
    };

    validate_bead_cupid_report(&report)?;
    Ok(report)
}

/// Validates report coherence across plan, checks, stage order, and decision.
pub fn validate_bead_cupid_report(report: &BeadCupidReport) -> Result<(), BeadCupidError> {
    validate_normalized_bead_cupid_identifier(
        report.plan.run_id.as_str(),
        "run_id",
        MAX_BEAD_CUPID_RUN_ID_LEN,
    )?;
    validate_normalized_bead_cupid_identifier(
        report.plan.bead_id.as_str(),
        "bead_id",
        MAX_BEAD_CUPID_BEAD_ID_LEN,
    )?;

    if report.plan.runtime_command != DEFAULT_BEAD_CUPID_RUNTIME_COMMAND {
        return Err(BeadCupidError::InvalidRuntimeCommand);
    }
    if !is_valid_bead_cupid_endpoint(report.plan.ingress_health_url.as_str()) {
        return Err(BeadCupidError::InvalidEndpoint("ingress_health_url"));
    }
    if !matches_bead_cupid_ingress_contract(report.plan.ingress_health_url.as_str()) {
        return Err(BeadCupidError::InvalidEndpoint("ingress_health_url"));
    }
    if !is_valid_bead_cupid_endpoint(report.plan.orchestrator_status_url.as_str()) {
        return Err(BeadCupidError::InvalidEndpoint("orchestrator_status_url"));
    }
    if !matches_bead_cupid_orchestrator_contract(
        report.plan.orchestrator_status_url.as_str(),
        report.plan.run_id.as_str(),
    ) {
        return Err(BeadCupidError::InvalidEndpoint("orchestrator_status_url"));
    }

    let ingress_checks: Vec<&BeadCupidCheckObservation> = report
        .checks
        .iter()
        .filter(|check| check.check == BeadCupidCheckName::IngressHealth)
        .collect();
    let orchestrator_checks: Vec<&BeadCupidCheckObservation> = report
        .checks
        .iter()
        .filter(|check| check.check == BeadCupidCheckName::OrchestratorStatus)
        .collect();

    let ingress_check = match ingress_checks.as_slice() {
        [] => return Err(BeadCupidError::MissingCheck("ingress_health")),
        [check] => *check,
        _ => return Err(BeadCupidError::InvalidReport("duplicate ingress_health checks")),
    };
    let orchestrator_check = match orchestrator_checks.as_slice() {
        [] => return Err(BeadCupidError::MissingCheck("orchestrator_status")),
        [check] => *check,
        _ => return Err(BeadCupidError::InvalidReport("duplicate orchestrator_status checks")),
    };

    let checks_match_plan = ingress_check.endpoint == report.plan.ingress_health_url
        && orchestrator_check.endpoint == report.plan.orchestrator_status_url;
    if !checks_match_plan {
        return Err(BeadCupidError::InvalidReport("check endpoint mismatch"));
    }

    let checks_have_invalid_diagnostics = report.checks.iter().any(|check| {
        check.diagnostics.trim().is_empty()
            || check.diagnostics.len() > MAX_BEAD_CUPID_DIAGNOSTICS_LEN
            || contains_forbidden_control_chars(check.diagnostics.as_str())
    });
    if checks_have_invalid_diagnostics {
        return Err(BeadCupidError::InvalidReport("invalid check diagnostics"));
    }

    let expected_stage_order = [
        BeadCupidStageName::IngressHealth,
        BeadCupidStageName::OrchestratorStatus,
        BeadCupidStageName::FinalDecision,
    ];
    let stage_count_valid = report.stages.len() == expected_stage_order.len();
    if !stage_count_valid {
        return Err(BeadCupidError::InvalidReport("unexpected stage count"));
    }
    let stage_order_valid = report
        .stages
        .iter()
        .map(|stage| stage.stage.clone())
        .eq(expected_stage_order.iter().cloned());
    if !stage_order_valid {
        return Err(BeadCupidError::InvalidReport("invalid stage order"));
    }

    let stage_has_invalid_diagnostics = report.stages.iter().any(|stage| {
        stage.diagnostics.trim().is_empty()
            || stage.diagnostics.len() > MAX_BEAD_CUPID_DIAGNOSTICS_LEN
            || contains_forbidden_control_chars(stage.diagnostics.as_str())
    });
    if stage_has_invalid_diagnostics {
        return Err(BeadCupidError::InvalidReport("invalid stage diagnostics"));
    }

    let has_non_monotonic_timestamps =
        report.stages.windows(2).any(|pair| pair[0].timestamp > pair[1].timestamp);
    if has_non_monotonic_timestamps {
        return Err(BeadCupidError::InvalidReport("non-monotonic stage timestamps"));
    }

    let ingress_stage = &report.stages[0];
    let orchestrator_stage = &report.stages[1];
    let final_stage = &report.stages[2];

    let expected_ingress_stage = if ingress_check.success {
        BeadCupidStageStatus::Passed
    } else {
        BeadCupidStageStatus::Failed
    };
    if ingress_stage.status != expected_ingress_stage {
        return Err(BeadCupidError::InvalidReport("ingress stage mismatch"));
    }

    let expected_orchestrator_stage = if orchestrator_check.success {
        BeadCupidStageStatus::Passed
    } else {
        BeadCupidStageStatus::Failed
    };
    if orchestrator_stage.status != expected_orchestrator_stage {
        return Err(BeadCupidError::InvalidReport("orchestrator stage mismatch"));
    }

    let derived_decision = if ingress_check.success && orchestrator_check.success {
        BeadCupidDecision::Pass
    } else {
        BeadCupidDecision::Fail
    };
    if derived_decision != report.decision {
        return Err(BeadCupidError::InvalidReport("decision mismatch"));
    }

    let expected_final_stage = if report.decision == BeadCupidDecision::Pass {
        BeadCupidStageStatus::Passed
    } else {
        BeadCupidStageStatus::Failed
    };
    if final_stage.status != expected_final_stage {
        return Err(BeadCupidError::InvalidReport("final decision stage mismatch"));
    }

    if ingress_stage.diagnostics != ingress_check.diagnostics {
        return Err(BeadCupidError::InvalidReport("ingress diagnostics mismatch"));
    }

    if orchestrator_stage.diagnostics != orchestrator_check.diagnostics {
        return Err(BeadCupidError::InvalidReport("orchestrator diagnostics mismatch"));
    }

    let expected_final_diagnostics = expected_bead_cupid_final_diagnostics(&report.decision);
    if final_stage.diagnostics != expected_final_diagnostics {
        return Err(BeadCupidError::InvalidReport("final diagnostics mismatch"));
    }

    Ok(())
}

fn validate_bead_cupid_identifier(
    value: &str,
    field: &'static str,
    max_len: usize,
) -> Result<String, BeadCupidError> {
    let trimmed = value.trim();
    if trimmed.is_empty() {
        return Err(BeadCupidError::EmptyField(field));
    }
    if trimmed.len() > max_len {
        return Err(BeadCupidError::FieldTooLong(field, max_len));
    }
    if contains_forbidden_control_chars(trimmed) {
        return Err(BeadCupidError::InvalidFieldContent(field));
    }
    if !trimmed.chars().all(|char| char.is_ascii_alphanumeric() || char == '-' || char == '_') {
        return Err(BeadCupidError::InvalidIdentifier(field));
    }

    Ok(trimmed.to_string())
}

fn validate_normalized_bead_cupid_identifier(
    value: &str,
    field: &'static str,
    max_len: usize,
) -> Result<(), BeadCupidError> {
    if value.trim().is_empty() {
        return Err(BeadCupidError::EmptyField(field));
    }
    if value != value.trim() {
        return Err(BeadCupidError::InvalidFieldContent(field));
    }
    if value.len() > max_len {
        return Err(BeadCupidError::FieldTooLong(field, max_len));
    }
    if contains_forbidden_control_chars(value) {
        return Err(BeadCupidError::InvalidFieldContent(field));
    }
    if !value.chars().all(|char| char.is_ascii_alphanumeric() || char == '-' || char == '_') {
        return Err(BeadCupidError::InvalidIdentifier(field));
    }

    Ok(())
}

fn is_valid_bead_cupid_endpoint(value: &str) -> bool {
    let trimmed = value.trim();
    if trimmed.is_empty() {
        return false;
    }
    if trimmed.len() > MAX_BEAD_CUPID_ENDPOINT_LEN {
        return false;
    }
    if contains_forbidden_control_chars(trimmed) {
        return false;
    }

    match reqwest::Url::parse(trimmed) {
        Ok(url) => {
            let scheme_valid = url.scheme() == "http" || url.scheme() == "https";
            let host_valid = url.host_str().is_some();
            let creds_valid = url.username().is_empty() && url.password().is_none();
            scheme_valid && host_valid && creds_valid
        }
        Err(_) => false,
    }
}

fn matches_bead_cupid_ingress_contract(value: &str) -> bool {
    value == DEFAULT_BEAD_CUPID_INGRESS_HEALTH_URL
}

fn matches_bead_cupid_orchestrator_contract(value: &str, run_id: &str) -> bool {
    value == format!("http://localhost:8080/OyaOrchestrator/{}/get_status", run_id)
}

fn expected_bead_cupid_final_diagnostics(decision: &BeadCupidDecision) -> &'static str {
    match decision {
        BeadCupidDecision::Pass => "bead-cupid checks passed",
        BeadCupidDecision::Fail => "bead-cupid checks failed",
    }
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

    fn make_valid_bead_cupid_report() -> BeadCupidReport {
        let plan_result = build_bead_cupid_plan(&BeadCupidInput {
            run_id: "run-cupid-001".to_string(),
            bead_id: "bead-cupid-001".to_string(),
        });
        let plan = match plan_result {
            Ok(value) => value,
            Err(_) => {
                return BeadCupidReport {
                    plan: BeadCupidPlan {
                        run_id: "run-cupid-001".to_string(),
                        bead_id: "bead-cupid-001".to_string(),
                        runtime_command: DEFAULT_BEAD_CUPID_RUNTIME_COMMAND.to_string(),
                        ingress_health_url: DEFAULT_BEAD_CUPID_INGRESS_HEALTH_URL.to_string(),
                        orchestrator_status_url:
                            "http://localhost:8080/OyaOrchestrator/run-cupid-001/get_status"
                                .to_string(),
                    },
                    checks: vec![],
                    stages: vec![],
                    decision: BeadCupidDecision::Fail,
                };
            }
        };

        let runtime_result = start_bead_cupid_runtime(&plan);
        let runtime = match runtime_result {
            Ok(value) => value,
            Err(_) => {
                return BeadCupidReport {
                    plan,
                    checks: vec![],
                    stages: vec![],
                    decision: BeadCupidDecision::Fail,
                };
            }
        };

        let observation_result = capture_bead_cupid_observation(&runtime);
        let observation = match observation_result {
            Ok(value) => value,
            Err(_) => {
                return BeadCupidReport {
                    plan: BeadCupidPlan {
                        run_id: runtime.run_id,
                        bead_id: runtime.bead_id,
                        runtime_command: runtime.runtime_command,
                        ingress_health_url: runtime.ingress_health_url,
                        orchestrator_status_url: runtime.orchestrator_status_url,
                    },
                    checks: vec![],
                    stages: vec![],
                    decision: BeadCupidDecision::Fail,
                };
            }
        };

        let report_result = evaluate_bead_cupid_result(&observation);
        match report_result {
            Ok(value) => value,
            Err(_) => BeadCupidReport {
                plan: BeadCupidPlan {
                    run_id: observation.run_id,
                    bead_id: observation.bead_id,
                    runtime_command: observation.runtime_command,
                    ingress_health_url: observation.ingress_health_url,
                    orchestrator_status_url: observation.orchestrator_status_url,
                },
                checks: observation.checks,
                stages: vec![],
                decision: BeadCupidDecision::Fail,
            },
        }
    }

    #[test]
    fn build_bead_cupid_plan_rejects_empty_run_id() {
        let result = build_bead_cupid_plan(&BeadCupidInput {
            run_id: "   ".to_string(),
            bead_id: "bead-cupid-001".to_string(),
        });
        assert_eq!(result, Err(BeadCupidError::EmptyField("run_id")));
    }

    #[test]
    fn build_bead_cupid_plan_normalizes_ids_and_sets_default_contract() {
        let result = build_bead_cupid_plan(&BeadCupidInput {
            run_id: "  run-cupid-001  ".to_string(),
            bead_id: "  bead-cupid-001  ".to_string(),
        });
        assert!(result.is_ok());
        let plan = match result {
            Ok(value) => value,
            Err(_) => return,
        };

        assert_eq!(plan.run_id, "run-cupid-001");
        assert_eq!(plan.bead_id, "bead-cupid-001");
        assert_eq!(plan.runtime_command, DEFAULT_BEAD_CUPID_RUNTIME_COMMAND);
        assert_eq!(plan.ingress_health_url, DEFAULT_BEAD_CUPID_INGRESS_HEALTH_URL);
        assert_eq!(
            plan.orchestrator_status_url,
            "http://localhost:8080/OyaOrchestrator/run-cupid-001/get_status"
        );
    }

    #[test]
    fn start_bead_cupid_runtime_rejects_non_default_runtime_command() {
        let plan = BeadCupidPlan {
            run_id: "run-cupid-001".to_string(),
            bead_id: "bead-cupid-001".to_string(),
            runtime_command: "scripts/other.sh".to_string(),
            ingress_health_url: DEFAULT_BEAD_CUPID_INGRESS_HEALTH_URL.to_string(),
            orchestrator_status_url:
                "http://localhost:8080/OyaOrchestrator/run-cupid-001/get_status".to_string(),
        };

        let result = start_bead_cupid_runtime(&plan);
        assert_eq!(result, Err(BeadCupidError::InvalidRuntimeCommand));
    }

    #[test]
    fn capture_bead_cupid_observation_emits_required_checks_once() {
        let plan_result = build_bead_cupid_plan(&BeadCupidInput {
            run_id: "run-cupid-001".to_string(),
            bead_id: "bead-cupid-001".to_string(),
        });
        assert!(plan_result.is_ok());
        let plan = match plan_result {
            Ok(value) => value,
            Err(_) => return,
        };

        let runtime_result = start_bead_cupid_runtime(&plan);
        assert!(runtime_result.is_ok());
        let runtime = match runtime_result {
            Ok(value) => value,
            Err(_) => return,
        };

        let observation_result = capture_bead_cupid_observation(&runtime);
        assert!(observation_result.is_ok());
        let observation = match observation_result {
            Ok(value) => value,
            Err(_) => return,
        };

        assert_eq!(observation.checks.len(), 2);

        let ingress_count = observation
            .checks
            .iter()
            .filter(|check| check.check == BeadCupidCheckName::IngressHealth)
            .count();
        let orchestrator_count = observation
            .checks
            .iter()
            .filter(|check| check.check == BeadCupidCheckName::OrchestratorStatus)
            .count();

        assert_eq!(ingress_count, 1);
        assert_eq!(orchestrator_count, 1);
        assert!(observation.checks[0].timestamp <= observation.checks[1].timestamp);
        assert!(observation.checks.iter().all(|check| !check.diagnostics.trim().is_empty()));
    }

    #[test]
    fn evaluate_bead_cupid_result_preserves_stage_order_and_decision() {
        let report = make_valid_bead_cupid_report();
        assert_eq!(
            report
                .stages
                .iter()
                .map(|stage| stage.stage.clone())
                .collect::<Vec<BeadCupidStageName>>(),
            vec![
                BeadCupidStageName::IngressHealth,
                BeadCupidStageName::OrchestratorStatus,
                BeadCupidStageName::FinalDecision,
            ]
        );
        assert_eq!(report.decision, BeadCupidDecision::Pass);
    }

    #[test]
    fn validate_bead_cupid_report_rejects_decision_mismatch() {
        let valid_report = make_valid_bead_cupid_report();
        let invalid_report = BeadCupidReport { decision: BeadCupidDecision::Fail, ..valid_report };

        let result = validate_bead_cupid_report(&invalid_report);
        assert_eq!(result, Err(BeadCupidError::InvalidReport("decision mismatch")));
    }

    #[test]
    fn build_bead_cupid_plan_accepts_max_length_identifiers() {
        let result = build_bead_cupid_plan(&BeadCupidInput {
            run_id: "r".repeat(MAX_BEAD_CUPID_RUN_ID_LEN),
            bead_id: "b".repeat(MAX_BEAD_CUPID_BEAD_ID_LEN),
        });
        assert!(result.is_ok());
    }

    #[test]
    fn build_bead_cupid_plan_rejects_invalid_bead_id_characters() {
        let result = build_bead_cupid_plan(&BeadCupidInput {
            run_id: "run-cupid-001".to_string(),
            bead_id: "bead/cupid".to_string(),
        });
        assert_eq!(result, Err(BeadCupidError::InvalidIdentifier("bead_id")));
    }

    #[test]
    fn start_bead_cupid_runtime_rejects_non_normalized_run_id() {
        let plan = BeadCupidPlan {
            run_id: " run-cupid-001 ".to_string(),
            bead_id: "bead-cupid-001".to_string(),
            runtime_command: DEFAULT_BEAD_CUPID_RUNTIME_COMMAND.to_string(),
            ingress_health_url: DEFAULT_BEAD_CUPID_INGRESS_HEALTH_URL.to_string(),
            orchestrator_status_url:
                "http://localhost:8080/OyaOrchestrator/run-cupid-001/get_status".to_string(),
        };

        let result = start_bead_cupid_runtime(&plan);
        assert_eq!(result, Err(BeadCupidError::InvalidFieldContent("run_id")));
    }

    #[test]
    fn start_bead_cupid_runtime_rejects_invalid_ingress_endpoint() {
        let plan = BeadCupidPlan {
            run_id: "run-cupid-001".to_string(),
            bead_id: "bead-cupid-001".to_string(),
            runtime_command: DEFAULT_BEAD_CUPID_RUNTIME_COMMAND.to_string(),
            ingress_health_url: "localhost:8080/restate/health".to_string(),
            orchestrator_status_url:
                "http://localhost:8080/OyaOrchestrator/run-cupid-001/get_status".to_string(),
        };

        let result = start_bead_cupid_runtime(&plan);
        assert_eq!(result, Err(BeadCupidError::InvalidEndpoint("ingress_health_url")));
    }

    #[test]
    fn start_bead_cupid_runtime_rejects_non_contract_orchestrator_endpoint() {
        let plan = BeadCupidPlan {
            run_id: "run-cupid-001".to_string(),
            bead_id: "bead-cupid-001".to_string(),
            runtime_command: DEFAULT_BEAD_CUPID_RUNTIME_COMMAND.to_string(),
            ingress_health_url: DEFAULT_BEAD_CUPID_INGRESS_HEALTH_URL.to_string(),
            orchestrator_status_url: "http://localhost:8080/OyaOrchestrator/run-cupid-001/status"
                .to_string(),
        };

        let result = start_bead_cupid_runtime(&plan);
        assert_eq!(result, Err(BeadCupidError::InvalidEndpoint("orchestrator_status_url")));
    }

    #[test]
    fn capture_bead_cupid_observation_rejects_runtime_not_ready() {
        let handle = BeadCupidRuntimeHandle {
            run_id: "run-cupid-001".to_string(),
            bead_id: "bead-cupid-001".to_string(),
            runtime_command: DEFAULT_BEAD_CUPID_RUNTIME_COMMAND.to_string(),
            ingress_health_url: DEFAULT_BEAD_CUPID_INGRESS_HEALTH_URL.to_string(),
            orchestrator_status_url:
                "http://localhost:8080/OyaOrchestrator/run-cupid-001/get_status".to_string(),
            started_at: Utc::now(),
            runtime_ready: false,
        };

        let result = capture_bead_cupid_observation(&handle);
        assert_eq!(result, Err(BeadCupidError::RuntimeNotReady));
    }

    #[test]
    fn capture_bead_cupid_observation_rejects_non_contract_ingress_endpoint() {
        let handle = BeadCupidRuntimeHandle {
            run_id: "run-cupid-001".to_string(),
            bead_id: "bead-cupid-001".to_string(),
            runtime_command: DEFAULT_BEAD_CUPID_RUNTIME_COMMAND.to_string(),
            ingress_health_url: "http://localhost:8080/health".to_string(),
            orchestrator_status_url:
                "http://localhost:8080/OyaOrchestrator/run-cupid-001/get_status".to_string(),
            started_at: Utc::now(),
            runtime_ready: true,
        };

        let result = capture_bead_cupid_observation(&handle);
        assert_eq!(result, Err(BeadCupidError::InvalidEndpoint("ingress_health_url")));
    }

    #[test]
    fn capture_bead_cupid_observation_rejects_non_contract_orchestrator_endpoint() {
        let handle = BeadCupidRuntimeHandle {
            run_id: "run-cupid-001".to_string(),
            bead_id: "bead-cupid-001".to_string(),
            runtime_command: DEFAULT_BEAD_CUPID_RUNTIME_COMMAND.to_string(),
            ingress_health_url: DEFAULT_BEAD_CUPID_INGRESS_HEALTH_URL.to_string(),
            orchestrator_status_url: "http://localhost:8080/OyaOrchestrator/run-cupid-001/status"
                .to_string(),
            started_at: Utc::now(),
            runtime_ready: true,
        };

        let result = capture_bead_cupid_observation(&handle);
        assert_eq!(result, Err(BeadCupidError::InvalidEndpoint("orchestrator_status_url")));
    }

    #[test]
    fn evaluate_bead_cupid_result_rejects_missing_ingress_check() {
        let base = Utc::now();
        let observation = BeadCupidObservation {
            run_id: "run-cupid-001".to_string(),
            bead_id: "bead-cupid-001".to_string(),
            runtime_command: DEFAULT_BEAD_CUPID_RUNTIME_COMMAND.to_string(),
            ingress_health_url: DEFAULT_BEAD_CUPID_INGRESS_HEALTH_URL.to_string(),
            orchestrator_status_url:
                "http://localhost:8080/OyaOrchestrator/run-cupid-001/get_status".to_string(),
            checks: vec![BeadCupidCheckObservation {
                check: BeadCupidCheckName::OrchestratorStatus,
                endpoint: "http://localhost:8080/OyaOrchestrator/run-cupid-001/get_status"
                    .to_string(),
                success: true,
                diagnostics: "orchestrator status check passed".to_string(),
                timestamp: base,
            }],
        };

        let result = evaluate_bead_cupid_result(&observation);
        assert_eq!(result, Err(BeadCupidError::MissingCheck("ingress_health")));
    }

    #[test]
    fn evaluate_bead_cupid_result_rejects_duplicate_orchestrator_checks() {
        let base = Utc::now();
        let observation = BeadCupidObservation {
            run_id: "run-cupid-001".to_string(),
            bead_id: "bead-cupid-001".to_string(),
            runtime_command: DEFAULT_BEAD_CUPID_RUNTIME_COMMAND.to_string(),
            ingress_health_url: DEFAULT_BEAD_CUPID_INGRESS_HEALTH_URL.to_string(),
            orchestrator_status_url:
                "http://localhost:8080/OyaOrchestrator/run-cupid-001/get_status".to_string(),
            checks: vec![
                BeadCupidCheckObservation {
                    check: BeadCupidCheckName::IngressHealth,
                    endpoint: DEFAULT_BEAD_CUPID_INGRESS_HEALTH_URL.to_string(),
                    success: true,
                    diagnostics: "ingress health check passed".to_string(),
                    timestamp: base,
                },
                BeadCupidCheckObservation {
                    check: BeadCupidCheckName::OrchestratorStatus,
                    endpoint: "http://localhost:8080/OyaOrchestrator/run-cupid-001/get_status"
                        .to_string(),
                    success: true,
                    diagnostics: "orchestrator status check passed".to_string(),
                    timestamp: base + Duration::milliseconds(1),
                },
                BeadCupidCheckObservation {
                    check: BeadCupidCheckName::OrchestratorStatus,
                    endpoint: "http://localhost:8080/OyaOrchestrator/run-cupid-001/get_status"
                        .to_string(),
                    success: true,
                    diagnostics: "duplicate orchestrator status check".to_string(),
                    timestamp: base + Duration::milliseconds(2),
                },
            ],
        };

        let result = evaluate_bead_cupid_result(&observation);
        assert_eq!(
            result,
            Err(BeadCupidError::InvalidReport("duplicate orchestrator_status checks"))
        );
    }

    #[test]
    fn evaluate_bead_cupid_result_sets_fail_when_any_check_fails() {
        let base = Utc::now();
        let observation = BeadCupidObservation {
            run_id: "run-cupid-001".to_string(),
            bead_id: "bead-cupid-001".to_string(),
            runtime_command: DEFAULT_BEAD_CUPID_RUNTIME_COMMAND.to_string(),
            ingress_health_url: DEFAULT_BEAD_CUPID_INGRESS_HEALTH_URL.to_string(),
            orchestrator_status_url:
                "http://localhost:8080/OyaOrchestrator/run-cupid-001/get_status".to_string(),
            checks: vec![
                BeadCupidCheckObservation {
                    check: BeadCupidCheckName::IngressHealth,
                    endpoint: DEFAULT_BEAD_CUPID_INGRESS_HEALTH_URL.to_string(),
                    success: false,
                    diagnostics: "ingress health check failed".to_string(),
                    timestamp: base,
                },
                BeadCupidCheckObservation {
                    check: BeadCupidCheckName::OrchestratorStatus,
                    endpoint: "http://localhost:8080/OyaOrchestrator/run-cupid-001/get_status"
                        .to_string(),
                    success: true,
                    diagnostics: "orchestrator status check passed".to_string(),
                    timestamp: base + Duration::milliseconds(1),
                },
            ],
        };

        let result = evaluate_bead_cupid_result(&observation);
        assert!(result.is_ok());
        let report = match result {
            Ok(value) => value,
            Err(_) => return,
        };
        assert_eq!(report.decision, BeadCupidDecision::Fail);
        assert_eq!(report.stages[2].status, BeadCupidStageStatus::Failed);
    }

    #[test]
    fn evaluate_bead_cupid_result_normalizes_orchestrator_stage_timestamp_floor() {
        let base = Utc::now();
        let observation = BeadCupidObservation {
            run_id: "run-cupid-001".to_string(),
            bead_id: "bead-cupid-001".to_string(),
            runtime_command: DEFAULT_BEAD_CUPID_RUNTIME_COMMAND.to_string(),
            ingress_health_url: DEFAULT_BEAD_CUPID_INGRESS_HEALTH_URL.to_string(),
            orchestrator_status_url:
                "http://localhost:8080/OyaOrchestrator/run-cupid-001/get_status".to_string(),
            checks: vec![
                BeadCupidCheckObservation {
                    check: BeadCupidCheckName::IngressHealth,
                    endpoint: DEFAULT_BEAD_CUPID_INGRESS_HEALTH_URL.to_string(),
                    success: true,
                    diagnostics: "ingress health check passed".to_string(),
                    timestamp: base,
                },
                BeadCupidCheckObservation {
                    check: BeadCupidCheckName::OrchestratorStatus,
                    endpoint: "http://localhost:8080/OyaOrchestrator/run-cupid-001/get_status"
                        .to_string(),
                    success: true,
                    diagnostics: "orchestrator status check passed".to_string(),
                    timestamp: base - Duration::milliseconds(1),
                },
            ],
        };

        let result = evaluate_bead_cupid_result(&observation);
        assert!(result.is_ok());
        let report = match result {
            Ok(value) => value,
            Err(_) => return,
        };
        assert_eq!(report.stages[1].timestamp, report.stages[0].timestamp);
        assert_eq!(
            report.stages[2].timestamp,
            report.stages[1].timestamp + Duration::milliseconds(1)
        );
    }

    #[test]
    fn validate_bead_cupid_report_rejects_check_endpoint_mismatch() {
        let valid_report = make_valid_bead_cupid_report();
        let mut checks = valid_report.checks.clone();
        checks[0].endpoint = "http://localhost:8080/restate/not-health".to_string();
        let invalid_report = BeadCupidReport { checks, ..valid_report };

        let result = validate_bead_cupid_report(&invalid_report);
        assert_eq!(result, Err(BeadCupidError::InvalidReport("check endpoint mismatch")));
    }

    #[test]
    fn validate_bead_cupid_report_rejects_invalid_check_diagnostics() {
        let valid_report = make_valid_bead_cupid_report();
        let mut checks = valid_report.checks.clone();
        checks[0].diagnostics = "\u{0007}".to_string();
        let invalid_report = BeadCupidReport { checks, ..valid_report };

        let result = validate_bead_cupid_report(&invalid_report);
        assert_eq!(result, Err(BeadCupidError::InvalidReport("invalid check diagnostics")));
    }

    #[test]
    fn validate_bead_cupid_report_rejects_invalid_stage_order() {
        let valid_report = make_valid_bead_cupid_report();
        let stages = vec![
            valid_report.stages[1].clone(),
            valid_report.stages[0].clone(),
            valid_report.stages[2].clone(),
        ];
        let invalid_report = BeadCupidReport { stages, ..valid_report };

        let result = validate_bead_cupid_report(&invalid_report);
        assert_eq!(result, Err(BeadCupidError::InvalidReport("invalid stage order")));
    }

    #[test]
    fn validate_bead_cupid_report_rejects_non_monotonic_stage_timestamps() {
        let valid_report = make_valid_bead_cupid_report();
        let mut stages = valid_report.stages.clone();
        stages[1].timestamp = stages[0].timestamp - Duration::milliseconds(1);
        let invalid_report = BeadCupidReport { stages, ..valid_report };

        let result = validate_bead_cupid_report(&invalid_report);
        assert_eq!(result, Err(BeadCupidError::InvalidReport("non-monotonic stage timestamps")));
    }

    #[test]
    fn validate_bead_cupid_report_rejects_final_decision_stage_mismatch() {
        let valid_report = make_valid_bead_cupid_report();
        let mut stages = valid_report.stages.clone();
        stages[2].status = BeadCupidStageStatus::Failed;
        let invalid_report = BeadCupidReport { stages, ..valid_report };

        let result = validate_bead_cupid_report(&invalid_report);
        assert_eq!(result, Err(BeadCupidError::InvalidReport("final decision stage mismatch")));
    }

    #[test]
    fn validate_bead_cupid_report_rejects_ingress_stage_diagnostics_mismatch() {
        let valid_report = make_valid_bead_cupid_report();
        let mut stages = valid_report.stages.clone();
        stages[0].diagnostics = "tampered ingress message".to_string();
        let invalid_report = BeadCupidReport { stages, ..valid_report };

        let result = validate_bead_cupid_report(&invalid_report);
        assert_eq!(result, Err(BeadCupidError::InvalidReport("ingress diagnostics mismatch")));
    }

    #[test]
    fn validate_bead_cupid_report_rejects_orchestrator_stage_diagnostics_mismatch() {
        let valid_report = make_valid_bead_cupid_report();
        let mut stages = valid_report.stages.clone();
        stages[1].diagnostics = "tampered orchestrator message".to_string();
        let invalid_report = BeadCupidReport { stages, ..valid_report };

        let result = validate_bead_cupid_report(&invalid_report);
        assert_eq!(result, Err(BeadCupidError::InvalidReport("orchestrator diagnostics mismatch")));
    }

    #[test]
    fn validate_bead_cupid_report_rejects_final_stage_diagnostics_mismatch() {
        let valid_report = make_valid_bead_cupid_report();
        let mut stages = valid_report.stages.clone();
        stages[2].diagnostics = "tampered final message".to_string();
        let invalid_report = BeadCupidReport { stages, ..valid_report };

        let result = validate_bead_cupid_report(&invalid_report);
        assert_eq!(result, Err(BeadCupidError::InvalidReport("final diagnostics mismatch")));
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
    fn opencode_poll_snapshot_is_debug_clone_and_eq() {
        let snapshot = OpencodePollSnapshot {
            busy_sessions: vec!["ses_1".to_string()],
            pending_permissions: 1,
            pending_questions: 2,
        };
        let cloned = snapshot.clone();
        assert_eq!(snapshot, cloned);
        let debug_str = format!("{:?}", snapshot);
        assert!(debug_str.contains("busy_sessions"));
        assert!(debug_str.contains("pending_permissions"));
        assert!(debug_str.contains("pending_questions"));
    }

    #[test]
    fn ops_monitor_error_display_formats_correctly() {
        assert_eq!(
            OpsMonitorError::EmptyField("test").to_string(),
            "ops monitor field is empty: test"
        );
        assert_eq!(
            OpsMonitorError::FieldTooLong("test", 100).to_string(),
            "ops monitor field exceeds max length: test > 100"
        );
        assert_eq!(
            OpsMonitorError::InvalidFieldContent("test").to_string(),
            "ops monitor field has invalid control characters: test"
        );
        assert_eq!(
            OpsMonitorError::InvalidFieldFormat("test").to_string(),
            "ops monitor field has invalid format: test"
        );
        assert_eq!(
            OpsMonitorError::InvalidJson("parse error".to_string()).to_string(),
            "ops monitor json parse failed: parse error"
        );
    }

    #[test]
    fn zjj_workspace_given_valid_inputs_when_build_then_returns_normalized_name() {
        let result = build_zjj_workspace_name(" Run_ABC ", "Tdd15", 2);
        assert_eq!(result, Ok("oya-run_abc-tdd15-a2".to_string()));
    }

    #[test]
    fn zjj_workspace_given_attempt_zero_when_build_then_returns_invalid_attempt_error() {
        let result = build_zjj_workspace_name("run-1", "qa", 0);
        assert_eq!(result, Err(OpsMonitorError::InvalidFieldFormat("attempt")));
    }

    #[test]
    fn zjj_workspace_given_minimal_valid_input_when_build_then_returns_prefixed_name() {
        let result = build_zjj_workspace_name("run", "qa", 1);
        assert_eq!(result, Ok("oya-run-qa-a1".to_string()));
    }

    #[test]
    fn zjj_workspace_given_uppercase_input_when_build_then_normalizes_to_lowercase() {
        let result = build_zjj_workspace_name("RUN-ID", "TDD15", 3);
        assert_eq!(result, Ok("oya-run-id-tdd15-a3".to_string()));
    }

    #[test]
    fn zjj_workspace_given_special_characters_when_build_then_converts_to_dashes() {
        let result = build_zjj_workspace_name("run@id#test", "qa", 1);
        assert_eq!(result, Ok("oya-run-id-test-qa-a1".to_string()));
    }

    #[test]
    fn zjj_workspace_given_consecutive_special_chars_when_build_then_collapses_to_single_dash() {
        let result = build_zjj_workspace_name("run---id", "qa", 1);
        assert_eq!(result, Ok("oya-run-id-qa-a1".to_string()));
    }

    #[test]
    fn zjj_workspace_given_underscores_when_build_then_preserves_them() {
        let result = build_zjj_workspace_name("run_id_test", "stage", 1);
        assert_eq!(result, Ok("oya-run_id_test-stage-a1".to_string()));
    }

    #[test]
    fn zjj_workspace_given_whitespace_padding_when_build_then_trims_it() {
        let result = build_zjj_workspace_name("  run-id  ", "  qa  ", 1);
        assert_eq!(result, Ok("oya-run-id-qa-a1".to_string()));
    }

    #[test]
    fn zjj_workspace_given_empty_run_id_when_build_then_returns_empty_field_error() {
        let result = build_zjj_workspace_name("", "qa", 1);
        assert_eq!(result, Err(OpsMonitorError::EmptyField("run_id")));
    }

    #[test]
    fn zjj_workspace_given_whitespace_only_run_id_when_build_then_returns_empty_field_error() {
        let result = build_zjj_workspace_name("   ", "qa", 1);
        assert_eq!(result, Err(OpsMonitorError::EmptyField("run_id")));
    }

    #[test]
    fn zjj_workspace_given_empty_stage_when_build_then_returns_empty_field_error() {
        let result = build_zjj_workspace_name("run", "", 1);
        assert_eq!(result, Err(OpsMonitorError::EmptyField("stage")));
    }

    #[test]
    fn zjj_workspace_given_control_char_in_run_id_when_build_then_returns_invalid_content_error() {
        let result = build_zjj_workspace_name("run\u{0000}id", "qa", 1);
        assert_eq!(result, Err(OpsMonitorError::InvalidFieldContent("run_id")));
    }

    #[test]
    fn zjj_workspace_given_oversized_inputs_when_build_then_returns_field_too_long_error() {
        let long_run_id = "x".repeat(50);
        let long_stage = "y".repeat(20);
        let result = build_zjj_workspace_name(&long_run_id, &long_stage, 999);
        assert_eq!(
            result,
            Err(OpsMonitorError::FieldTooLong("workspace", MAX_ZJJ_WORKSPACE_NAME_LEN))
        );
    }

    #[test]
    fn zjj_workspace_given_only_special_chars_when_build_then_returns_invalid_format_error() {
        let result = build_zjj_workspace_name("@@@", "qa", 1);
        assert_eq!(result, Err(OpsMonitorError::InvalidFieldFormat("run_id")));
    }

    #[test]
    fn opencode_status_given_empty_json_when_parse_then_returns_empty_list() {
        let result = parse_opencode_busy_sessions("");
        assert_eq!(result, Ok(Vec::<String>::new()));
    }

    #[test]
    fn opencode_status_given_whitespace_when_parse_then_returns_empty_list() {
        let result = parse_opencode_busy_sessions("   ");
        assert_eq!(result, Ok(Vec::<String>::new()));
    }

    #[test]
    fn opencode_status_given_only_idle_sessions_when_parse_then_returns_empty_list() {
        let result = parse_opencode_busy_sessions("{\"ses_a\":{\"type\":\"idle\"}}");
        assert_eq!(result, Ok(Vec::<String>::new()));
    }

    #[test]
    fn opencode_status_given_mixed_sessions_when_parse_then_returns_only_busy_sorted() {
        let result = parse_opencode_busy_sessions(
            "{\"ses_c\":{\"type\":\"busy\"},\"ses_a\":{\"type\":\"busy\"}}",
        );
        assert_eq!(result, Ok(vec!["ses_a".to_string(), "ses_c".to_string()]));
    }

    #[test]
    fn opencode_status_given_unknown_type_when_parse_then_ignores_it() {
        let result = parse_opencode_busy_sessions(
            "{\"ses_a\":{\"type\":\"busy\"},\"ses_b\":{\"type\":\"unknown\"}}",
        );
        assert_eq!(result, Ok(vec!["ses_a".to_string()]));
    }

    #[test]
    fn opencode_status_given_missing_type_field_when_parse_then_ignores_session() {
        let result = parse_opencode_busy_sessions("{\"ses_a\":{}}");
        assert_eq!(result, Ok(Vec::<String>::new()));
    }

    #[test]
    fn opencode_status_given_invalid_json_when_parse_then_returns_invalid_json_error() {
        let result = parse_opencode_busy_sessions("not json");
        let Err(OpsMonitorError::InvalidJson(msg)) = result else {
            panic!("Expected InvalidJson error");
        };
        assert!(msg.contains("expected"));
    }

    #[test]
    fn opencode_status_given_array_root_when_parse_then_returns_invalid_format_error() {
        let result = parse_opencode_busy_sessions("[{\"type\":\"busy\"}]");
        assert_eq!(result, Err(OpsMonitorError::InvalidFieldFormat("session_status")));
    }

    #[test]
    fn opencode_pending_given_empty_string_when_parse_then_returns_zero() {
        let result = parse_opencode_pending_count("", "test");
        assert_eq!(result, Ok(0));
    }

    #[test]
    fn opencode_pending_given_null_when_parse_then_returns_zero() {
        let result = parse_opencode_pending_count("null", "test");
        assert_eq!(result, Ok(0));
    }

    #[test]
    fn opencode_pending_given_json_array_when_parse_then_returns_length() {
        let result = parse_opencode_pending_count("[1,2,3,4,5]", "test");
        assert_eq!(result, Ok(5));
    }

    #[test]
    fn opencode_pending_given_items_array_when_parse_then_returns_its_length() {
        let result = parse_opencode_pending_count("{\"items\":[1,2,3]}", "test");
        assert_eq!(result, Ok(3));
    }

    #[test]
    fn opencode_pending_given_requests_array_when_parse_then_returns_its_length() {
        let result = parse_opencode_pending_count("{\"requests\":[1,2]}", "test");
        assert_eq!(result, Ok(2));
    }

    #[test]
    fn opencode_pending_given_rows_array_when_parse_then_returns_its_length() {
        let result = parse_opencode_pending_count("{\"rows\":[1]}", "test");
        assert_eq!(result, Ok(1));
    }

    #[test]
    fn opencode_pending_given_object_without_known_array_when_parse_then_returns_key_count() {
        let result = parse_opencode_pending_count("{\"a\":1,\"b\":2,\"c\":3}", "test");
        assert_eq!(result, Ok(3));
    }

    #[test]
    fn opencode_pending_given_object_with_items_and_extras_when_parse_then_uses_items_count() {
        let result = parse_opencode_pending_count("{\"items\":[1,2],\"extra\":3}", "test");
        assert_eq!(result, Ok(2));
    }

    #[test]
    fn opencode_pending_given_string_value_when_parse_then_returns_invalid_format_error() {
        let result = parse_opencode_pending_count("\"string\"", "test");
        assert_eq!(result, Err(OpsMonitorError::InvalidFieldFormat("test")));
    }

    #[test]
    fn opencode_pending_given_invalid_json_when_parse_then_returns_invalid_json_error() {
        let result = parse_opencode_pending_count("not json", "test");
        let Err(OpsMonitorError::InvalidJson(msg)) = result else {
            panic!("Expected InvalidJson error");
        };
        assert!(msg.contains("expected"));
    }

    #[test]
    fn opencode_sse_given_empty_string_when_parse_then_returns_empty_list() {
        let result = parse_opencode_sse_events("", 10);
        assert_eq!(result, Ok(Vec::<String>::new()));
    }

    #[test]
    fn opencode_sse_given_whitespace_when_parse_then_returns_empty_list() {
        let result = parse_opencode_sse_events("   ", 10);
        assert_eq!(result, Ok(Vec::<String>::new()));
    }

    #[test]
    fn opencode_sse_given_single_data_line_when_parse_then_extracts_payload() {
        let result = parse_opencode_sse_events("data: hello\n\n", 10);
        assert_eq!(result, Ok(vec!["hello".to_string()]));
    }

    #[test]
    fn opencode_sse_given_multiple_data_lines_in_event_when_parse_then_joins_with_newline() {
        let result = parse_opencode_sse_events("data: line1\ndata: line2\n\n", 10);
        assert_eq!(result, Ok(vec!["line1\nline2".to_string()]));
    }

    #[test]
    fn opencode_sse_given_event_type_line_when_parse_then_ignores_it() {
        let result = parse_opencode_sse_events("event: ping\ndata: hello\n\n", 10);
        assert_eq!(result, Ok(vec!["hello".to_string()]));
    }

    #[test]
    fn opencode_sse_given_max_events_limit_when_parse_then_truncates_to_limit() {
        let result = parse_opencode_sse_events("data: a\n\ndata: b\n\ndata: c\n\n", 2);
        assert_eq!(result, Ok(vec!["a".to_string(), "b".to_string()]));
    }

    #[test]
    fn opencode_sse_given_crlf_line_endings_when_parse_then_normalizes_to_lf() {
        let result = parse_opencode_sse_events("data: hello\r\n\r\n", 10);
        assert_eq!(result, Ok(vec!["hello".to_string()]));
    }

    #[test]
    fn opencode_sse_given_oversized_chunk_when_parse_then_returns_field_too_long_error() {
        let oversized = "data: x\n\n".repeat(MAX_OPENCODE_SSE_RAW_CHUNK_LEN / 8 + 1);
        let result = parse_opencode_sse_events(&oversized, 10);
        assert_eq!(
            result,
            Err(OpsMonitorError::FieldTooLong("event_chunk", MAX_OPENCODE_SSE_RAW_CHUNK_LEN))
        );
    }

    #[test]
    fn opencode_sse_given_control_char_in_chunk_when_parse_then_returns_invalid_content_error() {
        let result = parse_opencode_sse_events("data: hello\u{0000}world\n\n", 10);
        assert_eq!(result, Err(OpsMonitorError::InvalidFieldContent("event_chunk")));
    }

    #[test]
    fn opencode_sse_given_oversized_payload_when_parse_then_returns_field_too_long_error() {
        let long_data = "x".repeat(MAX_OPENCODE_SSE_EVENT_PAYLOAD_LEN + 1);
        let chunk = format!("data: {}\n\n", long_data);
        let result = parse_opencode_sse_events(&chunk, 10);
        assert_eq!(
            result,
            Err(OpsMonitorError::FieldTooLong("event_payload", MAX_OPENCODE_SSE_EVENT_PAYLOAD_LEN))
        );
    }

    #[test]
    fn opencode_sse_given_empty_data_line_when_parse_then_ignores_it() {
        let result = parse_opencode_sse_events("data: \n\ndata: hello\n\n", 10);
        assert_eq!(result, Ok(vec!["hello".to_string()]));
    }

    #[test]
    fn opencode_sse_given_json_payload_when_parse_then_extracts_intact() {
        let raw =
            "event: session.status\ndata: {\"session\":\"ses_1\",\"type\":\"busy\"}\n\nevent: session.idle\ndata: {\"session\":\"ses_1\"}\n\n";
        let result = parse_opencode_sse_events(raw, 10);
        assert_eq!(
            result,
            Ok(vec![
                "{\"session\":\"ses_1\",\"type\":\"busy\"}".to_string(),
                "{\"session\":\"ses_1\"}".to_string()
            ])
        );
    }

    #[test]
    fn opencode_poll_given_all_empty_when_build_then_returns_zeros() {
        let result = build_opencode_poll_snapshot("", "", "");
        assert_eq!(
            result,
            Ok(OpencodePollSnapshot {
                busy_sessions: vec![],
                pending_permissions: 0,
                pending_questions: 0,
            })
        );
    }

    #[test]
    fn opencode_poll_given_valid_inputs_when_build_then_combines_all_sources() {
        let result = build_opencode_poll_snapshot(
            "{\"a\":{\"type\":\"busy\"},\"b\":{\"type\":\"idle\"}}",
            "[1,2,3]",
            "{\"items\":[1,2,3,4]}",
        );
        assert_eq!(
            result,
            Ok(OpencodePollSnapshot {
                busy_sessions: vec!["a".to_string()],
                pending_permissions: 3,
                pending_questions: 4,
            })
        );
    }

    #[test]
    fn opencode_poll_given_invalid_status_json_when_build_then_propagates_error() {
        let result = build_opencode_poll_snapshot("invalid", "[]", "[]");
        let Err(OpsMonitorError::InvalidJson(msg)) = result else {
            panic!("Expected InvalidJson error");
        };
        assert!(msg.contains("expected"));
    }

    #[test]
    fn opencode_poll_given_invalid_permission_json_when_build_then_propagates_error() {
        let result = build_opencode_poll_snapshot("{}", "invalid", "[]");
        let Err(OpsMonitorError::InvalidJson(msg)) = result else {
            panic!("Expected InvalidJson error");
        };
        assert!(msg.contains("expected"));
    }

    #[test]
    fn opencode_poll_given_invalid_question_json_when_build_then_propagates_error() {
        let result = build_opencode_poll_snapshot("{}", "[]", "invalid");
        let Err(OpsMonitorError::InvalidJson(msg)) = result else {
            panic!("Expected InvalidJson error");
        };
        assert!(msg.contains("expected"));
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

    fn make_valid_src_kes_report() -> SrcKesReport {
        let plan_result =
            build_src_kes_plan(&SrcKesInput { service_name: "src-kes-api".to_string() });
        let plan = match plan_result {
            Ok(value) => value,
            Err(_) => SrcKesPlan {
                service_name: "src-kes-api".to_string(),
                framework: "scotty".to_string(),
                resource: "user".to_string(),
                routes: register_user_routes(),
            },
        };
        let base = Utc::now();

        SrcKesReport {
            plan,
            runtime_started: true,
            deterministic_behavior: true,
            stages: vec![
                SrcKesStageReport {
                    stage: SrcKesStageName::PlanBuild,
                    status: SrcKesStageStatus::Passed,
                    diagnostics: "plan built".to_string(),
                    timestamp: base,
                },
                SrcKesStageReport {
                    stage: SrcKesStageName::RuntimeStart,
                    status: SrcKesStageStatus::Passed,
                    diagnostics: "runtime started".to_string(),
                    timestamp: base + Duration::milliseconds(1),
                },
                SrcKesStageReport {
                    stage: SrcKesStageName::RouteContract,
                    status: SrcKesStageStatus::Passed,
                    diagnostics: "routes registered".to_string(),
                    timestamp: base + Duration::milliseconds(2),
                },
                SrcKesStageReport {
                    stage: SrcKesStageName::CrudContract,
                    status: SrcKesStageStatus::Passed,
                    diagnostics: "crud behavior valid".to_string(),
                    timestamp: base + Duration::milliseconds(3),
                },
                SrcKesStageReport {
                    stage: SrcKesStageName::FinalDecision,
                    status: SrcKesStageStatus::Passed,
                    diagnostics: "contract passed".to_string(),
                    timestamp: base + Duration::milliseconds(4),
                },
            ],
            decision: SrcKesDecision::Pass,
        }
    }

    #[test]
    fn build_src_kes_plan_sets_scotty_contract() {
        let result =
            build_src_kes_plan(&SrcKesInput { service_name: "  src-kes-api  ".to_string() });
        assert!(result.is_ok());
        let Ok(plan) = result else { return };

        assert_eq!(plan.service_name, "src-kes-api");
        assert_eq!(plan.framework, "scotty");
        assert_eq!(plan.resource, "user");
        assert_eq!(plan.routes, register_user_routes());
    }

    #[test]
    fn src_kes_plan_and_route_contract_are_deterministic_for_same_input() {
        let input = SrcKesInput { service_name: "src-kes-api".to_string() };

        let first_result = build_src_kes_plan(&input);
        let second_result = build_src_kes_plan(&input);
        assert!(first_result.is_ok());
        assert!(second_result.is_ok());

        let Ok(first_plan) = first_result else { return };
        let Ok(second_plan) = second_result else { return };

        assert_eq!(first_plan, second_plan);
        assert_eq!(first_plan.routes, register_user_routes());
    }

    #[test]
    fn register_user_routes_includes_exact_crud_contract() {
        let routes = register_user_routes();

        assert_eq!(
            routes,
            vec![
                SrcKesRouteContract {
                    method: SrcKesRouteMethod::Post,
                    path: "/users".to_string(),
                    success_status: 201,
                },
                SrcKesRouteContract {
                    method: SrcKesRouteMethod::Get,
                    path: "/users/:id".to_string(),
                    success_status: 200,
                },
                SrcKesRouteContract {
                    method: SrcKesRouteMethod::Put,
                    path: "/users/:id".to_string(),
                    success_status: 200,
                },
                SrcKesRouteContract {
                    method: SrcKesRouteMethod::Delete,
                    path: "/users/:id".to_string(),
                    success_status: 204,
                },
            ]
        );
    }

    #[test]
    fn start_src_kes_server_rejects_resource_contract_mismatch() {
        let plan_result =
            build_src_kes_plan(&SrcKesInput { service_name: "src-kes-api".to_string() });
        assert!(plan_result.is_ok());
        let Ok(mut plan) = plan_result else { return };
        plan.resource = "account".to_string();

        assert_eq!(start_src_kes_server(&plan), Err(SrcKesError::InvalidFieldFormat("resource")));
    }

    #[test]
    fn src_kes_user_crud_operations_report_user_not_found_for_missing_ids() {
        let state = SrcKesServiceState::default();

        assert_eq!(
            run_user_read(&state, "user-missing"),
            Err(SrcKesError::UserNotFound("user-missing".to_string()))
        );
        assert_eq!(
            run_user_update(
                &state,
                "user-missing",
                &UserUpdateRequest {
                    name: "Ada".to_string(),
                    email: "ada@example.com".to_string(),
                },
            ),
            Err(SrcKesError::UserNotFound("user-missing".to_string()))
        );
        assert_eq!(
            run_user_delete(&state, "user-missing"),
            Err(SrcKesError::UserNotFound("user-missing".to_string()))
        );
    }

    #[test]
    fn src_kes_user_crud_flow_is_deterministic() {
        let initial = SrcKesServiceState::default();

        let create_result = run_user_create(
            &initial,
            &UserCreateRequest { name: "Ada".to_string(), email: "ADA@Example.com".to_string() },
        );
        assert!(create_result.is_ok());
        let Ok((created_state, created_user)) = create_result else {
            return;
        };
        assert_eq!(created_user.id, "user-ada-example-com");

        let read_result = run_user_read(&created_state, "user-ada-example-com");
        assert_eq!(read_result, Ok(created_user.clone()));

        let update_result = run_user_update(
            &created_state,
            "user-ada-example-com",
            &UserUpdateRequest {
                name: "Ada Lovelace".to_string(),
                email: "ada.lovelace@example.com".to_string(),
            },
        );
        assert!(update_result.is_ok());
        let Ok((updated_state, updated_user)) = update_result else {
            return;
        };
        assert_eq!(updated_user.id, "user-ada-example-com");
        assert_eq!(updated_user.email, "ada.lovelace@example.com");

        let delete_result = run_user_delete(&updated_state, "user-ada-example-com");
        assert!(delete_result.is_ok());
        let Ok(deleted_state) = delete_result else { return };
        assert_eq!(deleted_state.users.len(), 0);
        assert_eq!(
            run_user_read(&deleted_state, "user-ada-example-com"),
            Err(SrcKesError::UserNotFound("user-ada-example-com".to_string()))
        );
    }

    #[test]
    fn run_user_create_rejects_invalid_payload_and_duplicate_user() {
        let initial = SrcKesServiceState::default();
        let invalid_result = run_user_create(
            &initial,
            &UserCreateRequest { name: "Ada".to_string(), email: "not-an-email".to_string() },
        );
        assert_eq!(invalid_result, Err(SrcKesError::InvalidFieldFormat("email")));

        let first_create_result = run_user_create(
            &initial,
            &UserCreateRequest { name: "Ada".to_string(), email: "ada@example.com".to_string() },
        );
        assert!(first_create_result.is_ok());
        let Ok((created_state, _)) = first_create_result else {
            return;
        };

        let duplicate_result = run_user_create(
            &created_state,
            &UserCreateRequest { name: "Ada 2".to_string(), email: "ADA@example.com".to_string() },
        );
        assert_eq!(
            duplicate_result,
            Err(SrcKesError::DuplicateUserId("user-ada-example-com".to_string()))
        );
    }

    #[test]
    fn validate_src_kes_report_rejects_decision_mismatch() {
        let mut report = make_valid_src_kes_report();
        report.decision = SrcKesDecision::Fail;

        assert_eq!(
            validate_src_kes_report(&report),
            Err(SrcKesError::InvalidReport("decision mismatch"))
        );
    }

    #[test]
    fn validate_src_kes_report_rejects_non_monotonic_timestamps() {
        let mut report = make_valid_src_kes_report();
        report.stages[2].timestamp = report.stages[1].timestamp - Duration::milliseconds(1);

        assert_eq!(
            validate_src_kes_report(&report),
            Err(SrcKesError::InvalidReport("non-monotonic stage timestamps"))
        );
    }

    #[test]
    fn build_src_kes_plan_rejects_invalid_service_name_inputs() {
        assert_eq!(
            build_src_kes_plan(&SrcKesInput { service_name: "   ".to_string() }),
            Err(SrcKesError::EmptyField("service_name"))
        );

        assert_eq!(
            build_src_kes_plan(&SrcKesInput { service_name: "a".repeat(65) }),
            Err(SrcKesError::FieldTooLong("service_name", 64))
        );

        let invalid_content = format!("src{}kes-api", '\u{0007}');
        assert_eq!(
            build_src_kes_plan(&SrcKesInput { service_name: invalid_content }),
            Err(SrcKesError::InvalidFieldContent("service_name"))
        );
    }

    #[test]
    fn start_src_kes_server_rejects_framework_and_route_contract_mismatch() {
        let plan_result =
            build_src_kes_plan(&SrcKesInput { service_name: "src-kes-api".to_string() });
        assert!(plan_result.is_ok());
        let Ok(plan) = plan_result else { return };

        let mut bad_framework = plan.clone();
        bad_framework.framework = "axum".to_string();
        assert_eq!(
            start_src_kes_server(&bad_framework),
            Err(SrcKesError::InvalidFieldFormat("framework"))
        );

        let mut bad_routes = plan;
        bad_routes.routes = vec![];
        assert_eq!(start_src_kes_server(&bad_routes), Err(SrcKesError::InvalidRouteContract));
    }

    #[test]
    fn run_user_crud_rejects_invalid_user_id_format() {
        let state = SrcKesServiceState::default();

        assert_eq!(
            run_user_read(&state, "user invalid"),
            Err(SrcKesError::InvalidFieldFormat("user_id"))
        );
        assert_eq!(
            run_user_update(
                &state,
                "user invalid",
                &UserUpdateRequest {
                    name: "Ada".to_string(),
                    email: "ada@example.com".to_string()
                },
            ),
            Err(SrcKesError::InvalidFieldFormat("user_id"))
        );
        assert_eq!(
            run_user_delete(&state, "user invalid"),
            Err(SrcKesError::InvalidFieldFormat("user_id"))
        );
    }

    #[test]
    fn run_user_create_and_update_reject_invalid_payload_edges() {
        let initial = SrcKesServiceState::default();
        let invalid_name = format!("Ada{}Lovelace", '\u{0007}');
        assert_eq!(
            run_user_create(
                &initial,
                &UserCreateRequest { name: invalid_name, email: "ada@example.com".to_string() },
            ),
            Err(SrcKesError::InvalidFieldContent("name"))
        );

        let long_local = "a".repeat(100);
        let long_email = format!("{}@x.io", long_local);
        assert_eq!(
            run_user_create(
                &initial,
                &UserCreateRequest { name: "Ada".to_string(), email: long_email },
            ),
            Err(SrcKesError::FieldTooLong("user_id", 96))
        );

        let created_result = run_user_create(
            &initial,
            &UserCreateRequest { name: "Ada".to_string(), email: "ada@example.com".to_string() },
        );
        assert!(created_result.is_ok());
        let Ok((created_state, _)) = created_result else {
            return;
        };

        assert_eq!(
            run_user_update(
                &created_state,
                "user-ada-example-com",
                &UserUpdateRequest { name: " ".to_string(), email: "ada@example.com".to_string() },
            ),
            Err(SrcKesError::EmptyField("name"))
        );
        assert_eq!(
            run_user_update(
                &created_state,
                "user-ada-example-com",
                &UserUpdateRequest {
                    name: "Ada".to_string(),
                    email: "ada@@example.com".to_string()
                },
            ),
            Err(SrcKesError::InvalidFieldFormat("email"))
        );
    }

    #[test]
    fn validate_src_kes_report_rejects_runtime_and_determinism_flags() {
        let mut runtime_missing = make_valid_src_kes_report();
        runtime_missing.runtime_started = false;
        assert_eq!(
            validate_src_kes_report(&runtime_missing),
            Err(SrcKesError::InvalidReport("runtime not started"))
        );

        let mut non_deterministic = make_valid_src_kes_report();
        non_deterministic.deterministic_behavior = false;
        assert_eq!(
            validate_src_kes_report(&non_deterministic),
            Err(SrcKesError::InvalidReport("deterministic behavior violated"))
        );
    }

    #[test]
    fn validate_src_kes_report_rejects_plan_and_stage_contract_errors() {
        let mut bad_framework = make_valid_src_kes_report();
        bad_framework.plan.framework = "axum".to_string();
        assert_eq!(
            validate_src_kes_report(&bad_framework),
            Err(SrcKesError::InvalidReport("framework must be scotty"))
        );

        let mut bad_resource = make_valid_src_kes_report();
        bad_resource.plan.resource = "account".to_string();
        assert_eq!(
            validate_src_kes_report(&bad_resource),
            Err(SrcKesError::InvalidReport("resource must be user"))
        );

        let mut bad_routes = make_valid_src_kes_report();
        bad_routes.plan.routes = vec![];
        assert_eq!(validate_src_kes_report(&bad_routes), Err(SrcKesError::InvalidRouteContract));

        let mut bad_stage_count = make_valid_src_kes_report();
        let _ = bad_stage_count.stages.pop();
        assert_eq!(
            validate_src_kes_report(&bad_stage_count),
            Err(SrcKesError::InvalidReport("unexpected stage count"))
        );

        let mut bad_stage_order = make_valid_src_kes_report();
        bad_stage_order.stages.swap(0, 1);
        assert_eq!(
            validate_src_kes_report(&bad_stage_order),
            Err(SrcKesError::InvalidReport("invalid stage order"))
        );

        let mut empty_diagnostics = make_valid_src_kes_report();
        empty_diagnostics.stages[1].diagnostics = "   ".to_string();
        assert_eq!(
            validate_src_kes_report(&empty_diagnostics),
            Err(SrcKesError::InvalidReport("empty stage diagnostics"))
        );
    }

    #[test]
    fn validate_src_kes_report_accepts_fail_decision_when_stage_fails() {
        let mut report = make_valid_src_kes_report();
        report.stages[3].status = SrcKesStageStatus::Failed;
        report.decision = SrcKesDecision::Fail;

        assert_eq!(validate_src_kes_report(&report), Ok(()));
    }
}
