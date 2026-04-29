//! Execution record types.

use chrono::DateTime;
use chrono::Utc;
use serde::{Deserialize, Serialize};
use std::fmt;
use uuid::Uuid;

use crate::graph::{ExecutionState, NodeId};

// ===========================================================================
// Error Types
// ===========================================================================

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(try_from = "String", into = "String")]
pub struct ExecutionError(String);

impl ExecutionError {
    #[must_use]
    pub fn new(message: impl Into<String>) -> Self {
        Self(message.into())
    }

    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }

    #[must_use]
    pub fn into_message(self) -> String {
        self.0
    }
}

impl TryFrom<String> for ExecutionError {
    type Error = EmptyErrorMessage;

    fn try_from(value: String) -> Result<Self, Self::Error> {
        if value.is_empty() {
            return Err(EmptyErrorMessage);
        }
        Ok(Self(value))
    }
}

impl From<ExecutionError> for String {
    fn from(value: ExecutionError) -> Self {
        value.0
    }
}

impl fmt::Display for ExecutionError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl std::error::Error for ExecutionError {}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct EmptyErrorMessage;

impl fmt::Display for EmptyErrorMessage {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "error message cannot be empty")
    }
}

impl std::error::Error for EmptyErrorMessage {}

// ===========================================================================
// Record Identifiers
// ===========================================================================

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct ExecutionRecordId(Uuid);

impl ExecutionRecordId {
    #[must_use]
    pub fn new() -> Self {
        Self(Uuid::new_v4())
    }

    #[must_use]
    pub const fn as_uuid(&self) -> Uuid {
        self.0
    }
}

impl Default for ExecutionRecordId {
    fn default() -> Self {
        Self::new()
    }
}

impl From<Uuid> for ExecutionRecordId {
    fn from(uuid: Uuid) -> Self {
        Self(uuid)
    }
}

impl From<ExecutionRecordId> for Uuid {
    fn from(id: ExecutionRecordId) -> Self {
        id.0
    }
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct WorkflowName(String);

impl WorkflowName {
    #[must_use]
    pub fn new(name: impl Into<String>) -> Self {
        Self(name.into())
    }

    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for WorkflowName {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct StepCount(pub u32);

impl StepCount {
    #[must_use]
    pub const fn zero() -> Self {
        Self(0)
    }

    #[must_use]
    pub const fn increment(self) -> Self {
        Self(self.0.saturating_add(1))
    }

    #[must_use]
    pub const fn get(self) -> u32 {
        self.0
    }
}

impl Default for StepCount {
    fn default() -> Self {
        Self::zero()
    }
}

// ===========================================================================
// Status Types
// ===========================================================================

/// Overall status of a complete workflow execution run.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ExecutionOverallStatus {
    Running,
    Succeeded,
    Failed,
    Cancelled,
}

impl ExecutionOverallStatus {
    /// Returns `true` if the execution has reached a terminal state.
    #[must_use]
    pub const fn is_terminal(self) -> bool {
        matches!(self, Self::Succeeded | Self::Failed | Self::Cancelled)
    }
}

// ===========================================================================
// Step Output
// ===========================================================================

/// Step output representing the result of a step execution.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case", tag = "status")]
pub enum StepOutput {
    /// Step completed successfully with output data
    Success(serde_json::Value),
    /// Step failed with error message
    Failure { error: ExecutionError, attempted_at: Option<DateTime<Utc>> },
    /// Step is still running
    Running { started_at: DateTime<Utc>, attempt: super::AttemptNumber },
    /// Step was cancelled
    Cancelled,
}

impl StepOutput {
    #[must_use]
    pub const fn success(value: serde_json::Value) -> Self {
        Self::Success(value)
    }

    #[must_use]
    pub const fn failure(error: ExecutionError) -> Self {
        Self::Failure { error, attempted_at: None }
    }

    #[must_use]
    pub fn running() -> Self {
        Self::Running { started_at: Utc::now(), attempt: super::AttemptNumber::first() }
    }
}

// ===========================================================================
// Step Record
// ===========================================================================

/// Record of a single step execution within a workflow run.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct StepRecord {
    pub step_name: super::StepName,
    pub step_type: super::StepType,
    pub status: ExecutionState,
    pub start_time: Option<DateTime<Utc>>,
    pub end_time: Option<DateTime<Utc>>,
    pub attempt: super::AttemptNumber,
    pub input: Option<serde_json::Value>,
    pub output: StepOutput,
}

impl StepRecord {
    #[must_use]
    pub fn new(step_name: super::StepName, step_type: super::StepType) -> Self {
        Self {
            step_name,
            step_type,
            status: ExecutionState::Idle,
            start_time: None,
            end_time: None,
            attempt: super::AttemptNumber::first(),
            input: None,
            output: StepOutput::running(),
        }
    }
}

// ===========================================================================
// Execution Record
// ===========================================================================

/// A complete, frozen snapshot of a single workflow execution run.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ExecutionRecord {
    pub id: ExecutionRecordId,
    pub workflow_name: WorkflowName,
    pub status: ExecutionOverallStatus,
    pub start_time: DateTime<Utc>,
    pub end_time: Option<DateTime<Utc>>,
    pub steps: Vec<(NodeId, StepRecord)>,
    pub steps_completed: StepCount,
    pub steps_failed: StepCount,
}

impl ExecutionRecord {
    #[must_use]
    pub fn duration_ms(&self) -> Option<i64> {
        let end = self.end_time?;
        end.signed_duration_since(self.start_time).num_milliseconds().into()
    }

    #[must_use]
    pub const fn is_frozen(&self) -> bool {
        self.status.is_terminal()
    }

    #[must_use]
    pub fn step_for_node(&self, id: NodeId) -> Option<&StepRecord> {
        self.steps.iter().find(|(node_id, _)| *node_id == id).map(|(_, record)| record)
    }
}
