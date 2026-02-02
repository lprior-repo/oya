//! Comprehensive tests for zoom functionality
//!
//! Tests cover:
//! - Configuration validation
//! - Zoom factor calculation
//! - Zoom application with bounds
//! - Origin preservation
//! - Viewport offset calculation
//! - Complete zoom-at-point workflow
//! - Edge cases and error handling

use super::bounds::ZoomLevel;
use super::zoom::{
    ZoomConfig, ZoomResult, apply_zoom, calculate_viewport_offset, calculate_zoom_factor,
    calculate_zoom_origin, zoom_at_point,
};

// ============================================================================
// ZoomConfig Tests
// ============================================================================

#[test]
fn test_zoom_config_default_values() {
    let config = ZoomConfig::default();
    assert_eq!(config.sensitivity(), 0.001);
    assert_eq!(config.min_zoom_factor(), 0.5);
    assert_eq!(config.max_zoom_factor(), 2.0);
}

#[test]
fn test_zoom_config_custom_valid() -> Result<(), String> {
    let config = ZoomConfig::new(0.002, 0.7, 1.5)?;
    assert_eq!(config.sensitivity(), 0.002);
    assert_eq!(config.min_zoom_factor(), 0.7);
    assert_eq!(config.max_zoom_factor(), 1.5);
    Ok(())
}

#[test]
fn test_zoom_config_slow_zoom() -> Result<(), String> {
    let config = ZoomConfig::new(0.0001, 0.9, 1.1)?;
    assert_eq!(config.sensitivity(), 0.0001);
    Ok(())
}

#[test]
fn test_zoom_config_fast_zoom() -> Result<(), String> {
    let config = ZoomConfig::new(0.005, 0.3, 3.0)?;
    assert_eq!(config.sensitivity(), 0.005);
    Ok(())
}

#[test]
fn test_zoom_config_negative_sensitivity() {
    let result = ZoomConfig::new(-0.001, 0.5, 2.0);
    assert!(result.is_err());
    assert!(
        result
            .as_ref()
            .err()
            .map_or(false, |e| e.contains("positive"))
    );
}

#[test]
fn test_zoom_config_zero_sensitivity() {
    let result = ZoomConfig::new(0.0, 0.5, 2.0);
    assert!(result.is_err());
}

#[test]
fn test_zoom_config_nan_sensitivity() {
    let result = ZoomConfig::new(f32::NAN, 0.5, 2.0);
    assert!(result.is_err());
    assert!(
        result
            .as_ref()
            .err()
            .map_or(false, |e| e.contains("finite"))
    );
}

#[test]
fn test_zoom_config_infinite_sensitivity() {
    let result = ZoomConfig::new(f32::INFINITY, 0.5, 2.0);
    assert!(result.is_err());
}

#[test]
fn test_zoom_config_min_factor_too_large() {
    let result = ZoomConfig::new(0.001, 1.5, 2.0);
    assert!(result.is_err());
    assert!(result.as_ref().err().map_or(false, |e| e.contains("range")));
}

#[test]
fn test_zoom_config_min_factor_negative() {
    let result = ZoomConfig::new(0.001, -0.5, 2.0);
    assert!(result.is_err());
}

#[test]
fn test_zoom_config_max_factor_too_small() {
    let result = ZoomConfig::new(0.001, 0.5, 0.8);
    assert!(result.is_err());
    assert!(result.as_ref().err().map_or(false, |e| e.contains(">=")));
}

#[test]
fn test_zoom_config_min_equals_max() {
    // When min == max, we get an error (could be min/max range error or less than error)
    let result = ZoomConfig::new(0.001, 0.8, 0.8);
    assert!(result.is_err());
    // The error could be about range or about min < max, both are valid
}

#[test]
fn test_zoom_config_min_greater_than_max() {
    let result = ZoomConfig::new(0.001, 0.9, 0.7);
    assert!(result.is_err());
}

// ============================================================================
// calculate_zoom_factor Tests
// ============================================================================

#[test]
fn test_calculate_zoom_factor_zoom_in_small() -> Result<(), String> {
    let config = ZoomConfig::default();
    let factor = calculate_zoom_factor(-50.0, &config)?;

    // Negative delta = zoom in (factor > 1.0)
    assert!(factor > 1.0);
    assert!(factor <= config.max_zoom_factor());
    // -50 * 0.001 = -0.05, so factor = 1 - (-0.05) = 1.05
    assert!((factor - 1.05).abs() < 0.001);

    Ok(())
}

