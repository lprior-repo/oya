//! Canvas resize handling with responsive layout support
//!
//! Provides functional, panic-free canvas resizing with aspect ratio maintenance,
//! dimension constraints, and DPI scaling preservation.

use gloo_timers::callback::Timeout;
use std::cell::RefCell;
use std::rc::Rc;
use wasm_bindgen::JsCast;
use wasm_bindgen::closure::Closure;
use web_sys::{HtmlCanvasElement, window};

/// Configuration for canvas resize behavior
#[derive(Debug, Clone, Copy, Default)]
pub struct ResizeConfig {
    /// Whether to maintain aspect ratio during resize
    pub maintain_aspect_ratio: bool,
    /// Maximum width constraint in logical pixels
    pub max_width: Option<f32>,
    /// Maximum height constraint in logical pixels
    pub max_height: Option<f32>,
}

/// Calculate new canvas dimensions based on window size and configuration
///
/// # Errors
///
/// Returns an error if:
/// - Window width is not finite or non-positive
/// - Window height is not finite or non-positive
///
/// # Example
///
/// ```
/// use oya_ui::components::canvas::resize::{calculate_canvas_size, ResizeConfig};
///
/// let config = ResizeConfig::default();
/// let (width, height) = calculate_canvas_size(1920.0, 1080.0, &config)?;
/// assert_eq!(width, 1920);
/// assert_eq!(height, 1080);
/// # Ok::<(), String>(())
/// ```
pub fn calculate_canvas_size(
    window_width: f32,
    window_height: f32,
    config: &ResizeConfig,
) -> Result<(u32, u32), String> {
    // Validate window dimensions
    if !window_width.is_finite() || window_width <= 0.0 {
        return Err(format!("Invalid window width: {}", window_width));
    }
    if !window_height.is_finite() || window_height <= 0.0 {
        return Err(format!("Invalid window height: {}", window_height));
    }

    let mut width = window_width;
    let mut height = window_height;

    // Apply maximum width constraint
    if let Some(max_w) = config.max_width {
        width = width.min(max_w);
    }

    // Apply maximum height constraint
    if let Some(max_h) = config.max_height {
        height = height.min(max_h);
    }

    // Maintain aspect ratio if requested (default 3:2 ratio from 1200x800)
    if config.maintain_aspect_ratio {
        let aspect_ratio = 1200.0 / 800.0; // 3:2 ratio
        let current_ratio = width / height;

        if current_ratio > aspect_ratio {
            // Too wide, reduce width
            width = height * aspect_ratio;
        } else {
            // Too tall, reduce height
            height = width / aspect_ratio;
        }
    }

    // Convert to integers, ensuring non-zero dimensions
    let final_width = width.floor().max(1.0) as u32;
    let final_height = height.floor().max(1.0) as u32;

    Ok((final_width, final_height))
}

/// Resize canvas element to new dimensions
///
/// # Errors
///
/// Returns an error if:
/// - Canvas resize operation fails (dimension mismatch after set)
///
/// # Example
///
/// ```no_run
/// use oya_ui::components::canvas::resize::resize_canvas;
/// use web_sys::HtmlCanvasElement;
///
/// # fn example(canvas: &HtmlCanvasElement) -> Result<(), String> {
/// resize_canvas(canvas, 1920, 1080)?;
/// # Ok(())
/// # }
/// ```
pub fn resize_canvas(canvas: &HtmlCanvasElement, width: u32, height: u32) -> Result<(), String> {
    // Set canvas dimensions
    canvas.set_width(width);
    canvas.set_height(height);

    // Verify resize succeeded
    if canvas.width() != width || canvas.height() != height {
        return Err(format!(
            "Canvas resize failed: expected {}x{}, got {}x{}",
            width,
            height,
            canvas.width(),
            canvas.height()
        ));
    }

    Ok(())
}

