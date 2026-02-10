//! Real-time workflow graph visualization for terminal UI.
//!
//! This module provides ASCII/unicode art rendering of workflow DAGs with:
//! - Color-coded task status
//! - Progress bars
//! - Keyboard navigation
//! - Critical path highlighting
//!
//! # Design Principles
//!
//! - **Zero unwrap**: All errors handled explicitly with Result types
//! - **Functional core**: Pure rendering functions
//! - **Railway-oriented**: Error propagation with context
//! - **Terminal safety**: Proper cursor restoration

#![deny(clippy::unwrap_used)]
#![deny(clippy::expect_used)]
#![deny(clippy::panic)]
#![warn(clippy::pedantic)]
#![warn(clippy::nursery)]
#![forbid(unsafe_code)]

use crate::Workflow;
use crate::execution::{TaskExecutionStatus, WorkflowState};
use std::collections::{HashMap, HashSet};
use thiserror::Error;

/// Visualization errors.
#[derive(Debug, Error)]
pub enum VisualizationError {
    /// Terminal dimensions are too small.
    #[error(
        "Terminal too small: minimum {min_width}x{min_height}, actual {actual_width}x{actual_height}"
    )]
    TerminalTooSmall {
        min_width: usize,
        min_height: usize,
        actual_width: usize,
        actual_height: usize,
    },

    /// Workflow graph contains a cycle.
    #[error("Workflow contains cycle: {cycle:?}")]
    CyclicGraph { cycle: Vec<String> },

    /// Render operation failed.
    #[error("Render failed: {cause}")]
    RenderFailed { cause: String },

    /// `EventBus` disconnected.
    #[error("EventBus disconnected")]
    EventBusDisconnected,

    /// Invalid task status.
    #[error("Invalid task status for task {task_id}")]
    InvalidTaskStatus { task_id: String },
}

/// Color codes for terminal output.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Color {
    /// Yellow for pending.
    Yellow,
    /// Blue for in progress.
    Blue,
    /// Green for completed.
    Green,
    /// Red for failed.
    Red,
    /// Gray for rolled back.
    Gray,
    /// White for normal text.
    White,
    /// Bold for emphasis.
    Bold,
    /// Reset to default.
    Reset,
}

impl Color {
    /// Get ANSI escape code for this color.
    #[must_use]
    pub const fn ansi_code(self) -> &'static str {
        match self {
            Self::Yellow => "\x1b[33m",
            Self::Blue => "\x1b[34m",
            Self::Green => "\x1b[32m",
            Self::Red => "\x1b[31m",
            Self::Gray => "\x1b[90m",
            Self::White => "\x1b[37m",
            Self::Bold => "\x1b[1m",
            Self::Reset => "\x1b[0m",
        }
    }

    /// Get color for task status.
    #[must_use]
    pub const fn for_status(status: &TaskExecutionStatus) -> Self {
        match status {
            TaskExecutionStatus::Pending => Self::Yellow,
            TaskExecutionStatus::InProgress => Self::Blue,
            TaskExecutionStatus::Completed => Self::Green,
            TaskExecutionStatus::Failed { .. } => Self::Red,
            TaskExecutionStatus::RolledBack => Self::Gray,
            TaskExecutionStatus::Cancelled => Self::Gray,
        }
    }
}

/// Rendered visualization data.
#[derive(Clone, Debug)]
pub struct RenderedGraph {
    /// Lines of rendered output.
    pub lines: Vec<String>,
    /// Width of the rendered graph.
    pub width: usize,
    /// Height of the rendered graph.
    pub height: usize,
}

/// Action to take in response to keyboard input.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum InputAction {
    /// Move focus to a different task.
    FocusTask(String),
    /// Show detailed information about focused task.
    ShowDetails(String),
    /// Scroll the view up.
    ScrollUp(usize),
    /// Scroll the view down.
    ScrollDown(usize),
    /// Quit the visualization.
    Quit,
    /// No action (ignore input).
    None,
}

