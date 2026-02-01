//! Tests for canvas element creation and DOM mounting
//!
//! Note: These tests require a WASM environment with DOM access.
//! Run with: wasm-pack test --headless --firefox

#![cfg(target_arch = "wasm32")]

use super::*;
use wasm_bindgen_test::*;

wasm_bindgen_test_configure!(run_in_browser);

#[wasm_bindgen_test]
fn test_canvas_creation_succeeds() {
    let config = CanvasConfig {
        width: 1200,
        height: 800,
        id: "test-canvas-1".to_string(),
    };

    let result = create_canvas(&config);
    assert!(result.is_ok(), "Canvas creation should succeed");

    let canvas = result.unwrap();
    assert_eq!(canvas.width(), 1200);
    assert_eq!(canvas.height(), 800);
    assert_eq!(canvas.id(), "test-canvas-1");
}

#[wasm_bindgen_test]
fn test_canvas_appends_to_body() {
    let config = CanvasConfig {
        width: 800,
        height: 600,
        id: "test-canvas-2".to_string(),
    };

    let result = create_canvas(&config);
    assert!(result.is_ok(), "Canvas should append to body");

    let canvas = result.unwrap();

    // Verify canvas is in the document
    let window = web_sys::window().expect("should have window");
    let document = window.document().expect("should have document");
    let found = document.get_element_by_id("test-canvas-2");

    assert!(found.is_some(), "Canvas should be in document");
}

#[wasm_bindgen_test]
fn test_canvas_zero_dimensions() {
    let config = CanvasConfig {
        width: 0,
        height: 0,
        id: "test-canvas-3".to_string(),
    };

    let result = create_canvas(&config);
    assert!(
        result.is_ok(),
        "Canvas with zero dimensions should still create"
    );

    let canvas = result.unwrap();
    assert_eq!(canvas.width(), 0);
    assert_eq!(canvas.height(), 0);
}

#[wasm_bindgen_test]
fn test_canvas_large_dimensions() {
    let config = CanvasConfig {
        width: 4096,
        height: 4096,
        id: "test-canvas-4".to_string(),
    };

    let result = create_canvas(&config);
    assert!(result.is_ok(), "Canvas with large dimensions should create");

    let canvas = result.unwrap();
    assert_eq!(canvas.width(), 4096);
    assert_eq!(canvas.height(), 4096);
}

#[wasm_bindgen_test]
fn test_canvas_duplicate_id() {
    let config1 = CanvasConfig {
        width: 100,
        height: 100,
        id: "duplicate-id".to_string(),
    };

    let config2 = CanvasConfig {
        width: 200,
        height: 200,
        id: "duplicate-id".to_string(),
    };

    let result1 = create_canvas(&config1);
    assert!(result1.is_ok(), "First canvas should create");

    let result2 = create_canvas(&config2);
    assert!(
        result2.is_ok(),
        "Second canvas with duplicate ID should still create"
    );

    // Both should exist, though DOM will have ID collision
    let canvas2 = result2.unwrap();
    assert_eq!(canvas2.id(), "duplicate-id");
}

#[wasm_bindgen_test]
fn test_canvas_accessibility_attributes() {
    let config = CanvasConfig {
        width: 800,
        height: 600,
        id: "test-canvas-a11y".to_string(),
    };

    let result = create_canvas(&config);
    assert!(result.is_ok(), "Canvas creation should succeed");

    let canvas = result.unwrap();

    // Verify accessibility attributes
    let role = canvas.get_attribute("role");
    assert_eq!(
        role,
        Some("img".to_string()),
        "Canvas should have role='img'"
    );

    let aria_label = canvas.get_attribute("aria-label");
    assert!(
        aria_label.is_some(),
        "Canvas should have aria-label attribute"
    );
}

#[wasm_bindgen_test]
fn test_canvas_css_class() {
    let config = CanvasConfig {
        width: 800,
        height: 600,
        id: "test-canvas-class".to_string(),
    };

    let result = create_canvas(&config);
    assert!(result.is_ok(), "Canvas creation should succeed");

    let canvas = result.unwrap();
    assert_eq!(
        canvas.class_name(),
        "dag-canvas",
        "Canvas should have 'dag-canvas' class"
    );
}

#[wasm_bindgen_test]
fn test_canvas_config_default() {
    let config = CanvasConfig::default();

    assert_eq!(config.width, 1200, "Default width should be 1200");
    assert_eq!(config.height, 800, "Default height should be 800");
    assert_eq!(config.id, "dag-canvas", "Default ID should be 'dag-canvas'");
}
