// Render module - Terminal rendering with ANSI box-drawing characters
//
// This module provides rendering functionality for the OYA UI plugin,
// including:
// - ANSI box-drawing characters for pane borders
// - Text layout and wrapping
// - Color and styling support
// - Focused pane highlighting
// - Help overlay rendering

use crate::layout::{Layout, Pane, PaneType};
use crate::plugin::{TaskRow, StageState};
use thiserror::Error;

// Style helper functions using functional patterns
mod style_helpers {
    use crate::components;

    #[must_use]
    pub const fn selected() -> &'static str {
        components::selected()
    }

    #[must_use]
    pub const fn header() -> &'static str {
        components::header()
    }

    #[must_use]
    pub const fn label() -> &'static str {
        components::label()
    }

    #[must_use]
    pub const fn text() -> &'static str {
        components::text()
    }

    #[must_use]
    pub const fn overlay() -> &'static str {
        components::overlay()
    }

    #[must_use]
    pub const fn border_normal() -> &'static str {
        components::border_normal()
    }

    #[must_use]
    pub const fn border_focused() -> &'static str {
        components::border_focused()
    }
}

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
        // Render panes using functional fold pattern
        let panes_rendered = layout
            .panes()
            .iter()
            .fold(String::new(), |mut acc: String, pane| {
                let pane_content = match pane.pane_type {
                    PaneType::BeadList => self.render_bead_list(layout, tasks, selected_index),
                    PaneType::BeadDetail => {
                        tasks
                            .get(selected_index)
                            .map_or_else(String::new, |task| self.render_bead_detail(layout, task))
                    }
                    PaneType::WorkflowGraph => self.render_workflow_graph(pane, focused_pane),
                    PaneType::PipelineView => {
                        tasks
                            .get(selected_index)
                            .map_or_else(String::new, |task| self.render_pipeline_view(pane, task))
                    }
                };

                let rendered = self.render_pane(pane, &pane_content, focused_pane);
                acc.push_str(&rendered);
                acc
            });

        // Add status bar
        let status = self.render_status_bar(focused_pane, status_message);
        format!("{panes_rendered}{status}")
    }

    /// Render a single pane with border
    fn render_pane(&self, pane: &Pane, content: &str, focused_pane: PaneType) -> String {
        let is_focused = pane.pane_type == focused_pane;
        let border_color = if is_focused { style_helpers::border_focused() } else { style_helpers::border_normal() };
        let width = pane.width;
        let title = format!(" {} ", pane.title);

        // Render top border
        let top_border = self.render_top_border(width, &title);

        // Render content lines using functional fold pattern
        let content_lines = content.lines().fold(String::new(), |mut acc, line| {
            let line_len = line.chars().count();
            let padding = " ".repeat(width.saturating_sub(2).saturating_sub(line_len));
            acc.push_str(border_color);
            acc.push('│');
            acc.push_str(line);
            acc.push_str(&padding);
            acc.push_str("│\n");
            acc
        });

        // Assemble complete pane
        let bottom_border = self.render_bottom_border(width);
        format!("{border_color}{top_border}{content_lines}{bottom_border}\x1b[0m")
    }

    /// Render top border with title
    fn render_top_border(&self, width: usize, title: &str) -> String {
        let title_len = title.chars().count();
        let remaining = width.saturating_sub(2).saturating_sub(title_len);

        // Functional pattern: use repeat instead of loop
        let border_line = "─".repeat(remaining);

        format!("┌{title}{border_line}┐\n")
    }

    /// Render bottom border
    fn render_bottom_border(&self, width: usize) -> String {
        let border_line = "─".repeat(width.saturating_sub(2));
        format!("└{border_line}┘")
    }

    /// Render task list pane
    fn render_bead_list(
        &self,
        layout: &Layout,
        tasks: &[TaskRow],
        selected_index: usize,
    ) -> String {
        // Get the bead list pane for width calculation
        let pane_width = layout
            .get_pane(PaneType::BeadList)
            .map_or(40, |p| p.width);

        // Render each task line using functional fold pattern
        tasks
            .iter()
            .enumerate()
            .fold(String::new(), |mut acc, (i, task)| {
                let is_selected = i == selected_index;

                // Selection indicator
                let indicator = if is_selected {
                    format!("{}►", style_helpers::selected())
                } else {
                    " ".to_string()
                };
                acc.push_str(&indicator);

                // Slug (truncated if needed)
                let slug = truncate(&task.slug, 12);
                let slug_padding = " ".repeat(14_usize.saturating_sub(slug.chars().count()));
                acc.push_str(&slug);
                acc.push_str(&slug_padding);

                // Stage symbol - use map_or for default
                let symbol = task
                    .stage
                    .as_ref()
                    .and_then(|s| s.split(':').next())
                    .map_or("○", stage_symbol);
                let symbol_padding = " ".repeat(16_usize.saturating_sub(symbol.chars().count()));
                acc.push_str(symbol);
                acc.push_str(&symbol_padding);

                // Slug as title
                let title = truncate(&task.slug, pane_width.saturating_sub(40));
                acc.push_str(&title);

                // Reset colors
                acc.push_str("\x1b[0m");

                // Progress bar if in progress
                if task.status == "in_progress" {
                    let (running_stage, failed_stage, completed) = get_stage_info(task);
                    let progress = calculate_stage_progress(&task.stages, running_stage, failed_stage, completed);
                    let bar = render_progress_bar(progress, 10);
                    acc.push_str(&bar);
                }

                acc.push('\n');
                acc
            })
    }

    /// Render task detail pane
    fn render_bead_detail(&self, _layout: &Layout, task: &TaskRow) -> String {
        // Header with slug
        let header = format!("{}{}\n\n", style_helpers::header(), task.slug);

        // Build field lines using functional pattern
        let fields = [
            ("Status", &task.status),
            ("Priority", &task.priority),
            ("Language", &task.language),
            ("Branch", &task.branch),
        ];

        let field_lines = fields.iter().fold(String::new(), |mut acc, (label, value)| {
            acc.push_str(style_helpers::label());
            acc.push_str(&format!("{label:<9} "));
            acc.push_str(style_helpers::text());
            acc.push_str(value);
            acc.push('\n');
            acc
        });

        // Stage line if present
        let stage_line = task
            .stage
            .as_ref()
            .map_or_else(String::new, |stage| {
                format!("{}Stage:    {}{}\n", style_helpers::label(), style_helpers::text(), stage)
            });

        // Pipeline header
        let pipeline_header = format!("\n{}Pipeline:\n", style_helpers::header());

        // Calculate stage info once
        let (running_stage, failed_stage, completed) = get_stage_info(task);

        // Render pipeline stages using functional fold
        let pipeline_stages = task
            .stages
            .iter()
            .fold(String::new(), |mut acc, stage_info| {
                let progress = calculate_stage_progress(&task.stages, running_stage, failed_stage, completed);
                let bar = render_progress_bar(progress, 15);
                acc.push_str(style_helpers::text());
                acc.push_str("  ");
                acc.push_str(stage_info.symbol());
                acc.push(' ');
                acc.push_str(&stage_info.name);
                acc.push_str(&bar);
                acc.push('\n');
                acc
            });

        // Assemble complete detail view
        format!("{header}{field_lines}{stage_line}{pipeline_header}{pipeline_stages}")
    }

    /// Render pipeline view pane
    fn render_pipeline_view(&self, pane: &Pane, task: &TaskRow) -> String {
        let header = format!("{}Pipeline: {}\n\n", style_helpers::header(), task.slug);

        // Calculate stage info once
        let (running_stage, failed_stage, completed) = get_stage_info(task);

        // Render pipeline stages using functional fold
        let stages = task
            .stages
            .iter()
            .fold(String::new(), |mut acc, stage_info| {
                let progress = calculate_stage_progress(&task.stages, running_stage, failed_stage, completed);
                let bar = render_progress_bar(progress, pane.width.saturating_sub(20));
                acc.push_str(style_helpers::text());
                acc.push_str(stage_info.symbol());
                acc.push(' ');
                acc.push_str(&stage_info.name);
                acc.push_str(&bar);
                acc.push('\n');
                acc
            });

        format!("{header}{stages}")
    }

    /// Render workflow graph pane (DAG visualization)
    fn render_workflow_graph(&self, pane: &Pane, focused_pane: PaneType) -> String {
        let is_focused = pane.pane_type == focused_pane;

        if is_focused {
            let dag = self.render_horizontal_dag();
            format!(
                "{}Workflow Dependency Graph\n{}(Horizontal DAG visualization)\n\n{dag}",
                style_helpers::header(),
                style_helpers::text()
            )
        } else {
            "Press Enter to view graph".to_string()
        }
    }

    /// Render a horizontal DAG (left-to-right flow)
    fn render_horizontal_dag(&self) -> String {
        // Static DAG visualization using multiline string
        concat!(
            "┌─────────┐     ┌─────────┐     ┌─────────┐\n",
            "│ src-abc │ ──▶ │ src-def │ ──▶ │ src-ghi │\n",
            "└─────────┘     └─────────┘     └─────────┘\n",
            "                   │\n",
            "                   ▼\n",
            "                ┌─────────┐\n",
            "                │ src-jkl │\n",
            "                └─────────┘\n",
        )
        .to_string()
    }

    /// Render status bar at bottom of screen
    fn render_status_bar(&self, focused_pane: PaneType, status_message: Option<&str>) -> String {
        let border_line = "─".repeat(78);

        // Focus indicator
        let focus_text = match focused_pane {
            PaneType::BeadList => "Beads",
            PaneType::BeadDetail => "Details",
            PaneType::WorkflowGraph => "Graph",
            PaneType::PipelineView => "Pipeline",
        };

        // Status message using map_or for default
        let msg = status_message.map_or("↑↓: navigate | Enter: focus | ?: help", |s| s);

        // Build content line
        let content = format!(" Focus: {:<8} {msg}", focus_text);
        let content_len = content.chars().count();
        let padding = " ".repeat(80_usize.saturating_sub(content_len).saturating_sub(1));

        // Assemble complete status bar
        format!(
            "{}┌{border_line}┐\n│{content}{padding}│\n└{border_line}┘\x1b[0m\n",
            style_helpers::border_normal()
        )
    }

    /// Render help overlay
    ///
    /// # Arguments
    ///
    /// * `rows` - Terminal rows
    /// * `cols` - Terminal columns
    /// * `keybindings` - Keybindings for current pane
    /// * `focused_pane` - Currently focused pane type
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
        keybindings: &[(char, &str)],
        focused_pane: PaneType,
    ) -> HelpOverlayResult<String> {
        const MIN_ROWS: usize = 10;
        const MIN_COLS: usize = 40;

        if rows < MIN_ROWS || cols < MIN_COLS {
            return Err(HelpOverlayError::TerminalTooSmall { rows, cols });
        }

        let width = cols.min(80);
        let height = rows.min(25);

        // Render top border
        let top_border = self.render_overlay_top_border(width, " Help ");

        // Focus indicator
        let focus_text = match focused_pane {
            PaneType::BeadList => "Bead List",
            PaneType::BeadDetail => "Bead Details",
            PaneType::WorkflowGraph => "Workflow Graph",
            PaneType::PipelineView => "Pipeline View",
        };

        // Build all content lines using functional patterns
        // Create iterator of static content lines
        let static_lines = [
            "Global Keys:",
            "  ?      Show/hide this help",
            "  q      Quit",
            "  Tab    Switch focus between panes",
            "",
        ]
        .iter()
        .map(|s| (*s).to_string());

        // Create iterator of focus section
        let focus_lines = [
            format!("Current Focus: {focus_text}"),
            "".to_string(),
            "Keybindings for current pane:".to_string(),
        ]
        .into_iter();

        // Combine all lines: static + focus + keybindings
        let all_lines: Vec<String> = static_lines
            .chain(focus_lines)
            .chain(keybindings.iter().map(|(key, desc)| format!("  {key}      {desc}")))
            .collect();

        let content_height = all_lines.len();
        let padding_top = height.saturating_sub(content_height.saturating_add(4)) / 2;
        let padding_bottom = height.saturating_sub(content_height.saturating_add(4).saturating_add(padding_top));
        let inner_width = width.saturating_sub(2);
        let overlay_style = style_helpers::overlay();

        // Helper to render padding lines using repeat
        let padding_line = format!("{overlay_style}│{}│\n", " ".repeat(inner_width));
        let top_padding = padding_line.repeat(padding_top);

        // Render content lines using functional fold pattern
        let content_lines = all_lines.iter().fold(String::new(), |mut acc, line| {
            let padding = " ".repeat(width.saturating_sub(2_usize.saturating_add(line.chars().count())));
            acc.push_str(overlay_style);
            acc.push_str("│ ");
            acc.push_str(line);
            acc.push_str(&padding);
            acc.push_str("│\n");
            acc
        });

        let bottom_padding = padding_line.repeat(padding_bottom);

        // Assemble complete overlay
        let bottom_border = self.render_overlay_bottom_border(width);
        Ok(format!(
            "{overlay_style}{top_border}{top_padding}{content_lines}{bottom_padding}{bottom_border}\x1b[0m"
        ))
    }

    /// Render overlay top border
    fn render_overlay_top_border(&self, width: usize, title: &str) -> String {
        let title_len = title.chars().count();
        let remaining = width.saturating_sub(2).saturating_sub(title_len);
        let border_line = "═".repeat(remaining);

        format!("╔{title}{border_line}╗\n")
    }

    /// Render overlay bottom border
    fn render_overlay_bottom_border(&self, width: usize) -> String {
        let border_line = "═".repeat(width.saturating_sub(2));
        format!("╚{border_line}╝")
    }
}

