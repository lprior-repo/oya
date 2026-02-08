// Render module - Terminal rendering for OYA UI
//
// Handles rendering of layouts and content to terminal output using
// ANSI box-drawing characters.

use crate::layout::{Layout, Pane};
use thiserror::Error;

/// Errors that can occur during rendering
#[derive(Debug, Error)]
pub enum RenderError {
    #[error("Rendering failed: {0}")]
    RenderFailed(String),

    #[error("Invalid content: {0}")]
    InvalidContent(String),
}

/// Result type for rendering operations
pub type RenderResult<T> = Result<T, RenderError>;

/// Terminal renderer for OYA UI
///
/// Renders layouts using ANSI box-drawing characters:
/// ┌─┬─┐
/// │ │ │
/// ├─┼─┤
/// │ │ │
/// └─┴─┘
pub struct Renderer;

impl Renderer {
    /// Create a new renderer
    #[must_use]
    pub const fn new() -> Self {
        Self
    }

    /// Render a layout to a string
    ///
    /// # Arguments
    ///
    /// * `layout` - The layout to render
    /// * `rows` - Total rows in terminal
    /// * `cols` - Total columns in terminal
    ///
    /// # Returns
    ///
    /// A string containing the rendered output
    ///
    /// # Errors
    ///
    /// Returns an error if rendering fails
    pub fn render_layout(&self, _layout: &Layout, rows: usize, cols: usize) -> RenderResult<String> {
        let mut output = String::new();

        // Clear screen
        output.push_str("\x1b[2J\x1b[H");

        // Calculate layout for terminal size
        let adjusted_layout = Layout::calculate_for_terminal(rows, cols)
            .map_err(|e| RenderError::RenderFailed(e.to_string()))?;

        // Render top border
        self.render_top_border(&mut output, cols);

        // Render panes
        for pane in adjusted_layout.panes() {
            self.render_pane(&mut output, pane)?;
        }

        // Render bottom border
        self.render_bottom_border(&mut output, cols);

        Ok(output)
    }

    /// Render top border
    fn render_top_border(&self, output: &mut String, width: usize) {
        output.push_str("┏");
        for _ in 1..width.saturating_sub(1) {
            output.push('━');
        }
        output.push_str("┓\r\n");
    }

    /// Render bottom border
    fn render_bottom_border(&self, output: &mut String, width: usize) {
        output.push_str("┗");
        for _ in 1..width.saturating_sub(1) {
            output.push('━');
        }
        output.push_str("┛\r\n");
    }

    /// Render a single pane
    ///
    /// # Arguments
    ///
    /// * `output` - Output buffer to write to
    /// * `pane` - The pane to render
    ///
    /// # Errors
    ///
    /// Returns an error if rendering fails
    fn render_pane(&self, output: &mut String, pane: &Pane) -> RenderResult<()> {
        // Render top border with title
        output.push_str("┃");
        output.push_str(&pane.title);

        // Pad with spaces
        let title_len = pane.title.len();
        if title_len < pane.width {
            for _ in 0..pane.width.saturating_sub(title_len) {
                output.push(' ');
            }
        }

        output.push_str("┃\r\n");

        // Render separator
        output.push_str("┃");
        for _ in 0..pane.width {
            output.push('─');
        }
        output.push_str("┃\r\n");

        // Render content area (empty for now)
        for _ in 0..pane.height.saturating_sub(2) {
            output.push_str("┃");
            for _ in 0..pane.width {
                output.push(' ');
            }
            output.push_str("┃\r\n");
        }

        Ok(())
    }

    /// Render horizontal divider
    ///
    /// # Arguments
    ///
    /// * `output` - Output buffer
    /// * `width` - Width of the divider
    /// * `left` - Width of left section
    fn render_horizontal_divider(&self, output: &mut String, width: usize, left: usize) {
        // Render left section
        output.push_str("┃");
        for _ in 0..left {
            output.push('─');
        }

        // Render middle
        output.push_str("╋");

        // Render right section
        let remaining = width.saturating_sub(left).saturating_sub(1);
        for _ in 0..remaining {
            output.push('─');
        }

        output.push_str("┃\r\n");
    }

    /// Render vertical divider
    ///
    /// # Arguments
    ///
    /// * `output` - Output buffer
    /// * `row` - Row position
    /// * `col` - Column position of divider
    fn render_vertical_divider(&self, output: &mut String, row: usize, col: usize) {
        // Move cursor to position
        output.push_str(&format!("\x1b[{};{}H", row, col));
        output.push('┣');
        output.push('━');
        output.push('┫');
    }

    /// Render text at position
    ///
    /// # Arguments
    ///
    /// * `output` - Output buffer
    /// * `row` - Row position
    /// * `col` - Column position
    /// * `text` - Text to render
    pub fn render_text(&self, output: &mut String, row: usize, col: usize, text: &str) {
        output.push_str(&format!("\x1b[{};{}H", row, col));
        output.push_str(text);
    }

    /// Clear area
    ///
    /// # Arguments
    ///
    /// * `output` - Output buffer
    /// * `row` - Starting row
    /// * `col` - Starting column
    /// * `height` - Height of area
    /// * `width` - Width of area
    pub fn clear_area(&self, output: &mut String, row: usize, col: usize, height: usize, width: usize) {
        for r in 0..height {
            self.render_text(output, row + r, col, &" ".repeat(width));
        }
    }
}

impl Default for Renderer {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used)]
    #![allow(clippy::expect_used)]

    use super::*;

    #[test]
    fn test_renderer_creation() {
        let renderer = Renderer::new();
        assert_eq!(renderer.render_layout(&Layout::new_3_pane(), 24, 80).is_ok(), true);
    }

    #[test]
    fn test_render_layout() {
        let renderer = Renderer::new();
        let layout = Layout::new_3_pane();
        let output = renderer.render_layout(&layout, 24, 80).expect("Failed to render");

        assert!(!output.is_empty());
        assert!(output.contains('┏'));
        assert!(output.contains('┃'));
    }

    #[test]
    fn test_render_text() {
        let renderer = Renderer::new();
        let mut output = String::new();
        renderer.render_text(&mut output, 5, 10, "Test");

        assert!(output.contains("\x1b[5;10H"));
        assert!(output.contains("Test"));
    }

    #[test]
    fn test_clear_area() {
        let renderer = Renderer::new();
        let mut output = String::new();
        renderer.clear_area(&mut output, 5, 10, 3, 20);

        // Should have moved cursor and written spaces
        assert!(!output.is_empty());
    }

    #[test]
    fn test_render_small_terminal() {
        let renderer = Renderer::new();
        let layout = Layout::new_3_pane();
        let result = renderer.render_layout(&layout, 10, 20);
        assert!(result.is_err());
    }
}
