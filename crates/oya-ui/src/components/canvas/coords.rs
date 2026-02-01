//! Canvas coordinate transformation system
//!
//! This module provides pure functional coordinate transformations between
//! screen space (mouse events, pixels) and world space (DAG nodes, logical coordinates).
//!
//! # Architecture
//!
//! - **Viewport**: Immutable state container for pan/zoom transformations
//! - **screen_to_world**: Converts screen coordinates to world coordinates
//! - **world_to_screen**: Converts world coordinates to screen coordinates
//!
//! All operations use Railway-Oriented Programming with Result types.
//! No unwraps, no panics, comprehensive validation.

use crate::components::controls::bounds::ZoomLevel;
use crate::models::node::Position;

/// Canvas viewport state for coordinate transformations
///
/// This is an immutable value type that encapsulates all state needed
/// for coordinate transformations. All transformations are pure functions.
///
/// # Examples
///
/// ```
/// use oya_ui::components::canvas::coords::Viewport;
///
/// let viewport = Viewport::new(1200.0, 800.0)?;
/// assert_eq!(viewport.canvas_width, 1200.0);
/// assert_eq!(viewport.canvas_height, 800.0);
/// # Ok::<(), String>(())
/// ```
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Viewport {
    pub pan_x: f32,
    pub pan_y: f32,
    pub zoom: ZoomLevel,
    pub canvas_width: f32,
    pub canvas_height: f32,
}

impl Viewport {
    /// Creates a new viewport with default pan (0, 0) and zoom (1.0)
    ///
    /// # Errors
    ///
    /// Returns an error if width or height are not positive finite values.
    ///
    /// # Examples
    ///
    /// ```
    /// # use oya_ui::components::canvas::coords::Viewport;
    /// let viewport = Viewport::new(1920.0, 1080.0)?;
    /// assert_eq!(viewport.pan_x, 0.0);
    /// assert_eq!(viewport.pan_y, 0.0);
    /// assert_eq!(viewport.zoom.value(), 1.0);
    /// # Ok::<(), String>(())
    /// ```
    pub fn new(width: f32, height: f32) -> Result<Self, String> {
        if !width.is_finite() || width <= 0.0 {
            return Err(format!(
                "Canvas width must be positive and finite, got: {}",
                width
            ));
        }
        if !height.is_finite() || height <= 0.0 {
            return Err(format!(
                "Canvas height must be positive and finite, got: {}",
                height
            ));
        }

        Ok(Self {
            pan_x: 0.0,
            pan_y: 0.0,
            zoom: ZoomLevel::default(),
            canvas_width: width,
            canvas_height: height,
        })
    }

    /// Creates a viewport with specified pan and zoom
    ///
    /// # Errors
    ///
    /// Returns an error if any parameter is invalid.
    ///
    /// # Examples
    ///
    /// ```
    /// # use oya_ui::components::canvas::coords::Viewport;
    /// # use oya_ui::components::controls::bounds::ZoomLevel;
    /// let zoom = ZoomLevel::new(2.0)?;
    /// let viewport = Viewport::with_transform(1200.0, 800.0, 100.0, 50.0, zoom)?;
    /// assert_eq!(viewport.pan_x, 100.0);
    /// assert_eq!(viewport.pan_y, 50.0);
    /// # Ok::<(), String>(())
    /// ```
    pub fn with_transform(
        width: f32,
        height: f32,
        pan_x: f32,
        pan_y: f32,
        zoom: ZoomLevel,
    ) -> Result<Self, String> {
        if !width.is_finite() || width <= 0.0 {
            return Err(format!(
                "Canvas width must be positive and finite, got: {}",
                width
            ));
        }
        if !height.is_finite() || height <= 0.0 {
            return Err(format!(
                "Canvas height must be positive and finite, got: {}",
                height
            ));
        }
        if !pan_x.is_finite() {
            return Err(format!("Pan X must be finite, got: {}", pan_x));
        }
        if !pan_y.is_finite() {
            return Err(format!("Pan Y must be finite, got: {}", pan_y));
        }

        Ok(Self {
            pan_x,
            pan_y,
            zoom,
            canvas_width: width,
            canvas_height: height,
        })
    }
}

