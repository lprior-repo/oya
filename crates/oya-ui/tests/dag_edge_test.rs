#![deny(clippy::unwrap_used)]
#![deny(clippy::expect_used)]
#![deny(clippy::panic)]
#![warn(clippy::pedantic)]
#![warn(clippy::nursery)]
#![forbid(unsafe_code)]

use oya_shared::graph::{
    Edge, EdgeState, EdgeStyle, EdgeType, Node, NodeId, Position as GraphPosition,
};
use oya_ui::dag_edge::{calculate_line_path, EdgeError};
use proptest::prelude::*;
use proptest::test_runner::TestCaseError;

const EPSILON: f64 = 1e-6;

// ============================================================================
// Edge Struct Tests
// ============================================================================

/// Tests that Edge creation with all fields works correctly
#[test]
fn test_edge_creation_all_fields() -> Result<(), String> {
    let source = NodeId::new("source")?;
    let target = NodeId::new("target")?;

    let edge = Edge::new(source.clone(), target.clone(), EdgeType::Dependency)?;

    assert_eq!(edge.source().as_str(), "source");
    assert_eq!(edge.target().as_str(), "target");
    assert_eq!(edge.edge_type(), EdgeType::Dependency);
    assert_eq!(edge.style(), EdgeStyle::Solid);
    assert_eq!(edge.state(), EdgeState::Normal);
    assert_eq!(edge.label(), None);

    Ok(())
}

/// Tests that Edge state transitions work correctly
#[test]
fn test_edge_state_transitions() -> Result<(), String> {
    let source = NodeId::new("node1")?;
    let target = NodeId::new("node2")?;
    let mut edge = Edge::new(source, target, EdgeType::Dependency)?;

    assert_eq!(edge.state(), EdgeState::Normal);

    edge.set_state(EdgeState::Highlighted);
    assert_eq!(edge.state(), EdgeState::Highlighted);

    edge.set_state(EdgeState::Dimmed);
    assert_eq!(edge.state(), EdgeState::Dimmed);

    edge.set_state(EdgeState::Normal);
    assert_eq!(edge.state(), EdgeState::Normal);

    Ok(())
}

/// Tests that Edge style variants work correctly
#[test]
fn test_edge_style_variants() -> Result<(), String> {
    let source = NodeId::new("node1")?;
    let target = NodeId::new("node2")?;

    let styles = [EdgeStyle::Solid, EdgeStyle::Dashed, EdgeStyle::Dotted];

    for style in styles {
        let mut edge = Edge::new(source.clone(), target.clone(), EdgeType::DataFlow)?;
        edge.set_style(style);
        assert_eq!(edge.style(), style);
    }

    Ok(())
}

/// Tests that Edge type variants work correctly
#[test]
fn test_edge_type_variants() -> Result<(), String> {
    let source = NodeId::new("node1")?;
    let target = NodeId::new("node2")?;

    let edge1 = Edge::new(source.clone(), target.clone(), EdgeType::Dependency)?;
    assert_eq!(edge1.edge_type(), EdgeType::Dependency);

    let edge2 = Edge::new(source.clone(), target.clone(), EdgeType::DataFlow)?;
    assert_eq!(edge2.edge_type(), EdgeType::DataFlow);

    let edge3 = Edge::new(source, target, EdgeType::Trigger)?;
    assert_eq!(edge3.edge_type(), EdgeType::Trigger);

    Ok(())
}

/// Tests that Edge with label works correctly
#[test]
fn test_edge_with_label() -> Result<(), String> {
    let source = NodeId::new("node1")?;
    let target = NodeId::new("node2")?;

    let mut edge = Edge::new(source, target, EdgeType::Dependency)?;
    assert_eq!(edge.label(), None);

    edge.set_label(Some("test label".to_string()));
    assert_eq!(edge.label(), Some("test label"));

    edge.set_label(None);
    assert_eq!(edge.label(), None);

    Ok(())
}

