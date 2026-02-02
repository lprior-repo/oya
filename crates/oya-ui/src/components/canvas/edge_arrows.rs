//! Arrow head rendering for directed graph edges
//!
//! Implements arrow head calculation and rendering for Canvas2D, following
//! functional programming patterns with zero unwraps and proper error handling.

use crate::error::{LeptosError, Result};

/// Arrow style variants for edge rendering
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ArrowStyle {
    /// Filled solid triangle
    Filled,
    /// Outline stroke only
    Outline,
    /// No arrow (for undirected or weak dependencies)
    None,
}

impl Default for ArrowStyle {
    fn default() -> Self {
        Self::Filled
    }
}

/// Arrow head geometry with three triangle vertices
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ArrowPath {
    /// Arrow tip point (at edge target)
    pub tip: (f64, f64),
    /// First wing point
    pub wing1: (f64, f64),
    /// Second wing point
    pub wing2: (f64, f64),
}

impl ArrowPath {
    /// Creates a new arrow path with validation
    ///
    /// # Errors
    /// Returns error if points form a degenerate triangle (all collinear)
    pub fn new(tip: (f64, f64), wing1: (f64, f64), wing2: (f64, f64)) -> Result<Self> {
        // Validate that points aren't identical (degenerate case)
        if Self::points_nearly_equal(tip, wing1)
            || Self::points_nearly_equal(tip, wing2)
            || Self::points_nearly_equal(wing1, wing2)
        {
            return Err(LeptosError::CanvasError(
                "Arrow path points must be distinct".to_string(),
            ));
        }

        Ok(Self { tip, wing1, wing2 })
    }

    /// Checks if two points are nearly equal (within epsilon)
    fn points_nearly_equal(p1: (f64, f64), p2: (f64, f64)) -> bool {
        const EPSILON: f64 = 1e-6;
        (p1.0 - p2.0).abs() < EPSILON && (p1.1 - p2.1).abs() < EPSILON
    }
}

/// Configuration for arrow rendering
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ArrowConfig {
    /// Arrow length in pixels
    pub length: f64,
    /// Arrow width in pixels
    pub width: f64,
    /// Arrow style (filled, outline, none)
    pub style: ArrowStyle,
}

impl Default for ArrowConfig {
    fn default() -> Self {
        Self {
            length: 12.0,
            width: 8.0,
            style: ArrowStyle::Filled,
        }
    }
}

impl ArrowConfig {
    /// Creates a new arrow configuration with validation
    ///
    /// # Errors
    /// Returns error if length or width are non-positive
    pub fn new(length: f64, width: f64, style: ArrowStyle) -> Result<Self> {
        if length <= 0.0 {
            return Err(LeptosError::CanvasError(format!(
                "Arrow length must be positive, got {}",
                length
            )));
        }
        if width <= 0.0 {
            return Err(LeptosError::CanvasError(format!(
                "Arrow width must be positive, got {}",
                width
            )));
        }

        Ok(Self {
            length,
            width,
            style,
        })
    }

    /// Scales arrow size by zoom factor
    ///
    /// # Errors
    /// Returns error if zoom factor is non-positive
    pub fn with_zoom(self, zoom: f64) -> Result<Self> {
        if zoom <= 0.0 {
            return Err(LeptosError::CanvasError(format!(
                "Zoom factor must be positive, got {}",
                zoom
            )));
        }

        Ok(Self {
            length: self.length * zoom,
            width: self.width * zoom,
            style: self.style,
        })
    }
}

/// Normalizes a 2D vector to unit length
///
/// # Errors
/// Returns error if vector has zero length (cannot normalize)
fn normalize_vector(v: (f64, f64)) -> Result<(f64, f64)> {
    let magnitude = (v.0 * v.0 + v.1 * v.1).sqrt();

    if magnitude < 1e-10 {
        return Err(LeptosError::CanvasError(
            "Cannot normalize zero-length vector".to_string(),
        ));
    }

    Ok((v.0 / magnitude, v.1 / magnitude))
}

