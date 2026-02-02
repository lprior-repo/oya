//! Arrow head rendering for DAG edges
//!
//! This module provides pure functional arrow head calculation and rendering
//! for directed edges in the dependency graph.

use std::f64::consts::PI;

/// Arrow style for edge endpoints
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ArrowStyle {
    /// Filled solid triangle
    Filled,
    /// Stroke outline only
    Outline,
    /// No arrow (for weak dependencies)
    None,
}

impl Default for ArrowStyle {
    fn default() -> Self {
        Self::Filled
    }
}

/// Arrow head geometry as three triangle vertices
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ArrowPath {
    /// Tip of the arrow (at edge target)
    pub tip: (f64, f64),
    /// First wing vertex
    pub wing1: (f64, f64),
    /// Second wing vertex
    pub wing2: (f64, f64),
}

/// Error type for arrow calculations
#[derive(Debug, Clone, PartialEq)]
pub enum ArrowError {
    /// Direction vector has zero length
    ZeroDirectionVector,
    /// Invalid arrow dimensions (negative or NaN)
    InvalidDimensions { length: f64, width: f64 },
}

impl std::fmt::Display for ArrowError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::ZeroDirectionVector => write!(f, "Direction vector has zero length"),
            Self::InvalidDimensions { length, width } => {
                write!(
                    f,
                    "Invalid arrow dimensions: length={}, width={}",
                    length, width
                )
            }
        }
    }
}

impl std::error::Error for ArrowError {}

/// Default arrow head length in pixels
pub const DEFAULT_ARROW_LENGTH: f64 = 12.0;

/// Default arrow head width in pixels
pub const DEFAULT_ARROW_WIDTH: f64 = 8.0;

/// Calculates arrow head triangle vertices
///
/// # Arguments
/// * `edge_end` - Target endpoint coordinates (x, y)
/// * `direction` - Direction vector pointing toward target (does not need to be normalized)
/// * `arrow_length` - Length of arrow from tip to base
/// * `arrow_width` - Width of arrow at base
///
/// # Returns
/// * `Ok(ArrowPath)` - Triangle vertices for the arrow head
/// * `Err(ArrowError)` - If direction is zero or dimensions are invalid
///
/// # Algorithm
/// 1. Validate inputs (non-zero direction, positive dimensions)
/// 2. Normalize direction vector
/// 3. Calculate perpendicular vector for arrow wings
/// 4. Compute base point by moving backward from tip
/// 5. Compute wing points perpendicular to direction
///
/// # Example
/// ```
/// use oya_ui::components::canvas::arrows::{calculate_arrow_head, DEFAULT_ARROW_LENGTH, DEFAULT_ARROW_WIDTH};
///
/// let tip = (100.0, 100.0);
/// let direction = (1.0, 0.0);  // Pointing right
/// let arrow = calculate_arrow_head(tip, direction, DEFAULT_ARROW_LENGTH, DEFAULT_ARROW_WIDTH)
///     .expect("Valid arrow");
///
/// assert_eq!(arrow.tip, (100.0, 100.0));
/// // Wings are behind and to sides of tip
/// ```
pub fn calculate_arrow_head(
    edge_end: (f64, f64),
    direction: (f64, f64),
    arrow_length: f64,
    arrow_width: f64,
) -> Result<ArrowPath, ArrowError> {
    // Validate dimensions
    if !arrow_length.is_finite()
        || !arrow_width.is_finite()
        || arrow_length <= 0.0
        || arrow_width <= 0.0
    {
        return Err(ArrowError::InvalidDimensions {
            length: arrow_length,
            width: arrow_width,
        });
    }

    // Calculate direction magnitude
    let magnitude = (direction.0 * direction.0 + direction.1 * direction.1).sqrt();

    if magnitude < f64::EPSILON {
        return Err(ArrowError::ZeroDirectionVector);
    }

    // Normalize direction vector
    let norm_dir = (direction.0 / magnitude, direction.1 / magnitude);

    // Calculate perpendicular vector (rotate 90 degrees)
    // For vector (dx, dy), perpendicular is (-dy, dx)
    let perpendicular = (-norm_dir.1, norm_dir.0);

    // Calculate base point (move backward from tip along direction)
    let base_x = edge_end.0 - norm_dir.0 * arrow_length;
    let base_y = edge_end.1 - norm_dir.1 * arrow_length;

    // Calculate wing points (perpendicular offset from base)
    let half_width = arrow_width / 2.0;
    let wing1 = (
        base_x + perpendicular.0 * half_width,
        base_y + perpendicular.1 * half_width,
    );
    let wing2 = (
        base_x - perpendicular.0 * half_width,
        base_y - perpendicular.1 * half_width,
    );

    Ok(ArrowPath {
        tip: edge_end,
        wing1,
        wing2,
    })
}

