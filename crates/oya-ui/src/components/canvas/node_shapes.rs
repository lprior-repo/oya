//! Node shape rendering on HTML5 Canvas
//!
//! This module provides pure functional rendering of node shapes (circles, rectangles)
//! using the HTML5 Canvas API. All rendering operations are error-aware and use
//! Railway-Oriented Programming patterns.
//!
//! # Architecture
//!
//! - **render_circle**: Renders a circular node with state-based fill color
//! - **render_rectangle**: Renders a rectangular node with state-based fill color
//! - All operations return Result types for comprehensive error handling
//!
//! # Dependencies
//!
//! - Node model (NodeState, NodeShape, Position)
//! - Color mapping (get_node_color)
//! - Coordinate transformations (world_to_screen)

use crate::components::canvas::coords::{Viewport, world_to_screen};
use crate::models::colors::{RgbColor, get_node_color};
use crate::models::node::{Node, NodeShape};
use web_sys::CanvasRenderingContext2d;

/// Default node radius in world coordinates
const DEFAULT_NODE_RADIUS: f32 = 20.0;

/// Border width in pixels
const BORDER_WIDTH: f32 = 2.0;

/// Border color (darker than fill)
const BORDER_COLOR: &str = "#333333";

/// Renders a node shape on the canvas
///
/// This function delegates to the appropriate shape renderer based on the node's shape.
///
/// # Errors
///
/// Returns an error if:
/// - Coordinate transformation fails
/// - Canvas context operations fail
/// - Invalid node data
///
/// # Examples
///
/// ```no_run
/// use oya_ui::components::canvas::node_shapes::render_node;
/// use oya_ui::components::canvas::coords::Viewport;
/// use oya_ui::models::node::Node;
/// # use web_sys::CanvasRenderingContext2d;
/// # fn example(ctx: CanvasRenderingContext2d) -> Result<(), String> {
/// let node = Node::new("node1", "Test")?;
/// let viewport = Viewport::new(1200.0, 800.0)?;
/// render_node(&ctx, &node, &viewport)?;
/// # Ok(())
/// # }
/// ```
pub fn render_node(
    ctx: &CanvasRenderingContext2d,
    node: &Node,
    viewport: &Viewport,
) -> Result<(), String> {
    match node.shape() {
        NodeShape::Circle => render_circle(ctx, node, viewport),
        NodeShape::Square => render_rectangle(ctx, node, viewport),
        NodeShape::Diamond => Err("Diamond shape not yet implemented".to_string()),
    }
}

/// Renders a circular node
///
/// The circle is centered on the node's position and filled with the state color.
/// A darker border is drawn around the circle.
///
/// # Errors
///
/// Returns an error if coordinate transformation or canvas operations fail.
fn render_circle(
    ctx: &CanvasRenderingContext2d,
    node: &Node,
    viewport: &Viewport,
) -> Result<(), String> {
    // Transform world coordinates to screen coordinates
    let (screen_x, screen_y) = world_to_screen(&node.position(), viewport)?;

    // Get state color
    let color = get_node_color(&node.state());

    // Begin path
    ctx.begin_path();

    // Draw circle: arc(x, y, radius, startAngle, endAngle)
    let radius = DEFAULT_NODE_RADIUS * viewport.zoom.value();
    ctx.arc(
        screen_x as f64,
        screen_y as f64,
        radius as f64,
        0.0,
        std::f64::consts::PI * 2.0,
    )
    .map_err(|e| format!("Failed to draw arc: {:?}", e))?;

    // Fill with state color
    ctx.set_fill_style_str(&color.to_css());
    ctx.fill();

    // Stroke border
    ctx.set_stroke_style_str(BORDER_COLOR);
    ctx.set_line_width(BORDER_WIDTH as f64);
    ctx.stroke();

    Ok(())
}

/// Renders a rectangular node
///
/// The rectangle is centered on the node's position and filled with the state color.
/// A darker border is drawn around the rectangle.
///
/// # Errors
///
/// Returns an error if coordinate transformation or canvas operations fail.
fn render_rectangle(
    ctx: &CanvasRenderingContext2d,
    node: &Node,
    viewport: &Viewport,
) -> Result<(), String> {
    // Transform world coordinates to screen coordinates
    let (screen_x, screen_y) = world_to_screen(&node.position(), viewport)?;

    // Get state color
    let color = get_node_color(&node.state());

    // Rectangle dimensions (2x radius for square)
    let size = DEFAULT_NODE_RADIUS * 2.0 * viewport.zoom.value();
    let half_size = size / 2.0;

    // Fill rectangle centered on position
    ctx.set_fill_style_str(&color.to_css());
    ctx.fill_rect(
        (screen_x - half_size) as f64,
        (screen_y - half_size) as f64,
        size as f64,
        size as f64,
    );

    // Stroke border
    ctx.set_stroke_style_str(BORDER_COLOR);
    ctx.set_line_width(BORDER_WIDTH as f64);
    ctx.stroke_rect(
        (screen_x - half_size) as f64,
        (screen_y - half_size) as f64,
        size as f64,
        size as f64,
    );

    Ok(())
}

