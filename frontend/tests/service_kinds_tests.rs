//! Comprehensive tests for service_kinds module.
//!
//! Covers: context_type mapping, sync/async detection, client availability,
//! trait availability, serde roundtrips, and parse/display inverses.
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use oya_frontend::graph::service_kinds::{ClientType, ContextTrait, ContextType, ServiceKind};

// ===========================================================================
// ServiceKind::context_type
// ===========================================================================

#[test]
fn handler_returns_synchronous_context() {
    assert_eq!(ServiceKind::Handler.context_type(), ContextType::Synchronous);
}

#[test]
fn actor_returns_synchronous_context() {
    assert_eq!(ServiceKind::Actor.context_type(), ContextType::Synchronous);
}

#[test]
fn workflow_returns_asynchronous_context() {
    assert_eq!(ServiceKind::Workflow.context_type(), ContextType::Asynchronous);
}

// ===========================================================================
// Synchronous vs Asynchronous detection
// ===========================================================================

#[test]
fn synchronous_context_is_synchronous() {
    assert!(ContextType::Synchronous.is_synchronous());
    assert!(!ContextType::Synchronous.is_asynchronous());
}

#[test]
fn asynchronous_context_is_asynchronous() {
    assert!(ContextType::Asynchronous.is_asynchronous());
    assert!(!ContextType::Asynchronous.is_synchronous());
}

#[test]
fn handler_and_actor_are_synchronous() {
    assert!(ServiceKind::Handler.context_type().is_synchronous());
    assert!(ServiceKind::Actor.context_type().is_synchronous());
}

#[test]
fn workflow_is_asynchronous() {
    assert!(ServiceKind::Workflow.context_type().is_asynchronous());
}

// ===========================================================================
// Client availability
// ===========================================================================

#[test]
fn handler_has_only_service_client() {
    assert_eq!(ServiceKind::Handler.available_clients(), &[ClientType::Service]);
}

#[test]
fn actor_has_service_and_object_clients() {
    assert_eq!(ServiceKind::Actor.available_clients(), &[ClientType::Service, ClientType::Object]);
}

#[test]
fn workflow_has_all_three_clients() {
    assert_eq!(
        ServiceKind::Workflow.available_clients(),
        &[ClientType::Service, ClientType::Object, ClientType::Workflow]
    );
}

#[test]
fn no_service_kind_has_empty_clients() {
    for kind in [ServiceKind::Handler, ServiceKind::Actor, ServiceKind::Workflow] {
        assert!(!kind.available_clients().is_empty(), "{kind:?} has no clients");
    }
}

// ===========================================================================
// Trait availability
// ===========================================================================

#[test]
fn synchronous_has_six_traits() {
    let traits = ContextType::Synchronous.available_traits();
    assert_eq!(traits.len(), 6);
}

#[test]
fn asynchronous_has_seven_traits() {
    let traits = ContextType::Asynchronous.available_traits();
    assert_eq!(traits.len(), 7);
}

#[test]
fn synchronous_traits_include_core_set() {
    let traits = ContextType::Synchronous.available_traits();
    assert!(traits.contains(&ContextTrait::ContextClient));
    assert!(traits.contains(&ContextTrait::ContextTimers));
    assert!(traits.contains(&ContextTrait::ContextSideEffects));
    assert!(traits.contains(&ContextTrait::ContextAwakeables));
    assert!(traits.contains(&ContextTrait::ContextReadState));
    assert!(traits.contains(&ContextTrait::ContextWriteState));
}

#[test]
fn asynchronous_traits_add_promise_on_top_of_synchronous() {
    let sync_traits = ContextType::Synchronous.available_traits();
    let async_traits = ContextType::Asynchronous.available_traits();
    for t in sync_traits {
        assert!(async_traits.contains(t), "async should contain {t:?}");
    }
    assert!(async_traits.contains(&ContextTrait::ContextPromises));
}

#[test]
fn synchronous_does_not_have_promises_trait() {
    let traits = ContextType::Synchronous.available_traits();
    assert!(!traits.contains(&ContextTrait::ContextPromises));
}

// ===========================================================================
// Supports state and promises
// ===========================================================================

#[test]
fn handler_does_not_support_state() {
    assert!(!ServiceKind::Handler.supports_state());
}

#[test]
fn workflow_supports_state() {
    assert!(ServiceKind::Workflow.supports_state());
}

#[test]
fn actor_supports_state() {
    assert!(ServiceKind::Actor.supports_state());
}

#[test]
fn only_workflow_supports_promises() {
    assert!(ServiceKind::Workflow.supports_promises());
    assert!(!ServiceKind::Handler.supports_promises());
    assert!(!ServiceKind::Actor.supports_promises());
}

// ===========================================================================
// Serde roundtrips
// ===========================================================================

#[test]
fn service_kind_serde_roundtrip() {
    for kind in [ServiceKind::Handler, ServiceKind::Workflow, ServiceKind::Actor] {
        let json = serde_json::to_string(&kind).unwrap();
        let parsed: ServiceKind = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed, kind);
    }
}

#[test]
fn context_type_serde_roundtrip() {
    for ct in [ContextType::Synchronous, ContextType::Asynchronous] {
        let json = serde_json::to_string(&ct).unwrap();
        let parsed: ContextType = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed, ct);
    }
}

#[test]
fn service_kind_serializes_lowercase() {
    assert_eq!(serde_json::to_string(&ServiceKind::Handler).unwrap(), "\"handler\"");
    assert_eq!(serde_json::to_string(&ServiceKind::Workflow).unwrap(), "\"workflow\"");
    assert_eq!(serde_json::to_string(&ServiceKind::Actor).unwrap(), "\"actor\"");
}

#[test]
fn context_type_serializes_lowercase() {
    assert_eq!(serde_json::to_string(&ContextType::Synchronous).unwrap(), "\"synchronous\"");
    assert_eq!(serde_json::to_string(&ContextType::Asynchronous).unwrap(), "\"asynchronous\"");
}

// ===========================================================================
// Parse / Display inverses
// ===========================================================================

#[test]
fn service_kind_display_then_parse_roundtrip() {
    for kind in [ServiceKind::Handler, ServiceKind::Workflow, ServiceKind::Actor] {
        let s = format!("{kind}");
        let parsed: ServiceKind = s.parse().unwrap();
        assert_eq!(parsed, kind);
    }
}

#[test]
fn context_type_display_then_parse_roundtrip() {
    for ct in [ContextType::Synchronous, ContextType::Asynchronous] {
        let s = format!("{ct}");
        let parsed: ContextType = s.parse().unwrap();
        assert_eq!(parsed, ct);
    }
}

#[test]
fn context_type_parses_sync_alias() {
    let result: Result<ContextType, _> = "sync".parse();
    assert_eq!(result, Ok(ContextType::Synchronous));
}

#[test]
fn context_type_parses_async_alias() {
    let result: Result<ContextType, _> = "async".parse();
    assert_eq!(result, Ok(ContextType::Asynchronous));
}

#[test]
fn service_kind_parse_rejects_empty() {
    let result: Result<ServiceKind, _> = "".parse();
    assert!(result.is_err());
}

#[test]
fn context_type_parse_rejects_empty() {
    let result: Result<ContextType, _> = "".parse();
    assert!(result.is_err());
}
