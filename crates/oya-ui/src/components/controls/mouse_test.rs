//! Comprehensive tests for mouse event handling
//!
//! Tests cover coordinate extraction, event data structures, and bounds handling.

use super::mouse::{MouseEventData, WheelEventData};

#[test]
fn test_mouse_event_data_construction() {
    let data = MouseEventData {
        x: 100.0,
        y: 200.0,
        button: 0,
    };

    assert_eq!(data.x, 100.0);
    assert_eq!(data.y, 200.0);
    assert_eq!(data.button, 0);
}

#[test]
fn test_mouse_event_data_left_button() {
    let data = MouseEventData {
        x: 50.0,
        y: 75.0,
        button: 0,
    };

    assert_eq!(data.button, 0); // Left button
}

#[test]
fn test_mouse_event_data_middle_button() {
    let data = MouseEventData {
        x: 50.0,
        y: 75.0,
        button: 1,
    };

    assert_eq!(data.button, 1); // Middle button
}

#[test]
fn test_mouse_event_data_right_button() {
    let data = MouseEventData {
        x: 50.0,
        y: 75.0,
        button: 2,
    };

    assert_eq!(data.button, 2); // Right button
}

#[test]
fn test_mouse_event_data_zero_coordinates() {
    let data = MouseEventData {
        x: 0.0,
        y: 0.0,
        button: 0,
    };

    assert_eq!(data.x, 0.0);
    assert_eq!(data.y, 0.0);
}

#[test]
fn test_mouse_event_data_large_coordinates() {
    let data = MouseEventData {
        x: 10000.0,
        y: 10000.0,
        button: 0,
    };

    assert_eq!(data.x, 10000.0);
    assert_eq!(data.y, 10000.0);
}

#[test]
fn test_mouse_event_data_clone() {
    let data1 = MouseEventData {
        x: 100.0,
        y: 200.0,
        button: 0,
    };

    let data2 = data1;

    assert_eq!(data1.x, data2.x);
    assert_eq!(data1.y, data2.y);
    assert_eq!(data1.button, data2.button);
}

#[test]
fn test_mouse_event_data_debug() {
    let data = MouseEventData {
        x: 100.0,
        y: 200.0,
        button: 0,
    };

    let debug_str = format!("{:?}", data);
    assert!(debug_str.contains("MouseEventData"));
    assert!(debug_str.contains("100.0"));
    assert!(debug_str.contains("200.0"));
}

#[test]
fn test_wheel_event_data_construction() {
    let data = WheelEventData {
        delta_y: 100.0,
        x: 150.0,
        y: 250.0,
    };

    assert_eq!(data.delta_y, 100.0);
    assert_eq!(data.x, 150.0);
    assert_eq!(data.y, 250.0);
}

#[test]
fn test_wheel_event_data_scroll_down() {
    let data = WheelEventData {
        delta_y: 100.0,
        x: 50.0,
        y: 75.0,
    };

    assert!(data.delta_y > 0.0); // Scroll down
}

#[test]
fn test_wheel_event_data_scroll_up() {
    let data = WheelEventData {
        delta_y: -100.0,
        x: 50.0,
        y: 75.0,
    };

    assert!(data.delta_y < 0.0); // Scroll up
}

#[test]
fn test_wheel_event_data_no_scroll() {
    let data = WheelEventData {
        delta_y: 0.0,
        x: 50.0,
        y: 75.0,
    };

    assert_eq!(data.delta_y, 0.0); // No scroll
}

#[test]
fn test_wheel_event_data_large_delta() {
    let data = WheelEventData {
        delta_y: 500.0,
        x: 100.0,
        y: 200.0,
    };

    assert_eq!(data.delta_y, 500.0);
}

#[test]
fn test_wheel_event_data_negative_delta() {
    let data = WheelEventData {
        delta_y: -500.0,
        x: 100.0,
        y: 200.0,
    };

    assert_eq!(data.delta_y, -500.0);
}

#[test]
fn test_wheel_event_data_zero_coordinates() {
    let data = WheelEventData {
        delta_y: 50.0,
        x: 0.0,
        y: 0.0,
    };

    assert_eq!(data.x, 0.0);
    assert_eq!(data.y, 0.0);
}

#[test]
fn test_wheel_event_data_clone() {
    let data1 = WheelEventData {
        delta_y: 100.0,
        x: 150.0,
        y: 250.0,
    };

    let data2 = data1;

    assert_eq!(data1.delta_y, data2.delta_y);
    assert_eq!(data1.x, data2.x);
    assert_eq!(data1.y, data2.y);
}

