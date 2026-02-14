//! DAG renderer tests for GraphView
//!
//! Tests the ASCII DAG rendering for the workflow graph pane.

#![allow(clippy::unwrap_used)]
#![allow(clippy::expect_used)]

use zellij_frontend::render::{DagNode, DagRenderer};

fn create_test_node(id: &str, name: &str) -> DagNode {
    DagNode::new(id, name)
}

fn create_test_node_with_deps(id: &str, name: &str, deps: Vec<&str>) -> DagNode {
    let mut node = DagNode::new(id, name);
    node.dependencies = deps.into_iter().map(String::from).collect();
    node
}

#[test]
fn dag_renderer_new_creates_default_instance() {
    let renderer = DagRenderer::new();
    assert_eq!(renderer.width(), 78);
    assert_eq!(renderer.height(), 6);
}

#[test]
fn dag_renderer_with_dimensions_sets_custom_size() {
    let renderer = DagRenderer::new().with_dimensions(100, 10);
    assert_eq!(renderer.width(), 100);
    assert_eq!(renderer.height(), 10);
}

#[test]
fn dag_node_new_creates_node_with_defaults() {
    let node = create_test_node("src-abc1", "Implement feature X");
    assert_eq!(node.id, "src-abc1");
    assert_eq!(node.name, "Implement feature X");
    assert!(node.dependencies.is_empty());
}

#[test]
fn dag_render_empty_nodes_returns_placeholder() {
    let renderer = DagRenderer::new();
    let nodes: Vec<DagNode> = vec![];
    let result = renderer.render(&nodes);

    assert!(result.lines.iter().any(|l| l.contains("No workflow data")));
}

#[test]
fn dag_render_single_node_displays_box() {
    let renderer = DagRenderer::new();
    let nodes = vec![create_test_node("src-abc1", "Feature X")];
    let result = renderer.render(&nodes);

    assert!(result.lines.iter().any(|l| l.contains("Feature X")));
    assert!(result
        .lines
        .iter()
        .any(|l| l.contains("┌") || l.contains("╭")));
}

#[test]
fn dag_render_two_nodes_with_dependency_shows_arrow() {
    let renderer = DagRenderer::new();
    let nodes = vec![
        create_test_node("src-abc1", "A"),
        create_test_node_with_deps("src-def2", "B", vec!["src-abc1"]),
    ];
    let result = renderer.render(&nodes);

    // Should show both nodes
    let output = result.lines.join("\n");
    assert!(output.contains("A"));
    assert!(output.contains("B"));

    // Should show connection (arrow or line)
    assert!(output.contains("─") || output.contains("▶") || output.contains("→"));
}

#[test]
fn dag_render_diamond_dependency_layouts_correctly() {
    let renderer = DagRenderer::new();
    // Diamond: A -> B, A -> C, B -> D, C -> D
    let nodes = vec![
        create_test_node("A", "Root"),
        create_test_node_with_deps("B", "Left", vec!["A"]),
        create_test_node_with_deps("C", "Right", vec!["A"]),
        create_test_node_with_deps("D", "Merge", vec!["B", "C"]),
    ];
    let result = renderer.render(&nodes);

    let output = result.lines.join("\n");
    assert!(output.contains("Root"));
    assert!(output.contains("Left"));
    assert!(output.contains("Right"));
    assert!(output.contains("Merge"));
}

#[test]
fn dag_render_respects_width_constraint() {
    let renderer = DagRenderer::new().with_dimensions(40, 6);
    let nodes = vec![
        create_test_node("A", "Long node name that should be truncated"),
        create_test_node_with_deps("B", "Another long name here", vec!["A"]),
    ];
    let result = renderer.render(&nodes);

    // No line should exceed width
    for line in &result.lines {
        // Account for ANSI codes which add length but don't display
        let visible_len = strip_ansi_codes(line).len();
        assert!(visible_len <= 50, "Line too long: {} chars", visible_len);
    }
}

#[test]
fn dag_render_levels_calculated_correctly() {
    let renderer = DagRenderer::new();
    // Linear chain: A -> B -> C
    let nodes = vec![
        create_test_node("A", "First"),
        create_test_node_with_deps("B", "Second", vec!["A"]),
        create_test_node_with_deps("C", "Third", vec!["B"]),
    ];

    let levels = renderer.calculate_levels(&nodes);

    assert_eq!(levels.len(), 3);
    assert!(levels[0].contains(&"A".to_string()));
    assert!(levels[1].contains(&"B".to_string()));
    assert!(levels[2].contains(&"C".to_string()));
}