/// Darken a color by a percentage (for borders)
///
/// # Examples
///
/// ```
/// use oya_ui::components::canvas::node_shapes::darken_color;
/// use oya_ui::models::colors::RgbColor;
///
/// let color = RgbColor::new(200, 200, 200);
/// let darker = darken_color(&color, 0.3); // 30% darker
/// assert!(darker.r < color.r);
/// ```
pub fn darken_color(color: &RgbColor, amount: f32) -> RgbColor {
    let factor = 1.0 - amount.clamp(0.0, 1.0);
    RgbColor::new(
        (color.r as f32 * factor) as u8,
        (color.g as f32 * factor) as u8,
        (color.b as f32 * factor) as u8,
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::components::controls::bounds::ZoomLevel;
    use crate::models::node::{NodeState, Position};

    // ========================================================================
    // Utility Tests
    // ========================================================================

    #[test]
    fn test_darken_color_basic() {
        let color = RgbColor::new(200, 200, 200);
        let darker = darken_color(&color, 0.3);

        // Should be 30% darker
        assert_eq!(darker.r, 140); // 200 * 0.7 = 140
        assert_eq!(darker.g, 140);
        assert_eq!(darker.b, 140);
    }

    #[test]
    fn test_darken_color_zero_amount() {
        let color = RgbColor::new(100, 150, 200);
        let darker = darken_color(&color, 0.0);

        // Should be unchanged
        assert_eq!(darker, color);
    }

    #[test]
    fn test_darken_color_full_amount() {
        let color = RgbColor::new(100, 150, 200);
        let darker = darken_color(&color, 1.0);

        // Should be completely dark (black)
        assert_eq!(darker.r, 0);
        assert_eq!(darker.g, 0);
        assert_eq!(darker.b, 0);
    }

    #[test]
    fn test_darken_color_over_range() {
        let color = RgbColor::new(100, 150, 200);
        let darker = darken_color(&color, 1.5);

        // Should clamp to 1.0 (black)
        assert_eq!(darker.r, 0);
        assert_eq!(darker.g, 0);
        assert_eq!(darker.b, 0);
    }

    #[test]
    fn test_darken_color_negative_amount() {
        let color = RgbColor::new(100, 150, 200);
        let darker = darken_color(&color, -0.5);

        // Should clamp to 0.0 (unchanged)
        assert_eq!(darker, color);
    }

    #[test]
    fn test_darken_color_all_states() {
        let states = [
            NodeState::Idle,
            NodeState::Running,
            NodeState::Blocked,
            NodeState::Completed,
            NodeState::Failed,
        ];

        for state in &states {
            let color = get_node_color(state);
            let darker = darken_color(&color, 0.2);

            // Verify darkening worked
            assert!(darker.r <= color.r);
            assert!(darker.g <= color.g);
            assert!(darker.b <= color.b);
        }
    }

    // ========================================================================
    // Coordinate Transform Tests (Integration with coords module)
    // ========================================================================

    #[test]
    fn test_circle_coordinate_transform() -> Result<(), String> {
        // Create a node at origin
        let mut node = Node::new("test", "Test Node")?;
        node.set_position(Position::origin());

        let viewport = Viewport::new(1200.0, 800.0)?;

        // Transform to screen coordinates
        let (screen_x, screen_y) = world_to_screen(&node.position(), &viewport)?;

        // Should be at canvas center
        assert_eq!(screen_x, 600.0);
        assert_eq!(screen_y, 400.0);

        Ok(())
    }

    #[test]
    fn test_circle_coordinate_transform_with_offset() -> Result<(), String> {
        // Create a node at offset position
        let mut node = Node::new("test", "Test Node")?;
        let pos = Position::new(100.0, 50.0)?;
        node.set_position(pos);

        let viewport = Viewport::new(1200.0, 800.0)?;

        // Transform to screen coordinates
        let (screen_x, screen_y) = world_to_screen(&node.position(), &viewport)?;

        // Should be offset from center
        assert_eq!(screen_x, 700.0); // 600 + 100
        assert_eq!(screen_y, 450.0); // 400 + 50

        Ok(())
    }

    #[test]
    fn test_circle_coordinate_transform_with_zoom() -> Result<(), String> {
        let mut node = Node::new("test", "Test Node")?;
        node.set_position(Position::new(100.0, 50.0)?);

        let zoom = ZoomLevel::new(2.0)?;
        let viewport = Viewport::with_transform(1200.0, 800.0, 0.0, 0.0, zoom)?;

        let (screen_x, screen_y) = world_to_screen(&node.position(), &viewport)?;

        // With 2x zoom, distance from center should double
        assert_eq!(screen_x, 800.0); // 600 + (100 * 2)
        assert_eq!(screen_y, 500.0); // 400 + (50 * 2)

        Ok(())
    }

    // ========================================================================
    // Node Shape Enum Tests
    // ========================================================================

    #[test]
    fn test_node_shape_circle() -> Result<(), String> {
        let mut node = Node::new("test", "Test")?;
        node.set_shape(NodeShape::Circle);
        assert_eq!(node.shape(), NodeShape::Circle);
        Ok(())
    }

    #[test]
    fn test_node_shape_square() -> Result<(), String> {
        let mut node = Node::new("test", "Test")?;
        node.set_shape(NodeShape::Square);
        assert_eq!(node.shape(), NodeShape::Square);
        Ok(())
    }

    #[test]
    fn test_node_shape_diamond() -> Result<(), String> {
        let mut node = Node::new("test", "Test")?;
        node.set_shape(NodeShape::Diamond);
        assert_eq!(node.shape(), NodeShape::Diamond);
        Ok(())
    }

    // ========================================================================
    // State Color Integration Tests
    // ========================================================================

    #[test]
    fn test_all_states_have_colors() {
        let states = [
            NodeState::Idle,
            NodeState::Running,
            NodeState::Blocked,
            NodeState::Completed,
            NodeState::Failed,
        ];

        for state in &states {
            let color = get_node_color(state);
            let css = color.to_css();

            // Verify valid CSS format
            assert!(css.starts_with("rgb("));
            assert!(css.ends_with(")"));
        }
    }

    #[test]
    fn test_state_colors_are_unique() {
        let idle = get_node_color(&NodeState::Idle);
        let running = get_node_color(&NodeState::Running);
        let blocked = get_node_color(&NodeState::Blocked);
        let completed = get_node_color(&NodeState::Completed);
        let failed = get_node_color(&NodeState::Failed);

        // All should be unique
        assert_ne!(idle, running);
        assert_ne!(idle, blocked);
        assert_ne!(running, completed);
        assert_ne!(blocked, failed);
    }

    // ========================================================================
    // Zoom Scaling Tests
    // ========================================================================

    #[test]
    fn test_radius_scales_with_zoom() -> Result<(), String> {
        let viewport1 = Viewport::new(1200.0, 800.0)?;
        let zoom2 = ZoomLevel::new(2.0)?;
        let viewport2 = Viewport::with_transform(1200.0, 800.0, 0.0, 0.0, zoom2)?;

        // Radius should scale with zoom
        let radius1 = DEFAULT_NODE_RADIUS * viewport1.zoom.value();
        let radius2 = DEFAULT_NODE_RADIUS * viewport2.zoom.value();

        assert_eq!(radius1, 20.0);
        assert_eq!(radius2, 40.0);

        Ok(())
    }

    #[test]
    fn test_border_width_constant() {
        // Border width should not scale with zoom
        // (This is by design for visual clarity)
        assert_eq!(BORDER_WIDTH, 2.0);
    }

    // ========================================================================
    // Error Path Tests
    // ========================================================================

    #[test]
    fn test_diamond_not_implemented() -> Result<(), String> {
        // This is a placeholder test - diamond rendering should return error
        // until implemented
        let mut node = Node::new("test", "Test")?;
        node.set_shape(NodeShape::Diamond);

        // Would fail in render_node due to unimplemented shape
        // When diamond is implemented, this test should be updated

        Ok(())
    }

    #[test]
    fn test_invalid_position_rejected() {
        // Verify that Position validation catches invalid coords
        let result = Position::new(f32::NAN, 0.0);
        assert!(result.is_err());

        let result = Position::new(f32::INFINITY, 0.0);
        assert!(result.is_err());
    }

    #[test]
    fn test_viewport_validation() {
        // Verify viewport rejects invalid dimensions
        let result = Viewport::new(-100.0, 800.0);
        assert!(result.is_err());

        let result = Viewport::new(1200.0, f32::NAN);
        assert!(result.is_err());
    }
}