/// Property test: Edge creation always succeeds with valid inputs
proptest! {
    #[test]
    fn prop_edge_valid_creation(
        source_id in "[a-z]{1,20}",
        target_id in "[a-z]{1,20}"
    ) {
        prop_assume!(source_id != target_id);

        let source = NodeId::new(source_id.clone())
            .map_err(|err| TestCaseError::fail(err.to_string()))?;
        let target = NodeId::new(target_id.clone())
            .map_err(|err| TestCaseError::fail(err.to_string()))?;

        let edge_types = [EdgeType::Dependency, EdgeType::DataFlow, EdgeType::Trigger];

        for edge_type in edge_types {
            let edge = Edge::new(source.clone(), target.clone(), edge_type)
                .map_err(|err| TestCaseError::fail(err.to_string()))?;
            prop_assert_eq!(edge.source().as_str(), source_id);
            prop_assert_eq!(edge.target().as_str(), target_id);
            prop_assert_eq!(edge.edge_type(), edge_type);
        }
    }
}

/// Property test: Self-referencing edges always fail
proptest! {
    #[test]
    fn prop_self_referencing_edge_fails(id in "[a-z]{1,20}") {
        let node_id = NodeId::new(id.clone())
            .map_err(|err| TestCaseError::fail(err.to_string()))?;
        let result = Edge::new(node_id.clone(), node_id, EdgeType::Dependency);
        prop_assert!(result.is_err());
    }
}

// ============================================================================
// Line Path Tests
// ============================================================================

/// Test basic line path calculation (horizontal line)
#[test]
fn test_line_path_basic() -> Result<(), EdgeError> {
    let path = calculate_line_path(
        (0.0, 0.0),
        10.0, // source
        (100.0, 0.0),
        10.0, // target
    )?;
    assert_eq!(path.start, (10.0, 0.0));
    assert_eq!(path.end, (90.0, 0.0));
    assert_eq!(path.length, 80.0);
    Ok(())
}

/// Test vertical line path
#[test]
fn test_line_path_vertical() -> Result<(), EdgeError> {
    let path = calculate_line_path((0.0, 0.0), 5.0, (0.0, 50.0), 5.0)?;
    assert_eq!(path.start, (0.0, 5.0));
    assert_eq!(path.end, (0.0, 45.0));
    assert_eq!(path.length, 40.0);
    Ok(())
}

/// Test diagonal line path
#[test]
fn test_line_path_diagonal() -> Result<(), EdgeError> {
    let path = calculate_line_path((0.0, 0.0), 10.0, (30.0, 40.0), 10.0)?;
    assert!((path.start.0 - 6.0).abs() < EPSILON);
    assert!((path.start.1 - 8.0).abs() < EPSILON);
    assert!((path.end.0 - 24.0).abs() < EPSILON);
    assert!((path.end.1 - 32.0).abs() < EPSILON);
    assert!((path.length - 30.0).abs() < EPSILON);
    Ok(())
}

/// Test line path with zero radii (points)
#[test]
fn test_line_path_zero_radius() -> Result<(), EdgeError> {
    let path = calculate_line_path((0.0, 0.0), 0.0, (100.0, 0.0), 0.0)?;
    assert_eq!(path.start, (0.0, 0.0));
    assert_eq!(path.end, (100.0, 0.0));
    assert_eq!(path.length, 100.0);
    Ok(())
}

/// Test line path with asymmetric radii
#[test]
fn test_line_path_asymmetric_radii() -> Result<(), EdgeError> {
    let path = calculate_line_path((0.0, 0.0), 5.0, (100.0, 0.0), 15.0)?;
    assert_eq!(path.start, (5.0, 0.0));
    assert_eq!(path.end, (85.0, 0.0));
    assert_eq!(path.length, 80.0);
    Ok(())
}

/// Test coincident nodes error case
#[test]
fn test_coincident_nodes_error() {
    let result = calculate_line_path((50.0, 50.0), 10.0, (50.0, 50.0), 10.0);
    assert_eq!(result, Err(EdgeError::CoincidentNodes));
}

