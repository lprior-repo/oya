//! WASM-specific tests for canvas resize functionality

use super::*;
use wasm_bindgen::JsCast;
use wasm_bindgen_test::*;
use web_sys::HtmlCanvasElement;

wasm_bindgen_test_configure!(run_in_browser);

/// Helper to create a test canvas element
fn create_test_canvas() -> Result<HtmlCanvasElement, String> {
    let window = window().ok_or("No window object")?;
    let document = window.document().ok_or("No document object")?;

    let canvas = document
        .create_element("canvas")
        .map_err(|e| format!("Failed to create canvas: {:?}", e))?
        .dyn_into::<HtmlCanvasElement>()
        .map_err(|_| "Failed to cast to HtmlCanvasElement")?;

    Ok(canvas)
}

#[wasm_bindgen_test]
fn test_resize_canvas_basic() {
    let canvas = create_test_canvas().expect("Failed to create canvas");

    let result = resize_canvas(&canvas, 1920, 1080);
    assert!(result.is_ok(), "Resize should succeed");
    assert_eq!(canvas.width(), 1920);
    assert_eq!(canvas.height(), 1080);
}

#[wasm_bindgen_test]
fn test_resize_canvas_multiple_times() {
    let canvas = create_test_canvas().expect("Failed to create canvas");

    // First resize
    resize_canvas(&canvas, 1920, 1080).expect("First resize failed");
    assert_eq!(canvas.width(), 1920);
    assert_eq!(canvas.height(), 1080);

    // Second resize
    resize_canvas(&canvas, 800, 600).expect("Second resize failed");
    assert_eq!(canvas.width(), 800);
    assert_eq!(canvas.height(), 600);

    // Third resize back to larger
    resize_canvas(&canvas, 2560, 1440).expect("Third resize failed");
    assert_eq!(canvas.width(), 2560);
    assert_eq!(canvas.height(), 1440);
}

#[wasm_bindgen_test]
fn test_resize_canvas_extreme_small() {
    let canvas = create_test_canvas().expect("Failed to create canvas");

    let result = resize_canvas(&canvas, 1, 1);
    assert!(result.is_ok(), "Resize to 1x1 should succeed");
    assert_eq!(canvas.width(), 1);
    assert_eq!(canvas.height(), 1);
}

#[wasm_bindgen_test]
fn test_resize_canvas_extreme_large() {
    let canvas = create_test_canvas().expect("Failed to create canvas");

    let result = resize_canvas(&canvas, 8192, 8192);
    assert!(result.is_ok(), "Resize to 8192x8192 should succeed");
    assert_eq!(canvas.width(), 8192);
    assert_eq!(canvas.height(), 8192);
}

#[wasm_bindgen_test]
fn test_get_window_size_returns_positive() {
    let result = get_window_size();
    assert!(result.is_ok(), "get_window_size should succeed in browser");

    let (width, height) = result.expect("Should have dimensions");
    assert!(width > 0.0, "Window width should be positive");
    assert!(height > 0.0, "Window height should be positive");
    assert!(width.is_finite(), "Window width should be finite");
    assert!(height.is_finite(), "Window height should be finite");
}

#[wasm_bindgen_test]
fn test_resize_canvas_preserves_aspect_ratio() {
    let canvas = create_test_canvas().expect("Failed to create canvas");

    // Set initial 16:9 aspect ratio
    resize_canvas(&canvas, 1920, 1080).expect("Initial resize failed");

    // Verify dimensions
    let ratio = canvas.width() as f32 / canvas.height() as f32;
    let expected_ratio = 16.0 / 9.0;
    assert!((ratio - expected_ratio).abs() < 0.01, "Should maintain 16:9 aspect ratio");
}

#[wasm_bindgen_test]
fn test_calculate_and_resize_workflow() {
    let canvas = create_test_canvas().expect("Failed to create canvas");

    let config = ResizeConfig {
        maintain_aspect_ratio: true,
        max_width: Some(1600.0),
        max_height: Some(900.0),
    };

    // Simulate window size
    let (new_width, new_height) = calculate_canvas_size(1920.0, 1080.0, &config)
        .expect("Calculate should succeed");

    // Apply calculated dimensions
    resize_canvas(&canvas, new_width, new_height)
        .expect("Resize should succeed");

    // Verify constraints applied
    assert!(canvas.width() <= 1600, "Width should respect max constraint");
    assert!(canvas.height() <= 900, "Height should respect max constraint");
}

#[wasm_bindgen_test]
fn test_rapid_resize_sequence() {
    let canvas = create_test_canvas().expect("Failed to create canvas");

    // Simulate rapid resize events (like user dragging window)
    let sizes = [
        (1920, 1080),
        (1900, 1070),
        (1880, 1060),
        (1860, 1050),
        (1840, 1040),
        (1820, 1030),
        (1800, 1020),
    ];

    for (w, h) in &sizes {
        let result = resize_canvas(&canvas, *w, *h);
        assert!(result.is_ok(), "Rapid resize should not fail at {}x{}", w, h);
        assert_eq!(canvas.width(), *w);
        assert_eq!(canvas.height(), *h);
    }
}

#[wasm_bindgen_test]
fn test_resize_zero_dimensions() {
    let canvas = create_test_canvas().expect("Failed to create canvas");

    // Browsers may handle zero dimensions differently
    // This test documents the behavior
    resize_canvas(&canvas, 0, 0).ok();
    // No assertion - just ensure no panic
}
