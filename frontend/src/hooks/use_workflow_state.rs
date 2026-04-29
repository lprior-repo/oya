#![deny(clippy::unwrap_used)]
#![deny(clippy::expect_used)]
#![deny(clippy::panic)]

use crate::errors::{WorkflowError, WorkflowResult};
use crate::graph::{
    Connection, ConnectionResult, ConnectivityConnectionError, Node, NodeId, PortName, Viewport,
    Workflow,
};
use crate::ui::constants::{NODE_CENTER_X_OFFSET, NODE_HANDLE_Y_OFFSET};
use dioxus::prelude::*;
use std::collections::HashMap;

fn push_undo_snapshot(undo_stack: &mut Vec<Workflow>, snapshot: Workflow, cap: usize) {
    undo_stack.push(snapshot);
    if undo_stack.len() > cap {
        undo_stack.remove(0);
    }
}

fn apply_undo(
    workflow: &mut Workflow,
    undo_stack: &mut Vec<Workflow>,
    redo_stack: &mut Vec<Workflow>,
) -> bool {
    match undo_stack.pop() {
        Some(snapshot) => {
            let current = workflow.clone();
            redo_stack.push(current);
            *workflow = snapshot;
            true
        }
        None => false,
    }
}

fn apply_redo(
    workflow: &mut Workflow,
    undo_stack: &mut Vec<Workflow>,
    redo_stack: &mut Vec<Workflow>,
) -> bool {
    match redo_stack.pop() {
        Some(snapshot) => {
            let current = workflow.clone();
            undo_stack.push(current);
            *workflow = snapshot;
            true
        }
        None => false,
    }
}

fn viewport_center_node_origin(
    viewport: &Viewport,
    canvas_width: f32,
    canvas_height: f32,
) -> Option<(f32, f32)> {
    let zoom_val = viewport.zoom;
    if !viewport.x.is_finite()
        || !viewport.y.is_finite()
        || !zoom_val.is_finite()
        || zoom_val <= 0.0
        || !canvas_width.is_finite()
        || !canvas_height.is_finite()
        || canvas_width <= 0.0
        || canvas_height <= 0.0
    {
        return None;
    }

    let center_x = (canvas_width * 0.5 - viewport.x) / zoom_val;
    let center_y = (canvas_height * 0.5 - viewport.y) / zoom_val;
    if !center_x.is_finite() || !center_y.is_finite() {
        return None;
    }

    Some((center_x - NODE_CENTER_X_OFFSET, center_y - NODE_HANDLE_Y_OFFSET))
}

fn merge_run_result(mut current: Workflow, completed: Workflow) -> Workflow {
    let Workflow { nodes, execution_queue, current_step, history, execution_records, .. } =
        completed;

    let mut completed_nodes: HashMap<NodeId, Node> =
        nodes.into_iter().map(|node| (node.id, node)).collect();

    current.nodes.iter_mut().for_each(|node| {
        if let Some(executed) = completed_nodes.remove(&node.id) {
            node.last_output = executed.last_output;
            node.executing = executed.executing;
            node.skipped = executed.skipped;
            node.error = executed.error;
            node.execution_state = executed.execution_state;
            node.metadata = executed.metadata;
            node.execution_data = executed.execution_data;
        } else {
            node.executing = false;
        }
    });

    current.execution_queue = execution_queue;
    current.current_step = current_step;
    current.history = history;
    current.execution_records = execution_records;
    current
}

fn add_connection_transaction(
    workflow: &mut Workflow,
    undo_stack: &mut Vec<Workflow>,
    redo_stack: &mut Vec<Workflow>,
    source: NodeId,
    target: NodeId,
    source_port: &PortName,
    target_port: &PortName,
) -> WorkflowResult<()> {
    let snapshot = workflow.clone();
    match workflow.add_connection_checked(source, target, source_port, target_port) {
        Ok(ConnectionResult::Created) => {
            push_undo_snapshot(undo_stack, snapshot, 60);
            redo_stack.clear();
            Ok(())
        }
        Err(error) => Err(map_connection_error(&error)),
    }
}

