//! Bezier curve rendering for parallel edges in DAG visualization
//!
//! This module provides quadratic Bezier curve calculations for rendering parallel edges
//! (edges between the same nodes in the same or opposite directions) with smooth curves
//! to prevent visual overlap.

/// Errors that can occur during edge curve calculations
#[derive(Debug, Clone, PartialEq)]
pub enum EdgeCurveError {
    /// Positions are identical (zero-length edge)
    ZeroLengthEdge,
    /// Invalid offset factor (must be non-negative)
    InvalidOffsetFactor(f64),
    /// Canvas rendering error
    CanvasError(String),
}

impl std::fmt::Display for EdgeCurveError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::ZeroLengthEdge => write!(f, "Edge has zero length (identical source and target positions)"),
            Self::InvalidOffsetFactor(val) => write!(f, "Invalid offset factor: {} (must be non-negative)", val),
            Self::CanvasError(msg) => write!(f, "Canvas rendering error: {}", msg),
        }
    }
}

impl std::error::Error for EdgeCurveError {}

/// A quadratic Bezier curve path with sampled points for rendering
#[derive(Debug, Clone, PartialEq)]
pub struct CurvePath {
    /// Start point (source position)
    pub start: (f64, f64),
    /// Control point (defines curve shape)
    pub control: (f64, f64),
    /// End point (target position)
    pub end: (f64, f64),
    /// Sampled points along curve for hit-testing and rendering
    pub segments: Vec<(f64, f64)>,
}

impl CurvePath {
    /// Creates a new curve path with validation
    ///
    /// # Errors
    /// Returns `EdgeCurveError::ZeroLengthEdge` if start and end are identical
    pub fn new(start: (f64, f64), control: (f64, f64), end: (f64, f64), sample_count: usize) -> Result<Self, EdgeCurveError> {
        // Validate non-zero length
        if (start.0 - end.0).abs() < f64::EPSILON && (start.1 - end.1).abs() < f64::EPSILON {
            return Err(EdgeCurveError::ZeroLengthEdge);
        }

        // Sample the curve
        let segments = sample_quadratic_bezier(start, control, end, sample_count);

        Ok(Self {
            start,
            control,
            end,
            segments,
        })
    }
}

/// Calculates a quadratic Bezier curve for rendering parallel edges
///
/// This function generates a smooth curve between two nodes, with the control point
/// offset perpendicular to the straight line between them. Multiple parallel edges
/// get increasing offsets to prevent overlap.
///
/// # Arguments
/// * `source_pos` - (x, y) position of source node
/// * `target_pos` - (x, y) position of target node
/// * `offset_factor` - Curve intensity: 0.0 = straight line, 1.0 = full curve (must be >= 0.0)
/// * `edge_index` - Zero-based index for multiple parallel edges (0, 1, 2, ...)
///
/// # Returns
/// A `CurvePath` with start, control, end points and sampled segments
///
/// # Errors
/// * `EdgeCurveError::ZeroLengthEdge` - If source and target are at same position
/// * `EdgeCurveError::InvalidOffsetFactor` - If offset_factor is negative
///
/// # Examples
/// ```
/// # use oya_ui::components::canvas::edge_curves::calculate_bezier_curve;
/// let curve = calculate_bezier_curve(
///     (0.0, 0.0),
///     (100.0, 0.0),
///     0.5,
///     0
/// ).unwrap();
/// assert_eq!(curve.start, (0.0, 0.0));
/// assert_eq!(curve.end, (100.0, 0.0));
/// ```
pub fn calculate_bezier_curve(
    source_pos: (f64, f64),
    target_pos: (f64, f64),
    offset_factor: f64,
    edge_index: usize,
) -> Result<CurvePath, EdgeCurveError> {
    // Validate offset factor
    if offset_factor < 0.0 {
        return Err(EdgeCurveError::InvalidOffsetFactor(offset_factor));
    }

    // Calculate midpoint
    let mid_x = (source_pos.0 + target_pos.0) / 2.0;
    let mid_y = (source_pos.1 + target_pos.1) / 2.0;

    // Calculate direction vector
    let dx = target_pos.0 - source_pos.0;
    let dy = target_pos.1 - source_pos.1;

    // Calculate edge length
    let length = (dx * dx + dy * dy).sqrt();

    // Handle zero-length edge
    if length < f64::EPSILON {
        return Err(EdgeCurveError::ZeroLengthEdge);
    }

    // Calculate perpendicular vector (rotate 90 degrees)
    // For right-hand perpendicular: (-dy, dx)
    let perp_x = -dy / length;
    let perp_y = dx / length;

    // Calculate offset distance
    // Base offset is proportional to edge length (20% of length)
    // Additional offset for multiple parallel edges
    let base_offset = length * 0.2 * offset_factor;
    let edge_spacing = 20.0; // pixels between parallel edges
    let edge_multiplier = if edge_index == 0 {
        0.0 // First edge is straight (or slightly curved)
    } else {
        // Alternating sides: 1 -> +1, 2 -> -1, 3 -> +2, 4 -> -2, etc.
        let sign = if edge_index % 2 == 0 { -1.0 } else { 1.0 };
        let magnitude = ((edge_index + 1) / 2) as f64;
        sign * magnitude
    };

    let total_offset = base_offset + (edge_spacing * edge_multiplier);

    // Calculate control point
    let control_x = mid_x + perp_x * total_offset;
    let control_y = mid_y + perp_y * total_offset;

    // Create curve path with 20 sample points
    CurvePath::new(source_pos, (control_x, control_y), target_pos, 20)
}

