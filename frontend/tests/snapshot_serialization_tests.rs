//! Snapshot tests for serialization of key graph types.
//!
//! Uses insta to lock down JSON and YAML serialization output for:
//! - Node (with various WorkflowNode variants)
//! - Workflow (multi-node graph)
//! - ExecutionState (all 6 variants)
//! - Connection
//!
//! These snapshots serve as regression guards: any change to serde derives,
//! rename attributes, skip directives, or field names will cause a snapshot
//! diff that must be reviewed before acceptance.
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic, clippy::float_cmp)]

use oya_frontend::graph::workflow_node::configs::*;
use oya_frontend::graph::{
    Connection, ExecutionState, Node, NodeId, PortName, Viewport, Workflow, WorkflowNode,
};
use uuid::Uuid;

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

/// Create a deterministic NodeId from a fixed UUID string.
fn fixed_node_id(seed: u8) -> NodeId {
    // Build a UUID where the last two hex chars encode the seed.
    // e.g. seed=1 => "00000000-0000-0000-0000-000000000001"
    let hex = format!("{seed:032x}");
    NodeId(Uuid::parse_str(&hex).expect("seed-based UUID is always valid hex"))
}

/// Create a Node with a deterministic id so snapshots are stable.
fn make_node(name: &str, variant: WorkflowNode, x: f32, y: f32, id_seed: u8) -> Node {
    let mut node = Node::from_workflow_node(name.to_string(), variant, x, y);
    node.id = fixed_node_id(id_seed);
    node
}

// ===========================================================================
// 1. ExecutionState snapshots (all 6 variants, JSON + YAML)
// ===========================================================================

#[test]
fn snapshot_execution_state_json() {
    insta::assert_json_snapshot!("execution_state_idle", &ExecutionState::Idle);
    insta::assert_json_snapshot!("execution_state_queued", &ExecutionState::Queued);
    insta::assert_json_snapshot!("execution_state_running", &ExecutionState::Running);
    insta::assert_json_snapshot!("execution_state_completed", &ExecutionState::Completed);
    insta::assert_json_snapshot!("execution_state_failed", &ExecutionState::Failed);
    insta::assert_json_snapshot!("execution_state_skipped", &ExecutionState::Skipped);
}

#[test]
fn snapshot_execution_state_yaml() {
    insta::assert_yaml_snapshot!("execution_state_yaml_idle", &ExecutionState::Idle);
    insta::assert_yaml_snapshot!("execution_state_yaml_running", &ExecutionState::Running);
    insta::assert_yaml_snapshot!("execution_state_yaml_completed", &ExecutionState::Completed);
}

// ===========================================================================
// 2. Node snapshots -- one per major category of WorkflowNode
// ===========================================================================

#[test]
fn snapshot_node_http_handler() {
    let node = make_node(
        "API Endpoint",
        WorkflowNode::HttpHandler(HttpHandlerConfig {
            path: Some("/api/orders".to_string()),
            method: Some("POST".to_string()),
        }),
        100.0,
        200.0,
        1,
    );
    insta::assert_json_snapshot!("node_http_handler", &node);
}

#[test]
fn snapshot_node_http_call() {
    let node = make_node(
        "Call External",
        WorkflowNode::HttpCall(HttpCallConfig {
            url: Some("https://api.example.com/v1/data".to_string()),
        }),
        300.0,
        100.0,
        2,
    );
    insta::assert_json_snapshot!("node_http_call", &node);
}

#[test]
fn snapshot_node_kafka_handler() {
    let node = make_node(
        "Order Events",
        WorkflowNode::KafkaHandler(KafkaHandlerConfig {
            topic: Some("orders.created".to_string()),
        }),
        50.0,
        50.0,
        3,
    );
    insta::assert_json_snapshot!("node_kafka_handler", &node);
}

#[test]
fn snapshot_node_run() {
    let node = make_node(
        "Transform Data",
        WorkflowNode::Run(RunConfig {
            mapping: Some(serde_json::json!({"input": "$.body"})),
            code: Some("return { result: input * 2 };".to_string()),
            durable_step_name: None,
        }),
        400.0,
        200.0,
        4,
    );
    insta::assert_json_snapshot!("node_run", &node);
}

#[test]
fn snapshot_node_condition() {
    let node = make_node(
        "Check Stock",
        WorkflowNode::Condition(ConditionConfig { expression: Some("$.stock > 0".to_string()) }),
        500.0,
        150.0,
        5,
    );
    insta::assert_json_snapshot!("node_condition", &node);
}