fn remove_nodes_transaction(
    workflow: &mut Workflow,
    undo_stack: &mut Vec<Workflow>,
    redo_stack: &mut Vec<Workflow>,
    node_ids: &[NodeId],
) -> WorkflowResult<()> {
    if node_ids.is_empty() {
        return Ok(());
    }

    if let Some(missing_id) =
        node_ids.iter().copied().find(|id| !workflow.nodes.iter().any(|node| node.id == *id))
    {
        return Err(WorkflowError::NodeNotFound(missing_id));
    }

    let snapshot = workflow.clone();
    for node_id in node_ids {
        workflow.remove_node(*node_id);
    }
    push_undo_snapshot(undo_stack, snapshot, 60);
    redo_stack.clear();
    Ok(())
}

/// Workflow state hook - manages workflow data, undo/redo, and derived views.
///
/// This follows the functional reactive pattern where:
/// - State is stored in Copy signals
/// - Actions are methods that mutate state immutably (clone, modify, set)
/// - Derived data is computed via Memo for performance
#[derive(Clone, Copy, PartialEq)]
pub struct WorkflowState {
    workflow: Signal<Workflow>,
    workflow_name: Signal<String>,
    undo_stack: Signal<Vec<Workflow>>,
    redo_stack: Signal<Vec<Workflow>>,
    nodes: Memo<Vec<Node>>,
    nodes_by_id: Memo<HashMap<NodeId, Node>>,
    connections: Memo<Vec<Connection>>,
    viewport: Memo<Viewport>,
}

async fn run_workflow_detached(mut workflow: Workflow, ingress_url: String) -> Workflow {
    workflow.restate_ingress_url = ingress_url;
    workflow.run().await;
    workflow
}

impl WorkflowState {
    /// Access to workflow data signal
    #[must_use]
    pub fn workflow(&self) -> Signal<Workflow> {
        self.workflow
    }

    /// Access to workflow name signal
    #[must_use]
    pub fn workflow_name(&self) -> Signal<String> {
        self.workflow_name
    }

    /// Replace the entire workflow with a new one (for import)
    pub fn load_workflow(&mut self, workflow: crate::graph::Workflow) {
        self.workflow.set(workflow);
    }

    /// Read-only access to nodes list (memoized)
    #[must_use]
    pub fn nodes(&self) -> ReadSignal<Vec<Node>> {
        self.nodes.into()
    }

    /// Read-only access to nodes by ID map (memoized)
    #[must_use]
    pub fn nodes_by_id(&self) -> ReadSignal<HashMap<NodeId, Node>> {
        self.nodes_by_id.into()
    }

    /// Read-only access to connections (memoized)
    #[must_use]
    pub fn connections(&self) -> ReadSignal<Vec<Connection>> {
        self.connections.into()
    }

    /// Read-only access to viewport (memoized)
    #[must_use]
    pub fn viewport(&self) -> ReadSignal<Viewport> {
        self.viewport.into()
    }

    /// Save current state to undo stack before mutation
    pub fn save_undo_point(mut self) {
        let current = self.workflow.read().clone();
        push_undo_snapshot(&mut self.undo_stack.write(), current, 60);
        self.redo_stack.write().clear();
    }

    /// Add a new node at the specified position
    #[must_use]
    pub fn add_node(mut self, node_type: &str, x: f32, y: f32) -> NodeId {
        self.save_undo_point();
        self.workflow.write().add_node(node_type, x, y)
    }

    /// Duplicate a node at an offset position. Returns the new node's ID.
    #[must_use]
    pub fn duplicate_node(mut self, node_id: NodeId) -> Option<NodeId> {
        let wf = self.workflow.read();
        let original = wf.nodes.iter().find(|n| n.id == node_id)?;
        let new_id = NodeId::new();
        let mut new_node = original.clone();
        new_node.id = new_id;
        new_node.x += 40.0;
        new_node.y += 40.0;
        new_node.name = format!("{} (copy)", original.name);
        new_node.execution_state = crate::graph::ExecutionState::Idle;
        new_node.last_output = None;
        new_node.error = None;
        drop(wf);

        self.save_undo_point();
        self.workflow.write().nodes.push(new_node);
        Some(new_id)
    }