/// Test negative radius error case
#[test]
fn test_invalid_radius_error() {
    let result = calculate_line_path((0.0, 0.0), -5.0, (100.0, 0.0), 10.0);
    assert!(result.is_err());
    assert_eq!(result, Err(EdgeError::InvalidRadius(-5.0)));
}

/// Property test: Line path never panics with valid inputs
proptest! {
    #[test]
    fn prop_line_path_never_panics(
        x1 in -1000.0..=1000.0f64,
        y1 in -1000.0..=1000.0f64,
        x2 in -1000.0..=1000.0f64,
        y2 in -1000.0..=1000.0f64,
        r1 in 0.0..=100.0f64,
        r2 in 0.0..=100.0f64
    ) {
        // Skip coincident positions
        prop_assume!((x1 - x2).abs() > EPSILON || (y1 - y2).abs() > EPSILON);

        let path = calculate_line_path((x1, y1), r1, (x2, y2), r2)
            .map_err(|err| TestCaseError::fail(err.to_string()))?;
        prop_assert!(path.length >= 0.0);
        prop_assert!(path.start.0.is_finite());
        prop_assert!(path.start.1.is_finite());
        prop_assert!(path.end.0.is_finite());
        prop_assert!(path.end.1.is_finite());
    }
}

/// Property test: Zero radius doesn't affect position correctness
proptest! {
    #[test]
    fn prop_zero_radius_uses_center(
        x1 in -1000.0..=1000.0f64,
        y1 in -1000.0..=1000.0f64,
        x2 in -1000.0..=1000.0f64,
        y2 in -1000.0..=1000.0f64
    ) {
        // Skip coincident positions
        prop_assume!((x1 - x2).abs() > EPSILON || (y1 - y2).abs() > EPSILON);

        let path = calculate_line_path((x1, y1), 0.0, (x2, y2), 0.0)
            .map_err(|err| TestCaseError::fail(err.to_string()))?;

        prop_assert!((path.start.0 - x1).abs() < EPSILON);
        prop_assert!((path.start.1 - y1).abs() < EPSILON);
        prop_assert!((path.end.0 - x2).abs() < EPSILON);
        prop_assert!((path.end.1 - y2).abs() < EPSILON);
    }
}

/// Property test: Path length is always positive
proptest! {
    #[test]
    fn prop_path_length_positive(
        x1 in -1000.0..=1000.0f64,
        y1 in -1000.0..=1000.0f64,
        x2 in -1000.0..=1000.0f64,
        y2 in -1000.0..=1000.0f64,
        r1 in 0.0..=100.0f64,
        r2 in 0.0..=100.0f64
    ) {
        // Skip coincident positions
        prop_assume!((x1 - x2).abs() > EPSILON || (y1 - y2).abs() > EPSILON);

        let path = calculate_line_path((x1, y1), r1, (x2, y2), r2)
            .map_err(|err| TestCaseError::fail(err.to_string()))?;

        let distance = ((x2 - x1).powi(2) + (y2 - y1).powi(2)).sqrt();
        let expected_length = distance - r1 - r2;

        if expected_length > 0.0 {
            prop_assert!(path.length > 0.0);
            prop_assert!((path.length - expected_length).abs() < EPSILON);
        }
    }
}

/// Property test: Start point is always along direction vector
proptest! {
    #[test]
    fn prop_start_along_direction(
        x1 in -1000.0..=1000.0f64,
        y1 in -1000.0..=1000.0f64,
        x2 in -1000.0..=1000.0f64,
        y2 in -1000.0..=1000.0f64,
        r1 in 0.0..=100.0f64
    ) {
        // Skip coincident positions
        prop_assume!((x1 - x2).abs() > EPSILON || (y1 - y2).abs() > EPSILON);

        let path = calculate_line_path((x1, y1), r1, (x2, y2), r1)
            .map_err(|err| TestCaseError::fail(err.to_string()))?;

        // Vector from source to start should be r1 in direction of target
        let dx = path.start.0 - x1;
        let dy = path.start.1 - y1;
        let dist = (dx * dx + dy * dy).sqrt();
        prop_assert!((dist - r1).abs() < EPSILON);
    }
}

