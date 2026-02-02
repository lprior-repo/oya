//! Canvas clearing and redrawing
//!
//! Provides pure functions for clearing canvas and rendering backgrounds.
//! All operations return Results for proper error handling.

use web_sys::CanvasRenderingContext2d;

/// Configuration for canvas clearing
#[derive(Debug, Clone, Copy)]
pub struct ClearConfig {
    pub width: f32,
    pub height: f32,
    pub background_color: &'static str,
    pub render_grid: bool,
}

impl Default for ClearConfig {
    fn default() -> Self {
        Self {
            width: 1200.0,
            height: 800.0,
            background_color: "#FFFFFF",
            render_grid: false,
        }
    }
}

/// Clear canvas and apply background
///
/// Pure rendering function - no state mutation.
/// Returns Result for error handling (though canvas ops rarely fail).
///
/// # Errors
///
/// Returns an error if canvas operations fail (rare in practice)
///
/// # Example
///
/// ```no_run
/// use oya_ui::components::canvas::{create_canvas, CanvasConfig};
/// use oya_ui::components::canvas::context::get_2d_context;
/// use oya_ui::components::canvas::clear::{clear_canvas, ClearConfig};
///
/// let canvas_config = CanvasConfig::default();
/// let canvas = create_canvas(&canvas_config)?;
/// let context = get_2d_context(&canvas)?;
///
/// let clear_config = ClearConfig::default();
/// clear_canvas(&context, &clear_config)?;
/// # Ok::<(), String>(())
/// ```
pub fn clear_canvas(
    context: &CanvasRenderingContext2d,
    config: &ClearConfig,
) -> Result<(), String> {
    // Clear entire canvas area
    context.clear_rect(0.0, 0.0, config.width.into(), config.height.into());

    // Apply background color
    context.set_fill_style_str(config.background_color);
    context.fill_rect(0.0, 0.0, config.width.into(), config.height.into());

    // Optionally render grid
    if config.render_grid {
        render_grid(context, config)?;
    }

    Ok(())
}

/// Render optional background grid
///
/// Helpful for visualizing coordinate space during development.
///
/// # Errors
///
/// Returns an error if canvas operations fail (rare in practice)
fn render_grid(context: &CanvasRenderingContext2d, config: &ClearConfig) -> Result<(), String> {
    const GRID_SIZE: f32 = 50.0;
    const GRID_COLOR: &str = "#E0E0E0";

    context.set_stroke_style_str(GRID_COLOR);
    context.set_line_width(1.0);

    context.begin_path();

    // Vertical lines
    let mut x = 0.0;
    while x <= config.width {
        context.move_to(x.into(), 0.0);
        context.line_to(x.into(), config.height.into());
        x += GRID_SIZE;
    }

    // Horizontal lines
    let mut y = 0.0;
    while y <= config.height {
        context.move_to(0.0, y.into());
        context.line_to(config.width.into(), y.into());
        y += GRID_SIZE;
    }

    context.stroke();

    Ok(())
}

#[cfg(test)]
#[cfg(target_arch = "wasm32")]
mod tests {
    use super::*;
    use crate::components::canvas::{
        context::get_2d_context,
        init::{CanvasConfig, create_canvas},
    };
    use wasm_bindgen_test::*;

    wasm_bindgen_test_configure!(run_in_browser);

    #[wasm_bindgen_test]
    fn test_clear_canvas_succeeds() -> Result<(), Box<dyn std::error::Error>> {
        let canvas_config = CanvasConfig {
            width: 800,
            height: 600,
            id: "test-clear-1".to_string(),
        };

        let canvas = create_canvas(&canvas_config)?;
        let context = get_2d_context(&canvas)?;

        let clear_config = ClearConfig::default();
        clear_canvas(&context, &clear_config)?;
        Ok(())
    }

    #[wasm_bindgen_test]
    fn test_clear_with_custom_dimensions() -> Result<(), Box<dyn std::error::Error>> {
        let canvas_config = CanvasConfig {
            width: 1920,
            height: 1080,
            id: "test-clear-2".to_string(),
        };

        let canvas = create_canvas(&canvas_config)?;
        let context = get_2d_context(&canvas)?;

        let clear_config = ClearConfig {
            width: 1920.0,
            height: 1080.0,
            ..Default::default()
        };

        clear_canvas(&context, &clear_config)?;
        Ok(())
    }

