// Adversarial QA tests for hover state detection
// Following Red Queen methodology: Attack relentlessly, fix surgically, regress never

#![deny(clippy::unwrap_used)]
#![deny(clippy::expect_used)]
#![deny(clippy::panic)]

use oya_ui::interaction::hover::{HoverError, HoverManager, ViewportTransform};
use oya_ui::models::node::{Node, NodeShape, NodeSize};

#[test]
// CATEGORY 2: Input Boundary Attack - NaN coordinates
// WHEN: Hover coordinates are NaN
// THE SYSTEM SHALL: Handle gracefully without panicking
fn test_hover_with_nan_coordinates() {
    let mut manager = HoverManager::new();
    let node = Node::builder("test".to_string(), "Test".to_string())
        .position(100.0, 100.0)
        .circle_radius(10.0)
        .build()
        .unwrap();
    manager.set_node(node);

    let result = manager.hit_test(f64::NAN, f64::NAN);
    assert_eq!(result.node_id, None);
}

#[test]
// CATEGORY 2: Input Boundary Attack - Infinity coordinates
// WHEN: Hover coordinates are infinity
// THE SYSTEM SHALL: Handle gracefully without panicking
fn test_hover_with_infinity_coordinates() {
    let mut manager = HoverManager::new();
    let node = Node::builder("test".to_string(), "Test".to_string())
        .position(100.0, 100.0)
        .circle_radius(10.0)
        .build()
        .unwrap();
    manager.set_node(node);

    let result = manager.hit_test(f64::INFINITY, f64::INFINITY);
    assert_eq!(result.node_id, None);

    let result = manager.hit_test(f64::NEG_INFINITY, f64::NEG_INFINITY);
    assert_eq!(result.node_id, None);
}

#[test]
// CATEGORY 2: Input Boundary Attack - Negative scale
// WHEN: Viewport scale is negative
// THE SYSTEM SHALL: Return error
fn test_viewport_negative_scale() {
    let result = ViewportTransform::new().with_scale(-1.0);
    assert!(matches!(result, Err(HoverError::InvalidScale(_))));
}

#[test]
// CATEGORY 2: Input Boundary Attack - Zero scale
// WHEN: Viewport scale is zero
// THE SYSTEM SHALL: Return error
fn test_viewport_zero_scale() {
    let result = ViewportTransform::new().with_scale(0.0);
    assert!(matches!(result, Err(HoverError::InvalidScale(_))));
}

#[test]
// CATEGORY 3: State Attack - Empty HoverManager
// WHEN: Hit testing on an empty manager
// THE SYSTEM SHALL: Return no hit without panicking
fn test_hover_manager_empty() {
    let manager = HoverManager::new();
    let result = manager.hit_test(100.0, 100.0);
    assert_eq!(result.node_id, None);
    assert!(!result.cursor_changed);
}

#[test]
// CATEGORY 3: State Attack - Overlapping nodes
// WHEN: Multiple nodes overlap
// THE SYSTEM SHALL: Hit exactly one node
fn test_hover_overlapping_nodes() {
    let mut manager = HoverManager::new();
    let node1 = Node::builder("node1".to_string(), "Node 1".to_string())
        .position(100.0, 100.0)
        .circle_radius(50.0)
        .build()
        .unwrap();
    let node2 = Node::builder("node2".to_string(), "Node 2".to_string())
        .position(100.0, 100.0)
        .circle_radius(30.0)
        .build()
        .unwrap();

    manager.set_node(node1);
    manager.set_node(node2);

    let result = manager.hit_test(100.0, 100.0);
    assert!(result.node_id.is_some());
    assert!(
        result.node_id == Some("node1".to_string()) || result.node_id == Some("node2".to_string())
    );
}

#[test]
// CATEGORY 3: State Attack - Partially overlapping rectangles
// WHEN: Rectangles partially overlap
// THE SYSTEM SHALL: Hit only one under cursor
fn test_hover_overlapping_rectangles() {
    let mut manager = HoverManager::new();
    let node1 = Node::builder("node1".to_string(), "Node 1".to_string())
        .position(50.0, 50.0)
        .shape(NodeShape::Rectangle)
        .rectangle_size(100.0, 100.0)
        .build()
        .unwrap();
    let node2 = Node::builder("node2".to_string(), "Node 2".to_string())
        .position(100.0, 100.0)
        .shape(NodeShape::Rectangle)
        .rectangle_size(100.0, 100.0)
        .build()
        .unwrap();

    manager.set_node(node1);
    manager.set_node(node2);

    let result = manager.hit_test(30.0, 50.0);
    assert_eq!(result.node_id, Some("node1".to_string()));

    let result = manager.hit_test(150.0, 150.0);
    assert_eq!(result.node_id, Some("node2".to_string()));

    let result = manager.hit_test(100.0, 100.0);
    assert!(result.node_id.is_some());
    assert!(
        result.node_id == Some("node1".to_string()) || result.node_id == Some("node2".to_string())
    );
}