#[test]
fn test_calculate_zoom_factor_zoom_in_large() -> Result<(), String> {
    let config = ZoomConfig::default();
    let factor = calculate_zoom_factor(-500.0, &config)?;

    // Large negative delta = zoom in (factor > 1.0)
    assert!(factor > 1.0);
    // -500 * 0.001 = -0.5, so factor = 1 - (-0.5) = 1.5
    assert!((factor - 1.5).abs() < 0.001);

    Ok(())
}

#[test]
fn test_calculate_zoom_factor_zoom_out_small() -> Result<(), String> {
    let config = ZoomConfig::default();
    let factor = calculate_zoom_factor(50.0, &config)?;

    // Positive delta = zoom out (factor < 1.0)
    assert!(factor < 1.0);
    assert!(factor >= config.min_zoom_factor());
    // 50 * 0.001 = 0.05, so factor = 1 - 0.05 = 0.95
    assert!((factor - 0.95).abs() < 0.001);

    Ok(())
}

#[test]
fn test_calculate_zoom_factor_zoom_out_large() -> Result<(), String> {
    let config = ZoomConfig::default();
    let factor = calculate_zoom_factor(500.0, &config)?;

    // Large positive delta = zoom out (factor < 1.0)
    assert!(factor < 1.0);
    // 500 * 0.001 = 0.5, so factor = 1 - 0.5 = 0.5
    assert!((factor - 0.5).abs() < 0.001);

    Ok(())
}

#[test]
fn test_calculate_zoom_factor_zero_delta() -> Result<(), String> {
    let config = ZoomConfig::default();
    let factor = calculate_zoom_factor(0.0, &config)?;

    // Zero delta = no zoom (factor = 1.0)
    assert_eq!(factor, 1.0);

    Ok(())
}

#[test]
fn test_calculate_zoom_factor_clamping_max() -> Result<(), String> {
    let config = ZoomConfig::default();
    let factor = calculate_zoom_factor(-5000.0, &config)?;

    // Very large negative delta gets clamped to max
    assert_eq!(factor, config.max_zoom_factor());
    assert_eq!(factor, 2.0);

    Ok(())
}

#[test]
fn test_calculate_zoom_factor_clamping_min() -> Result<(), String> {
    let config = ZoomConfig::default();
    let factor = calculate_zoom_factor(5000.0, &config)?;

    // Very large positive delta gets clamped to min
    assert_eq!(factor, config.min_zoom_factor());
    assert_eq!(factor, 0.5);

    Ok(())
}

#[test]
fn test_calculate_zoom_factor_nan_delta() {
    let config = ZoomConfig::default();
    let result = calculate_zoom_factor(f64::NAN, &config);

    assert!(result.is_err());
    assert!(
        result
            .as_ref()
            .err()
            .map_or(false, |e| e.contains("finite"))
    );
}

#[test]
fn test_calculate_zoom_factor_infinite_delta() {
    let config = ZoomConfig::default();

    assert!(calculate_zoom_factor(f64::INFINITY, &config).is_err());
    assert!(calculate_zoom_factor(f64::NEG_INFINITY, &config).is_err());
}

#[test]
fn test_calculate_zoom_factor_custom_sensitivity() -> Result<(), String> {
    let config = ZoomConfig::new(0.002, 0.5, 2.0)?;
    let factor = calculate_zoom_factor(-100.0, &config)?;

    // -100 * 0.002 = -0.2, so factor = 1 - (-0.2) = 1.2
    assert!((factor - 1.2).abs() < 0.001);

    Ok(())
}

// ============================================================================
// apply_zoom Tests
// ============================================================================

#[test]
fn test_apply_zoom_zoom_in() -> Result<(), String> {
    let current = ZoomLevel::new(1.0)?;
    let zoomed = apply_zoom(current, 1.5)?;

    assert_eq!(zoomed.value(), 1.5);

    Ok(())
}

#[test]
fn test_apply_zoom_zoom_out() -> Result<(), String> {
    let current = ZoomLevel::new(2.0)?;
    let zoomed = apply_zoom(current, 0.75)?;

    assert_eq!(zoomed.value(), 1.5);

    Ok(())
}

