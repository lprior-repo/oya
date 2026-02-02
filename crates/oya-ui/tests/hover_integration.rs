//! Integration tests for hover detection
//!
//! These tests verify that the hover detection system works correctly
//! with real-world scenarios including viewport transforms.

use oya_ui::components::canvas::coords::Viewport;
use oya_ui::components::controls::bounds::ZoomLevel;
use oya_ui::interaction::hover::contains_point;
use oya_ui::models::node::{Node, NodeShape, Position};

#[test]
fn test_hover_detection_at_origin() -> Result<(), String> {
    let mut node = Node::new("test", "Test Node")?;
    node.set_position(Position::origin());
    node.set_shape(NodeShape::Circle);

    let viewport = Viewport::new(1200.0, 800.0)?;

    // Mouse at center of screen (where origin is rendered)
    assert!(contains_point(&node, 600.0, 400.0, &viewport)?);

    // Mouse outside node
    assert!(!contains_point(&node, 100.0, 100.0, &viewport)?);

    Ok(())
}

#[test]
fn test_hover_with_viewport_pan() -> Result<(), String> {
    let mut node = Node::new("test", "Test Node")?;
    node.set_position(Position::origin());
    node.set_shape(NodeShape::Circle);

    // Pan the viewport
    let viewport = Viewport::with_transform(1200.0, 800.0, 100.0, 50.0, ZoomLevel::default())?;

    // Node now appears at different screen location due to pan
    assert!(contains_point(&node, 700.0, 450.0, &viewport)?);

    Ok(())
}

#[test]
fn test_hover_with_viewport_zoom() -> Result<(), String> {
    let mut node = Node::new("test", "Test Node")?;
    node.set_position(Position::origin());
    node.set_shape(NodeShape::Circle);

    let zoom = ZoomLevel::new(2.0)?;
    let viewport = Viewport::with_transform(1200.0, 800.0, 0.0, 0.0, zoom)?;

    // Center should still be hoverable despite zoom
    assert!(contains_point(&node, 600.0, 400.0, &viewport)?);

    Ok(())
}

#[test]
fn test_hover_rectangle_shape() -> Result<(), String> {
    let mut node = Node::new("test", "Test Node")?;
    node.set_position(Position::origin());
    node.set_shape(NodeShape::Square);

    let viewport = Viewport::new(1200.0, 800.0)?;

    // Center
    assert!(contains_point(&node, 600.0, 400.0, &viewport)?);

    // Edge
    assert!(contains_point(&node, 620.0, 400.0, &viewport)?);

    // Just outside
    assert!(!contains_point(&node, 621.0, 400.0, &viewport)?);

    Ok(())
}

#[test]
fn test_hover_with_offset_position() -> Result<(), String> {
    let mut node = Node::new("test", "Test Node")?;
    node.set_position(Position::new(100.0, 50.0)?);
    node.set_shape(NodeShape::Circle);

    let viewport = Viewport::new(1200.0, 800.0)?;

    // Node is at (600 + 100, 400 + 50) = (700, 450) in screen space
    assert!(contains_point(&node, 700.0, 450.0, &viewport)?);
    assert!(!contains_point(&node, 600.0, 400.0, &viewport)?);

    Ok(())
}

#[test]
fn test_multiple_nodes_no_false_positives() -> Result<(), String> {
    let mut node1 = Node::new("node1", "Node 1")?;
    node1.set_position(Position::new(0.0, 0.0)?);
    node1.set_shape(NodeShape::Circle);

    let mut node2 = Node::new("node2", "Node 2")?;
    node2.set_position(Position::new(100.0, 0.0)?);
    node2.set_shape(NodeShape::Circle);

    let viewport = Viewport::new(1200.0, 800.0)?;

    // Point that hits node1 but not node2
    assert!(contains_point(&node1, 600.0, 400.0, &viewport)?);
    assert!(!contains_point(&node2, 600.0, 400.0, &viewport)?);

    // Point that hits node2 but not node1
    assert!(!contains_point(&node1, 700.0, 400.0, &viewport)?);
    assert!(contains_point(&node2, 700.0, 400.0, &viewport)?);

    Ok(())
}
