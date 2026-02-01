//! Mouse event listeners and data extraction
//!
//! Provides functional, panic-free mouse event handling for canvas interactions.
//! All DOM operations return Results for proper error handling.

use wasm_bindgen::JsCast;
use web_sys::{HtmlCanvasElement, MouseEvent, WheelEvent};

/// Mouse event data extracted from DOM events
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct MouseEventData {
    /// X coordinate relative to canvas
    pub x: f32,
    /// Y coordinate relative to canvas
    pub y: f32,
    /// Mouse button pressed (0 = left, 1 = middle, 2 = right)
    pub button: u16,
}

/// Wheel event data
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct WheelEventData {
    /// Vertical scroll delta
    pub delta_y: f32,
    /// X coordinate relative to canvas
    pub x: f32,
    /// Y coordinate relative to canvas
    pub y: f32,
}

/// Extract mouse coordinates from MouseEvent relative to canvas
///
/// # Errors
///
/// Returns an error if:
/// - Canvas bounding rect cannot be obtained
/// - Coordinate calculations fail
///
/// # Example
///
/// ```no_run
/// use oya_ui::components::controls::mouse::get_mouse_coords;
/// use web_sys::{HtmlCanvasElement, MouseEvent};
///
/// fn handle_mouse(event: &MouseEvent, canvas: &HtmlCanvasElement) -> Result<(), String> {
///     let (x, y) = get_mouse_coords(event, canvas)?;
///     // Use coordinates...
///     Ok(())
/// }
/// ```
pub fn get_mouse_coords(
    event: &MouseEvent,
    canvas: &HtmlCanvasElement,
) -> Result<(f32, f32), String> {
    let rect = canvas.get_bounding_client_rect();

    let x = (event.client_x() as f32 - rect.left() as f32).max(0.0);
    let y = (event.client_y() as f32 - rect.top() as f32).max(0.0);

    Ok((x, y))
}

/// Extract complete mouse event data
///
/// # Errors
///
/// Returns an error if coordinate extraction fails
///
/// # Example
///
/// ```no_run
/// use oya_ui::components::controls::mouse::extract_mouse_data;
/// use web_sys::{HtmlCanvasElement, MouseEvent};
///
/// fn on_mousedown(event: &MouseEvent, canvas: &HtmlCanvasElement) -> Result<(), String> {
///     let data = extract_mouse_data(event, canvas)?;
///     println!("Mouse down at ({}, {}) button {}", data.x, data.y, data.button);
///     Ok(())
/// }
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

/// Extract wheel event data including mouse position
///
/// # Errors
///
/// Returns an error if coordinate extraction fails
///
/// # Example
///
/// ```no_run
/// use oya_ui::components::controls::mouse::extract_wheel_data;
/// use web_sys::{HtmlCanvasElement, WheelEvent};
///
/// fn on_wheel(event: &WheelEvent, canvas: &HtmlCanvasElement) -> Result<(), String> {
///     let data = extract_wheel_data(event, canvas)?;
///     println!("Wheel delta {} at ({}, {})", data.delta_y, data.x, data.y);
///     Ok(())
/// }
/// ```
pub fn extract_wheel_data(
    event: &WheelEvent,
    canvas: &HtmlCanvasElement,
) -> Result<WheelEventData, String> {
    // WheelEvent inherits from MouseEvent, so we can upcast
    let mouse_event: &MouseEvent = event.unchecked_ref();
    let (x, y) = get_mouse_coords(mouse_event, canvas)?;

    Ok(WheelEventData {
        delta_y: event.delta_y() as f32,
        x,
        y,
    })
}

#[cfg(all(test, target_arch = "wasm32"))]
mod mouse_test;
