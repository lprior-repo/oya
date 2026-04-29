#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic, clippy::float_cmp)]
use anyhow::{anyhow, Result};
use oya_frontend::flow_extender::{
    apply_extension, preview_extension, suggest_extensions, suggest_extensions_with_analysis,
    RestateCapability, RestateServiceKind,
};
use oya_frontend::graph::Workflow;
use oya_frontend::graph::{
    connection_errors::{check_connection, get_node_by_id, ConnectionError},
    port_types::{types_compatible, PortType},
    service_kinds::{ClientType, ContextTrait, ContextType, ServiceKind},
    workflow_node::WorkflowNode,
    NodeId, PortName,
};

// ===========================================================================
// ServiceKind Tests
// ===========================================================================

#[test]
fn service_kind_parses_lowercase_handler() {
    let result: Result<ServiceKind, _> = "handler".parse();
    assert_eq!(result, Ok(ServiceKind::Handler));
}

#[test]
fn service_kind_parses_uppercase_handler() {
    let result: Result<ServiceKind, _> = "HANDLER".parse();
    assert_eq!(result, Ok(ServiceKind::Handler));
}

#[test]
fn service_kind_parses_mixed_case_handler() {
    let result: Result<ServiceKind, _> = "Handler".parse();
    assert_eq!(result, Ok(ServiceKind::Handler));
}

#[test]
fn service_kind_parses_lowercase_workflow() {
    let result: Result<ServiceKind, _> = "workflow".parse();
    assert_eq!(result, Ok(ServiceKind::Workflow));
}

#[test]
fn service_kind_parses_uppercase_workflow() {
    let result: Result<ServiceKind, _> = "WORKFLOW".parse();
    assert_eq!(result, Ok(ServiceKind::Workflow));
}

#[test]
fn service_kind_parses_lowercase_actor() {
    let result: Result<ServiceKind, _> = "actor".parse();
    assert_eq!(result, Ok(ServiceKind::Actor));
}

#[test]
fn service_kind_parses_uppercase_actor() {
    let result: Result<ServiceKind, _> = "ACTOR".parse();
    assert_eq!(result, Ok(ServiceKind::Actor));
}

#[test]
fn service_kind_from_str_rejects_invalid() {
    let result: Result<ServiceKind, _> = "invalid".parse();
    assert!(result.is_err());
    let err = result.unwrap_err();
    assert_eq!(
        err.to_string(),
        "Unknown ServiceKind 'invalid': expected one of handler, workflow, actor".to_string()
    );
}

#[test]
fn service_kind_display_handler() {
    assert_eq!(format!("{}", ServiceKind::Handler), "handler");
}

#[test]
fn service_kind_display_workflow() {
    assert_eq!(format!("{}", ServiceKind::Workflow), "workflow");
}

#[test]
fn service_kind_display_actor() {
    assert_eq!(format!("{}", ServiceKind::Actor), "actor");
}

#[test]
fn service_kind_display_from_str_roundtrip_handler() {
    let original = ServiceKind::Handler;
    let display = format!("{}", original);
    let parsed: ServiceKind = display.parse().unwrap();
    assert_eq!(original, parsed);
}

#[test]
fn service_kind_display_from_str_roundtrip_workflow() {
    let original = ServiceKind::Workflow;
    let display = format!("{}", original);
    let parsed: ServiceKind = display.parse().unwrap();
    assert_eq!(original, parsed);
}

#[test]
fn service_kind_display_from_str_roundtrip_actor() {
    let original = ServiceKind::Actor;
    let display = format!("{}", original);
    let parsed: ServiceKind = display.parse().unwrap();
    assert_eq!(original, parsed);
}

#[test]
fn service_kind_handler_supports_state_returns_false() {
    assert!(!ServiceKind::Handler.supports_state());
}

#[test]
fn service_kind_handler_supports_promises_returns_false() {
    assert!(!ServiceKind::Handler.supports_promises());
}

#[test]
fn service_kind_actor_supports_state_returns_true() {
    assert!(ServiceKind::Actor.supports_state());
}

#[test]
fn service_kind_actor_supports_promises_returns_false() {
    assert!(!ServiceKind::Actor.supports_promises());
}

#[test]
fn service_kind_workflow_supports_state_returns_true() {
    assert!(ServiceKind::Workflow.supports_state());
}

#[test]
fn service_kind_workflow_supports_promises_returns_true() {
    assert!(ServiceKind::Workflow.supports_promises());
}

#[test]
fn service_kind_handler_context_type_is_synchronous() {
    assert_eq!(ServiceKind::Handler.context_type(), ContextType::Synchronous);
}

#[test]
fn service_kind_actor_context_type_is_synchronous() {
    assert_eq!(ServiceKind::Actor.context_type(), ContextType::Synchronous);
}

#[test]
fn service_kind_workflow_context_type_is_asynchronous() {
    assert_eq!(ServiceKind::Workflow.context_type(), ContextType::Asynchronous);
}

#[test]
fn handler_available_clients_contains_only_service() {
    let clients = ServiceKind::Handler.available_clients();
    assert_eq!(clients.len(), 1);
    assert!(clients.contains(&ClientType::Service));
    assert!(!clients.contains(&ClientType::Object));
    assert!(!clients.contains(&ClientType::Workflow));
}

#[test]
fn actor_available_clients_contains_service_and_object() {
    let clients = ServiceKind::Actor.available_clients();
    assert_eq!(clients.len(), 2);
    assert!(clients.contains(&ClientType::Service));
    assert!(clients.contains(&ClientType::Object));
    assert!(!clients.contains(&ClientType::Workflow));
}

#[test]
fn workflow_available_clients_contains_service_object_and_workflow() {
    let clients = ServiceKind::Workflow.available_clients();
    assert_eq!(clients.len(), 3);
    assert!(clients.contains(&ClientType::Service));
    assert!(clients.contains(&ClientType::Object));
    assert!(clients.contains(&ClientType::Workflow));
}

#[test]
fn service_kind_serde_lowercases_handler() {
    let json = serde_json::to_string(&ServiceKind::Handler).unwrap();
    assert_eq!(json, "\"handler\"");
}

#[test]
fn service_kind_serde_lowercases_workflow() {
    let json = serde_json::to_string(&ServiceKind::Workflow).unwrap();
    assert_eq!(json, "\"workflow\"");
}

#[test]
fn service_kind_serde_lowercases_actor() {
    let json = serde_json::to_string(&ServiceKind::Actor).unwrap();
    assert_eq!(json, "\"actor\"");
}