#[test]
fn test_apply_zoom_no_change() -> Result<(), String> {
    let current = ZoomLevel::new(1.5)?;
    let zoomed = apply_zoom(current, 1.0)?;

    assert_eq!(zoomed.value(), 1.5);

    Ok(())
}

#[test]
fn test_apply_zoom_clamping_max() -> Result<(), String> {
    let current = ZoomLevel::new(3.0)?;
    let zoomed = apply_zoom(current, 5.0)?;

    // 3.0 * 5.0 = 15.0, clamped to ZoomLevel max (5.0)
    assert_eq!(zoomed.value(), 5.0);

    Ok(())
}

#[test]
fn test_apply_zoom_clamping_min() -> Result<(), String> {
    let current = ZoomLevel::new(0.5)?;
    let zoomed = apply_zoom(current, 0.1)?;

    // 0.5 * 0.1 = 0.05, clamped to ZoomLevel min (0.1)
    assert_eq!(zoomed.value(), 0.1);

    Ok(())
}

#[test]
fn test_apply_zoom_at_max_boundary() -> Result<(), String> {
    let current = ZoomLevel::new(5.0)?;
    let zoomed = apply_zoom(current, 1.01)?;

    // Already at max, should stay at max
    assert_eq!(zoomed.value(), 5.0);

    Ok(())
}

#[test]
fn test_apply_zoom_at_min_boundary() -> Result<(), String> {
    let current = ZoomLevel::new(0.1)?;
    let zoomed = apply_zoom(current, 0.99)?;

    // Would go below min, should stay at min
    assert_eq!(zoomed.value(), 0.1);

    Ok(())
}

#[test]
fn test_apply_zoom_nan_factor() -> Result<(), String> {
    let current = ZoomLevel::new(1.0)?;
    let result = apply_zoom(current, f32::NAN);

    assert!(result.is_err());
    assert!(
        result
            .as_ref()
            .err()
            .map_or(false, |e| e.contains("finite"))
    );

    Ok(())
}

#[test]
fn test_apply_zoom_infinite_factor() -> Result<(), String> {
    let current = ZoomLevel::new(1.0)?;

    assert!(apply_zoom(current, f32::INFINITY).is_err());
    assert!(apply_zoom(current, f32::NEG_INFINITY).is_err());

    Ok(())
}

// ============================================================================
// calculate_zoom_origin Tests
// ============================================================================

#[test]
fn test_calculate_zoom_origin_center() -> Result<(), String> {
    let zoom = ZoomLevel::new(1.0)?;
    let (origin_x, origin_y) = calculate_zoom_origin(400.0, 300.0, 0.0, 0.0, zoom)?;

    // At zoom 1.0 with no viewport offset, canvas = world
    assert_eq!(origin_x, 400.0);
    assert_eq!(origin_y, 300.0);

    Ok(())
}

#[test]
fn test_calculate_zoom_origin_with_viewport_offset() -> Result<(), String> {
    let zoom = ZoomLevel::new(1.0)?;
    let (origin_x, origin_y) = calculate_zoom_origin(400.0, 300.0, 100.0, 50.0, zoom)?;

    // world = (canvas - viewport) / zoom
    // world_x = (400 - 100) / 1.0 = 300
    // world_y = (300 - 50) / 1.0 = 250
    assert_eq!(origin_x, 300.0);
    assert_eq!(origin_y, 250.0);

    Ok(())
}

#[test]
fn test_calculate_zoom_origin_with_zoom() -> Result<(), String> {
    let zoom = ZoomLevel::new(2.0)?;
    let (origin_x, origin_y) = calculate_zoom_origin(400.0, 300.0, 100.0, 50.0, zoom)?;

    // world = (canvas - viewport) / zoom
    // world_x = (400 - 100) / 2.0 = 150
    // world_y = (300 - 50) / 2.0 = 125
    assert_eq!(origin_x, 150.0);
    assert_eq!(origin_y, 125.0);

    Ok(())
}

