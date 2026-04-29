// QA verification test for bead 1af
use oya_frontend::graph::connection_errors::{check_connection, get_node_by_id, ConnectionError};
use oya_frontend::graph::port_types::{types_compatible, PortType};
use oya_frontend::graph::service_kinds::{ClientType, ContextType, ServiceKind};
use oya_frontend::graph::workflow_node::configs::{
    HttpCallConfig, HttpHandlerConfig, ObjectCallConfig, WorkflowCallConfig,
};
use oya_frontend::graph::{Node, NodeId, WorkflowNode};

#[test]
fn service_kind_available_clients_handler_exact_service() {
    let clients = ServiceKind::Handler.available_clients();
    assert_eq!(clients.len(), 1);
    assert_eq!(clients[0], ClientType::Service);
}

#[test]
fn service_kind_available_clients_actor_exact_service_object() {
    let clients = ServiceKind::Actor.available_clients();
    assert_eq!(clients.len(), 2);
    assert_eq!(clients[0], ClientType::Service);
    assert_eq!(clients[1], ClientType::Object);
}

#[test]
fn service_kind_available_clients_workflow_exact_service_object_workflow() {
    let clients = ServiceKind::Workflow.available_clients();
    assert_eq!(clients.len(), 3);
    assert_eq!(clients[0], ClientType::Service);
    assert_eq!(clients[1], ClientType::Object);
    assert_eq!(clients[2], ClientType::Workflow);
}

#[test]
fn context_type_available_traits_synchronous_exact_6_traits() {
    let traits = ContextType::Synchronous.available_traits();
    assert_eq!(traits.len(), 6);
}

#[test]
fn context_type_available_traits_asynchronous_exact_7_traits() {
    let traits = ContextType::Asynchronous.available_traits();
    assert_eq!(traits.len(), 7);
}

#[test]
fn http_handler_has_handler_service_kind() {
    let node = WorkflowNode::HttpHandler(HttpHandlerConfig::default());
    assert_eq!(node.service_kind(), ServiceKind::Handler);
}

#[test]
fn object_call_has_actor_service_kind() {
    let node = WorkflowNode::ObjectCall(ObjectCallConfig::default());
    assert_eq!(node.service_kind(), ServiceKind::Actor);
}

#[test]
fn workflow_call_has_workflow_service_kind() {
    let node = WorkflowNode::WorkflowCall(WorkflowCallConfig::default());
    assert_eq!(node.service_kind(), ServiceKind::Workflow);
}

#[test]
fn check_connection_happy_path_http_handler_to_http_call() {
    let source = WorkflowNode::HttpHandler(HttpHandlerConfig::default());
    let target = WorkflowNode::HttpCall(HttpCallConfig::default());
    assert!(check_connection(&source, &target).is_ok());
}

#[test]
fn get_node_by_id_find_existing_node() {
    let mut node1 = Node::from_workflow_node(
        "handler".into(),
        WorkflowNode::HttpHandler(HttpHandlerConfig::default()),
        0.0,
        0.0,
    );
    let id1 = NodeId::new();
    node1.id = id1;

    let mut node2 = Node::from_workflow_node(
        "run".into(),
        WorkflowNode::Run(oya_frontend::graph::RunConfig::default()),
        0.0,
        0.0,
    );
    let id2 = NodeId::new();
    node2.id = id2;

    let nodes = vec![node1, node2];
    assert!(get_node_by_id(id1, &nodes).is_ok());
}

#[test]
fn get_node_by_id_rejects_nonexistent() {
    let node1 = Node::from_workflow_node(
        "handler".into(),
        WorkflowNode::HttpHandler(HttpHandlerConfig::default()),
        0.0,
        0.0,
    );
    let nodes = vec![node1];

    let id = NodeId::new();
    assert!(matches!(get_node_by_id(id, &nodes), Err(ConnectionError::NodeNotFound { .. })));
}

#[test]
fn service_kind_from_str_rejects_invalid_string() {
    let result: Result<ServiceKind, _> = "invalid".parse();
    assert!(result.is_err());
}

#[test]
fn context_type_from_str_rejects_invalid_string() {
    let result: Result<ContextType, _> = "invalid".parse();
    assert!(result.is_err());
}

#[test]
fn types_compatible_any_universal_source_event() {
    assert!(types_compatible(PortType::Any, PortType::Event));
}

#[test]
fn types_compatible_json_universal_source_state() {
    assert!(types_compatible(PortType::Json, PortType::State));
}

#[test]
fn types_compatible_incompatible_event_state() {
    assert!(!types_compatible(PortType::Event, PortType::State));
}