/// Keyboard key events.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Key {
    /// Up arrow or 'k' (vim).
    Up,
    /// Down arrow or 'j' (vim).
    Down,
    /// Left arrow or 'h' (vim).
    Left,
    /// Right arrow or 'l' (vim).
    Right,
    /// Enter key.
    Enter,
    /// 'q' key.
    Quit,
    /// 'd' key.
    Details,
    /// Escape key.
    Escape,
    /// Unknown key.
    Unknown(char),
}

/// Workflow visualization renderer.
pub struct WorkflowVisualization {
    /// Minimum terminal width required.
    min_width: usize,
    /// Minimum terminal height required.
    min_height: usize,
    /// Enable color output.
    enable_color: bool,
    /// Focused task for keyboard navigation.
    focused_task: Option<String>,
}

impl WorkflowVisualization {
    /// Create a new visualization renderer.
    #[must_use]
    pub const fn new() -> Self {
        Self {
            min_width: 80,
            min_height: 24,
            enable_color: true,
            focused_task: None,
        }
    }

    /// Set minimum terminal dimensions.
    #[must_use]
    pub const fn with_min_dimensions(mut self, width: usize, height: usize) -> Self {
        self.min_width = width;
        self.min_height = height;
        self
    }

    /// Disable color output.
    #[must_use]
    pub const fn without_color(mut self) -> Self {
        self.enable_color = false;
        self
    }

    /// Set the focused task for navigation.
    #[must_use]
    pub fn with_focus(mut self, task_id: Option<String>) -> Self {
        self.focused_task = task_id;
        self
    }

    /// Render workflow DAG as ASCII art.
    ///
    /// # Errors
    /// Returns `VisualizationError::TerminalTooSmall` if terminal is too small.
    /// Returns `VisualizationError::CyclicGraph` if workflow has cycles.
    pub fn render_dag(
        &self,
        workflow: &Workflow,
        state: &WorkflowState,
        term_width: usize,
        term_height: usize,
    ) -> Result<RenderedGraph, VisualizationError> {
        // Check terminal dimensions
        if term_width < self.min_width || term_height < self.min_height {
            return Err(VisualizationError::TerminalTooSmall {
                min_width: self.min_width,
                min_height: self.min_height,
                actual_width: term_width,
                actual_height: term_height,
            });
        }

        // Check for cycles
        let cycle = self.detect_cycle(workflow);
        if !cycle.is_empty() {
            return Err(VisualizationError::CyclicGraph { cycle });
        }

        // Calculate critical path
        let critical_path = self.calculate_critical_path(workflow);

        // Build level-based layout
        let levels = self.build_level_layout(workflow);

        // Render each level
        let mut lines = Vec::new();

        // Header
        lines.push(self.format_line(format!("Workflow: {}", workflow.name), Color::Bold));
        lines.push(self.format_line(format!("Tasks: {}", workflow.tasks.len()), Color::White));
        lines.push(String::new());

        // Render tasks by level
        for (level_idx, level) in levels.iter().enumerate() {
            let level_line = self.render_level(
                workflow,
                state,
                level,
                level_idx,
                &critical_path,
                term_width,
            )?;
            lines.extend(level_line);
            lines.push(String::new());
        }

        // Legend
        lines.push(self.format_line("Legend:".to_string(), Color::Bold));
        lines.push(self.format_line(
            format!(
                "{} Pending",
                self.symbol_for_status(&TaskExecutionStatus::Pending)
            ),
            Color::Yellow,
        ));
        lines.push(self.format_line(
            format!(
                "{} In Progress",
                self.symbol_for_status(&TaskExecutionStatus::InProgress)
            ),
            Color::Blue,
        ));
        lines.push(self.format_line(
            format!(
                "{} Completed",
                self.symbol_for_status(&TaskExecutionStatus::Completed)
            ),
            Color::Green,
        ));
        lines.push(self.format_line(
            format!(
                "{} Failed",
                self.symbol_for_status(&TaskExecutionStatus::Failed {
                    error: String::new()
                })
            ),
            Color::Red,
        ));
        lines.push(self.format_line("* Critical path task".to_string(), Color::Bold));

        let height = lines.len();
        let width = lines
            .iter()
            .map(std::string::String::len)
            .max()
            .unwrap_or(0);

        Ok(RenderedGraph {
            lines,
            width,
            height,
        })
    }