#[test]
fn snapshot_node_get_state() {
    let node = make_node(
        "Read Cart",
        WorkflowNode::GetState(GetStateConfig { key: Some("cart".to_string()) }),
        200.0,
        300.0,
        6,
    );
    insta::assert_json_snapshot!("node_get_state", &node);
}

#[test]
fn snapshot_node_set_state() {
    let node = make_node(
        "Save Cart",
        WorkflowNode::SetState(SetStateConfig {
            key: Some("cart".to_string()),
            value: Some("{\"items\":[]}".to_string()),
        }),
        200.0,
        400.0,
        7,
    );
    insta::assert_json_snapshot!("node_set_state", &node);
}

#[test]
fn snapshot_node_sleep() {
    let node = make_node(
        "Wait 5s",
        WorkflowNode::Sleep(SleepConfig { duration_ms: Some(5000) }),
        600.0,
        100.0,
        8,
    );
    insta::assert_json_snapshot!("node_sleep", &node);
}

#[test]
fn snapshot_node_durable_promise() {
    let node = make_node(
        "Await Confirmation",
        WorkflowNode::DurablePromise(DurablePromiseConfig {
            promise_name: Some("order-confirmed".to_string()),
        }),
        700.0,
        200.0,
        9,
    );
    insta::assert_json_snapshot!("node_durable_promise", &node);
}

#[test]
fn snapshot_node_signal_handler() {
    let node = make_node(
        "On Cancel",
        WorkflowNode::SignalHandler(SignalHandlerConfig {
            signal_name: Some("order-cancelled".to_string()),
        }),
        100.0,
        500.0,
        10,
    );
    insta::assert_json_snapshot!("node_signal_handler", &node);
}

// ===========================================================================
// 3. WorkflowNode direct serialization (kebab-case tag)
// ===========================================================================

#[test]
fn snapshot_workflow_node_variants_json() {
    // Verify the externally-tagged serde format: {"type": "kebab-case", ...fields}
    insta::assert_json_snapshot!(
        "workflow_node_http_handler",
        &WorkflowNode::HttpHandler(HttpHandlerConfig {
            path: Some("/health".to_string()),
            method: Some("GET".to_string()),
        })
    );
    insta::assert_json_snapshot!(
        "workflow_node_cron_trigger",
        &WorkflowNode::CronTrigger(CronTriggerConfig { schedule: Some("0 * * * *".to_string()) })
    );
    insta::assert_json_snapshot!(
        "workflow_node_parallel",
        &WorkflowNode::Parallel(ParallelConfig { branches: Some(4) })
    );
    insta::assert_json_snapshot!(
        "workflow_node_send_message",
        &WorkflowNode::SendMessage(SendMessageConfig {
            target: Some("notification-service".to_string()),
        })
    );
}

// ===========================================================================
// 4. Connection snapshot
// ===========================================================================

#[test]
fn snapshot_connection_json() {
    let conn = Connection {
        id: Uuid::parse_str("aaaaaaaa-bbbb-cccc-dddd-eeeeeeeeeeee")
            .expect("hardcoded UUID is valid"),
        source: fixed_node_id(1),
        target: fixed_node_id(2),
        source_port: PortName::from("main"),
        target_port: PortName::from("input"),
    };
    insta::assert_json_snapshot!("connection", &conn);
}

#[test]
fn snapshot_connection_yaml() {
    let conn = Connection {
        id: Uuid::parse_str("aaaaaaaa-bbbb-cccc-dddd-eeeeeeeeeeee")
            .expect("hardcoded UUID is valid"),
        source: fixed_node_id(1),
        target: fixed_node_id(2),
        source_port: PortName::from("true"),
        target_port: PortName::from("main"),
    };
    insta::assert_yaml_snapshot!("connection_yaml", &conn);
}

// ===========================================================================
// 5. Viewport snapshot
// ===========================================================================

#[test]
fn snapshot_viewport_json() {
    let viewport = Viewport { x: -150.0, y: 75.5, zoom: 1.25 };
    insta::assert_json_snapshot!("viewport", &viewport);
}

// ===========================================================================
// 6. Workflow snapshot (3 nodes, 2 connections)
// ===========================================================================