/// Samples a quadratic Bezier curve into line segments
///
/// Uses the quadratic Bezier formula: B(t) = (1-t)²P₀ + 2(1-t)tP₁ + t²P₂
///
/// # Arguments
/// * `start` - Start point P₀
/// * `control` - Control point P₁
/// * `end` - End point P₂
/// * `sample_count` - Number of points to sample (minimum 2)
///
/// # Returns
/// Vector of (x, y) points along the curve
fn sample_quadratic_bezier(
    start: (f64, f64),
    control: (f64, f64),
    end: (f64, f64),
    sample_count: usize,
) -> Vec<(f64, f64)> {
    let count = sample_count.max(2); // At least start and end
    let mut points = Vec::with_capacity(count);

    for i in 0..count {
        let t = i as f64 / (count - 1) as f64;
        let point = evaluate_quadratic_bezier(start, control, end, t);
        points.push(point);
    }

    points
}

/// Evaluates a quadratic Bezier curve at parameter t
///
/// Formula: B(t) = (1-t)²P₀ + 2(1-t)tP₁ + t²P₂
///
/// # Arguments
/// * `start` - Start point P₀
/// * `control` - Control point P₁
/// * `end` - End point P₂
/// * `t` - Parameter in range [0, 1]
///
/// # Returns
/// Point (x, y) on the curve at parameter t
fn evaluate_quadratic_bezier(
    start: (f64, f64),
    control: (f64, f64),
    end: (f64, f64),
    t: f64,
) -> (f64, f64) {
    let t_clamped = t.clamp(0.0, 1.0);
    let one_minus_t = 1.0 - t_clamped;

    // Quadratic Bezier formula
    let x = one_minus_t * one_minus_t * start.0
        + 2.0 * one_minus_t * t_clamped * control.0
        + t_clamped * t_clamped * end.0;

    let y = one_minus_t * one_minus_t * start.1
        + 2.0 * one_minus_t * t_clamped * control.1
        + t_clamped * t_clamped * end.1;

    (x, y)
}

/// Renders a curve path to an HTML5 Canvas using quadraticCurveTo
///
/// # Arguments
/// * `ctx` - Canvas 2D rendering context
/// * `curve` - Curve path to render
/// * `stroke_style` - CSS color string (e.g., "#4B5563")
/// * `line_width` - Line width in pixels
///
/// # Errors
/// Returns `EdgeCurveError::CanvasError` if rendering fails
pub fn render_curve_to_canvas(
    ctx: &web_sys::CanvasRenderingContext2d,
    curve: &CurvePath,
    stroke_style: &str,
    line_width: f64,
) -> Result<(), EdgeCurveError> {
    // Set style using proper web_sys API
    ctx.set_stroke_style_str(stroke_style);
    ctx.set_line_width(line_width);

    // Begin path
    ctx.begin_path();

    // Move to start
    ctx.move_to(curve.start.0, curve.start.1);

    // Draw quadratic curve
    ctx.quadratic_curve_to(
        curve.control.0,
        curve.control.1,
        curve.end.0,
        curve.end.1,
    );

    // Stroke the path
    ctx.stroke();

    Ok(())
}

