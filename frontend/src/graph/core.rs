use super::execution_types::ExecutionConfig;
use super::{can_transition, ExecutionState, Node, NodeId, RollbackAction, Viewport, Workflow};
use crate::canonical_ports::default_ingress_url;
use crate::graph::{calc, workflow_node::WorkflowNode};
use std::str::FromStr;

fn config_with_status(config: &serde_json::Value, status_text: String) -> serde_json::Value {
    let mut config_obj = config.as_object().cloned().unwrap_or_default();
    config_obj.insert("status".to_owned(), serde_json::Value::String(status_text));
    serde_json::Value::Object(config_obj)
}

impl Workflow {
    /// Set the status of a node, validating the state transition.
    ///
    /// This function is the primary public API for state transitions.
    /// It validates that the transition is allowed by the state machine
    /// and updates both the `execution_state` and `config["status"]` fields.
    ///
    /// # Errors
    /// Returns `InvalidTransition` if the state transition is not allowed.
    pub fn set_node_status(
        node: &mut Node,
        proposed_status: ExecutionState,
    ) -> Result<(), super::InvalidTransition> {
        // Validate state transition using the state machine
        if !can_transition(node.execution_state, proposed_status) {
            return Err(super::InvalidTransition::new(node.execution_state, proposed_status));
        }

        let status_text = proposed_status.to_string();
        node.execution_state = proposed_status;
        node.config = config_with_status(&node.config, status_text);
        Ok(())
    }

    /// Set a node's pending status, transitioning `Idle` -> `Queued` or `Queued` -> `Queued`.
    ///
    /// This is a specialized function for setting pending status on nodes.
    /// It validates that the transition is allowed and updates `config["status"]` to "pending".
    ///
    /// # Errors
    /// Returns `InvalidTransition` if the node is not in `Idle` or `Queued` state.
    pub fn set_node_pending_status(node: &mut Node) -> Result<(), super::InvalidTransition> {
        // Validate state transition: Idle -> Queued or Queued -> Queued (self-transition allowed)
        let is_valid_transition = can_transition(node.execution_state, ExecutionState::Queued)
            || (node.execution_state == ExecutionState::Queued);
        if !is_valid_transition {
            return Err(super::InvalidTransition::new(
                node.execution_state,
                ExecutionState::Queued,
            ));
        }

        node.execution_state = ExecutionState::Queued;
        node.config = config_with_status(&node.config, "pending".to_owned());
        Ok(())
    }

    #[must_use]
    pub fn new() -> Self {
        Self {
            nodes: Vec::new(),
            connections: Vec::new(),
            viewport: Viewport { x: 0.0, y: 0.0, zoom: 1.0 },
            execution_queue: Vec::new(),
            current_step: 0,
            history: Vec::new(),
            execution_records: Vec::new(),
            restate_ingress_url: default_ingress_url(),
            current_memory_bytes: 0,
            execution_config: ExecutionConfig::default(),
            execution_failed: false,
            last_checkpoint_step: None,
            rollback_stack: Vec::new(),
        }
    }

    /// Create a checkpoint at the current step for durable execution recovery.
    #[allow(clippy::missing_const_for_fn)]
    pub fn create_checkpoint(&mut self) {
        self.last_checkpoint_step = Some(self.current_step);
    }

    /// Reset checkpoint state for a new execution.
    pub const fn reset_checkpoint(&mut self) {
        self.last_checkpoint_step = None;
    }

    /// Push a rollback action for saga compensation.
    pub fn push_rollback(
        &mut self,
        node_id: NodeId,
        previous_output: Option<serde_json::Value>,
        compensation_handler: Option<String>,
    ) {
        self.rollback_stack.push(RollbackAction { node_id, previous_output, compensation_handler });
    }

    /// Pop and return the next rollback action.
    pub fn pop_rollback(&mut self) -> Option<RollbackAction> {
        self.rollback_stack.pop()
    }

    /// Clear all rollback actions.
    pub fn clear_rollback_stack(&mut self) {
        self.rollback_stack.clear();
    }

    /// Get the number of pending rollback actions.
    #[must_use]
    pub const fn rollback_count(&self) -> usize {
        self.rollback_stack.len()
    }

    pub fn add_node(&mut self, node_type: &str, x: f32, y: f32) -> NodeId {
        // Avoid allocating a Vec: pass a slice of references to existing positions
        let existing_positions: Vec<(f32, f32)> = self.nodes.iter().map(|n| (n.x, n.y)).collect();
        let (final_x, final_y) = calc::find_safe_position(&existing_positions, x, y, 30.0);

        let id = NodeId::new();
        let name = format!("{node_type} {}", self.nodes.len() + 1);

        let workflow_node = WorkflowNode::from_str(node_type)
            .unwrap_or_else(|_| WorkflowNode::Run(crate::graph::RunConfig::default()));

        let mut node = Node::from_workflow_node(name, workflow_node, final_x, final_y);
        node.id = id;
        self.nodes.push(node);
        id
    }

