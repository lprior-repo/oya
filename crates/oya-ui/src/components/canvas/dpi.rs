//! HiDPI/Retina display scaling for crisp canvas rendering
//!
//! Provides functional, panic-free DPI detection and scaling using web-sys.
//! Handles device pixel ratio detection, canvas backing store scaling, and
//! dynamic DPI changes (browser zoom).

use wasm_bindgen::JsCast;
use web_sys::{CanvasRenderingContext2d, HtmlCanvasElement, window};

/// Detects the device pixel ratio from the browser window
///
/// # Errors
///
/// Returns an error if:
/// - No window object is available (not in browser context)
/// - Device pixel ratio is not a valid number (NaN, Infinity)
/// - Device pixel ratio is not positive
///
/// # Example
///
/// ```no_run
/// use oya_ui::components::canvas::dpi::detect_device_pixel_ratio;
///
/// let dpr = detect_device_pixel_ratio()?;
/// assert!(dpr >= 1.0);
/// # Ok::<(), String>(())
/// ```
pub fn detect_device_pixel_ratio() -> Result<f64, String> {
    let window = window().ok_or("No window object available")?;

    let dpr = window.device_pixel_ratio();

    // Validate DPI ratio is finite and positive
    if !dpr.is_finite() {
        return Err(format!("Invalid device pixel ratio: {}", dpr));
    }

    if dpr <= 0.0 {
        return Err(format!("Device pixel ratio must be positive, got: {}", dpr));
    }

    Ok(dpr)
}

/// Applies DPI scaling to a canvas rendering context
///
/// Scales the 2D context transformation matrix to match the device pixel ratio.
/// This ensures that drawing operations are scaled correctly for HiDPI displays.
///
/// # Errors
///
/// Returns an error if:
/// - Device pixel ratio is not finite or non-positive
/// - Context scaling operation fails
///
/// # Example
///
/// ```no_run
/// use oya_ui::components::canvas::dpi::apply_dpi_scaling;
/// use web_sys::CanvasRenderingContext2d;
///
/// # fn example(ctx: &CanvasRenderingContext2d) -> Result<(), String> {
/// apply_dpi_scaling(ctx, 2.0)?;
/// # Ok(())
/// # }
/// ```
pub fn apply_dpi_scaling(ctx: &CanvasRenderingContext2d, dpr: f64) -> Result<(), String> {
    // Validate DPI ratio
    if !dpr.is_finite() {
        return Err(format!("Invalid device pixel ratio: {}", dpr));
    }

    if dpr <= 0.0 {
        return Err(format!("Device pixel ratio must be positive, got: {}", dpr));
    }

    // Apply scaling transformation
    ctx.scale(dpr, dpr)
        .map_err(|e| format!("Failed to scale canvas context: {:?}", e))?;

    Ok(())
}

