//! Mouse wheel zoom for DAG viewport with origin preservation
//!
//! This module implements zoom functionality for the DAG visualization using
//! mouse wheel events. All operations follow Railway-Oriented Programming with
//! Result types and zero panics/unwraps.
//!
//! # Features
//! - Mouse wheel zoom in/out with configurable sensitivity
//! - Origin-based scaling (zoom centers on cursor position)
//! - Smooth zoom factor calculation from wheel delta
//! - Integration with ZoomLevel bounds validation
//!
//! # Example
//! ```no_run
//! use oya_ui::components::controls::zoom::{ZoomConfig, calculate_zoom_factor, apply_zoom};
//! use oya_ui::components::controls::bounds::ZoomLevel;
//!
//! # fn example() -> Result<(), String> {
//! let config = ZoomConfig::default();
//! let current_zoom = ZoomLevel::default();
//! let wheel_delta = -100.0; // scroll up = zoom in
//!
//! let zoom_factor = calculate_zoom_factor(wheel_delta, &config)?;
//! let new_zoom = apply_zoom(current_zoom, zoom_factor)?;
//! # Ok(())
//! # }
//! ```

use super::bounds::ZoomLevel;

/// Configuration for zoom behavior
///
/// Controls how zoom responds to wheel events and constrains zoom speed.
///
/// # Examples
///
/// ```
/// use oya_ui::components::controls::zoom::ZoomConfig;
///
/// let config = ZoomConfig::default();
/// assert_eq!(config.sensitivity(), 0.001);
/// assert_eq!(config.min_zoom_factor(), 0.5);
/// assert_eq!(config.max_zoom_factor(), 2.0);
/// ```
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ZoomConfig {
    /// How much wheel delta affects zoom (0.001 = 0.1% per unit)
    sensitivity: f32,
    /// Minimum zoom factor per wheel event (prevents too-fast zoom out)
    min_zoom_factor: f32,
    /// Maximum zoom factor per wheel event (prevents too-fast zoom in)
    max_zoom_factor: f32,
}

impl ZoomConfig {
    /// Creates a new zoom configuration with custom parameters.
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - sensitivity is not finite or is negative
    /// - min_zoom_factor is not in range (0.0, 1.0]
    /// - max_zoom_factor is not in range [1.0, infinity)
    /// - min_zoom_factor >= max_zoom_factor
    ///
    /// # Examples
    ///
    /// ```
    /// use oya_ui::components::controls::zoom::ZoomConfig;
    ///
    /// // Slower zoom
    /// let config = ZoomConfig::new(0.0005, 0.8, 1.2)?;
    /// assert_eq!(config.sensitivity(), 0.0005);
    ///
    /// // Faster zoom
    /// let config = ZoomConfig::new(0.002, 0.5, 2.0)?;
    /// assert_eq!(config.sensitivity(), 0.002);
    ///
    /// // Invalid configs return errors
    /// assert!(ZoomConfig::new(-0.001, 0.5, 2.0).is_err()); // negative sensitivity
    /// assert!(ZoomConfig::new(0.001, 1.5, 2.0).is_err());  // min > 1.0
    /// assert!(ZoomConfig::new(0.001, 0.8, 0.5).is_err());  // max < 1.0
    /// # Ok::<(), String>(())
    /// ```
    pub fn new(
        sensitivity: f32,
        min_zoom_factor: f32,
        max_zoom_factor: f32,
    ) -> Result<Self, String> {
        // Validate sensitivity
        if !sensitivity.is_finite() {
            return Err("Sensitivity must be finite".to_string());
        }
        if sensitivity <= 0.0 {
            return Err("Sensitivity must be positive".to_string());
        }

        // Validate min_zoom_factor
        if !min_zoom_factor.is_finite() {
            return Err("Min zoom factor must be finite".to_string());
        }
        if min_zoom_factor <= 0.0 || min_zoom_factor > 1.0 {
            return Err("Min zoom factor must be in range (0.0, 1.0]".to_string());
        }

        // Validate max_zoom_factor
        if !max_zoom_factor.is_finite() {
            return Err("Max zoom factor must be finite".to_string());
        }
        if max_zoom_factor < 1.0 {
            return Err("Max zoom factor must be >= 1.0".to_string());
        }

        // Validate relationship
        if min_zoom_factor >= max_zoom_factor {
            return Err("Min zoom factor must be less than max zoom factor".to_string());
        }

        Ok(Self {
            sensitivity,
            min_zoom_factor,
            max_zoom_factor,
        })
    }

