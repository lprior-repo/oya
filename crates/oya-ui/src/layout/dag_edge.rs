//! # DAG Edge Path Calculation
//!
//! Calculates path segments for drawing directed graph edges.

#![deny(clippy::unwrap_used)]
#![deny(clippy::expect_used)]
#![deny(clippy::panic)]
#![warn(clippy::pedantic)]
#![warn(clippy::nursery)]
#![forbid(unsafe_code)]

use std::f64::consts::PI;

use thiserror::Error;

/// Error types for edge calculations
#[derive(Debug, Error, Clone, PartialEq)]
pub enum EdgeError {
    #[error("invalid angle parameter: {0}")]
    InvalidAngle(f64),

    #[error("identical start and end positions")]
    IdenticalPositions,
}

/// A path segment for drawing edge paths
#[derive(Debug, Clone, PartialEq)]
pub struct PathSegment {
    /// Start point (x, y)
    pub start: (f64, f64),
    /// End point (x, y)
    pub end: (f64, f64),
    /// Control point for Bezier curves (optional)
    pub control: Option<(f64, f64)>,
}

impl PathSegment {
    /// Creates a new linear path segment
    #[must_use]
    pub const fn linear(start: (f64, f64), end: (f64, f64)) -> Self {
        Self {
            start,
            end,
            control: None,
        }
    }

    /// Creates a new curved path segment with control point
    #[must_use]
    pub const fn curved(start: (f64, f64), control: (f64, f64), end: (f64, f64)) -> Self {
        Self {
            start,
            end,
            control: Some(control),
        }
    }
}

/// Calculate a path line for a directed edge with arrow
///
/// # Arguments
///
/// * `start` - Starting position (x, y)
/// * `end` - Ending position (x, y)
/// * `arrow_angle` - Angle for arrow head in radians (default: PI/6)
///
/// # Errors
///
/// Returns `EdgeError` if positions are identical
///
/// # Examples
///
/// ```ignore
/// let segments = calculate_line_path((0.0, 0.0), (100.0, 50.0), Some(PI / 6.0))?;
/// ```
pub fn calculate_line_path(
    start: (f64, f64),
    end: (f64, f64),
    arrow_angle: Option<f64>,
) -> Result<Vec<PathSegment>, EdgeError> {
    // Validate positions are different
    let dx = end.0 - start.0;
    let dy = end.1 - start.1;

    let distance = dx.hypot(dy);
    if distance < f64::EPSILON {
        return Err(EdgeError::IdenticalPositions);
    }

    let angle = arrow_angle.map_or_else(
        || Ok(PI / 6.0), // Default 30 degrees
        |a| {
            if a <= 0.0 || a > PI / 2.0 {
                Err(EdgeError::InvalidAngle(a))
            } else {
                Ok(a)
            }
        },
    )?;

    let mut segments = Vec::with_capacity(3);

    // Main line segment
    segments.push(PathSegment::linear(start, end));

    // Arrow head calculations
    let line_angle = dy.atan2(dx);
    let arrow_length = distance * 0.15; // 15% of edge length

    // Left arrow wing
    let left_angle = line_angle + angle;
    let left_end = (
        end.0 + arrow_length * left_angle.cos(),
        end.1 + arrow_length * left_angle.sin(),
    );
    segments.push(PathSegment::linear(end, left_end));

    // Right arrow wing
    let right_angle = line_angle - angle;
    let right_end = (
        end.0 + arrow_length * right_angle.cos(),
        end.1 + arrow_length * right_angle.sin(),
    );
    segments.push(PathSegment::linear(end, right_end));

    Ok(segments)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_linear_segment() {
        let segment = PathSegment::linear((0.0, 0.0), (10.0, 20.0));
        assert_eq!(segment.start, (0.0, 0.0));
        assert_eq!(segment.end, (10.0, 20.0));
        assert!(segment.control.is_none());
    }

    #[test]
    fn test_curved_segment() {
        let segment = PathSegment::curved((0.0, 0.0), (5.0, 10.0), (10.0, 20.0));
        assert_eq!(segment.start, (0.0, 0.0));
        assert_eq!(segment.control, Some((5.0, 10.0)));
        assert_eq!(segment.end, (10.0, 20.0));
    }

    #[test]
    fn test_calculate_line_path_horizontal() {
        let segments = calculate_line_path((0.0, 0.0), (100.0, 0.0), None).unwrap();
        assert_eq!(segments.len(), 3); // Main line + 2 arrow wings
    }

    #[test]
    fn test_calculate_line_path_identical_positions() {
        let result = calculate_line_path((50.0, 50.0), (50.0, 50.0), None);
        assert!(matches!(result, Err(EdgeError::IdenticalPositions)));
    }

    #[test]
    fn test_calculate_line_path_invalid_angle() {
        let result = calculate_line_path((0.0, 0.0), (100.0, 0.0), Some(2.0));
        assert!(matches!(result, Err(EdgeError::InvalidAngle(_))));
    }
}
