//! Serde round-trip tests for key serializable types.
//!
//! Each test serializes a value to JSON, deserializes it back, and compares
//! field-by-field to verify that serde `Serialize` + `Deserialize` impls are
//! consistent and lossless for the serialized fields.
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic, clippy::float_cmp)]

use oya_frontend::canonical_ports::default_ingress_url;
use oya_frontend::graph::execution_state::ExecutionState;
use oya_frontend::graph::workflow_node::configs::*;
use oya_frontend::graph::{Connection, Node, NodeId, PortName, Viewport, Workflow, WorkflowNode};
use uuid::Uuid;

// ---------------------------------------------------------------------------
// Helper
// ---------------------------------------------------------------------------

/// Serialize `value` to a JSON string, then deserialize it back.
/// Panics with a descriptive message if either step fails.
fn round_trip<T>(value: &T) -> T
where
    T: serde::Serialize + serde::de::DeserializeOwned + std::fmt::Debug,
{
    let json = serde_json::to_string(value).expect("serialization should succeed");
    serde_json::from_str::<T>(&json).expect("deserialization should succeed")
}

// ===========================================================================
// 1. WorkflowNode variants
// ===========================================================================

#[test]
fn workflow_node_all_variants_round_trip() {
    let variants: Vec<WorkflowNode> = vec![
        WorkflowNode::HttpHandler(HttpHandlerConfig {
            path: Some("/api/v1/users".to_string()),
            method: Some("POST".to_string()),
        }),
        WorkflowNode::HttpCall(HttpCallConfig { url: Some("https://example.com".to_string()) }),
        WorkflowNode::KafkaHandler(KafkaHandlerConfig { topic: Some("orders".to_string()) }),
        WorkflowNode::CronTrigger(CronTriggerConfig { schedule: Some("0 * * * *".to_string()) }),
        WorkflowNode::WorkflowSubmit(WorkflowSubmitConfig {
            workflow_name: Some("child-flow".to_string()),
        }),
        WorkflowNode::Run(RunConfig {
            mapping: Some(serde_json::json!({"key": "value"})),
            code: Some("ctx.result(42)".to_string()),
            durable_step_name: Some("step-1".to_string()),
        }),
        WorkflowNode::ServiceCall(ServiceCallConfig { service: Some("payment".to_string()) }),
        WorkflowNode::ObjectCall(ObjectCallConfig { object_name: Some("order-123".to_string()) }),
        WorkflowNode::WorkflowCall(WorkflowCallConfig {
            workflow_name: Some("parent".to_string()),
        }),
        WorkflowNode::SendMessage(SendMessageConfig {
            target: Some("notification-svc".to_string()),
        }),
        WorkflowNode::DelayedSend(DelayedSendConfig {
            target: Some("delayed-svc".to_string()),
            delay_ms: Some(5000),
        }),
        WorkflowNode::GetState(GetStateConfig { key: Some("user_id".to_string()) }),
        WorkflowNode::SetState(SetStateConfig {
            key: Some("cart".to_string()),
            value: Some("{\"items\":[]}".to_string()),
        }),
        WorkflowNode::ClearState(ClearStateConfig { key: Some("temp".to_string()) }),
        WorkflowNode::Condition(ConditionConfig { expression: Some("x > 10".to_string()) }),
        WorkflowNode::Switch(SwitchConfig { expression: Some("status".to_string()) }),
        WorkflowNode::Loop(LoopConfig { iterator: Some("items".to_string()) }),
        WorkflowNode::Parallel(ParallelConfig { branches: Some(4) }),
        WorkflowNode::Compensate(CompensateConfig { target_step: Some("charge".to_string()) }),
        WorkflowNode::Sleep(SleepConfig { duration_ms: Some(1000) }),
        WorkflowNode::Timeout(TimeoutConfig { timeout_ms: Some(30000) }),
        WorkflowNode::DurablePromise(DurablePromiseConfig {
            promise_name: Some("order-complete".to_string()),
        }),
        WorkflowNode::Awakeable(AwakeableConfig { awakeable_id: Some("awake-123".to_string()) }),
        WorkflowNode::ResolvePromise(ResolvePromiseConfig {
            promise_name: Some("order-complete".to_string()),
        }),
        WorkflowNode::SignalHandler(SignalHandlerConfig {
            signal_name: Some("cancel".to_string()),
        }),
    ];

    for original in &variants {
        let deserialized = round_trip(original);
        assert_eq!(*original, deserialized, "WorkflowNode round-trip failed");
    }
}