#[test]
fn service_kind_serde_roundtrip_handler() {
    let json = r#""handler""#;
    let parsed: ServiceKind = serde_json::from_str(json).unwrap();
    assert_eq!(parsed, ServiceKind::Handler);
}

#[test]
fn service_kind_serde_roundtrip_workflow() {
    let json = r#""workflow""#;
    let parsed: ServiceKind = serde_json::from_str(json).unwrap();
    assert_eq!(parsed, ServiceKind::Workflow);
}

#[test]
fn service_kind_serde_roundtrip_actor() {
    let json = r#""actor""#;
    let parsed: ServiceKind = serde_json::from_str(json).unwrap();
    assert_eq!(parsed, ServiceKind::Actor);
}

#[test]
fn service_kind_boundary_empty_string_rejects() {
    let result: Result<ServiceKind, _> = "".parse();
    assert!(result.is_err());
}

#[test]
fn service_kind_boundary_1kb_string_rejects() {
    let long_string = "h".repeat(1024);
    let result: Result<ServiceKind, _> = long_string.parse();
    assert!(result.is_err());
}

#[test]
fn service_kind_boundary_truncated_string_rejects() {
    let truncated = "ha";
    let result: Result<ServiceKind, _> = truncated.parse();
    assert!(result.is_err());
}

// ===========================================================================
// ContextType Tests
// ===========================================================================

#[test]
fn context_type_parses_lowercase_synchronous() {
    let result: Result<ContextType, _> = "synchronous".parse();
    assert_eq!(result, Ok(ContextType::Synchronous));
}

#[test]
fn context_type_parses_uppercase_synchronous() {
    let result: Result<ContextType, _> = "SYNCHRONOUS".parse();
    assert_eq!(result, Ok(ContextType::Synchronous));
}

#[test]
fn context_type_parses_alias_sync_lowercase() {
    let result: Result<ContextType, _> = "sync".parse();
    assert_eq!(result, Ok(ContextType::Synchronous));
}

#[test]
fn context_type_parses_alias_sync_uppercase() {
    let result: Result<ContextType, _> = "SYNC".parse();
    assert_eq!(result, Ok(ContextType::Synchronous));
}

#[test]
fn context_type_parses_lowercase_asynchronous() {
    let result: Result<ContextType, _> = "asynchronous".parse();
    assert_eq!(result, Ok(ContextType::Asynchronous));
}

#[test]
fn context_type_parses_uppercase_asynchronous() {
    let result: Result<ContextType, _> = "ASYNCHRONOUS".parse();
    assert_eq!(result, Ok(ContextType::Asynchronous));
}

#[test]
fn context_type_parses_alias_async_lowercase() {
    let result: Result<ContextType, _> = "async".parse();
    assert_eq!(result, Ok(ContextType::Asynchronous));
}

#[test]
fn context_type_parses_alias_async_uppercase() {
    let result: Result<ContextType, _> = "ASYNC".parse();
    assert_eq!(result, Ok(ContextType::Asynchronous));
}

#[test]
fn context_type_from_str_rejects_invalid() {
    let result: Result<ContextType, _> = "invalid".parse();
    assert!(result.is_err());
    let err = result.unwrap_err();
    assert_eq!(
        err.to_string(),
        "Unknown ContextType 'invalid': expected one of synchronous, sync, asynchronous, async"
            .to_string()
    );
}

#[test]
fn context_type_synchronous_is_synchronous() {
    assert!(ContextType::Synchronous.is_synchronous());
}

#[test]
fn context_type_asynchronous_is_synchronous_returns_false() {
    assert!(!ContextType::Asynchronous.is_synchronous());
}

#[test]
fn context_type_synchronous_is_asynchronous_returns_false() {
    assert!(!ContextType::Synchronous.is_asynchronous());
}

#[test]
fn context_type_asynchronous_is_asynchronous() {
    assert!(ContextType::Asynchronous.is_asynchronous());
}

#[test]
fn context_type_display_synchronous() {
    assert_eq!(format!("{}", ContextType::Synchronous), "synchronous");
}

#[test]
fn context_type_display_asynchronous() {
    assert_eq!(format!("{}", ContextType::Asynchronous), "asynchronous");
}

#[test]
fn context_type_display_from_str_roundtrip_synchronous() {
    let original = ContextType::Synchronous;
    let display = format!("{}", original);
    let parsed: ContextType = display.parse().unwrap();
    assert_eq!(original, parsed);
}

#[test]
fn context_type_display_from_str_roundtrip_asynchronous() {
    let original = ContextType::Asynchronous;
    let display = format!("{}", original);
    let parsed: ContextType = display.parse().unwrap();
    assert_eq!(original, parsed);
}

#[test]
fn context_type_synchronous_available_traits_count() {
    let traits = ContextType::Synchronous.available_traits();
    assert_eq!(traits.len(), 6);
}

#[test]
fn context_type_asynchronous_available_traits_count() {
    let traits = ContextType::Asynchronous.available_traits();
    assert_eq!(traits.len(), 7);
}

#[test]
fn context_type_synchronous_available_traits_contains_client() {
    let traits = ContextType::Synchronous.available_traits();
    assert!(traits.contains(&ContextTrait::ContextClient));
}

#[test]
fn context_type_synchronous_available_traits_contains_promises_returns_false() {
    let traits = ContextType::Synchronous.available_traits();
    assert!(!traits.contains(&ContextTrait::ContextPromises));
}

#[test]
fn context_type_asynchronous_available_traits_contains_promises() {
    let traits = ContextType::Asynchronous.available_traits();
    assert!(traits.contains(&ContextTrait::ContextPromises));
}

#[test]
fn context_type_serde_lowercases_synchronous() {
    let json = serde_json::to_string(&ContextType::Synchronous).unwrap();
    assert_eq!(json, "\"synchronous\"");
}

#[test]
fn context_type_serde_lowercases_asynchronous() {
    let json = serde_json::to_string(&ContextType::Asynchronous).unwrap();
    assert_eq!(json, "\"asynchronous\"");
}

#[test]
fn context_type_serde_roundtrip_synchronous() {
    let json = r#""synchronous""#;
    let parsed: ContextType = serde_json::from_str(json).unwrap();
    assert_eq!(parsed, ContextType::Synchronous);
}

#[test]
fn context_type_serde_roundtrip_asynchronous() {
    let json = r#""asynchronous""#;
    let parsed: ContextType = serde_json::from_str(json).unwrap();
    assert_eq!(parsed, ContextType::Asynchronous);
}

#[test]
fn context_type_boundary_empty_string_rejects() {
    let result: Result<ContextType, _> = "".parse();
    assert!(result.is_err());
}