/// Convert screen coordinates to world coordinates
///
/// Screen coordinates have origin (0, 0) at top-left corner.
/// World coordinates have origin at the center of the canvas.
///
/// The transformation pipeline:
/// 1. Translate screen coords to canvas-centered coords
/// 2. Apply inverse zoom
/// 3. Apply pan offset
///
/// # Errors
///
/// Returns an error if the resulting world coordinates are not finite.
///
/// # Examples
///
/// ```
/// # use oya_ui::components::canvas::coords::{Viewport, screen_to_world};
/// let viewport = Viewport::new(1200.0, 800.0)?;
///
/// // Center of screen maps to world origin
/// let world = screen_to_world(600.0, 400.0, &viewport)?;
/// assert!((world.x - 0.0).abs() < 0.01);
/// assert!((world.y - 0.0).abs() < 0.01);
/// # Ok::<(), String>(())
/// ```
pub fn screen_to_world(
    screen_x: f32,
    screen_y: f32,
    viewport: &Viewport,
) -> Result<Position, String> {
    // Step 1: Center the coordinates (origin at canvas center)
    let centered_x = screen_x - viewport.canvas_width / 2.0;
    let centered_y = screen_y - viewport.canvas_height / 2.0;

    // Step 2: Apply inverse zoom
    let zoom = viewport.zoom.value();
    let world_x = centered_x / zoom - viewport.pan_x;
    let world_y = centered_y / zoom - viewport.pan_y;

    // Step 3: Validate and return
    Position::new(world_x, world_y)
}

/// Convert world coordinates to screen coordinates
///
/// World coordinates have origin at the center of the canvas.
/// Screen coordinates have origin (0, 0) at top-left corner.
///
/// The transformation pipeline:
/// 1. Apply pan offset
/// 2. Apply zoom
/// 3. Translate to screen coords (origin at top-left)
///
/// # Errors
///
/// Returns an error if the resulting screen coordinates are not finite.
///
/// # Examples
///
/// ```
/// # use oya_ui::components::canvas::coords::{Viewport, world_to_screen};
/// # use oya_ui::models::node::Position;
/// let viewport = Viewport::new(1200.0, 800.0)?;
///
/// // World origin maps to center of screen
/// let world = Position::origin();
/// let (screen_x, screen_y) = world_to_screen(&world, &viewport)?;
/// assert!((screen_x - 600.0).abs() < 0.01);
/// assert!((screen_y - 400.0).abs() < 0.01);
/// # Ok::<(), String>(())
/// ```
pub fn world_to_screen(world_pos: &Position, viewport: &Viewport) -> Result<(f32, f32), String> {
    let zoom = viewport.zoom.value();

    // Step 1 & 2: Apply pan and zoom
    let transformed_x = (world_pos.x + viewport.pan_x) * zoom;
    let transformed_y = (world_pos.y + viewport.pan_y) * zoom;

    // Step 3: Un-center (origin at top-left)
    let screen_x = transformed_x + viewport.canvas_width / 2.0;
    let screen_y = transformed_y + viewport.canvas_height / 2.0;

    // Validate
    if !screen_x.is_finite() || !screen_y.is_finite() {
        return Err(format!(
            "Coordinate transformation produced non-finite values: ({}, {})",
            screen_x, screen_y
        ));
    }

    Ok((screen_x, screen_y))
}

#[cfg(test)]
mod tests {
    use super::*;

    // ========================================================================
    // Viewport Construction Tests
    // ========================================================================

    #[test]
    fn test_viewport_new_valid() -> Result<(), String> {
        let viewport = Viewport::new(1200.0, 800.0)?;
        assert_eq!(viewport.canvas_width, 1200.0);
        assert_eq!(viewport.canvas_height, 800.0);
        assert_eq!(viewport.pan_x, 0.0);
        assert_eq!(viewport.pan_y, 0.0);
        assert_eq!(viewport.zoom.value(), 1.0);
        Ok(())
    }