#[test]
fn test_calculate_zoom_origin_negative_viewport() -> Result<(), String> {
    let zoom = ZoomLevel::new(1.5)?;
    let (origin_x, origin_y) = calculate_zoom_origin(200.0, 150.0, -100.0, -50.0, zoom)?;

    // world = (canvas - viewport) / zoom
    // world_x = (200 - (-100)) / 1.5 = 300 / 1.5 = 200
    // world_y = (150 - (-50)) / 1.5 = 200 / 1.5 = 133.333...
    assert!((origin_x - 200.0).abs() < 0.001);
    assert!((origin_y - 133.333).abs() < 0.01);

    Ok(())
}

#[test]
fn test_calculate_zoom_origin_nan_cursor() -> Result<(), String> {
    let zoom = ZoomLevel::new(1.0)?;

    assert!(calculate_zoom_origin(f64::NAN, 0.0, 0.0, 0.0, zoom).is_err());
    assert!(calculate_zoom_origin(0.0, f64::NAN, 0.0, 0.0, zoom).is_err());

    Ok(())
}

#[test]
fn test_calculate_zoom_origin_nan_viewport() -> Result<(), String> {
    let zoom = ZoomLevel::new(1.0)?;

    assert!(calculate_zoom_origin(0.0, 0.0, f64::NAN, 0.0, zoom).is_err());
    assert!(calculate_zoom_origin(0.0, 0.0, 0.0, f64::NAN, zoom).is_err());

    Ok(())
}

#[test]
fn test_calculate_zoom_origin_infinite_coords() -> Result<(), String> {
    let zoom = ZoomLevel::new(1.0)?;

    assert!(calculate_zoom_origin(f64::INFINITY, 0.0, 0.0, 0.0, zoom).is_err());
    assert!(calculate_zoom_origin(0.0, f64::INFINITY, 0.0, 0.0, zoom).is_err());
    assert!(calculate_zoom_origin(0.0, 0.0, f64::INFINITY, 0.0, zoom).is_err());
    assert!(calculate_zoom_origin(0.0, 0.0, 0.0, f64::INFINITY, zoom).is_err());

    Ok(())
}

// ============================================================================
// calculate_viewport_offset Tests
// ============================================================================

#[test]
fn test_calculate_viewport_offset_center() -> Result<(), String> {
    let zoom = ZoomLevel::new(1.0)?;
    let (viewport_x, viewport_y) = calculate_viewport_offset(400.0, 300.0, 400.0, 300.0, zoom)?;

    // viewport = cursor - (origin * zoom)
    // viewport_x = 400 - (400 * 1.0) = 0
    // viewport_y = 300 - (300 * 1.0) = 0
    assert_eq!(viewport_x, 0.0);
    assert_eq!(viewport_y, 0.0);

    Ok(())
}

#[test]
fn test_calculate_viewport_offset_with_zoom() -> Result<(), String> {
    let zoom = ZoomLevel::new(2.0)?;
    let (viewport_x, viewport_y) = calculate_viewport_offset(400.0, 300.0, 150.0, 125.0, zoom)?;

    // viewport = cursor - (origin * zoom)
    // viewport_x = 400 - (150 * 2.0) = 100
    // viewport_y = 300 - (125 * 2.0) = 50
    assert_eq!(viewport_x, 100.0);
    assert_eq!(viewport_y, 50.0);

    Ok(())
}

#[test]
fn test_calculate_viewport_offset_negative() -> Result<(), String> {
    let zoom = ZoomLevel::new(1.5)?;
    let (viewport_x, viewport_y) = calculate_viewport_offset(100.0, 100.0, 200.0, 150.0, zoom)?;

    // viewport = cursor - (origin * zoom)
    // viewport_x = 100 - (200 * 1.5) = -200
    // viewport_y = 100 - (150 * 1.5) = -125
    assert_eq!(viewport_x, -200.0);
    assert_eq!(viewport_y, -125.0);

    Ok(())
}

#[test]
fn test_calculate_viewport_offset_nan_cursor() -> Result<(), String> {
    let zoom = ZoomLevel::new(1.0)?;

    assert!(calculate_viewport_offset(f64::NAN, 0.0, 0.0, 0.0, zoom).is_err());
    assert!(calculate_viewport_offset(0.0, f64::NAN, 0.0, 0.0, zoom).is_err());

    Ok(())
}