#[test]
fn context_type_boundary_1kb_string_rejects() {
    let long_string = "s".repeat(1024);
    let result: Result<ContextType, _> = long_string.parse();
    assert!(result.is_err());
}

#[test]
fn context_type_boundary_truncated_string_rejects() {
    let truncated = "sy";
    let result: Result<ContextType, _> = truncated.parse();
    assert!(result.is_err());
}

// ===========================================================================
// PortType Tests
// ===========================================================================

#[test]
fn port_type_default_returns_any() {
    let default: PortType = Default::default();
    assert_eq!(default, PortType::Any);
}

#[test]
fn port_type_parses_lowercase_any() {
    let result: Result<PortType, _> = "any".parse();
    assert_eq!(result, Ok(PortType::Any));
}

#[test]
fn port_type_parses_uppercase_any() {
    let result: Result<PortType, _> = "ANY".parse();
    assert_eq!(result, Ok(PortType::Any));
}

#[test]
fn port_type_parses_lowercase_event() {
    let result: Result<PortType, _> = "event".parse();
    assert_eq!(result, Ok(PortType::Event));
}

#[test]
fn port_type_parses_lowercase_state() {
    let result: Result<PortType, _> = "state".parse();
    assert_eq!(result, Ok(PortType::State));
}

#[test]
fn port_type_parses_lowercase_signal() {
    let result: Result<PortType, _> = "signal".parse();
    assert_eq!(result, Ok(PortType::Signal));
}

#[test]
fn port_type_parses_flowcontrol_hyphenated() {
    let result: Result<PortType, _> = "flow-control".parse();
    assert_eq!(result, Ok(PortType::FlowControl));
}

#[test]
fn port_type_parses_flowcontrol_hyphenated_uppercase() {
    let result: Result<PortType, _> = "FLOW-CONTROL".parse();
    assert_eq!(result, Ok(PortType::FlowControl));
}

#[test]
fn port_type_parses_flowcontrol_no_hyphen() {
    let result: Result<PortType, _> = "flowcontrol".parse();
    assert_eq!(result, Ok(PortType::FlowControl));
}

#[test]
fn port_type_parses_flowcontrol_no_hyphen_uppercase() {
    let result: Result<PortType, _> = "FLOWCONTROL".parse();
    assert_eq!(result, Ok(PortType::FlowControl));
}

#[test]
fn port_type_parses_lowercase_json() {
    let result: Result<PortType, _> = "json".parse();
    assert_eq!(result, Ok(PortType::Json));
}

#[test]
fn port_type_from_str_rejects_invalid() {
    let result: Result<PortType, _> = "invalid".parse();
    assert!(result.is_err());
    let err = result.unwrap_err();
    assert_eq!(
        err.to_string(),
        "Unknown PortType 'invalid': expected one of any, event, state, signal, flow-control, json"
            .to_string()
    );
}

#[test]
fn port_type_display_any() {
    assert_eq!(format!("{}", PortType::Any), "any");
}

#[test]
fn port_type_display_event() {
    assert_eq!(format!("{}", PortType::Event), "event");
}

#[test]
fn port_type_display_state() {
    assert_eq!(format!("{}", PortType::State), "state");
}

#[test]
fn port_type_display_signal() {
    assert_eq!(format!("{}", PortType::Signal), "signal");
}

#[test]
fn port_type_display_flow_control() {
    assert_eq!(format!("{}", PortType::FlowControl), "flow-control");
}

#[test]
fn port_type_display_json() {
    assert_eq!(format!("{}", PortType::Json), "json");
}

#[test]
fn port_type_display_from_str_roundtrip_any() {
    let original = PortType::Any;
    let display = format!("{}", original);
    let parsed: PortType = display.parse().unwrap();
    assert_eq!(original, parsed);
}

#[test]
fn port_type_display_from_str_roundtrip_event() {
    let original = PortType::Event;
    let display = format!("{}", original);
    let parsed: PortType = display.parse().unwrap();
    assert_eq!(original, parsed);
}

#[test]
fn port_type_display_from_str_roundtrip_state() {
    let original = PortType::State;
    let display = format!("{}", original);
    let parsed: PortType = display.parse().unwrap();
    assert_eq!(original, parsed);
}

#[test]
fn port_type_display_from_str_roundtrip_signal() {
    let original = PortType::Signal;
    let display = format!("{}", original);
    let parsed: PortType = display.parse().unwrap();
    assert_eq!(original, parsed);
}

#[test]
fn port_type_display_from_str_roundtrip_flow_control() {
    let original = PortType::FlowControl;
    let display = format!("{}", original);
    let parsed: PortType = display.parse().unwrap();
    assert_eq!(original, parsed);
}

#[test]
fn port_type_display_from_str_roundtrip_json() {
    let original = PortType::Json;
    let display = format!("{}", original);
    let parsed: PortType = display.parse().unwrap();
    assert_eq!(original, parsed);
}

#[test]
fn port_type_serde_lowercases_any() {
    let json = serde_json::to_string(&PortType::Any).unwrap();
    assert_eq!(json, "\"any\"");
}

#[test]
fn port_type_serde_lowercases_event() {
    let json = serde_json::to_string(&PortType::Event).unwrap();
    assert_eq!(json, "\"event\"");
}

#[test]
fn port_type_serde_lowercases_state() {
    let json = serde_json::to_string(&PortType::State).unwrap();
    assert_eq!(json, "\"state\"");
}

#[test]
fn port_type_serde_lowercases_signal() {
    let json = serde_json::to_string(&PortType::Signal).unwrap();
    assert_eq!(json, "\"signal\"");
}

#[test]
fn port_type_serde_lowercases_flow_control() {
    let json = serde_json::to_string(&PortType::FlowControl).unwrap();
    assert_eq!(json, "\"flow-control\"");
}

#[test]
fn port_type_serde_lowercases_json() {
    let json = serde_json::to_string(&PortType::Json).unwrap();
    assert_eq!(json, "\"json\"");
}

#[test]
fn port_type_serde_roundtrip_event() {
    let json = r#""event""#;
    let parsed: PortType = serde_json::from_str(json).unwrap();
    assert_eq!(parsed, PortType::Event);
}

#[test]
fn port_type_serde_roundtrip_state() {
    let json = r#""state""#;
    let parsed: PortType = serde_json::from_str(json).unwrap();
    assert_eq!(parsed, PortType::State);
}

#[test]
fn port_type_boundary_empty_string_rejects() {
    let result: Result<PortType, _> = "".parse();
    assert!(result.is_err());
}

