//! Integration tests for the RightPanel component.
//!
//! RightPanel lives in the binary crate (src/ui/right_panel.rs) and depends on
//! Dioxus reactive primitives (Signal, Memo, EventHandler) that cannot be
//! instantiated outside a Dioxus runtime. These integration tests therefore
//! verify RightPanel's data contract through the public library types that
//! flow through its props:
//!
//!   - ValidationResult  (validation_result prop)
//!   - Workflow.history   (feeds ExecutionHistoryPanel via history_signal)
//!   - NodeId             (on_select_node EventHandler)
//!   - Viewport           (used by WorkflowState)
//!
//! Unit tests inside src/ui/right_panel.rs or src/hooks/ would cover runtime
//! rendering; these tests guard the structural integrity of the public API.
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic, clippy::float_cmp)]

use oya_frontend::graph::{
    validate_workflow, Node, NodeCategory, NodeId, RunRecord, ValidationIssue, ValidationResult,
    ValidationSeverity, Viewport, Workflow,
};
use std::collections::HashMap;
use uuid::Uuid;

// ===========================================================================
// 1. ValidationResult contract (RightPanel's validation_result prop)
// ===========================================================================

#[test]
fn given_valid_workflow_when_validating_then_result_is_valid_with_zero_errors() {
    let node = Node {
        id: NodeId::new(),
        name: "Entry".to_string(),
        category: NodeCategory::Entry,
        ..Default::default()
    };
    let workflow = Workflow { nodes: vec![node], connections: vec![], ..Default::default() };

    let result = validate_workflow(&workflow);

    assert!(result.valid);
    assert_eq!(result.error_count(), 0);
    assert_eq!(result.warning_count(), 0);
    assert!(!result.has_errors());
}

#[test]
fn given_workflow_without_entry_when_validating_then_result_has_errors() {
    let node = Node {
        id: NodeId::new(),
        name: "Durable".to_string(),
        category: NodeCategory::Durable,
        ..Default::default()
    };
    let workflow = Workflow { nodes: vec![node], connections: vec![], ..Default::default() };

    let result = validate_workflow(&workflow);

    assert!(!result.valid);
    assert!(result.has_errors());
    assert!(result.error_count() > 0);
}

#[test]
fn given_validation_result_when_constructing_manually_then_field_access_works() {
    let result = ValidationResult::new(true, vec![]);

    assert!(result.valid);
    assert!(result.issues.is_empty());
    assert_eq!(result.error_count(), 0);
    assert_eq!(result.warning_count(), 0);
    assert!(!result.has_errors());
}

#[test]
fn given_validation_issues_when_building_from_issues_then_valid_depends_on_severity() {
    let issues = vec![
        ValidationIssue::error("bad node"),
        ValidationIssue::error("another bad"),
        ValidationIssue::warning("sketchy node"),
    ];
    let result = ValidationResult::from_issues(issues);

    assert!(!result.valid);
    assert_eq!(result.error_count(), 2);
    assert_eq!(result.warning_count(), 1);
    assert!(result.has_errors());
}

#[test]
fn given_only_warning_issues_when_building_from_issues_then_result_is_valid() {
    let issues = vec![ValidationIssue::warning("minor issue")];
    let result = ValidationResult::from_issues(issues);

    assert!(result.valid);
    assert_eq!(result.error_count(), 0);
    assert_eq!(result.warning_count(), 1);
    assert!(!result.has_errors());
}

// ===========================================================================
// 2. RunRecord / Workflow.history (feeds ExecutionHistoryPanel)
// ===========================================================================

#[test]
fn given_workflow_with_history_when_accessing_then_records_are_available() {
    let record = RunRecord {
        id: Uuid::new_v4(),
        timestamp: chrono::Utc::now(),
        results: HashMap::new(),
        success: true,
        restate_invocation_id: None,
    };
    let workflow = Workflow {
        nodes: vec![],
        connections: vec![],
        history: vec![record],
        ..Default::default()
    };

    assert_eq!(workflow.history.len(), 1);
    assert!(workflow.history[0].success);
}

#[test]
fn given_workflow_with_multiple_runs_when_accessing_history_then_order_is_preserved() {
    let r1 = RunRecord {
        id: Uuid::new_v4(),
        timestamp: chrono::Utc::now(),
        results: HashMap::new(),
        success: true,
        restate_invocation_id: None,
    };
    let r2 = RunRecord {
        id: Uuid::new_v4(),
        timestamp: chrono::Utc::now(),
        results: HashMap::new(),
        success: false,
        restate_invocation_id: Some("inv-123".to_string()),
    };

    let workflow = Workflow {
        nodes: vec![],
        connections: vec![],
        history: vec![r1, r2],
        ..Default::default()
    };

    assert_eq!(workflow.history.len(), 2);
    assert!(workflow.history[0].success);
    assert!(!workflow.history[1].success);
    assert_eq!(workflow.history[1].restate_invocation_id.as_deref(), Some("inv-123"));
}