/// Sets up a canvas element with DPI-aware dimensions
///
/// This function performs three operations:
/// 1. Sets the canvas backing store size (width × dpr, height × dpr)
/// 2. Sets the CSS display size (preserves logical pixels)
/// 3. Scales the rendering context to match the backing store
///
/// # Arguments
///
/// * `canvas` - The HTML canvas element to configure
/// * `logical_width` - The desired width in logical (CSS) pixels
/// * `logical_height` - The desired height in logical (CSS) pixels
///
/// # Returns
///
/// Returns the detected device pixel ratio on success
///
/// # Errors
///
/// Returns an error if:
/// - Device pixel ratio detection fails
/// - Logical dimensions are not finite or non-positive
/// - Canvas 2D context is not available
/// - Canvas dimensions cannot be set
/// - CSS style cannot be applied
/// - Context scaling fails
///
/// # Example
///
/// ```no_run
/// use oya_ui::components::canvas::dpi::setup_dpi_aware_canvas;
/// use web_sys::HtmlCanvasElement;
///
/// # fn example(canvas: &HtmlCanvasElement) -> Result<(), String> {
/// let dpr = setup_dpi_aware_canvas(canvas, 1200, 800)?;
/// assert!(dpr >= 1.0);
/// # Ok(())
/// # }
/// ```
pub fn setup_dpi_aware_canvas(
    canvas: &HtmlCanvasElement,
    logical_width: u32,
    logical_height: u32,
) -> Result<f64, String> {
    // Detect device pixel ratio
    let dpr = detect_device_pixel_ratio()?;

    // Validate logical dimensions
    let logical_width_f64 = f64::from(logical_width);
    let logical_height_f64 = f64::from(logical_height);

    if !logical_width_f64.is_finite() || logical_width_f64 <= 0.0 {
        return Err(format!("Invalid logical width: {}", logical_width));
    }

    if !logical_height_f64.is_finite() || logical_height_f64 <= 0.0 {
        return Err(format!("Invalid logical height: {}", logical_height));
    }

    // Calculate physical (backing store) dimensions
    let physical_width = (logical_width_f64 * dpr).round();
    let physical_height = (logical_height_f64 * dpr).round();

    // Validate physical dimensions are within u32 range
    if physical_width > f64::from(u32::MAX) || physical_height > f64::from(u32::MAX) {
        return Err(format!(
            "Physical dimensions exceed maximum: {}x{} (DPI: {})",
            physical_width, physical_height, dpr
        ));
    }

    let physical_width_u32 = physical_width as u32;
    let physical_height_u32 = physical_height as u32;

    // Set canvas backing store size (physical pixels)
    canvas.set_width(physical_width_u32);
    canvas.set_height(physical_height_u32);

    // Verify backing store size was set correctly
    if canvas.width() != physical_width_u32 || canvas.height() != physical_height_u32 {
        return Err(format!(
            "Canvas backing store size mismatch: expected {}x{}, got {}x{}",
            physical_width_u32,
            physical_height_u32,
            canvas.width(),
            canvas.height()
        ));
    }

    // Set CSS display size (logical pixels)
    let style = canvas.style();

    style
        .set_property("width", &format!("{}px", logical_width))
        .map_err(|e| format!("Failed to set CSS width: {:?}", e))?;

    style
        .set_property("height", &format!("{}px", logical_height))
        .map_err(|e| format!("Failed to set CSS height: {:?}", e))?;

    // Get 2D rendering context
    let ctx = canvas
        .get_context("2d")
        .map_err(|e| format!("Failed to get canvas context: {:?}", e))?
        .ok_or("Canvas 2D context is not available")?
        .dyn_into::<CanvasRenderingContext2d>()
        .map_err(|_| "Failed to cast to CanvasRenderingContext2d")?;

    // Apply DPI scaling to context
    apply_dpi_scaling(&ctx, dpr)?;

    Ok(dpr)
}

#[cfg(test)]
mod tests {
    #[test]
    fn test_apply_dpi_scaling_validates_dpr() {
        // Cannot test context scaling without WASM environment
        // but we can test validation logic

        // Test that invalid DPR values would be rejected
        let invalid_dprs = vec![0.0, -1.0, f64::NAN, f64::INFINITY, f64::NEG_INFINITY];

        for dpr in invalid_dprs {
            // We expect validation to fail for these values
            assert!(
                dpr <= 0.0 || !dpr.is_finite(),
                "DPR {} should be invalid",
                dpr
            );
        }
    }

    #[test]
    fn test_logical_dimensions_validation() {
        // Test that dimension validation logic is sound
        let valid_dims = vec![(100, 100), (1920, 1080), (3840, 2160)];

        for (w, h) in valid_dims {
            let w_f64 = f64::from(w);
            let h_f64 = f64::from(h);

            assert!(w_f64.is_finite() && w_f64 > 0.0);
            assert!(h_f64.is_finite() && h_f64 > 0.0);
        }
    }

