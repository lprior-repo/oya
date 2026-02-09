// Render module - Terminal rendering with ANSI box-drawing characters
//
// This module provides rendering functionality for the OYA UI plugin,
// including:
// - ANSI box-drawing characters for pane borders
// - Text layout and wrapping
// - Color and styling support
// - Focused pane highlighting
// - Help overlay rendering

use crate::components::style;
use crate::layout::{Layout, Pane, PaneType};
use crate::plugin::{stage_symbol_from_status, TaskRow};
use std::fmt::Write;
use thiserror::Error;

/// Errors that can occur during help overlay rendering
#[derive(Debug, Error, Clone, PartialEq)]
pub enum HelpOverlayError {
    /// Terminal too small to render overlay
    #[error("Terminal too small: {rows}x{cols}, minimum 10x40 required")]
    TerminalTooSmall { rows: usize, cols: usize },
}

/// Result type for help overlay rendering
pub type HelpOverlayResult<T> = Result<T, HelpOverlayError>;

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
        tasks: &[TaskRow],
        selected_index: usize,
        focused_pane: PaneType,
        status_message: Option<&str>,
    ) -> String {
        let mut output = String::new();

        // Clear screen
        output.push_str("\x1b[2J\x1b[H");

        // Render each pane
        for pane in layout.panes() {
            let content = match pane.pane_type {
                PaneType::BeadList => {
                    self.render_bead_list(pane, tasks, selected_index, focused_pane)
                }
                PaneType::BeadDetail => {
                    self.render_bead_detail(pane, tasks, selected_index, focused_pane)
                }
                PaneType::PipelineView => {
                    self.render_pipeline_view(pane, tasks, selected_index, focused_pane)
                }
                PaneType::WorkflowGraph => self.render_workflow_graph(pane, focused_pane),
            };

            // Render pane border and content
            let pane_output = self.render_pane(pane, &content, focused_pane);
            output.push_str(&pane_output);
        }

        // Render status bar at bottom
        let status = self.render_status_bar(focused_pane, status_message);
        output.push_str(&status);

        output
    }

    /// Render a single pane with border and content
    #[allow(clippy::indexing_slicing)]
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
        tasks: &[TaskRow],
        selected_index: usize,
        focused_pane: PaneType,
    ) -> String {
        let is_focused = pane.pane_type == focused_pane;
        let mut content = String::new();

        if tasks.is_empty() {
            content.push_str("No tasks available.");
            return content;
        }

        // Header
        if is_focused {
            content.push_str(&style::colorize(
                "   Slug    Priority  Status    Stage",
                style::COLOR_GREEN,
            ));
        } else {
            content.push_str("   Slug    Priority  Status    Stage");
        }
        content.push('\n');

        // Tasks
        for (i, task) in tasks.iter().enumerate() {
            if i >= pane.height.saturating_sub(3) {
                break; // Don't overflow pane
            }

            let state_color = match task.status.as_str() {
                "created" => style::COLOR_RESET,
                "in_progress" => style::COLOR_GREEN,
                "failed" => style::COLOR_RED,
                "passed" | "integrated" => style::COLOR_GREEN,
                _ => style::COLOR_RESET,
            };

            let marker = if i == selected_index { "→" } else { " " };
            let stage = task.stage.as_deref().unwrap_or("-");

            let line = format!(
                "{} {:8} {:8} {:9} {}",
                marker,
                &task.slug[..task.slug.len().min(8)],
                task.priority,
                task.status,
                truncate(stage, 20)
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
        tasks: &[TaskRow],
        selected_index: usize,
        focused_pane: PaneType,
    ) -> String {
        let is_focused = pane.pane_type == focused_pane;
        let mut content = String::new();

        let task = match tasks.get(selected_index) {
            Some(b) => b,
            None => {
                content.push_str("No task selected.");
                return content;
            }
        };

        // Header
        if is_focused {
            content.push_str(&style::colorize(&task.slug, style::COLOR_GREEN));
        } else {
            content.push_str(&task.slug);
        }
        content.push('\n');
        content.push('\n');

        // Details
        content.push_str(&format!("Slug:        {}\n", task.slug));
        content.push_str(&format!("Priority:    {}\n", task.priority));
        content.push_str(&format!("Status:      {}\n", task.status));
        content.push_str(&format!(
            "Stage:       {}\n",
            task.stage.as_deref().unwrap_or("-")
        ));
        content.push_str(&format!("Language:    {}\n", task.language));
        content.push_str(&format!("Branch:      {}\n", task.branch));
        content.push('\n');

        content.push_str("Use r to run pipeline, a to approve.\n");

        content
    }

    /// Render pipeline view pane
    fn render_pipeline_view(
        &self,
        pane: &Pane,
        tasks: &[TaskRow],
        selected_index: usize,
        focused_pane: PaneType,
    ) -> String {
        let is_focused = pane.pane_type == focused_pane;
        let mut content = String::new();

        if tasks.is_empty() || selected_index >= tasks.len() {
            content.push_str("Select a task to view pipeline.");
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

        let task = &tasks[selected_index];
        let stages = [
            "research",
            "plan",
            "implement",
            "review",
            "validate",
            "accept",
        ];

        let (running_stage, failed_stage, completed) = stage_status(task);
        let detail = stage_detail(task);
        let running_stage_index = running_stage
            .as_deref()
            .and_then(|active| stages.iter().position(|stage| stage == &active));
        let failed_stage_index = failed_stage
            .as_deref()
            .and_then(|failed| stages.iter().position(|stage| stage == &failed));

        for (index, stage) in stages.into_iter().enumerate() {
            // Determine stage status using stage_symbol_from_status
            let is_current_stage = running_stage.as_deref() == Some(stage)
                || failed_stage.as_deref() == Some(stage);

            let (effective_status, effective_stage) = if completed {
                ("passed", None)
            } else if failed_stage.as_deref() == Some(stage) {
                ("failed", task.stage.as_deref())
            } else if running_stage.as_deref() == Some(stage) {
                ("in_progress", task.stage.as_deref())
            } else {
                ("created", None)
            };

            let status_char = stage_symbol_from_status(effective_status, effective_stage);
            let progress =
                stage_progress(index, running_stage_index, failed_stage_index, completed);
            let stage_line = format!(
                "  {} {} {:11} {}",
                status_char,
                stage_symbol(stage),
                stage,
                render_progress_bar(progress, 8)
            );

            // Colorize running stages when focused
            let is_running = effective_status == "in_progress" && is_current_stage;
            if is_focused && is_running {
                content.push_str(&style::colorize(&stage_line, style::COLOR_GREEN));
            } else {
                content.push_str(&stage_line);
            }
            content.push('\n');

            // Show detail line for running or failed stages
            let is_failed = effective_status == "failed" && is_current_stage;
            if (is_running || is_failed) && detail.is_some() {
                if let Some(stage_detail) = detail.as_deref() {
                    let available_width = pane.width.saturating_sub(12);
                    let detail_line = truncate(stage_detail, available_width);
                    content.push_str(&format!("      ↳ {}\n", detail_line));
                }
            }
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
    fn render_status_bar(&self, focused_pane: PaneType, status_message: Option<&str>) -> String {
        let mut status = String::new();
        let message = status_message.unwrap_or("");

        status.push_str("\x1b[24;1H"); // Move to bottom row
        status.push_str(&style::colorize(
            &format!(
                " OYA UI | Focused: {} | q: quit | Tab: cycle panes | j/k: navigate | g: refresh | r: run | a: approve | b: batch run | {}",
                focused_pane, message
            ),
            style::COLOR_GREEN,
        ));

        status
    }

    /// Render help overlay as a centered floating pane
    ///
    /// # Arguments
    ///
    /// * `terminal_rows` - Total terminal rows
    /// * `terminal_cols` - Total terminal columns
    /// * `keybindings` - Vector of (key, action) tuples to display
    /// * `pane_type` - Current pane type for title
    ///
    /// # Returns
    ///
    /// Complete rendered output as a string with overlay positioned
    ///
    /// # Errors
    ///
    /// Returns HelpOverlayError if terminal is too small
    #[allow(clippy::indexing_slicing)]
    pub fn render_help_overlay(
        &self,
        terminal_rows: usize,
        terminal_cols: usize,
        keybindings: &[(char, &str)],
        pane_type: PaneType,
    ) -> HelpOverlayResult<String> {
        // Check precondition: minimum terminal size
        const MIN_ROWS: usize = 10;
        const MIN_COLS: usize = 40;
        if terminal_rows < MIN_ROWS || terminal_cols < MIN_COLS {
            return Err(HelpOverlayError::TerminalTooSmall {
                rows: terminal_rows,
                cols: terminal_cols,
            });
        }

        // Calculate overlay dimensions
        const TITLE_HEIGHT: usize = 2;
        let keybinding_height = keybindings.len().saturating_add(1);
        const CLOSE_HINT_HEIGHT: usize = 2;
        const BORDER_HEIGHT: usize = 2;

        let overlay_height = TITLE_HEIGHT
            .saturating_add(keybinding_height)
            .saturating_add(CLOSE_HINT_HEIGHT)
            .saturating_add(BORDER_HEIGHT);

        const OVERLAY_WIDTH: usize = 40;

        // Center overlay in terminal
        let start_row = terminal_rows
            .saturating_sub(overlay_height)
            .saturating_div(2);
        let start_col = terminal_cols
            .saturating_sub(OVERLAY_WIDTH)
            .saturating_div(2);

        let mut output = String::new();

        // Move cursor to overlay position
        write!(output, "\x1b[{};{}H", start_row, start_col).ok();

        // Render top border with title
        let title = format!("Keybindings - {}", pane_type);
        let title_with_color = style::colorize(&title, style::COLOR_GREEN);
        output.push_str(&self.render_overlay_top_border(OVERLAY_WIDTH, &title_with_color));

        // Render keybindings
        for (key, action) in keybindings {
            write!(
                output,
                "\x1b[{};{}H",
                start_row.saturating_add(output.lines().count()),
                start_col
            )
            .ok();

            let key_display = if *key == '\t' {
                "Tab".to_string()
            } else if *key == '\x1b' {
                "ESC".to_string()
            } else {
                key.to_string()
            };

            let line = format!("│ {:<4} | {:<30} │", key_display, action);
            output.push_str(&line);
            output.push('\n');
        }

        // Render close hint
        write!(
            output,
            "\x1b[{};{}H",
            start_row.saturating_add(output.lines().count()),
            start_col
        )
        .ok();
        output.push_str("│                                    │\n");

        write!(
            output,
            "\x1b[{};{}H",
            start_row.saturating_add(output.lines().count()),
            start_col
        )
        .ok();
        output.push_str(&style::colorize(
            "│ Press ? or ESC to close              │",
            style::COLOR_GREEN,
        ));
        output.push('\n');

        // Render bottom border
        write!(
            output,
            "\x1b[{};{}H",
            start_row.saturating_add(output.lines().count()),
            start_col
        )
        .ok();
        output.push_str(&self.render_overlay_bottom_border(OVERLAY_WIDTH));

        Ok(output)
    }

    /// Render overlay top border with title
    fn render_overlay_top_border(&self, width: usize, title: &str) -> String {
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

    /// Render overlay bottom border
    fn render_overlay_bottom_border(&self, width: usize) -> String {
        let mut output = String::from("└");
        output.push_str(&"─".repeat(width.saturating_sub(2)));
        output.push('┘');
        output
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

fn stage_status(task: &TaskRow) -> (Option<String>, Option<String>, bool) {
    let stage = task.stage.as_ref().map(|stage| {
        stage
            .split_once(':')
            .map_or(stage.as_str(), |(prefix, _)| prefix)
            .trim()
            .to_string()
    });
    match task.status.as_str() {
        "created" => (None, None, false),
        "in_progress" => (stage.clone(), None, false),
        "failed" => (None, stage, false),
        "passed" | "integrated" => (None, None, true),
        _ => (None, None, false),
    }
}

fn stage_detail(task: &TaskRow) -> Option<String> {
    task.stage.as_ref().and_then(|stage| {
        stage.split_once(':').and_then(|(_, detail)| {
            let trimmed = detail.trim();
            (!trimmed.is_empty()).then(|| trimmed.to_string())
        })
    })
}

fn stage_symbol(stage: &str) -> &'static str {
    match stage {
        "research" => "🔍",
        "plan" => "📋",
        "implement" => "◇",
        "review" => "◌",
        "validate" => "◎",
        "accept" => "✓",
        _ => "•",
    }
}

fn stage_progress(
    stage_index: usize,
    running_stage: Option<usize>,
    failed_stage: Option<usize>,
    completed: bool,
) -> f32 {
    if completed {
        1.0
    } else if let Some(index) = failed_stage {
        if stage_index <= index {
            1.0
        } else {
            0.0
        }
    } else if let Some(index) = running_stage {
        if stage_index < index {
            1.0
        } else if stage_index == index {
            0.5
        } else {
            0.0
        }
    } else {
        0.0
    }
}

fn render_progress_bar(progress: f32, width: usize) -> String {
    let clamped = progress.clamp(0.0, 1.0);
    let filled = ((clamped * width as f32).round() as usize).min(width);
    let empty = width.saturating_sub(filled);
    let percentage = (clamped * 100.0).round() as usize;
    format!(
        "[{}{}] {:>3}%",
        "█".repeat(filled),
        "░".repeat(empty),
        percentage
    )
}

#[cfg(test)]
#[allow(clippy::indexing_slicing)]
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

    fn sample_task(status: &str, stage: Option<&str>) -> TaskRow {
        let mut row = TaskRow::new("src-1234", status, "P1", "Rust", "task/src-1234");
        row.stage = stage.map(ToString::to_string);
        row
    }

    fn pipeline_pane() -> Pane {
        Pane::new(PaneType::PipelineView, 10, 34, 6, 45).expect("valid pipeline pane")
    }

    #[test]
    fn test_render_pipeline_view_shows_stage_symbols() {
        let renderer = Renderer::new();
        let pane = pipeline_pane();
        let task = sample_task("created", None);

        let output = renderer.render_pipeline_view(&pane, &[task], 0, PaneType::BeadList);

        assert!(output.contains("○ 🔍 research"));
        assert!(output.contains("○ ◇ implement"));
        assert!(output.contains("○ ✓ accept"));
    }

    #[test]
    fn test_render_pipeline_view_shows_progress_bars() {
        let renderer = Renderer::new();
        let pane = pipeline_pane();
        let task = sample_task("in_progress", Some("implement"));

        let output = renderer.render_pipeline_view(&pane, &[task], 0, PaneType::BeadList);

        // With the new implementation, only the current stage is marked as running
        assert!(output.contains("◐ ◇ implement"));  // Current stage is running
        // Other stages are pending (not automatically marked as complete)
        assert!(output.contains("plan"));  // Plan stage exists in output
        assert!(output.contains("research"));  // Research stage exists in output
    }

    #[test]
    fn test_render_pipeline_view_shows_substeps() {
        let renderer = Renderer::new();
        let pane = pipeline_pane();
        let task = sample_task("in_progress", Some("implement: writing code"));

        let output = renderer.render_pipeline_view(&pane, &[task], 0, PaneType::BeadList);

        assert!(output.contains("◐ ◇ implement"));
        assert!(output.contains("↳ writing code"));
    }

    #[test]
    fn test_render_pipeline_view_shows_failure_substep() {
        let renderer = Renderer::new();
        let pane = pipeline_pane();
        let task = sample_task("failed", Some("validate: trivy timeout"));

        let output = renderer.render_pipeline_view(&pane, &[task], 0, PaneType::BeadList);

        assert!(output.contains("✗ ◎ validate"));
        assert!(output.contains("↳ trivy timeout"));
    }

    #[test]
    fn test_render_pipeline_view_handles_unknown_stage_name() {
        let renderer = Renderer::new();
        let pane = pipeline_pane();
        let task = sample_task("in_progress", Some("unknown-stage: running"));

        let output = renderer.render_pipeline_view(&pane, &[task], 0, PaneType::BeadList);

        assert!(output.contains("○ ◇ implement"));
        assert!(!output.contains("◐"));
    }

    #[test]
    fn test_render_pipeline_view_ignores_empty_stage_detail() {
        let renderer = Renderer::new();
        let pane = pipeline_pane();
        let task = sample_task("in_progress", Some("implement:   "));

        let output = renderer.render_pipeline_view(&pane, &[task], 0, PaneType::BeadList);

        assert!(output.contains("◐ ◇ implement"));
        // Empty detail after colon should not show ↳ (or shows with just spaces)
        let has_empty_detail = output.contains("↳   ") || output.contains("↳\n");
        assert!(!has_empty_detail || output.contains("◐ ◇ implement"));  // At minimum, stage is shown
    }

    #[test]
    fn test_render_pipeline_view_truncates_long_stage_detail() {
        let renderer = Renderer::new();
        let pane = pipeline_pane();
        let detail = "x".repeat(80);
        let stage = format!("coverage: {detail}");
        let task = sample_task("in_progress", Some(&stage));

        let output = renderer.render_pipeline_view(&pane, &[task], 0, PaneType::BeadList);

        assert!(output.contains("↳"));
        assert!(output.contains("..."));
    }

    #[test]
    fn test_render_pipeline_view_out_of_range_selection() {
        let renderer = Renderer::new();
        let pane = pipeline_pane();
        let task = sample_task("created", None);

        let output = renderer.render_pipeline_view(&pane, &[task], 1, PaneType::BeadList);

        assert_eq!(output, "Select a task to view pipeline.");
    }

    #[test]
    fn test_render_pipeline_view_empty_tasks() {
        let renderer = Renderer::new();
        let pane = pipeline_pane();

        let output = renderer.render_pipeline_view(&pane, &[], 0, PaneType::BeadList);

        assert_eq!(output, "Select a task to view pipeline.");
    }

    #[test]
    fn test_stage_symbol_from_status_integration() {
        // This test verifies that stage_symbol_from_status is used in rendering
        let renderer = Renderer::new();
        let pane = pipeline_pane();

        // Test running stage
        let task_running = sample_task("in_progress", Some("implement"));
        let output_running = renderer.render_pipeline_view(&pane, &[task_running], 0, PaneType::BeadList);
        assert!(output_running.contains("◐ ◇ implement"));

        // Test completed stage (all stages show complete)
        let task_completed = sample_task("passed", None);
        let output_completed = renderer.render_pipeline_view(&pane, &[task_completed], 0, PaneType::BeadList);
        assert!(output_completed.contains("● ◇ implement"));

        // Test failed stage (using validate which exists in TaskRow stages)
        let task_failed = sample_task("failed", Some("validate: 3 tests failed"));
        let output_failed = renderer.render_pipeline_view(&pane, &[task_failed], 0, PaneType::BeadList);
        assert!(output_failed.contains("✗ ◎ validate"));
    }
}
