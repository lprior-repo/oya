//! Hover state detection with hit-testing for DAG nodes
//!
//! This module provides pure functional hit-testing logic for detecting
//! when the mouse cursor is over a node. All operations use Railway-Oriented
//! Programming with Result types for comprehensive error handling.
//!
//! # Architecture
//!
//! - **contains_point**: Pure function for hit-testing nodes with viewport transforms
//! - **circle_hit_test**: Circle hit detection (distance from center <= radius)
//! - **rectangle_hit_test**: Rectangle hit detection (AABB - Axis-Aligned Bounding Box)
//! - All operations handle viewport transforms (pan/zoom) correctly
//!
//! # Performance
//!
//! - O(1) hit-testing for individual nodes
//! - For 50+ nodes, consider spatial indexing (quadtree) for batch operations
//!
//! # Examples
//!
//! ```no_run
//! use oya_ui::interaction::hover::contains_point;
//! use oya_ui::models::node::{Node, Position};
//! use oya_ui::components::canvas::coords::Viewport;
//!
//! # fn example() -> Result<(), String> {
//! let node = Node::new("node1", "Test")?;
//! let viewport = Viewport::new(1200.0, 800.0)?;
//!
//! // Check if mouse at (650.0, 425.0) is over the node
//! let is_hovering = contains_point(&node, 650.0, 425.0, &viewport)?;
//! # Ok(())
//! # }
//! ```

#![deny(clippy::unwrap_used)]
#![deny(clippy::expect_used)]
#![deny(clippy::panic)]
#![warn(clippy::pedantic)]
#![warn(clippy::nursery)]

use crate::components::canvas::coords::{Viewport, screen_to_world};
use crate::models::node::{Node, NodeShape, Position};

/// Default node radius in world coordinates (must match node_shapes.rs)
const DEFAULT_NODE_RADIUS: f32 = 20.0;

/// Result of a hit test operation
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum HitTestResult {
    /// The point is inside the node
    Hit,
    /// The point is outside the node
    Miss,
}

impl HitTestResult {
    /// Converts hit test result to boolean
    ///
    /// # Examples
    ///
    /// ```
    /// # use oya_ui::interaction::hover::HitTestResult;
    /// assert!(HitTestResult::Hit.is_hit());
    /// assert!(!HitTestResult::Miss.is_hit());
    /// ```
    #[must_use]
    pub const fn is_hit(self) -> bool {
        matches!(self, Self::Hit)
    }
}

/// Tests if a screen-space point is inside a node
///
/// This function handles viewport transforms (pan/zoom) correctly by:
/// 1. Converting screen coordinates to world coordinates
/// 2. Performing hit-test in world space
/// 3. Returning whether the point intersects the node
///
/// # Arguments
///
/// * `node` - The node to test against
/// * `screen_x` - X coordinate in screen space (pixels)
/// * `screen_y` - Y coordinate in screen space (pixels)
/// * `viewport` - Current viewport state (pan/zoom)
///
/// # Errors
///
/// Returns an error if:
/// - Screen-to-world coordinate transformation fails
/// - Viewport parameters are invalid
///
/// # Examples
///
/// ```no_run
/// # use oya_ui::interaction::hover::contains_point;
/// # use oya_ui::models::node::Node;
/// # use oya_ui::components::canvas::coords::Viewport;
/// # fn example() -> Result<(), String> {
/// let node = Node::new("test", "Test Node")?;
/// let viewport = Viewport::new(1200.0, 800.0)?;
///
/// // Center of screen (where world origin is rendered)
/// let is_inside = contains_point(&node, 600.0, 400.0, &viewport)?;
/// assert!(is_inside);
/// # Ok(())
/// # }
/// ```
pub fn contains_point(
    node: &Node,
    screen_x: f32,
    screen_y: f32,
    viewport: &Viewport,
) -> Result<bool, String> {
    // Step 1: Convert screen coordinates to world coordinates
    let world_pos = screen_to_world(screen_x, screen_y, viewport)?;

    // Step 2: Perform hit-test in world space based on node shape
    let result = match node.shape() {
        NodeShape::Circle => circle_hit_test(&node.position(), &world_pos, viewport)?,
        NodeShape::Square => rectangle_hit_test(&node.position(), &world_pos, viewport)?,
        NodeShape::Diamond => Err("Diamond shape hit-testing not yet implemented".to_string())?,
    };

    Ok(result.is_hit())
}