// ============================================================================
// Bezier Curve Tests (TODO - Not Implemented Yet)
// ============================================================================

/// Test parallel edge detection
#[test]
#[ignore = "Bezier curves not implemented yet"]
fn test_parallel_edge_detection() {
    // TODO: Implement when calculate_bezier_curve is available
}

/// Test control point calculation
#[test]
#[ignore = "Bezier curves not implemented yet"]
fn test_bezier_control_point() {
    // TODO: Implement when calculate_bezier_curve is available
}

/// Test curve sampling accuracy
#[test]
#[ignore = "Bezier curves not implemented yet"]
fn test_bezier_curve_sampling() {
    // TODO: Implement when calculate_bezier_curve is available
}

/// Test multiple parallel edges (3+)
#[test]
#[ignore = "Bezier curves not implemented yet"]
fn test_multiple_parallel_edges() {
    // TODO: Implement when calculate_bezier_curve is available
}

/// Property test: Bezier curves are always valid
#[cfg(feature = "proptest")]
proptest! {
    #[test]
    #[ignore = "Bezier curves not implemented yet"]
    fn prop_bezier_always_valid(
        x1 in -1000.0..=1000.0f64,
        y1 in -1000.0..=1000.0f64,
        x2 in -1000.0..=1000.0f64,
        y2 in -1000.0..=1000.0f64
    ) {
        // TODO: Implement when calculate_bezier_curve is available
    }
}

// ============================================================================
// Arrow Head Tests (TODO - Not Implemented Yet)
// ============================================================================

/// Test arrow triangle generation
#[test]
#[ignore = "Arrow heads not implemented yet"]
fn test_arrow_triangle_generation() {
    // TODO: Implement when calculate_arrow_head is available
}

/// Test arrow direction accuracy
#[test]
#[ignore = "Arrow heads not implemented yet"]
fn test_arrow_direction() -> Result<(), EdgeError> {
    // TODO: Implement when calculate_arrow_head is available
    // Example from bead spec:
    // let arrow = calculate_arrow_head(
    //     (100.0, 50.0),
    //     (1.0, 0.0),  // pointing right
    //     12.0, 8.0,
    // )?;
    // assert!(arrow.tip.0 > arrow.wing1.0);
    Ok(())
}

/// Test arrow scaling with zoom
#[test]
#[ignore = "Arrow heads not implemented yet"]
fn test_arrow_zoom_scaling() {
    // TODO: Implement when calculate_arrow_head is available
}

/// Test different arrow styles
#[test]
#[ignore = "Arrow heads not implemented yet"]
fn test_arrow_styles() {
    // TODO: Implement when calculate_arrow_head is available
}

/// Property test: Arrow heads always point toward target
#[cfg(feature = "proptest")]
proptest! {
    #[test]
    #[ignore = "Arrow heads not implemented yet"]
    fn prop_arrow_direction_correct(
        tip_x in -1000.0..=1000.0f64,
        tip_y in -1000.0..=1000.0f64,
        dir_x in -1.0..=1.0f64,
        dir_y in -1.0..=1.0f64
    ) {
        // TODO: Implement when calculate_arrow_head is available
        // Arrow tip should always be furthest in direction vector
    }
}

// ============================================================================
// Integration Tests
// ============================================================================