/// Calculates perpendicular vector (rotated 90 degrees counterclockwise)
fn perpendicular(v: (f64, f64)) -> (f64, f64) {
    (-v.1, v.0)
}

/// Calculates arrow head geometry at target end of edge
///
/// # Arguments
/// * `edge_end` - Target endpoint position
/// * `direction` - Direction vector pointing toward target (will be normalized)
/// * `config` - Arrow size and style configuration
///
/// # Returns
/// Arrow path with three triangle vertices (tip + two wings)
///
/// # Errors
/// Returns error if:
/// - Direction vector has zero length
/// - Arrow configuration is invalid
///
/// # Algorithm
/// 1. Normalize direction vector
/// 2. Calculate base point (tip - direction * length)
/// 3. Calculate perpendicular offset for wings
/// 4. Generate wing points: base ± perpendicular * (width/2)
pub fn calculate_arrow_head(
    edge_end: (f64, f64),
    direction: (f64, f64),
    config: &ArrowConfig,
) -> Result<ArrowPath> {
    // Validate inputs
    if config.style == ArrowStyle::None {
        return Err(LeptosError::CanvasError(
            "Cannot calculate arrow head for ArrowStyle::None".to_string(),
        ));
    }

    // Normalize direction vector
    let dir_normalized = normalize_vector(direction)?;

    // Calculate base point of arrow (length back from tip)
    let base_x = edge_end.0 - dir_normalized.0 * config.length;
    let base_y = edge_end.1 - dir_normalized.1 * config.length;

    // Calculate perpendicular vector for wings
    let perp = perpendicular(dir_normalized);
    let half_width = config.width / 2.0;

    // Generate wing points
    let wing1 = (base_x + perp.0 * half_width, base_y + perp.1 * half_width);
    let wing2 = (base_x - perp.0 * half_width, base_y - perp.1 * half_width);

    ArrowPath::new(edge_end, wing1, wing2)
}