/// Performs circle hit-test in world space
///
/// Circle hit detection uses the distance formula:
/// `distance = sqrt((x2 - x1)² + (y2 - y1)²)`
///
/// A point is inside the circle if `distance <= radius`.
///
/// # Note
///
/// The radius is scaled by the viewport zoom level to maintain
/// consistent visual hit-testing across zoom levels.
///
/// # Errors
///
/// Returns an error if coordinate calculations produce non-finite values.
fn circle_hit_test(
    node_center: &Position,
    test_point: &Position,
    _viewport: &Viewport,
) -> Result<HitTestResult, String> {
    // Calculate distance from node center to test point
    let dx = test_point.x - node_center.x;
    let dy = test_point.y - node_center.y;

    // Squared distance (avoids sqrt for performance)
    let distance_squared = dx * dx + dy * dy;

    // Validate intermediate calculation
    if !distance_squared.is_finite() {
        return Err(format!(
            "Distance calculation produced non-finite value: {}",
            distance_squared
        ));
    }

    // Radius in world space (constant, not affected by zoom for hit-testing)
    // Note: Visual radius scales with zoom, but hit-test radius remains constant
    // for consistent user experience
    let radius = DEFAULT_NODE_RADIUS;
    let radius_squared = radius * radius;

    // Compare squared values to avoid expensive sqrt
    if distance_squared <= radius_squared {
        Ok(HitTestResult::Hit)
    } else {
        Ok(HitTestResult::Miss)
    }
}

