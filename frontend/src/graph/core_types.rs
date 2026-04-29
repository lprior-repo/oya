//! Core types for the graph module.
//!
//! Contains fundamental types used throughout the graph system:
//! - `Node`
//! - `Viewport`
//! - `RunRecord`
//! - `Workflow`

use crate::canonical_ports::default_ingress_url;
use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};

use super::{ExecutionState, WorkflowNode};
use crate::graph::{Connection, NodeCategory, NodeId};

// ===========================================================================
// Node
// ===========================================================================

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Node {
    pub id: NodeId,
    pub name: String,
    #[serde(skip)]
    pub node: WorkflowNode,
    pub category: NodeCategory,
    pub icon: String,
    pub x: f32,
    pub y: f32,
    pub last_output: Option<serde_json::Value>,
    #[serde(default)]
    pub selected: bool,
    #[serde(default)]
    pub executing: bool,
    #[serde(default)]
    pub skipped: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
    #[serde(default, skip)]
    pub execution_state: ExecutionState,
    #[serde(default, skip)]
    pub metadata: serde_json::Value,
    #[serde(default, skip)]
    pub execution_data: serde_json::Value,
    #[serde(default)]
    pub node_type: String,
    #[serde(default)]
    pub description: String,
    #[serde(default)]
    pub config: serde_json::Value,
}

impl Node {
    fn alias_target_for_config_key(key: &str) -> Option<&'static str> {
        match key {
            "stateKey" => Some("key"),
            "conditionExpression" => Some("expression"),
            "durableStepName" => Some("durable_step_name"),
            "targetService" | "service_name" => Some("target"),
            "handler_name" => Some("handler"),
            "loopIterator" => Some("iterator"),
            "compensationHandler" => Some("target_step"),
            "cronExpression" => Some("schedule"),
            "workflowKey" => Some("workflow_name"),
            "promiseName" => Some("promise_name"),
            "awakeableId" => Some("awakeable_id"),
            "signalName" => Some("signal_name"),
            "timeoutMs" => Some("timeout_ms"),
            _ => None,
        }
    }

    fn should_apply_alias(config_object: &Map<String, Value>, target: &str) -> bool {
        config_object.get(target).is_none_or(|v| matches!(v, Value::Null))
    }

    fn normalize_config_aliases(config: &Value) -> Value {
        let Value::Object(config_object) = config else {
            return config.clone();
        };

        let mut normalized = config_object.clone();
        for (key, value) in config_object {
            if let Some(target) = Self::alias_target_for_config_key(key) {
                if Self::should_apply_alias(&normalized, target) {
                    normalized.insert(target.to_string(), value.clone());
                }
            }
        }

        Value::Object(normalized)
    }

    fn merged_node_json(&self, config: &Value) -> Option<Value> {
        let Value::Object(base_object) = serde_json::to_value(&self.node).ok()? else {
            return None;
        };

        let Value::Object(config_object) = config else {
            return None;
        };

        let mut merged = base_object.clone();
        for (key, value) in config_object {
            merged.insert(key.clone(), value.clone());
        }

        if let Some(base_type) = base_object.get("type").cloned() {
            merged.insert("type".to_string(), base_type);
        }

        Some(Value::Object(merged))
    }

    pub fn apply_config_update(&mut self, new_config: &Value) {
        let normalized_config = Self::normalize_config_aliases(new_config);
        self.config = normalized_config.clone();

        if let Some(updated_node) = self
            .merged_node_json(&normalized_config)
            .and_then(|json| serde_json::from_value::<WorkflowNode>(json).ok())
        {
            self.node = updated_node;
            self.node_type = self.node.to_string();
            self.category = self.node.category();
            self.icon = self.node.icon().to_string();
            self.description = self.node.description().to_string();
        }
    }

    #[must_use]
    pub fn from_workflow_node(name: String, node: WorkflowNode, x: f32, y: f32) -> Self {
        let category = node.category();
        let icon = node.icon().to_string();
        let node_type = node.to_string();
        let description = node.description().to_string();
        let config = serde_json::to_value(&node).unwrap_or_default();

        Self {
            id: NodeId::new(),
            name,
            node,
            category,
            icon,
            x,
            y,
            last_output: None,
            selected: false,
            executing: false,
            skipped: false,
            error: None,
            execution_state: ExecutionState::default(),
            metadata: Value::default(),
            execution_data: Value::default(),
            node_type,
            description,
            config,
        }
    }

    pub const fn set_selected(&mut self, selected: bool) {
        self.selected = selected;
    }

    /// Check if a state transition is possible.
    ///
    /// This is a convenience wrapper around `try_transition` for testing.
    #[must_use]
    pub const fn can_transition(&self, to: ExecutionState) -> bool {
        super::can_transition(self.execution_state, to)
    }

    /// Try to transition to a new state without modifying the node.
    ///
    /// This is a convenience wrapper around `try_transition` for testing.
    #[must_use]
    pub const fn try_transition(&self, to: ExecutionState) -> Option<super::StateTransition> {
        super::try_transition(self.execution_state, to)
    }
}

