//! Scenario tests for flow_extender extension bundles.
//!
//! Covers: preview diff, apply idempotency, awakeable signal resolution,
//! and rejection clearing. Scenarios derived from specs/scenarios/flow_extender/.
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use oya_frontend::flow_extender::{apply_extension, preview_extension, suggest_extensions};
use oya_frontend::graph::{PortName, Workflow};

// ---------------------------------------------------------------------------
// Scenario: ext-preview — Preview produces correct diff before mutation
// Ref: specs/scenarios/flow_extender/extension_preview.yaml
// ---------------------------------------------------------------------------

#[test]
fn extension_preview_produces_correct_diff() {
    let mut workflow = Workflow::new();
    let run_id = workflow.add_node("run", 0.0, 0.0);

    let preview = preview_extension(&workflow, "add-entry-trigger")
        .expect("preview should not error")
        .expect("add-entry-trigger should be available");

    assert_eq!(preview.nodes.len(), 1, "preview should show exactly one new node");
    assert_eq!(preview.nodes[0].node_type, "http-handler");

    // Preview must not mutate the workflow
    assert_eq!(workflow.nodes.len(), 1, "workflow should still have exactly the original node");
    assert_eq!(workflow.nodes[0].id, run_id);
    assert!(workflow.connections.is_empty());
}

// ---------------------------------------------------------------------------
// Scenario: ext-apply-idempotent — Applying twice does not duplicate nodes
// Ref: specs/scenarios/flow_extender/extension_apply_idempotent.yaml
// ---------------------------------------------------------------------------

#[test]
fn extension_apply_idempotent_no_duplicate_nodes() {
    let mut workflow = Workflow::new();
    workflow.add_node("run", 10.0, 10.0);

    let first =
        apply_extension(&mut workflow, "add-timeout-guard").expect("first apply should succeed");
    assert!(!first.created_nodes.is_empty(), "first apply should create nodes");

    let node_count_after_first = workflow.nodes.len();
    let conn_count_after_first = workflow.connections.len();

    let second =
        apply_extension(&mut workflow, "add-timeout-guard").expect("second apply should succeed");
    assert!(
        second.created_nodes.is_empty(),
        "second apply should create no new nodes (idempotent)"
    );

    assert_eq!(
        workflow.nodes.len(),
        node_count_after_first,
        "node count should not change on re-apply"
    );
    assert_eq!(
        workflow.connections.len(),
        conn_count_after_first,
        "connection count should not change on re-apply"
    );
}

// ---------------------------------------------------------------------------
// Scenario: ext-awakeable-signal — Awakeable signal resolution works
// Ref: specs/scenarios/flow_extender/extension_awakeable_signal_resolution.yaml
// ---------------------------------------------------------------------------

#[test]
fn extension_awakeable_signal_resolution() {
    let mut workflow = Workflow::new();
    let awakeable_id = workflow.add_node("awakeable", 120.0, 64.0);
    let run_id = workflow.add_node("run", 360.0, 64.0);
    workflow
        .add_connection(awakeable_id, run_id, &PortName::from("out"), &PortName::from("in"))
        .expect("connection should be valid");

    // Preview should exist for awakeable
    let preview = preview_extension(&workflow, "add-signal-resolution")
        .expect("preview should not error")
        .expect("signal-resolution should be suggested for awakeable");
    assert_eq!(preview.nodes.len(), 1);
    assert_eq!(preview.nodes[0].node_type, "resolve-promise");

    // Apply should insert resolve-promise between awakeable and run
    apply_extension(&mut workflow, "add-signal-resolution").expect("apply should succeed");

    let resolve_id = workflow
        .nodes
        .iter()
        .find(|n| n.node_type == "resolve-promise")
        .map(|n| n.id)
        .expect("resolve-promise node should exist after apply");

    // awakeable -> resolve-promise
    assert!(
        workflow.connections.iter().any(|c| {
            c.source == awakeable_id && c.target == resolve_id && c.source_port.0 == "out"
        }),
        "awakeable should connect to resolve-promise"
    );
}

// ---------------------------------------------------------------------------
// Scenario: ext-reject-clear — Ignoring suggestions leaves workflow unchanged
// Ref: specs/scenarios/flow_extender/extension_reject_clear.yaml
// ---------------------------------------------------------------------------

#[test]
fn extension_reject_clear_workflow_unchanged() {
    let mut workflow = Workflow::new();
    let run_id = workflow.add_node("run", 50.0, 50.0);

    let suggestions = suggest_extensions(&workflow);
    assert!(!suggestions.is_empty(), "run-only workflow should have suggestions");

    // Simulate "reject" by not applying any extension.
    // The workflow must remain identical.
    let snapshot_nodes: Vec<_> = workflow.nodes.iter().map(|n| n.id).collect();
    let snapshot_conns = workflow.connections.clone();

    // No apply call — workflow untouched
    assert_eq!(workflow.nodes.len(), 1);
    assert_eq!(workflow.nodes[0].id, run_id);
    assert_eq!(workflow.nodes.iter().map(|n| n.id).collect::<Vec<_>>(), snapshot_nodes);
    assert_eq!(workflow.connections, snapshot_conns);
}

// ---------------------------------------------------------------------------
// Scenario: ext-bundle-preview-apply — Bundle preview matches apply output
// Ref: specs/scenarios/flow_extender/extension_bundle_preview_apply_consistency.yaml
// ---------------------------------------------------------------------------

#[test]
fn extension_bundle_preview_matches_applied_nodes() {
    let mut workflow = Workflow::new();
    workflow.add_node("run", 32.0, 32.0);

    let preview = preview_extension(&workflow, "add-reliability-bundle")
        .expect("preview should not error")
        .expect("bundle should be available");
    let preview_types: Vec<_> = preview.nodes.iter().map(|n| n.node_type.clone()).collect();

    apply_extension(&mut workflow, "add-reliability-bundle").expect("apply should succeed");

    let applied_types: Vec<_> = workflow
        .nodes
        .iter()
        .filter(|n| {
            n.node_type == "timeout" || n.node_type == "set-state" || n.node_type == "compensate"
        })
        .map(|n| n.node_type.clone())
        .collect();

    assert_eq!(
        applied_types, preview_types,
        "applied node types should match previewed types exactly"
    );
}