    /// Add a node at the viewport center using explicit canvas dimensions
    #[must_use]
    pub fn add_node_at_viewport_center_with_canvas(
        mut self,
        node_type: &str,
        canvas_width: f32,
        canvas_height: f32,
    ) -> NodeId {
        self.save_undo_point();
        let viewport = self.workflow.read().viewport.clone();
        if let Some((x, y)) = viewport_center_node_origin(&viewport, canvas_width, canvas_height) {
            self.workflow.write().add_node(node_type, x, y)
        } else {
            self.workflow.write().add_node(node_type, 0.0, 0.0)
        }
    }

    /// Remove multiple nodes as a single undo transaction
    ///
    /// # Errors
    /// Returns `WorkflowError::NodeNotFound` if any of the provided node IDs do not exist in the workflow.
    pub fn remove_nodes(mut self, node_ids: &[NodeId]) -> WorkflowResult<()> {
        let mut workflow = self.workflow.write();
        let mut undo_stack = self.undo_stack.write();
        let mut redo_stack = self.redo_stack.write();
        remove_nodes_transaction(&mut workflow, &mut undo_stack, &mut redo_stack, node_ids)
    }

    /// Add a connection between two nodes
    ///
    /// # Errors
    /// Returns `WorkflowError` if the connection is invalid (e.g. self-connection, cycle detected, type mismatch).
    pub fn add_connection(
        mut self,
        source: NodeId,
        target: NodeId,
        source_port: &PortName,
        target_port: &PortName,
    ) -> WorkflowResult<()> {
        let mut workflow = self.workflow.write();
        let mut undo_stack = self.undo_stack.write();
        let mut redo_stack = self.redo_stack.write();
        add_connection_transaction(
            &mut workflow,
            &mut undo_stack,
            &mut redo_stack,
            source,
            target,
            source_port,
            target_port,
        )
    }

    /// Zoom the viewport
    pub fn zoom(mut self, delta: f32, center_x: f32, center_y: f32) {
        self.workflow.write().zoom(delta, center_x, center_y);
    }

    /// Pan the viewport
    pub fn pan(mut self, dx: f32, dy: f32) {
        self.workflow.write().viewport.x += dx;
        self.workflow.write().viewport.y += dy;
    }

    /// Fit view to show all nodes
    pub fn fit_view(mut self, width: f32, height: f32, padding: f32) {
        self.workflow.write().fit_view(width, height, padding);
    }

    /// Apply auto-layout to nodes
    pub fn apply_layout(mut self) {
        self.save_undo_point();
        self.workflow.write().apply_layout();
    }

    /// Undo last action - returns true if undo was performed
    #[must_use]
    pub fn undo(mut self) -> bool {
        let mut workflow = self.workflow.read().clone();
        let did_undo =
            apply_undo(&mut workflow, &mut self.undo_stack.write(), &mut self.redo_stack.write());
        if did_undo {
            self.workflow.set(workflow);
        }
        did_undo
    }

    /// Redo last undone action - returns true if redo was performed
    #[must_use]
    pub fn redo(mut self) -> bool {
        let mut workflow = self.workflow.read().clone();
        let did_redo =
            apply_redo(&mut workflow, &mut self.undo_stack.write(), &mut self.redo_stack.write());
        if did_redo {
            self.workflow.set(workflow);
        }
        did_redo
    }

    /// Check if undo is available
    #[must_use]
    pub fn can_undo(&self) -> bool {
        !self.undo_stack.read().is_empty()
    }

    /// Check if redo is available
    #[must_use]
    pub fn can_redo(&self) -> bool {
        !self.redo_stack.read().is_empty()
    }

    /// Update node position
    pub fn update_node_position(mut self, node_id: NodeId, dx: f32, dy: f32) {
        if !dx.is_finite() || !dy.is_finite() {
            return;
        }
        self.workflow.write().update_node_position(node_id, dx, dy);
    }