// ===========================================================================
// 2. Node with each category
// ===========================================================================

#[test]
fn node_round_trip_with_each_category() {
    let node_variants: Vec<WorkflowNode> = vec![
        // Entry
        WorkflowNode::HttpHandler(HttpHandlerConfig {
            path: Some("/health".to_string()),
            method: None,
        }),
        // Durable
        WorkflowNode::ServiceCall(ServiceCallConfig { service: Some("billing".to_string()) }),
        // State
        WorkflowNode::GetState(GetStateConfig { key: Some("counter".to_string()) }),
        // Flow
        WorkflowNode::Condition(ConditionConfig { expression: Some("amount > 0".to_string()) }),
        // Timing
        WorkflowNode::Sleep(SleepConfig { duration_ms: Some(500) }),
        // Signal
        WorkflowNode::SignalHandler(SignalHandlerConfig {
            signal_name: Some("shutdown".to_string()),
        }),
    ];

    for variant in node_variants {
        let original = Node::from_workflow_node(format!("test-{}", variant), variant, 42.0, 99.0);

        let json_str = serde_json::to_string(&original).expect("Node serialization failed");
        let deserialized: Node =
            serde_json::from_str(&json_str).expect("Node deserialization failed");

        // Serialized fields must match exactly
        assert_eq!(original.id, deserialized.id, "Node id mismatch");
        assert_eq!(original.name, deserialized.name, "Node name mismatch");
        assert_eq!(original.category, deserialized.category, "Node category mismatch");
        assert_eq!(original.icon, deserialized.icon, "Node icon mismatch");
        assert_eq!(original.x, deserialized.x, "Node x mismatch");
        assert_eq!(original.y, deserialized.y, "Node y mismatch");
        assert_eq!(original.last_output, deserialized.last_output, "Node last_output mismatch");
        assert_eq!(original.selected, deserialized.selected, "Node selected mismatch");
        assert_eq!(original.executing, deserialized.executing, "Node executing mismatch");
        assert_eq!(original.skipped, deserialized.skipped, "Node skipped mismatch");
        assert_eq!(original.error, deserialized.error, "Node error mismatch");
        assert_eq!(original.node_type, deserialized.node_type, "Node node_type mismatch");
        assert_eq!(original.description, deserialized.description, "Node description mismatch");
        assert_eq!(original.config, deserialized.config, "Node config mismatch");

        // Fields with #[serde(skip)] get default values on deserialization
        assert_eq!(
            deserialized.node,
            WorkflowNode::default(),
            "Node.node should default (serde skip)"
        );
        assert_eq!(
            deserialized.execution_state,
            ExecutionState::default(),
            "Node.execution_state should default (serde skip)"
        );
    }
}

// ===========================================================================
// 3. Connection
// ===========================================================================

#[test]
fn connection_round_trip() {
    let original = Connection {
        id: Uuid::new_v4(),
        source: NodeId::new(),
        target: NodeId::new(),
        source_port: PortName("output".to_string()),
        target_port: PortName("input".to_string()),
    };

    let deserialized = round_trip(&original);

    assert_eq!(original.id, deserialized.id, "Connection id mismatch");
    assert_eq!(original.source, deserialized.source, "Connection source mismatch");
    assert_eq!(original.target, deserialized.target, "Connection target mismatch");
    assert_eq!(original.source_port, deserialized.source_port, "Connection source_port mismatch");
    assert_eq!(original.target_port, deserialized.target_port, "Connection target_port mismatch");
}

// ===========================================================================
// 4. Workflow with nodes and connections
// ===========================================================================