#[test]
fn test_calculate_viewport_offset_nan_origin() -> Result<(), String> {
    let zoom = ZoomLevel::new(1.0)?;

    assert!(calculate_viewport_offset(0.0, 0.0, f64::NAN, 0.0, zoom).is_err());
    assert!(calculate_viewport_offset(0.0, 0.0, 0.0, f64::NAN, zoom).is_err());

    Ok(())
}

#[test]
fn test_calculate_viewport_offset_infinite_coords() -> Result<(), String> {
    let zoom = ZoomLevel::new(1.0)?;

    assert!(calculate_viewport_offset(f64::INFINITY, 0.0, 0.0, 0.0, zoom).is_err());
    assert!(calculate_viewport_offset(0.0, 0.0, f64::INFINITY, 0.0, zoom).is_err());

    Ok(())
}

// ============================================================================
// zoom_at_point Integration Tests
// ============================================================================

#[test]
fn test_zoom_at_point_zoom_in_from_default() -> Result<(), String> {
    let config = ZoomConfig::default();
    let current_zoom = ZoomLevel::default();

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

    // Viewport should be adjusted
    assert!(result.new_viewport_x.is_finite());
    assert!(result.new_viewport_y.is_finite());

    // Origin should be recorded
    assert_eq!(result.origin_x, 400.0); // cursor was at 400
    assert_eq!(result.origin_y, 300.0); // cursor was at 300

    Ok(())
}

#[test]
fn test_zoom_at_point_zoom_out_from_zoomed() -> Result<(), String> {
    let config = ZoomConfig::default();
    let current_zoom = ZoomLevel::new(3.0)?;

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
    let cursor_x = 400.0;
    let cursor_y = 300.0;

    let result = zoom_at_point(
        current_zoom,
        0.0,
        0.0,
        cursor_x,
        cursor_y,
        -100.0, // zoom in
        &config,
    )?;

    // After zoom, the origin should map back to cursor position
    let zoom_f64 = result.new_zoom.value() as f64;
    let canvas_x = result.new_viewport_x + (result.origin_x * zoom_f64);
    let canvas_y = result.new_viewport_y + (result.origin_y * zoom_f64);

    // Should be very close to original cursor position
    assert!((canvas_x - cursor_x).abs() < 0.01);
    assert!((canvas_y - cursor_y).abs() < 0.01);

    Ok(())
}

#[test]
fn test_zoom_at_point_with_viewport_offset() -> Result<(), String> {
    let config = ZoomConfig::default();
    let current_zoom = ZoomLevel::new(1.5)?;

    let result = zoom_at_point(current_zoom, -50.0, -100.0, 200.0, 150.0, -75.0, &config)?;

    // Zoom should be applied
    assert_ne!(result.new_zoom.value(), current_zoom.value());

    // Viewport should be adjusted
    assert_ne!(result.new_viewport_x, -50.0);
    assert_ne!(result.new_viewport_y, -100.0);

    Ok(())
}

#[test]
fn test_zoom_at_point_zero_delta() -> Result<(), String> {
    let config = ZoomConfig::default();
    let current_zoom = ZoomLevel::new(2.0)?;
    let viewport_x = 100.0;
    let viewport_y = 50.0;

    let result = zoom_at_point(
        current_zoom,
        viewport_x,
        viewport_y,
        400.0,
        300.0,
        0.0, // no zoom
        &config,
    )?;

    // Zoom should not change
    assert_eq!(result.new_zoom.value(), current_zoom.value());

    Ok(())
}

#[test]
fn test_zoom_at_point_multiple_zooms() -> Result<(), String> {
    let config = ZoomConfig::default();
    let mut current_zoom = ZoomLevel::new(1.0)?;
    let mut viewport_x = 0.0;
    let mut viewport_y = 0.0;
    let cursor_x = 400.0;
    let cursor_y = 300.0;

    // Zoom in twice
    for _ in 0..2 {
        let result = zoom_at_point(
            current_zoom,
            viewport_x,
            viewport_y,
            cursor_x,
            cursor_y,
            -100.0,
            &config,
        )?;

        current_zoom = result.new_zoom;
        viewport_x = result.new_viewport_x;
        viewport_y = result.new_viewport_y;
    }

    // Should have zoomed in
    assert!(current_zoom.value() > 1.0);

    // Origin preservation should still work
    let zoom_f64 = current_zoom.value() as f64;
    let canvas_x = viewport_x + ((cursor_x - 0.0) / 1.0 * zoom_f64);
    let canvas_y = viewport_y + ((cursor_y - 0.0) / 1.0 * zoom_f64);

    // After multiple zooms, point should still be close
    assert!((canvas_x - cursor_x).abs() < 1.0);
    assert!((canvas_y - cursor_y).abs() < 1.0);

    Ok(())
}

