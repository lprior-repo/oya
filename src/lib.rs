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
//! - Define a stable final-stage trace contract for planning, trace collection,
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
//! - Maintain Restate's determinism guarantee by separating non-stable operations
//!   from stable journaling in the workflow execution context.
//! - Ensure workflow state consistency across executions and replays by following the
//!   correct execution pattern for blocking operations.
//!
//! ## Key functions to implement
//! - `execute_stage_real(ctx: &WorkflowContext<'_>, request: StageExecutionRequest, merge_queue_policy: MergeQueuePolicy, repo_root: PathBuf) -> Result<(StageResult, String), OyaError>`
//!   - Fixed implementation with `spawn_blocking` outside `ctx.run()`
//! - `execute_stage_blocking(input: StageBlockingInput) -> Result<StageExecution, OyaError>`
//!   - Existing synchronous blocking execution (no changes needed)
//! - `test_execute_stage_real_stable_replay()`
//!   - New test to verify spawn_blocking is not called on replay
//!
//! ## Acceptance criteria
//! - `spawn_blocking` is called OUTSIDE of `ctx.run()` (non-stable part)
//! - Only result mapping is inside `ctx.run()` (stable journaling)
//! - Error handling properly separates OyaError (outer) from HandlerError (inner)
//! - Test added that verifies spawn_blocking is not called on replay
//! - Documentation explains the determinism pattern with proper doc comments
//! - `moon run :clippy` passes with no unwrap/expect/panic violations
//! - `moon run :test` passes with all tests green
//! - Code review confirms no other functions have this anti-pattern
//!
//! # Design Contract: `src-23s`
//!
//! ## Purpose and goals
//! - Verify that zjj workspace isolation is properly disabled by default
//! - Ensure zjj commands execute in the current working directory without creating
//!   isolated workspaces when not explicitly requested
//! - Validate that the default behavior provides direct command execution
//!   for backward compatibility and simple use cases
//!
//! ## Key functions to implement
//! - `verify_zjj_default_disabled() -> Result<(), ZjjVerificationError>`
//!   - Checks that zjj operates in current directory by default
//! - `test_zjj_no_workspace_creation()`
//!   - Test to verify no workspace directories are created implicitly
//! - `test_zjj_commands_in_current_dir()`
//!   - Test to verify commands execute in the current working directory
//! - `validate_zjj_default_config() -> Result<(), ZjjVerificationError>`
//!   - Validates that default configuration has workspace isolation disabled
//!
//! ## Acceptance criteria
//! - zjj commands execute in current directory by default without workspace creation
//! - No implicit workspace directories are created when using basic zjj commands
//! - Default configuration explicitly disables workspace isolation
//! - Commands like `zjj status`, `zjj list`, `zjj help` work in current directory
//! - Tests verify both the absence of workspace creation and correct command behavior
//! - Error handling works correctly when workspace isolation is not enabled
//! - Documentation clearly states the default behavior and how to enable workspaces
//!
//! # Design Contract: `src-1k3.1`
//!
//! ## Purpose and goals
//! - Remediate retry-exhausted scenarios caused by opencode plugin module resolution failures
//! - Ensure `Cannot find module '@opencode-ai/plugin'` errors are classified as ProviderUnavailable
//! - Enable proper recovery path when node_modules resolution fails in opencode cache
//!
//! ## Key functions to implement
//! - `classify_opencode_plugin_error(stderr: &str) -> Option<FailureCategory>`
//!   - Detects plugin resolution failures and maps to ProviderUnavailable
//! - `detect_opencode_module_resolution_failure(stderr: &str) -> bool`
//!   - Identifies ResolveMessage patterns with @opencode-ai/plugin references
//! - `remediate_plugin_unavailable(error: &OyaError) -> Result<RemediationAction, OyaError>`
//!   - Returns appropriate recovery action for provider unavailability
//!
//! ## Acceptance criteria
//! - `Cannot find module '@opencode-ai/plugin'` classified as ProviderUnavailable (not RateLimited)
//! - ResolveMessage patterns correctly parsed and matched
//! - Non-retryable failure category triggers immediate remediation path
//! - Retry-exhausted beads spawn remediation children with correct failure context
//! - Plugin resolution failures do not trigger infinite retry loops
//!
//! # Design Contract: `src-1k3.1 src-1ml.1 src-1oy.1 src-23s.1 src-23s.2 src-23s.3`
//!
//! ## Purpose and goals
//! - Define deterministic, auditable contracts for opencode failure classification,
//!   polling/parse hygiene, and zjj default workspace behavior.
//! - Ensure provider/module-resolution failures are classified correctly and routed to
//!   non-retry remediation paths.
//! - Guarantee safe parsing and validation boundaries so malformed or oversized input
//!   cannot silently pass into orchestration decisions.
//! - Preserve default zjj execution in the current directory unless explicit workspace
//!   isolation is requested.
//!
//! ## Key functions to implement
//! - `classify_opencode_error(stderr: &str) -> Option<FailureCategory>`
//! - `parse_opencode_output(raw: &str) -> Result<OpencodeRunOutput, OpencodeParseError>`
//! - `parse_opencode_sse_events(raw_chunk: &str, max_events: usize) -> Result<Vec<String>, OpsMonitorError>`
//! - `build_opencode_poll_snapshot(session_status_json: &str, permission_json: &str, question_json: &str) -> Result<OpencodePollSnapshot, OpsMonitorError>`
//! - `build_zjj_workspace_name(run_id: &str, stage: &str, attempt: u32) -> Result<String, OpsMonitorError>`
//! - `is_retryable_failure(category: &FailureCategory) -> bool`
//!
//! ## Acceptance criteria
//! - Plugin/module-resolution errors containing `@opencode-ai/plugin` classify as
//!   `FailureCategory::ProviderUnavailable` and never as retryable test/lint categories.
//! - Poll/snapshot parsing rejects invalid JSON, invalid shapes, forbidden control characters,
//!   and over-limit payloads with explicit error variants.
//! - SSE parsing normalizes line endings, extracts only `data:` payloads, enforces payload
//!   size limits, and returns events in deterministic source order.
//! - `parse_opencode_output` supports both structured JSON and SSE/text-event extraction,
//!   while enforcing stdout type/length/content validation.
//! - Workspace naming rejects empty/invalid segments and zero attempts, producing stable,
//!   normalized names within configured length bounds.
//! - Retryability decisions remain limited to code-fixable failures; provider and rate-limit
//!   conditions remain non-retryable orchestration signals.
//!
//! # Design Contract: `src-1k3.1` (contract)
//!
//! ## Purpose and goals
//! - Establish deterministic classification for opencode plugin module-resolution failures.
//! - Route provider-unavailable failures to remediation instead of retry loops.
//! - Preserve stable orchestration behavior across repeated runs.
//!
//! ## Key functions to implement
//! - `classify_opencode_error(stderr: &str) -> Option<FailureCategory>`
//! - `is_retryable_failure(category: &FailureCategory) -> bool`
//! - `remediate_retry_exhausted_failure(failure: &StageFailure) -> Result<RemediationPlan, OyaError>`
//!
//! ## Acceptance criteria
//! - Errors containing `Cannot find module '@opencode-ai/plugin'` classify as `FailureCategory::ProviderUnavailable`.
//! - `FailureCategory::ProviderUnavailable` is non-retryable in retryability decisions.
//! - Retry-exhausted handling emits remediation plans with preserved run/bead failure context.
//!
//! # Design Contract: `src-1gw`
//!
//! ## Purpose and goals
//! - Define a deterministic contract-validation stage that turns raw contract input into a validated decision artifact.
//! - Ensure invalid or unsafe contract payloads are rejected at boundaries with explicit, auditable errors.
//! - Preserve stable outcomes so equivalent inputs always produce equivalent validation results.
//!
//! ## Key functions to implement
//! - `build_contract_validation_plan(input: &ContractValidationInput) -> Result<ContractValidationPlan, ContractValidationError>`
//! - `collect_contract_validation_observation(plan: &ContractValidationPlan) -> Result<ContractValidationObservation, ContractValidationError>`
//! - `evaluate_contract_validation_report(observation: &ContractValidationObservation) -> Result<ContractValidationReport, ContractValidationError>`
//! - `derive_contract_validation_decision(report: &ContractValidationReport) -> ContractValidationDecision`
//! - `validate_contract_validation_report(report: &ContractValidationReport) -> Result<(), ContractValidationError>`
//!
//! ## Acceptance criteria
//! - Planning rejects empty required fields, over-limit payloads, and forbidden control characters.
//! - Observation collection records ordered checks with non-empty diagnostics and valid timestamps.
//! - Report evaluation preserves canonical stage order and enforces monotonic timestamps.
//! - Final decision is derived exclusively from report outcomes and matches report validation.
//! - Re-running with equivalent inputs yields equivalent report structure and decision outputs.
//!
//! # Design Contract: `src-2ey`
//!
//! ## Purpose and goals
//! - Pin moon CI evidence to an exact git revision at collection time to prevent stale evidence
//!   from bypassing land checks.
//! - Ensure the ship gate validates that moon evidence revision matches current HEAD before
//!   allowing merge operations.
//! - Detect and reject revision mismatches with explicit, auditable error messages.
//!
//! ## Key functions to implement
//! - `collect_moon_evidence_with_revision(repo_root: &Path) -> Result<MoonEvidence, ShipGateError>`
//!   - Captures moon output AND current git HEAD revision atomically
//! - `validate_moon_evidence_revision(evidence: &MoonEvidence, current_head: &str) -> Result<(), ShipGateError>`
//!   - Compares pinned revision against current HEAD, rejects on mismatch
//! - `pin_evidence_revision(evidence: &mut MoonEvidence, revision: &str) -> Result<(), ShipGateError>`
//!   - Sets the revision field on evidence with format validation
//!
//! ## Acceptance criteria
//! - Moon evidence includes mandatory `revision` field containing full 40-char git SHA
//! - Revision is captured atomically with moon execution (not before/after separately)
//! - Ship gate rejects evidence where `evidence.revision != git rev-parse HEAD`
//! - Revision mismatch returns `ShipGateError::StaleEvidence` with both revisions in message
//! - Empty or malformed revision fields are rejected at collection time
//! - Tests verify stale evidence rejection with mocked revision mismatch scenarios
//! - `moon run :ci` passes with no clippy warnings

