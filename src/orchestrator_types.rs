use super::OyaError;
use oya::types::TimelineEntry;
use restate_sdk::prelude::*;
use serde::{Deserialize, Serialize};

// ---------------------------------------------------------------------------
// Public request/response types for OyaOpsMonitor handlers
// ---------------------------------------------------------------------------

#[derive(Debug, Deserialize)]
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

#[derive(Debug, Serialize)]
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
#[derive(Debug, Clone, Serialize)]
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

#[derive(Debug, Clone, Serialize)]
pub(super) struct StageTiming {
    pub started_at: String,
    pub completed_at: String,
    pub duration_ms: u64,
}

#[derive(Debug, Clone, Serialize)]
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

#[derive(Debug, Clone, Serialize)]
pub(super) struct StageInputData {
    pub run_id: String,
    pub bead_id: String,
    pub context: String,
    pub model: String,
    pub last_failure: Option<FailureSnapshot>,
}

#[derive(Debug, Clone, Serialize)]
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

#[derive(Debug, Clone, Serialize)]
pub(super) struct TaskTracking {
    pub tasks_created: Vec<String>,
    pub tasks_updated: Vec<String>,
    pub tasks_completed: Vec<String>,
    pub task_states: std::collections::HashMap<String, TaskState>,
}

#[derive(Debug, Clone, Serialize)]
pub(super) struct TaskState {
    pub subject: String,
    pub status: String,
}

#[derive(Debug, Clone, Serialize)]
pub(super) struct GateResultData {
    pub gate: String,
    pub passed: bool,
    pub exit_code: i32,
    pub command: String,
    pub output: String,
}

/// Stage status - mutually exclusive states making illegal states unrepresentable.
#[derive(Debug, Clone, Serialize)]
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

#[derive(Debug, Clone, Serialize)]
pub(super) struct FailureSnapshot {
    pub category: String,
    pub message: String,
}

// ---------------------------------------------------------------------------
// Start-request parsing
// ---------------------------------------------------------------------------

#[derive(Debug, Deserialize)]
pub(super) struct StartRequestPayload {
    pub bead_id: Option<String>,
    pub context: Option<String>,
    pub model: Option<String>,
}

pub(super) fn parse_start_request(
    request: serde_json::Value,
) -> Result<StartRequestPayload, TerminalError> {
    match request {
        serde_json::Value::Object(_) => serde_json::from_value(request)
            .map_err(|e| TerminalError::new_with_code(400, format!("Invalid JSON body: {}", e))),
        serde_json::Value::String(raw) => serde_json::from_str::<StartRequestPayload>(&raw)
            .map_err(|e| {
                TerminalError::new_with_code(400, format!("Invalid JSON string body: {}", e))
            }),
        other => Err(TerminalError::new_with_code(
            400,
            format!("Invalid request payload type: expected object or JSON string, got {}", other),
        )),
    }
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