    /// Returns the zoom sensitivity value.
    ///
    /// # Examples
    ///
    /// ```
    /// use oya_ui::components::controls::zoom::ZoomConfig;
    ///
    /// let config = ZoomConfig::default();
    /// assert_eq!(config.sensitivity(), 0.001);
    /// ```
    pub fn sensitivity(&self) -> f32 {
        self.sensitivity
    }

    /// Returns the minimum zoom factor per event.
    ///
    /// # Examples
    ///
    /// ```
    /// use oya_ui::components::controls::zoom::ZoomConfig;
    ///
    /// let config = ZoomConfig::default();
    /// assert_eq!(config.min_zoom_factor(), 0.5);
    /// ```
    pub fn min_zoom_factor(&self) -> f32 {
        self.min_zoom_factor
    }

    /// Returns the maximum zoom factor per event.
    ///
    /// # Examples
    ///
    /// ```
    /// use oya_ui::components::controls::zoom::ZoomConfig;
    ///
    /// let config = ZoomConfig::default();
    /// assert_eq!(config.max_zoom_factor(), 2.0);
    /// ```
    pub fn max_zoom_factor(&self) -> f32 {
        self.max_zoom_factor
    }
}

impl Default for ZoomConfig {
    /// Returns default zoom configuration.
    ///
    /// - Sensitivity: 0.001 (0.1% per wheel unit)
    /// - Min factor: 0.5 (max 50% zoom out per event)
    /// - Max factor: 2.0 (max 2x zoom in per event)
    ///
    /// # Examples
    ///
    /// ```
    /// use oya_ui::components::controls::zoom::ZoomConfig;
    ///
    /// let config = ZoomConfig::default();
    /// assert_eq!(config.sensitivity(), 0.001);
    /// assert_eq!(config.min_zoom_factor(), 0.5);
    /// assert_eq!(config.max_zoom_factor(), 2.0);
    /// ```
    fn default() -> Self {
        Self {
            sensitivity: 0.001,
            min_zoom_factor: 0.5,
            max_zoom_factor: 2.0,
        }
    }
}

/// Calculate zoom factor from wheel delta.
///
/// Converts wheel event delta (positive = zoom out, negative = zoom in)
/// into a zoom factor (>1.0 = zoom in, <1.0 = zoom out, 1.0 = no change).
///
/// The zoom factor is clamped to the config's min/max range to prevent
/// jarring zoom jumps from large wheel deltas.
///
/// # Errors
///
/// Returns an error if delta is not finite (NaN or infinite).
///
/// # Examples
///
/// ```
/// use oya_ui::components::controls::zoom::{ZoomConfig, calculate_zoom_factor};
///
/// let config = ZoomConfig::default();
///
/// // Scroll up (negative delta) = zoom in (factor > 1.0)
/// let factor = calculate_zoom_factor(-100.0, &config)?;
/// assert!(factor > 1.0);
///
/// // Scroll down (positive delta) = zoom out (factor < 1.0)
/// let factor = calculate_zoom_factor(100.0, &config)?;
/// assert!(factor < 1.0);
///
/// // Zero delta = no zoom (factor = 1.0)
/// let factor = calculate_zoom_factor(0.0, &config)?;
/// assert_eq!(factor, 1.0);
///
/// // Invalid delta returns error
/// assert!(calculate_zoom_factor(f64::NAN, &config).is_err());
/// # Ok::<(), String>(())
/// ```
pub fn calculate_zoom_factor(delta: f64, config: &ZoomConfig) -> Result<f32, String> {
    if !delta.is_finite() {
        return Err("Wheel delta must be finite".to_string());
    }

    // Convert delta to zoom factor
    // Negative delta (scroll up) = zoom in (factor > 1.0)
    // Positive delta (scroll down) = zoom out (factor < 1.0)
    let factor = 1.0 - (delta as f32 * config.sensitivity);

    // Clamp to prevent extreme zoom jumps
    let clamped = factor.clamp(config.min_zoom_factor, config.max_zoom_factor);

    Ok(clamped)
}

