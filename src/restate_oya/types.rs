#![deny(clippy::unwrap_used)]
#![deny(clippy::expect_used)]
#![deny(clippy::panic)]
use crate::lifecycle::types::{BeadId, BeadStatus, CancelState};
pub use oya_contracts::{LifecycleGateSnapshot, LifecycleStatusSnapshot, LifecycleStepSnapshot};
use serde::{Deserialize, Serialize};
use serde_json::Value;

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct StartRequest {
    pub prompt: String,
    pub model: Option<String>,
    pub bead_id: Option<String>,
    pub bead_status: Option<String>,
    pub bead_state: Option<Value>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct BeadSyncRequest {
    pub bead_id: String,
    pub bead_status: String,
    pub bead_state: Value,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct PipelineRequest {
    pub model: Option<String>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct LifecycleRequest {
    pub bead_id: Option<String>,
    pub model: Option<String>,
    pub repo: Option<String>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct KeyRequest {
    pub key: String,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct BeadSnapshot {
    pub bead_id: Option<BeadId>,
    pub bead_status: Option<BeadStatus>,
    pub bead_state: Option<Value>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct MemorySnapshot {
    pub bead: BeadSnapshot,
    pub last_output_summary: Option<Value>,
    pub last_output_trace: Option<Value>,
    pub active_invocation_id: Option<String>,
    pub cancel_state: CancelState,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct CancelResponse {
    pub cancelled: bool,
    pub message: String,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct StartResponse {
    pub output: String,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct OpenCodeTraceEvent {
    pub sequence: u64,
    pub received_at: String,
    pub kind: String,
    pub step: Option<u64>,
    pub tool: Option<String>,
    pub description: Option<String>,
    pub command: Option<String>,
    pub query: Option<String>,
    pub text: Option<String>,
    pub error: Option<String>,
    pub raw: Value,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct OpenCodeTraceSnapshot {
    pub bead_id: Option<String>,
    pub workflow_key: String,
    pub active_invocation_id: Option<String>,
    pub model: Option<String>,
    pub started_at: Option<String>,
    pub updated_at: Option<String>,
    pub finished_at: Option<String>,
    pub status: String,
    pub current_event: Option<OpenCodeTraceEvent>,
    pub events: Vec<OpenCodeTraceEvent>,
    pub tool_call_count: u64,
    pub text_event_count: u64,
    pub last_error: Option<String>,
    pub summary: Option<Value>,
}