/// Scales arrow dimensions based on zoom level
///
/// # Arguments
/// * `base_length` - Base arrow length at zoom=1.0
/// * `base_width` - Base arrow width at zoom=1.0
/// * `zoom_factor` - Current zoom level (1.0 = 100%)
///
/// # Returns
/// Scaled dimensions as (length, width) tuple
///
/// # Example
/// ```
/// use oya_ui::components::canvas::arrows::{scale_arrow_with_zoom, DEFAULT_ARROW_LENGTH, DEFAULT_ARROW_WIDTH};
///
/// let (length, width) = scale_arrow_with_zoom(DEFAULT_ARROW_LENGTH, DEFAULT_ARROW_WIDTH, 2.0);
/// assert_eq!(length, 24.0);  // Doubled
/// assert_eq!(width, 16.0);   // Doubled
/// ```
pub fn scale_arrow_with_zoom(base_length: f64, base_width: f64, zoom_factor: f64) -> (f64, f64) {
    (base_length * zoom_factor, base_width * zoom_factor)
}

/// Calculates arrow direction from edge endpoints
///
/// Returns normalized direction vector pointing from source to target.
///
/// # Arguments
/// * `source` - Start point of edge
/// * `target` - End point of edge
///
/// # Returns
/// * `Ok((dx, dy))` - Normalized direction vector
/// * `Err(ArrowError)` - If source equals target
///
/// # Example
/// ```
/// use oya_ui::components::canvas::arrows::calculate_edge_direction;
///
/// let direction = calculate_edge_direction((0.0, 0.0), (10.0, 0.0))
///     .expect("Valid direction");
/// assert_eq!(direction, (1.0, 0.0));  // Pointing right
/// ```
pub fn calculate_edge_direction(
    source: (f64, f64),
    target: (f64, f64),
) -> Result<(f64, f64), ArrowError> {
    let dx = target.0 - source.0;
    let dy = target.1 - source.1;
    let magnitude = (dx * dx + dy * dy).sqrt();

    if magnitude < f64::EPSILON {
        return Err(ArrowError::ZeroDirectionVector);
    }

    Ok((dx / magnitude, dy / magnitude))
}

/// Calculates tangent direction for Bezier curve at endpoint
///
/// For a quadratic Bezier curve with control point,
/// the tangent at t=1.0 (endpoint) is (target - control).
///
/// # Arguments
/// * `control` - Bezier control point
/// * `target` - Bezier endpoint
///
/// # Returns
/// * `Ok((dx, dy))` - Normalized tangent vector at endpoint
/// * `Err(ArrowError)` - If control equals target
///
/// # Example
/// ```
/// use oya_ui::components::canvas::arrows::calculate_bezier_tangent;
///
/// let tangent = calculate_bezier_tangent((50.0, 50.0), (100.0, 100.0))
///     .expect("Valid tangent");
/// // Tangent points from control to target
/// ```
pub fn calculate_bezier_tangent(
    control: (f64, f64),
    target: (f64, f64),
) -> Result<(f64, f64), ArrowError> {
    calculate_edge_direction(control, target)
}

#[cfg(test)]
mod tests {
    use super::*;

    const EPSILON: f64 = 1e-10;

    fn approx_eq(a: f64, b: f64) -> bool {
        (a - b).abs() < EPSILON
    }