    pub fn add_node_at_viewport_center(&mut self, node_type: &str) {
        let vx = self.viewport.x;
        let vy = self.viewport.y;
        let vz = self.viewport.zoom;
        let nx = (400.0 - vx) / vz;
        let ny = (300.0 - vy) / vz;
        self.add_node(node_type, nx, ny);
    }

    pub fn update_node_position(&mut self, id: NodeId, dx: f32, dy: f32) {
        if let Some(node) = self.nodes.iter_mut().find(|n| n.id == id) {
            let (new_x, new_y) = calc::update_node_position(node.x, node.y, dx, dy);
            node.x = new_x;
            node.y = new_y;
        }
    }

    pub fn deselect_all(&mut self) {
        self.nodes.iter_mut().for_each(|node| {
            node.set_selected(false);
        });
    }

    pub fn remove_node(&mut self, id: NodeId) {
        self.nodes.retain(|n| n.id != id);
        self.connections.retain(|c| c.source != id && c.target != id);
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used, clippy::panic, clippy::float_cmp)]
mod tests {
    use super::*;
    use crate::graph::{PortName, RunConfig, WorkflowNode};

    #[test]
    fn occupied_position_when_adding_node_then_safe_position_offsets_new_node() {
        let mut workflow = Workflow::new();
        workflow.add_node("run", 100.0, 100.0);
        let id = workflow.add_node("run", 100.0, 100.0);

        let added = workflow.nodes.iter().find(|node| node.id == id);
        assert!(added.is_some_and(|node| (node.x, node.y) == (130.0, 130.0)));
    }

    #[test]
    fn given_added_node_when_returning_id_then_it_matches_inserted_node_id() {
        let mut workflow = Workflow::new();
        let id = workflow.add_node("run", 0.0, 0.0);

        assert!(workflow.nodes.iter().any(|node| node.id == id));
    }

    #[test]
    fn given_viewport_offset_and_zoom_when_adding_node_at_center_then_node_is_centered() {
        let mut workflow = Workflow::new();
        workflow.viewport = Viewport { x: -200.0, y: -100.0, zoom: 2.0_f32.clamp(0.15, 3.0) };

        workflow.add_node_at_viewport_center("run");

        let node = workflow.nodes.first();
        assert!(node.is_some_and(|n| (n.x, n.y) == (300.0, 200.0)));
    }

    #[test]
    fn removed_node_when_removing_then_incident_connections_are_removed() {
        let mut workflow = Workflow::new();
        let a = workflow.add_node("http-handler", 0.0, 0.0);
        let b = workflow.add_node("run", 100.0, 0.0);
        let c = workflow.add_node("run", 200.0, 0.0);
        let main = PortName::from("main");

        let _ = workflow.add_connection_checked(a, b, &main, &main);
        let _ = workflow.add_connection_checked(b, c, &main, &main);

        workflow.remove_node(b);

        assert_eq!(workflow.nodes.len(), 2);
        assert!(workflow.connections.iter().all(|conn| conn.source != b && conn.target != b));
    }

    #[test]
    fn node_when_setting_status_then_status_is_updated_in_execution_state_and_config() {
        let mut node = Node::from_workflow_node(
            "n".to_string(),
            WorkflowNode::Run(RunConfig::default()),
            0.0,
            0.0,
        );

        // Node starts in Idle state, need to transition to Queued first
        let _ = Workflow::set_node_status(&mut node, ExecutionState::Queued);

        // Now we can transition from Queued to Running
        let _ = Workflow::set_node_status(&mut node, ExecutionState::Running);

        // execution_state should be updated
        assert_eq!(node.execution_state, ExecutionState::Running);
        // config["status"] should be updated
        assert_eq!(node.config.get("status").and_then(serde_json::Value::as_str), Some("running"));
    }

    // ---------------------------------------------------------------------------
    // set_node_status — error path: terminal state transitions
    // ---------------------------------------------------------------------------

    fn node_in_state(state: ExecutionState) -> Node {
        let mut node = Node::from_workflow_node(
            "n".to_string(),
            WorkflowNode::Run(RunConfig::default()),
            0.0,
            0.0,
        );
        node.execution_state = state;
        node
    }