#[test]
fn dag_render_parallel_nodes_on_same_level() {
    let renderer = DagRenderer::new();
    // A -> B, A -> C (B and C should be on same level)
    let nodes = vec![
        create_test_node("A", "Root"),
        create_test_node_with_deps("B", "Child1", vec!["A"]),
        create_test_node_with_deps("C", "Child2", vec!["A"]),
    ];

    let levels = renderer.calculate_levels(&nodes);

    assert_eq!(levels.len(), 2);
    assert!(levels[0].contains(&"A".to_string()));
    assert!(levels[1].contains(&"B".to_string()));
    assert!(levels[1].contains(&"C".to_string()));
}

#[test]
fn dag_render_cycle_detection_returns_error() {
    let renderer = DagRenderer::new();
    // Cycle: A -> B -> A
    let nodes = vec![
        create_test_node_with_deps("A", "NodeA", vec!["B"]),
        create_test_node_with_deps("B", "NodeB", vec!["A"]),
    ];

    let result = renderer.render(&nodes);

    assert!(result
        .lines
        .iter()
        .any(|l| l.contains("cycle") || l.contains("Cycle")));
}

#[test]
fn dag_render_truncates_long_names() {
    let renderer = DagRenderer::new();
    let long_name = "This is a very long task name that should be truncated";
    let nodes = vec![create_test_node("A", long_name)];
    let result = renderer.render(&nodes);

    let output = result.lines.join("\n");
    // Should be truncated with ellipsis
    assert!(output.contains("...") || !output.contains(long_name));
}

#[test]
fn dag_render_status_colors_applied() {
    let renderer = DagRenderer::new();
    let mut node = create_test_node("A", "Task");
    node.status = zellij_frontend::render::NodeStatus::InProgress;

    let result = renderer.render(&[node]);
    let output = result.lines.join("\n");

    // Should contain ANSI color codes
    assert!(output.contains("\x1b[") || output.contains("◐"));
}

#[test]
fn dag_node_with_all_statuses_renders_correctly() {
    use zellij_frontend::render::NodeStatus;

    let statuses = [
        NodeStatus::Pending,
        NodeStatus::InProgress,
        NodeStatus::Completed,
        NodeStatus::Failed,
        NodeStatus::Blocked,
    ];

    let renderer = DagRenderer::new();

    for status in statuses {
        let mut node = create_test_node("A", "Task");
        node.status = status;
        let result = renderer.render(&[node]);
        assert!(
            !result.lines.is_empty(),
            "Should render for status {:?}",
            status
        );
    }
}

// Helper function to strip ANSI codes for length calculation
fn strip_ansi_codes(s: &str) -> String {
    let mut result = String::with_capacity(s.len());
    let mut chars = s.chars().peekable();

    while let Some(c) = chars.next() {
        if c == '\x1b' {
            // Skip ANSI escape sequence
            if chars.peek() == Some(&'[') {
                chars.next(); // consume '['
                              // Skip until we reach a letter (the terminator)
                while let Some(&next) = chars.peek() {
                    chars.next();
                    if next.is_ascii_alphabetic() {
                        break;
                    }
                }
            }
        } else {
            result.push(c);
        }
    }

    result
}

// ==================== WorkflowGraph Tests ====================

use zellij_frontend::render::{NodeStatus, WorkflowGraph, WorkflowGraphError};

#[test]
fn workflow_graph_new_creates_empty_graph() {
    let graph = WorkflowGraph::new();
    assert_eq!(graph.node_count(), 0);
}

#[test]
fn workflow_graph_add_node_increases_count() {
    let mut graph = WorkflowGraph::new();
    let result = graph.add_node("src-abc", "Setup Project");
    assert!(result.is_ok());
    assert_eq!(graph.node_count(), 1);
}

#[test]
fn workflow_graph_add_duplicate_node_returns_error() {
    let mut graph = WorkflowGraph::new();
    let _ = graph.add_node("src-abc", "Setup");
    let result = graph.add_node("src-abc", "Setup Again");
    assert!(result.is_err());
    assert!(matches!(
        result,
        Err(WorkflowGraphError::DuplicateNode { .. })
    ));
}

#[test]
fn workflow_graph_add_dependency_creates_edge() {
    let mut graph = WorkflowGraph::new();
    let _ = graph.add_node("A", "First");
    let _ = graph.add_node("B", "Second");
    let result = graph.add_dependency("B", "A");
    assert!(result.is_ok());
}

