use super::super::graph_ops;
use super::super::NodeId;
use super::super::Workflow;
use super::super::WorkflowExecutionError;

use std::collections::HashSet;

// ===========================================================================
// Execution Orchestrator
// ===========================================================================

impl Workflow {
    /// Prepare the workflow for execution.
    ///
    /// # Errors
    ///
    /// Returns `WorkflowExecutionError` if the workflow is invalid.
    pub fn prepare_run(&mut self) -> Result<(), WorkflowExecutionError> {
        // Precondition checks (Data layer) - FIRST, before any state changes
        // This ensures we fail fast if there's a problem
        self.check_non_empty()?;
        self.check_dirty_state()?;
        self.validate_dependencies_exist()?;
        self.check_duplicate_connections()?;
        self.verify_graph_connectivity()?;
        self.check_self_references()?;

        // Cycle detection before building queue (Calc layer)
        if let Some(cycle) = self.find_cycle() {
            return Err(WorkflowExecutionError::CycleDetected { cycle_nodes: cycle });
        }

        // Build execution queue using Kahn's algorithm (Calc layer)
        let execution_queue = self.build_execution_queue()?;

        // Update state (Action layer) - reset all node states
        self.execution_queue = execution_queue;
        self.current_step = 0;

        for node in &mut self.nodes {
            node.executing = false;
            node.last_output = None;
            node.skipped = false;
            node.error = None;
            let _ = Self::set_node_pending_status(node);
        }

        // Reset memory tracking
        self.current_memory_bytes = 0;
        self.execution_failed = false;

        // Reset checkpoint and rollback state for new execution
        self.reset_checkpoint();
        self.clear_rollback_stack();

        Ok(())
    }

    /// Collect all descendant nodes reachable from the given start IDs.
    pub(in super::super) fn collect_descendants(&self, start_ids: &[NodeId]) -> HashSet<NodeId> {
        let node_ids = graph_ops::collect_node_ids(&self.nodes);
        let outgoing = graph_ops::build_outgoing_adjacency(&self.connections, &node_ids);
        graph_ops::find_reachable(start_ids, &outgoing)
    }
}