    #[test]
    fn test_viewport_new_invalid_width() {
        let result = Viewport::new(-100.0, 800.0);
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("width"));
    }

    #[test]
    fn test_viewport_new_invalid_height() {
        let result = Viewport::new(1200.0, 0.0);
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("height"));
    }

    #[test]
    fn test_viewport_new_nan_width() {
        let result = Viewport::new(f32::NAN, 800.0);
        assert!(result.is_err());
    }

    #[test]
    fn test_viewport_new_infinity_height() {
        let result = Viewport::new(1200.0, f32::INFINITY);
        assert!(result.is_err());
    }

    #[test]
    fn test_viewport_with_transform_valid() -> Result<(), String> {
        let zoom = ZoomLevel::new(2.0)?;
        let viewport = Viewport::with_transform(1200.0, 800.0, 100.0, 50.0, zoom)?;
        assert_eq!(viewport.pan_x, 100.0);
        assert_eq!(viewport.pan_y, 50.0);
        assert_eq!(viewport.zoom.value(), 2.0);
        Ok(())
    }

    #[test]
    fn test_viewport_with_transform_invalid_pan() {
        let zoom = ZoomLevel::default();
        let result = Viewport::with_transform(1200.0, 800.0, f32::NAN, 50.0, zoom);
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("Pan X"));
    }

    // ========================================================================
    // Screen to World Transformation Tests
    // ========================================================================

    #[test]
    fn test_screen_to_world_center_maps_to_origin() -> Result<(), String> {
        let viewport = Viewport::new(1200.0, 800.0)?;

        // Center of screen should map to world origin
        let world = screen_to_world(600.0, 400.0, &viewport)?;
        assert!((world.x - 0.0).abs() < 0.01);
        assert!((world.y - 0.0).abs() < 0.01);
        Ok(())
    }

    #[test]
    fn test_screen_to_world_top_left_corner() -> Result<(), String> {
        let viewport = Viewport::new(1200.0, 800.0)?;

        // Top-left corner of screen
        let world = screen_to_world(0.0, 0.0, &viewport)?;
        assert_eq!(world.x, -600.0);
        assert_eq!(world.y, -400.0);
        Ok(())
    }

    #[test]
    fn test_screen_to_world_bottom_right_corner() -> Result<(), String> {
        let viewport = Viewport::new(1200.0, 800.0)?;

        // Bottom-right corner of screen
        let world = screen_to_world(1200.0, 800.0, &viewport)?;
        assert_eq!(world.x, 600.0);
        assert_eq!(world.y, 400.0);
        Ok(())
    }

    #[test]
    fn test_screen_to_world_with_zoom() -> Result<(), String> {
        let zoom = ZoomLevel::new(2.0)?;
        let viewport = Viewport::with_transform(1200.0, 800.0, 0.0, 0.0, zoom)?;

        // With 2x zoom, world coordinates should be half the distance
        let world = screen_to_world(600.0, 200.0, &viewport)?;
        assert_eq!(world.x, 0.0);
        assert_eq!(world.y, -100.0); // Half of -200 due to zoom
        Ok(())
    }

    #[test]
    fn test_screen_to_world_with_pan() -> Result<(), String> {
        let viewport = Viewport::with_transform(1200.0, 800.0, 100.0, 50.0, ZoomLevel::default())?;

        // Pan should shift the world coordinates
        let world = screen_to_world(600.0, 400.0, &viewport)?;
        assert_eq!(world.x, -100.0);
        assert_eq!(world.y, -50.0);
        Ok(())
    }

    #[test]
    fn test_screen_to_world_with_pan_and_zoom() -> Result<(), String> {
        let zoom = ZoomLevel::new(2.0)?;
        let viewport = Viewport::with_transform(1200.0, 800.0, 100.0, 50.0, zoom)?;

        let world = screen_to_world(600.0, 400.0, &viewport)?;
        assert_eq!(world.x, -100.0);
        assert_eq!(world.y, -50.0);
        Ok(())
    }

    #[test]
    fn test_screen_to_world_extreme_zoom_in() -> Result<(), String> {
        let zoom = ZoomLevel::new(5.0)?; // Max zoom
        let viewport = Viewport::with_transform(1200.0, 800.0, 0.0, 0.0, zoom)?;

        let world = screen_to_world(700.0, 400.0, &viewport)?;
        assert_eq!(world.x, 20.0); // 100 / 5
        assert_eq!(world.y, 0.0);
        Ok(())
    }

    #[test]
    fn test_screen_to_world_extreme_zoom_out() -> Result<(), String> {
        let zoom = ZoomLevel::new(0.1)?; // Min zoom
        let viewport = Viewport::with_transform(1200.0, 800.0, 0.0, 0.0, zoom)?;

        let world = screen_to_world(700.0, 400.0, &viewport)?;
        assert_eq!(world.x, 1000.0); // 100 / 0.1
        assert_eq!(world.y, 0.0);
        Ok(())
    }

    // ========================================================================
    // World to Screen Transformation Tests
    // ========================================================================

    #[test]
    fn test_world_to_screen_origin_maps_to_center() -> Result<(), String> {
        let viewport = Viewport::new(1200.0, 800.0)?;

        // World origin should map to center of screen
        let world = Position::origin();
        let (screen_x, screen_y) = world_to_screen(&world, &viewport)?;
        assert_eq!(screen_x, 600.0);
        assert_eq!(screen_y, 400.0);
        Ok(())
    }

    #[test]
    fn test_world_to_screen_positive_coords() -> Result<(), String> {
        let viewport = Viewport::new(1200.0, 800.0)?;

        let world = Position::new(100.0, 50.0)?;
        let (screen_x, screen_y) = world_to_screen(&world, &viewport)?;
        assert_eq!(screen_x, 700.0);
        assert_eq!(screen_y, 450.0);
        Ok(())
    }

    #[test]
    fn test_world_to_screen_negative_coords() -> Result<(), String> {
        let viewport = Viewport::new(1200.0, 800.0)?;

        let world = Position::new(-100.0, -50.0)?;
        let (screen_x, screen_y) = world_to_screen(&world, &viewport)?;
        assert_eq!(screen_x, 500.0);
        assert_eq!(screen_y, 350.0);
        Ok(())
    }

    #[test]
    fn test_world_to_screen_with_zoom() -> Result<(), String> {
        let zoom = ZoomLevel::new(2.0)?;
        let viewport = Viewport::with_transform(1200.0, 800.0, 0.0, 0.0, zoom)?;

        // With 2x zoom, screen coordinates should be doubled
        let world = Position::new(100.0, 50.0)?;
        let (screen_x, screen_y) = world_to_screen(&world, &viewport)?;
        assert_eq!(screen_x, 800.0); // 600 + 200
        assert_eq!(screen_y, 500.0); // 400 + 100
        Ok(())
    }

    #[test]
    fn test_world_to_screen_with_pan() -> Result<(), String> {
        let viewport = Viewport::with_transform(1200.0, 800.0, 100.0, 50.0, ZoomLevel::default())?;

        // Pan should shift the screen coordinates
        let world = Position::origin();
        let (screen_x, screen_y) = world_to_screen(&world, &viewport)?;
        assert_eq!(screen_x, 700.0); // 600 + 100
        assert_eq!(screen_y, 450.0); // 400 + 50
        Ok(())
    }

    #[test]
    fn test_world_to_screen_with_pan_and_zoom() -> Result<(), String> {
        let zoom = ZoomLevel::new(2.0)?;
        let viewport = Viewport::with_transform(1200.0, 800.0, 100.0, 50.0, zoom)?;

        let world = Position::origin();
        let (screen_x, screen_y) = world_to_screen(&world, &viewport)?;
        assert_eq!(screen_x, 800.0); // 600 + (100 * 2)
        assert_eq!(screen_y, 500.0); // 400 + (50 * 2)
        Ok(())
    }

    // ========================================================================
    // Roundtrip Conversion Tests
    // ========================================================================

    #[test]
    fn test_roundtrip_screen_world_screen() -> Result<(), String> {
        let viewport = Viewport::new(1200.0, 800.0)?;

        let original_x = 700.0;
        let original_y = 450.0;

        // Convert to world and back
        let world = screen_to_world(original_x, original_y, &viewport)?;
        let (screen_x, screen_y) = world_to_screen(&world, &viewport)?;

        assert!((screen_x - original_x).abs() < 0.01);
        assert!((screen_y - original_y).abs() < 0.01);
        Ok(())
    }

    #[test]
    fn test_roundtrip_world_screen_world() -> Result<(), String> {
        let viewport = Viewport::new(1200.0, 800.0)?;

        let original_world = Position::new(123.45, 67.89)?;

        // Convert to screen and back
        let (screen_x, screen_y) = world_to_screen(&original_world, &viewport)?;
        let world = screen_to_world(screen_x, screen_y, &viewport)?;

        assert!((world.x - original_world.x).abs() < 0.01);
        assert!((world.y - original_world.y).abs() < 0.01);
        Ok(())
    }

    #[test]
    fn test_roundtrip_with_zoom() -> Result<(), String> {
        let zoom = ZoomLevel::new(3.5)?;
        let viewport = Viewport::with_transform(1920.0, 1080.0, 0.0, 0.0, zoom)?;

        let original_world = Position::new(42.0, -17.5)?;

        let (screen_x, screen_y) = world_to_screen(&original_world, &viewport)?;
        let world = screen_to_world(screen_x, screen_y, &viewport)?;

        assert!((world.x - original_world.x).abs() < 0.01);
        assert!((world.y - original_world.y).abs() < 0.01);
        Ok(())
    }

    #[test]
    fn test_roundtrip_with_pan_and_zoom() -> Result<(), String> {
        let zoom = ZoomLevel::new(2.5)?;
        let viewport = Viewport::with_transform(1600.0, 900.0, 200.0, -150.0, zoom)?;

        let original_world = Position::new(99.9, -88.8)?;

        let (screen_x, screen_y) = world_to_screen(&original_world, &viewport)?;
        let world = screen_to_world(screen_x, screen_y, &viewport)?;

        assert!((world.x - original_world.x).abs() < 0.01);
        assert!((world.y - original_world.y).abs() < 0.01);
        Ok(())
    }

    #[test]
    fn test_roundtrip_extreme_zoom_in() -> Result<(), String> {
        let zoom = ZoomLevel::new(5.0)?; // Max zoom
        let viewport = Viewport::with_transform(1200.0, 800.0, 50.0, 25.0, zoom)?;

        let original_world = Position::new(10.0, -5.0)?;

        let (screen_x, screen_y) = world_to_screen(&original_world, &viewport)?;
        let world = screen_to_world(screen_x, screen_y, &viewport)?;

        assert!((world.x - original_world.x).abs() < 0.01);
        assert!((world.y - original_world.y).abs() < 0.01);
        Ok(())
    }

    #[test]
    fn test_roundtrip_extreme_zoom_out() -> Result<(), String> {
        let zoom = ZoomLevel::new(0.1)?; // Min zoom
        let viewport = Viewport::with_transform(1200.0, 800.0, 1000.0, 500.0, zoom)?;

        let original_world = Position::new(500.0, -250.0)?;

        let (screen_x, screen_y) = world_to_screen(&original_world, &viewport)?;
        let world = screen_to_world(screen_x, screen_y, &viewport)?;

        assert!((world.x - original_world.x).abs() < 0.01);
        assert!((world.y - original_world.y).abs() < 0.01);
        Ok(())
    }

    // ========================================================================
    // Edge Cases and Error Handling
    // ========================================================================

    #[test]
    fn test_large_pan_offsets() -> Result<(), String> {
        let viewport =
            Viewport::with_transform(1200.0, 800.0, 10000.0, -10000.0, ZoomLevel::default())?;

        let world = screen_to_world(600.0, 400.0, &viewport)?;
        assert!(world.x.is_finite());
        assert!(world.y.is_finite());

        let (screen_x, screen_y) = world_to_screen(&Position::origin(), &viewport)?;
        assert!(screen_x.is_finite());
        assert!(screen_y.is_finite());
        Ok(())
    }

    #[test]
    fn test_different_canvas_sizes() -> Result<(), String> {
        let small = Viewport::new(320.0, 240.0)?;
        let large = Viewport::new(3840.0, 2160.0)?;

        // Test that center always maps to origin
        let world_small = screen_to_world(160.0, 120.0, &small)?;
        let world_large = screen_to_world(1920.0, 1080.0, &large)?;

        assert!((world_small.x - 0.0).abs() < 0.01);
        assert!((world_small.y - 0.0).abs() < 0.01);
        assert!((world_large.x - 0.0).abs() < 0.01);
        assert!((world_large.y - 0.0).abs() < 0.01);
        Ok(())
    }

    #[test]
    fn test_canvas_resize_maintains_world_origin() -> Result<(), String> {
        let viewport1 = Viewport::new(1200.0, 800.0)?;
        let viewport2 = Viewport::new(1920.0, 1080.0)?;

        // World origin should always map to canvas center
        let (x1, y1) = world_to_screen(&Position::origin(), &viewport1)?;
        let (x2, y2) = world_to_screen(&Position::origin(), &viewport2)?;

        assert_eq!(x1, 600.0);
        assert_eq!(y1, 400.0);
        assert_eq!(x2, 960.0);
        assert_eq!(y2, 540.0);
        Ok(())
    }

    #[test]
    fn test_negative_pan_offsets() -> Result<(), String> {
        let viewport =
            Viewport::with_transform(1200.0, 800.0, -200.0, -100.0, ZoomLevel::default())?;

        let world = Position::origin();
        let (screen_x, screen_y) = world_to_screen(&world, &viewport)?;

        assert_eq!(screen_x, 400.0); // 600 - 200
        assert_eq!(screen_y, 300.0); // 400 - 100
        Ok(())
    }

    #[test]
    fn test_fractional_coordinates() -> Result<(), String> {
        let viewport = Viewport::new(1200.0, 800.0)?;

        let world = Position::new(12.34567, -98.76543)?;
        let (screen_x, screen_y) = world_to_screen(&world, &viewport)?;
        let roundtrip = screen_to_world(screen_x, screen_y, &viewport)?;

        assert!((roundtrip.x - world.x).abs() < 0.0001);
        assert!((roundtrip.y - world.y).abs() < 0.0001);
        Ok(())
    }
}