/// Test complete edge rendering pipeline with straight lines
#[test]
fn test_integration_edge_rendering() -> Result<(), Box<dyn std::error::Error>> {
    // Create nodes with positions
    let source = Node::with_position("source", "Source Node", 50.0, 50.0)?;
    let target = Node::with_position("target", "Target Node", 150.0, 50.0)?;

    // Create edge
    let edge = Edge::new(
        source.id().clone(),
        target.id().clone(),
        EdgeType::Dependency,
    )?;

    // Calculate line path
    let path = calculate_line_path(
        (source.position().x as f64, source.position().y as f64),
        10.0, // assume radius 10
        (target.position().x as f64, target.position().y as f64),
        10.0,
    )?;

    // Verify the path connects the nodes
    assert!(path.length > 0.0);
    assert_eq!(path.start, (60.0, 50.0));
    assert_eq!(path.end, (140.0, 50.0));

    // Verify edge metadata
    assert_eq!(edge.source(), source.id());
    assert_eq!(edge.target(), target.id());
    assert_eq!(edge.edge_type(), EdgeType::Dependency);

    Ok(())
}

/// Test edge + arrow rendering (arrow placeholder)
#[test]
#[ignore = "Arrow heads not implemented yet"]
fn test_integration_edge_with_arrow() {
    // TODO: Implement when arrow rendering is available
    // Should test that arrow head is positioned at end of path
    // and points in correct direction
}

/// Test curve + arrow rendering (both placeholders)
#[test]
#[ignore = "Bezier curves not implemented yet"]
fn test_integration_curve_with_arrow() {
    // TODO: Implement when bezier curves and arrow rendering are available
    // Should test that arrow head follows curve direction
    // at end of bezier curve
}

/// Test edge state updates affect rendering
#[test]
fn test_integration_edge_state_updates() -> Result<(), Box<dyn std::error::Error>> {
    let source = NodeId::new("source")?;
    let target = NodeId::new("target")?;
    let mut edge = Edge::new(source, target, EdgeType::Dependency)?;

    // Normal state
    assert_eq!(edge.state(), EdgeState::Normal);

    // Update to highlighted
    edge.set_state(EdgeState::Highlighted);
    assert_eq!(edge.state(), EdgeState::Highlighted);

    // Update to dimmed
    edge.set_state(EdgeState::Dimmed);
    assert_eq!(edge.state(), EdgeState::Dimmed);

    // Update style as well
    edge.set_style(EdgeStyle::Dashed);
    assert_eq!(edge.style(), EdgeStyle::Dashed);

    // Path calculation should work independently of state/style
    let path = calculate_line_path((0.0, 0.0), 10.0, (100.0, 0.0), 10.0)?;
    assert_eq!(path.length, 80.0);

    Ok(())
}

/// Test multiple edges between same nodes (parallel edges)
#[test]
fn test_integration_parallel_edges() -> Result<(), Box<dyn std::error::Error>> {
    let source = NodeId::new("source")?;
    let target = NodeId::new("target")?;

    // Create multiple edges between same nodes
    let edge1 = Edge::new(source.clone(), target.clone(), EdgeType::Dependency)?;
    let edge2 = Edge::new(source.clone(), target.clone(), EdgeType::DataFlow)?;
    let edge3 = Edge::new(source, target, EdgeType::Trigger)?;

    assert_eq!(edge1.source(), edge2.source());
    assert_eq!(edge2.source(), edge3.source());
    assert_eq!(edge1.target(), edge2.target());
    assert_eq!(edge2.target(), edge3.target());

    // Each should have different type
    assert_eq!(edge1.edge_type(), EdgeType::Dependency);
    assert_eq!(edge2.edge_type(), EdgeType::DataFlow);
    assert_eq!(edge3.edge_type(), EdgeType::Trigger);

    Ok(())
}

/// Test bidirectional edges (A→B and B→A)
#[test]
fn test_integration_bidirectional_edges() -> Result<(), String> {
    let node1 = NodeId::new("node1")?;
    let node2 = NodeId::new("node2")?;

    let edge_forward = Edge::new(node1.clone(), node2.clone(), EdgeType::Dependency)?;
    let edge_backward = Edge::new(node2, node1, EdgeType::DataFlow)?;

    assert_eq!(edge_forward.source(), edge_backward.target());
    assert_eq!(edge_forward.target(), edge_backward.source());

    Ok(())
}

