//! WASM browser tests for mouse event handling
//!
//! Run with: wasm-pack test --headless --firefox

use super::*;
use crate::components::canvas::init::{create_canvas, CanvasConfig};
use wasm_bindgen::JsCast;
use wasm_bindgen_test::*;
use web_sys::{Event, MouseEvent, WheelEvent};

wasm_bindgen_test_configure!(run_in_browser);

/// Helper to create a test canvas
fn setup_test_canvas(id: &str) -> Result<HtmlCanvasElement, String> {
    let config = CanvasConfig {
        width: 800,
        height: 600,
        id: id.to_string(),
    };
    create_canvas(&config)
}

/// Helper to create a mock MouseEvent
fn create_mouse_event(
    event_type: &str,
    client_x: i32,
    client_y: i32,
    button: i16,
) -> Result<MouseEvent, String> {
    let window = web_sys::window().ok_or("No window")?;

    let event_init = web_sys::MouseEventInit::new();
    event_init.set_client_x(client_x);
    event_init.set_client_y(client_y);
    event_init.set_button(button);
    event_init.set_bubbles(true);
    event_init.set_cancelable(true);

    MouseEvent::new_with_mouse_event_init_dict(event_type, &event_init)
        .map_err(|e| format!("Failed to create MouseEvent: {:?}", e))
}

/// Helper to create a mock WheelEvent
fn create_wheel_event(
    client_x: i32,
    client_y: i32,
    delta_y: f64,
) -> Result<WheelEvent, String> {
    let event_init = web_sys::WheelEventInit::new();
    event_init.set_client_x(client_x);
    event_init.set_client_y(client_y);
    event_init.set_delta_y(delta_y);
    event_init.set_bubbles(true);
    event_init.set_cancelable(true);

    WheelEvent::new_with_wheel_event_init_dict("wheel", &event_init)
        .map_err(|e| format!("Failed to create WheelEvent: {:?}", e))
}

#[wasm_bindgen_test]
fn test_mouse_coords_extraction() {
    let canvas = setup_test_canvas("test-mouse-coords").expect("Canvas setup failed");

    // Create event at (100, 150)
    let event = create_mouse_event("mousedown", 100, 150, 0).expect("Event creation failed");

    let result = get_mouse_coords(&event, &canvas);
    assert!(
        result.is_ok(),
        "get_mouse_coords should succeed: {:?}",
        result.err()
    );

    let (x, y) = result.unwrap();

    // Coordinates should be relative to canvas position
    // Since canvas is at body origin in test, coords should match client coords
    assert!(x >= 0.0, "X coordinate should be non-negative");
    assert!(y >= 0.0, "Y coordinate should be non-negative");
}

#[wasm_bindgen_test]
fn test_negative_coordinates_clamped() {
    let canvas = setup_test_canvas("test-mouse-negative").expect("Canvas setup failed");

    // Create event with negative client coordinates (outside canvas)
    let event = create_mouse_event("mousedown", -10, -20, 0).expect("Event creation failed");

    let result = get_mouse_coords(&event, &canvas);
    assert!(result.is_ok(), "Should handle negative coordinates");

    let (x, y) = result.unwrap();

    // Negative coordinates should be clamped to 0
    assert_eq!(x, 0.0, "Negative X should be clamped to 0");
    assert_eq!(y, 0.0, "Negative Y should be clamped to 0");
}

#[wasm_bindgen_test]
fn test_extract_mouse_data_left_button() {
    let canvas = setup_test_canvas("test-mouse-left").expect("Canvas setup failed");

    // Left button = 0
    let event = create_mouse_event("mousedown", 250, 300, 0).expect("Event creation failed");

    let result = extract_mouse_data(&event, &canvas);
    assert!(
        result.is_ok(),
        "extract_mouse_data should succeed: {:?}",
        result.err()
    );

    let data = result.unwrap();

    assert!(data.x >= 0.0, "X should be non-negative");
    assert!(data.y >= 0.0, "Y should be non-negative");
    assert_eq!(data.button, 0, "Should detect left button");
}

#[wasm_bindgen_test]
fn test_extract_mouse_data_middle_button() {
    let canvas = setup_test_canvas("test-mouse-middle").expect("Canvas setup failed");

    // Middle button = 1
    let event = create_mouse_event("mousedown", 250, 300, 1).expect("Event creation failed");

    let result = extract_mouse_data(&event, &canvas);
    assert!(result.is_ok(), "Should handle middle button");

    let data = result.unwrap();
    assert_eq!(data.button, 1, "Should detect middle button");
}

#[wasm_bindgen_test]
fn test_extract_mouse_data_right_button() {
    let canvas = setup_test_canvas("test-mouse-right").expect("Canvas setup failed");

    // Right button = 2
    let event = create_mouse_event("contextmenu", 250, 300, 2).expect("Event creation failed");

    let result = extract_mouse_data(&event, &canvas);
    assert!(result.is_ok(), "Should handle right button");

    let data = result.unwrap();
    assert_eq!(data.button, 2, "Should detect right button");
}