#[test]
fn port_type_boundary_1kb_string_rejects() {
    let long_string = "a".repeat(1024);
    let result: Result<PortType, _> = long_string.parse();
    assert!(result.is_err());
}

#[test]
fn port_type_boundary_truncated_string_rejects() {
    let truncated = "an";
    let result: Result<PortType, _> = truncated.parse();
    assert!(result.is_err());
}

// ===========================================================================
// Port Type Compatibility Tests
// ===========================================================================

#[test]
fn types_compatible_any_matches_all() {
    assert!(types_compatible(PortType::Any, PortType::Event));
    assert!(types_compatible(PortType::Any, PortType::State));
    assert!(types_compatible(PortType::Any, PortType::Signal));
    assert!(types_compatible(PortType::Any, PortType::FlowControl));
    assert!(types_compatible(PortType::Any, PortType::Json));
    assert!(types_compatible(PortType::Any, PortType::Any));
}

#[test]
fn types_compatible_json_matches_all() {
    assert!(types_compatible(PortType::Json, PortType::Event));
    assert!(types_compatible(PortType::Json, PortType::State));
    assert!(types_compatible(PortType::Json, PortType::Signal));
    assert!(types_compatible(PortType::Json, PortType::FlowControl));
    assert!(types_compatible(PortType::Json, PortType::Any));
    assert!(types_compatible(PortType::Json, PortType::Json));
}

#[test]
fn types_compatible_specific_same_type_returns_true() {
    assert!(types_compatible(PortType::Event, PortType::Event));
    assert!(types_compatible(PortType::State, PortType::State));
    assert!(types_compatible(PortType::Signal, PortType::Signal));
    assert!(types_compatible(PortType::FlowControl, PortType::FlowControl));
    assert!(types_compatible(PortType::Json, PortType::Json));
    assert!(types_compatible(PortType::Any, PortType::Any));
}

#[test]
fn types_compatible_different_specific_types_returns_false() {
    assert!(!types_compatible(PortType::Event, PortType::State));
    assert!(!types_compatible(PortType::Event, PortType::Signal));
    assert!(!types_compatible(PortType::Event, PortType::FlowControl));
    assert!(!types_compatible(PortType::State, PortType::Event));
    assert!(!types_compatible(PortType::State, PortType::Signal));
    assert!(!types_compatible(PortType::State, PortType::FlowControl));
    assert!(!types_compatible(PortType::Signal, PortType::Event));
    assert!(!types_compatible(PortType::Signal, PortType::State));
    assert!(!types_compatible(PortType::Signal, PortType::FlowControl));
    assert!(!types_compatible(PortType::FlowControl, PortType::Event));
    assert!(!types_compatible(PortType::FlowControl, PortType::State));
    assert!(!types_compatible(PortType::FlowControl, PortType::Signal));
}

#[test]
fn port_type_compatibility_any_to_any_returns_true() {
    assert!(types_compatible(PortType::Any, PortType::Any));
}

#[test]
fn port_type_compatibility_any_to_event_returns_true() {
    assert!(types_compatible(PortType::Any, PortType::Event));
}

#[test]
fn port_type_compatibility_any_to_state_returns_true() {
    assert!(types_compatible(PortType::Any, PortType::State));
}

#[test]
fn port_type_compatibility_any_to_signal_returns_true() {
    assert!(types_compatible(PortType::Any, PortType::Signal));
}

#[test]
fn port_type_compatibility_any_to_flowcontrol_returns_true() {
    assert!(types_compatible(PortType::Any, PortType::FlowControl));
}

#[test]
fn port_type_compatibility_any_to_json_returns_true() {
    assert!(types_compatible(PortType::Any, PortType::Json));
}

#[test]
fn port_type_compatibility_event_to_any_returns_true() {
    assert!(types_compatible(PortType::Event, PortType::Any));
}

#[test]
fn port_type_compatibility_event_to_event_returns_true() {
    assert!(types_compatible(PortType::Event, PortType::Event));
}

#[test]
fn port_type_compatibility_event_to_state_returns_false() {
    assert!(!types_compatible(PortType::Event, PortType::State));
}

#[test]
fn port_type_compatibility_event_to_signal_returns_false() {
    assert!(!types_compatible(PortType::Event, PortType::Signal));
}

#[test]
fn port_type_compatibility_event_to_flowcontrol_returns_false() {
    assert!(!types_compatible(PortType::Event, PortType::FlowControl));
}

#[test]
fn port_type_compatibility_event_to_json_returns_true() {
    assert!(types_compatible(PortType::Event, PortType::Json));
}

#[test]
fn port_type_compatibility_state_to_any_returns_true() {
    assert!(types_compatible(PortType::State, PortType::Any));
}

#[test]
fn port_type_compatibility_state_to_event_returns_false() {
    assert!(!types_compatible(PortType::State, PortType::Event));
}

#[test]
fn port_type_compatibility_state_to_state_returns_true() {
    assert!(types_compatible(PortType::State, PortType::State));
}

#[test]
fn port_type_compatibility_state_to_signal_returns_false() {
    assert!(!types_compatible(PortType::State, PortType::Signal));
}

#[test]
fn port_type_compatibility_state_to_flowcontrol_returns_false() {
    assert!(!types_compatible(PortType::State, PortType::FlowControl));
}

#[test]
fn port_type_compatibility_state_to_json_returns_true() {
    assert!(types_compatible(PortType::State, PortType::Json));
}

#[test]
fn port_type_compatibility_signal_to_any_returns_true() {
    assert!(types_compatible(PortType::Signal, PortType::Any));
}

#[test]
fn port_type_compatibility_signal_to_event_returns_false() {
    assert!(!types_compatible(PortType::Signal, PortType::Event));
}

#[test]
fn port_type_compatibility_signal_to_state_returns_false() {
    assert!(!types_compatible(PortType::Signal, PortType::State));
}

#[test]
fn port_type_compatibility_signal_to_signal_returns_true() {
    assert!(types_compatible(PortType::Signal, PortType::Signal));
}

#[test]
fn port_type_compatibility_signal_to_flowcontrol_returns_false() {
    assert!(!types_compatible(PortType::Signal, PortType::FlowControl));
}

#[test]
fn port_type_compatibility_signal_to_json_returns_true() {
    assert!(types_compatible(PortType::Signal, PortType::Json));
}

#[test]
fn port_type_compatibility_flowcontrol_to_any_returns_true() {
    assert!(types_compatible(PortType::FlowControl, PortType::Any));
}

#[test]
fn port_type_compatibility_flowcontrol_to_event_returns_false() {
    assert!(!types_compatible(PortType::FlowControl, PortType::Event));
}