#[test]
// CATEGORY 2: Input Boundary Attack - Negative coordinates
// WHEN: Hover coordinates are negative
// THE SYSTEM SHALL: Handle correctly
fn test_hover_negative_coordinates() {
    let mut manager = HoverManager::new();
    let node = Node::builder("test".to_string(), "Test".to_string())
        .position(-100.0, -100.0)
        .circle_radius(50.0)
        .build()
        .unwrap();
    manager.set_node(node);

    let result = manager.hit_test(-100.0, -100.0);
    assert_eq!(result.node_id, Some("test".to_string()));

    let result = manager.hit_test(100.0, 100.0);
    assert_eq!(result.node_id, None);
}

#[test]
// CATEGORY 2: Input Boundary Attack - Very large coordinates
// WHEN: Coordinates are extremely large
// THE SYSTEM SHALL: Handle without overflow
fn test_hover_very_large_coordinates() {
    let mut manager = HoverManager::new();
    let node = Node::builder("test".to_string(), "Test".to_string())
        .position(1e10, 1e10)
        .circle_radius(1e8)
        .build()
        .unwrap();
    manager.set_node(node);

    let result = manager.hit_test(1e10, 1e10);
    assert_eq!(result.node_id, Some("test".to_string()));
}

#[test]
// CATEGORY 4: Output Contract Attack - Cursor changed flag accuracy
// WHEN: Hovering over different areas
// THE SYSTEM SHALL: Accurately report cursor changes
fn test_cursor_changed_accuracy() {
    let mut manager = HoverManager::new();
    let node = Node::builder("test".to_string(), "Test".to_string())
        .position(100.0, 100.0)
        .circle_radius(10.0)
        .build()
        .unwrap();
    manager.set_node(node);

    let result = manager.update_hover(100.0, 100.0);
    assert!(result.cursor_changed);

    let result = manager.update_hover(100.0, 100.0);
    assert!(!result.cursor_changed);

    let result = manager.update_hover(105.0, 100.0);
    assert!(!result.cursor_changed);

    let result = manager.update_hover(200.0, 200.0);
    assert!(result.cursor_changed);
}

#[test]
// CATEGORY 4: Output Contract Attack - Clear hover clears state
// WHEN: clear_hover is called
// THE SYSTEM SHALL: Reset all hover states
fn test_clear_hover_completely_clears() {
    let mut manager = HoverManager::new();
    let node1 = Node::builder("node1".to_string(), "Node 1".to_string())
        .position(100.0, 100.0)
        .circle_radius(10.0)
        .build()
        .unwrap();
    let node2 = Node::builder("node2".to_string(), "Node 2".to_string())
        .position(200.0, 200.0)
        .circle_radius(10.0)
        .build()
        .unwrap();

    manager.set_node(node1);
    manager.set_node(node2);

    manager.update_hover(100.0, 100.0);
    assert!(manager.get_node("node1").unwrap().hovered);
    assert!(!manager.get_node("node2").unwrap().hovered);

    manager.update_hover(200.0, 200.0);
    assert!(!manager.get_node("node1").unwrap().hovered);
    assert!(manager.get_node("node2").unwrap().hovered);

    manager.clear_hover();

    assert!(!manager.get_node("node1").unwrap().hovered);
    assert!(!manager.get_node("node2").unwrap().hovered);
    assert!(manager.hovered_node().is_none());
}

#[test]
// CATEGORY 2: Input Boundary Attack - Viewport transform edge cases
// WHEN: Using viewport transforms at edge cases
// THE SYSTEM SHALL: Transform correctly
fn test_viewport_transform_edge_cases() {
    let viewport = ViewportTransform::new().with_scale(0.001).unwrap();
    let world = viewport.screen_to_world(1000.0, 1000.0);
    assert_eq!(world.x, 1000000.0);

    let viewport = ViewportTransform::new().with_scale(1000.0).unwrap();
    let world = viewport.screen_to_world(1000.0, 1000.0);
    assert_eq!(world.x, 1.0);
}

#[test]
// CATEGORY 3: State Attack - Remove hovered node
// WHEN: The hovered node is removed
// THE SYSTEM SHALL: Clear hover state
fn test_remove_hovered_node() {
    let mut manager = HoverManager::new();
    let node = Node::builder("test".to_string(), "Test".to_string())
        .position(100.0, 100.0)
        .circle_radius(10.0)
        .build()
        .unwrap();
    manager.set_node(node);

    manager.update_hover(100.0, 100.0);
    assert!(manager.hovered_node().is_some());

    manager.remove_node("test");

    assert!(manager.hovered_node().is_none());
}

#[test]
// CATEGORY 2: Input Boundary Attack - Circle radius zero
// WHEN: Circle has radius of zero
// THE SYSTEM SHALL: Reject creation
fn test_circle_zero_radius() {
    let result = NodeSize::circle(0.0);
    assert!(result.is_err());

    let result = NodeSize::circle(10.0);
    assert!(result.is_ok());
}

#[test]
// CATEGORY 2: Input Boundary Attack - Rectangle zero dimensions
// WHEN: Rectangle has zero width or height
// THE SYSTEM SHALL: Reject creation
fn test_rectangle_zero_dimensions() {
    let result = NodeSize::rectangle(0.0, 10.0);
    assert!(result.is_err());

    let result = NodeSize::rectangle(10.0, 0.0);
    assert!(result.is_err());

    let result = NodeSize::rectangle(100.0, 50.0);
    assert!(result.is_ok());
}