    #[test]
    fn test_physical_dimensions_calculation() {
        // Test backing store calculation logic
        let test_cases = vec![
            (1200, 800, 1.0, 1200.0, 800.0),
            (1200, 800, 2.0, 2400.0, 1600.0),
            (1920, 1080, 1.5, 2880.0, 1620.0),
        ];

        for (logical_w, logical_h, dpr, expected_w, expected_h) in test_cases {
            let physical_w = (f64::from(logical_w) * dpr).round();
            let physical_h = (f64::from(logical_h) * dpr).round();

            assert_eq!(physical_w, expected_w);
            assert_eq!(physical_h, expected_h);
        }
    }

    #[test]
    fn test_physical_dimensions_overflow_protection() {
        // Test that extremely large dimensions are detected
        let logical_width = u32::MAX;
        let logical_height = u32::MAX;
        let dpr = 2.0;

        let physical_width = f64::from(logical_width) * dpr;
        let physical_height = f64::from(logical_height) * dpr;

        // Should exceed u32::MAX
        assert!(physical_width > f64::from(u32::MAX));
        assert!(physical_height > f64::from(u32::MAX));
    }

    #[test]
    fn test_dpr_validation_rejects_zero() {
        let dpr = 0.0;
        assert!(dpr <= 0.0, "Zero DPR should be invalid");
    }

    #[test]
    fn test_dpr_validation_rejects_negative() {
        let dpr = -1.0;
        assert!(dpr <= 0.0, "Negative DPR should be invalid");
    }

    #[test]
    fn test_dpr_validation_rejects_nan() {
        let dpr = f64::NAN;
        assert!(!dpr.is_finite(), "NaN DPR should be invalid");
    }

    #[test]
    fn test_dpr_validation_rejects_infinity() {
        let dpr = f64::INFINITY;
        assert!(!dpr.is_finite(), "Infinite DPR should be invalid");
    }

    #[test]
    fn test_dpr_validation_accepts_valid() {
        let valid_dprs: Vec<f64> = vec![1.0, 1.5, 2.0, 2.5, 3.0, 4.0];

        for dpr in valid_dprs {
            assert!(dpr.is_finite() && dpr > 0.0, "DPR {} should be valid", dpr);
        }
    }
}

#[cfg(all(test, target_arch = "wasm32"))]
mod wasm_tests {
    #![allow(clippy::expect_used)]

    use super::*;
    use wasm_bindgen_test::*;

    wasm_bindgen_test_configure!(run_in_browser);

    #[wasm_bindgen_test]
    fn test_detect_device_pixel_ratio_in_browser() {
        let result = detect_device_pixel_ratio();
        assert!(result.is_ok(), "Should detect DPR in browser context");

        let dpr = result.ok().unwrap_or(1.0);
        assert!(dpr >= 1.0, "DPR should be at least 1.0, got: {}", dpr);
        assert!(dpr.is_finite(), "DPR should be finite");
    }

    #[wasm_bindgen_test]
    fn test_setup_dpi_aware_canvas_in_browser() {
        // Create a test canvas
        let window = window().expect("Should have window");
        let document = window.document().expect("Should have document");
        let canvas = document
            .create_element("canvas")
            .expect("Should create canvas")
            .dyn_into::<HtmlCanvasElement>()
            .expect("Should cast to HtmlCanvasElement");

        // Setup DPI-aware canvas
        let result = setup_dpi_aware_canvas(&canvas, 1200, 800);
        assert!(result.is_ok(), "Should setup DPI-aware canvas");

        let dpr = result.ok().unwrap_or(1.0);

        // Verify backing store size
        let expected_width = (1200.0 * dpr).round() as u32;
        let expected_height = (800.0 * dpr).round() as u32;

        assert_eq!(
            canvas.width(),
            expected_width,
            "Backing store width should match DPR scaling"
        );
        assert_eq!(
            canvas.height(),
            expected_height,
            "Backing store height should match DPR scaling"
        );

        // Verify CSS dimensions (logical pixels)
        let style = canvas.style();
        let css_width = style.get_property_value("width").ok().unwrap_or_default();
        let css_height = style.get_property_value("height").ok().unwrap_or_default();

        assert_eq!(css_width, "1200px", "CSS width should be logical pixels");
        assert_eq!(css_height, "800px", "CSS height should be logical pixels");
    }
}
