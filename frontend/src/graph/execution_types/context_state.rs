//! Execution context maintained during workflow execution.

use std::collections::HashMap;

use uuid::Uuid;

use super::{ExecutionMetadata, NodeOutput, SharedContext};
use crate::graph::{NodeId, WorkflowExecutionError};

// ===========================================================================
// Execution Context
// ===========================================================================

/// Execution context maintained during workflow execution.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ExecutionContext {
    /// Output from each executed node (`node_id` -> output).
    pub node_outputs: HashMap<NodeId, NodeOutput>,
    /// Shared context passed through entire workflow.
    pub shared_state: SharedContext,
    /// Errors collected during execution (non-fatal).
    pub errors: Vec<WorkflowExecutionError>,
    /// Execution metadata (timestamps, iteration count, etc.).
    pub metadata: ExecutionMetadata,
    /// Current iteration count (for cycle handling).
    pub iteration: usize,
}

impl ExecutionContext {
    /// Create new execution context with initial state.
    #[must_use]
    pub fn new(workflow_id: Uuid, run_id: Uuid, total_nodes: usize) -> Self {
        Self {
            node_outputs: HashMap::new(),
            shared_state: SharedContext::new(),
            errors: Vec::new(),
            metadata: ExecutionMetadata::new(workflow_id, run_id, total_nodes),
            iteration: 0,
        }
    }

    /// Create with shared state.
    #[must_use]
    pub fn with_shared_state(
        workflow_id: Uuid,
        run_id: Uuid,
        total_nodes: usize,
        shared_state: SharedContext,
    ) -> Self {
        Self {
            node_outputs: HashMap::new(),
            shared_state,
            errors: Vec::new(),
            metadata: ExecutionMetadata::new(workflow_id, run_id, total_nodes),
            iteration: 0,
        }
    }

    /// Add a node output to the context (returns new context).
    #[must_use]
    pub fn with_node_output(mut self, node_output: NodeOutput) -> Self {
        self.node_outputs.insert(node_output.node_id, node_output);
        self
    }

    /// Add an error to the context.
    #[must_use]
    pub fn with_error(mut self, error: WorkflowExecutionError) -> Self {
        self.errors.push(error);
        self
    }

    /// Get node output by ID.
    #[must_use]
    pub fn get_node_output(&self, node_id: &NodeId) -> Option<&NodeOutput> {
        self.node_outputs.get(node_id)
    }

    /// Check if node has been executed.
    #[must_use]
    pub fn has_executed(&self, node_id: &NodeId) -> bool {
        self.node_outputs.contains_key(node_id)
    }
}
