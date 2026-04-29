#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic, clippy::float_cmp)]
use oya_frontend::flow_extender::{
    apply_extension, detect_extension_conflicts, preview_extension, ConflictKind,
};
use oya_frontend::graph::Workflow;

#[test]
fn bead_required_unknown_extension_returns_structured_error() {
    let workflow = Workflow::new();

    let result = preview_extension(&workflow, "not-a-real-extension");

    assert!(matches!(result, Err(_)));
    if let Err(err) = result {
        assert!(err.contains("Unknown extension key"));
    }
}

#[test]
fn bead_required_conflict_detection_returns_diagnostics_without_mutation() {
    let mut workflow = Workflow::new();
    workflow.add_node("run", 10.0, 10.0);
    workflow.add_node("condition", 40.0, 10.0);
    workflow.add_node("awakeable", 70.0, 10.0);

    let before_nodes = workflow.nodes.clone();
    let before_connections = workflow.connections.clone();
    let keys = vec!["add-signal-resolution".to_string()];

    let conflicts = detect_extension_conflicts(&workflow, &keys).unwrap_or_default();

    assert!(conflicts
        .iter()
        .any(|conflict| conflict.kind == ConflictKind::WorkflowSemanticMismatch));
    assert_eq!(workflow.nodes, before_nodes);
    assert_eq!(workflow.connections, before_connections);
}

#[test]
fn bead_required_repeated_apply_is_idempotent() {
    let mut workflow = Workflow::new();
    workflow.add_node("run", 20.0, 20.0);

    let first = apply_extension(&mut workflow, "add-timeout-guard");
    match first {
        Ok(applied) => assert!(!applied.created_nodes.is_empty()),
        Err(e) => panic!("Expected Ok, got Err: {e}"),
    }

    let node_count_after_first = workflow.nodes.len();
    let connection_count_after_first = workflow.connections.len();

    let second = apply_extension(&mut workflow, "add-timeout-guard");
    match second {
        Ok(applied) => assert!(applied.created_nodes.is_empty()),
        Err(e) => panic!("Expected Ok, got Err: {e}"),
    }

    assert_eq!(workflow.nodes.len(), node_count_after_first);
    assert_eq!(workflow.connections.len(), connection_count_after_first);
}