#[test]
fn port_type_compatibility_flowcontrol_to_state_returns_false() {
    assert!(!types_compatible(PortType::FlowControl, PortType::State));
}

#[test]
fn port_type_compatibility_flowcontrol_to_signal_returns_false() {
    assert!(!types_compatible(PortType::FlowControl, PortType::Signal));
}

#[test]
fn port_type_compatibility_flowcontrol_to_flowcontrol_returns_true() {
    assert!(types_compatible(PortType::FlowControl, PortType::FlowControl));
}

#[test]
fn port_type_compatibility_flowcontrol_to_json_returns_true() {
    assert!(types_compatible(PortType::FlowControl, PortType::Json));
}

#[test]
fn port_type_compatibility_json_to_any_returns_true() {
    assert!(types_compatible(PortType::Json, PortType::Any));
}

#[test]
fn port_type_compatibility_json_to_event_returns_true() {
    assert!(types_compatible(PortType::Json, PortType::Event));
}

#[test]
fn port_type_compatibility_json_to_state_returns_true() {
    assert!(types_compatible(PortType::Json, PortType::State));
}

#[test]
fn port_type_compatibility_json_to_signal_returns_true() {
    assert!(types_compatible(PortType::Json, PortType::Signal));
}

#[test]
fn port_type_compatibility_json_to_flowcontrol_returns_true() {
    assert!(types_compatible(PortType::Json, PortType::FlowControl));
}

#[test]
fn port_type_compatibility_json_to_json_returns_true() {
    assert!(types_compatible(PortType::Json, PortType::Json));
}

// ===========================================================================
// ConnectionError Tests
// ===========================================================================
// ConnectionError Tests
// ===========================================================================

#[test]
fn check_connection_returns_port_type_mismatch_when_event_to_state() {
    let source = WorkflowNode::CronTrigger(Default::default());
    let target = WorkflowNode::GetState(Default::default());
    let result = check_connection(&source, &target);
    assert!(result.is_err());
    let err = result.unwrap_err();
    assert_eq!(
        err,
        ConnectionError::PortTypeMismatch {
            source: PortType::Event,
            target: PortType::FlowControl
        }
    );
}

#[test]
fn check_connection_returns_context_type_mismatch_when_workflow_to_handler() {
    let source = WorkflowNode::DurablePromise(Default::default());
    let target = WorkflowNode::Run(Default::default());
    let result = check_connection(&source, &target);
    assert!(result.is_err());
    let err = result.unwrap_err();
    assert_eq!(
        err,
        ConnectionError::ContextTypeMismatch {
            source_context: ContextType::Asynchronous,
            target_context: ContextType::Synchronous
        }
    );
}

#[test]
fn check_connection_succeeds_for_valid_handler_to_handler() {
    let source = WorkflowNode::HttpHandler(Default::default());
    let target = WorkflowNode::Run(Default::default());
    let result = check_connection(&source, &target);
    assert_eq!(result, Ok(()));
}

#[test]
fn check_connection_succeeds_for_workflow_to_workflow() {
    let source = WorkflowNode::DurablePromise(Default::default());
    let target = WorkflowNode::ResolvePromise(Default::default());
    let result = check_connection(&source, &target);
    assert_eq!(result, Ok(()));
}

#[test]
fn connection_error_port_type_mismatch_display() {
    let err =
        ConnectionError::PortTypeMismatch { source: PortType::Event, target: PortType::State };
    let display = format!("{err}");
    assert!(display.contains("Port type mismatch"));
    assert!(display.contains("event"));
    assert!(display.contains("state"));
}

#[test]
fn connection_error_context_type_mismatch_display() {
    let err = ConnectionError::ContextTypeMismatch {
        source_context: ContextType::Asynchronous,
        target_context: ContextType::Synchronous,
    };
    let display = format!("{err}");
    assert!(display.contains("Context type mismatch"));
    assert!(display.contains("asynchronous"));
    assert!(display.contains("synchronous"));
}

#[test]
fn check_connection_impl_service_kind_incompatible_handler_to_actor() {
    let source = WorkflowNode::HttpCall(Default::default());
    let target = WorkflowNode::GetState(Default::default());
    let result = check_connection(&source, &target);
    assert_eq!(
        result,
        Err(ConnectionError::ServiceKindIncompatible {
            source_kind: ServiceKind::Handler,
            target_kind: ServiceKind::Actor,
            reason: "Handler cannot call Actor without state context"
        })
    );
}

// ===========================================================================
// get_node_by_id Tests
// ===========================================================================

#[test]
fn get_node_by_id_returns_exact_node_reference_when_found() {
    let mut workflow = Workflow::new();
    let test_id = workflow.add_node("run", 0.0, 0.0);

    let nodes: &[oya_frontend::graph::Node] = &workflow.nodes;
    let result = get_node_by_id(test_id, nodes);

    let node = result.expect("node should exist");
    assert_eq!(node.id, test_id);
}

#[test]
fn get_node_by_id_empty_slice_returns_node_not_found() {
    let nodes: &[oya_frontend::graph::Node] = &[];
    let test_id = NodeId::new();

    let result = get_node_by_id(test_id, nodes);
    let err = result.unwrap_err();
    assert_eq!(err, ConnectionError::NodeNotFound { node_id: test_id });
}

#[test]
fn get_node_by_id_exact_id_match_returns_node() {
    let mut workflow = Workflow::new();
    let node1_id = workflow.add_node("run", 0.0, 0.0);
    let node2_id = workflow.add_node("run", 10.0, 10.0);

    let nodes: &[oya_frontend::graph::Node] = &workflow.nodes;

    let result = get_node_by_id(node1_id, nodes);
    let node = result.expect("node1 should exist");
    assert_eq!(node.id, node1_id);

    let result = get_node_by_id(node2_id, nodes);
    let node = result.expect("node2 should exist");
    assert_eq!(node.id, node2_id);
}

#[test]
fn get_node_by_id_rejects_nonexistent() {
    let nodes: Vec<_> = Vec::new();
    let nonexistent_id = NodeId::new();

    let result = get_node_by_id(nonexistent_id, &nodes);
    let err = result.unwrap_err();
    assert_eq!(err, ConnectionError::NodeNotFound { node_id: nonexistent_id });
}

#[test]
fn get_node_by_id_first_match_wins() {
    let mut workflow = Workflow::new();
    workflow.add_node("run", 0.0, 0.0);
    let node_id = workflow.add_node("run", 10.0, 10.0);
    workflow.add_node("run", 20.0, 20.0);

    let nodes: &[oya_frontend::graph::Node] = &workflow.nodes;
    let result = get_node_by_id(node_id, nodes);
    let node = result.expect("node should exist");
    assert_eq!(node.id, node_id);
}