    /// Run the workflow asynchronously, using `ingress_url` for Restate service calls.
    pub fn run(self, ingress_url: String) {
        let mut workflow_signal = self.workflow;
        let workflow_snapshot = workflow_signal.read().clone();

        spawn(async move {
            let workflow_result = run_workflow_detached(workflow_snapshot, ingress_url).await;
            let merged = merge_run_result(workflow_signal.read().clone(), workflow_result);
            workflow_signal.set(merged);
        });
    }

    /// Find downstream nodes (nodes connected FROM the given node)
    #[must_use]
    pub fn downstream_nodes(&self, node_id: NodeId) -> Vec<NodeId> {
        self.connections.read().iter().filter(|c| c.source == node_id).map(|c| c.target).collect()
    }

    /// Find upstream nodes (nodes connected TO the given node)
    #[must_use]
    pub fn upstream_nodes(&self, node_id: NodeId) -> Vec<NodeId> {
        self.connections.read().iter().filter(|c| c.target == node_id).map(|c| c.source).collect()
    }

    /// Move a node by a delta amount (for keyboard navigation)
    pub fn move_node_by(self, node_id: NodeId, dx: f32, dy: f32) {
        self.update_node_position(node_id, dx, dy);
    }

    /// Get the first node in the workflow (for initial selection)
    #[must_use]
    pub fn first_node_id(&self) -> Option<NodeId> {
        self.nodes.read().first().map(|n| n.id)
    }
}

fn map_connection_error(error: &ConnectivityConnectionError) -> WorkflowError {
    match error {
        ConnectivityConnectionError::SelfConnection => WorkflowError::SelfConnection,
        ConnectivityConnectionError::MissingSourceNode(node_id)
        | ConnectivityConnectionError::MissingTargetNode(node_id) => {
            WorkflowError::NodeNotFound(*node_id)
        }
        ConnectivityConnectionError::WouldCreateCycle => WorkflowError::CycleDetected,
        ConnectivityConnectionError::Duplicate => WorkflowError::DuplicateConnection,
        ConnectivityConnectionError::TypeMismatch { source_type, target_type } => {
            WorkflowError::InvalidConnection(format!(
                "Type mismatch: {source_type} is not compatible with {target_type}"
            ))
        }
        ConnectivityConnectionError::ParseError(_) => {
            WorkflowError::InvalidConnection("Parse error".to_string())
        }
    }
}

pub fn provide_workflow_state_context() -> WorkflowState {
    let workflow = use_signal(|| {
        #[cfg(target_arch = "wasm32")]
        {
            use web_sys::window;
            let storage = window().and_then(|w| w.local_storage().ok()).flatten();
            if let Some(s) = storage {
                match s.get_item("flow-wasm-v1-workflow") {
                    Ok(Some(json)) => {
                        if let Ok(mut parsed) = serde_json::from_str::<Workflow>(&json) {
                            parsed.nodes.iter_mut().for_each(|node| {
                                let config = node.config.clone();
                                node.apply_config_update(&config);
                            });
                            return parsed;
                        }
                    }
                    _ => {}
                }
            }
        }
        crate::ui::app_bootstrap::default_workflow()
    });

    let workflow_name = use_signal(|| "SignupWorkflow".to_string());
    let undo_stack = use_signal(Vec::<Workflow>::new);
    let redo_stack = use_signal(Vec::<Workflow>::new);

    // Derived memos for performance
    let nodes = use_memo(move || workflow.read().nodes.clone());
    let nodes_by_id =
        use_memo(move || workflow.read().nodes.iter().map(|n| (n.id, n.clone())).collect());
    let connections = use_memo(move || workflow.read().connections.clone());
    let viewport = use_memo(move || workflow.read().viewport.clone());

    let state = WorkflowState {
        workflow,
        workflow_name,
        undo_stack,
        redo_stack,
        nodes,
        nodes_by_id,
        connections,
        viewport,
    };
    provide_context(state)
}