    /// Detect cycle in workflow graph.
    fn detect_cycle(&self, workflow: &Workflow) -> Vec<String> {
        let mut visited: HashSet<String> = HashSet::new();
        let mut rec_stack: HashSet<String> = HashSet::new();
        let mut path: Vec<String> = Vec::new();

        for task_id in workflow.tasks.keys() {
            if !visited.contains(task_id)
                && self.dfs_cycle_detect(workflow, task_id, &mut visited, &mut rec_stack, &mut path)
            {
                return path;
            }
        }

        vec![]
    }

    /// DFS helper for cycle detection.
    fn dfs_cycle_detect(
        &self,
        workflow: &Workflow,
        task_id: &str,
        visited: &mut HashSet<String>,
        rec_stack: &mut HashSet<String>,
        path: &mut Vec<String>,
    ) -> bool {
        visited.insert(task_id.to_string());
        rec_stack.insert(task_id.to_string());
        path.push(task_id.to_string());

        // Get tasks that depend on this task
        for (other_id, deps) in &workflow.dependencies {
            if deps.contains(task_id) {
                if !visited.contains(other_id) {
                    if self.dfs_cycle_detect(workflow, other_id, visited, rec_stack, path) {
                        return true;
                    }
                } else if rec_stack.contains(other_id) {
                    let cycle_start = path.iter().position(|id| id == other_id);
                    if let Some(start) = cycle_start {
                        *path = path[start..].to_vec();
                        path.push(other_id.clone());
                    }
                    return true;
                }
            }
        }

        rec_stack.remove(task_id);
        path.pop();
        false
    }

    /// Calculate critical path (longest path through DAG).
    fn calculate_critical_path(&self, workflow: &Workflow) -> HashSet<String> {
        // For simplicity, use longest path in terms of task count
        // A more sophisticated implementation would use task durations
        let mut longest_dist: HashMap<String, usize> = HashMap::new();
        let mut visited: HashSet<String> = HashSet::new();

        // Initialize distances
        for task_id in workflow.tasks.keys() {
            longest_dist.insert(task_id.clone(), 0);
        }

        // Find nodes with no dependencies (sources)
        let sources: Vec<String> = workflow
            .tasks
            .keys()
            .filter(|id| {
                workflow
                    .dependencies
                    .get(*id)
                    .is_none_or(std::collections::HashSet::is_empty)
            })
            .cloned()
            .collect();

        // DFS from each source to find longest path
        for source in sources {
            self.find_longest_path(workflow, &source, &mut longest_dist, &mut visited);
        }

        // Backtrack to find the critical path
        let max_dist = longest_dist.values().copied().max().unwrap_or(0);
        let mut critical_path = HashSet::new();

        if max_dist > 0 {
            // Find tasks on the longest path
            for (task_id, dist) in &longest_dist {
                if *dist == max_dist || *dist == max_dist - 1 {
                    critical_path.insert(task_id.clone());
                }
            }
        }

        critical_path
    }

    /// Find longest path from a node.
    fn find_longest_path(
        &self,
        workflow: &Workflow,
        task_id: &str,
        dist: &mut HashMap<String, usize>,
        visited: &mut HashSet<String>,
    ) {
        visited.insert(task_id.to_string());

        // Find dependent tasks
        for (other_id, deps) in &workflow.dependencies {
            if deps.contains(task_id) {
                let new_dist = dist.get(task_id).copied().unwrap_or(0) + 1;
                let current_dist = dist.get(other_id).copied().unwrap_or(0);

                if new_dist > current_dist {
                    dist.insert(other_id.clone(), new_dist);
                }

                if !visited.contains(other_id) {
                    self.find_longest_path(workflow, other_id, dist, visited);
                }
            }
        }

        visited.remove(task_id);
    }