/// Calculates direction vector from source to target
///
/// # Errors
/// Returns error if source and target are the same point
pub fn edge_direction(source: (f64, f64), target: (f64, f64)) -> Result<(f64, f64)> {
    let dx = target.0 - source.0;
    let dy = target.1 - source.1;

    if dx.abs() < 1e-10 && dy.abs() < 1e-10 {
        return Err(LeptosError::CanvasError(
            "Cannot calculate direction for zero-length edge".to_string(),
        ));
    }

    Ok((dx, dy))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_arrow_config() {
        let config = ArrowConfig::default();
        assert_eq!(config.length, 12.0);
        assert_eq!(config.width, 8.0);
        assert_eq!(config.style, ArrowStyle::Filled);
    }

    #[test]
    fn test_arrow_style_default() {
        let style = ArrowStyle::default();
        assert_eq!(style, ArrowStyle::Filled);
    }

    #[test]
    fn test_arrow_config_validation() {
        // Valid config
        let result = ArrowConfig::new(10.0, 6.0, ArrowStyle::Filled);
        assert!(result.is_ok());

        // Invalid length
        let result = ArrowConfig::new(-5.0, 6.0, ArrowStyle::Filled);
        assert!(result.is_err());

        // Invalid width
        let result = ArrowConfig::new(10.0, 0.0, ArrowStyle::Filled);
        assert!(result.is_err());
    }

    #[test]
    fn test_arrow_config_zoom() -> Result<()> {
        let config = ArrowConfig::default();
        let zoomed = config.with_zoom(2.0)?;

        assert_eq!(zoomed.length, 24.0);
        assert_eq!(zoomed.width, 16.0);
        assert_eq!(zoomed.style, ArrowStyle::Filled);
        Ok(())
    }

    #[test]
    fn test_arrow_config_zoom_validation() {
        let config = ArrowConfig::default();

        // Valid zoom
        assert!(config.with_zoom(1.5).is_ok());

        // Invalid zoom (zero)
        assert!(config.with_zoom(0.0).is_err());

        // Invalid zoom (negative)
        assert!(config.with_zoom(-1.0).is_err());
    }

    #[test]
    fn test_normalize_vector() -> Result<()> {
        // Horizontal vector
        let normalized = normalize_vector((3.0, 0.0))?;
        assert!((normalized.0 - 1.0).abs() < 1e-10);
        assert!((normalized.1).abs() < 1e-10);

        // Vertical vector
        let normalized = normalize_vector((0.0, 4.0))?;
        assert!((normalized.0).abs() < 1e-10);
        assert!((normalized.1 - 1.0).abs() < 1e-10);

        // Diagonal vector
        let normalized = normalize_vector((3.0, 4.0))?;
        assert!((normalized.0 - 0.6).abs() < 1e-10);
        assert!((normalized.1 - 0.8).abs() < 1e-10);

        Ok(())
    }

    #[test]
    fn test_normalize_zero_vector() {
        let result = normalize_vector((0.0, 0.0));
        assert!(result.is_err());
    }

    #[test]
    fn test_perpendicular() {
        // Right → Up
        let perp = perpendicular((1.0, 0.0));
        assert!((perp.0).abs() < 1e-10);
        assert!((perp.1 - 1.0).abs() < 1e-10);

        // Up → Left
        let perp = perpendicular((0.0, 1.0));
        assert!((perp.0 + 1.0).abs() < 1e-10);
        assert!((perp.1).abs() < 1e-10);

        // Diagonal
        let perp = perpendicular((1.0, 1.0));
        assert!((perp.0 + 1.0).abs() < 1e-10);
        assert!((perp.1 - 1.0).abs() < 1e-10);
    }

    #[test]
    fn test_edge_direction() -> Result<()> {
        // Horizontal edge
        let dir = edge_direction((0.0, 0.0), (10.0, 0.0))?;
        assert_eq!(dir, (10.0, 0.0));

        // Vertical edge
        let dir = edge_direction((0.0, 0.0), (0.0, 10.0))?;
        assert_eq!(dir, (0.0, 10.0));

        // Diagonal edge
        let dir = edge_direction((1.0, 2.0), (4.0, 6.0))?;
        assert_eq!(dir, (3.0, 4.0));

        Ok(())
    }

    #[test]
    fn test_edge_direction_zero_length() {
        let result = edge_direction((5.0, 5.0), (5.0, 5.0));
        assert!(result.is_err());
    }

    #[test]
    fn test_calculate_arrow_head_horizontal() -> Result<()> {
        let config = ArrowConfig::new(12.0, 8.0, ArrowStyle::Filled)?;
        let edge_end = (100.0, 50.0);
        let direction = (10.0, 0.0); // Pointing right

        let arrow = calculate_arrow_head(edge_end, direction, &config)?;

        // Tip should be at edge end
        assert_eq!(arrow.tip, edge_end);

        // Base should be 12 pixels left of tip
        let expected_base_x = 100.0 - 12.0;
        let expected_base_y = 50.0;

        // Wings should be ±4 pixels above/below base
        assert!((arrow.wing1.0 - expected_base_x).abs() < 1e-6);
        assert!((arrow.wing1.1 - (expected_base_y + 4.0)).abs() < 1e-6);

        assert!((arrow.wing2.0 - expected_base_x).abs() < 1e-6);
        assert!((arrow.wing2.1 - (expected_base_y - 4.0)).abs() < 1e-6);

        Ok(())
    }

    #[test]
    fn test_calculate_arrow_head_vertical() -> Result<()> {
        let config = ArrowConfig::new(12.0, 8.0, ArrowStyle::Filled)?;
        let edge_end = (50.0, 100.0);
        let direction = (0.0, 10.0); // Pointing down

        let arrow = calculate_arrow_head(edge_end, direction, &config)?;

        // Tip should be at edge end
        assert_eq!(arrow.tip, edge_end);

        // Base should be 12 pixels above tip
        let expected_base_x = 50.0;
        let expected_base_y = 100.0 - 12.0;

        // Wings should be ±4 pixels left/right of base
        assert!((arrow.wing1.0 - (expected_base_x - 4.0)).abs() < 1e-6);
        assert!((arrow.wing1.1 - expected_base_y).abs() < 1e-6);

        assert!((arrow.wing2.0 - (expected_base_x + 4.0)).abs() < 1e-6);
        assert!((arrow.wing2.1 - expected_base_y).abs() < 1e-6);

        Ok(())
    }

    #[test]
    fn test_calculate_arrow_head_diagonal() -> Result<()> {
        let config = ArrowConfig::new(10.0, 6.0, ArrowStyle::Filled)?;
        let edge_end = (100.0, 100.0);
        let direction = (3.0, 4.0); // 3-4-5 triangle

        let arrow = calculate_arrow_head(edge_end, direction, &config)?;

        // Tip should be at edge end
        assert_eq!(arrow.tip, edge_end);

        // Direction normalized: (0.6, 0.8)
        // Base: (100 - 0.6*10, 100 - 0.8*10) = (94, 92)
        // Perpendicular: (-0.8, 0.6)
        // Half width: 3.0
        // Wing1: (94 - 0.8*3, 92 + 0.6*3) = (91.6, 93.8)
        // Wing2: (94 + 0.8*3, 92 - 0.6*3) = (96.4, 90.2)

        assert!((arrow.wing1.0 - 91.6).abs() < 1e-6);
        assert!((arrow.wing1.1 - 93.8).abs() < 1e-6);

        assert!((arrow.wing2.0 - 96.4).abs() < 1e-6);
        assert!((arrow.wing2.1 - 90.2).abs() < 1e-6);

        Ok(())
    }

    #[test]
    fn test_calculate_arrow_head_none_style() {
        let config = ArrowConfig {
            length: 12.0,
            width: 8.0,
            style: ArrowStyle::None,
        };
        let edge_end = (100.0, 50.0);
        let direction = (10.0, 0.0);

        let result = calculate_arrow_head(edge_end, direction, &config);
        assert!(result.is_err());
    }

    #[test]
    fn test_arrow_path_validation() {
        // Valid arrow path
        let result = ArrowPath::new((100.0, 100.0), (95.0, 104.0), (95.0, 96.0));
        assert!(result.is_ok());

        // Degenerate: identical tip and wing1
        let result = ArrowPath::new((100.0, 100.0), (100.0, 100.0), (95.0, 96.0));
        assert!(result.is_err());

        // Degenerate: identical wing1 and wing2
        let result = ArrowPath::new((100.0, 100.0), (95.0, 104.0), (95.0, 104.0));
        assert!(result.is_err());
    }

    #[test]
    fn test_arrow_styles() {
        // Test all style variants exist
        let _filled = ArrowStyle::Filled;
        let _outline = ArrowStyle::Outline;
        let _none = ArrowStyle::None;

        // Test equality
        assert_eq!(ArrowStyle::Filled, ArrowStyle::Filled);
        assert_ne!(ArrowStyle::Filled, ArrowStyle::Outline);
        assert_ne!(ArrowStyle::Outline, ArrowStyle::None);
    }

    #[test]
    fn test_arrow_with_zoom() -> Result<()> {
        let config = ArrowConfig::default();
        let edge_end = (100.0, 50.0);
        let direction = (10.0, 0.0);

        // Normal size arrow
        let arrow_normal = calculate_arrow_head(edge_end, direction, &config)?;

        // 2x zoom arrow
        let config_zoomed = config.with_zoom(2.0)?;
        let arrow_zoomed = calculate_arrow_head(edge_end, direction, &config_zoomed)?;

        // Tip stays same
        assert_eq!(arrow_normal.tip, arrow_zoomed.tip);

        // Wings should be further from tip with zoom
        let wing1_dist_normal = (arrow_normal.wing1.0 - edge_end.0).abs();
        let wing1_dist_zoomed = (arrow_zoomed.wing1.0 - edge_end.0).abs();

        assert!(wing1_dist_zoomed > wing1_dist_normal);
        assert!((wing1_dist_zoomed / wing1_dist_normal - 2.0).abs() < 0.1);

        Ok(())
    }
}