impl Default for Node {
    fn default() -> Self {
        Self::from_workflow_node(String::new(), WorkflowNode::default(), 0.0, 0.0)
    }
}

// ===========================================================================
// Viewport
// ===========================================================================

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Viewport {
    pub x: f32,
    pub y: f32,
    pub zoom: f32,
}

// ===========================================================================
// Run Record
// ===========================================================================

#[allow(clippy::derive_partial_eq_without_eq)]
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct RunRecord {
    pub id: uuid::Uuid,
    pub timestamp: chrono::DateTime<chrono::Utc>,
    pub results: std::collections::HashMap<NodeId, serde_json::Value>,
    pub success: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub restate_invocation_id: Option<String>,
}

// ===========================================================================
// Workflow
// ===========================================================================

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Workflow {
    pub nodes: Vec<Node>,
    pub connections: Vec<Connection>,
    pub viewport: Viewport,
    pub execution_queue: Vec<NodeId>,
    pub current_step: usize,
    pub history: Vec<RunRecord>,
    #[serde(default)]
    pub execution_records: Vec<super::ExecutionRecord>,
    /// Base URL for Restate ingress (default: `<http://localhost:909>`).
    /// Populated at runtime before `run()`; not part of the saved workflow definition.
    #[serde(default = "default_restate_ingress_url", skip_serializing)]
    pub restate_ingress_url: String,
    /// Current memory usage in bytes during execution.
    /// Reset to 0 at the start of each execution.
    #[serde(default, skip)]
    pub current_memory_bytes: u64,
    /// Execution configuration for this workflow run.
    /// Contains memory limits, timeouts, and other runtime constraints.
    #[serde(skip)]
    pub execution_config: super::execution_types::ExecutionConfig,
    /// Flag indicating execution failed due to memory limit or other error.
    /// Used to stop the execution loop when limits are exceeded.
    #[serde(skip)]
    pub execution_failed: bool,
    /// Track checkpoint state for durable execution recovery.
    #[serde(skip, default)]
    pub last_checkpoint_step: Option<usize>,
    /// Track rollback state for saga compensation.
    #[serde(skip, default)]
    pub rollback_stack: Vec<RollbackAction>,
}

/// Action to perform during saga rollback.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RollbackAction {
    /// The node ID that needs to be rolled back.
    pub node_id: NodeId,
    /// The state to restore (previous output).
    pub previous_output: Option<serde_json::Value>,
    /// Compensation handler to invoke if available.
    pub compensation_handler: Option<String>,
}

fn default_restate_ingress_url() -> String {
    default_ingress_url()
}

impl Default for Workflow {
    fn default() -> Self {
        Self::new()
    }
}