    /// Build level-based layout for tasks.
    fn build_level_layout(&self, workflow: &Workflow) -> Vec<Vec<String>> {
        let mut levels: Vec<Vec<String>> = Vec::new();
        let mut assigned: HashSet<String> = HashSet::new();

        loop {
            // Find tasks that can be assigned to next level
            let ready: Vec<String> = workflow
                .tasks
                .keys()
                .filter(|id| {
                    !assigned.contains(id.as_str())
                        && workflow.dependencies.get(id.as_str()).is_none_or(|deps| {
                            deps.iter().all(|dep| assigned.contains(dep.as_str()))
                        })
                })
                .cloned()
                .collect();

            if ready.is_empty() {
                break;
            }

            ready.iter().for_each(|id| {
                assigned.insert(id.clone());
            });

            levels.push(ready);
        }

        levels
    }

    /// Render a single level of tasks.
    fn render_level(
        &self,
        workflow: &Workflow,
        state: &WorkflowState,
        level: &[String],
        level_idx: usize,
        critical_path: &HashSet<String>,
        _term_width: usize,
    ) -> Result<Vec<String>, VisualizationError> {
        let mut lines = Vec::new();

        if level.is_empty() {
            return Ok(lines);
        }

        // Render each task in the level
        for task_id in level {
            let task = workflow.get_task(task_id).ok_or_else(|| {
                VisualizationError::InvalidTaskStatus {
                    task_id: task_id.clone(),
                }
            })?;

            let status = state
                .task_status
                .get(task_id)
                .unwrap_or(&TaskExecutionStatus::Pending);

            // Build task display
            let is_critical = critical_path.contains(task_id);
            let is_focused = self.focused_task.as_ref() == Some(task_id);

            let task_box = self.render_task_box(task, status, is_critical, is_focused);

            // Add level indicator
            lines.push(format!("Level {level_idx}:"));
            lines.extend(task_box);
            lines.push(String::new());
        }

        Ok(lines)
    }

    /// Render a single task box.
    fn render_task_box(
        &self,
        task: &crate::Task,
        status: &TaskExecutionStatus,
        is_critical: bool,
        is_focused: bool,
    ) -> Vec<String> {
        let _color = Color::for_status(status);
        let symbol = self.symbol_for_status(status);
        let focus_marker = if is_focused { ">" } else { " " };
        let critical_marker = if is_critical { "*" } else { " " };

        let title = if task.title.len() > 15 {
            format!("{}...", &task.title[..12])
        } else {
            task.title.clone()
        };

        vec![
            format!(
                "{}[{}{}{}]{}",
                focus_marker,
                critical_marker,
                symbol,
                title,
                Color::Reset.ansi_code()
            ),
            format!(
                "{}├─ {}{}",
                Color::for_status(status).ansi_code(),
                progress_bar(status),
                Color::Reset.ansi_code()
            ),
            format!("{}└─ {}", Color::Gray.ansi_code(), task.id.as_str()),
        ]
    }

    /// Get symbol for task status.
    const fn symbol_for_status(&self, status: &TaskExecutionStatus) -> char {
        match status {
            TaskExecutionStatus::Pending => '○',
            TaskExecutionStatus::InProgress => '◐',
            TaskExecutionStatus::Completed => '✓',
            TaskExecutionStatus::Failed { .. } => '✗',
            TaskExecutionStatus::RolledBack => '↩',
            TaskExecutionStatus::Cancelled => '⊘',
        }
    }

    /// Format a line with color.
    fn format_line(&self, text: String, color: Color) -> String {
        if !self.enable_color {
            return text;
        }

        format!("{}{}{}", color.ansi_code(), text, Color::Reset.ansi_code())
    }