#[test]
fn snapshot_workflow_json() {
    let mut workflow = Workflow::new();

    // Override the random IDs with deterministic ones.
    let handler = make_node(
        "HTTP Trigger",
        WorkflowNode::HttpHandler(HttpHandlerConfig {
            path: Some("/order".to_string()),
            method: Some("POST".to_string()),
        }),
        0.0,
        0.0,
        1,
    );
    let transform = make_node(
        "Process Order",
        WorkflowNode::Run(RunConfig {
            code: Some("return input;".to_string()),
            ..RunConfig::default()
        }),
        250.0,
        0.0,
        2,
    );
    let check = make_node(
        "In Stock?",
        WorkflowNode::Condition(ConditionConfig { expression: Some("$.quantity > 0".to_string()) }),
        500.0,
        0.0,
        3,
    );

    workflow.nodes = vec![handler, transform, check];

    workflow.connections = vec![
        Connection {
            id: Uuid::parse_str("11111111-1111-1111-1111-111111111111")
                .expect("hardcoded UUID is valid"),
            source: fixed_node_id(1),
            target: fixed_node_id(2),
            source_port: PortName::from("main"),
            target_port: PortName::from("main"),
        },
        Connection {
            id: Uuid::parse_str("22222222-2222-2222-2222-222222222222")
                .expect("hardcoded UUID is valid"),
            source: fixed_node_id(2),
            target: fixed_node_id(3),
            source_port: PortName::from("main"),
            target_port: PortName::from("main"),
        },
    ];

    workflow.viewport = Viewport { x: 0.0, y: 0.0, zoom: 1.0 };

    insta::assert_json_snapshot!("workflow_3_nodes_2_connections", &workflow);
}

// ===========================================================================
// 7. Node round-trip: serialize then deserialize must be lossless
// ===========================================================================

#[test]
fn snapshot_node_round_trip_http_handler() {
    let original = make_node(
        "Round Trip",
        WorkflowNode::HttpHandler(HttpHandlerConfig {
            path: Some("/ping".to_string()),
            method: Some("GET".to_string()),
        }),
        42.0,
        99.0,
        20,
    );

    let json = serde_json::to_string(&original).expect("Node serialization must succeed");
    let deserialized: Node =
        serde_json::from_str(&json).expect("Node deserialization must succeed");

    // Note: `node` field is #[serde(skip)], so it won't survive round-trip.
    // We verify that the non-skipped fields round-trip correctly.
    assert_eq!(original.id, deserialized.id);
    assert_eq!(original.name, deserialized.name);
    assert_eq!(original.x, deserialized.x);
    assert_eq!(original.y, deserialized.y);
    assert_eq!(original.category, deserialized.category);
    assert_eq!(original.icon, deserialized.icon);
    assert_eq!(original.node_type, deserialized.node_type);
    assert_eq!(original.description, deserialized.description);
    assert_eq!(original.config, deserialized.config);

    // Snapshot the serialized form for visual confirmation of the output shape.
    insta::assert_json_snapshot!("node_round_trip_http_handler", &original);
}

#[test]
fn snapshot_workflow_round_trip() {
    let mut workflow = Workflow::new();
    let n1 = make_node("A", WorkflowNode::Run(RunConfig::default()), 0.0, 0.0, 30);
    let n2 = make_node(
        "B",
        WorkflowNode::Sleep(SleepConfig { duration_ms: Some(1000) }),
        200.0,
        0.0,
        31,
    );

    workflow.nodes = vec![n1, n2];
    workflow.connections = vec![Connection {
        id: Uuid::parse_str("33333333-3333-3333-3333-333333333333")
            .expect("hardcoded UUID is valid"),
        source: fixed_node_id(30),
        target: fixed_node_id(31),
        source_port: PortName::from("main"),
        target_port: PortName::from("main"),
    }];

    let json = serde_json::to_string(&workflow).expect("Workflow serialization must succeed");
    let deserialized: Workflow =
        serde_json::from_str(&json).expect("Workflow deserialization must succeed");

    assert_eq!(workflow.nodes.len(), deserialized.nodes.len());
    assert_eq!(workflow.connections.len(), deserialized.connections.len());
    assert_eq!(workflow.viewport, deserialized.viewport);

    insta::assert_json_snapshot!("workflow_round_trip", &workflow);
}

// ===========================================================================
// 8. Exhaustive snapshot tests for all 25 WorkflowNode variants
// ===========================================================================
//
// Each test creates a Node via make_node with a deterministic id_seed (starting
// at 50 to avoid collisions with earlier tests), then asserts a JSON snapshot.
// Function names follow the convention:
//   given_<node_type>_node_when_serializing_then_snapshot_matches

#[test]
fn given_http_handler_node_when_serializing_then_snapshot_matches() {
    let node = make_node(
        "HTTP Handler",
        WorkflowNode::HttpHandler(HttpHandlerConfig {
            path: Some("/api/ping".to_string()),
            method: Some("GET".to_string()),
        }),
        10.0,
        20.0,
        50,
    );
    insta::assert_json_snapshot!("given_http_handler_node", &node);
}