/// Apply zoom factor to current zoom level.
///
/// Multiplies the current zoom by the factor and validates the result
/// through ZoomLevel bounds checking.
///
/// # Errors
///
/// Returns an error if:
/// - zoom_factor is not finite
/// - resulting zoom level is invalid (caught by ZoomLevel::new)
///
/// # Examples
///
/// ```
/// use oya_ui::components::controls::zoom::apply_zoom;
/// use oya_ui::components::controls::bounds::ZoomLevel;
///
/// let current = ZoomLevel::new(1.0)?;
///
/// // Zoom in by 10%
/// let zoomed = apply_zoom(current, 1.1)?;
/// assert_eq!(zoomed.value(), 1.1);
///
/// // Zoom out by 20%
/// let zoomed = apply_zoom(current, 0.8)?;
/// assert_eq!(zoomed.value(), 0.8);
///
/// // Zoom beyond bounds gets clamped by ZoomLevel
/// let zoomed = apply_zoom(current, 10.0)?;
/// assert_eq!(zoomed.value(), 5.0); // clamped to max
///
/// // Invalid factor returns error
/// assert!(apply_zoom(current, f32::NAN).is_err());
/// # Ok::<(), String>(())
/// ```
pub fn apply_zoom(current: ZoomLevel, zoom_factor: f32) -> Result<ZoomLevel, String> {
    if !zoom_factor.is_finite() {
        return Err("Zoom factor must be finite".to_string());
    }

    let new_value = current.value() * zoom_factor;
    ZoomLevel::new(new_value)
}

/// Calculate zoom transform origin for cursor-centered zooming.
///
/// Computes the canvas-space coordinates that should remain fixed under
/// the zoom transformation. This ensures the point under the cursor stays
/// in the same position after zooming.
///
/// # Arguments
///
/// - `cursor_x`, `cursor_y`: Canvas-relative cursor position
/// - `viewport_x`, `viewport_y`: Current viewport offset
/// - `current_zoom`: Current zoom level
///
/// # Errors
///
/// Returns an error if any coordinate is not finite.
///
/// # Examples
///
/// ```
/// use oya_ui::components::controls::zoom::calculate_zoom_origin;
/// use oya_ui::components::controls::bounds::ZoomLevel;
///
/// let cursor_x = 400.0;
/// let cursor_y = 300.0;
/// let viewport_x = 100.0;
/// let viewport_y = 50.0;
/// let zoom = ZoomLevel::new(1.5)?;
///
/// let (origin_x, origin_y) = calculate_zoom_origin(
///     cursor_x,
///     cursor_y,
///     viewport_x,
///     viewport_y,
///     zoom,
/// )?;
///
/// // Origin is in world coordinates
/// assert!(origin_x.is_finite());
/// assert!(origin_y.is_finite());
///
/// // Invalid coords return error
/// assert!(calculate_zoom_origin(f64::NAN, 0.0, 0.0, 0.0, zoom).is_err());
/// # Ok::<(), String>(())
/// ```
pub fn calculate_zoom_origin(
    cursor_x: f64,
    cursor_y: f64,
    viewport_x: f64,
    viewport_y: f64,
    current_zoom: ZoomLevel,
) -> Result<(f64, f64), String> {
    // Validate inputs
    if !cursor_x.is_finite() || !cursor_y.is_finite() {
        return Err("Cursor coordinates must be finite".to_string());
    }
    if !viewport_x.is_finite() || !viewport_y.is_finite() {
        return Err("Viewport coordinates must be finite".to_string());
    }

    // Convert cursor position to world coordinates
    // world = (canvas - viewport) / zoom
    let zoom_f64 = current_zoom.value() as f64;
    let world_x = (cursor_x - viewport_x) / zoom_f64;
    let world_y = (cursor_y - viewport_y) / zoom_f64;

    Ok((world_x, world_y))
}