    /// Update task status in the visualization.
    ///
    /// # Note
    /// This function documents the stateless rendering approach.
    /// The visualization renderer is stateless and does not maintain internal state.
    /// To update task status, modify the `WorkflowState` and call `render_dag` again.
    ///
    /// # Example
    /// ```ignore
    /// let mut state = workflow_state;
    /// state.task_status.insert("task-1".to_string(), TaskExecutionStatus::Completed);
    /// let graph = viz.render_dag(&workflow, &state, 80, 24)?;
    /// ```
    ///
    /// # Errors
    /// This function currently always returns Ok(()) as the renderer is stateless.
    pub const fn update_task_status(
        &mut self,
        _task_id: &str,
        _status: &TaskExecutionStatus,
        _progress: f32,
    ) -> Result<(), VisualizationError> {
        // Stateless rendering: status updates are handled by modifying WorkflowState
        // and re-rendering. This approach is simpler and ensures consistency.
        //
        // If incremental updates are needed in the future for performance,
        // consider:
        // 1. Caching the rendered graph
        // 2. Updating only the affected task box
        // 3. Tracking dirty regions
        Ok(())
    }

    /// Handle keyboard input for interactive navigation.
    ///
    /// # Errors
    /// Returns `VisualizationError::InvalidTaskStatus` if navigation fails.
    pub fn handle_keyboard_input(
        &mut self,
        key: Key,
        workflow: &Workflow,
    ) -> Result<InputAction, VisualizationError> {
        match key {
            Key::Up | Key::Left => {
                // Navigate to previous task
                let task_ids: Vec<&String> = workflow.tasks.keys().collect();

                if let Some(current) = &self.focused_task {
                    if let Some(pos) = task_ids.iter().position(|id| *id == current) {
                        if pos > 0 {
                            let new_focus = task_ids[pos - 1].clone();
                            self.focused_task = Some(new_focus.clone());
                            return Ok(InputAction::FocusTask(new_focus));
                        }
                    }
                } else if !task_ids.is_empty() {
                    let first = task_ids[0].clone();
                    self.focused_task = Some(first.clone());
                    return Ok(InputAction::FocusTask(first));
                }

                Ok(InputAction::None)
            }

            Key::Down | Key::Right => {
                // Navigate to next task
                let task_ids: Vec<&String> = workflow.tasks.keys().collect();

                if let Some(current) = &self.focused_task {
                    if let Some(pos) = task_ids.iter().position(|id| *id == current) {
                        if pos + 1 < task_ids.len() {
                            let new_focus = task_ids[pos + 1].clone();
                            self.focused_task = Some(new_focus.clone());
                            return Ok(InputAction::FocusTask(new_focus));
                        }
                    }
                } else if !task_ids.is_empty() {
                    let first = task_ids[0].clone();
                    self.focused_task = Some(first.clone());
                    return Ok(InputAction::FocusTask(first));
                }

                Ok(InputAction::None)
            }

            Key::Enter => {
                // Show details for focused task
                if let Some(ref task_id) = self.focused_task {
                    Ok(InputAction::ShowDetails(task_id.clone()))
                } else {
                    Ok(InputAction::None)
                }
            }

            Key::Details => {
                // 'd' key - same as Enter
                if let Some(ref task_id) = self.focused_task {
                    Ok(InputAction::ShowDetails(task_id.clone()))
                } else {
                    Ok(InputAction::None)
                }
            }

            Key::Quit | Key::Escape => Ok(InputAction::Quit),

            Key::Unknown(_) => Ok(InputAction::None),
        }
    }
}

impl Default for WorkflowVisualization {
    fn default() -> Self {
        Self::new()
    }
}