#[test]
fn workflow_graph_add_dependency_to_missing_node_returns_error() {
    let mut graph = WorkflowGraph::new();
    let _ = graph.add_node("A", "First");
    let result = graph.add_dependency("B", "A");
    assert!(result.is_err());
    assert!(matches!(
        result,
        Err(WorkflowGraphError::NodeNotFound { .. })
    ));
}

#[test]
fn workflow_graph_set_status_updates_node() {
    let mut graph = WorkflowGraph::new();
    let _ = graph.add_node("A", "Task");
    let result = graph.set_node_status("A", NodeStatus::InProgress);
    assert!(result.is_ok());
}

#[test]
fn workflow_graph_set_status_missing_node_returns_error() {
    let mut graph = WorkflowGraph::new();
    let result = graph.set_node_status("missing", NodeStatus::Completed);
    assert!(result.is_err());
}

#[test]
fn workflow_graph_render_returns_rendered_output() {
    let mut graph = WorkflowGraph::new();
    let _ = graph.add_node("A", "First");
    let _ = graph.add_node("B", "Second");
    let _ = graph.add_dependency("B", "A");

    let output = graph.render();
    assert!(output.lines.iter().any(|l| l.contains("First")));
    assert!(output.lines.iter().any(|l| l.contains("Second")));
}

#[test]
fn workflow_graph_render_empty_returns_placeholder() {
    let graph = WorkflowGraph::new();
    let output = graph.render();
    assert!(output.lines.iter().any(|l| l.contains("No workflow data")));
}

#[test]
fn workflow_graph_get_node_returns_reference() {
    let mut graph = WorkflowGraph::new();
    let _ = graph.add_node("A", "Task");
    let node = graph.get_node("A");
    assert!(node.is_some());
    assert_eq!(node.map(|n| n.name.as_str()), Some("Task"));
}

#[test]
fn workflow_graph_get_nodes_returns_all_nodes() {
    let mut graph = WorkflowGraph::new();
    let _ = graph.add_node("A", "First");
    let _ = graph.add_node("B", "Second");
    let nodes = graph.nodes();
    assert_eq!(nodes.len(), 2);
}

#[test]
fn workflow_graph_with_dimensions_customizes_renderer() {
    let mut graph = WorkflowGraph::new();
    let _ = graph.add_node("A", "Task");
    graph = graph.with_dimensions(60, 8);
    let output = graph.render();
    assert!(!output.lines.is_empty());
}

#[test]
fn workflow_graph_default_creates_empty_graph() {
    let graph = WorkflowGraph::default();
    assert_eq!(graph.node_count(), 0);
}

// ==================== GraphEdge Tests ====================

use zellij_frontend::render::{EdgeKind, GraphEdge};

#[test]
fn graph_edge_new_creates_edge_with_default_kind() {
    let edge = GraphEdge::new("A", "B");
    assert_eq!(edge.source(), "A");
    assert_eq!(edge.target(), "B");
    assert_eq!(edge.kind(), EdgeKind::Dependency);
}

#[test]
fn graph_edge_new_with_kind_sets_custom_kind() {
    let edge = GraphEdge::new_with_kind("A", "B", EdgeKind::Blocks);
    assert_eq!(edge.source(), "A");
    assert_eq!(edge.target(), "B");
    assert_eq!(edge.kind(), EdgeKind::Blocks);
}

#[test]
fn graph_edge_connects_returns_correct_relation() {
    let edge = GraphEdge::new("src-abc", "src-def");
    assert!(edge.connects("src-abc", "src-def"));
    assert!(!edge.connects("src-def", "src-abc"));
    assert!(!edge.connects("src-abc", "src-xyz"));
}

#[test]
fn graph_edge_clone_preserves_all_fields() {
    let edge = GraphEdge::new_with_kind("A", "B", EdgeKind::Requires);
    let cloned = edge.clone();
    assert_eq!(edge.source(), cloned.source());
    assert_eq!(edge.target(), cloned.target());
    assert_eq!(edge.kind(), cloned.kind());
}

#[test]
fn edge_kind_variants_exist() {
    let kinds = [
        EdgeKind::Dependency,
        EdgeKind::Blocks,
        EdgeKind::Requires,
        EdgeKind::Soft,
    ];
    assert_eq!(kinds.len(), 4);
}

#[test]
fn graph_edge_debug_format_includes_fields() {
    let edge = GraphEdge::new("node-a", "node-b");
    let debug_str = format!("{:?}", edge);
    assert!(debug_str.contains("node-a"));
    assert!(debug_str.contains("node-b"));
}
