//! Shared context and execution metadata types.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use uuid::Uuid;

// ===========================================================================
// Shared Context
// ===========================================================================

/// Shared context passed through the entire workflow execution.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct SharedContext {
    /// Global variables accessible to all nodes.
    pub variables: HashMap<String, serde_json::Value>,
    /// Workflow-wide metadata.
    pub metadata: HashMap<String, String>,
}

impl SharedContext {
    /// Create a new empty shared context.
    #[must_use]
    pub fn new() -> Self {
        Self { variables: HashMap::new(), metadata: HashMap::new() }
    }

    /// Create with initial variables.
    #[must_use]
    pub fn with_variables(variables: HashMap<String, serde_json::Value>) -> Self {
        Self { variables, metadata: HashMap::new() }
    }

    /// Create with initial metadata.
    #[must_use]
    pub fn with_metadata(metadata: HashMap<String, String>) -> Self {
        Self { variables: HashMap::new(), metadata }
    }
}

// ===========================================================================
// Execution Metadata
// ===========================================================================

/// Execution metadata for tracking workflow run statistics.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct ExecutionMetadata {
    pub workflow_id: Uuid,
    pub run_id: Uuid,
    pub started_at: DateTime<Utc>,
    pub completed_at: Option<DateTime<Utc>>,
    pub total_nodes: usize,
    pub executed_nodes: usize,
    pub skipped_nodes: usize,
    pub failed_nodes: usize,
}

impl ExecutionMetadata {
    /// Create new execution metadata with workflow and run IDs.
    #[must_use]
    pub fn new(workflow_id: Uuid, run_id: Uuid, total_nodes: usize) -> Self {
        Self {
            workflow_id,
            run_id,
            started_at: Utc::now(),
            completed_at: None,
            total_nodes,
            executed_nodes: 0,
            skipped_nodes: 0,
            failed_nodes: 0,
        }
    }

    /// Increment executed node count.
    #[must_use]
    #[allow(clippy::missing_const_for_fn)] // DateTime<Utc> is not const-safe
    pub fn with_executed_node(self) -> Self {
        Self { executed_nodes: self.executed_nodes + 1, ..self }
    }

    /// Increment skipped node count.
    #[must_use]
    #[allow(clippy::missing_const_for_fn)] // DateTime<Utc> is not const-safe
    pub fn with_skipped_node(self) -> Self {
        Self { skipped_nodes: self.skipped_nodes + 1, ..self }
    }

    /// Increment failed node count.
    #[must_use]
    #[allow(clippy::missing_const_for_fn)] // DateTime<Utc> is not const-safe
    pub fn with_failed_node(self) -> Self {
        Self { failed_nodes: self.failed_nodes + 1, ..self }
    }

    /// Set completion time.
    #[must_use]
    #[allow(clippy::missing_const_for_fn)] // DateTime<Utc> is not const-safe
    pub fn with_completed_at(self, completed_at: DateTime<Utc>) -> Self {
        Self { completed_at: Some(completed_at), ..self }
    }
}
