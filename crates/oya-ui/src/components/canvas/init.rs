//! Canvas element creation and DOM mounting
//!
//! Provides functional, panic-free canvas initialization using web-sys.
//! All DOM operations return Results for proper error handling.

use wasm_bindgen::JsCast;
use web_sys::HtmlCanvasElement;

/// Configuration for canvas element creation
#[derive(Debug, Clone)]
pub struct CanvasConfig {
    /// Canvas width in logical pixels
    pub width: u32,
    /// Canvas height in logical pixels
    pub height: u32,
    /// DOM element ID
    pub id: String,
}

impl Default for CanvasConfig {
    fn default() -> Self {
        Self {
            width: 1200,
            height: 800,
            id: "dag-canvas".to_string(),
        }
    }
}

/// Creates an HTML5 Canvas element and appends it to the document body
///
/// # Errors
///
/// Returns an error if:
/// - No window object is available (not in browser context)
/// - No document object is available
/// - Canvas element creation fails
/// - Type casting to HtmlCanvasElement fails
/// - No body element exists
/// - Appending to body fails
///
/// # Example
///
/// ```no_run
/// use oya_ui::components::canvas::init::{CanvasConfig, create_canvas};
///
/// let config = CanvasConfig {
///     width: 1200,
///     height: 800,
///     id: "my-canvas".to_string(),
/// };
///
/// let canvas = create_canvas(&config)?;
/// # Ok::<(), String>(())
/// ```
pub fn create_canvas(config: &CanvasConfig) -> Result<HtmlCanvasElement, String> {
    // Get window and document with error handling
    let window = web_sys::window().ok_or("No window object available")?;

    let document = window
        .document()
        .ok_or("No document object available")?;

    // Create canvas element
    let canvas = document
        .create_element("canvas")
        .map_err(|e| format!("Failed to create canvas: {:?}", e))?
        .dyn_into::<HtmlCanvasElement>()
        .map_err(|_| "Failed to cast to HtmlCanvasElement")?;

    // Set dimensions and ID
    canvas.set_width(config.width);
    canvas.set_height(config.height);
    canvas.set_id(&config.id);

    // Set accessibility attributes
    canvas
        .set_attribute("role", "img")
        .map_err(|e| format!("Failed to set role attribute: {:?}", e))?;

    canvas
        .set_attribute("aria-label", "DAG visualization canvas")
        .map_err(|e| format!("Failed to set aria-label attribute: {:?}", e))?;

    // Set CSS class
    canvas.set_class_name("dag-canvas");

    // Append to body
    let body = document.body().ok_or("No body element")?;

    body.append_child(&canvas)
        .map_err(|e| format!("Failed to append canvas: {:?}", e))?;

    Ok(canvas)
}

#[cfg(all(test, target_arch = "wasm32"))]
mod init_test;