    #[test]
    fn given_completed_node_when_transitioning_to_idle_then_invalid_transition_error_is_returned() {
        let mut node = node_in_state(ExecutionState::Completed);
        let result = Workflow::set_node_status(&mut node, ExecutionState::Idle);

        assert!(result.is_err());
        let err = result.err().expect("should have error");
        assert_eq!(err.from_state(), ExecutionState::Completed);
        assert_eq!(err.to_state(), ExecutionState::Idle);
        assert_eq!(
            node.execution_state,
            ExecutionState::Completed,
            "node state should remain unchanged after failed transition"
        );
    }

    #[test]
    fn given_completed_node_when_transitioning_to_queued_then_invalid_transition_error_is_returned()
    {
        let mut node = node_in_state(ExecutionState::Completed);
        let result = Workflow::set_node_status(&mut node, ExecutionState::Queued);

        assert!(result.is_err());
        let err = result.err().expect("should have error");
        assert_eq!(err.from_state(), ExecutionState::Completed);
        assert_eq!(err.to_state(), ExecutionState::Queued);
    }

    #[test]
    fn given_completed_node_when_transitioning_to_running_then_invalid_transition_error_is_returned(
    ) {
        let mut node = node_in_state(ExecutionState::Completed);
        let result = Workflow::set_node_status(&mut node, ExecutionState::Running);

        assert!(result.is_err());
        let err = result.err().expect("should have error");
        assert_eq!(err.from_state(), ExecutionState::Completed);
        assert_eq!(err.to_state(), ExecutionState::Running);
    }

    #[test]
    fn given_completed_node_when_transitioning_to_failed_then_invalid_transition_error_is_returned()
    {
        let mut node = node_in_state(ExecutionState::Completed);
        let result = Workflow::set_node_status(&mut node, ExecutionState::Failed);

        assert!(result.is_err());
        let err = result.err().expect("should have error");
        assert_eq!(err.from_state(), ExecutionState::Completed);
        assert_eq!(err.to_state(), ExecutionState::Failed);
    }

    #[test]
    fn given_completed_node_when_transitioning_to_skipped_then_invalid_transition_error_is_returned(
    ) {
        let mut node = node_in_state(ExecutionState::Completed);
        let result = Workflow::set_node_status(&mut node, ExecutionState::Skipped);

        assert!(result.is_err());
        let err = result.err().expect("should have error");
        assert_eq!(err.from_state(), ExecutionState::Completed);
        assert_eq!(err.to_state(), ExecutionState::Skipped);
    }

    #[test]
    fn given_failed_node_when_transitioning_to_idle_then_invalid_transition_error_is_returned() {
        let mut node = node_in_state(ExecutionState::Failed);
        let result = Workflow::set_node_status(&mut node, ExecutionState::Idle);

        assert!(result.is_err());
        let err = result.err().expect("should have error");
        assert_eq!(err.from_state(), ExecutionState::Failed);
        assert_eq!(err.to_state(), ExecutionState::Idle);
        assert_eq!(
            node.execution_state,
            ExecutionState::Failed,
            "node state should remain unchanged after failed transition"
        );
    }

    #[test]
    fn given_failed_node_when_transitioning_to_queued_then_invalid_transition_error_is_returned() {
        let mut node = node_in_state(ExecutionState::Failed);
        let result = Workflow::set_node_status(&mut node, ExecutionState::Queued);

        assert!(result.is_err());
        let err = result.err().expect("should have error");
        assert_eq!(err.from_state(), ExecutionState::Failed);
        assert_eq!(err.to_state(), ExecutionState::Queued);
    }

    #[test]
    fn given_failed_node_when_transitioning_to_running_then_invalid_transition_error_is_returned() {
        let mut node = node_in_state(ExecutionState::Failed);
        let result = Workflow::set_node_status(&mut node, ExecutionState::Running);

        assert!(result.is_err());
        let err = result.err().expect("should have error");
        assert_eq!(err.from_state(), ExecutionState::Failed);
        assert_eq!(err.to_state(), ExecutionState::Running);
    }

    #[test]
    fn given_skipped_node_when_transitioning_to_idle_then_invalid_transition_error_is_returned() {
        let mut node = node_in_state(ExecutionState::Skipped);
        let result = Workflow::set_node_status(&mut node, ExecutionState::Idle);

        assert!(result.is_err());
        let err = result.err().expect("should have error");
        assert_eq!(err.from_state(), ExecutionState::Skipped);
        assert_eq!(err.to_state(), ExecutionState::Idle);
        assert_eq!(
            node.execution_state,
            ExecutionState::Skipped,
            "node state should remain unchanged after failed transition"
        );
    }