    fn approx_eq_point(p1: (f64, f64), p2: (f64, f64)) -> bool {
        approx_eq(p1.0, p2.0) && approx_eq(p1.1, p2.1)
    }

    #[test]
    fn test_arrow_pointing_right() {
        let tip = (100.0, 100.0);
        let direction = (1.0, 0.0);
        let result = calculate_arrow_head(tip, direction, 12.0, 8.0);

        assert!(result.is_ok());
        let arrow = result.unwrap();

        assert_eq!(arrow.tip, (100.0, 100.0));
        // Base should be 12 pixels to the left
        assert!(approx_eq(arrow.wing1.0, 88.0));
        assert!(approx_eq(arrow.wing2.0, 88.0));
        // Wings should be 4 pixels above and below
        assert!(approx_eq(arrow.wing1.1, 104.0));
        assert!(approx_eq(arrow.wing2.1, 96.0));
    }

    #[test]
    fn test_arrow_pointing_up() {
        let tip = (100.0, 100.0);
        let direction = (0.0, -1.0); // Pointing up (negative Y)
        let result = calculate_arrow_head(tip, direction, 12.0, 8.0);

        assert!(result.is_ok());
        let arrow = result.unwrap();

        assert_eq!(arrow.tip, (100.0, 100.0));
        // Base should be 12 pixels below (positive Y)
        assert!(approx_eq(arrow.wing1.1, 112.0));
        assert!(approx_eq(arrow.wing2.1, 112.0));
        // Wings should be 4 pixels left and right
        assert!(approx_eq(arrow.wing1.0, 104.0));
        assert!(approx_eq(arrow.wing2.0, 96.0));
    }

    #[test]
    fn test_arrow_pointing_diagonal() {
        let tip = (100.0, 100.0);
        let direction = (1.0, 1.0); // 45 degrees
        let result = calculate_arrow_head(tip, direction, 12.0, 8.0);

        assert!(result.is_ok());
        let arrow = result.unwrap();

        assert_eq!(arrow.tip, (100.0, 100.0));
        // Base should be sqrt(2)*12/2 back in both X and Y
        let offset = 12.0 / 2.0_f64.sqrt();
        assert!(approx_eq(
            arrow.wing1.0 + arrow.wing2.0,
            2.0 * (100.0 - offset)
        ));
        assert!(approx_eq(
            arrow.wing1.1 + arrow.wing2.1,
            2.0 * (100.0 - offset)
        ));
    }

    #[test]
    fn test_unnormalized_direction() {
        let tip = (100.0, 100.0);
        let direction = (3.0, 4.0); // Magnitude 5
        let result = calculate_arrow_head(tip, direction, 10.0, 6.0);

        assert!(result.is_ok());
        let arrow = result.unwrap();

        // Should normalize to (0.6, 0.8)
        // Base at tip - 10*(0.6, 0.8) = (94, 92)
        assert!(approx_eq((arrow.wing1.0 + arrow.wing2.0) / 2.0, 94.0));
        assert!(approx_eq((arrow.wing1.1 + arrow.wing2.1) / 2.0, 92.0));
    }

    #[test]
    fn test_zero_direction_error() {
        let tip = (100.0, 100.0);
        let direction = (0.0, 0.0);
        let result = calculate_arrow_head(tip, direction, 12.0, 8.0);

        assert!(result.is_err());
        assert_eq!(result.unwrap_err(), ArrowError::ZeroDirectionVector);
    }

    #[test]
    fn test_invalid_dimensions() {
        let tip = (100.0, 100.0);
        let direction = (1.0, 0.0);

        // Negative length
        let result = calculate_arrow_head(tip, direction, -12.0, 8.0);
        assert!(result.is_err());

        // Zero width
        let result = calculate_arrow_head(tip, direction, 12.0, 0.0);
        assert!(result.is_err());

        // NaN
        let result = calculate_arrow_head(tip, direction, f64::NAN, 8.0);
        assert!(result.is_err());
    }

