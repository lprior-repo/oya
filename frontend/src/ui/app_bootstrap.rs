use crate::canonical_ports::default_ingress_url;
use crate::graph::{execution_types::ExecutionConfig, Node, Viewport, Workflow};
use crate::graph::{ConditionConfig, HttpHandlerConfig, RunConfig, WorkflowNode};

fn starter_nodes() -> Vec<Node> {
    vec![http_handler_node(), durable_step_node(), condition_node()]
}

fn http_handler_node() -> Node {
    Node::from_workflow_node(
        "HTTP Handler".to_string(),
        WorkflowNode::HttpHandler(HttpHandlerConfig {
            path: Some("/SignupWorkflow/{userId}/run".to_string()),
            method: Some("POST".to_string()),
        }),
        350.0,
        40.0,
    )
}

fn durable_step_node() -> Node {
    Node::from_workflow_node(
        "Durable Step".to_string(),
        WorkflowNode::Run(RunConfig {
            durable_step_name: Some("create-user".to_string()),
            code: None,
            mapping: None,
        }),
        350.0,
        170.0,
    )
}

fn condition_node() -> Node {
    Node::from_workflow_node(
        "If / Else".to_string(),
        WorkflowNode::Condition(ConditionConfig {
            expression: Some("Check if user creation succeeded".to_string()),
        }),
        350.0,
        300.0,
    )
}

pub fn default_workflow() -> Workflow {
    Workflow {
        nodes: starter_nodes(),
        connections: vec![],
        viewport: Viewport { x: 0.0, y: 0.0, zoom: 0.85 },
        execution_queue: vec![],
        current_step: 0,
        history: vec![],
        execution_records: vec![],
        restate_ingress_url: default_ingress_url(),
        current_memory_bytes: 0,
        execution_config: ExecutionConfig::default(),
        execution_failed: false,
        last_checkpoint_step: None,
        rollback_stack: vec![],
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used, clippy::panic, clippy::float_cmp)]
mod tests {
    use super::default_workflow;
    use crate::graph::NodeCategory;

    #[test]
    fn given_default_workflow_when_created_then_it_contains_expected_starter_nodes() {
        let workflow = default_workflow();

        assert_eq!(workflow.nodes.len(), 3);
        assert_eq!(workflow.nodes[0].node_type, "http-handler");
        assert_eq!(workflow.nodes[1].node_type, "run");
        assert_eq!(workflow.nodes[2].node_type, "condition");
        assert_eq!(workflow.nodes[0].category, NodeCategory::Entry);
    }

    #[test]
    fn given_default_workflow_when_created_then_viewport_defaults_are_expected() {
        let workflow = default_workflow();

        assert_eq!(workflow.viewport.x, 0.0);
        assert_eq!(workflow.viewport.y, 0.0);
        assert_eq!(workflow.viewport.zoom, 0.85);
    }
}