#[must_use]
pub fn use_workflow_state() -> WorkflowState {
    use_context::<WorkflowState>()
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used, clippy::panic, clippy::float_cmp)]
mod tests {
    use super::{
        add_connection_transaction, apply_redo, apply_undo, map_connection_error, merge_run_result,
        push_undo_snapshot, remove_nodes_transaction, run_workflow_detached,
        viewport_center_node_origin,
    };
    use crate::canonical_ports::default_ingress_url;
    use crate::errors::WorkflowError;
    use crate::graph::restate_types::PortType;
    use crate::graph::{NodeId, PortName, Viewport, Workflow};
    use serde_json::json;

    #[tokio::test]
    async fn detached_workflow_when_running_then_history_is_recorded() {
        let mut workflow = Workflow::new();
        workflow.add_node("http-handler", 0.0, 0.0);

        let updated = run_workflow_detached(workflow, default_ingress_url()).await;

        assert_eq!(updated.history.len(), 1);
        assert!(updated.history[0].success);
    }

    #[test]
    fn given_workflow_state_source_when_running_async_then_no_write_guard_spans_await() {
        let source = include_str!("use_workflow_state.rs");
        let bad_pattern = [".write()", ".run()", ".await"].join("");

        assert!(!source.contains(&bad_pattern));
    }

    #[test]
    fn given_connection_error_when_mapping_to_workflow_error_then_taxonomy_is_preserved() {
        use crate::errors::WorkflowError;
        use crate::graph::ConnectivityConnectionError;

        let mismatch = map_connection_error(&ConnectivityConnectionError::TypeMismatch {
            source_type: PortType::Event,
            target_type: PortType::Signal,
        });
        assert!(matches!(mismatch, WorkflowError::InvalidConnection(_)));

        // Note: connectivity::ConnectionError doesn't have ContextTypeMismatch or ServiceKindIncompatible
        // These are in connection_errors::ConnectionError for Restate type validation
    }

    #[test]
    fn given_more_than_sixty_snapshots_when_pushing_undo_then_stack_is_capped() {
        let mut undo_stack = Vec::new();
        let workflow = Workflow::new();

        for _ in 0..65 {
            push_undo_snapshot(&mut undo_stack, workflow.clone(), 60);
        }

        assert_eq!(undo_stack.len(), 60);
    }

    #[test]
    fn undo_then_redo_sequence_when_applied_then_snapshots_restore_correctly() {
        let mut workflow = Workflow::new();
        let mut undo_stack = Vec::new();
        let mut redo_stack = Vec::new();

        let mut older = Workflow::new();
        older.add_node("http-handler", 0.0, 0.0);
        let mut newer = Workflow::new();
        newer.add_node("http-handler", 0.0, 0.0);
        newer.add_node("run", 0.0, 0.0);

        workflow.clone_from(&newer);
        undo_stack.push(older.clone());

        assert!(apply_undo(&mut workflow, &mut undo_stack, &mut redo_stack));
        assert_eq!(workflow.nodes.len(), 1);
        assert_eq!(redo_stack.len(), 1);

        assert!(apply_redo(&mut workflow, &mut undo_stack, &mut redo_stack));
        assert_eq!(workflow.nodes.len(), 2);
    }

    #[test]
    fn given_viewport_and_canvas_when_computing_center_origin_then_origin_is_centered() {
        let viewport = Viewport { x: -200.0, y: -100.0, zoom: 2.0_f32.clamp(0.15, 3.0) };

        let origin = viewport_center_node_origin(&viewport, 1280.0, 760.0);

        assert_eq!(origin, Some((310.0, 206.0)));
    }

    #[test]
    fn given_min_zoom_when_computing_center_origin_then_some_is_returned() {
        let viewport = Viewport { x: 0.0, y: 0.0, zoom: 0.0_f32.clamp(0.15, 3.0) };

        let origin = viewport_center_node_origin(&viewport, 1280.0, 760.0);

        assert!(origin.is_some());
    }

    #[test]
    fn given_failed_connection_attempt_when_adding_then_undo_and_redo_are_unchanged() {
        let mut workflow = Workflow::new();
        let node = workflow.add_node("run", 0.0, 0.0);
        let mut undo_stack = vec![Workflow::new()];
        let mut redo_stack = vec![Workflow::new()];
        let main = PortName::from("main");
        let workflow_before = workflow.clone();
        let undo_before = undo_stack.clone();
        let redo_before = redo_stack.clone();

        let result = add_connection_transaction(
            &mut workflow,
            &mut undo_stack,
            &mut redo_stack,
            node,
            node,
            &main,
            &main,
        );

        assert!(matches!(result, Err(WorkflowError::SelfConnection)));
        assert_eq!(workflow, workflow_before);
        assert_eq!(undo_stack, undo_before);
        assert_eq!(redo_stack, redo_before);
    }

