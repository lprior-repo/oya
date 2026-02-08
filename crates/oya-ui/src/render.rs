// Render module - Terminal rendering with ANSI box-drawing characters
//
// This module provides rendering functionality for the OYA UI plugin,
// including:
// - ANSI box-drawing characters for pane borders
// - Text layout and wrapping
// - Color and styling support
// - Focused pane highlighting

use crate::components::style;
use crate::layout::{Layout, Pane, PaneType};
use crate::plugin::SampleBead;
use std::fmt::Write;

/// Terminal renderer for OYA UI
pub struct Renderer {
    /// Use colors (can be disabled for non-color terminals)
    use_colors: bool,
}

impl Renderer {
    /// Create a new renderer
    #[must_use]
    pub const fn new() -> Self {
        Self { use_colors: true }
    }

    /// Disable color output
    pub fn disable_colors(&mut self) {
        self.use_colors = false;
    }

    /// Enable color output
    pub fn enable_colors(&mut self) {
        self.use_colors = true;
    }

    /// Render the complete layout
    ///
    /// # Arguments
    ///
    /// * `layout` - Layout configuration
    /// * `beads` - List of beads to display
    /// * `selected_index` - Index of selected bead
    /// * `focused_pane` - Currently focused pane type
    ///
    /// # Returns
    ///
    /// Complete rendered output as a string
    #[must_use]
    pub fn render_layout(
        &self,
        layout: &Layout,
        beads: &[SampleBead],
        selected_index: usize,
        focused_pane: PaneType,
    ) -> String {
        let mut output = String::new();

        // Clear screen
        output.push_str("\x1b[2J\x1b[H");

        // Render each pane
        for pane in layout.panes() {
            let content = match pane.pane_type {
                PaneType::BeadList => {
                    self.render_bead_list(pane, beads, selected_index, focused_pane)
                }
                PaneType::BeadDetail => {
                    self.render_bead_detail(pane, beads, selected_index, focused_pane)
                }
                PaneType::PipelineView => {
                    self.render_pipeline_view(pane, beads, selected_index, focused_pane)
                }
                PaneType::WorkflowGraph => self.render_workflow_graph(pane, focused_pane),
            };

            // Render pane border and content
            let pane_output = self.render_pane(pane, &content, focused_pane);
            output.push_str(&pane_output);
        }

        // Render status bar at bottom
        let status = self.render_status_bar(focused_pane);
        output.push_str(&status);

        output
    }

    /// Render a single pane with border and content
    fn render_pane(&self, pane: &Pane, content: &str, focused_pane: PaneType) -> String {
        let is_focused = pane.pane_type == focused_pane;
        let mut output = String::new();

        // Move cursor to pane position
        write!(output, "\x1b[{};{}H", pane.row, pane.col).ok();

        // Top border
        if is_focused {
            let title = style::colorize(&pane.title, style::COLOR_GREEN);
            output.push_str(&self.render_top_border(pane.width, &title));
        } else {
            output.push_str(&self.render_top_border(pane.width, &pane.title));
        }

        // Content area
        let content_lines: Vec<&str> = content.lines().collect();
        let content_height = pane.height.saturating_sub(2); // Account for top and bottom borders

        for i in 0..content_height {
            write!(
                output,
                "\x1b[{};{}H",
                pane.row.saturating_add(1).saturating_add(i),
                pane.col
            )
            .ok();

            if i < content_lines.len() {
                let line = content_lines[i];
                output.push_str("│ ");
                output.push_str(line);
                output.push_str(
                    &" ".repeat(
                        pane.width
                            .saturating_sub(2)
                            .saturating_sub(line.chars().count()),
                    ),
                );
                output.push('│');
            } else {
                output.push('│');
                output.push_str(&" ".repeat(pane.width.saturating_sub(2)));
                output.push('│');
            }
        }

        // Bottom border
        write!(
            output,
            "\x1b[{};{}H",
            pane.row.saturating_add(pane.height).saturating_sub(1),
            pane.col
        )
        .ok();
        output.push_str(&self.render_bottom_border(pane.width));

        output
    }