    #[test]
    fn test_scale_arrow_with_zoom() {
        let (length, width) = scale_arrow_with_zoom(12.0, 8.0, 2.0);
        assert_eq!(length, 24.0);
        assert_eq!(width, 16.0);

        let (length, width) = scale_arrow_with_zoom(12.0, 8.0, 0.5);
        assert_eq!(length, 6.0);
        assert_eq!(width, 4.0);
    }

    #[test]
    fn test_calculate_edge_direction_horizontal() {
        let result = calculate_edge_direction((0.0, 0.0), (10.0, 0.0));
        assert!(result.is_ok());
        let direction = result.unwrap();
        assert!(approx_eq_point(direction, (1.0, 0.0)));
    }

    #[test]
    fn test_calculate_edge_direction_vertical() {
        let result = calculate_edge_direction((0.0, 0.0), (0.0, 10.0));
        assert!(result.is_ok());
        let direction = result.unwrap();
        assert!(approx_eq_point(direction, (0.0, 1.0)));
    }

    #[test]
    fn test_calculate_edge_direction_diagonal() {
        let result = calculate_edge_direction((0.0, 0.0), (3.0, 4.0));
        assert!(result.is_ok());
        let direction = result.unwrap();
        assert!(approx_eq_point(direction, (0.6, 0.8)));
    }

    #[test]
    fn test_calculate_edge_direction_same_point() {
        let result = calculate_edge_direction((5.0, 5.0), (5.0, 5.0));
        assert!(result.is_err());
        assert_eq!(result.unwrap_err(), ArrowError::ZeroDirectionVector);
    }

    #[test]
    fn test_calculate_bezier_tangent() {
        let control = (50.0, 50.0);
        let target = (100.0, 100.0);
        let result = calculate_bezier_tangent(control, target);

        assert!(result.is_ok());
        let tangent = result.unwrap();

        // Tangent should point from control to target
        // Direction: (50, 50), magnitude: sqrt(5000)
        let expected = (1.0 / 2.0_f64.sqrt(), 1.0 / 2.0_f64.sqrt());
        assert!(approx_eq_point(tangent, expected));
    }

    #[test]
    fn test_arrow_style_default() {
        let style = ArrowStyle::default();
        assert_eq!(style, ArrowStyle::Filled);
    }

    #[test]
    fn test_arrow_path_clone() {
        let path = ArrowPath {
            tip: (1.0, 2.0),
            wing1: (3.0, 4.0),
            wing2: (5.0, 6.0),
        };
        let cloned = path.clone();
        assert_eq!(path, cloned);
    }

    #[test]
    fn test_arrow_error_display() {
        let err = ArrowError::ZeroDirectionVector;
        assert_eq!(err.to_string(), "Direction vector has zero length");

        let err = ArrowError::InvalidDimensions {
            length: -1.0,
            width: 8.0,
        };
        assert!(err.to_string().contains("Invalid arrow dimensions"));
    }

    #[test]
    fn test_various_angles() {
        let tip = (100.0, 100.0);
        let angles = [0.0, PI / 4.0, PI / 2.0, 3.0 * PI / 4.0, PI];

        for angle in angles {
            let direction = (angle.cos(), angle.sin());
            let result = calculate_arrow_head(tip, direction, 12.0, 8.0);
            assert!(result.is_ok(), "Failed at angle {}", angle);

            let arrow = result.unwrap();
            assert_eq!(arrow.tip, tip);

            // Verify wings are equidistant from tip
            let dist1 = ((arrow.wing1.0 - tip.0).powi(2) + (arrow.wing1.1 - tip.1).powi(2)).sqrt();
            let dist2 = ((arrow.wing2.0 - tip.0).powi(2) + (arrow.wing2.1 - tip.1).powi(2)).sqrt();
            assert!(
                approx_eq(dist1, dist2),
                "Wings not equidistant at angle {}",
                angle
            );
        }
    }

    #[test]
    fn test_constants() {
        assert_eq!(DEFAULT_ARROW_LENGTH, 12.0);
        assert_eq!(DEFAULT_ARROW_WIDTH, 8.0);
    }
}