    #[test]
    fn given_successful_connection_attempt_when_adding_then_undo_is_pushed_and_redo_cleared() {
        let mut workflow = Workflow::new();
        let source = workflow.add_node("http-handler", 0.0, 0.0);
        let target = workflow.add_node("run", 100.0, 0.0);
        let workflow_before = workflow.clone();
        let mut undo_stack = Vec::new();
        let mut redo_stack = vec![Workflow::new()];
        let main = PortName::from("main");

        let result = add_connection_transaction(
            &mut workflow,
            &mut undo_stack,
            &mut redo_stack,
            source,
            target,
            &main,
            &main,
        );

        assert!(result.is_ok());
        assert_eq!(workflow.connections.len(), 1);
        assert_eq!(undo_stack, vec![workflow_before]);
        assert!(redo_stack.is_empty());
    }

    #[test]
    fn given_local_edits_when_merging_run_result_then_layout_edits_are_preserved() {
        let mut baseline = Workflow::new();
        let node_id = baseline.add_node("run", 0.0, 0.0);

        let mut current = baseline.clone();
        if let Some(node) = current.nodes.iter_mut().find(|node| node.id == node_id) {
            node.x = 500.0;
            node.y = 260.0;
            node.description = "local-edit".to_string();
        }

        let mut completed = baseline.clone();
        if let Some(node) = completed.nodes.iter_mut().find(|node| node.id == node_id) {
            node.last_output = Some(json!({"ok": true}));
            node.skipped = true;
        }

        let merged = merge_run_result(current, completed);
        let merged_node = merged.nodes.iter().find(|node| node.id == node_id).cloned();

        assert!(merged_node.is_some());
        let merged_node = merged_node.unwrap_or_default();
        assert!((merged_node.x - 500.0).abs() < f32::EPSILON);
        assert!((merged_node.y - 260.0).abs() < f32::EPSILON);
        assert_eq!(merged_node.description, "local-edit");
        assert_eq!(merged_node.last_output, Some(json!({"ok": true})));
        assert!(merged_node.skipped);
    }

    #[test]
    fn given_batch_remove_when_removing_nodes_then_single_snapshot_is_pushed() {
        let mut workflow = Workflow::new();
        let first = workflow.add_node("run", 0.0, 0.0);
        let second = workflow.add_node("run", 120.0, 0.0);
        let _third = workflow.add_node("run", 240.0, 0.0);
        let workflow_before = workflow.clone();
        let mut undo_stack = Vec::new();
        let mut redo_stack = vec![Workflow::new()];

        let result = remove_nodes_transaction(
            &mut workflow,
            &mut undo_stack,
            &mut redo_stack,
            &[first, second],
        );

        assert!(result.is_ok());
        assert_eq!(workflow.nodes.len(), 1);
        assert_eq!(undo_stack, vec![workflow_before]);
        assert!(redo_stack.is_empty());
    }

    #[test]
    fn given_missing_node_in_batch_when_removing_nodes_then_no_snapshot_is_pushed() {
        let mut workflow = Workflow::new();
        let existing = workflow.add_node("run", 0.0, 0.0);
        let missing = NodeId::new();
        let workflow_before = workflow.clone();
        let mut undo_stack = Vec::new();
        let mut redo_stack = vec![Workflow::new()];

        let result = remove_nodes_transaction(
            &mut workflow,
            &mut undo_stack,
            &mut redo_stack,
            &[existing, missing],
        );

        assert!(matches!(result, Err(WorkflowError::NodeNotFound(id)) if id == missing));
        assert_eq!(workflow, workflow_before);
        assert!(undo_stack.is_empty());
        assert_eq!(redo_stack.len(), 1);
    }
}
