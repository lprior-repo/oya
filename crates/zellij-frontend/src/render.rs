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
use crate::plugin::{TaskRow, stage_symbol_from_status};
use crate::spinner::SpinnerFrame;
use std::fmt::Write;
use std::time::SystemTime;
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

/// Calculate the current spinner frame based on system time.
///
/// # Preconditions
/// - SystemTime::now() returns a valid time (always true in practice)
///
/// # Postconditions
/// - Returns a valid SpinnerFrame (Frame0, Frame1, Frame2, or Frame3)
/// - Frame advances every 250ms for smooth animation
///
/// # Returns
/// The current spinner frame based on elapsed time since UNIX_EPOCH
#[must_use]
fn current_spinner_frame() -> SpinnerFrame {
    // Get current time since UNIX_EPOCH
    let duration_since_epoch = SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)
        .unwrap_or_else(|_| std::time::Duration::from_secs(0));

    // Convert to milliseconds and divide by 250ms per frame
    // This gives us 4 frames per second for smooth animation
    let frame_number = duration_since_epoch.as_millis() as usize / 250;

    SpinnerFrame::from_frame_number(frame_number)
}

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

        // Render each pane
        for pane in &layout.panes {
            let pane_content = match pane.pane_type {
                PaneType::TaskList => self.render_bead_list(layout, tasks, selected_index),
                PaneType::TaskDetail => {
                    if let Some(task) = tasks.get(selected_index) {
                        self.render_bead_detail(layout, task)
                    } else {
                        String::new()
                    }
                }
                PaneType::WorkflowGraph => self.render_workflow_graph(pane, focused_pane),
                PaneType::PipelineView => {
                    if let Some(task) = tasks.get(selected_index) {
                        self.render_pipeline_view(layout, task)
                    } else {
                        String::new()
                    }
                }
            };

            let rendered = self.render_pane(pane, &pane_content, focused_pane);
            output.push_str(&rendered);
        }

        // Render status bar
        let status = self.render_status_bar(focused_pane, status_message);
        output.push_str(&status);

        output
    }

    /// Render a single pane with border
    fn render_pane(&self, pane: &Pane, content: &str, focused_pane: PaneType) -> String {
        let is_focused = pane.pane_type == focused_pane;
        let border_color = if is_focused { style::border_focused() } else { style::border_normal() };
        let width = pane.width;

        let mut output = String::new();

        // Top border with title
        let title = if is_focused {
            format!(" {} ", pane.title)
        } else {
            format!(" {} ", pane.title)
        };
        output.push_str(&border_color);
        output.push_str(&self.render_top_border(width, &title));

        // Content lines
        for line in content.lines() {
            output.push_str(&border_color);
            output.push_str("│");
            output.push_str(line);
            // Pad to width
            let line_len = line.chars().count();
            if line_len < width.saturating_sub(2) {
                for _ in 0..(width.saturating_sub(2).saturating_sub(line_len)) {
                    output.push(' ');
                }
            }
            output.push_str("│\n");
        }

        // Bottom border
        output.push_str(&self.render_bottom_border(width));
        output.push_str("\x1b[0m"); // Reset colors

        output
    }

    /// Render top border with title
    fn render_top_border(&self, width: usize, title: &str) -> String {
        let title_len = title.chars().count();
        let remaining = width.saturating_sub(2).saturating_sub(title_len);

        let mut border = String::from("┌");
        border.push_str(title);

        for _ in 0..remaining {
            border.push('─');
        }

        border.push_str("┐\n");
        border
    }

    /// Render bottom border
    fn render_bottom_border(&self, width: usize) -> String {
        let mut border = String::from("└");

        for _ in 0..width.saturating_sub(2) {
            border.push('─');
        }

        border.push_str("┘");
        border
    }

    /// Render task list pane
    fn render_bead_list(
        &self,
        layout: &Layout,
        tasks: &[TaskRow],
        selected_index: usize,
    ) -> String {
        let mut output = String::new();

        for (i, task) in tasks.iter().enumerate() {
            let is_selected = i == selected_index;

            // Selection indicator
            if is_selected {
                output.push_str(&style::selected());
                output.push_str("►");
            } else {
                output.push_str(" ");
            }

            // ID (truncated if needed)
            let id = truncate(&task.id, 12);
            output.push_str(&id);

            // Padding
            for _ in 0..(14_usize.saturating_sub(id.chars().count())) {
                output.push(' ');
            }

            // Stage symbol and status
            let (stage, status_info, completed) = stage_status(task);

            if let Some(s) = stage {
                output.push_str(&s);
                for _ in 0..(16_usize.saturating_sub(s.chars().count())) {
                    output.push(' ');
                }
            } else if let Some(info) = status_info {
                output.push_str(&info);
                for _ in 0..(16_usize.saturating_sub(info.chars().count())) {
                    output.push(' ');
                }
            } else {
                for _ in 0..16 {
                    output.push(' ');
                }
            }

            // Title
            let title = truncate(&task.title, layout.width.saturating_sub(40));
            output.push_str(&title);

            // Reset colors
            output.push_str("\x1b[0m");

            // Progress bar if in progress
            if task.status == "in_progress" && !completed {
                let progress = stage_progress(
                    task.stage_index,
                    task.running_stage,
                    task.failed_stage,
                    completed,
                );
                let bar = render_progress_bar(progress, 10);
                output.push_str(&bar);
            }

            output.push('\n');
        }

        output
    }

    /// Render task detail pane
    fn render_bead_detail(&self, _layout: &Layout, task: &TaskRow) -> String {
        let mut output = String::new();

        output.push_str(&style::header());
        output.push_str(&task.title);
        output.push_str("\n\n");

        output.push_str(&style::label());
        output.push_str("ID:       ");
        output.push_str(&style::text());
        output.push_str(&task.id);
        output.push('\n');

        output.push_str(&style::label());
        output.push_str("Status:   ");
        output.push_str(&style::text());
        output.push_str(&task.status);
        output.push('\n');

        if let Some(ref stage) = task.stage {
            output.push_str(&style::label());
            output.push_str("Stage:    ");
            output.push_str(&style::text());
            output.push_str(stage);
            output.push('\n');
        }

        if let Some(ref detail) = task.stage_detail {
            output.push_str(&style::label());
            output.push_str("Detail:   ");
            output.push_str(&style::text());
            output.push_str(detail);
            output.push('\n');
        }

        // Stage pipeline
        output.push_str("\n");
        output.push_str(&style::header());
        output.push_str("Pipeline:\n");

        for (i, stage_name) in task
            .pipeline_stages
            .iter()
            .enumerate()
        {
            let symbol = stage_symbol(stage_name);
            let progress = stage_progress(
                i,
                task.running_stage,
                task.failed_stage,
                task.status == "passed" || task.status == "integrated",
            );

            output.push_str(&style::text());
            output.push_str("  ");
            output.push_str(symbol);
            output.push(' ');
            output.push_str(stage_name);

            // Progress bar
            let bar = render_progress_bar(progress, 15);
            output.push_str(&bar);
            output.push('\n');
        }

        output
    }

    /// Render pipeline view pane
    fn render_pipeline_view(&self, layout: &Layout, task: &TaskRow) -> String {
        let mut output = String::new();

        output.push_str(&style::header());
        output.push_str("Pipeline: ");
        output.push_str(&task.title);
        output.push_str("\n\n");

        for (i, stage_name) in task.pipeline_stages.iter().enumerate() {
            let symbol = stage_symbol(stage_name);
            let progress = stage_progress(
                i,
                task.running_stage,
                task.failed_stage,
                task.status == "passed" || task.status == "integrated",
            );

            // Stage name
            output.push_str(&style::text());
            output.push_str(symbol);
            output.push(' ');
            output.push_str(stage_name);

            // Progress bar
            let bar = render_progress_bar(progress, layout.width.saturating_sub(20));
            output.push_str(&bar);
            output.push('\n');
        }

        output
    }

    /// Render workflow graph pane (DAG visualization)
    fn render_workflow_graph(&self, pane: &Pane, focused_pane: PaneType) -> String {
        let is_focused = pane.pane_type == focused_pane;
        let mut output = String::new();

        if is_focused {
            output.push_str(&style::header());
            output.push_str("Workflow Dependency Graph\n");
            output.push_str(&style::text());
            output.push_str("(Horizontal DAG visualization)\n\n");
            output.push_str(&self.render_horizontal_dag());
        } else {
            output.push_str("Press Enter to view graph");
        }

        output
    }

    /// Render a horizontal DAG (left-to-right flow)
    fn render_horizontal_dag(&self) -> String {
        let mut output = String::new();

        output.push_str("┌─────────┐     ┌─────────┐     ┌─────────┐\n");
        output.push_str("│ src-abc │ ──▶ │ src-def │ ──▶ │ src-ghi │\n");
        output.push_str("└─────────┘     └─────────┘     └─────────┘\n");
        output.push_str("                   │\n");
        output.push_str("                   ▼\n");
        output.push_str("                ┌─────────┐\n");
        output.push_str("                │ src-jkl │\n");
        output.push_str("                └─────────┘\n");

        output
    }

    /// Render status bar at bottom of screen
    fn render_status_bar(&self, focused_pane: PaneType, status_message: Option<&str>) -> String {
        let mut output = String::new();

        output.push_str(&style::border_normal());
        output.push_str("┌");

        for _ in 0..78 {
            output.push('─');
        }

        output.push_str("┐\n");
        output.push_str("│");

        // Focus indicator
        let focus_text = match focused_pane {
            PaneType::TaskList => "Tasks",
            PaneType::TaskDetail => "Detail",
            PaneType::WorkflowGraph => "Graph",
            PaneType::PipelineView => "Pipeline",
        };
        output.push_str(&format!(" Focus: {:<8} ", focus_text));

        // Status message
        if let Some(msg) = status_message {
            output.push_str(msg);
        } else {
            output.push_str("↑↓: navigate | Enter: focus | ?: help");
        }

        // Pad to width
        let current_len = output.chars().count();
        for _ in 0..(80_usize.saturating_sub(current_len).saturating_sub(1)) {
            output.push(' ');
        }

        output.push_str("│\n");
        output.push_str("└");

        for _ in 0..78 {
            output.push('─');
        }

        output.push_str("┘\x1b[0m\n");

        output
    }

    /// Render help overlay
    ///
    /// # Arguments
    ///
    /// * `rows` - Terminal rows
    /// * `cols` - Terminal columns
    ///
    /// # Returns
    ///
    /// Help overlay content or error if terminal too small
    ///
    /// # Errors
    ///
    /// Returns `Err(HelpOverlayError::TerminalTooSmall)` if terminal is below minimum size
    pub fn render_help_overlay(
        &self,
        rows: usize,
        cols: usize,
    ) -> HelpOverlayResult<String> {
        const MIN_ROWS: usize = 10;
        const MIN_COLS: usize = 40;

        if rows < MIN_ROWS || cols < MIN_COLS {
            return Err(HelpOverlayError::TerminalTooSmall { rows, cols });
        }

        let mut output = String::new();

        // Overlay border
        let width = cols.min(80);
        let height = rows.min(25);

        output.push_str(&style::overlay());
        output.push_str(&self.render_overlay_top_border(width, " Help "));

        // Help content
        let content = [
            "Navigation:",
            "  ↑/k    Move up",
            "  ↓/j    Move down",
            "  Enter  Focus pane",
            "",
            "Actions:",
            "  ?      Show/hide help",
            "  q      Quit",
            "",
            "Pane Types:",
            "  Tasks    - Task list",
            "  Detail   - Task details",
            "  Graph    - Workflow DAG",
            "  Pipeline - Stage pipeline",
        ];

        let content_height = content.len();
        let padding_top = height.saturating_sub(content_height + 4) / 2;
        let padding_bottom = height.saturating_sub(content_height + 4 + padding_top);

        // Top padding
        for _ in 0..padding_top {
            output.push_str(&style::overlay());
            output.push_str("│");
            for _ in 0..width.saturating_sub(2) {
                output.push(' ');
            }
            output.push_str("│\n");
        }

        // Content
        for line in content {
            output.push_str(&style::overlay());
            output.push_str("│ ");
            output.push_str(line);
            for _ in 0..width.saturating_sub(2 + line.chars().count()) {
                output.push(' ');
            }
            output.push_str("│\n");
        }

        // Bottom padding
        for _ in 0..padding_bottom {
            output.push_str(&style::overlay());
            output.push_str("│");
            for _ in 0..width.saturating_sub(2) {
                output.push(' ');
            }
            output.push_str("│\n");
        }

        output.push_str(&self.render_overlay_bottom_border(width));
        output.push_str("\x1b[0m");

        Ok(output)
    }

    /// Render overlay top border
    fn render_overlay_top_border(&self, width: usize, title: &str) -> String {
        let title_len = title.chars().count();
        let remaining = width.saturating_sub(2).saturating_sub(title_len);

        let mut border = String::from("╔");
        border.push_str(title);

        for _ in 0..remaining {
            border.push('═');
        }

        border.push_str("╗\n");
        border
    }

    /// Render overlay bottom border
    fn render_overlay_bottom_border(&self, width: usize) -> String {
        let mut border = String::from("╚");

        for _ in 0..width.saturating_sub(2) {
            border.push('═');
        }

        border.push_str("╝");
        border
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

/// Wrap text to fit width, breaking at word boundaries.
///
/// Uses functional fold pattern with zero mut and no unwrapping.
fn textwrap(text: &str, width: usize) -> Vec<String> {
    /// Accumulator state for the fold
    #[derive(Debug, Clone)]
    struct Accumulator {
        completed_lines: Vec<String>,
        current_line: String,
        current_length: usize,
    }

    /// Append a line to completed_lines
    fn push_to(mut lines: Vec<String>, line: String) -> Vec<String> {
        lines.push(line);
        lines
    }

    /// Create accumulator with finalized current line and new current line
    fn with_finalized(acc: Accumulator, new_line: String, new_length: usize) -> Accumulator {
        Accumulator {
            completed_lines: push_to(acc.completed_lines, acc.current_line),
            current_line: new_line,
            current_length: new_length,
        }
    }

    /// Add a word/chunk to the accumulator
    fn add_chunk(acc: Accumulator, chunk: &str, width: usize) -> Accumulator {
        let chunk_len = chunk.chars().count();

        match acc.current_length {
            0 => Accumulator {
                completed_lines: acc.completed_lines,
                current_line: chunk.to_string(),
                current_length: chunk_len,
            },
            len if len + 1 + chunk_len <= width => Accumulator {
                completed_lines: acc.completed_lines,
                current_line: format!("{} {}", acc.current_line, chunk),
                current_length: len + 1 + chunk_len,
            },
            _ => with_finalized(acc, chunk.to_string(), chunk_len),
        }
    }

    /// Split a word into chunks if it exceeds width
    fn chunk_word(word: &str, width: usize) -> Vec<String> {
        word.chars()
            .collect::<Vec<_>>()
            .chunks(width)
            .map(|c| c.iter().collect::<String>())
            .collect()
    }

    /// Process a single word, handling chunking if needed
    fn process_word(acc: Accumulator, word: &str, width: usize) -> Accumulator {
        let word_len = word.chars().count();
        match word_len > width {
            true => {
                let chunks = chunk_word(word, width);
                chunks.into_iter().fold(acc, |a, c| add_chunk(a, &c, width))
            }
            false => add_chunk(acc, word, width),
        }
    }

    /// Extract final lines from accumulator
    fn finalize(acc: Accumulator) -> Vec<String> {
        match acc.current_line.is_empty() {
            true => acc.completed_lines,
            false => push_to(acc.completed_lines, acc.current_line),
        }
    }

    let initial = Accumulator {
        completed_lines: Vec::new(),
        current_line: String::new(),
        current_length: 0,
    };

    let result = text
        .split_whitespace()
        .fold(initial, |acc, word| process_word(acc, word, width));

    finalize(result)
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
        if stage_index <= index { 1.0 } else { 0.0 }
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

    let mut bar = String::new();
    bar.push('[');

    // Filled portion
    for _ in 0..filled {
        bar.push('█');
    }

    // Empty portion
    for _ in 0..empty {
        bar.push('░');
    }

    bar.push(']');
    bar.push_str(&format!(" {}%", percentage));

    bar
}

#[cfg(test)]
mod tests {
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
        assert!(border.ends_with("┘"));
    }

    #[test]
    fn test_textwrap_single_word() {
        let lines = textwrap("hello", 10);
        assert_eq!(lines.len(), 1);
        assert_eq!(lines[0], "hello");
    }

    #[test]
    fn test_textwrap_empty() {
        let lines = textwrap("", 10);
        assert!(lines.is_empty());
    }

    #[test]
    fn test_textwrap_long_word() {
        let lines = textwrap("supercalifragilisticexpialidocious", 10);
        assert!(!lines.is_empty());
        assert!(lines[0].len() <= 10);
    }

    #[test]
    fn test_stage_progress_none_started() {
        let progress = stage_progress(0, None, None, false);
        assert_eq!(progress, 0.0);
    }

    #[test]
    fn test_stage_progress_completed() {
        let progress = stage_progress(0, None, None, true);
        assert_eq!(progress, 1.0);
    }

    #[test]
    fn test_stage_progress_running_current() {
        let progress = stage_progress(2, Some(2), None, false);
        assert_eq!(progress, 0.5);
    }

    #[test]
    fn test_stage_progress_running_past() {
        let progress = stage_progress(1, Some(2), None, false);
        assert_eq!(progress, 1.0);
    }

    #[test]
    fn test_stage_progress_running_future() {
        let progress = stage_progress(3, Some(2), None, false);
        assert_eq!(progress, 0.0);
    }

    #[test]
    fn test_stage_progress_failed_before() {
        let progress = stage_progress(0, None, Some(1), false);
        assert_eq!(progress, 1.0);
    }

    #[test]
    fn test_stage_progress_failed_at() {
        let progress = stage_progress(1, None, Some(1), false);
        assert_eq!(progress, 1.0);
    }

    #[test]
    fn test_stage_progress_failed_after() {
        let progress = stage_progress(2, None, Some(1), false);
        assert_eq!(progress, 0.0);
    }
}
