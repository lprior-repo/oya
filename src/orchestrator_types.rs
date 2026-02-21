use super::OyaError;
use oya::types::TimelineEntry;
use restate_sdk::prelude::*;
use serde::{Deserialize, Serialize};

// ---------------------------------------------------------------------------
// Public request/response types for OyaOpsMonitor handlers
// ---------------------------------------------------------------------------

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
/// Request body for polling OpenCode event stream snapshots.
pub struct OpsMonitorEventRequest {
    /// Maximum number of events to return in one poll.
    pub max_events: Option<usize>,
    /// Long-poll timeout in seconds for the event endpoint.
    pub timeout_seconds: Option<u64>,
}

#[derive(Debug, Serialize)]
/// Aggregated OpenCode status counters at one observation timestamp.
pub struct OpsMonitorPollResponse {
    pub source: String,
    pub observed_at: String,
    pub busy_sessions: Vec<String>,
    pub pending_permissions: usize,
    pub pending_questions: usize,
}

#[derive(Debug, Serialize)]
/// One raw OpenCode SSE event plus optional parsed JSON payload.
pub struct OpsMonitorEventEnvelope {
    pub raw: String,
    pub parsed: Option<serde_json::Value>,
}

#[derive(Debug, Serialize)]
/// Event polling response with bounded event payloads and timing metadata.
pub struct OpsMonitorEventResponse {
    pub source: String,
    pub observed_at: String,
    pub events: Vec<OpsMonitorEventEnvelope>,
    pub count: usize,
    pub timeout_seconds: u64,
}

// ---------------------------------------------------------------------------
// Internal orchestrator event types persisted into Restate state
// ---------------------------------------------------------------------------

#[derive(Debug, Serialize, Deserialize)]
pub(super) struct WorkspaceLifecycleEvent {
    pub workspace: String,
    pub workspace_path: String,
    pub queue_command: String,
    pub queue_passed: bool,
    pub queue_exit_code: i32,
    pub queue_output: String,
    pub add_command: String,
    pub add_passed: bool,
    pub add_exit_code: i32,
    pub add_output: String,
    pub recorded_at: String,
}

#[derive(Debug, Clone, Serialize)]
pub(super) struct OrchestratorState {
    pub status: String,
    pub stage: String,
    pub attempt: u32,
    pub bead_id: String,
    pub context: String,
    pub model: String,
    pub last_failure: String,
    pub last_output: String,
    pub last_prompt: String,
    pub updated_at: String,
}