#[test]
fn given_run_record_with_results_when_accessing_then_node_results_are_present() {
    let node_id = NodeId::new();
    let mut results = HashMap::new();
    results.insert(node_id, serde_json::json!({"output": 42}));

    let record = RunRecord {
        id: Uuid::new_v4(),
        timestamp: chrono::Utc::now(),
        results,
        success: true,
        restate_invocation_id: None,
    };

    assert!(record.results.contains_key(&node_id));
    assert_eq!(record.results[&node_id]["output"], 42);
}

// ===========================================================================
// 3. NodeId contract (feeds on_select_node EventHandler)
// ===========================================================================

#[test]
fn given_two_node_ids_when_comparing_then_they_are_distinct() {
    let a = NodeId::new();
    let b = NodeId::new();

    assert_ne!(a, b);
}

#[test]
fn given_node_id_when_displaying_then_it_renders_uuid() {
    let id = NodeId::new();
    let display = format!("{id}");

    assert!(!display.is_empty());
    assert_eq!(display.len(), 36);
}

#[test]
fn given_node_id_when_cloning_then_clone_equals_original() {
    let id = NodeId::new();
    let cloned = id;

    assert_eq!(id, cloned);
}

// ===========================================================================
// 4. Viewport contract (used by WorkflowState which feeds RightPanel)
// ===========================================================================

#[test]
fn given_default_workflow_when_inspecting_viewport_then_defaults_are_sane() {
    let workflow = Workflow { nodes: vec![], connections: vec![], ..Default::default() };

    let vp = &workflow.viewport;
    assert!(vp.zoom > 0.0);
    assert!(vp.zoom.is_finite());
    assert!(vp.x.is_finite());
    assert!(vp.y.is_finite());
}

#[test]
fn given_custom_viewport_when_constructing_then_fields_are_preserved() {
    let vp = Viewport { x: -100.0, y: -50.0, zoom: 1.5 };

    assert_eq!(vp.x, -100.0);
    assert_eq!(vp.y, -50.0);
    assert_eq!(vp.zoom, 1.5);
}

// ===========================================================================
// 5. ValidationIssue constructors (feed ValidationPanel via RightPanel)
// ===========================================================================

#[test]
fn given_validation_issue_constructors_when_creating_then_fields_are_correct() {
    let error = ValidationIssue::error("test error");
    assert_eq!(error.severity, ValidationSeverity::Error);
    assert!(error.node_id.is_none());
    assert_eq!(error.message, "test error");

    let warning = ValidationIssue::warning("test warning");
    assert_eq!(warning.severity, ValidationSeverity::Warning);
    assert!(warning.node_id.is_none());

    let node_id = NodeId::new();
    let node_error = ValidationIssue::error_for_node("node error", node_id);
    assert_eq!(node_error.severity, ValidationSeverity::Error);
    assert_eq!(node_error.node_id, Some(node_id));

    let node_id_2 = NodeId::new();
    let node_warning = ValidationIssue::warning_for_node("node warning", node_id_2);
    assert_eq!(node_warning.severity, ValidationSeverity::Warning);
    assert_eq!(node_warning.node_id, Some(node_id_2));
}

#[test]
fn given_empty_workflow_when_validating_then_result_reports_entry_point_error() {
    let workflow = Workflow { nodes: vec![], connections: vec![], ..Default::default() };

    let result = validate_workflow(&workflow);

    assert!(!result.valid);
    let has_entry_error = result.issues.iter().any(|issue| issue.message.contains("entry point"));
    assert!(has_entry_error, "Empty workflow should report entry point error");
}

#[test]
fn given_workflow_with_errors_and_warnings_when_inspecting_issues_then_severity_is_preserved() {
    let issues = vec![
        ValidationIssue::error("critical"),
        ValidationIssue::warning("minor"),
        ValidationIssue::error_for_node("node-specific", NodeId::new()),
    ];
    let result = ValidationResult::from_issues(issues);

    assert_eq!(result.issues.len(), 3);
    assert_eq!(result.issues[0].severity, ValidationSeverity::Error);
    assert_eq!(result.issues[1].severity, ValidationSeverity::Warning);
    assert_eq!(result.issues[2].severity, ValidationSeverity::Error);
    assert!(result.issues[2].node_id.is_some());
}

// ===========================================================================
// 6. Cross-cutting: Workflow supports history mutations RightPanel observes
// ===========================================================================

#[test]
fn given_workflow_when_adding_node_then_history_can_be_recorded() {
    let mut workflow = Workflow::new();
    workflow.add_node("http-handler", 100.0, 200.0);

    assert_eq!(workflow.nodes.len(), 1);
    assert!(workflow.history.is_empty());
}

#[test]
fn given_workflow_clone_when_mutating_original_then_clone_is_unaffected() {
    let mut workflow = Workflow::new();
    workflow.add_node("http-handler", 0.0, 0.0);

    let snapshot = workflow.clone();
    workflow.add_node("run", 100.0, 0.0);

    assert_eq!(snapshot.nodes.len(), 1);
    assert_eq!(workflow.nodes.len(), 2);
}