// ===========================================================================
// WorkflowNode ServiceKind Mapping Tests
// ===========================================================================

#[test]
fn http_handler_has_handler_service_kind() {
    assert_eq!(WorkflowNode::HttpHandler(Default::default()).service_kind(), ServiceKind::Handler);
}

#[test]
fn kafka_handler_has_handler_service_kind() {
    assert_eq!(WorkflowNode::KafkaHandler(Default::default()).service_kind(), ServiceKind::Handler);
}

#[test]
fn cron_trigger_has_handler_service_kind() {
    assert_eq!(WorkflowNode::CronTrigger(Default::default()).service_kind(), ServiceKind::Handler);
}

#[test]
fn signal_handler_has_handler_service_kind() {
    assert_eq!(
        WorkflowNode::SignalHandler(Default::default()).service_kind(),
        ServiceKind::Handler
    );
}

#[test]
fn http_call_has_handler_service_kind() {
    assert_eq!(WorkflowNode::HttpCall(Default::default()).service_kind(), ServiceKind::Handler);
}

#[test]
fn service_call_has_handler_service_kind() {
    assert_eq!(WorkflowNode::ServiceCall(Default::default()).service_kind(), ServiceKind::Handler);
}

#[test]
fn run_has_handler_service_kind() {
    assert_eq!(WorkflowNode::Run(Default::default()).service_kind(), ServiceKind::Handler);
}

#[test]
fn send_message_has_handler_service_kind() {
    assert_eq!(WorkflowNode::SendMessage(Default::default()).service_kind(), ServiceKind::Handler);
}

#[test]
fn delayed_send_has_handler_service_kind() {
    assert_eq!(WorkflowNode::DelayedSend(Default::default()).service_kind(), ServiceKind::Handler);
}

#[test]
fn condition_has_handler_service_kind() {
    assert_eq!(WorkflowNode::Condition(Default::default()).service_kind(), ServiceKind::Handler);
}

#[test]
fn switch_has_handler_service_kind() {
    assert_eq!(WorkflowNode::Switch(Default::default()).service_kind(), ServiceKind::Handler);
}

#[test]
fn loop_has_handler_service_kind() {
    assert_eq!(WorkflowNode::Loop(Default::default()).service_kind(), ServiceKind::Handler);
}

#[test]
fn parallel_has_handler_service_kind() {
    assert_eq!(WorkflowNode::Parallel(Default::default()).service_kind(), ServiceKind::Handler);
}

#[test]
fn compensate_has_handler_service_kind() {
    assert_eq!(WorkflowNode::Compensate(Default::default()).service_kind(), ServiceKind::Handler);
}

#[test]
fn sleep_has_handler_service_kind() {
    assert_eq!(WorkflowNode::Sleep(Default::default()).service_kind(), ServiceKind::Handler);
}

#[test]
fn timeout_has_handler_service_kind() {
    assert_eq!(WorkflowNode::Timeout(Default::default()).service_kind(), ServiceKind::Handler);
}

#[test]
fn workflow_call_has_workflow_service_kind() {
    assert_eq!(
        WorkflowNode::WorkflowCall(Default::default()).service_kind(),
        ServiceKind::Workflow
    );
}

#[test]
fn workflow_submit_has_workflow_service_kind() {
    assert_eq!(
        WorkflowNode::WorkflowSubmit(Default::default()).service_kind(),
        ServiceKind::Workflow
    );
}

#[test]
fn durable_promise_has_workflow_service_kind() {
    assert_eq!(
        WorkflowNode::DurablePromise(Default::default()).service_kind(),
        ServiceKind::Workflow
    );
}

#[test]
fn awakeable_has_workflow_service_kind() {
    assert_eq!(WorkflowNode::Awakeable(Default::default()).service_kind(), ServiceKind::Workflow);
}

#[test]
fn resolve_promise_has_workflow_service_kind() {
    assert_eq!(
        WorkflowNode::ResolvePromise(Default::default()).service_kind(),
        ServiceKind::Workflow
    );
}

#[test]
fn object_call_has_actor_service_kind() {
    assert_eq!(WorkflowNode::ObjectCall(Default::default()).service_kind(), ServiceKind::Actor);
}

#[test]
fn get_state_has_actor_service_kind() {
    assert_eq!(WorkflowNode::GetState(Default::default()).service_kind(), ServiceKind::Actor);
}

#[test]
fn set_state_has_actor_service_kind() {
    assert_eq!(WorkflowNode::SetState(Default::default()).service_kind(), ServiceKind::Actor);
}

#[test]
fn clear_state_has_actor_service_kind() {
    assert_eq!(WorkflowNode::ClearState(Default::default()).service_kind(), ServiceKind::Actor);
}

// ===========================================================================
// WorkflowNode ContextType Tests
// ===========================================================================

#[test]
fn http_handler_context_type_is_synchronous() {
    assert_eq!(
        WorkflowNode::HttpHandler(Default::default()).context_type(),
        ContextType::Synchronous
    );
}

#[test]
fn workflow_call_context_type_is_asynchronous() {
    assert_eq!(
        WorkflowNode::WorkflowCall(Default::default()).context_type(),
        ContextType::Asynchronous
    );
}

#[test]
fn object_call_context_type_is_synchronous() {
    assert_eq!(
        WorkflowNode::ObjectCall(Default::default()).context_type(),
        ContextType::Synchronous
    );
}

// ===========================================================================
// WorkflowNode Port Type Tests
// ===========================================================================

#[test]
fn http_handler_output_port_type_is_json() {
    assert_eq!(WorkflowNode::HttpHandler(Default::default()).output_port_type(), PortType::Json);
}

#[test]
fn http_handler_input_port_type_is_json() {
    assert_eq!(WorkflowNode::HttpHandler(Default::default()).input_port_type(), PortType::Json);
}

#[test]
fn cron_trigger_output_port_type_is_event() {
    assert_eq!(WorkflowNode::CronTrigger(Default::default()).output_port_type(), PortType::Event);
}

#[test]
fn signal_handler_output_port_type_is_signal() {
    assert_eq!(
        WorkflowNode::SignalHandler(Default::default()).output_port_type(),
        PortType::Signal
    );
}

#[test]
fn run_output_port_type_is_flow_control() {
    assert_eq!(WorkflowNode::Run(Default::default()).output_port_type(), PortType::FlowControl);
}

#[test]
fn run_input_port_type_is_flow_control() {
    assert_eq!(WorkflowNode::Run(Default::default()).input_port_type(), PortType::FlowControl);
}