/// Consolidated stage artifact containing all data for one stage attempt.
/// Replaces 8+ individual keys with a single rich payload.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub(super) struct StageArtifact {
    pub stage: String,
    pub attempt: u32,
    pub failure_category: Option<String>,
    pub next_stage: Option<String>,
    pub timing: StageTiming,
    pub workspace: Option<WorkspaceLifecycle>,
    pub input: StageInputData,
    pub prompt: String,
    pub output: StageOutputData,
    pub task_tracking: Option<TaskTracking>,
    pub gates: Vec<GateResultData>,
    pub status: StageStatus,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub(super) struct StageTiming {
    pub started_at: String,
    pub completed_at: String,
    pub duration_ms: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub(super) struct WorkspaceLifecycle {
    pub name: String,
    pub path: String,
    pub queue_command: String,
    pub queue_passed: bool,
    pub queue_exit_code: i32,
    pub add_command: String,
    pub add_passed: bool,
    pub add_exit_code: i32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub(super) struct StageInputData {
    pub run_id: String,
    pub bead_id: String,
    pub context: String,
    pub model: String,
    pub last_failure: Option<FailureSnapshot>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub(super) struct StageOutputData {
    pub success: bool,
    pub exit_code: i32,
    pub full_log: String,
    pub feedback: String,
    pub contract_document: Option<String>,
    pub implementation_code: Option<String>,
    pub test_results: Option<String>,
    pub adversarial_report: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub(super) struct TaskTracking {
    pub tasks_created: Vec<String>,
    pub tasks_updated: Vec<String>,
    pub tasks_completed: Vec<String>,
    pub task_states: std::collections::HashMap<String, TaskState>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub(super) struct TaskState {
    pub subject: String,
    pub status: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub(super) struct GateResultData {
    pub gate: String,
    pub passed: bool,
    pub exit_code: i32,
    pub command: String,
    pub output: String,
}

/// Stage status - mutually exclusive states making illegal states unrepresentable.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub(super) enum StageStatus {
    Completed,
    Failed,
}

#[derive(Debug, Serialize)]
pub(super) struct RunRequestEvent {
    pub run_id: String,
    pub bead_id: String,
    pub context: String,
    pub started_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub(super) struct FailureSnapshot {
    pub category: String,
    pub message: String,
}

// ---------------------------------------------------------------------------
// Start-request parsing
// ---------------------------------------------------------------------------

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct StartRequestPayload {
    pub bead_id: Option<String>,
    pub context: Option<String>,
    pub model: Option<String>,
}

/// Maximum allowed length for bead_id in API requests
const MAX_API_BEAD_ID_LEN: usize = 128;

/// Maximum allowed length for context in API requests
const MAX_API_CONTEXT_LEN: usize = 4096;

/// Maximum allowed length for model name in API requests
const MAX_API_MODEL_LEN: usize = 128;

/// Maximum allowed raw request size (to prevent DoS)
const MAX_RAW_REQUEST_LEN: usize = 32 * 1024; // 32KB

pub(super) fn parse_start_request(
    request: serde_json::Value,
) -> Result<StartRequestPayload, TerminalError> {
    // Check raw request size for strings
    if let serde_json::Value::String(raw) = &request {
        if raw.len() > MAX_RAW_REQUEST_LEN {
            return Err(TerminalError::new_with_code(
                413,
                format!("Request payload too large: {} > {} bytes", raw.len(), MAX_RAW_REQUEST_LEN),
            ));
        }
    }

    let payload: StartRequestPayload = match request {
        serde_json::Value::Object(_) => serde_json::from_value(request)
            .map_err(|e| TerminalError::new_with_code(400, format!("Invalid JSON body: {}", e)))?,
        serde_json::Value::String(raw) => serde_json::from_str::<StartRequestPayload>(&raw)
            .map_err(|e| {
                TerminalError::new_with_code(400, format!("Invalid JSON string body: {}", e))
            })?,
        other => Err(TerminalError::new_with_code(
            400,
            format!("Invalid request payload type: expected object or JSON string, got {}", other),
        ))?,
    };

    // Validate field sizes (src-1dr)
    validate_start_request_payload(&payload)?;
    Ok(payload)
}

fn validate_start_request_payload(payload: &StartRequestPayload) -> Result<(), TerminalError> {
    if let Some(bead_id) = &payload.bead_id {
        validate_api_bead_id(bead_id)?;
    }
    if let Some(context) = &payload.context {
        validate_api_context(context)?;
    }
    if let Some(model) = &payload.model {
        validate_api_model(model)?;
    }
    Ok(())
}

fn validate_api_bead_id(bead_id: &str) -> Result<(), TerminalError> {
    let trimmed = bead_id.trim();
    if trimmed.is_empty() {
        return Err(TerminalError::new_with_code(400, "bead_id cannot be empty".to_string()));
    }
    if trimmed.len() > MAX_API_BEAD_ID_LEN {
        return Err(TerminalError::new_with_code(
            413,
            format!("bead_id exceeds maximum length: {} > {}", trimmed.len(), MAX_API_BEAD_ID_LEN),
        ));
    }
    if contains_forbidden_control_chars(trimmed) {
        return Err(TerminalError::new_with_code(
            400,
            "bead_id contains invalid control characters".to_string(),
        ));
    }
    Ok(())
}

fn validate_api_context(context: &str) -> Result<(), TerminalError> {
    if context.len() > MAX_API_CONTEXT_LEN {
        return Err(TerminalError::new_with_code(
            413,
            format!("context exceeds maximum length: {} > {}", context.len(), MAX_API_CONTEXT_LEN),
        ));
    }
    if contains_forbidden_control_chars(context) {
        return Err(TerminalError::new_with_code(
            400,
            "context contains invalid control characters".to_string(),
        ));
    }
    Ok(())
}

fn validate_api_model(model: &str) -> Result<(), TerminalError> {
    let trimmed = model.trim();
    if trimmed.is_empty() {
        return Err(TerminalError::new_with_code(
            400,
            "model cannot be empty when provided".to_string(),
        ));
    }
    if trimmed.len() > MAX_API_MODEL_LEN {
        return Err(TerminalError::new_with_code(
            413,
            format!("model exceeds maximum length: {} > {}", trimmed.len(), MAX_API_MODEL_LEN),
        ));
    }
    if contains_forbidden_control_chars(trimmed) {
        return Err(TerminalError::new_with_code(
            400,
            "model contains invalid control characters".to_string(),
        ));
    }
    Ok(())
}

fn contains_forbidden_control_chars(value: &str) -> bool {
    value.chars().any(|char| char.is_control() && char != '\n' && char != '\r' && char != '\t')
}

// ---------------------------------------------------------------------------
// Restate state helpers
// ---------------------------------------------------------------------------

pub(super) fn to_json_string<T: Serialize>(value: &T) -> Result<String, OyaError> {
    serde_json::to_string(value).map_err(|error| OyaError(format!("json encode failed: {}", error)))
}

pub(super) fn set_json_state<T: Serialize>(
    ctx: &WorkflowContext<'_>,
    key: &str,
    value: &T,
) -> Result<(), OyaError> {
    let encoded = to_json_string(value)?;
    ctx.set(key, encoded);
    Ok(())
}

pub(super) fn write_orchestrator_state(
    ctx: &WorkflowContext<'_>,
    state: &OrchestratorState,
) -> Result<(), OyaError> {
    set_json_state(ctx, "state", state)
}

pub(super) async fn append_timeline(
    ctx: &WorkflowContext<'_>,
    entry: TimelineEntry,
) -> Result<(), OyaError> {
    let existing = ctx
        .get::<String>("timeline")
        .await
        .map_err(|error| OyaError(format!("timeline read failed: {}", error)))?;
    let existing = existing.unwrap_or_default();

    let event_seq = ctx
        .get::<u32>("event_seq")
        .await
        .map_err(|error| OyaError(format!("event_seq read failed: {}", error)))?
        .map_or(1, |value| value + 1);
    ctx.set("event_seq", event_seq);

    let event_key = format!("event_{:04}", event_seq);
    set_json_state(ctx, &event_key, &entry)?;

    let line = to_json_string(&entry)?;
    let next = if existing.is_empty() { line } else { format!("{}\n{}", existing, line) };

    ctx.set("timeline", next);
    Ok(())
}

/// Persist a single consolidated stage artifact.
pub(super) fn set_stage_artifact(
    ctx: &WorkflowContext<'_>,
    key: &str,
    artifact: &StageArtifact,
) -> Result<(), OyaError> {
    set_json_state(ctx, key, artifact)
}

/// Set lean timeline as a single JSON array instead of incremental appends.
pub(super) fn set_timeline_once(ctx: &WorkflowContext<'_>, timeline: &str) -> Result<(), OyaError> {
    ctx.set("timeline", timeline.to_string());
    Ok(())
}

// ---------------------------------------------------------------------------
// Tests for API input validation (src-1dr)
// ---------------------------------------------------------------------------

#[cfg(test)]
mod validation_tests {
    use super::*;

    #[test]
    fn test_parse_start_request_accepts_valid_payload() {
        let json = serde_json::json!({
            "bead_id": "src-abc123",
            "context": "test context",
            "model": "claude-3-opus"
        });
        let result = parse_start_request(json);
        assert!(result.is_ok());
        let payload = result.unwrap();
        assert_eq!(payload.bead_id, Some("src-abc123".to_string()));
    }

    #[test]
    fn test_parse_start_request_accepts_minimal_payload() {
        let json = serde_json::json!({});
        let result = parse_start_request(json);
        assert!(result.is_ok());
    }

    #[test]
    fn test_parse_start_request_rejects_oversized_bead_id() {
        let oversized = "x".repeat(129);
        let json = serde_json::json!({
            "bead_id": oversized
        });
        let result = parse_start_request(json);
        assert!(result.is_err());
        let error = result.err();
        assert!(error.is_some());
        let terminal_error = error.unwrap();
        assert_eq!(terminal_error.code(), 413);
        assert!(terminal_error.message().contains("exceeds maximum length"));
    }

    #[test]
    fn test_parse_start_request_rejects_oversized_context() {
        let oversized = "x".repeat(4097);
        let json = serde_json::json!({
            "context": oversized
        });
        let result = parse_start_request(json);
        assert!(result.is_err());
        let error = result.err();
        assert!(error.is_some());
        let terminal_error = error.unwrap();
        assert_eq!(terminal_error.code(), 413);
        assert!(terminal_error.message().contains("exceeds maximum length"));
    }

    #[test]
    fn test_parse_start_request_rejects_oversized_model() {
        let oversized = "x".repeat(129);
        let json = serde_json::json!({
            "model": oversized
        });
        let result = parse_start_request(json);
        assert!(result.is_err());
        let error = result.err();
        assert!(error.is_some());
        let terminal_error = error.unwrap();
        assert_eq!(terminal_error.code(), 413);
        assert!(terminal_error.message().contains("exceeds maximum length"));
    }

    #[test]
    fn test_parse_start_request_rejects_bead_id_control_chars() {
        let json = serde_json::json!({
            "bead_id": "test\x00bead"
        });
        let result = parse_start_request(json);
        assert!(result.is_err());
        let error = result.err();
        assert!(error.is_some());
        let terminal_error = error.unwrap();
        assert_eq!(terminal_error.code(), 400);
        assert!(terminal_error.message().contains("invalid control characters"));
    }

    #[test]
    fn test_parse_start_request_rejects_context_control_chars() {
        let json = serde_json::json!({
            "context": "test\x1bcontext"
        });
        let result = parse_start_request(json);
        assert!(result.is_err());
        let error = result.err();
        assert!(error.is_some());
        let terminal_error = error.unwrap();
        assert_eq!(terminal_error.code(), 400);
        assert!(terminal_error.message().contains("invalid control characters"));
    }

    #[test]
    fn test_parse_start_request_accepts_context_with_newlines() {
        let json = serde_json::json!({
            "context": "line1\nline2\ttab\r\nwindows"
        });
        let result = parse_start_request(json);
        assert!(result.is_ok());
    }

    #[test]
    fn test_parse_start_request_rejects_oversized_raw_string() {
        let oversized = "x".repeat(33 * 1024); // 33KB, over the 32KB limit
        let json = serde_json::Value::String(oversized);
        let result = parse_start_request(json);
        assert!(result.is_err());
        let error = result.err();
        assert!(error.is_some());
        let terminal_error = error.unwrap();
        assert_eq!(terminal_error.code(), 413);
        assert!(terminal_error.message().contains("Request payload too large"));
    }

    #[test]
    fn test_parse_start_request_accepts_at_max_bead_id_length() {
        let max_len = "x".repeat(128);
        let json = serde_json::json!({
            "bead_id": max_len
        });
        let result = parse_start_request(json);
        assert!(result.is_ok());
    }

    #[test]
    fn test_parse_start_request_accepts_at_max_context_length() {
        let max_len = "x".repeat(4096);
        let json = serde_json::json!({
            "context": max_len
        });
        let result = parse_start_request(json);
        assert!(result.is_ok());
    }

    #[test]
    fn test_parse_start_request_rejects_empty_bead_id() {
        let json = serde_json::json!({
            "bead_id": ""
        });
        let result = parse_start_request(json);
        assert!(result.is_err());
        let error = result.err();
        assert!(error.is_some());
        let terminal_error = error.unwrap();
        assert_eq!(terminal_error.code(), 400);
        assert!(terminal_error.message().contains("cannot be empty"));
    }

    #[test]
    fn test_parse_start_request_rejects_empty_model() {
        let json = serde_json::json!({
            "model": ""
        });
        let result = parse_start_request(json);
        assert!(result.is_err());
        let error = result.err();
        assert!(error.is_some());
        let terminal_error = error.unwrap();
        assert_eq!(terminal_error.code(), 400);
        assert!(terminal_error.message().contains("cannot be empty"));
    }

    // ---------------------------------------------------------------------------
    // Strict JSON schema tests: unknown field rejection (src-17y)
    // ---------------------------------------------------------------------------

    #[test]
    fn test_start_request_rejects_unknown_field_typo() {
        // Test that a typo in field name is rejected (not silently ignored)
        let json = serde_json::json!({
            "bead_idx": "src-abc123"  // typo: bead_idx instead of bead_id
        });
        let result = parse_start_request(json);
        assert!(result.is_err(), "unknown field 'bead_idx' should be rejected");
        let error = result.err();
        assert!(error.is_some());
        let terminal_error = error.unwrap();
        assert_eq!(terminal_error.code(), 400);
        let message = terminal_error.message();
        assert!(
            message.contains("unknown field") || message.contains("unexpected"),
            "error message should mention unknown/unexpected field, got: {}",
            message
        );
    }

    #[test]
    fn test_start_request_rejects_unknown_field_extra() {
        // Test that extra unknown fields are rejected
        let json = serde_json::json!({
            "bead_id": "src-abc123",
            "context": "test",
            "extra_field": "should be rejected"
        });
        let result = parse_start_request(json);
        assert!(result.is_err(), "unknown field 'extra_field' should be rejected");
        let error = result.err();
        assert!(error.is_some());
        let terminal_error = error.unwrap();
        assert_eq!(terminal_error.code(), 400);
        let message = terminal_error.message();
        assert!(
            message.contains("unknown field") || message.contains("unexpected"),
            "error message should mention unknown/unexpected field, got: {}",
            message
        );
    }

    #[test]
    fn test_start_request_rejects_unknown_field_mixed() {
        // Test that unknown fields are rejected even with valid fields present
        let json = serde_json::json!({
            "bead_id": "src-abc123",
            "context": "valid context",
            "model": "claude-3-opus",
            "typo_modl": "should fail"  // typo: modl instead of model
        });
        let result = parse_start_request(json);
        assert!(result.is_err(), "unknown field 'typo_modl' should be rejected");
        let error = result.err();
        assert!(error.is_some());
        let terminal_error = error.unwrap();
        assert_eq!(terminal_error.code(), 400);
    }

    #[test]
    fn test_ops_monitor_event_request_rejects_unknown_field() {
        // Test that OpsMonitorEventRequest rejects unknown fields
        let json = serde_json::json!({
            "max_events": 10,
            "unknown_option": 123
        });
        let result: Result<OpsMonitorEventRequest, _> = serde_json::from_value(json);
        assert!(result.is_err(), "unknown field 'unknown_option' should be rejected");
        let error_msg = result.err().map(|e| e.to_string()).unwrap_or_default();
        assert!(
            error_msg.contains("unknown field") || error_msg.contains("unexpected"),
            "error message should mention unknown/unexpected field, got: {}",
            error_msg
        );
    }
}