pub mod beads;
pub mod config;
pub mod orchestrator;
pub mod quality_gate;
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
/// Note: RateLimited is non-retryable because it triggers model rotation instead of stage retry
pub fn is_retryable_failure(category: &FailureCategory) -> bool {
    matches!(
        category,
        FailureCategory::TestFailed
            | FailureCategory::TestsUnexpectedlyGreen
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
        || has_opencode_plugin_resolution_failure(stderr)
    {
        return Some(FailureCategory::ProviderUnavailable);
    }

    None
}

fn has_opencode_plugin_resolution_failure(stderr: &str) -> bool {
    let normalized = stderr.to_ascii_lowercase();
    let normalized_no_escape = normalized.replace('\\', "");
    normalized_no_escape.contains("cannot find module")
        && (normalized_no_escape.contains("@opencode-ai/plugin")
            || normalized_no_escape.contains("opencode-gemini-auth")
            || normalized_no_escape.contains("resolvemessage: cannot find module"))
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
    if let Ok(value) = serde_json::from_str::<serde_json::Value>(trimmed) {
        return match value.get("stdout") {
            Some(stdout) => parse_opencode_output_stdout(stdout).map(Some),
            None => Err(OpencodeParseError::new("opencode json missing stdout field")),
        };
    }

    match find_stdout_field_in_json_lines(trimmed) {
        Some(stdout) => parse_opencode_output_stdout(&stdout).map(Some),
        None => Ok(None),
    }
}

fn find_stdout_field_in_json_lines(raw: &str) -> Option<serde_json::Value> {
    raw.lines().find_map(find_stdout_field_in_line)
}

fn find_stdout_field_in_line(line: &str) -> Option<serde_json::Value> {
    let trimmed = line.trim();
    if let Ok(value) = serde_json::from_str::<serde_json::Value>(trimmed) {
        return value.get("stdout").cloned();
    }

    trimmed.find('{').and_then(|start| {
        serde_json::from_str::<serde_json::Value>(&trimmed[start..])
            .ok()
            .and_then(|value| value.get("stdout").cloned())
    })
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
    #[error("manual e2e field has invalid format: {0}")]
    InvalidFieldFormat(&'static str),
    #[error("manual e2e report invalid: {0}")]
    InvalidReport(&'static str),
    #[error("manual e2e command invalid: {0}")]
    InvalidCommand(&'static str),
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
    validate_manual_e2e_command(command)?;

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

pub fn run_e2e_validation_pipeline(
    plan: &ManualE2ePlan,
) -> Result<ManualE2eReport, ManualE2eError> {
    validate_e2e_validation_command(plan.command.as_str())?;
    run_manual_e2e_pipeline(plan)
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

fn validate_e2e_validation_command(command: &str) -> Result<(), ManualE2eError> {
    let command_tokens = split_shell_command_tokens(command);
    if contains_exact_command(&command_tokens, "oya", "run") {
        return Err(ManualE2eError::InvalidCommand("oya_run_not_allowed"));
    }

    if !contains_moon_task(&command_tokens, ":test") {
        return Err(ManualE2eError::InvalidCommand("missing_moon_test"));
    }
    if !contains_moon_task(&command_tokens, ":ci") {
        return Err(ManualE2eError::InvalidCommand("missing_moon_ci"));
    }

    Ok(())
}

fn split_shell_command_tokens(command: &str) -> Vec<Vec<String>> {
    split_shell_segments(command)
        .into_iter()
        .map(|segment| shell_tokenize(segment.as_str()))
        .filter(|tokens| !tokens.is_empty())
        .collect()
}

fn split_shell_segments(command: &str) -> Vec<String> {
    let mut segments = Vec::new();
    let mut current = String::new();
    let mut quote: Option<char> = None;
    let mut chars = command.chars().peekable();

    while let Some(ch) = chars.next() {
        if quote.is_none() && (ch == ';' || ch == '&' || ch == '|') {
            if (ch == '&' || ch == '|') && chars.peek().copied() == Some(ch) {
                let _ = chars.next();
            }
            push_shell_segment(&mut segments, &mut current);
            continue;
        }
        if ch == '\'' || ch == '"' {
            quote = if quote == Some(ch) {
                None
            } else if quote.is_none() {
                Some(ch)
            } else {
                quote
            };
        }
        current.push(ch);
    }

    push_shell_segment(&mut segments, &mut current);
    segments
}

fn push_shell_segment(segments: &mut Vec<String>, current: &mut String) {
    let trimmed = current.trim();
    if !trimmed.is_empty() {
        segments.push(trimmed.to_string());
    }
    current.clear();
}

fn shell_tokenize(segment: &str) -> Vec<String> {
    let mut tokens = Vec::new();
    let mut current = String::new();
    let mut quote: Option<char> = None;
    let mut escaped = false;

    for ch in segment.chars() {
        if escaped {
            current.push(ch);
            escaped = false;
            continue;
        }
        if ch == '\\' {
            escaped = true;
            continue;
        }
        if ch == '\'' || ch == '"' {
            quote = if quote == Some(ch) {
                None
            } else if quote.is_none() {
                Some(ch)
            } else {
                quote
            };
            continue;
        }
        if quote.is_none() && ch.is_whitespace() {
            push_shell_token(&mut tokens, &mut current);
            continue;
        }
        current.push(ch);
    }

    push_shell_token(&mut tokens, &mut current);
    tokens
}

fn push_shell_token(tokens: &mut Vec<String>, current: &mut String) {
    if !current.is_empty() {
        tokens.push(std::mem::take(current));
    }
}

fn contains_exact_command(command_tokens: &[Vec<String>], program: &str, subcommand: &str) -> bool {
    command_tokens.iter().any(|tokens| {
        tokens.first().is_some_and(|token| token == program)
            && tokens.get(1).is_some_and(|token| token == subcommand)
    })
}

fn contains_moon_task(command_tokens: &[Vec<String>], task: &str) -> bool {
    command_tokens.iter().any(|tokens| {
        tokens.first().is_some_and(|token| token == "moon")
            && tokens.get(1).is_some_and(|token| token == "run")
            && tokens.get(2).is_some_and(|token| token == task)
    })
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

fn validate_manual_e2e_command(command: &str) -> Result<(), ManualE2eError> {
    let tokens = command.split_whitespace().map(str::to_ascii_lowercase).collect::<Vec<_>>();
    if !is_allowed_manual_e2e_command(tokens.as_slice()) {
        return Err(ManualE2eError::InvalidFieldFormat("command"));
    }

    Ok(())
}

fn is_allowed_manual_e2e_command(tokens: &[String]) -> bool {
    matches!(tokens, [moon, run, task] if moon == "moon" && run == "run" && is_allowed_manual_e2e_task(task.as_str()))
}

fn is_allowed_manual_e2e_task(task: &str) -> bool {
    task == ":test" || task == ":ci"
}

const MAX_MERGE_TRAIN_PRIORITY: u8 = 4;
type MergeTrainPriorities = std::collections::HashMap<String, u8>;
type MergeTrainIndegree = std::collections::HashMap<String, usize>;
type MergeTrainDependents = std::collections::HashMap<String, Vec<String>>;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MergeTrainCandidate {
    pub bead_id: String,
    pub priority: u8,
    pub depends_on: Vec<String>,
}

#[derive(Debug, Error, Clone, PartialEq, Eq)]
pub enum MergeTrainError {
    #[error("merge train field is empty: {0}")]
    EmptyField(&'static str),
    #[error("merge train priority out of range: {0}")]
    InvalidPriority(u8),
    #[error("merge train duplicate bead id: {0}")]
    DuplicateBeadId(String),
    #[error("merge train unknown dependency: bead={bead_id} dependency={dependency}")]
    UnknownDependency { bead_id: String, dependency: String },
    #[error("merge train dependency cycle detected")]
    DependencyCycle,
}

pub fn schedule_dependency_aware_priority_processing(
    candidates: &[MergeTrainCandidate],
) -> Result<Vec<String>, MergeTrainError> {
    let priorities = validate_and_collect_priorities(candidates)?;
    let (mut indegree, dependents) = build_merge_train_graph(candidates, &priorities)?;
    let mut ready = initial_ready_queue(candidates, &indegree);
    let mut schedule = Vec::with_capacity(candidates.len());

    while let Some(next) = pop_next_ready(&mut ready, &priorities) {
        schedule.push(next.clone());
        if let Some(deps) = dependents.get(&next) {
            for dependent in deps {
                decrement_indegree_and_queue(dependent, &mut indegree, &mut ready)?;
            }
        }
    }

    if schedule.len() == candidates.len() {
        Ok(schedule)
    } else {
        Err(MergeTrainError::DependencyCycle)
    }
}

fn validate_and_collect_priorities(
    candidates: &[MergeTrainCandidate],
) -> Result<MergeTrainPriorities, MergeTrainError> {
    let mut priorities = std::collections::HashMap::new();
    for candidate in candidates {
        if candidate.bead_id.trim().is_empty() {
            return Err(MergeTrainError::EmptyField("bead_id"));
        }
        if candidate.priority > MAX_MERGE_TRAIN_PRIORITY {
            return Err(MergeTrainError::InvalidPriority(candidate.priority));
        }
        if priorities.insert(candidate.bead_id.clone(), candidate.priority).is_some() {
            return Err(MergeTrainError::DuplicateBeadId(candidate.bead_id.clone()));
        }
    }
    Ok(priorities)
}

fn build_merge_train_graph(
    candidates: &[MergeTrainCandidate],
    priorities: &MergeTrainPriorities,
) -> Result<(MergeTrainIndegree, MergeTrainDependents), MergeTrainError> {
    let mut indegree = priorities.keys().map(|id| (id.clone(), 0_usize)).collect::<_>();
    let mut dependents = std::collections::HashMap::<String, Vec<String>>::new();

    for candidate in candidates {
        for dependency in &candidate.depends_on {
            if !priorities.contains_key(dependency) {
                return Err(MergeTrainError::UnknownDependency {
                    bead_id: candidate.bead_id.clone(),
                    dependency: dependency.clone(),
                });
            }
            increment_indegree(candidate.bead_id.as_str(), &mut indegree)?;
            dependents.entry(dependency.clone()).or_default().push(candidate.bead_id.clone());
        }
    }

    Ok((indegree, dependents))
}

fn initial_ready_queue(
    candidates: &[MergeTrainCandidate],
    indegree: &MergeTrainIndegree,
) -> Vec<String> {
    candidates
        .iter()
        .filter(|candidate| indegree.get(&candidate.bead_id).copied().unwrap_or(0) == 0)
        .map(|candidate| candidate.bead_id.clone())
        .collect::<std::collections::BTreeSet<_>>()
        .into_iter()
        .collect()
}

fn pop_next_ready(ready: &mut Vec<String>, priorities: &MergeTrainPriorities) -> Option<String> {
    ready.sort_by(|left, right| {
        let left_priority = priorities.get(left).copied().unwrap_or(MAX_MERGE_TRAIN_PRIORITY);
        let right_priority = priorities.get(right).copied().unwrap_or(MAX_MERGE_TRAIN_PRIORITY);
        left_priority.cmp(&right_priority).then_with(|| left.cmp(right))
    });
    if ready.is_empty() {
        None
    } else {
        Some(ready.remove(0))
    }
}

fn increment_indegree(
    bead_id: &str,
    indegree: &mut MergeTrainIndegree,
) -> Result<(), MergeTrainError> {
    if let Some(value) = indegree.get_mut(bead_id) {
        *value = value.saturating_add(1);
        Ok(())
    } else {
        Err(MergeTrainError::UnknownDependency {
            bead_id: bead_id.to_string(),
            dependency: bead_id.to_string(),
        })
    }
}

fn decrement_indegree_and_queue(
    bead_id: &str,
    indegree: &mut MergeTrainIndegree,
    ready: &mut Vec<String>,
) -> Result<(), MergeTrainError> {
    if let Some(value) = indegree.get_mut(bead_id) {
        if *value == 0 {
            return Err(MergeTrainError::DependencyCycle);
        }
        *value -= 1;
        if *value == 0 {
            ready.push(bead_id.to_string());
        }
        Ok(())
    } else {
        Err(MergeTrainError::UnknownDependency {
            bead_id: bead_id.to_string(),
            dependency: bead_id.to_string(),
        })
    }
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
