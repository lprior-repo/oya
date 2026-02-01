//! Node label text rendering with automatic truncation
//!
//! Provides functional, panic-free text rendering for node labels on canvas.
//! Handles text measurement, truncation with ellipsis, and proper positioning
//! relative to node shapes.

use web_sys::CanvasRenderingContext2d;

use crate::models::node::{Node, NodeShape};

/// Font family for node labels
const LABEL_FONT_FAMILY: &str = "12px Inter, sans-serif";

/// Text color for labels (dark gray)
const LABEL_COLOR: &str = "#1F2937";

/// Maximum width for label text before truncation
const MAX_LABEL_WIDTH: f64 = 120.0;

/// Default node radius for circle shapes
const DEFAULT_CIRCLE_RADIUS: f64 = 20.0;

/// Default node height for rectangle shapes
const DEFAULT_RECT_HEIGHT: f64 = 40.0;

/// Ellipsis character for truncated text
const ELLIPSIS: &str = "...";

/// Y-offset for circle node labels (radius + offset)
const CIRCLE_LABEL_OFFSET: f64 = 16.0;

/// Y-offset for rectangle node labels (height/2 + offset)
const RECT_LABEL_OFFSET: f64 = 12.0;

/// Truncate text to fit within max width using canvas text measurement
///
/// # Errors
///
/// Returns an error if:
/// - measureText() fails (JS error)
/// - TextMetrics is not available
///
/// # Example
///
/// ```no_run
/// use oya_ui::components::canvas::node_labels::truncate_text;
/// use oya_ui::components::canvas::context::get_2d_context;
/// use oya_ui::components::canvas::{create_canvas, CanvasConfig};
///
/// let config = CanvasConfig::default();
/// let canvas = create_canvas(&config)?;
/// let ctx = get_2d_context(&canvas)?;
/// ctx.set_font("12px Inter, sans-serif");
///
/// let text = "Very long node label that needs truncation";
/// let truncated = truncate_text(&ctx, text, 120.0)?;
/// # Ok::<(), String>(())
/// ```
pub fn truncate_text(
    ctx: &CanvasRenderingContext2d,
    text: &str,
    max_width: f64,
) -> Result<String, String> {
    // Handle empty text
    if text.is_empty() {
        return Ok(String::new());
    }

    // Measure full text width
    let metrics = ctx
        .measure_text(text)
        .map_err(|e| format!("Failed to measure text: {:?}", e))?;

    let text_width = metrics.width();

    // No truncation needed
    if text_width <= max_width {
        return Ok(text.to_string());
    }

    // Binary search for optimal truncation point
    let mut left = 0;
    let mut right = text.len();
    let mut best_fit = String::new();

    while left <= right {
        let mid = (left + right) / 2;

        // Get substring at valid UTF-8 boundary
        let substr = match text.get(..mid) {
            Some(s) => s,
            None => {
                // Not on char boundary, move left
                right = mid.saturating_sub(1);
                continue;
            }
        };

        let test_text = format!("{}{}", substr, ELLIPSIS);

        let test_metrics = ctx
            .measure_text(&test_text)
            .map_err(|e| format!("Failed to measure truncated text: {:?}", e))?;

        if test_metrics.width() <= max_width {
            best_fit = test_text;
            left = mid + 1;
        } else {
            right = mid.saturating_sub(1);
        }
    }

    // If no fit found, return just ellipsis
    if best_fit.is_empty() {
        Ok(ELLIPSIS.to_string())
    } else {
        Ok(best_fit)
    }
}

/// Calculate Y-offset for label based on node shape
///
/// Returns the vertical offset from the node's center where the label should be rendered.
///
/// # Arguments
///
/// * `shape` - The node's shape (Circle or Square)
/// * `radius` - Optional radius for circle shapes (defaults to 20px)
/// * `height` - Optional height for rectangle shapes (defaults to 40px)
///
/// # Example
///
/// ```
/// use oya_ui::components::canvas::node_labels::calculate_label_position;
/// use oya_ui::models::node::NodeShape;
///
/// let circle_offset = calculate_label_position(NodeShape::Circle, Some(25.0), None);
/// assert_eq!(circle_offset, 25.0 + 16.0); // radius + offset
///
/// let rect_offset = calculate_label_position(NodeShape::Square, None, Some(50.0));
/// assert_eq!(rect_offset, 25.0 + 12.0); // height/2 + offset
/// ```
pub fn calculate_label_position(shape: NodeShape, radius: Option<f64>, height: Option<f64>) -> f64 {
    match shape {
        NodeShape::Circle => {
            let r = radius.unwrap_or(DEFAULT_CIRCLE_RADIUS);
            r + CIRCLE_LABEL_OFFSET
        }
        NodeShape::Square => {
            let h = height.unwrap_or(DEFAULT_RECT_HEIGHT);
            h / 2.0 + RECT_LABEL_OFFSET
        }
        NodeShape::Diamond => {
            // Diamond uses same logic as rectangle for now
            let h = height.unwrap_or(DEFAULT_RECT_HEIGHT);
            h / 2.0 + RECT_LABEL_OFFSET
        }
    }
}