#[test]
fn given_http_call_node_when_serializing_then_snapshot_matches() {
    let node = make_node(
        "HTTP Call",
        WorkflowNode::HttpCall(HttpCallConfig {
            url: Some("https://api.example.com/v1/data".to_string()),
        }),
        30.0,
        40.0,
        51,
    );
    insta::assert_json_snapshot!("given_http_call_node", &node);
}

#[test]
fn given_kafka_handler_node_when_serializing_then_snapshot_matches() {
    let node = make_node(
        "Kafka Handler",
        WorkflowNode::KafkaHandler(KafkaHandlerConfig {
            topic: Some("orders.created".to_string()),
        }),
        50.0,
        60.0,
        52,
    );
    insta::assert_json_snapshot!("given_kafka_handler_node", &node);
}

#[test]
fn given_cron_trigger_node_when_serializing_then_snapshot_matches() {
    let node = make_node(
        "Cron Trigger",
        WorkflowNode::CronTrigger(CronTriggerConfig { schedule: Some("0 */5 * * *".to_string()) }),
        70.0,
        80.0,
        53,
    );
    insta::assert_json_snapshot!("given_cron_trigger_node", &node);
}

#[test]
fn given_workflow_submit_node_when_serializing_then_snapshot_matches() {
    let node = make_node(
        "Workflow Submit",
        WorkflowNode::WorkflowSubmit(WorkflowSubmitConfig {
            workflow_name: Some("process-order".to_string()),
        }),
        90.0,
        100.0,
        54,
    );
    insta::assert_json_snapshot!("given_workflow_submit_node", &node);
}

#[test]
fn given_run_node_when_serializing_then_snapshot_matches() {
    let node = make_node(
        "Run Code",
        WorkflowNode::Run(RunConfig {
            mapping: Some(serde_json::json!({"input": "$.body"})),
            code: Some("return input;".to_string()),
            durable_step_name: Some("transform-step".to_string()),
        }),
        110.0,
        120.0,
        55,
    );
    insta::assert_json_snapshot!("given_run_node", &node);
}

#[test]
fn given_service_call_node_when_serializing_then_snapshot_matches() {
    let node = make_node(
        "Service Call",
        WorkflowNode::ServiceCall(ServiceCallConfig {
            service: Some("payment-service".to_string()),
        }),
        130.0,
        140.0,
        56,
    );
    insta::assert_json_snapshot!("given_service_call_node", &node);
}

#[test]
fn given_object_call_node_when_serializing_then_snapshot_matches() {
    let node = make_node(
        "Object Call",
        WorkflowNode::ObjectCall(ObjectCallConfig {
            object_name: Some("order-object".to_string()),
        }),
        150.0,
        160.0,
        57,
    );
    insta::assert_json_snapshot!("given_object_call_node", &node);
}

#[test]
fn given_workflow_call_node_when_serializing_then_snapshot_matches() {
    let node = make_node(
        "Workflow Call",
        WorkflowNode::WorkflowCall(WorkflowCallConfig {
            workflow_name: Some("handle-refund".to_string()),
        }),
        170.0,
        180.0,
        58,
    );
    insta::assert_json_snapshot!("given_workflow_call_node", &node);
}

#[test]
fn given_send_message_node_when_serializing_then_snapshot_matches() {
    let node = make_node(
        "Send Message",
        WorkflowNode::SendMessage(SendMessageConfig {
            target: Some("notification-queue".to_string()),
        }),
        190.0,
        200.0,
        59,
    );
    insta::assert_json_snapshot!("given_send_message_node", &node);
}

#[test]
fn given_delayed_send_node_when_serializing_then_snapshot_matches() {
    let node = make_node(
        "Delayed Send",
        WorkflowNode::DelayedSend(DelayedSendConfig {
            target: Some("reminder-queue".to_string()),
            delay_ms: Some(30_000),
        }),
        210.0,
        220.0,
        60,
    );
    insta::assert_json_snapshot!("given_delayed_send_node", &node);
}

#[test]
fn given_get_state_node_when_serializing_then_snapshot_matches() {
    let node = make_node(
        "Get State",
        WorkflowNode::GetState(GetStateConfig { key: Some("user-session".to_string()) }),
        230.0,
        240.0,
        61,
    );
    insta::assert_json_snapshot!("given_get_state_node", &node);
}