    #[test]
    fn given_skipped_node_when_transitioning_to_queued_then_invalid_transition_error_is_returned() {
        let mut node = node_in_state(ExecutionState::Skipped);
        let result = Workflow::set_node_status(&mut node, ExecutionState::Queued);

        assert!(result.is_err());
        let err = result.err().expect("should have error");
        assert_eq!(err.from_state(), ExecutionState::Skipped);
        assert_eq!(err.to_state(), ExecutionState::Queued);
    }

    #[test]
    fn given_skipped_node_when_transitioning_to_running_then_invalid_transition_error_is_returned()
    {
        let mut node = node_in_state(ExecutionState::Skipped);
        let result = Workflow::set_node_status(&mut node, ExecutionState::Running);

        assert!(result.is_err());
        let err = result.err().expect("should have error");
        assert_eq!(err.from_state(), ExecutionState::Skipped);
        assert_eq!(err.to_state(), ExecutionState::Running);
    }

    // ---------------------------------------------------------------------------
    // set_node_status — error path: invalid non-terminal transitions
    // ---------------------------------------------------------------------------

    #[test]
    fn given_idle_node_when_transitioning_to_running_then_invalid_transition_error_is_returned() {
        let mut node = node_in_state(ExecutionState::Idle);
        let result = Workflow::set_node_status(&mut node, ExecutionState::Running);

        assert!(result.is_err());
        let err = result.err().expect("should have error");
        assert_eq!(err.from_state(), ExecutionState::Idle);
        assert_eq!(err.to_state(), ExecutionState::Running);
    }

    #[test]
    fn given_idle_node_when_transitioning_to_completed_then_invalid_transition_error_is_returned() {
        let mut node = node_in_state(ExecutionState::Idle);
        let result = Workflow::set_node_status(&mut node, ExecutionState::Completed);

        assert!(result.is_err());
        assert_eq!(node.execution_state, ExecutionState::Idle);
    }

    #[test]
    fn given_idle_node_when_transitioning_to_failed_then_invalid_transition_error_is_returned() {
        let mut node = node_in_state(ExecutionState::Idle);
        let result = Workflow::set_node_status(&mut node, ExecutionState::Failed);

        assert!(result.is_err());
        assert_eq!(node.execution_state, ExecutionState::Idle);
    }

    #[test]
    fn given_queued_node_when_transitioning_to_completed_then_invalid_transition_error_is_returned()
    {
        let mut node = node_in_state(ExecutionState::Queued);
        let result = Workflow::set_node_status(&mut node, ExecutionState::Completed);

        assert!(result.is_err());
        assert_eq!(node.execution_state, ExecutionState::Queued);
    }

    #[test]
    fn given_running_node_when_transitioning_to_queued_then_invalid_transition_error_is_returned() {
        let mut node = node_in_state(ExecutionState::Running);
        let result = Workflow::set_node_status(&mut node, ExecutionState::Queued);

        assert!(result.is_err());
        assert_eq!(node.execution_state, ExecutionState::Running);
    }

    #[test]
    fn given_running_node_when_transitioning_to_idle_then_invalid_transition_error_is_returned() {
        let mut node = node_in_state(ExecutionState::Running);
        let result = Workflow::set_node_status(&mut node, ExecutionState::Idle);

        assert!(result.is_err());
        assert_eq!(node.execution_state, ExecutionState::Running);
    }

    // ---------------------------------------------------------------------------
    // set_node_pending_status — error path: terminal states
    // ---------------------------------------------------------------------------

    #[test]
    fn given_completed_node_when_setting_pending_status_then_invalid_transition_error_is_returned()
    {
        let mut node = node_in_state(ExecutionState::Completed);
        let result = Workflow::set_node_pending_status(&mut node);

        assert!(result.is_err());
        let err = result.err().expect("should have error");
        assert_eq!(err.from_state(), ExecutionState::Completed);
        assert_eq!(err.to_state(), ExecutionState::Queued);
    }

    #[test]
    fn given_failed_node_when_setting_pending_status_then_invalid_transition_error_is_returned() {
        let mut node = node_in_state(ExecutionState::Failed);
        let result = Workflow::set_node_pending_status(&mut node);

        assert!(result.is_err());
        let err = result.err().expect("should have error");
        assert_eq!(err.from_state(), ExecutionState::Failed);
        assert_eq!(err.to_state(), ExecutionState::Queued);
    }

