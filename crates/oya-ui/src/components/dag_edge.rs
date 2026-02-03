#![deny(clippy::unwrap_used)]
#![deny(clippy::expect_used)]
#![deny(clippy::panic)]
#![warn(clippy::pedantic)]
#![warn(clippy::nursery)]
#![forbid(unsafe_code)]

use thiserror::Error;

/// Error types for edge path calculations
#[derive(Debug, Error, Clone, PartialEq)]
pub enum EdgeError {
    #[error("nodes are at identical positions (zero-length edge)")]
    CoincidentNodes,

    #[error("invalid radius: {0} must be non-negative")]
    InvalidRadius(f64),
}

/// Represents a 2D position
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Position {
    x: f64,
    y: f64,
}

impl Position {
    /// Creates a new position from coordinates
    #[must_use]
    pub const fn new(x: f64, y: f64) -> Self {
        Self { x, y }
    }

    /// Creates a position from a tuple
    #[must_use]
    pub const fn from_tuple(pos: (f64, f64)) -> Self {
        Self { x: pos.0, y: pos.1 }
    }

    /// Returns the position as a tuple
    #[must_use]
    pub const fn as_tuple(&self) -> (f64, f64) {
        (self.x, self.y)
    }

    /// Calculates the Euclidean distance to another position
    #[must_use]
    pub fn distance(&self, other: &Self) -> f64 {
        let dx = other.x - self.x;
        let dy = other.y - self.y;
        dx.hypot(dy)
    }

    /// Returns the normalized direction vector to another position
    ///
    /// # Errors
    ///
    /// Returns `EdgeError::CoincidentNodes` if positions are identical
    pub fn direction_to(&self, other: &Self) -> Result<(f64, f64), EdgeError> {
        let distance = self.distance(other);
        if distance < f64::EPSILON {
            Err(EdgeError::CoincidentNodes)
        } else {
            let dx = other.x - self.x;
            let dy = other.y - self.y;
            Ok((dx / distance, dy / distance))
        }
    }
}

/// Represents a line path segment between two points
#[derive(Debug, Clone, PartialEq)]
pub struct PathSegment {
    pub start: (f64, f64),
    pub end: (f64, f64),
    pub length: f64,
}

impl PathSegment {
    /// Creates a new path segment
    #[must_use]
    pub const fn new(start: (f64, f64), end: (f64, f64)) -> Self {
        let dx = end.0 - start.0;
        let dy = end.1 - start.1;
        let length = (dx * dx + dy * dy).sqrt();
        Self { start, end, length }
    }

    /// Validates that a radius is non-negative
    ///
    /// # Errors
    ///
    /// Returns `EdgeError::InvalidRadius` if radius is negative
    fn validate_radius(radius: f64) -> Result<(), EdgeError> {
        if radius < 0.0 {
            Err(EdgeError::InvalidRadius(radius))
        } else {
            Ok(())
        }
    }
}

