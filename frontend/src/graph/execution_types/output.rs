//! Node output and execution outcome types.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use super::{ExecutionMetadata, SharedContext};
use crate::graph::{NodeId, WorkflowExecutionError};

// ===========================================================================
// Execution Outcome Types
// ===========================================================================

/// Execution outcome for a node.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ExecutionOutcome {
    Success,
    Skipped,
    Failed,
    Timeout,
}

/// Output from a single node execution.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct NodeOutput {
    pub node_id: NodeId,
    pub output: serde_json::Value,
    pub status: ExecutionOutcome,
    pub duration_ms: u64,
    pub timestamp: DateTime<Utc>,
    pub error_message: Option<String>,
}

// ===========================================================================
// Execution Result
// ===========================================================================

/// Result of executing a workflow.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ExecutionResult {
    /// Unique execution ID.
    pub execution_id: uuid::Uuid,
    /// Node execution outputs.
    pub node_results: std::collections::HashMap<NodeId, NodeOutput>,
    /// Errors encountered during execution.
    pub errors: Vec<WorkflowExecutionError>,
    /// Final shared context state.
    pub shared_context: SharedContext,
    /// Execution metadata.
    pub metadata: ExecutionMetadata,
    /// Whether execution completed successfully.
    pub success: bool,
}