// ===========================================================================
// Extension Preview Tests with Exact Node Structure Validation
// ===========================================================================

#[test]
fn entry_trigger_preview_creates_node_with_exact_coordinates() {
    let workflow = Workflow::new();
    let preview = preview_extension(&workflow, "add-entry-trigger")
        .map_err(|err| anyhow!(err))
        .unwrap()
        .expect("entry preview should exist");

    assert_eq!(preview.nodes.len(), 1);
    assert_eq!(preview.nodes[0].node_type, "http-handler");
    assert_eq!(preview.nodes[0].x, 120.0);
    assert_eq!(preview.nodes[0].y, 100.0);
}

#[test]
fn timeout_guard_creates_connection_with_exact_ports() {
    let mut workflow = Workflow::new();
    let anchor_id = workflow.add_node("run", 10.0, 10.0);

    let preview = preview_extension(&workflow, "add-timeout-guard")
        .map_err(|err| anyhow!(err))
        .unwrap()
        .expect("timeout preview should exist");

    assert_eq!(preview.nodes.len(), 1);
    assert_eq!(preview.nodes[0].node_type, "timeout");
    assert_eq!(preview.connections.len(), 1);
    assert_eq!(preview.connections[0].source_port, "out");
    assert_eq!(preview.connections[0].target_port, "in");

    // Verify exact source and target structure
    match &preview.connections[0].source {
        oya_frontend::flow_extender::PreviewEndpoint::Existing(id) => {
            assert_eq!(id, &anchor_id);
        }
        _ => panic!("Expected Existing endpoint"),
    }
}

#[test]
fn durable_checkpoint_creates_connection_with_exact_ports() {
    let mut workflow = Workflow::new();
    workflow.add_node("http-handler", 0.0, 0.0);
    workflow.add_node("get-state", 10.0, 10.0);
    let _anchor_id = workflow.add_node("run", 30.0, 30.0);

    let preview = preview_extension(&workflow, "add-durable-checkpoint")
        .map_err(|err| anyhow!(err))
        .unwrap()
        .expect("checkpoint preview should exist");

    assert_eq!(preview.nodes[0].node_type, "set-state");
    assert_eq!(preview.connections.len(), 1);
    assert_eq!(preview.connections[0].source_port, "out");
    assert_eq!(preview.connections[0].target_port, "in");

    // Verify exact target structure
    match &preview.connections[0].target {
        oya_frontend::flow_extender::PreviewEndpoint::Proposed(id) => {
            assert_eq!(id, "new-0");
        }
        _ => panic!("Expected Proposed endpoint"),
    }
}

#[test]
fn compensation_branch_creates_connection_with_exact_source_port_false() {
    let mut workflow = Workflow::new();
    workflow.add_node("durable-promise", 40.0, 90.0);
    let condition_id = workflow.add_node("condition", 100.0, 100.0);
    let _ = workflow.add_connection(
        condition_id,
        NodeId::new(),
        &PortName::from("true"),
        &PortName::from("in"),
    );

    let preview = preview_extension(&workflow, "add-compensation-branch")
        .map_err(|err| anyhow!(err))
        .unwrap()
        .expect("compensate preview should exist");

    assert_eq!(preview.nodes[0].node_type, "compensate");
    assert_eq!(preview.connections.len(), 1);
    assert_eq!(preview.connections[0].source_port, "false");
    assert_eq!(preview.connections[0].target_port, "in");
}

// ===========================================================================
// Workflow Tests
// ===========================================================================

#[test]
fn restate_semantic_tags_contract() -> Result<()> {
    let workflow = Workflow::new();

    let analysis = suggest_extensions_with_analysis(&workflow)
        .into_iter()
        .find(|item| item.key == "add-entry-trigger")
        .ok_or_else(|| anyhow!("entry analysis should exist"))?;

    assert!(analysis.semantics.compatible_service_kinds.contains(&RestateServiceKind::Handler));
    assert!(analysis.semantics.compatible_service_kinds.contains(&RestateServiceKind::Actor));
    assert!(analysis.semantics.compatible_service_kinds.contains(&RestateServiceKind::Workflow));
    assert!(analysis.semantics.provides.contains(&RestateCapability::EntryTrigger));
    Ok(())
}

#[test]
fn restate_semantic_guardrails_contract() -> Result<()> {
    let mut workflow = Workflow::new();
    workflow.add_node("run", 20.0, 20.0);
    workflow.add_node("condition", 60.0, 20.0);
    workflow.add_node("awakeable", 100.0, 20.0);

    let keys = suggest_extensions(&workflow).into_iter().map(|item| item.key).collect::<Vec<_>>();

    assert!(!keys.iter().any(|key| key == "add-durable-checkpoint"));
    assert!(!keys.iter().any(|key| key == "add-compensation-branch"));
    assert!(!keys.iter().any(|key| key == "add-signal-resolution"));
    Ok(())
}

#[test]
fn entry_trigger_contract() -> Result<()> {
    let mut workflow = Workflow::new();
    let run_id = workflow.add_node("run", 0.0, 0.0);

    let preview = preview_extension(&workflow, "add-entry-trigger")
        .map_err(|err| anyhow!(err))?
        .ok_or_else(|| anyhow!("entry preview should exist"))?;
    assert_eq!(preview.nodes.len(), 1);
    assert_eq!(preview.nodes[0].node_type, "http-handler");

    let initial_connections = workflow.connections.clone();
    let apply =
        apply_extension(&mut workflow, "add-entry-trigger").map_err(|err| anyhow!("{err}"))?;

    assert!(!apply.created_nodes.is_empty());
    assert!(workflow.nodes.iter().any(|node| node.node_type == "http-handler"));
    assert!(workflow.nodes.iter().any(|node| node.id == run_id));
    assert_eq!(workflow.connections, initial_connections);
    Ok(())
}

#[test]
fn timeout_guard_contract() -> Result<()> {
    let mut workflow = Workflow::new();
    let durable_id = workflow.add_node("run", 10.0, 10.0);

    let preview = preview_extension(&workflow, "add-timeout-guard")
        .map_err(|err| anyhow!(err))?
        .ok_or_else(|| anyhow!("timeout preview should exist"))?;
    assert_eq!(preview.nodes[0].node_type, "timeout");
    assert_eq!(preview.connections.len(), 1);

    apply_extension(&mut workflow, "add-timeout-guard").map_err(|err| anyhow!("{err}"))?;

    assert!(workflow.nodes.iter().any(|node| node.node_type == "timeout"));
    assert!(workflow.connections.iter().any(|connection| {
        connection.source == durable_id && connection.source_port.0 == "out"
    }));
    assert!(workflow.connections.iter().all(|connection| connection.source != connection.target));
    Ok(())
}