// ============================================================================
// Numerical Stability Tests
// ============================================================================

/// Test edge cases for numerical stability
#[test]
fn test_numerical_stability_line_paths() -> Result<(), EdgeError> {
    // Very small displacement
    let path1 = calculate_line_path((0.0, 0.0), 0.0, (0.0001, 0.0), 0.0)?;
    assert!(path1.length > 0.0);
    assert!(path1.length < 0.001);

    // Very large displacement
    let path2 = calculate_line_path((0.0, 0.0), 0.0, (1_000_000.0, 0.0), 0.0)?;
    assert!(path2.length > 1_000_000.0);

    // Very small radius
    let path3 = calculate_line_path((0.0, 0.0), 0.000001, (100.0, 0.0), 0.0)?;
    assert!((path3.start.0 - 0.000001).abs() < 1e-10);

    // Very large radius (should be handled gracefully)
    let path4 = calculate_line_path((0.0, 0.0), 1000.0, (100.0, 0.0), 0.0)?;
    assert_eq!(path4.start, (1000.0, 0.0));
    // End point will be at target since radius > distance
    assert_eq!(path4.end, (100.0, 0.0));

    Ok(())
}

/// Test edge cases with floating point precision
#[test]
fn test_floating_point_precision() -> Result<(), EdgeError> {
    // Values that can cause floating point issues
    let path1 = calculate_line_path((0.1, 0.2), 0.0, (0.3, 0.4), 0.0)?;
    assert!(path1.length.is_finite());

    let path2 = calculate_line_path((1e-10, 1e-10), 0.0, (1e10, 1e10), 0.0)?;
    assert!(path2.length.is_finite());

    Ok(())
}

// ============================================================================
// Property-Based Tests for Overall System
// ============================================================================

/// Property test: All edge operations are type-safe and panic-free
proptest! {
    #[test]
    fn prop_edge_operations_panic_free(
        source_id in "[a-z]{1,20}",
        target_id in "[a-z]{1,20}"
    ) {
        prop_assume!(source_id != target_id);

        // Edge creation
        let source = NodeId::new(source_id)
            .map_err(|err| TestCaseError::fail(err.to_string()))?;
        let target = NodeId::new(target_id)
            .map_err(|err| TestCaseError::fail(err.to_string()))?;
        let edge = Edge::new(source.clone(), target.clone(), EdgeType::Dependency)
            .map_err(|err| TestCaseError::fail(err.to_string()))?;

        // State transitions
        edge.set_state(EdgeState::Highlighted);
        edge.set_state(EdgeState::Dimmed);
        edge.set_state(EdgeState::Normal);

        // Style changes
        edge.set_style(EdgeStyle::Dashed);
        edge.set_style(EdgeStyle::Dotted);
        edge.set_style(EdgeStyle::Solid);

        // Label changes
        edge.set_label(Some("test".to_string()));
        edge.set_label(None);

        // All operations should complete without panicking
        prop_assert!(true);
    }
}

/// Property test: Line paths always produce valid coordinates
proptest! {
    #[test]
    fn prop_line_paths_valid_coordinates(
        x1 in -1e6..=1e6f64,
        y1 in -1e6..=1e6f64,
        x2 in -1e6..=1e6f64,
        y2 in -1e6..=1e6f64,
        r1 in 0.0..=1e3f64,
        r2 in 0.0..=1e3f64
    ) {
        // Skip coincident positions
        prop_assume!((x1 - x2).abs() > f64::EPSILON || (y1 - y2).abs() > f64::EPSILON);

        let result = calculate_line_path((x1, y1), r1, (x2, y2), r2);

        // Either success or valid error
        if let Ok(path) = result {
            prop_assert!(path.length.is_finite());
            prop_assert!(path.start.0.is_finite());
            prop_assert!(path.start.1.is_finite());
            prop_assert!(path.end.0.is_finite());
            prop_assert!(path.end.1.is_finite());
        }
    }
}