    #[wasm_bindgen_test]
    fn test_clear_with_grid() -> Result<(), Box<dyn std::error::Error>> {
        let canvas_config = CanvasConfig {
            width: 800,
            height: 600,
            id: "test-clear-3".to_string(),
        };

        let canvas = create_canvas(&canvas_config)?;
        let context = get_2d_context(&canvas)?;

        let clear_config = ClearConfig {
            render_grid: true,
            ..Default::default()
        };

        clear_canvas(&context, &clear_config)?;
        Ok(())
    }

    #[wasm_bindgen_test]
    fn test_multiple_clears() -> Result<(), Box<dyn std::error::Error>> {
        let canvas_config = CanvasConfig {
            width: 800,
            height: 600,
            id: "test-clear-4".to_string(),
        };

        let canvas = create_canvas(&canvas_config)?;
        let context = get_2d_context(&canvas)?;

        let clear_config = ClearConfig::default();

        // Multiple clears should all succeed
        clear_canvas(&context, &clear_config)?;
        clear_canvas(&context, &clear_config)?;
        clear_canvas(&context, &clear_config)?;
        Ok(())
    }

    #[wasm_bindgen_test]
    fn test_clear_with_different_background() -> Result<(), Box<dyn std::error::Error>> {
        let canvas_config = CanvasConfig {
            width: 800,
            height: 600,
            id: "test-clear-5".to_string(),
        };

        let canvas = create_canvas(&canvas_config)?;
        let context = get_2d_context(&canvas)?;

        let clear_config = ClearConfig {
            background_color: "#1F2937",
            ..Default::default()
        };

        clear_canvas(&context, &clear_config)?;
        Ok(())
    }

    #[wasm_bindgen_test]
    fn test_extreme_dimensions() -> Result<(), Box<dyn std::error::Error>> {
        let canvas_config = CanvasConfig {
            width: 4096,
            height: 4096,
            id: "test-clear-6".to_string(),
        };

        let canvas = create_canvas(&canvas_config)?;
        let context = get_2d_context(&canvas)?;

        let clear_config = ClearConfig {
            width: 4096.0,
            height: 4096.0,
            ..Default::default()
        };

        clear_canvas(&context, &clear_config)?;
        Ok(())
    }

    #[wasm_bindgen_test]
    fn test_small_dimensions() -> Result<(), Box<dyn std::error::Error>> {
        let canvas_config = CanvasConfig {
            width: 10,
            height: 10,
            id: "test-clear-7".to_string(),
        };

        let canvas = create_canvas(&canvas_config)?;
        let context = get_2d_context(&canvas)?;

        let clear_config = ClearConfig {
            width: 10.0,
            height: 10.0,
            ..Default::default()
        };

        clear_canvas(&context, &clear_config)?;
        Ok(())
    }

    #[wasm_bindgen_test]
    fn test_grid_with_various_dimensions() -> Result<(), Box<dyn std::error::Error>> {
        let canvas_config = CanvasConfig {
            width: 333,
            height: 777,
            id: "test-clear-8".to_string(),
        };

        let canvas = create_canvas(&canvas_config)?;
        let context = get_2d_context(&canvas)?;

        let clear_config = ClearConfig {
            width: 333.0,
            height: 777.0,
            render_grid: true,
            ..Default::default()
        };

        clear_canvas(&context, &clear_config)?;
        Ok(())
    }

    #[wasm_bindgen_test]
    fn test_rapid_repeated_clears() -> Result<(), Box<dyn std::error::Error>> {
        let canvas_config = CanvasConfig {
            width: 800,
            height: 600,
            id: "test-clear-9".to_string(),
        };

        let canvas = create_canvas(&canvas_config)?;
        let context = get_2d_context(&canvas)?;

        let clear_config = ClearConfig::default();

        // Simulate rapid frame clears
        for _ in 0..100 {
            let result = clear_canvas(&context, &clear_config);
            assert!(result.is_ok(), "Rapid clears should all succeed");
        }
        Ok(())
    }

    #[wasm_bindgen_test]
    fn test_alternating_grid_renders() -> Result<(), Box<dyn std::error::Error>> {
        let canvas_config = CanvasConfig {
            width: 800,
            height: 600,
            id: "test-clear-10".to_string(),
        };

        let canvas = create_canvas(&canvas_config)?;
        let context = get_2d_context(&canvas)?;

        // Alternate between grid and no grid
        for i in 0..10 {
            let clear_config = ClearConfig {
                render_grid: i % 2 == 0,
                ..Default::default()
            };

            clear_canvas(&context, &clear_config)?;
        }
        Ok(())
    }
}
