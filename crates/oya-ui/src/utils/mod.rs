//! Utility functions for canvas operations and helpers

/// Result type for utility operations
pub type UtilResult<T> = Result<T, UtilError>;

/// Errors that can occur in utility functions
#[derive(Debug, Clone)]
pub enum UtilError {
    /// Canvas not found
    CanvasNotFound,
    /// Failed to get context
    ContextError(String),
}

impl std::fmt::Display for UtilError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            UtilError::CanvasNotFound => write!(f, "Canvas element not found"),
            UtilError::ContextError(msg) => write!(f, "Context error: {}", msg),
        }
    }
}

impl std::error::Error for UtilError {}

/// Canvas utility functions
pub mod canvas {
    use super::*;

    /// Clears the canvas
    pub fn clear_canvas() -> UtilResult<()> {
        // For WASM target, this would interact with web-sys
        // For now, return Ok as a stub
        Ok(())
    }

    /// Draws a circle on the canvas
    pub fn draw_circle(x: f64, y: f64, radius: f64) -> UtilResult<()> {
        // Validate parameters
        if radius < 0.0 {
            return Err(UtilError::ContextError(
                "Radius cannot be negative".to_string(),
            ));
        }
        // Stub implementation
        let _ = (x, y, radius);
        Ok(())
    }

    /// Draws a line on the canvas
    pub fn draw_line(x1: f64, y1: f64, x2: f64, y2: f64) -> UtilResult<()> {
        // Stub implementation
        let _ = (x1, y1, x2, y2);
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_clear_canvas() {
        let result = canvas::clear_canvas();
        assert!(result.is_ok());
    }

    #[test]
    fn test_draw_circle_valid() {
        let result = canvas::draw_circle(100.0, 100.0, 50.0);
        assert!(result.is_ok());
    }

    #[test]
    fn test_draw_circle_negative_radius() {
        let result = canvas::draw_circle(100.0, 100.0, -10.0);
        assert!(result.is_err());
    }

    #[test]
    fn test_draw_line() {
        let result = canvas::draw_line(0.0, 0.0, 100.0, 100.0);
        assert!(result.is_ok());
    }
}
