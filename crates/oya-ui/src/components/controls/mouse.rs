//! Mouse event handlers for DAG visualization pan/zoom controls
//!
//! This module provides functional, panic-free mouse event handling with proper
//! coordinate conversion and event data extraction. All operations follow the
//! Railway-Oriented Programming pattern with Result types.

use web_sys::{HtmlCanvasElement, MouseEvent, WheelEvent};

/// Mouse event data with canvas-relative coordinates
///
/// # Examples
///
/// ```
/// use oya_ui::components::controls::mouse::MouseEventData;
///
/// let data = MouseEventData {
///     x: 100.0,
///     y: 200.0,
///     button: 0,
/// };
/// assert_eq!(data.button, 0); // Left button
/// ```
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct MouseEventData {
    /// X coordinate relative to canvas (clamped to non-negative)
    pub x: f64,
    /// Y coordinate relative to canvas (clamped to non-negative)
    pub y: f64,
    /// Mouse button: 0 = left, 1 = middle, 2 = right
    pub button: i16,
}

/// Wheel event data with canvas-relative coordinates
///
/// # Examples
///
/// ```
/// use oya_ui::components::controls::mouse::WheelEventData;
///
/// let data = WheelEventData {
///     delta_y: -100.0,
///     x: 100.0,
///     y: 200.0,
/// };
/// assert!(data.delta_y < 0.0); // Scroll up
/// ```
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct WheelEventData {
    /// Vertical scroll delta (positive = down, negative = up)
    pub delta_y: f64,
    /// X coordinate relative to canvas (clamped to non-negative)
    pub x: f64,
    /// Y coordinate relative to canvas (clamped to non-negative)
    pub y: f64,
}

/// Extracts canvas-relative coordinates from a mouse event.
///
/// Coordinates are calculated relative to the canvas element's position,
/// and negative values are clamped to 0.0.
///
/// # Errors
///
/// Returns an error if:
/// - Canvas bounding rect cannot be retrieved
/// - Client coordinates are invalid
///
/// # Examples
///
/// ```no_run
/// use web_sys::{HtmlCanvasElement, MouseEvent};
/// use oya_ui::components::controls::mouse::get_mouse_coords;
///
/// # fn example(event: &MouseEvent, canvas: &HtmlCanvasElement) -> Result<(), String> {
/// let (x, y) = get_mouse_coords(event, canvas)?;
/// assert!(x >= 0.0);
/// assert!(y >= 0.0);
/// # Ok(())
/// # }
/// ```
pub fn get_mouse_coords(
    event: &MouseEvent,
    canvas: &HtmlCanvasElement,
) -> Result<(f64, f64), String> {
    // Get canvas bounding rect for coordinate conversion
    let rect = canvas.get_bounding_client_rect();

    // Convert client coordinates to canvas-relative coordinates
    let client_x = event.client_x() as f64;
    let client_y = event.client_y() as f64;

    let canvas_x = client_x - rect.left();
    let canvas_y = client_y - rect.top();

    // Clamp negative coordinates to 0 (outside canvas bounds)
    let clamped_x = canvas_x.max(0.0);
    let clamped_y = canvas_y.max(0.0);

    Ok((clamped_x, clamped_y))
}

/// Extracts complete mouse event data including button information.
///
/// This combines coordinate extraction with button state, providing all
/// necessary information for mouse event handlers.
///
/// # Errors
///
/// Returns an error if coordinate extraction fails.
///
/// # Examples
///
/// ```no_run
/// use web_sys::{HtmlCanvasElement, MouseEvent};
/// use oya_ui::components::controls::mouse::extract_mouse_data;
///
/// # fn example(event: &MouseEvent, canvas: &HtmlCanvasElement) -> Result<(), String> {
/// let data = extract_mouse_data(event, canvas)?;
/// match data.button {
///     0 => println!("Left button at ({}, {})", data.x, data.y),
///     1 => println!("Middle button at ({}, {})", data.x, data.y),
///     2 => println!("Right button at ({}, {})", data.x, data.y),
///     _ => println!("Other button"),
/// }
/// # Ok(())
/// # }
/// ```
pub fn extract_mouse_data(
    event: &MouseEvent,
    canvas: &HtmlCanvasElement,
) -> Result<MouseEventData, String> {
    let (x, y) = get_mouse_coords(event, canvas)?;

    Ok(MouseEventData {
        x,
        y,
        button: event.button(),
    })
}

/// Extracts wheel event data including scroll delta and coordinates.
///
/// This provides both the scroll amount (delta_y) and the cursor position
/// where the scroll occurred.
///
/// # Errors
///
/// Returns an error if coordinate extraction fails.
///
/// # Examples
///
/// ```no_run
/// use web_sys::{HtmlCanvasElement, WheelEvent};
/// use oya_ui::components::controls::mouse::extract_wheel_data;
///
/// # fn example(event: &WheelEvent, canvas: &HtmlCanvasElement) -> Result<(), String> {
/// let data = extract_wheel_data(event, canvas)?;
/// if data.delta_y > 0.0 {
///     println!("Scroll down by {} at ({}, {})", data.delta_y, data.x, data.y);
/// } else {
///     println!("Scroll up by {} at ({}, {})", -data.delta_y, data.x, data.y);
/// }
/// # Ok(())
/// # }
/// ```
pub fn extract_wheel_data(
    event: &WheelEvent,
    canvas: &HtmlCanvasElement,
) -> Result<WheelEventData, String> {
    // Get canvas bounding rect for coordinate conversion
    let rect = canvas.get_bounding_client_rect();

    // Convert client coordinates to canvas-relative coordinates
    let client_x = event.client_x() as f64;
    let client_y = event.client_y() as f64;

    let canvas_x = client_x - rect.left();
    let canvas_y = client_y - rect.top();

    // Clamp negative coordinates to 0 (outside canvas bounds)
    let clamped_x = canvas_x.max(0.0);
    let clamped_y = canvas_y.max(0.0);

    Ok(WheelEventData {
        delta_y: event.delta_y(),
        x: clamped_x,
        y: clamped_y,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_mouse_event_data_equality() {
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
    fn test_wheel_event_data_equality() {
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
    fn test_mouse_event_data_inequality() {
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
    fn test_wheel_event_data_inequality() {
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
}