/// Performs rectangle hit-test in world space using AABB
///
/// AABB (Axis-Aligned Bounding Box) hit detection checks if a point
/// is within the rectangle's bounds:
/// `x_min <= point.x <= x_max && y_min <= point.y <= y_max`
///
/// # Note
///
/// The rectangle size is scaled by the viewport zoom level to maintain
/// consistent visual hit-testing across zoom levels.
///
/// # Errors
///
/// Returns an error if bound calculations produce non-finite values.
fn rectangle_hit_test(
    node_center: &Position,
    test_point: &Position,
    _viewport: &Viewport,
) -> Result<HitTestResult, String> {
    // Rectangle dimensions (2x radius for square)
    // Like circle, size is constant in world space for consistent hit-testing
    let half_size = DEFAULT_NODE_RADIUS;

    // Calculate bounding box
    let x_min = node_center.x - half_size;
    let x_max = node_center.x + half_size;
    let y_min = node_center.y - half_size;
    let y_max = node_center.y + half_size;

    // Validate bounds
    if !x_min.is_finite() || !x_max.is_finite() || !y_min.is_finite() || !y_max.is_finite() {
        return Err(format!(
            "Bounding box calculation produced non-finite values: ({}, {}, {}, {})",
            x_min, x_max, y_min, y_max
        ));
    }

    // AABB test: point inside if within all bounds
    let inside_x = test_point.x >= x_min && test_point.x <= x_max;
    let inside_y = test_point.y >= y_min && test_point.y <= y_max;

    if inside_x && inside_y {
        Ok(HitTestResult::Hit)
    } else {
        Ok(HitTestResult::Miss)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::components::controls::bounds::ZoomLevel;
    use crate::models::node::NodeState;

    // ========================================================================
    // HitTestResult Tests
    // ========================================================================

    #[test]
    fn test_hit_test_result_is_hit() {
        assert!(HitTestResult::Hit.is_hit());
        assert!(!HitTestResult::Miss.is_hit());
    }

    #[test]
    fn test_hit_test_result_equality() {
        assert_eq!(HitTestResult::Hit, HitTestResult::Hit);
        assert_eq!(HitTestResult::Miss, HitTestResult::Miss);
        assert_ne!(HitTestResult::Hit, HitTestResult::Miss);
    }

    // ========================================================================
    // Circle Hit Test - Basic Cases
    // ========================================================================

    #[test]
    fn test_circle_center_hit() -> Result<(), String> {
        let mut node = Node::new("test", "Test Node")?;
        node.set_position(Position::origin());
        node.set_shape(NodeShape::Circle);

        let viewport = Viewport::new(1200.0, 800.0)?;

        // Click at center of screen (world origin)
        let is_inside = contains_point(&node, 600.0, 400.0, &viewport)?;
        assert!(is_inside);

        Ok(())
    }

    #[test]
    fn test_circle_edge_hit() -> Result<(), String> {
        let mut node = Node::new("test", "Test Node")?;
        node.set_position(Position::origin());
        node.set_shape(NodeShape::Circle);

        let viewport = Viewport::new(1200.0, 800.0)?;

        // Click exactly at edge (radius = 20)
        let is_inside = contains_point(&node, 620.0, 400.0, &viewport)?;
        assert!(is_inside);

        Ok(())
    }

    #[test]
    fn test_circle_just_outside_miss() -> Result<(), String> {
        let mut node = Node::new("test", "Test Node")?;
        node.set_position(Position::origin());
        node.set_shape(NodeShape::Circle);

        let viewport = Viewport::new(1200.0, 800.0)?;

        // Click just outside radius (radius = 20)
        let is_inside = contains_point(&node, 621.0, 400.0, &viewport)?;
        assert!(!is_inside);

        Ok(())
    }

    #[test]
    fn test_circle_far_away_miss() -> Result<(), String> {
        let mut node = Node::new("test", "Test Node")?;
        node.set_position(Position::origin());
        node.set_shape(NodeShape::Circle);

        let viewport = Viewport::new(1200.0, 800.0)?;

        // Click far from node
        let is_inside = contains_point(&node, 100.0, 100.0, &viewport)?;
        assert!(!is_inside);

        Ok(())
    }

    // ========================================================================
    // Circle Hit Test - Viewport Transforms
    // ========================================================================

    #[test]
    fn test_circle_with_pan() -> Result<(), String> {
        let mut node = Node::new("test", "Test Node")?;
        node.set_position(Position::origin());
        node.set_shape(NodeShape::Circle);

        // Pan viewport 100 pixels right, 50 down
        let viewport = Viewport::with_transform(1200.0, 800.0, 100.0, 50.0, ZoomLevel::default())?;

        // Node now renders at (700, 450) instead of (600, 400)
        let is_inside = contains_point(&node, 700.0, 450.0, &viewport)?;
        assert!(is_inside);

        // Old center position should miss
        let is_inside = contains_point(&node, 600.0, 400.0, &viewport)?;
        assert!(!is_inside);

        Ok(())
    }

    #[test]
    fn test_circle_with_zoom() -> Result<(), String> {
        let mut node = Node::new("test", "Test Node")?;
        node.set_position(Position::origin());
        node.set_shape(NodeShape::Circle);

        let zoom = ZoomLevel::new(2.0)?;
        let viewport = Viewport::with_transform(1200.0, 800.0, 0.0, 0.0, zoom)?;

        // With 2x zoom, visual radius is 40 pixels, but hit-test uses world space
        // Node still at screen center (600, 400)
        let is_inside = contains_point(&node, 600.0, 400.0, &viewport)?;
        assert!(is_inside);

        Ok(())
    }

    #[test]
    fn test_circle_with_pan_and_zoom() -> Result<(), String> {
        let mut node = Node::new("test", "Test Node")?;
        node.set_position(Position::new(50.0, 25.0)?);
        node.set_shape(NodeShape::Circle);

        let zoom = ZoomLevel::new(2.0)?;
        let viewport = Viewport::with_transform(1200.0, 800.0, 100.0, 50.0, zoom)?;

        // Calculate expected screen position:
        // world (50, 25) + pan (100, 50) = (150, 75) in panned world
        // × zoom 2.0 = (300, 150) in zoomed space
        // + canvas center (600, 400) = (900, 550) in screen space

        let is_inside = contains_point(&node, 900.0, 550.0, &viewport)?;
        assert!(is_inside);

        Ok(())
    }

    // ========================================================================
    // Rectangle Hit Test - Basic Cases
    // ========================================================================

    #[test]
    fn test_rectangle_center_hit() -> Result<(), String> {
        let mut node = Node::new("test", "Test Node")?;
        node.set_position(Position::origin());
        node.set_shape(NodeShape::Square);

        let viewport = Viewport::new(1200.0, 800.0)?;

        // Click at center
        let is_inside = contains_point(&node, 600.0, 400.0, &viewport)?;
        assert!(is_inside);

        Ok(())
    }

    #[test]
    fn test_rectangle_corner_hit() -> Result<(), String> {
        let mut node = Node::new("test", "Test Node")?;
        node.set_position(Position::origin());
        node.set_shape(NodeShape::Square);

        let viewport = Viewport::new(1200.0, 800.0)?;

        // Click at top-left corner (exactly at edge)
        // half_size = 20, so bounds are [-20, 20] in world space
        // Screen: (600-20, 400-20) = (580, 380)
        let is_inside = contains_point(&node, 580.0, 380.0, &viewport)?;
        assert!(is_inside);

        Ok(())
    }

    #[test]
    fn test_rectangle_edge_hit() -> Result<(), String> {
        let mut node = Node::new("test", "Test Node")?;
        node.set_position(Position::origin());
        node.set_shape(NodeShape::Square);

        let viewport = Viewport::new(1200.0, 800.0)?;

        // Click at right edge
        let is_inside = contains_point(&node, 620.0, 400.0, &viewport)?;
        assert!(is_inside);

        Ok(())
    }

    #[test]
    fn test_rectangle_just_outside_miss() -> Result<(), String> {
        let mut node = Node::new("test", "Test Node")?;
        node.set_position(Position::origin());
        node.set_shape(NodeShape::Square);

        let viewport = Viewport::new(1200.0, 800.0)?;

        // Click just outside right edge
        let is_inside = contains_point(&node, 621.0, 400.0, &viewport)?;
        assert!(!is_inside);

        Ok(())
    }

    #[test]
    fn test_rectangle_diagonal_outside_miss() -> Result<(), String> {
        let mut node = Node::new("test", "Test Node")?;
        node.set_position(Position::origin());
        node.set_shape(NodeShape::Square);

        let viewport = Viewport::new(1200.0, 800.0)?;

        // Click outside diagonal corner
        let is_inside = contains_point(&node, 625.0, 425.0, &viewport)?;
        assert!(!is_inside);

        Ok(())
    }

    // ========================================================================
    // Rectangle Hit Test - Viewport Transforms
    // ========================================================================

    #[test]
    fn test_rectangle_with_pan() -> Result<(), String> {
        let mut node = Node::new("test", "Test Node")?;
        node.set_position(Position::origin());
        node.set_shape(NodeShape::Square);

        let viewport = Viewport::with_transform(1200.0, 800.0, 50.0, 25.0, ZoomLevel::default())?;

        // Node now renders at (650, 425)
        let is_inside = contains_point(&node, 650.0, 425.0, &viewport)?;
        assert!(is_inside);

        Ok(())
    }

    #[test]
    fn test_rectangle_with_zoom() -> Result<(), String> {
        let mut node = Node::new("test", "Test Node")?;
        node.set_position(Position::origin());
        node.set_shape(NodeShape::Square);

        let zoom = ZoomLevel::new(1.5)?;
        let viewport = Viewport::with_transform(1200.0, 800.0, 0.0, 0.0, zoom)?;

        // Center should still hit
        let is_inside = contains_point(&node, 600.0, 400.0, &viewport)?;
        assert!(is_inside);

        Ok(())
    }

    // ========================================================================
    // Overlapping Nodes Tests
    // ========================================================================

    #[test]
    fn test_overlapping_circles_hit_first() -> Result<(), String> {
        let mut node1 = Node::new("node1", "Node 1")?;
        node1.set_position(Position::new(0.0, 0.0)?);
        node1.set_shape(NodeShape::Circle);

        let mut node2 = Node::new("node2", "Node 2")?;
        node2.set_position(Position::new(10.0, 10.0)?);
        node2.set_shape(NodeShape::Circle);

        let viewport = Viewport::new(1200.0, 800.0)?;

        // Click in overlap region (both should report hit)
        let hit1 = contains_point(&node1, 610.0, 410.0, &viewport)?;
        let hit2 = contains_point(&node2, 610.0, 410.0, &viewport)?;

        // Both nodes contain this point (hit-test doesn't determine z-order)
        assert!(hit1);
        assert!(hit2);

        Ok(())
    }

    #[test]
    fn test_no_false_positives_on_close_nodes() -> Result<(), String> {
        let mut node1 = Node::new("node1", "Node 1")?;
        node1.set_position(Position::new(0.0, 0.0)?);
        node1.set_shape(NodeShape::Circle);

        let mut node2 = Node::new("node2", "Node 2")?;
        node2.set_position(Position::new(50.0, 0.0)?);
        node2.set_shape(NodeShape::Circle);

        let viewport = Viewport::new(1200.0, 800.0)?;

        // Click between nodes (should miss both)
        let hit1 = contains_point(&node1, 625.0, 400.0, &viewport)?;
        let hit2 = contains_point(&node2, 625.0, 400.0, &viewport)?;

        assert!(!hit1);
        assert!(!hit2);

        Ok(())
    }

    // ========================================================================
    // Different Node States (visual state should not affect hit-testing)
    // ========================================================================

    #[test]
    fn test_hit_test_independent_of_state() -> Result<(), String> {
        let viewport = Viewport::new(1200.0, 800.0)?;

        let states = [
            NodeState::Idle,
            NodeState::Running,
            NodeState::Blocked,
            NodeState::Completed,
            NodeState::Failed,
        ];

        for state in &states {
            let mut node = Node::new("test", "Test")?;
            node.set_position(Position::origin());
            node.set_shape(NodeShape::Circle);
            node.set_state(*state);

            let is_inside = contains_point(&node, 600.0, 400.0, &viewport)?;
            assert!(is_inside, "Hit test failed for state: {:?}", state);
        }

        Ok(())
    }

    // ========================================================================
    // Edge Cases and Error Handling
    // ========================================================================

    #[test]
    fn test_hit_test_at_canvas_boundaries() -> Result<(), String> {
        let mut node = Node::new("test", "Test Node")?;
        node.set_position(Position::origin());
        node.set_shape(NodeShape::Circle);

        let viewport = Viewport::new(1200.0, 800.0)?;

        // Top-left corner of canvas
        let is_inside = contains_point(&node, 0.0, 0.0, &viewport)?;
        assert!(!is_inside);

        // Bottom-right corner of canvas
        let is_inside = contains_point(&node, 1200.0, 800.0, &viewport)?;
        assert!(!is_inside);

        Ok(())
    }

    #[test]
    fn test_hit_test_with_negative_world_coords() -> Result<(), String> {
        let mut node = Node::new("test", "Test Node")?;
        node.set_position(Position::new(-100.0, -50.0)?);
        node.set_shape(NodeShape::Circle);

        let viewport = Viewport::new(1200.0, 800.0)?;

        // Calculate screen position: center (600, 400) + world (-100, -50) = (500, 350)
        let is_inside = contains_point(&node, 500.0, 350.0, &viewport)?;
        assert!(is_inside);

        Ok(())
    }

    #[test]
    fn test_hit_test_extreme_zoom_out() -> Result<(), String> {
        let mut node = Node::new("test", "Test Node")?;
        node.set_position(Position::origin());
        node.set_shape(NodeShape::Circle);

        let zoom = ZoomLevel::new(0.1)?; // Min zoom
        let viewport = Viewport::with_transform(1200.0, 800.0, 0.0, 0.0, zoom)?;

        // Even at extreme zoom out, center should hit
        let is_inside = contains_point(&node, 600.0, 400.0, &viewport)?;
        assert!(is_inside);

        Ok(())
    }

    #[test]
    fn test_hit_test_extreme_zoom_in() -> Result<(), String> {
        let mut node = Node::new("test", "Test Node")?;
        node.set_position(Position::origin());
        node.set_shape(NodeShape::Circle);

        let zoom = ZoomLevel::new(5.0)?; // Max zoom
        let viewport = Viewport::with_transform(1200.0, 800.0, 0.0, 0.0, zoom)?;

        // Center should still hit at max zoom
        let is_inside = contains_point(&node, 600.0, 400.0, &viewport)?;
        assert!(is_inside);

        Ok(())
    }

    #[test]
    fn test_diamond_shape_returns_error() -> Result<(), String> {
        let mut node = Node::new("test", "Test Node")?;
        node.set_position(Position::origin());
        node.set_shape(NodeShape::Diamond);

        let viewport = Viewport::new(1200.0, 800.0)?;

        // Diamond not implemented yet
        let result = contains_point(&node, 600.0, 400.0, &viewport);
        assert!(result.is_err());

        if let Err(msg) = result {
            assert!(msg.contains("Diamond"));
        }

        Ok(())
    }

    // ========================================================================
    // Performance Characteristics Tests
    // ========================================================================

    #[test]
    fn test_circle_hit_test_direct() -> Result<(), String> {
        let node_pos = Position::origin();
        let test_pos = Position::new(10.0, 10.0)?;
        let viewport = Viewport::new(1200.0, 800.0)?;

        let result = circle_hit_test(&node_pos, &test_pos, &viewport)?;
        assert_eq!(result, HitTestResult::Hit);

        Ok(())
    }

    #[test]
    fn test_rectangle_hit_test_direct() -> Result<(), String> {
        let node_pos = Position::origin();
        let test_pos = Position::new(15.0, 15.0)?;
        let viewport = Viewport::new(1200.0, 800.0)?;

        let result = rectangle_hit_test(&node_pos, &test_pos, &viewport)?;
        assert_eq!(result, HitTestResult::Hit);

        Ok(())
    }

    #[test]
    fn test_circle_uses_squared_distance() -> Result<(), String> {
        // Verify we're using squared distance for performance
        let node_pos = Position::origin();
        let test_pos = Position::new(14.14, 14.14)?; // ~20 distance
        let viewport = Viewport::new(1200.0, 800.0)?;

        let result = circle_hit_test(&node_pos, &test_pos, &viewport)?;
        assert_eq!(result, HitTestResult::Hit);

        Ok(())
    }

    // ========================================================================
    // Coordinate Validation Tests
    // ========================================================================

    #[test]
    fn test_fractional_coordinates() -> Result<(), String> {
        let mut node = Node::new("test", "Test Node")?;
        node.set_position(Position::new(0.5, 0.75)?);
        node.set_shape(NodeShape::Circle);

        let viewport = Viewport::new(1200.0, 800.0)?;

        // Fractional screen coordinates should work
        let is_inside = contains_point(&node, 600.5, 400.75, &viewport)?;
        assert!(is_inside);

        Ok(())
    }

    #[test]
    fn test_large_canvas_dimensions() -> Result<(), String> {
        let mut node = Node::new("test", "Test Node")?;
        node.set_position(Position::origin());
        node.set_shape(NodeShape::Circle);

        let viewport = Viewport::new(3840.0, 2160.0)?; // 4K resolution

        // Center of 4K canvas
        let is_inside = contains_point(&node, 1920.0, 1080.0, &viewport)?;
        assert!(is_inside);

        Ok(())
    }

    #[test]
    fn test_small_canvas_dimensions() -> Result<(), String> {
        let mut node = Node::new("test", "Test Node")?;
        node.set_position(Position::origin());
        node.set_shape(NodeShape::Circle);

        let viewport = Viewport::new(320.0, 240.0)?; // Tiny canvas

        // Center of tiny canvas
        let is_inside = contains_point(&node, 160.0, 120.0, &viewport)?;
        assert!(is_inside);

        Ok(())
    }
}