    /// Render top border with title
    fn render_top_border(&self, width: usize, title: &str) -> String {
        let mut output = String::from("┌");

        // Add title (truncated if too long)
        let title_len = title.chars().count();
        let available_width = width.saturating_sub(4);

        if title_len <= available_width {
            output.push_str(title);
            output.push_str(&"─".repeat(width.saturating_sub(2).saturating_sub(title_len)));
        } else {
            let truncated: String = title.chars().take(available_width).collect();
            output.push_str(&truncated);
            output.push_str(&"─".repeat(width.saturating_sub(2).saturating_sub(available_width)));
        }

        output.push('┐');
        output.push('\n');
        output
    }

    /// Render bottom border
    fn render_bottom_border(&self, width: usize) -> String {
        let mut output = String::from("└");
        output.push_str(&"─".repeat(width.saturating_sub(2)));
        output.push('┘');
        output
    }

    /// Render bead list pane
    fn render_bead_list(
        &self,
        pane: &Pane,
        beads: &[SampleBead],
        selected_index: usize,
        focused_pane: PaneType,
    ) -> String {
        let is_focused = pane.pane_type == focused_pane;
        let mut content = String::new();

        if beads.is_empty() {
            content.push_str("No beads available.");
            return content;
        }

        // Header
        if is_focused {
            content.push_str(&style::colorize(
                "   ID      Priority  State     Title",
                style::COLOR_GREEN,
            ));
        } else {
            content.push_str("   ID      Priority  State     Title");
        }
        content.push('\n');

        // Beads
        for (i, bead) in beads.iter().enumerate() {
            if i >= pane.height.saturating_sub(3) {
                break; // Don't overflow pane
            }

            let state_color = match bead.state.as_str() {
                "open" => style::COLOR_RESET,
                "in_progress" => style::COLOR_GREEN,
                "blocked" => style::COLOR_RED,
                "closed" => style::COLOR_RESET,
                _ => style::COLOR_RESET,
            };

            let priority = match bead.priority {
                1 => "P1",
                2 => "P2",
                3 => "P3",
                _ => "P?",
            };

            let marker = if i == selected_index { "→" } else { " " };

            let line = format!(
                "{} {:8} {:8} {:9} {}",
                marker,
                &bead.id[..bead.id.len().min(8)],
                priority,
                bead.state,
                truncate(&bead.title, 20)
            );

            if is_focused {
                content.push_str(&style::colorize(&line, state_color));
            } else {
                content.push_str(&line);
            }
            content.push('\n');
        }

        content
    }

    /// Render bead detail pane
    fn render_bead_detail(
        &self,
        pane: &Pane,
        beads: &[SampleBead],
        selected_index: usize,
        focused_pane: PaneType,
    ) -> String {
        let is_focused = pane.pane_type == focused_pane;
        let mut content = String::new();

        let bead = match beads.get(selected_index) {
            Some(b) => b,
            None => {
                content.push_str("No bead selected.");
                return content;
            }
        };

        // Header
        if is_focused {
            content.push_str(&style::colorize(&bead.title, style::COLOR_GREEN));
        } else {
            content.push_str(&bead.title);
        }
        content.push('\n');
        content.push('\n');

        // Details
        content.push_str(&format!("ID:          {}\n", bead.id));
        content.push_str(&format!("Priority:    P{}\n", bead.priority));
        content.push_str(&format!("State:       {}\n", bead.state));
        content.push('\n');

        // Description placeholder
        content.push_str("Full bead details will be available\n");
        content.push_str("after IPC integration is implemented.\n");

        content
    }

    /// Render pipeline view pane
    fn render_pipeline_view(
        &self,
        pane: &Pane,
        beads: &[SampleBead],
        selected_index: usize,
        focused_pane: PaneType,
    ) -> String {
        let is_focused = pane.pane_type == focused_pane;
        let mut content = String::new();

        if beads.is_empty() || selected_index >= beads.len() {
            content.push_str("Select a bead to view pipeline.");
            return content;
        }

        // Header
        if is_focused {
            content.push_str(&style::colorize("Pipeline Stages", style::COLOR_GREEN));
        } else {
            content.push_str("Pipeline Stages");
        }
        content.push('\n');
        content.push('\n');

        // Stages (placeholder for now)
        let stages = [
            ("implement", "pending"),
            ("unit-test", "pending"),
            ("coverage", "pending"),
            ("lint", "pending"),
            ("static", "pending"),
            ("integration", "pending"),
            ("security", "pending"),
            ("review", "pending"),
        ];

        for (stage, status) in stages {
            let status_char = match status {
                "pending" => "○",
                "running" => "◐",
                "complete" => "●",
                "failed" => "✗",
                _ => "?",
            };

            if is_focused && status == "running" {
                content.push_str(&style::colorize(
                    &format!("  {} {}", status_char, stage),
                    style::COLOR_GREEN,
                ));
            } else {
                content.push_str(&format!("  {} {}", status_char, stage));
            }
            content.push('\n');
        }

        content
    }