#[test]
fn test_zoom_at_point_at_max_zoom() -> Result<(), String> {
    let config = ZoomConfig::default();
    let current_zoom = ZoomLevel::new(5.0)?; // max zoom

    let result = zoom_at_point(
        current_zoom,
        0.0,
        0.0,
        400.0,
        300.0,
        -1000.0, // try to zoom in more
        &config,
    )?;

    // Should stay at max
    assert_eq!(result.new_zoom.value(), 5.0);

    Ok(())
}

#[test]
fn test_zoom_at_point_at_min_zoom() -> Result<(), String> {
    let config = ZoomConfig::default();
    let current_zoom = ZoomLevel::new(0.1)?; // min zoom

    let result = zoom_at_point(
        current_zoom,
        0.0,
        0.0,
        400.0,
        300.0,
        1000.0, // try to zoom out more
        &config,
    )?;

    // Should stay at min
    assert_eq!(result.new_zoom.value(), 0.1);

    Ok(())
}

// ============================================================================
// ZoomResult Tests
// ============================================================================

#[test]
fn test_zoom_result_structure() -> Result<(), String> {
    let result = ZoomResult {
        new_zoom: ZoomLevel::new(2.0)?,
        new_viewport_x: 100.0,
        new_viewport_y: 50.0,
        origin_x: 200.0,
        origin_y: 150.0,
    };

    assert_eq!(result.new_zoom.value(), 2.0);
    assert_eq!(result.new_viewport_x, 100.0);
    assert_eq!(result.new_viewport_y, 50.0);
    assert_eq!(result.origin_x, 200.0);
    assert_eq!(result.origin_y, 150.0);

    Ok(())
}

#[test]
fn test_zoom_result_equality() -> Result<(), String> {
    let result1 = ZoomResult {
        new_zoom: ZoomLevel::new(2.0)?,
        new_viewport_x: 100.0,
        new_viewport_y: 50.0,
        origin_x: 200.0,
        origin_y: 150.0,
    };

    let result2 = ZoomResult {
        new_zoom: ZoomLevel::new(2.0)?,
        new_viewport_x: 100.0,
        new_viewport_y: 50.0,
        origin_x: 200.0,
        origin_y: 150.0,
    };

    assert_eq!(result1, result2);

    Ok(())
}

// ============================================================================
// Property-Based Tests
// ============================================================================

#[test]
fn test_zoom_factor_always_positive() -> Result<(), String> {
    let config = ZoomConfig::default();

    for delta in [-1000.0, -100.0, -10.0, 0.0, 10.0, 100.0, 1000.0] {
        let factor = calculate_zoom_factor(delta, &config)?;
        assert!(
            factor > 0.0,
            "Factor should be positive for delta {}",
            delta
        );
    }

    Ok(())
}

#[test]
fn test_zoom_application_preserves_finiteness() -> Result<(), String> {
    let zoom_levels = [0.1, 0.5, 1.0, 2.0, 5.0];
    let factors = [0.5, 0.8, 1.0, 1.2, 2.0];

    for &level in &zoom_levels {
        for &factor in &factors {
            let current = ZoomLevel::new(level)?;
            let result = apply_zoom(current, factor)?;
            assert!(result.value().is_finite());
        }
    }

    Ok(())
}

#[test]
fn test_origin_calculation_always_finite() -> Result<(), String> {
    let zoom = ZoomLevel::new(1.5)?;
    let cursors = [(0.0, 0.0), (100.0, 100.0), (800.0, 600.0)];
    let viewports = [(-100.0, -100.0), (0.0, 0.0), (100.0, 100.0)];

    for &(cx, cy) in &cursors {
        for &(vx, vy) in &viewports {
            let (ox, oy) = calculate_zoom_origin(cx, cy, vx, vy, zoom)?;
            assert!(ox.is_finite());
            assert!(oy.is_finite());
        }
    }

    Ok(())
}