/// Update viewport offset to preserve zoom origin.
///
/// After applying a zoom, the viewport must be adjusted so that the
/// zoom origin (world coordinates) appears at the same canvas position.
///
/// # Arguments
///
/// - `cursor_x`, `cursor_y`: Canvas-relative cursor position
/// - `origin_x`, `origin_y`: World-space zoom origin
/// - `new_zoom`: New zoom level after zooming
///
/// # Errors
///
/// Returns an error if any coordinate is not finite.
///
/// # Examples
///
/// ```
/// use oya_ui::components::controls::zoom::calculate_viewport_offset;
/// use oya_ui::components::controls::bounds::ZoomLevel;
///
/// let cursor_x = 400.0;
/// let cursor_y = 300.0;
/// let origin_x = 200.0;
/// let origin_y = 150.0;
/// let new_zoom = ZoomLevel::new(2.0)?;
///
/// let (viewport_x, viewport_y) = calculate_viewport_offset(
///     cursor_x,
///     cursor_y,
///     origin_x,
///     origin_y,
///     new_zoom,
/// )?;
///
/// // Viewport offset positions the origin correctly
/// assert!(viewport_x.is_finite());
/// assert!(viewport_y.is_finite());
///
/// // Invalid coords return error
/// assert!(calculate_viewport_offset(f64::NAN, 0.0, 0.0, 0.0, new_zoom).is_err());
/// # Ok::<(), String>(())
/// ```
pub fn calculate_viewport_offset(
    cursor_x: f64,
    cursor_y: f64,
    origin_x: f64,
    origin_y: f64,
    new_zoom: ZoomLevel,
) -> Result<(f64, f64), String> {
    // Validate inputs
    if !cursor_x.is_finite() || !cursor_y.is_finite() {
        return Err("Cursor coordinates must be finite".to_string());
    }
    if !origin_x.is_finite() || !origin_y.is_finite() {
        return Err("Origin coordinates must be finite".to_string());
    }

    // Calculate new viewport offset
    // viewport = canvas - (world * zoom)
    let zoom_f64 = new_zoom.value() as f64;
    let viewport_x = cursor_x - (origin_x * zoom_f64);
    let viewport_y = cursor_y - (origin_y * zoom_f64);

    Ok((viewport_x, viewport_y))
}