#[test]
fn given_set_state_node_when_serializing_then_snapshot_matches() {
    let node = make_node(
        "Set State",
        WorkflowNode::SetState(SetStateConfig {
            key: Some("user-session".to_string()),
            value: Some("{\"active\":true}".to_string()),
        }),
        250.0,
        260.0,
        62,
    );
    insta::assert_json_snapshot!("given_set_state_node", &node);
}

#[test]
fn given_clear_state_node_when_serializing_then_snapshot_matches() {
    let node = make_node(
        "Clear State",
        WorkflowNode::ClearState(ClearStateConfig { key: Some("user-session".to_string()) }),
        270.0,
        280.0,
        63,
    );
    insta::assert_json_snapshot!("given_clear_state_node", &node);
}

#[test]
fn given_condition_node_when_serializing_then_snapshot_matches() {
    let node = make_node(
        "Condition",
        WorkflowNode::Condition(ConditionConfig { expression: Some("$.amount > 100".to_string()) }),
        290.0,
        300.0,
        64,
    );
    insta::assert_json_snapshot!("given_condition_node", &node);
}

#[test]
fn given_switch_node_when_serializing_then_snapshot_matches() {
    let node = make_node(
        "Switch",
        WorkflowNode::Switch(SwitchConfig { expression: Some("$.status".to_string()) }),
        310.0,
        320.0,
        65,
    );
    insta::assert_json_snapshot!("given_switch_node", &node);
}

#[test]
fn given_loop_node_when_serializing_then_snapshot_matches() {
    let node = make_node(
        "Loop",
        WorkflowNode::Loop(LoopConfig { iterator: Some("$.items".to_string()) }),
        330.0,
        340.0,
        66,
    );
    insta::assert_json_snapshot!("given_loop_node", &node);
}

#[test]
fn given_parallel_node_when_serializing_then_snapshot_matches() {
    let node = make_node(
        "Parallel",
        WorkflowNode::Parallel(ParallelConfig { branches: Some(3) }),
        350.0,
        360.0,
        67,
    );
    insta::assert_json_snapshot!("given_parallel_node", &node);
}

#[test]
fn given_compensate_node_when_serializing_then_snapshot_matches() {
    let node = make_node(
        "Compensate",
        WorkflowNode::Compensate(CompensateConfig { target_step: Some("charge-card".to_string()) }),
        370.0,
        380.0,
        68,
    );
    insta::assert_json_snapshot!("given_compensate_node", &node);
}

#[test]
fn given_sleep_node_when_serializing_then_snapshot_matches() {
    let node = make_node(
        "Sleep",
        WorkflowNode::Sleep(SleepConfig { duration_ms: Some(10_000) }),
        390.0,
        400.0,
        69,
    );
    insta::assert_json_snapshot!("given_sleep_node", &node);
}

#[test]
fn given_timeout_node_when_serializing_then_snapshot_matches() {
    let node = make_node(
        "Timeout",
        WorkflowNode::Timeout(TimeoutConfig { timeout_ms: Some(5_000) }),
        410.0,
        420.0,
        70,
    );
    insta::assert_json_snapshot!("given_timeout_node", &node);
}

#[test]
fn given_durable_promise_node_when_serializing_then_snapshot_matches() {
    let node = make_node(
        "Durable Promise",
        WorkflowNode::DurablePromise(DurablePromiseConfig {
            promise_name: Some("await-approval".to_string()),
        }),
        430.0,
        440.0,
        71,
    );
    insta::assert_json_snapshot!("given_durable_promise_node", &node);
}

#[test]
fn given_awakeable_node_when_serializing_then_snapshot_matches() {
    let node = make_node(
        "Awakeable",
        WorkflowNode::Awakeable(AwakeableConfig { awakeable_id: Some("callback-123".to_string()) }),
        450.0,
        460.0,
        72,
    );
    insta::assert_json_snapshot!("given_awakeable_node", &node);
}

#[test]
fn given_resolve_promise_node_when_serializing_then_snapshot_matches() {
    let node = make_node(
        "Resolve Promise",
        WorkflowNode::ResolvePromise(ResolvePromiseConfig {
            promise_name: Some("await-approval".to_string()),
        }),
        470.0,
        480.0,
        73,
    );
    insta::assert_json_snapshot!("given_resolve_promise_node", &node);
}

#[test]
fn given_signal_handler_node_when_serializing_then_snapshot_matches() {
    let node = make_node(
        "Signal Handler",
        WorkflowNode::SignalHandler(SignalHandlerConfig {
            signal_name: Some("order-cancelled".to_string()),
        }),
        490.0,
        500.0,
        74,
    );
    insta::assert_json_snapshot!("given_signal_handler_node", &node);
}