/// Calculates a straight line path from source node center to target node center,
/// adjusting endpoints to node boundaries.
///
/// This function implements the algorithm:
/// 1. Calculate direction vector from source to target
/// 2. Normalize direction
/// 3. Adjust start point: move from source center outward by source_radius
/// 4. Adjust end point: move from target center inward by target_radius
/// 5. Return the adjusted line segment
///
/// # Arguments
///
/// * `source_pos` - (x, y) position of source node center
/// * `source_radius` - radius of source node (for circle/ellipse shapes)
/// * `target_pos` - (x, y) position of target node center
/// * `target_radius` - radius of target node (for circle/ellipse shapes)
///
/// # Returns
///
/// A `PathSegment` containing:
/// - `start`: adjusted start point on source node boundary
/// - `end`: adjusted end point on target node boundary
/// - `length`: Euclidean distance between start and end points
///
/// # Errors
///
/// * `EdgeError::CoincidentNodes` - if source and target positions are identical
/// * `EdgeError::InvalidRadius` - if either radius is negative
///
/// # Examples
///
/// ```
/// # use oya_ui::dag_edge::{calculate_line_path, EdgeError};
///
/// // Horizontal line from (0, 0) to (100, 0) with radius 10
/// let path = calculate_line_path((0.0, 0.0), 10.0, (100.0, 0.0), 10.0)?;
/// assert_eq!(path.start, (10.0, 0.0));
/// assert_eq!(path.end, (90.0, 0.0));
/// assert_eq!(path.length, 80.0);
/// # Ok::<(), EdgeError>(())
/// ```
///
/// # Edge Cases
///
/// - **Coincident nodes**: Returns `EdgeError::CoincidentNodes`
/// - **Overlapping nodes**: Still calculates path correctly (length may be negative)
/// - **Zero radius**: Treats node as a point (no offset)
pub fn calculate_line_path(
    source_pos: (f64, f64),
    source_radius: f64,
    target_pos: (f64, f64),
    target_radius: f64,
) -> Result<PathSegment, EdgeError> {
    // Validate radii are non-negative
    PathSegment::validate_radius(source_radius)
        .and_then(|_| PathSegment::validate_radius(target_radius))?;

    // Create Position objects
    let source = Position::from_tuple(source_pos);
    let target = Position::from_tuple(target_pos);

    // Get normalized direction vector from source to target
    let (dir_x, dir_y) = source.direction_to(&target)?;

    // Adjust start point: move from source center outward by source_radius
    let start = (
        source.x + dir_x * source_radius,
        source.y + dir_y * source_radius,
    );

    // Adjust end point: move from target center inward by target_radius
    let end = (
        target.x - dir_x * target_radius,
        target.y - dir_y * target_radius,
    );

    // Create path segment (length is calculated automatically)
    Ok(PathSegment::new(start, end))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_position_creation() {
        let pos = Position::new(10.0, 20.0);
        assert_eq!(pos.x, 10.0);
        assert_eq!(pos.y, 20.0);
    }

    #[test]
    fn test_position_from_tuple() {
        let pos = Position::from_tuple((5.0, 15.0));
        assert_eq!(pos.as_tuple(), (5.0, 15.0));
    }

    #[test]
    fn test_position_distance() {
        let p1 = Position::new(0.0, 0.0);
        let p2 = Position::new(3.0, 4.0);
        assert!((p1.distance(&p2) - 5.0).abs() < f64::EPSILON);
    }

    #[test]
    fn test_position_direction_horizontal() {
        let p1 = Position::new(0.0, 0.0);
        let p2 = Position::new(1.0, 0.0);
        assert_eq!(p1.direction_to(&p2), Ok((1.0, 0.0)));
    }

    #[test]
    fn test_position_direction_vertical() {
        let p1 = Position::new(0.0, 0.0);
        let p2 = Position::new(0.0, 1.0);
        assert_eq!(p1.direction_to(&p2), Ok((0.0, 1.0)));
    }

    #[test]
    fn test_position_direction_diagonal() {
        let p1 = Position::new(0.0, 0.0);
        let p2 = Position::new(1.0, 1.0);
        let result = p1.direction_to(&p2).unwrap();
        let expected = 1.0 / 2.0_f64.sqrt();
        assert!((result.0 - expected).abs() < f64::EPSILON);
        assert!((result.1 - expected).abs() < f64::EPSILON);
    }

    #[test]
    fn test_position_direction_coincident() {
        let p1 = Position::new(5.0, 5.0);
        let p2 = Position::new(5.0, 5.0);
        assert_eq!(p1.direction_to(&p2), Err(EdgeError::CoincidentNodes));
    }

    #[test]
    fn test_path_segment_creation() {
        let segment = PathSegment::new((0.0, 0.0), (3.0, 4.0));
        assert_eq!(segment.start, (0.0, 0.0));
        assert_eq!(segment.end, (3.0, 4.0));
        assert!((segment.length - 5.0).abs() < f64::EPSILON);
    }

    #[test]
    fn test_validate_radius_valid() {
        assert!(PathSegment::validate_radius(0.0).is_ok());
        assert!(PathSegment::validate_radius(10.0).is_ok());
        assert!(PathSegment::validate_radius(100.0).is_ok());
    }

    #[test]
    fn test_validate_radius_invalid() {
        assert_eq!(
            PathSegment::validate_radius(-0.1),
            Err(EdgeError::InvalidRadius(-0.1))
        );
        assert_eq!(
            PathSegment::validate_radius(-10.0),
            Err(EdgeError::InvalidRadius(-10.0))
        );
    }

    #[test]
    fn test_calculate_line_path_horizontal() {
        let path = calculate_line_path((0.0, 0.0), 10.0, (100.0, 0.0), 10.0).unwrap();
        assert_eq!(path.start, (10.0, 0.0));
        assert_eq!(path.end, (90.0, 0.0));
        assert_eq!(path.length, 80.0);
    }

    #[test]
    fn test_calculate_line_path_vertical() {
        let path = calculate_line_path((0.0, 0.0), 5.0, (0.0, 50.0), 5.0).unwrap();
        assert_eq!(path.start, (0.0, 5.0));
        assert_eq!(path.end, (0.0, 45.0));
        assert_eq!(path.length, 40.0);
    }

    #[test]
    fn test_calculate_line_path_diagonal() {
        let path = calculate_line_path((0.0, 0.0), 10.0, (30.0, 40.0), 10.0).unwrap();
        let expected = 1.0 / 2.0_f64.sqrt();
        assert!((path.start.0 - expected * 10.0).abs() < f64::EPSILON);
        assert!((path.start.1 - expected * 10.0).abs() < f64::EPSILON);
        assert!((path.end.0 - (30.0 - expected * 10.0)).abs() < f64::EPSILON);
        assert!((path.end.1 - (40.0 - expected * 10.0)).abs() < f64::EPSILON);
        assert!((path.length - 30.0).abs() < f64::EPSILON);
    }

    #[test]
    fn test_calculate_line_path_zero_radius() {
        let path = calculate_line_path((0.0, 0.0), 0.0, (100.0, 0.0), 0.0).unwrap();
        assert_eq!(path.start, (0.0, 0.0));
        assert_eq!(path.end, (100.0, 0.0));
        assert_eq!(path.length, 100.0);
    }

    #[test]
    fn test_calculate_line_path_asymmetric_radii() {
        let path = calculate_line_path((0.0, 0.0), 5.0, (100.0, 0.0), 15.0).unwrap();
        assert_eq!(path.start, (5.0, 0.0));
        assert_eq!(path.end, (85.0, 0.0));
        assert_eq!(path.length, 80.0);
    }

    #[test]
    fn test_calculate_line_path_coincident_nodes() {
        let result = calculate_line_path((10.0, 10.0), 5.0, (10.0, 10.0), 5.0);
        assert_eq!(result, Err(EdgeError::CoincidentNodes));
    }

    #[test]
    fn test_calculate_line_path_invalid_source_radius() {
        let result = calculate_line_path((0.0, 0.0), -5.0, (100.0, 0.0), 10.0);
        assert_eq!(result, Err(EdgeError::InvalidRadius(-5.0)));
    }

    #[test]
    fn test_calculate_line_path_invalid_target_radius() {
        let result = calculate_line_path((0.0, 0.0), 10.0, (100.0, 0.0), -5.0);
        assert_eq!(result, Err(EdgeError::InvalidRadius(-5.0)));
    }

    #[test]
    fn test_calculate_line_path_overlapping_nodes() {
        let result = calculate_line_path((50.0, 50.0), 20.0, (50.0, 50.0), 20.0);
        assert_eq!(result, Err(EdgeError::CoincidentNodes));
    }

    #[test]
    fn test_calculate_line_path_negative_direction() {
        let path = calculate_line_path((100.0, 100.0), 10.0, (0.0, 0.0), 10.0).unwrap();
        assert_eq!(path.start, (90.0, 90.0));
        assert_eq!(path.end, (10.0, 10.0));
        assert!((path.length - 113.137).abs() < 0.01);
    }
}