/// Complete zoom operation with origin preservation.
///
/// This is the high-level function that performs a full zoom operation:
/// 1. Calculate zoom factor from wheel delta
/// 2. Calculate zoom origin (world coords under cursor)
/// 3. Apply zoom to current level
/// 4. Calculate new viewport offset to preserve origin
///
/// # Errors
///
/// Returns an error if any calculation step fails.
///
/// # Examples
///
/// ```
/// use oya_ui::components::controls::zoom::{ZoomConfig, zoom_at_point};
/// use oya_ui::components::controls::bounds::ZoomLevel;
///
/// let config = ZoomConfig::default();
/// let current_zoom = ZoomLevel::new(1.0)?;
/// let viewport_x = 0.0;
/// let viewport_y = 0.0;
/// let cursor_x = 400.0;
/// let cursor_y = 300.0;
/// let wheel_delta = -100.0; // zoom in
///
/// let result = zoom_at_point(
///     current_zoom,
///     viewport_x,
///     viewport_y,
///     cursor_x,
///     cursor_y,
///     wheel_delta,
///     &config,
/// )?;
///
/// assert!(result.new_zoom.value() > 1.0); // zoomed in
/// assert_ne!(result.new_viewport_x, viewport_x); // viewport adjusted
/// assert_ne!(result.new_viewport_y, viewport_y);
/// # Ok::<(), String>(())
/// ```
pub fn zoom_at_point(
    current_zoom: ZoomLevel,
    viewport_x: f64,
    viewport_y: f64,
    cursor_x: f64,
    cursor_y: f64,
    wheel_delta: f64,
    config: &ZoomConfig,
) -> Result<ZoomResult, String> {
    // Calculate zoom factor from wheel delta
    let zoom_factor = calculate_zoom_factor(wheel_delta, config)?;

    // Calculate world coordinates of zoom origin
    let (origin_x, origin_y) =
        calculate_zoom_origin(cursor_x, cursor_y, viewport_x, viewport_y, current_zoom)?;

    // Apply zoom
    let new_zoom = apply_zoom(current_zoom, zoom_factor)?;

    // Calculate new viewport offset to preserve origin
    let (new_viewport_x, new_viewport_y) =
        calculate_viewport_offset(cursor_x, cursor_y, origin_x, origin_y, new_zoom)?;

    Ok(ZoomResult {
        new_zoom,
        new_viewport_x,
        new_viewport_y,
        origin_x,
        origin_y,
    })
}

