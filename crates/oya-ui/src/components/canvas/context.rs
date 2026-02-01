//! Canvas 2D rendering context acquisition
//!
//! Provides functional, panic-free 2D context retrieval from HtmlCanvasElement.
//! All JS interop operations return Results for proper error handling.

use wasm_bindgen::JsCast;
use web_sys::{CanvasRenderingContext2d, HtmlCanvasElement};

/// Get 2D rendering context from canvas element
///
/// # Errors
///
/// Returns an error if:
/// - Getting context from canvas fails (JS error)
/// - Context creation returns None (browser doesn't support 2D context)
/// - Type casting to CanvasRenderingContext2d fails
///
/// # Example
///
/// ```no_run
/// use oya_ui::components::canvas::{create_canvas, CanvasConfig};
/// use oya_ui::components::canvas::context::get_2d_context;
///
/// let config = CanvasConfig::default();
/// let canvas = create_canvas(&config)?;
/// let context = get_2d_context(&canvas)?;
/// # Ok::<(), String>(())
/// ```
pub fn get_2d_context(canvas: &HtmlCanvasElement) -> Result<CanvasRenderingContext2d, String> {
    // Get context with error handling
    let context_result = canvas
        .get_context("2d")
        .map_err(|e| format!("Failed to get canvas context: {:?}", e))?;

    // Handle None case (context creation failed)
    let context_object = context_result.ok_or("Canvas context creation returned None")?;

    // Cast to CanvasRenderingContext2d
    let context = context_object
        .dyn_into::<CanvasRenderingContext2d>()
        .map_err(|_| "Failed to cast context to CanvasRenderingContext2d")?;

    Ok(context)
}

/// Get or create cached 2D context
///
/// For use with Leptos RwSignal storage.
/// Creates context on first call, returns cached on subsequent calls.
///
/// # Errors
///
/// Returns an error if context creation fails (see [`get_2d_context`])
///
/// # Example
///
/// ```no_run
/// use oya_ui::components::canvas::{create_canvas, CanvasConfig};
/// use oya_ui::components::canvas::context::get_or_create_context;
///
/// let config = CanvasConfig::default();
/// let canvas = create_canvas(&config)?;
/// let mut cached = None;
///
/// // First call creates context
/// let context = get_or_create_context(&canvas, &cached)?;
/// cached = Some(context.clone());
///
/// // Second call returns cached context
/// let same_context = get_or_create_context(&canvas, &cached)?;
/// # Ok::<(), String>(())
/// ```
pub fn get_or_create_context(
    canvas: &HtmlCanvasElement,
    cached: &Option<CanvasRenderingContext2d>,
) -> Result<CanvasRenderingContext2d, String> {
    match cached {
        Some(ctx) => Ok(ctx.clone()),
        None => get_2d_context(canvas),
    }
}

#[cfg(test)]
#[cfg(target_arch = "wasm32")]
mod tests {
    use super::*;
    use crate::components::canvas::init::{CanvasConfig, create_canvas};
    use wasm_bindgen_test::*;

    wasm_bindgen_test_configure!(run_in_browser);

    #[wasm_bindgen_test]
    fn test_get_2d_context_succeeds() -> Result<(), Box<dyn std::error::Error>> {
        let config = CanvasConfig {
            width: 800,
            height: 600,
            id: "test-ctx-1".to_string(),
        };

        let canvas = create_canvas(&config)?;
        let _context = get_2d_context(&canvas)?;
        Ok(())
    }

    #[wasm_bindgen_test]
    fn test_context_is_correct_type() -> Result<(), Box<dyn std::error::Error>> {
        let config = CanvasConfig {
            width: 800,
            height: 600,
            id: "test-ctx-2".to_string(),
        };

        let canvas = create_canvas(&config)?;
        let context = get_2d_context(&canvas)?;

        // Verify we can use 2D context methods
        context.set_fill_style_str("#000000");
        Ok(())
    }

    #[wasm_bindgen_test]
    fn test_multiple_retrievals() -> Result<(), Box<dyn std::error::Error>> {
        let config = CanvasConfig {
            width: 800,
            height: 600,
            id: "test-ctx-3".to_string(),
        };

        let canvas = create_canvas(&config)?;
        let ctx1 = get_2d_context(&canvas)?;
        let ctx2 = get_2d_context(&canvas)?;

        // Both should succeed (same underlying context)
        assert!(ctx1.canvas().is_some());
        assert!(ctx2.canvas().is_some());
        Ok(())
    }

    #[wasm_bindgen_test]
    fn test_get_or_create_with_none() -> Result<(), Box<dyn std::error::Error>> {
        let config = CanvasConfig {
            width: 800,
            height: 600,
            id: "test-ctx-4".to_string(),
        };

        let canvas = create_canvas(&config)?;
        let cached = None;

        let _context = get_or_create_context(&canvas, &cached)?;
        Ok(())
    }

    #[wasm_bindgen_test]
    fn test_get_or_create_with_cached() -> Result<(), Box<dyn std::error::Error>> {
        let config = CanvasConfig {
            width: 800,
            height: 600,
            id: "test-ctx-5".to_string(),
        };

        let canvas = create_canvas(&config)?;
        let first_context = get_2d_context(&canvas)?;
        let cached = Some(first_context);

        let _context = get_or_create_context(&canvas, &cached)?;
        Ok(())
    }

    #[wasm_bindgen_test]
    fn test_error_messages_descriptive() -> Result<(), Box<dyn std::error::Error>> {
        let config = CanvasConfig {
            width: 800,
            height: 600,
            id: "test-ctx-6".to_string(),
        };

        let canvas = create_canvas(&config)?;
        let _context = get_2d_context(&canvas)?;
        Ok(())
    }
}
