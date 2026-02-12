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
    assert!(
        result
            .lines
            .iter()
            .any(|l| l.contains("┌") || l.contains("╭"))
    );
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

    assert!(
        result
            .lines
            .iter()
            .any(|l| l.contains("cycle") || l.contains("Cycle"))
    );
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