/// Get current window dimensions
///
/// # Errors
///
/// Returns an error if:
/// - No window object is available (not in browser context)
/// - Failed to retrieve window width
/// - Failed to retrieve window height
/// - Window dimensions are not valid numbers
///
/// # Example
///
/// ```no_run
/// use oya_ui::components::canvas::resize::get_window_size;
///
/// let (width, height) = get_window_size()?;
/// assert!(width > 0.0);
/// assert!(height > 0.0);
/// # Ok::<(), String>(())
/// ```
pub fn get_window_size() -> Result<(f32, f32), String> {
    let window = window().ok_or("No window object available")?;

    let width = window
        .inner_width()
        .map_err(|e| format!("Failed to get window width: {:?}", e))?
        .as_f64()
        .ok_or("Window width is not a number")? as f32;

    let height = window
        .inner_height()
        .map_err(|e| format!("Failed to get window height: {:?}", e))?
        .as_f64()
        .ok_or("Window height is not a number")? as f32;

    Ok((width, height))
}

/// Handle for managing resize event listener cleanup
pub struct ResizeHandler {
    closure: Closure<dyn FnMut()>,
}

impl ResizeHandler {
    /// Remove the resize event listener from the window
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - Window object is not available
    /// - Event listener removal fails
    pub fn remove(self) -> Result<(), String> {
        let window = window().ok_or("No window object available")?;

        window
            .remove_event_listener_with_callback("resize", self.closure.as_ref().unchecked_ref())
            .map_err(|e| format!("Failed to remove resize listener: {:?}", e))?;

        Ok(())
    }
}