    /// Render workflow graph pane
    fn render_workflow_graph(&self, pane: &Pane, focused_pane: PaneType) -> String {
        let is_focused = pane.pane_type == focused_pane;
        let mut content = String::new();

        // Header
        if is_focused {
            content.push_str(&style::colorize("Workflow Graph", style::COLOR_GREEN));
        } else {
            content.push_str("Workflow Graph");
        }
        content.push('\n');
        content.push('\n');

        // Placeholder graph (will be replaced with actual graph rendering)
        content.push_str("┌─────────┐\n");
        content.push_str("│ src-1   │\n");
        content.push_str("└───┬─────┘\n");
        content.push_str("    │\n");
        content.push_str("    ▼\n");
        content.push_str("┌─────────┐\n");
        content.push_str("│ src-2   │\n");
        content.push_str("└─────────┘\n");
        content.push('\n');
        content.push_str("(Graph visualization not yet implemented)");

        content
    }

    /// Render status bar
    fn render_status_bar(&self, focused_pane: PaneType) -> String {
        let mut status = String::new();

        status.push_str("\x1b[24;1H"); // Move to bottom row
        status.push_str(&style::colorize(
            &format!(
                " OYA UI | Focused: {} | q: quit | Tab: cycle panes | j/k: navigate | Enter: select ",
                focused_pane
            ),
            style::COLOR_GREEN,
        ));

        status
    }
}

impl Default for Renderer {
    fn default() -> Self {
        Self::new()
    }
}

/// Truncate text to fit width
fn truncate(text: &str, width: usize) -> String {
    let chars: Vec<char> = text.chars().collect();
    if chars.len() <= width {
        text.to_string()
    } else if width > 3 {
        let truncated: String = chars.iter().take(width.saturating_sub(3)).collect();
        format!("{}...", truncated)
    } else {
        "...".to_string()
    }
}

/// Wrap text to fit width
fn textwrap(text: &str, width: usize) -> Vec<String> {
    let mut lines = Vec::new();
    let mut current_line = String::new();
    let mut current_length = 0;

    for word in text.split_whitespace() {
        let word_len = word.chars().count();

        if current_length == 0 {
            current_line.push_str(word);
            current_length = word_len;
        } else if current_length.saturating_add(1).saturating_add(word_len) <= width {
            current_line.push(' ');
            current_line.push_str(word);
            current_length = current_length.saturating_add(1).saturating_add(word_len);
        } else {
            lines.push(current_line);
            current_line = word.to_string();
            current_length = word_len;
        }
    }

    if !current_line.is_empty() {
        lines.push(current_line);
    }

    lines
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used)]
    #![allow(clippy::expect_used)]

    use super::*;

    #[test]
    fn test_renderer_creation() {
        let renderer = Renderer::new();
        assert!(renderer.use_colors);
    }

    #[test]
    fn test_disable_colors() {
        let mut renderer = Renderer::new();
        renderer.disable_colors();
        assert!(!renderer.use_colors);
    }

    #[test]
    fn test_truncate() {
        assert_eq!(truncate("hello world", 5), "he...");
        assert_eq!(truncate("hi", 10), "hi");
        assert_eq!(truncate("hello", 3), "...");
    }

    #[test]
    fn test_textwrap() {
        let lines = textwrap("hello world this is a test", 15);
        assert!(!lines.is_empty());
        assert!(lines[0].len() <= 15);
    }

    #[test]
    fn test_render_top_border() {
        let renderer = Renderer::new();
        let border = renderer.render_top_border(20, "Test");
        assert!(border.starts_with('┌'));
        assert!(border.ends_with("┐\n"));
    }

    #[test]
    fn test_render_bottom_border() {
        let renderer = Renderer::new();
        let border = renderer.render_bottom_border(20);
        assert!(border.starts_with('└'));
        assert!(border.ends_with('┘'));
    }

    #[test]
    fn test_renderer_default() {
        let renderer = Renderer::default();
        assert!(renderer.use_colors);
    }
}