#[test]
fn durable_checkpoint_contract() -> Result<()> {
    let mut workflow = Workflow::new();
    workflow.add_node("get-state", 0.0, 0.0);
    let durable_id = workflow.add_node("run", 30.0, 30.0);

    let preview = preview_extension(&workflow, "add-durable-checkpoint")
        .map_err(|err| anyhow!(err))?
        .ok_or_else(|| anyhow!("checkpoint preview should exist"))?;
    assert_eq!(preview.nodes[0].node_type, "set-state");

    let initial_nodes = workflow.nodes.clone();
    let _ =
        apply_extension(&mut workflow, "add-durable-checkpoint").map_err(|err| anyhow!("{err}"))?;

    assert!(workflow.nodes.iter().any(|node| node.node_type == "set-state"));
    assert!(initial_nodes
        .iter()
        .all(|existing| workflow.nodes.iter().any(|node| node.id == existing.id)));
    assert!(workflow
        .connections
        .iter()
        .any(|connection| { connection.source == durable_id && connection.target_port.0 == "in" }));
    Ok(())
}

#[test]
fn compensation_branch_contract() -> Result<()> {
    let mut workflow = Workflow::new();
    workflow.add_node("durable-promise", 40.0, 90.0);
    let condition_id = workflow.add_node("condition", 100.0, 100.0);
    let true_branch = workflow.add_node("run", 240.0, 60.0);
    let _ = workflow.add_connection(
        condition_id,
        true_branch,
        &PortName::from("true"),
        &PortName::from("in"),
    );

    let preview = preview_extension(&workflow, "add-compensation-branch")
        .map_err(|err| anyhow!(err))?
        .ok_or_else(|| anyhow!("compensate preview should exist"))?;
    assert_eq!(preview.nodes[0].node_type, "compensate");
    assert_eq!(preview.connections[0].source_port, "false");

    apply_extension(&mut workflow, "add-compensation-branch").map_err(|err| anyhow!("{err}"))?;

    let compensate_node_id = workflow
        .nodes
        .iter()
        .find(|node| node.node_type == "compensate")
        .map(|node| node.id)
        .ok_or_else(|| anyhow!("compensate node should exist"))?;
    assert!(workflow.connections.iter().any(|connection| {
        connection.source == condition_id
            && connection.target == compensate_node_id
            && connection.source_port.0 == "false"
    }));
    Ok(())
}

#[test]
fn signal_resolution_contract() -> Result<()> {
    let mut workflow = Workflow::new();
    let wait_id = workflow.add_node("durable-promise", 50.0, 75.0);
    let durable_id = workflow.add_node("run", 280.0, 75.0);

    let before_order = workflow.nodes.iter().map(|node| node.id).collect::<Vec<_>>();
    let preview = preview_extension(&workflow, "add-signal-resolution")
        .map_err(|err| anyhow!(err))?
        .ok_or_else(|| anyhow!("resolve preview should exist"))?;
    assert_eq!(preview.nodes[0].node_type, "resolve-promise");

    let _ =
        apply_extension(&mut workflow, "add-signal-resolution").map_err(|err| anyhow!("{err}"))?;

    assert!(workflow.nodes.iter().any(|node| node.node_type == "resolve-promise"));
    assert!(workflow.connections.iter().any(|connection| {
        connection.source == wait_id
            && connection.source_port.0 == "out"
            && connection.target_port.0 == "in"
    }));
    let after_order = workflow.nodes.iter().map(|node| node.id).collect::<Vec<_>>();
    assert_eq!(after_order[0], before_order[0]);
    assert_eq!(after_order[1], durable_id);
    Ok(())
}

#[test]
fn reliability_bundle_preview_contract_respects_service_semantics() -> Result<()> {
    let mut workflow = Workflow::new();
    workflow.add_node("run", 32.0, 32.0);

    let preview = preview_extension(&workflow, "add-reliability-bundle")
        .map_err(|err| anyhow!(err))?
        .ok_or_else(|| anyhow!("bundle preview should exist"))?;

    assert!(preview.nodes.iter().all(|node| node.node_type != "set-state"));
    Ok(())
}

#[test]
fn reliability_bundle_preview_apply_contract_match_in_service_context() -> Result<()> {
    let mut workflow = Workflow::new();
    workflow.add_node("run", 48.0, 48.0);

    let preview = preview_extension(&workflow, "add-reliability-bundle")
        .map_err(|err| anyhow!(err))?
        .ok_or_else(|| anyhow!("bundle preview should exist"))?;
    let preview_types = preview.nodes.iter().map(|node| node.node_type.clone()).collect::<Vec<_>>();

    apply_extension(&mut workflow, "add-reliability-bundle").map_err(|err| anyhow!(err))?;
    let applied_types = workflow
        .nodes
        .iter()
        .filter(|node| {
            node.node_type == "timeout"
                || node.node_type == "set-state"
                || node.node_type == "compensate"
        })
        .map(|node| node.node_type.clone())
        .collect::<Vec<_>>();

    assert_eq!(applied_types, preview_types);
    Ok(())
}

#[test]
fn awakeable_signal_resolution_contract() -> Result<()> {
    let mut workflow = Workflow::new();
    let awakeable_id = workflow.add_node("awakeable", 120.0, 64.0);
    let run_id = workflow.add_node("run", 360.0, 64.0);
    let _ = workflow.add_connection(
        awakeable_id,
        run_id,
        &PortName::from("out"),
        &PortName::from("in"),
    );

    let preview = preview_extension(&workflow, "add-signal-resolution")
        .map_err(|err| anyhow!(err))?
        .ok_or_else(|| anyhow!("awakeable signal-resolution preview should exist"))?;
    assert_eq!(preview.nodes.len(), 1);
    assert_eq!(preview.nodes[0].node_type, "resolve-promise");
    assert!(preview
        .connections
        .iter()
        .any(|connection| { connection.source_port == "out" && connection.target_port == "in" }));

    apply_extension(&mut workflow, "add-signal-resolution").map_err(|err| anyhow!(err))?;

    let resolve_id = workflow
        .nodes
        .iter()
        .find(|node| node.node_type == "resolve-promise")
        .map(|node| node.id)
        .ok_or_else(|| anyhow!("resolve-promise node should exist"))?;
    assert!(workflow.connections.iter().any(|connection| {
        connection.source == awakeable_id
            && connection.target == resolve_id
            && connection.source_port.0 == "out"
            && connection.target_port.0 == "in"
    }));
    Ok(())
}
