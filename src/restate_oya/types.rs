use crate::lifecycle::types::{BeadId, BeadStatus, CancelState};
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
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct LifecycleStepSnapshot {
    pub step: String,
    pub status: String,
    pub message: Option<String>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct LifecycleStatusSnapshot {
    pub bead_id: Option<String>,
    pub steps: Vec<LifecycleStepSnapshot>,
    pub state: Option<Value>,
    pub pr_url: Option<String>,
    pub done: bool,
    pub success: Option<bool>,
    pub message: Option<String>,
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