#[wasm_bindgen_test]
fn test_extract_wheel_data_scroll_down() {
    let canvas = setup_test_canvas("test-wheel-down").expect("Canvas setup failed");

    // Scroll down = positive delta
    let event = create_wheel_event(400, 300, 100.0).expect("Event creation failed");

    let result = extract_wheel_data(&event, &canvas);
    assert!(
        result.is_ok(),
        "extract_wheel_data should succeed: {:?}",
        result.err()
    );

    let data = result.unwrap();

    assert!(data.x >= 0.0, "X should be non-negative");
    assert!(data.y >= 0.0, "Y should be non-negative");
    assert_eq!(data.delta_y, 100.0, "Should capture scroll down delta");
}

#[wasm_bindgen_test]
fn test_extract_wheel_data_scroll_up() {
    let canvas = setup_test_canvas("test-wheel-up").expect("Canvas setup failed");

    // Scroll up = negative delta
    let event = create_wheel_event(400, 300, -100.0).expect("Event creation failed");

    let result = extract_wheel_data(&event, &canvas);
    assert!(result.is_ok(), "Should handle negative delta");

    let data = result.unwrap();
    assert_eq!(data.delta_y, -100.0, "Should capture scroll up delta");
}

#[wasm_bindgen_test]
fn test_mouse_data_struct_equality() {
    let data1 = MouseEventData {
        x: 100.0,
        y: 200.0,
        button: 0,
    };

    let data2 = MouseEventData {
        x: 100.0,
        y: 200.0,
        button: 0,
    };

    let data3 = MouseEventData {
        x: 100.0,
        y: 201.0,
        button: 0,
    };

    assert_eq!(data1, data2, "Identical data should be equal");
    assert_ne!(data1, data3, "Different data should not be equal");
}

#[wasm_bindgen_test]
fn test_wheel_data_struct_equality() {
    let data1 = WheelEventData {
        delta_y: 50.0,
        x: 100.0,
        y: 200.0,
    };

    let data2 = WheelEventData {
        delta_y: 50.0,
        x: 100.0,
        y: 200.0,
    };

    let data3 = WheelEventData {
        delta_y: 51.0,
        x: 100.0,
        y: 200.0,
    };

    assert_eq!(data1, data2, "Identical data should be equal");
    assert_ne!(data1, data3, "Different data should not be equal");
}

#[wasm_bindgen_test]
fn test_rapid_mouse_movements() {
    let canvas = setup_test_canvas("test-mouse-rapid").expect("Canvas setup failed");

    // Simulate rapid movements
    let positions = vec![(10, 20), (50, 60), (100, 150), (200, 300), (400, 500)];

    for (x, y) in positions {
        let event = create_mouse_event("mousemove", x, y, 0).expect("Event creation failed");

        let result = extract_mouse_data(&event, &canvas);
        assert!(
            result.is_ok(),
            "Should handle rapid movement at ({}, {})",
            x,
            y
        );
    }
}

#[wasm_bindgen_test]
fn test_edge_coordinates() {
    let canvas = setup_test_canvas("test-mouse-edge").expect("Canvas setup failed");

    // Test coordinates at canvas edges (0, 0) and (800, 600)
    let edge_positions = vec![(0, 0), (800, 0), (0, 600), (800, 600)];

    for (x, y) in edge_positions {
        let event = create_mouse_event("mousedown", x, y, 0).expect("Event creation failed");

        let result = extract_mouse_data(&event, &canvas);
        assert!(
            result.is_ok(),
            "Should handle edge coordinate ({}, {})",
            x,
            y
        );

        let data = result.unwrap();
        assert!(data.x >= 0.0, "Edge X should be non-negative");
        assert!(data.y >= 0.0, "Edge Y should be non-negative");
    }
}

#[wasm_bindgen_test]
fn test_multiple_event_types() {
    let canvas = setup_test_canvas("test-mouse-types").expect("Canvas setup failed");

    // Test different event types with same coordinates
    let event_types = vec!["mousedown", "mousemove", "mouseup"];

    for event_type in event_types {
        let event = create_mouse_event(event_type, 100, 100, 0).expect("Event creation failed");

        let result = extract_mouse_data(&event, &canvas);
        assert!(
            result.is_ok(),
            "Should handle event type: {}",
            event_type
        );
    }
}

#[wasm_bindgen_test]
fn test_wheel_event_at_canvas_edge() {
    let canvas = setup_test_canvas("test-wheel-edge").expect("Canvas setup failed");

    // Wheel event at top-left corner
    let event = create_wheel_event(0, 0, 50.0).expect("Event creation failed");

    let result = extract_wheel_data(&event, &canvas);
    assert!(result.is_ok(), "Should handle wheel at edge");

    let data = result.unwrap();
    assert_eq!(data.x, 0.0, "Edge X should be 0");
    assert_eq!(data.y, 0.0, "Edge Y should be 0");
}

#[wasm_bindgen_test]
fn test_zero_delta_wheel_event() {
    let canvas = setup_test_canvas("test-wheel-zero").expect("Canvas setup failed");

    // Zero delta (no scroll)
    let event = create_wheel_event(100, 100, 0.0).expect("Event creation failed");

    let result = extract_wheel_data(&event, &canvas);
    assert!(result.is_ok(), "Should handle zero delta");

    let data = result.unwrap();
    assert_eq!(data.delta_y, 0.0, "Should preserve zero delta");
}