/// Detects if two edges are parallel (same endpoints, possibly reversed)
///
/// # Arguments
/// * `edge1` - First edge as (source_id, target_id)
/// * `edge2` - Second edge as (source_id, target_id)
///
/// # Returns
/// True if edges connect the same nodes (in either direction)
pub fn are_edges_parallel(edge1: (&str, &str), edge2: (&str, &str)) -> bool {
    // Same direction: A→B and A→B
    let same_direction = edge1.0 == edge2.0 && edge1.1 == edge2.1;

    // Opposite direction: A→B and B→A
    let opposite_direction = edge1.0 == edge2.1 && edge1.1 == edge2.0;

    same_direction || opposite_direction
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_calculate_bezier_curve_valid() {
        let result = calculate_bezier_curve(
            (0.0, 0.0),
            (100.0, 0.0),
            0.5,
            0,
        );

        assert!(result.is_ok());
        let curve = result.unwrap();
        assert_eq!(curve.start, (0.0, 0.0));
        assert_eq!(curve.end, (100.0, 0.0));
        assert_eq!(curve.segments.len(), 20);
    }

    #[test]
    fn test_calculate_bezier_curve_zero_length() {
        let result = calculate_bezier_curve(
            (50.0, 50.0),
            (50.0, 50.0),
            0.5,
            0,
        );

        assert!(result.is_err());
        assert_eq!(result.unwrap_err(), EdgeCurveError::ZeroLengthEdge);
    }

    #[test]
    fn test_calculate_bezier_curve_negative_offset() {
        let result = calculate_bezier_curve(
            (0.0, 0.0),
            (100.0, 0.0),
            -0.5,
            0,
        );

        assert!(result.is_err());
        match result.unwrap_err() {
            EdgeCurveError::InvalidOffsetFactor(val) => assert_eq!(val, -0.5),
            _ => panic!("Expected InvalidOffsetFactor error"),
        }
    }

    #[test]
    fn test_evaluate_quadratic_bezier_endpoints() {
        let start = (0.0, 0.0);
        let control = (50.0, 100.0);
        let end = (100.0, 0.0);

        // At t=0, should be at start
        let p0 = evaluate_quadratic_bezier(start, control, end, 0.0);
        assert_eq!(p0, start);

        // At t=1, should be at end
        let p1 = evaluate_quadratic_bezier(start, control, end, 1.0);
        assert_eq!(p1, end);
    }

    #[test]
    fn test_evaluate_quadratic_bezier_midpoint() {
        let start = (0.0, 0.0);
        let control = (50.0, 100.0);
        let end = (100.0, 0.0);

        // At t=0.5, calculate expected midpoint
        let p = evaluate_quadratic_bezier(start, control, end, 0.5);

        // Expected: (1-0.5)²*0 + 2*(1-0.5)*0.5*50 + 0.5²*100 = 0 + 25 + 25 = 50
        assert_eq!(p.0, 50.0);

        // Expected: (1-0.5)²*0 + 2*(1-0.5)*0.5*100 + 0.5²*0 = 0 + 50 + 0 = 50
        assert_eq!(p.1, 50.0);
    }

    #[test]
    fn test_sample_quadratic_bezier_count() {
        let start = (0.0, 0.0);
        let control = (50.0, 100.0);
        let end = (100.0, 0.0);

        let points = sample_quadratic_bezier(start, control, end, 10);
        assert_eq!(points.len(), 10);
        assert_eq!(points[0], start);
        assert_eq!(points[9], end);
    }

    #[test]
    fn test_sample_quadratic_bezier_minimum_count() {
        let start = (0.0, 0.0);
        let control = (50.0, 100.0);
        let end = (100.0, 0.0);

        // Request 1 point, should get at least 2 (start and end)
        let points = sample_quadratic_bezier(start, control, end, 1);
        assert_eq!(points.len(), 2);
    }

    #[test]
    fn test_are_edges_parallel_same_direction() {
        assert!(are_edges_parallel(("A", "B"), ("A", "B")));
    }

    #[test]
    fn test_are_edges_parallel_opposite_direction() {
        assert!(are_edges_parallel(("A", "B"), ("B", "A")));
    }

    #[test]
    fn test_are_edges_parallel_different_edges() {
        assert!(!are_edges_parallel(("A", "B"), ("C", "D")));
    }

    #[test]
    fn test_are_edges_parallel_partial_match() {
        assert!(!are_edges_parallel(("A", "B"), ("A", "C")));
    }

    #[test]
    fn test_curve_path_new_valid() {
        let result = CurvePath::new(
            (0.0, 0.0),
            (50.0, 50.0),
            (100.0, 0.0),
            10,
        );

        assert!(result.is_ok());
        let curve = result.unwrap();
        assert_eq!(curve.segments.len(), 10);
    }

    #[test]
    fn test_curve_path_new_zero_length() {
        let result = CurvePath::new(
            (50.0, 50.0),
            (50.0, 100.0),
            (50.0, 50.0),
            10,
        );

        assert!(result.is_err());
        assert_eq!(result.unwrap_err(), EdgeCurveError::ZeroLengthEdge);
    }

    #[test]
    fn test_multiple_parallel_edges_spacing() {
        // Edge 0 should have minimal offset
        let curve0 = calculate_bezier_curve(
            (0.0, 0.0),
            (100.0, 0.0),
            1.0,
            0,
        ).unwrap();

        // Edge 1 should have positive offset
        let curve1 = calculate_bezier_curve(
            (0.0, 0.0),
            (100.0, 0.0),
            1.0,
            1,
        ).unwrap();

        // Edge 2 should have negative offset
        let curve2 = calculate_bezier_curve(
            (0.0, 0.0),
            (100.0, 0.0),
            1.0,
            2,
        ).unwrap();

        // Control points should be at different Y positions
        assert!(curve0.control.1.abs() < 30.0); // Nearly straight
        assert!(curve1.control.1 > 0.0); // Above
        assert!(curve2.control.1 < 0.0); // Below
    }

    #[test]
    fn test_edge_curve_error_display() {
        let err1 = EdgeCurveError::ZeroLengthEdge;
        assert!(err1.to_string().contains("zero length"));

        let err2 = EdgeCurveError::InvalidOffsetFactor(-1.0);
        assert!(err2.to_string().contains("Invalid offset factor"));

        let err3 = EdgeCurveError::CanvasError("test".to_string());
        assert!(err3.to_string().contains("Canvas rendering error"));
    }

    #[test]
    fn test_vertical_edge_curve() {
        // Test vertical edge (dy != 0, dx = 0)
        let result = calculate_bezier_curve(
            (50.0, 0.0),
            (50.0, 100.0),
            0.5,
            0,
        );

        assert!(result.is_ok());
        let curve = result.unwrap();

        // Control point should be offset horizontally (perpendicular to vertical line)
        assert_ne!(curve.control.0, 50.0);
    }

    #[test]
    fn test_diagonal_edge_curve() {
        // Test diagonal edge
        let result = calculate_bezier_curve(
            (0.0, 0.0),
            (100.0, 100.0),
            0.5,
            0,
        );

        assert!(result.is_ok());
        let curve = result.unwrap();

        // Control point should be offset perpendicular to diagonal
        let mid = ((curve.start.0 + curve.end.0) / 2.0, (curve.start.1 + curve.end.1) / 2.0);

        // Distance from control to midpoint should be non-zero
        let dist = ((curve.control.0 - mid.0).powi(2) + (curve.control.1 - mid.1).powi(2)).sqrt();
        assert!(dist > 0.0);
    }
}