/// Render a node's label on the canvas
///
/// Applies font styling, measures and truncates text if needed, and renders
/// the label centered horizontally below the node shape.
///
/// # Errors
///
/// Returns an error if:
/// - Setting font style fails
/// - Text measurement fails
/// - fillText() fails
///
/// # Example
///
/// ```no_run
/// use oya_ui::components::canvas::node_labels::render_node_label;
/// use oya_ui::components::canvas::{create_canvas, CanvasConfig};
/// use oya_ui::components::canvas::context::get_2d_context;
/// use oya_ui::models::node::Node;
///
/// let config = CanvasConfig::default();
/// let canvas = create_canvas(&config)?;
/// let ctx = get_2d_context(&canvas)?;
///
/// let node = Node::with_position("node1", "Test Node", 100.0, 100.0)?;
/// render_node_label(&ctx, &node, None, None)?;
/// # Ok::<(), String>(())
/// ```
pub fn render_node_label(
    ctx: &CanvasRenderingContext2d,
    node: &Node,
    radius: Option<f64>,
    height: Option<f64>,
) -> Result<(), String> {
    // Set font style
    ctx.set_font(LABEL_FONT_FAMILY);
    ctx.set_fill_style_str(LABEL_COLOR);

    // Get label text
    let label = node.label();

    // Truncate if needed
    let display_text = truncate_text(ctx, label, MAX_LABEL_WIDTH)?;

    // Calculate position
    let pos = node.position();
    let y_offset = calculate_label_position(node.shape(), radius, height);

    // Calculate x position for centered text
    let metrics = ctx
        .measure_text(&display_text)
        .map_err(|e| format!("Failed to measure display text: {:?}", e))?;

    let text_width = metrics.width();
    let x = f64::from(pos.x) - (text_width / 2.0);
    let y = f64::from(pos.y) + y_offset;

    // Render text
    ctx.fill_text(&display_text, x, y)
        .map_err(|e| format!("Failed to render text: {:?}", e))?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_calculate_label_position_circle_default() {
        let offset = calculate_label_position(NodeShape::Circle, None, None);
        assert_eq!(offset, DEFAULT_CIRCLE_RADIUS + CIRCLE_LABEL_OFFSET);
    }

    #[test]
    fn test_calculate_label_position_circle_custom() {
        let offset = calculate_label_position(NodeShape::Circle, Some(30.0), None);
        assert_eq!(offset, 30.0 + CIRCLE_LABEL_OFFSET);
    }

    #[test]
    fn test_calculate_label_position_rectangle_default() {
        let offset = calculate_label_position(NodeShape::Square, None, None);
        assert_eq!(offset, DEFAULT_RECT_HEIGHT / 2.0 + RECT_LABEL_OFFSET);
    }

    #[test]
    fn test_calculate_label_position_rectangle_custom() {
        let offset = calculate_label_position(NodeShape::Square, None, Some(60.0));
        assert_eq!(offset, 30.0 + RECT_LABEL_OFFSET);
    }

    #[test]
    fn test_calculate_label_position_diamond() {
        let offset = calculate_label_position(NodeShape::Diamond, None, Some(50.0));
        assert_eq!(offset, 25.0 + RECT_LABEL_OFFSET);
    }
}

#[cfg(test)]
#[cfg(target_arch = "wasm32")]
mod wasm_tests {
    use super::*;
    use crate::components::canvas::context::get_2d_context;
    use crate::components::canvas::init::{CanvasConfig, create_canvas};
    use wasm_bindgen_test::*;