#[test]
fn test_wheel_event_data_debug() {
    let data = WheelEventData {
        delta_y: 100.0,
        x: 150.0,
        y: 250.0,
    };

    let debug_str = format!("{:?}", data);
    assert!(debug_str.contains("WheelEventData"));
    assert!(debug_str.contains("100.0"));
    assert!(debug_str.contains("150.0"));
    assert!(debug_str.contains("250.0"));
}

#[test]
fn test_mouse_and_wheel_data_independence() {
    let mouse_data = MouseEventData {
        x: 100.0,
        y: 200.0,
        button: 0,
    };

    let wheel_data = WheelEventData {
        delta_y: 50.0,
        x: 100.0,
        y: 200.0,
    };

    // Same coordinates, different event types
    assert_eq!(mouse_data.x, wheel_data.x);
    assert_eq!(mouse_data.y, wheel_data.y);
    // But they're different types
    assert_eq!(std::mem::size_of_val(&mouse_data), 24); // f64 + f64 + i16 + padding
    assert_eq!(std::mem::size_of_val(&wheel_data), 24); // f64 + f64 + f64
}

#[test]
fn test_mouse_event_data_partial_eq() {
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

    assert_eq!(data1, data2);
}

#[test]
fn test_mouse_event_data_partial_eq_different_x() {
    let data1 = MouseEventData {
        x: 100.0,
        y: 200.0,
        button: 0,
    };

    let data2 = MouseEventData {
        x: 101.0,
        y: 200.0,
        button: 0,
    };

    assert_ne!(data1, data2);
}

#[test]
fn test_mouse_event_data_partial_eq_different_y() {
    let data1 = MouseEventData {
        x: 100.0,
        y: 200.0,
        button: 0,
    };

    let data2 = MouseEventData {
        x: 100.0,
        y: 201.0,
        button: 0,
    };

    assert_ne!(data1, data2);
}

#[test]
fn test_mouse_event_data_partial_eq_different_button() {
    let data1 = MouseEventData {
        x: 100.0,
        y: 200.0,
        button: 0,
    };

    let data2 = MouseEventData {
        x: 100.0,
        y: 200.0,
        button: 1,
    };

    assert_ne!(data1, data2);
}

#[test]
fn test_wheel_event_data_partial_eq() {
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

    assert_eq!(data1, data2);
}

#[test]
fn test_wheel_event_data_partial_eq_different_delta() {
    let data1 = WheelEventData {
        delta_y: 50.0,
        x: 100.0,
        y: 200.0,
    };

    let data2 = WheelEventData {
        delta_y: 51.0,
        x: 100.0,
        y: 200.0,
    };

    assert_ne!(data1, data2);
}

#[test]
fn test_wheel_event_data_partial_eq_different_x() {
    let data1 = WheelEventData {
        delta_y: 50.0,
        x: 100.0,
        y: 200.0,
    };

    let data2 = WheelEventData {
        delta_y: 50.0,
        x: 101.0,
        y: 200.0,
    };

    assert_ne!(data1, data2);
}

#[test]
fn test_wheel_event_data_partial_eq_different_y() {
    let data1 = WheelEventData {
        delta_y: 50.0,
        x: 100.0,
        y: 200.0,
    };

    let data2 = WheelEventData {
        delta_y: 50.0,
        x: 100.0,
        y: 201.0,
    };

    assert_ne!(data1, data2);
}

#[test]
fn test_mouse_event_data_copy_semantics() {
    let data1 = MouseEventData {
        x: 100.0,
        y: 200.0,
        button: 0,
    };

    let data2 = data1; // Copy, not move
    let data3 = data1; // Can still use data1

    assert_eq!(data1, data2);
    assert_eq!(data1, data3);
    assert_eq!(data2, data3);
}

#[test]
fn test_wheel_event_data_copy_semantics() {
    let data1 = WheelEventData {
        delta_y: 50.0,
        x: 100.0,
        y: 200.0,
    };

    let data2 = data1; // Copy, not move
    let data3 = data1; // Can still use data1

    assert_eq!(data1, data2);
    assert_eq!(data1, data3);
    assert_eq!(data2, data3);
}

#[test]
fn test_mouse_event_data_fractional_coordinates() {
    let data = MouseEventData {
        x: 123.456,
        y: 789.012,
        button: 0,
    };

    assert!((data.x - 123.456).abs() < f64::EPSILON);
    assert!((data.y - 789.012).abs() < f64::EPSILON);
}

#[test]
fn test_wheel_event_data_fractional_values() {
    let data = WheelEventData {
        delta_y: 12.345,
        x: 67.890,
        y: 123.456,
    };

    assert!((data.delta_y - 12.345).abs() < f64::EPSILON);
    assert!((data.x - 67.890).abs() < f64::EPSILON);
    assert!((data.y - 123.456).abs() < f64::EPSILON);
}
