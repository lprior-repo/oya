#![deny(clippy::unwrap_used)]
#![deny(clippy::expect_used)]
#![deny(clippy::panic)]
#![warn(clippy::pedantic)]
#![warn(clippy::nursery)]
#![forbid(unsafe_code)]

pub mod calc;
pub mod connectivity;
pub mod core;
mod core_types;
#[cfg(test)]
mod cycle_detection_tests;
mod domain_types;
mod execution;
pub mod execution_errors;
pub mod execution_record;
pub mod execution_record_types;
pub mod execution_runtime;
pub mod execution_state;
pub mod execution_types;
pub mod graph_ops;
mod metadata;
mod primitives;
mod view;

pub mod connection_errors;
pub mod expressions;
pub mod layout;
pub mod node_icon;
pub mod node_ui_state;
pub mod port_types;
pub mod restate_types;
pub mod service_kinds;
mod validation;
mod validation_checks;
pub mod value_objects;
pub mod workflow_node;

pub use connection_errors::{get_node_by_id, ConnectionError as RestateConnectionError};
pub use connectivity::{
    ConnectionError as GraphConnectionError, ConnectionError as ConnectivityConnectionError,
    ConnectionResult,
};

// Re-export ConnectionError for backward compatibility
// Tests expect connection_errors::ConnectionError to be available
pub use connection_errors::ConnectionError;
pub use core_types::{Node, RollbackAction, RunRecord, Viewport, Workflow};
pub use domain_types::{
    EmptyStringError, NodeIcon, NodeMetadata, NodeUiState, NonEmptyString, PositiveDuration,
    RunOutcome, ServiceName, StateKey,
};
pub use execution_errors::WorkflowExecutionError;
pub use execution_record::from_run_record;
pub use execution_record_types::{
    AttemptNumber, EmptyErrorMessage, ExecutionError, ExecutionOverallStatus, ExecutionRecord,
    ExecutionRecordId, StepCount, StepName, StepOutput, StepRecord, StepType, WorkflowName,
};
pub use execution_state::{
    can_transition, try_transition, CompletedState, ExecutionState, FailedState, IdleState,
    InvalidTransition, QueuedState, RunningState, SkippedState, StateTransition, TerminalState,
};
pub use primitives::{Connection, NodeCategory, NodeId, PortName};
pub use validation::{
    validate_unique_node_ids, validate_workflow, ValidationIssue, ValidationResult,
    ValidationSeverity,
};
pub use workflow_node::configs::{
    ConditionConfig, HttpHandlerConfig, RunConfig, SendMessageConfig, SetStateConfig,
};
pub use workflow_node::{ConditionResult, HttpMethod, UnknownHttpMethodError, WorkflowNode};

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used, clippy::panic, clippy::float_cmp)]
mod tests {
    use super::workflow_node::{SendMessageConfig, SetStateConfig};
    use super::{Node, NodeCategory, NodeId, PortName, WorkflowNode};
    use serde_json::{json, Value};

    #[test]
    fn given_node_id_when_displayed_then_it_matches_inner_uuid() {
        let id = NodeId::new();
        assert_eq!(id.to_string(), id.0.to_string());
    }

    #[test]
    fn given_default_node_id_when_created_then_it_is_not_nil() {
        let id = NodeId::default();
        assert_ne!(id.0, uuid::Uuid::nil());
    }

    #[test]
    fn given_string_when_converted_to_port_name_then_value_is_preserved() {
        let port = PortName::from("main");
        assert_eq!(port.0, "main");
    }

    #[test]
    fn given_node_categories_when_displayed_then_lowercase_labels_are_returned() {
        assert_eq!(NodeCategory::Entry.to_string(), "entry");
        assert_eq!(NodeCategory::Durable.to_string(), "durable");
        assert_eq!(NodeCategory::State.to_string(), "state");
        assert_eq!(NodeCategory::Flow.to_string(), "flow");
        assert_eq!(NodeCategory::Timing.to_string(), "timing");
        assert_eq!(NodeCategory::Signal.to_string(), "signal");
    }

    #[test]
    fn given_config_update_when_applied_then_node_config_is_replaced() {
        let mut node = Node::from_workflow_node(
            "state".to_string(),
            WorkflowNode::SetState(SetStateConfig::default()),
            0.0,
            0.0,
        );

        node.apply_config_update(&json!({
            "type": "set-state",
            "stateKey": "cart"
        }));

        assert_eq!(node.config.get("stateKey").and_then(Value::as_str), Some("cart"));
    }

    #[test]
    fn given_set_state_alias_state_key_when_applied_then_typed_key_is_updated() {
        let mut node = Node::from_workflow_node(
            "state".to_string(),
            WorkflowNode::SetState(SetStateConfig::default()),
            0.0,
            0.0,
        );

        node.apply_config_update(&json!({
            "type": "set-state",
            "stateKey": "session"
        }));

        assert_eq!(node.config.get("key").and_then(Value::as_str), Some("session"));

        assert!(matches!(
            &node.node,
            WorkflowNode::SetState(config)
                if config.key.as_deref() == Some("session")
        ));
    }

    #[test]
    fn given_send_message_alias_target_service_when_applied_then_typed_target_is_updated() {
        let mut node = Node::from_workflow_node(
            "send".to_string(),
            WorkflowNode::SendMessage(SendMessageConfig::default()),
            0.0,
            0.0,
        );

        node.apply_config_update(&json!({
            "type": "send-message",
            "targetService": "notification-service"
        }));

        assert_eq!(node.config.get("target").and_then(Value::as_str), Some("notification-service"));

        assert!(matches!(
            &node.node,
            WorkflowNode::SendMessage(config)
                if config.target.as_deref() == Some("notification-service")
        ));
    }

    #[test]
    fn given_non_object_config_when_applying_then_typed_node_is_preserved() {
        let mut node = Node::from_workflow_node(
            "state".to_string(),
            WorkflowNode::SetState(SetStateConfig {
                key: Some("session".to_string()),
                value: Some("active".to_string()),
            }),
            0.0,
            0.0,
        );
        let original_node = node.node.clone();

        node.apply_config_update(&json!("invalid-shape"));

        assert_eq!(node.config, json!("invalid-shape"));
        assert_eq!(node.node, original_node);
    }
}