impl Default for Renderer {
    fn default() -> Self {
        Self::new()
    }
}

/// Truncate text to fit width using functional patterns.
///
/// Uses char_indices for byte-efficient truncation without collecting into Vec.
#[must_use]
fn truncate(text: &str, width: usize) -> String {
    // Find the byte position at which to truncate
    let byte_pos = text
        .char_indices()
        .map(|(pos, _)| pos)
        .nth(width);

    match byte_pos {
        None => text.to_string(), // Text fits within width
        Some(pos) if width > 3 && pos < text.len() => {
            // Truncate and add ellipsis
            format!("{}...", &text[..pos])
        }
        Some(_) => "...".to_string(), // Width too small for meaningful truncation
    }
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

fn render_progress_bar(progress: f32, width: usize) -> String {
    let clamped = progress.clamp(0.0, 1.0);
    let filled = ((clamped * width as f32).round() as usize).min(width);
    let empty = width.saturating_sub(filled);
    let percentage = (clamped * 100.0).round() as usize;

    // Build bar using functional patterns: repeat chars and collect
    let filled_part = "█".repeat(filled);
    let empty_part = "░".repeat(empty);

    format!("[{}{}] {}%", filled_part, empty_part, percentage)
}

/// Extract stage lifecycle information from a task.
///
/// Returns (running_stage_index, failed_stage_index, is_completed) tuple.
/// All indices are Option<usize> representing positions in the stages vector.
/// Uses functional patterns: find_position with iterator combinators.
#[must_use]
fn get_stage_info(task: &TaskRow) -> (Option<usize>, Option<usize>, bool) {
    let is_completed = matches!(task.status.as_str(), "passed" | "integrated");

    // Use iterator find_position for running stage
    let running_stage = task
        .stages
        .iter()
        .position(|s| matches!(s.state, StageState::Running));

    // Use iterator find_position for failed stage
    let failed_stage = task
        .stages
        .iter()
        .position(|s| matches!(s.state, StageState::Failed));

    (running_stage, failed_stage, is_completed)
}

/// Calculate progress for a specific stage in the pipeline.
///
/// Uses functional patterns with match expressions instead of imperative logic.
/// Returns f32 between 0.0 and 1.0 representing completion percentage.
#[must_use]
fn calculate_stage_progress(
    stages: &[crate::plugin::StageInfo],
    running_stage: Option<usize>,
    failed_stage: Option<usize>,
    is_completed: bool,
) -> f32 {
    // Total number of stages for percentage calculation
    let total_stages = stages.len();

    // Calculate completed count using iterator combinators
    let completed_count = stages
        .iter()
        .filter(|s| matches!(s.state, StageState::Completed))
        .count();

    // Functional pattern: nested map_or_else for clean composition
    let base_progress = if is_completed {
        1.0
    } else if let Some(failed_idx) = failed_stage {
        // All stages up to and including failed are "done" (even if failed)
        failed_idx.saturating_add(1) as f32 / total_stages.max(1) as f32
    } else if running_stage.is_some() {
        // Completed stages + 0.5 for the running stage
        (completed_count as f32 + 0.5) / total_stages.max(1) as f32
    } else {
        // Only completed stages contribute
        completed_count as f32 / total_stages.max(1) as f32
    };

    base_progress.clamp(0.0, 1.0)
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
}