/// Attach debounced resize event listener to window
///
/// Listens for window resize events and updates canvas dimensions with debouncing
/// to prevent excessive redraws during rapid resize operations.
///
/// # Errors
///
/// Returns an error if:
/// - Window object is not available
/// - Event listener attachment fails
/// - Initial canvas resize fails
///
/// # Example
///
/// ```no_run
/// use oya_ui::components::canvas::resize::{attach_resize_listener, ResizeConfig};
/// use web_sys::HtmlCanvasElement;
///
/// # fn example(canvas: HtmlCanvasElement, on_resize: impl Fn() + 'static) -> Result<(), String> {
/// let config = ResizeConfig::default();
/// let handler = attach_resize_listener(canvas, config, 300, on_resize)?;
/// // ... later when component unmounts ...
/// handler.remove()?;
/// # Ok(())
/// # }
/// ```
pub fn attach_resize_listener<F>(
    canvas: HtmlCanvasElement,
    config: ResizeConfig,
    debounce_ms: u32,
    on_resize: F,
) -> Result<ResizeHandler, String>
where
    F: Fn() + 'static,
{
    let window = window().ok_or("No window object available")?;

    // Shared state for debounce timer
    let timeout_handle: Rc<RefCell<Option<Timeout>>> = Rc::new(RefCell::new(None));

    // Wrap callback in Rc for sharing
    let on_resize_rc = Rc::new(on_resize);

    // Clone canvas for use in closure (moved into closure)
    let canvas_for_closure = canvas.clone();

    // Create closure for resize event
    let timeout_handle_clone = timeout_handle.clone();
    let on_resize_clone = on_resize_rc.clone();
    let closure = Closure::wrap(Box::new(move || {
        // Clear existing timeout if any
        let timeout: Option<Timeout> = timeout_handle_clone.borrow_mut().take();
        if let Some(t) = timeout {
            t.cancel();
        }

        // Clone necessary values for the timeout closure
        let canvas_clone = canvas_for_closure.clone();
        let config_clone = config;
        let on_resize_inner = on_resize_clone.clone();
        let timeout_handle_inner = timeout_handle_clone.clone();

        // Create new debounced timeout
        let timeout = Timeout::new(debounce_ms, move || {
            // Get current window size
            if let Ok((width, height)) = get_window_size() {
                // Calculate new canvas dimensions
                if let Ok((new_width, new_height)) =
                    calculate_canvas_size(width, height, &config_clone)
                {
                    // Resize canvas
                    if resize_canvas(&canvas_clone, new_width, new_height).is_ok() {
                        // Trigger callback for redraw
                        on_resize_inner();
                    }
                }
            }

            // Clear the timeout handle
            timeout_handle_inner.borrow_mut().take();
        });

        // Store timeout handle
        *timeout_handle_clone.borrow_mut() = Some(timeout);
    }) as Box<dyn FnMut()>);

    // Attach event listener
    window
        .add_event_listener_with_callback("resize", closure.as_ref().unchecked_ref())
        .map_err(|e| format!("Failed to attach resize listener: {:?}", e))?;

    // Perform initial resize
    let (width, height) = get_window_size()?;
    let (new_width, new_height) = calculate_canvas_size(width, height, &config)?;
    resize_canvas(&canvas, new_width, new_height)?;

    Ok(ResizeHandler { closure })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_calculate_canvas_size_basic() -> Result<(), String> {
        let config = ResizeConfig::default();
        let (w, h) = calculate_canvas_size(1920.0, 1080.0, &config)?;
        assert_eq!(w, 1920);
        assert_eq!(h, 1080);
        Ok(())
    }

    #[test]
    fn test_calculate_canvas_size_with_max_width() -> Result<(), String> {
        let config = ResizeConfig {
            max_width: Some(1600.0),
            ..Default::default()
        };
        let (w, h) = calculate_canvas_size(1920.0, 1080.0, &config)?;
        assert_eq!(w, 1600);
        assert_eq!(h, 1080);
        Ok(())
    }

    #[test]
    fn test_calculate_canvas_size_with_max_height() -> Result<(), String> {
        let config = ResizeConfig {
            max_height: Some(900.0),
            ..Default::default()
        };
        let (w, h) = calculate_canvas_size(1920.0, 1080.0, &config)?;
        assert_eq!(w, 1920);
        assert_eq!(h, 900);
        Ok(())
    }

    #[test]
    fn test_calculate_canvas_size_with_both_max_constraints() -> Result<(), String> {
        let config = ResizeConfig {
            max_width: Some(1600.0),
            max_height: Some(900.0),
            ..Default::default()
        };
        let (w, h) = calculate_canvas_size(1920.0, 1080.0, &config)?;
        assert_eq!(w, 1600);
        assert_eq!(h, 900);
        Ok(())
    }

    #[test]
    fn test_maintain_aspect_ratio_wide_window() -> Result<(), String> {
        let config = ResizeConfig {
            maintain_aspect_ratio: true,
            ..Default::default()
        };
        let (w, h) = calculate_canvas_size(1920.0, 1080.0, &config)?;

        // Should maintain 3:2 aspect ratio (1200:800)
        let ratio = w as f32 / h as f32;
        let expected_ratio = 1200.0 / 800.0;
        assert!(
            (ratio - expected_ratio).abs() < 0.01,
            "ratio={}, expected={}",
            ratio,
            expected_ratio
        );
        Ok(())
    }

    #[test]
    fn test_maintain_aspect_ratio_tall_window() -> Result<(), String> {
        let config = ResizeConfig {
            maintain_aspect_ratio: true,
            ..Default::default()
        };
        let (w, h) = calculate_canvas_size(800.0, 1200.0, &config)?;

        // Should maintain 3:2 aspect ratio
        let ratio = w as f32 / h as f32;
        let expected_ratio = 1200.0 / 800.0;
        assert!(
            (ratio - expected_ratio).abs() < 0.01,
            "ratio={}, expected={}",
            ratio,
            expected_ratio
        );
        Ok(())
    }

    #[test]
    fn test_invalid_window_width_zero() {
        let config = ResizeConfig::default();
        let result = calculate_canvas_size(0.0, 1080.0, &config);
        assert!(result.is_err());
        if let Err(msg) = result {
            assert!(msg.contains("Invalid window width"));
        }
    }

    #[test]
    fn test_invalid_window_width_negative() {
        let config = ResizeConfig::default();
        let result = calculate_canvas_size(-100.0, 1080.0, &config);
        assert!(result.is_err());
        if let Err(msg) = result {
            assert!(msg.contains("Invalid window width"));
        }
    }

    #[test]
    fn test_invalid_window_width_nan() {
        let config = ResizeConfig::default();
        let result = calculate_canvas_size(f32::NAN, 1080.0, &config);
        assert!(result.is_err());
        if let Err(msg) = result {
            assert!(msg.contains("Invalid window width"));
        }
    }

    #[test]
    fn test_invalid_window_width_infinity() {
        let config = ResizeConfig::default();
        let result = calculate_canvas_size(f32::INFINITY, 1080.0, &config);
        assert!(result.is_err());
        if let Err(msg) = result {
            assert!(msg.contains("Invalid window width"));
        }
    }

    #[test]
    fn test_invalid_window_height_zero() {
        let config = ResizeConfig::default();
        let result = calculate_canvas_size(1920.0, 0.0, &config);
        assert!(result.is_err());
        if let Err(msg) = result {
            assert!(msg.contains("Invalid window height"));
        }
    }

    #[test]
    fn test_invalid_window_height_negative() {
        let config = ResizeConfig::default();
        let result = calculate_canvas_size(1920.0, -100.0, &config);
        assert!(result.is_err());
        if let Err(msg) = result {
            assert!(msg.contains("Invalid window height"));
        }
    }

    #[test]
    fn test_invalid_window_height_nan() {
        let config = ResizeConfig::default();
        let result = calculate_canvas_size(1920.0, f32::NAN, &config);
        assert!(result.is_err());
        if let Err(msg) = result {
            assert!(msg.contains("Invalid window height"));
        }
    }

    #[test]
    fn test_invalid_window_height_infinity() {
        let config = ResizeConfig::default();
        let result = calculate_canvas_size(1920.0, f32::INFINITY, &config);
        assert!(result.is_err());
        if let Err(msg) = result {
            assert!(msg.contains("Invalid window height"));
        }
    }

    #[test]
    fn test_extreme_small_dimensions() -> Result<(), String> {
        let config = ResizeConfig::default();
        let (w, h) = calculate_canvas_size(1.0, 1.0, &config)?;
        assert_eq!(w, 1);
        assert_eq!(h, 1);
        Ok(())
    }

    #[test]
    fn test_extreme_large_dimensions() -> Result<(), String> {
        let config = ResizeConfig::default();
        let (w, h) = calculate_canvas_size(10000.0, 10000.0, &config)?;
        assert_eq!(w, 10000);
        assert_eq!(h, 10000);
        Ok(())
    }

    #[test]
    fn test_fractional_dimensions_floor() -> Result<(), String> {
        let config = ResizeConfig::default();
        let (w, h) = calculate_canvas_size(1920.7, 1080.9, &config)?;
        assert_eq!(w, 1920);
        assert_eq!(h, 1080);
        Ok(())
    }

    #[test]
    fn test_aspect_ratio_with_max_constraints() -> Result<(), String> {
        let config = ResizeConfig {
            maintain_aspect_ratio: true,
            max_width: Some(1200.0),
            max_height: Some(800.0),
        };
        let (w, h) = calculate_canvas_size(2000.0, 2000.0, &config)?;

        // Should respect max constraints first, then aspect ratio
        assert!(w <= 1200);
        assert!(h <= 800);

        let ratio = w as f32 / h as f32;
        let expected_ratio = 1200.0 / 800.0;
        assert!(
            (ratio - expected_ratio).abs() < 0.01,
            "ratio={}, expected={}",
            ratio,
            expected_ratio
        );
        Ok(())
    }
}