/// Result of a zoom operation.
///
/// Contains the new zoom level and viewport offset needed to render
/// the zoomed view with preserved origin.
///
/// # Examples
///
/// ```
/// use oya_ui::components::controls::zoom::{ZoomResult, zoom_at_point, ZoomConfig};
/// use oya_ui::components::controls::bounds::ZoomLevel;
///
/// # fn example() -> Result<(), String> {
/// let result = zoom_at_point(
///     ZoomLevel::default(),
///     0.0, 0.0,
///     400.0, 300.0,
///     -100.0,
///     &ZoomConfig::default(),
/// )?;
///
/// // Use result to update viewport state
/// let zoom = result.new_zoom;
/// let viewport_x = result.new_viewport_x;
/// let viewport_y = result.new_viewport_y;
/// # Ok(())
/// # }
/// ```
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ZoomResult {
    /// New zoom level after applying zoom factor
    pub new_zoom: ZoomLevel,
    /// New viewport X offset for origin preservation
    pub new_viewport_x: f64,
    /// New viewport Y offset for origin preservation
    pub new_viewport_y: f64,
    /// World-space X coordinate of zoom origin
    pub origin_x: f64,
    /// World-space Y coordinate of zoom origin
    pub origin_y: f64,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_zoom_config_default() {
        let config = ZoomConfig::default();
        assert_eq!(config.sensitivity(), 0.001);
        assert_eq!(config.min_zoom_factor(), 0.5);
        assert_eq!(config.max_zoom_factor(), 2.0);
    }

    #[test]
    fn test_zoom_config_custom() -> Result<(), String> {
        let config = ZoomConfig::new(0.002, 0.7, 1.5)?;
        assert_eq!(config.sensitivity(), 0.002);
        assert_eq!(config.min_zoom_factor(), 0.7);
        assert_eq!(config.max_zoom_factor(), 1.5);
        Ok(())
    }

    #[test]
    fn test_zoom_config_validation() {
        // Negative sensitivity
        assert!(ZoomConfig::new(-0.001, 0.5, 2.0).is_err());

        // Zero sensitivity
        assert!(ZoomConfig::new(0.0, 0.5, 2.0).is_err());

        // Invalid min factor (> 1.0)
        assert!(ZoomConfig::new(0.001, 1.5, 2.0).is_err());

        // Invalid max factor (< 1.0)
        assert!(ZoomConfig::new(0.001, 0.5, 0.8).is_err());

        // Min >= max
        assert!(ZoomConfig::new(0.001, 0.9, 0.8).is_err());

        // NaN values
        assert!(ZoomConfig::new(f32::NAN, 0.5, 2.0).is_err());
        assert!(ZoomConfig::new(0.001, f32::NAN, 2.0).is_err());
        assert!(ZoomConfig::new(0.001, 0.5, f32::NAN).is_err());
    }

    #[test]
    fn test_calculate_zoom_factor_zoom_in() -> Result<(), String> {
        let config = ZoomConfig::default();

        // Negative delta = zoom in
        let factor = calculate_zoom_factor(-100.0, &config)?;
        assert!(factor > 1.0);
        assert!(factor <= config.max_zoom_factor());

        Ok(())
    }

    #[test]
    fn test_calculate_zoom_factor_zoom_out() -> Result<(), String> {
        let config = ZoomConfig::default();

        // Positive delta = zoom out
        let factor = calculate_zoom_factor(100.0, &config)?;
        assert!(factor < 1.0);
        assert!(factor >= config.min_zoom_factor());

        Ok(())
    }

    #[test]
    fn test_calculate_zoom_factor_no_zoom() -> Result<(), String> {
        let config = ZoomConfig::default();

        // Zero delta = no zoom
        let factor = calculate_zoom_factor(0.0, &config)?;
        assert_eq!(factor, 1.0);

        Ok(())
    }

    #[test]
    fn test_calculate_zoom_factor_clamping() -> Result<(), String> {
        let config = ZoomConfig::default();

        // Very large negative delta gets clamped to max
        let factor = calculate_zoom_factor(-10000.0, &config)?;
        assert_eq!(factor, config.max_zoom_factor());

        // Very large positive delta gets clamped to min
        let factor = calculate_zoom_factor(10000.0, &config)?;
        assert_eq!(factor, config.min_zoom_factor());

        Ok(())
    }

    #[test]
    fn test_calculate_zoom_factor_invalid() {
        let config = ZoomConfig::default();

        assert!(calculate_zoom_factor(f64::NAN, &config).is_err());
        assert!(calculate_zoom_factor(f64::INFINITY, &config).is_err());
    }

    #[test]
    fn test_apply_zoom() -> Result<(), String> {
        let current = ZoomLevel::new(1.0)?;

        // Zoom in
        let zoomed = apply_zoom(current, 1.5)?;
        assert_eq!(zoomed.value(), 1.5);

        // Zoom out
        let zoomed = apply_zoom(current, 0.5)?;
        assert_eq!(zoomed.value(), 0.5);

        // No zoom
        let zoomed = apply_zoom(current, 1.0)?;
        assert_eq!(zoomed.value(), 1.0);

        Ok(())
    }

    #[test]
    fn test_apply_zoom_clamping() -> Result<(), String> {
        let current = ZoomLevel::new(1.0)?;

        // Zoom beyond max gets clamped
        let zoomed = apply_zoom(current, 10.0)?;
        assert_eq!(zoomed.value(), 5.0); // ZoomLevel max

        // Zoom beyond min gets clamped
        let zoomed = apply_zoom(current, 0.01)?;
        assert_eq!(zoomed.value(), 0.1); // ZoomLevel min

        Ok(())
    }

    #[test]
    fn test_apply_zoom_invalid() -> Result<(), String> {
        let current = ZoomLevel::new(1.0)?;

        assert!(apply_zoom(current, f32::NAN).is_err());
        assert!(apply_zoom(current, f32::INFINITY).is_err());

        Ok(())
    }

    #[test]
    fn test_calculate_zoom_origin() -> Result<(), String> {
        let zoom = ZoomLevel::new(2.0)?;
        let (origin_x, origin_y) = calculate_zoom_origin(400.0, 300.0, 100.0, 50.0, zoom)?;

        // Should be world coordinates
        assert!(origin_x.is_finite());
        assert!(origin_y.is_finite());

        // At zoom 2.0, (400, 300) canvas with (100, 50) viewport
        // should map to ((400-100)/2, (300-50)/2) = (150, 125) world
        assert!((origin_x - 150.0).abs() < 0.001);
        assert!((origin_y - 125.0).abs() < 0.001);

        Ok(())
    }

    #[test]
    fn test_calculate_zoom_origin_invalid() -> Result<(), String> {
        let zoom = ZoomLevel::new(1.0)?;

        assert!(calculate_zoom_origin(f64::NAN, 0.0, 0.0, 0.0, zoom).is_err());
        assert!(calculate_zoom_origin(0.0, f64::NAN, 0.0, 0.0, zoom).is_err());
        assert!(calculate_zoom_origin(0.0, 0.0, f64::NAN, 0.0, zoom).is_err());
        assert!(calculate_zoom_origin(0.0, 0.0, 0.0, f64::NAN, zoom).is_err());

        Ok(())
    }

    #[test]
    fn test_calculate_viewport_offset() -> Result<(), String> {
        let zoom = ZoomLevel::new(2.0)?;
        let (viewport_x, viewport_y) = calculate_viewport_offset(400.0, 300.0, 150.0, 125.0, zoom)?;

        // Should position origin at cursor
        assert!(viewport_x.is_finite());
        assert!(viewport_y.is_finite());

        // viewport = cursor - (origin * zoom)
        // viewport_x = 400 - (150 * 2) = 100
        // viewport_y = 300 - (125 * 2) = 50
        assert!((viewport_x - 100.0).abs() < 0.001);
        assert!((viewport_y - 50.0).abs() < 0.001);

        Ok(())
    }

    #[test]
    fn test_calculate_viewport_offset_invalid() -> Result<(), String> {
        let zoom = ZoomLevel::new(1.0)?;

        assert!(calculate_viewport_offset(f64::NAN, 0.0, 0.0, 0.0, zoom).is_err());
        assert!(calculate_viewport_offset(0.0, f64::NAN, 0.0, 0.0, zoom).is_err());
        assert!(calculate_viewport_offset(0.0, 0.0, f64::NAN, 0.0, zoom).is_err());
        assert!(calculate_viewport_offset(0.0, 0.0, 0.0, f64::NAN, zoom).is_err());

        Ok(())
    }

    #[test]
    fn test_zoom_at_point_zoom_in() -> Result<(), String> {
        let config = ZoomConfig::default();
        let current_zoom = ZoomLevel::new(1.0)?;

        let result = zoom_at_point(
            current_zoom,
            0.0,
            0.0,
            400.0,
            300.0,
            -100.0, // zoom in
            &config,
        )?;

        // Should zoom in
        assert!(result.new_zoom.value() > current_zoom.value());

        // Viewport should adjust
        assert!(result.new_viewport_x.is_finite());
        assert!(result.new_viewport_y.is_finite());

        // Origin should be recorded
        assert!(result.origin_x.is_finite());
        assert!(result.origin_y.is_finite());

        Ok(())
    }

    #[test]
    fn test_zoom_at_point_zoom_out() -> Result<(), String> {
        let config = ZoomConfig::default();
        let current_zoom = ZoomLevel::new(2.0)?;

        let result = zoom_at_point(
            current_zoom,
            0.0,
            0.0,
            400.0,
            300.0,
            100.0, // zoom out
            &config,
        )?;

        // Should zoom out
        assert!(result.new_zoom.value() < current_zoom.value());

        Ok(())
    }

    #[test]
    fn test_zoom_at_point_origin_preservation() -> Result<(), String> {
        let config = ZoomConfig::default();
        let current_zoom = ZoomLevel::new(1.0)?;

        // Zoom in at a specific point
        let result = zoom_at_point(current_zoom, 0.0, 0.0, 400.0, 300.0, -100.0, &config)?;

        // After zoom, the origin should map back to cursor position
        let zoom_f64 = result.new_zoom.value() as f64;
        let canvas_x = result.new_viewport_x + (result.origin_x * zoom_f64);
        let canvas_y = result.new_viewport_y + (result.origin_y * zoom_f64);

        // Should be close to original cursor position
        assert!((canvas_x - 400.0).abs() < 0.1);
        assert!((canvas_y - 300.0).abs() < 0.1);

        Ok(())
    }
}