/// Generate progress bar for task status.
fn progress_bar(status: &TaskExecutionStatus) -> String {
    match status {
        TaskExecutionStatus::Pending => "[        ]".to_string(),
        TaskExecutionStatus::InProgress => "[███▓    ]".to_string(),
        TaskExecutionStatus::Completed => "[████████]".to_string(),
        TaskExecutionStatus::Failed { .. } => "[████✗✗✗]".to_string(),
        TaskExecutionStatus::RolledBack => "[   ░░░░ ]".to_string(),
        TaskExecutionStatus::Cancelled => "[ ░░░░░░ ]".to_string(),
    }
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used)]
    #![allow(clippy::expect_used)]
    #![allow(clippy::panic)]
    #![allow(clippy::indexing_slicing)]
    #![allow(clippy::assertions_on_constants)]

    use super::*;
    use crate::execution::TaskExecutionStatus;

    #[test]
    fn test_visualization_new() {
        let viz = WorkflowVisualization::new();
        assert_eq!(viz.min_width, 80);
        assert_eq!(viz.min_height, 24);
        assert!(viz.enable_color);
    }

    #[test]
    fn test_visualization_with_min_dimensions() {
        let viz = WorkflowVisualization::new().with_min_dimensions(100, 30);
        assert_eq!(viz.min_width, 100);
        assert_eq!(viz.min_height, 30);
    }

    #[test]
    fn test_visualization_without_color() {
        let viz = WorkflowVisualization::new().without_color();
        assert!(!viz.enable_color);
    }

    #[test]
    fn test_color_for_status() {
        assert_eq!(
            Color::for_status(&TaskExecutionStatus::Pending),
            Color::Yellow
        );
        assert_eq!(
            Color::for_status(&TaskExecutionStatus::InProgress),
            Color::Blue
        );
        assert_eq!(
            Color::for_status(&TaskExecutionStatus::Completed),
            Color::Green
        );
        assert_eq!(
            Color::for_status(&TaskExecutionStatus::Failed {
                error: String::new()
            }),
            Color::Red
        );
    }

    #[test]
    fn test_symbol_for_status() {
        let viz = WorkflowVisualization::new();
        assert_eq!(viz.symbol_for_status(&TaskExecutionStatus::Pending), '○');
        assert_eq!(viz.symbol_for_status(&TaskExecutionStatus::InProgress), '◐');
        assert_eq!(viz.symbol_for_status(&TaskExecutionStatus::Completed), '✓');
        assert_eq!(
            viz.symbol_for_status(&TaskExecutionStatus::Failed {
                error: String::new()
            }),
            '✗'
        );
    }

    #[test]
    fn test_progress_bar() {
        let pb = progress_bar(&TaskExecutionStatus::Pending);
        assert!(pb.contains("        "));

        let pb = progress_bar(&TaskExecutionStatus::Completed);
        assert!(pb.contains("████"));

        let pb = progress_bar(&TaskExecutionStatus::Failed {
            error: String::new(),
        });
        assert!(pb.contains("✗"));
    }

    #[test]
    fn test_render_dag_terminal_too_small() {
        let viz = WorkflowVisualization::new().with_min_dimensions(80, 24);
        let mut workflow =
            Workflow::new("test", "Test", "Description").expect("Failed to create workflow");

        let task =
            crate::Task::new("task-1", "Task 1", "First task").expect("Failed to create task");
        workflow.add_task(task).expect("Failed to add task");

        let state = WorkflowState {
            workflow_id: "test".to_string(),
            task_status: HashMap::from([("task-1".to_string(), TaskExecutionStatus::Pending)]),
            timestamp: chrono::Utc::now(),
        };

        let result = viz.render_dag(&workflow, &state, 60, 20);
        assert!(result.is_err());
        match result {
            Err(VisualizationError::TerminalTooSmall { .. }) => {}
            _ => panic!("Expected TerminalTooSmall error"),
        }
    }

    #[test]
    fn test_render_dag_with_cycle() {
        let viz = WorkflowVisualization::new();
        let mut workflow =
            Workflow::new("test", "Test", "Description").expect("Failed to create workflow");

        let task1 = crate::Task::new("a", "Task A", "First").expect("Failed to create task");
        let task2 = crate::Task::new("b", "Task B", "Second").expect("Failed to create task");

        workflow.add_task(task1).expect("Failed to add task");
        workflow.add_task(task2).expect("Failed to add task");

        // Create cycle
        workflow
            .dependencies
            .entry("a".to_string())
            .or_default()
            .insert("b".to_string());
        workflow
            .dependencies
            .entry("b".to_string())
            .or_default()
            .insert("a".to_string());

        let state = WorkflowState {
            workflow_id: "test".to_string(),
            task_status: HashMap::from([
                ("a".to_string(), TaskExecutionStatus::Pending),
                ("b".to_string(), TaskExecutionStatus::Pending),
            ]),
            timestamp: chrono::Utc::now(),
        };

        let result = viz.render_dag(&workflow, &state, 80, 24);
        assert!(result.is_err());
        match result {
            Err(VisualizationError::CyclicGraph { cycle }) => {
                assert!(!cycle.is_empty());
            }
            _ => panic!("Expected CyclicGraph error"),
        }
    }

    #[test]
    fn test_render_dag_simple_workflow() {
        let viz = WorkflowVisualization::new();
        let mut workflow = Workflow::new("test", "Test Workflow", "Description")
            .expect("Failed to create workflow");

        let task1 = crate::Task::new("a", "Task A", "First").expect("Failed to create task");
        let task2 = crate::Task::new("b", "Task B", "Second").expect("Failed to create task");

        workflow.add_task(task1).expect("Failed to add task");
        workflow.add_task(task2).expect("Failed to add task");
        workflow
            .add_dependency("a", "b")
            .expect("Failed to add dependency");

        let state = WorkflowState {
            workflow_id: "test".to_string(),
            task_status: HashMap::from([
                ("a".to_string(), TaskExecutionStatus::Completed),
                ("b".to_string(), TaskExecutionStatus::InProgress),
            ]),
            timestamp: chrono::Utc::now(),
        };

        let result = viz.render_dag(&workflow, &state, 80, 24);
        assert!(result.is_ok());

        let graph = result.unwrap();
        assert!(!graph.lines.is_empty());
        assert!(graph.lines.iter().any(|l| l.contains("Test Workflow")));
    }

    #[test]
    fn test_build_level_layout() {
        let viz = WorkflowVisualization::new();
        let mut workflow =
            Workflow::new("test", "Test", "Description").expect("Failed to create workflow");

        let task_a = crate::Task::new("a", "Task A", "First").expect("Failed to create task");
        let task_b = crate::Task::new("b", "Task B", "Second").expect("Failed to create task");
        let task_c = crate::Task::new("c", "Task C", "Third").expect("Failed to create task");

        workflow.add_task(task_a).expect("Failed to add task");
        workflow.add_task(task_b).expect("Failed to add task");
        workflow.add_task(task_c).expect("Failed to add task");

        workflow
            .add_dependency("a", "b")
            .expect("Failed to add dependency");
        workflow
            .add_dependency("a", "c")
            .expect("Failed to add dependency");

        let levels = viz.build_level_layout(&workflow);
        assert_eq!(levels.len(), 2);
        assert_eq!(levels[0], vec!["a"]);
        assert!(levels[1].contains(&"b".to_string()));
        assert!(levels[1].contains(&"c".to_string()));
    }

    #[test]
    fn test_format_line_with_color() {
        let viz = WorkflowVisualization::new().with_min_dimensions(80, 24);
        let line = viz.format_line("Test".to_string(), Color::Green);
        assert!(line.contains("\x1b[32m"));
        assert!(line.contains("\x1b[0m"));
    }

    #[test]
    fn test_format_line_without_color() {
        let viz = WorkflowVisualization::new().without_color();
        let line = viz.format_line("Test".to_string(), Color::Green);
        assert_eq!(line, "Test");
    }

    #[test]
    fn test_render_task_box() {
        let viz = WorkflowVisualization::new();
        let task = crate::Task::new("test-task", "Test Task", "A test task")
            .expect("Failed to create task");
        let status = TaskExecutionStatus::Completed;

        let lines = viz.render_task_box(&task, &status, true, false);
        assert_eq!(lines.len(), 3);
        assert!(lines[0].contains("✓"));
        assert!(lines[0].contains("*")); // Critical marker
        assert!(lines[1].contains("████"));
    }
}