    wasm_bindgen_test_configure!(run_in_browser);

    #[wasm_bindgen_test]
    fn test_truncate_text_empty() -> Result<(), Box<dyn std::error::Error>> {
        let config = CanvasConfig {
            width: 800,
            height: 600,
            id: "test-label-1".to_string(),
        };

        let canvas = create_canvas(&config)?;
        let ctx = get_2d_context(&canvas)?;
        ctx.set_font(LABEL_FONT_FAMILY);

        let result = truncate_text(&ctx, "", 120.0)?;
        assert_eq!(result, "");
        Ok(())
    }

    #[wasm_bindgen_test]
    fn test_truncate_text_short() -> Result<(), Box<dyn std::error::Error>> {
        let config = CanvasConfig {
            width: 800,
            height: 600,
            id: "test-label-2".to_string(),
        };

        let canvas = create_canvas(&config)?;
        let ctx = get_2d_context(&canvas)?;
        ctx.set_font(LABEL_FONT_FAMILY);

        let short_text = "Short";
        let result = truncate_text(&ctx, short_text, 120.0)?;
        assert_eq!(result, short_text);
        Ok(())
    }

    #[wasm_bindgen_test]
    fn test_truncate_text_long() -> Result<(), Box<dyn std::error::Error>> {
        let config = CanvasConfig {
            width: 800,
            height: 600,
            id: "test-label-3".to_string(),
        };

        let canvas = create_canvas(&config)?;
        let ctx = get_2d_context(&canvas)?;
        ctx.set_font(LABEL_FONT_FAMILY);

        let long_text = "This is a very long node label that definitely needs truncation";
        let truncated = truncate_text(&ctx, long_text, 120.0)?;

        assert!(truncated.ends_with(ELLIPSIS));
        assert!(truncated.len() < long_text.len());

        // Verify truncated text fits
        let metrics = ctx.measure_text(&truncated)?;
        assert!(metrics.width() <= 120.0);
        Ok(())
    }

    #[wasm_bindgen_test]
    fn test_truncate_text_unicode() {
        let config = CanvasConfig {
            width: 800,
            height: 600,
            id: "test-label-4".to_string(),
        };

        let canvas = create_canvas(&config).expect("Canvas creation");
        let ctx = get_2d_context(&canvas).expect("Context creation");
        ctx.set_font(LABEL_FONT_FAMILY);

        let unicode_text = "节点标签很长需要截断处理Unicode字符";
        let result = truncate_text(&ctx, unicode_text, 120.0);
        assert!(result.is_ok());

        // Should not panic on multibyte chars
        let truncated = result.expect("Unicode truncation");
        assert!(truncated.ends_with(ELLIPSIS) || truncated == unicode_text);
    }

    #[wasm_bindgen_test]
    fn test_render_node_label_circle() {
        let config = CanvasConfig {
            width: 800,
            height: 600,
            id: "test-label-5".to_string(),
        };

        let canvas = create_canvas(&config).expect("Canvas creation");
        let ctx = get_2d_context(&canvas).expect("Context creation");

        let node = Node::with_position("node1", "Test Node", 100.0, 100.0).expect("Node creation");

        let result = render_node_label(&ctx, &node, Some(25.0), None);
        assert!(result.is_ok());
    }

    #[wasm_bindgen_test]
    fn test_render_node_label_long_text() {
        let config = CanvasConfig {
            width: 800,
            height: 600,
            id: "test-label-6".to_string(),
        };

        let canvas = create_canvas(&config).expect("Canvas creation");
        let ctx = get_2d_context(&canvas).expect("Context creation");

        let node = Node::with_position(
            "node2",
            "Very long node label that should be truncated automatically",
            200.0,
            200.0,
        )
        .expect("Node creation");

        let result = render_node_label(&ctx, &node, None, None);
        assert!(result.is_ok());
    }

    #[wasm_bindgen_test]
    fn test_render_node_label_empty() {
        let config = CanvasConfig {
            width: 800,
            height: 600,
            id: "test-label-7".to_string(),
        };

        let canvas = create_canvas(&config).expect("Canvas creation");
        let ctx = get_2d_context(&canvas).expect("Context creation");

        let node = Node::with_position("node3", "", 150.0, 150.0).expect("Node creation");

        let result = render_node_label(&ctx, &node, None, None);
        assert!(result.is_ok());
    }
}