#[test]
fn workflow_round_trip() {
    let mut original = Workflow::new();

    let n1 = original.add_node("http-handler", 10.0, 20.0);
    let n2 = original.add_node("run", 100.0, 200.0);
    let n3 = original.add_node("get-state", 200.0, 100.0);

    let port = PortName("main".to_string());
    let _ = original.add_connection(n1, n2, &port, &port);
    let _ = original.add_connection(n2, n3, &port, &port);

    original.viewport = Viewport { x: -50.0, y: 25.0, zoom: 1.5 };
    original.current_step = 2;
    original.execution_queue = vec![n1, n2, n3];

    let json_str = serde_json::to_string(&original).expect("Workflow serialization failed");
    let deserialized: Workflow =
        serde_json::from_str(&json_str).expect("Workflow deserialization failed");

    // Serialized fields
    assert_eq!(original.nodes.len(), deserialized.nodes.len(), "Workflow node count mismatch");
    assert_eq!(original.connections, deserialized.connections, "Workflow connections mismatch");
    assert_eq!(original.viewport, deserialized.viewport, "Workflow viewport mismatch");
    assert_eq!(
        original.execution_queue, deserialized.execution_queue,
        "Workflow execution_queue mismatch"
    );
    assert_eq!(original.current_step, deserialized.current_step, "Workflow current_step mismatch");
    assert_eq!(
        original.history.len(),
        deserialized.history.len(),
        "Workflow history should be empty"
    );
    assert_eq!(
        original.execution_records, deserialized.execution_records,
        "Workflow execution_records mismatch"
    );

    // Verify each node round-tripped correctly
    for (orig, deser) in original.nodes.iter().zip(deserialized.nodes.iter()) {
        assert_eq!(orig.id, deser.id, "Workflow node id mismatch");
        assert_eq!(orig.name, deser.name, "Workflow node name mismatch");
        assert_eq!(orig.category, deser.category, "Workflow node category mismatch");
        assert_eq!(orig.x, deser.x, "Workflow node x mismatch");
        assert_eq!(orig.y, deser.y, "Workflow node y mismatch");
    }

    // Skipped fields revert to defaults
    assert_eq!(
        deserialized.restate_ingress_url,
        default_ingress_url(),
        "restate_ingress_url should revert to default (skip_serializing + default)"
    );
    assert_eq!(
        deserialized.current_memory_bytes, 0,
        "current_memory_bytes should be default (serde skip)"
    );
    assert!(!deserialized.execution_failed, "execution_failed should be default (serde skip)");
}

// ===========================================================================
// 5. Viewport
// ===========================================================================

#[test]
fn viewport_round_trip() {
    let original = Viewport { x: -123.45, y: 678.9, zoom: 2.5 };

    let deserialized = round_trip(&original);

    assert_eq!(original.x, deserialized.x, "Viewport x mismatch");
    assert_eq!(original.y, deserialized.y, "Viewport y mismatch");
    assert_eq!(original.zoom, deserialized.zoom, "Viewport zoom mismatch");
}

// ===========================================================================
// 6. ExecutionState all variants
// ===========================================================================

#[test]
fn execution_state_all_variants_round_trip() {
    let states = vec![
        ExecutionState::Idle,
        ExecutionState::Queued,
        ExecutionState::Running,
        ExecutionState::Completed,
        ExecutionState::Failed,
        ExecutionState::Skipped,
    ];

    for original in &states {
        let json = serde_json::to_string(original).expect("ExecutionState serialization failed");
        let deserialized: ExecutionState =
            serde_json::from_str(&json).expect("ExecutionState deserialization failed");

        assert_eq!(*original, deserialized, "ExecutionState round-trip failed");

        // Verify the JSON representation is lowercase (per rename_all)
        let expected_lowercase = match original {
            ExecutionState::Idle => "\"idle\"",
            ExecutionState::Queued => "\"queued\"",
            ExecutionState::Running => "\"running\"",
            ExecutionState::Completed => "\"completed\"",
            ExecutionState::Failed => "\"failed\"",
            ExecutionState::Skipped => "\"skipped\"",
        };
        assert_eq!(
            json, expected_lowercase,
            "ExecutionState JSON representation should be lowercase"
        );
    }
}