    #[test]
    fn given_skipped_node_when_setting_pending_status_then_invalid_transition_error_is_returned() {
        let mut node = node_in_state(ExecutionState::Skipped);
        let result = Workflow::set_node_pending_status(&mut node);

        assert!(result.is_err());
        let err = result.err().expect("should have error");
        assert_eq!(err.from_state(), ExecutionState::Skipped);
        assert_eq!(err.to_state(), ExecutionState::Queued);
    }

    #[test]
    fn given_running_node_when_setting_pending_status_then_invalid_transition_error_is_returned() {
        let mut node = node_in_state(ExecutionState::Running);
        let result = Workflow::set_node_pending_status(&mut node);

        assert!(result.is_err());
        let err = result.err().expect("should have error");
        assert_eq!(err.from_state(), ExecutionState::Running);
        assert_eq!(err.to_state(), ExecutionState::Queued);
    }

    // ---------------------------------------------------------------------------
    // set_node_pending_status — valid paths (ensure they work)
    // ---------------------------------------------------------------------------

    #[test]
    fn given_idle_node_when_setting_pending_status_then_status_is_queued() {
        let mut node = node_in_state(ExecutionState::Idle);
        let result = Workflow::set_node_pending_status(&mut node);

        assert!(result.is_ok());
        assert_eq!(node.execution_state, ExecutionState::Queued);
        assert_eq!(node.config.get("status").and_then(serde_json::Value::as_str), Some("pending"));
    }

    #[test]
    fn given_queued_node_when_setting_pending_status_then_status_remains_queued() {
        let mut node = node_in_state(ExecutionState::Queued);
        let result = Workflow::set_node_pending_status(&mut node);

        assert!(result.is_ok());
        assert_eq!(node.execution_state, ExecutionState::Queued);
    }

    // ---------------------------------------------------------------------------
    // empty workflow operations
    // ---------------------------------------------------------------------------

    #[test]
    fn given_empty_workflow_when_deselecting_all_then_no_panic() {
        let mut workflow = Workflow::new();
        workflow.deselect_all();
        assert!(workflow.nodes.is_empty());
    }

    #[test]
    fn given_empty_workflow_when_removing_nonexistent_node_then_no_panic() {
        let mut workflow = Workflow::new();
        workflow.remove_node(NodeId::new());
        assert!(workflow.nodes.is_empty());
        assert!(workflow.connections.is_empty());
    }

    #[test]
    fn given_empty_workflow_when_updating_nonexistent_node_position_then_no_panic() {
        let mut workflow = Workflow::new();
        workflow.update_node_position(NodeId::new(), 10.0, 20.0);
        assert!(workflow.nodes.is_empty());
    }
}

// ---------------------------------------------------------------------------
// checkpoint and rollback functionality
// ---------------------------------------------------------------------------

#[test]
fn given_workflow_when_create_checkpoint_then_last_checkpoint_step_is_set() {
    let mut workflow = Workflow::new();
    workflow.current_step = 5;
    workflow.create_checkpoint();
    assert_eq!(workflow.last_checkpoint_step, Some(5));
}

#[test]
fn given_workflow_when_reset_checkpoint_then_last_checkpoint_step_is_none() {
    let mut workflow = Workflow::new();
    workflow.last_checkpoint_step = Some(10);
    workflow.reset_checkpoint();
    assert_eq!(workflow.last_checkpoint_step, None);
}

#[test]
fn given_workflow_when_push_rollback_then_action_is_added_to_stack() {
    let mut workflow = Workflow::new();
    let node_id = NodeId::new();
    let output = serde_json::json!({"key": "value"});
    workflow.push_rollback(node_id, Some(output.clone()), Some("compensate".to_string()));
    assert_eq!(workflow.rollback_count(), 1);
}

#[test]
fn given_workflow_when_pop_rollback_then_action_is_removed() {
    let mut workflow = Workflow::new();
    let node_id = NodeId::new();
    workflow.push_rollback(node_id, None, None);
    let action = workflow.pop_rollback();
    assert!(action.is_some());
    assert_eq!(action.unwrap().node_id, node_id);
    assert_eq!(workflow.rollback_count(), 0);
}

#[test]
fn given_workflow_when_pop_rollback_on_empty_stack_then_none_is_returned() {
    let mut workflow = Workflow::new();
    let action = workflow.pop_rollback();
    assert!(action.is_none());
}

#[test]
fn given_workflow_when_clear_rollback_stack_then_stack_is_empty() {
    let mut workflow = Workflow::new();
    workflow.push_rollback(NodeId::new(), None, None);
    workflow.push_rollback(NodeId::new(), None, None);
    workflow.clear_rollback_stack();
    assert_eq!(workflow.rollback_count(), 0);
}
