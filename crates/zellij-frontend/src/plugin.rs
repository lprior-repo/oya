// Plugin module - Zellij plugin implementation for OYA UI
//
// This module implements the Zellij plugin protocol, handling:
// - Plugin initialization and sizing
// - Event processing (keyboard input, resize, etc.)
// - Basic UI rendering
//
// NOTE: IPC integration with oya-orchestrator will be added in a future bead

use crate::ipc::IpcClient;
use crate::layout::Layout;
use crate::render::Renderer;
use oya_ipc::{GuestMessage, HostMessage, TaskSummary};
use serde::{Deserialize, Serialize};
use thiserror::Error;

/// Plugin errors
#[derive(Debug, Error)]
pub enum PluginError {
    #[error("Render error: {0}")]
    RenderError(String),

    #[error("Invalid plugin state: {0}")]
    InvalidState(String),

    #[error("Layout calculation failed: {0}")]
    LayoutError(String),

    #[error("Terminal too small for help overlay: {rows}x{cols}, minimum 10x40 required")]
    TerminalTooSmall { rows: usize, cols: usize },
}

/// Result type for plugin operations
pub type PluginResult<T> = Result<T, PluginError>;

/// Terminal size in rows and columns
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct Size {
    /// Number of rows
    pub rows: usize,
    /// Number of columns
    pub cols: usize,
}

/// Plugin information provided by Zellij at startup
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PluginInfo {
    /// Terminal size
    pub size: Size,
    /// Plugin user configuration (if any)
    pub config: serde_json::Value,
}

/// Plugin events from Zellij
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum PluginEvent {
    /// Plugin started
    Start {
        /// Plugin info
        info: PluginInfo,
    },

    /// Terminal resized
    Resize {
        /// New size
        size: Size,
    },

    /// Keyboard input
    Key {
        /// Key character
        key: char,
        /// Modifiers (shift, ctrl, alt)
        modifiers: KeyModifiers,
    },

    /// Mouse input (future use)
    Mouse {
        /// Mouse event data
        event: MouseEvent,
    },

    /// Timer tick (future use for periodic refresh)
    Timer,
}

/// Keyboard modifier keys
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct KeyModifiers {
    /// Shift key
    pub shift: bool,
    /// Control key
    pub ctrl: bool,
    /// Alt key
    pub alt: bool,
}

/// Mouse event data
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MouseEvent {
    /// Row position
    pub row: usize,
    /// Column position
    pub col: usize,
    /// Mouse button
    pub button: MouseButton,
}

/// Mouse button
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MouseButton {
    /// Left button
    Left,
    /// Middle button
    Middle,
    /// Right button
    Right,
    /// Scroll up
    ScrollUp,
    /// Scroll down
    ScrollDown,
}

/// OYA Zellij plugin
///
/// Main plugin struct that handles:
/// - Plugin lifecycle (start, update, render)
/// - Event processing and state management
/// - Basic UI rendering with placeholder data
///
/// NOTE: Future bead will integrate IPC communication with oya-orchestrator
pub struct OyaPlugin {
    /// Terminal layout
    layout: Layout,
    /// Terminal size
    size: Size,
    /// Renderer for drawing UI
    renderer: Renderer,
    /// Currently selected pane
    focused_pane: crate::layout::PaneType,
    /// Plugin state
    state: PluginState,
    /// Task data (placeholder until IPC loads real data)
    tasks: Vec<TaskRow>,
    /// Currently selected bead index
    selected_index: usize,
    /// IPC client for orchestrator communication (not persisted)
    ipc: Option<IpcClient>,
    /// Status message shown in the UI
    status_message: Option<String>,
}

/// Stage state for tracking lifecycle
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum StageState {
    /// Stage not started
    NotStarted,
    /// Stage currently running
    Running,
    /// Stage completed successfully
    Completed,
    /// Stage failed
    Failed,
    /// Stage was reentered from later stage
    Reentered,
}

/// Stage information for display
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct StageInfo {
    /// Stage name (research, plan, implement, review, validate, accept)
    pub name: String,
    /// Current state of the stage
    pub state: StageState,
    /// Attempt number (1-indexed)
    pub attempt: u32,
}

impl StageInfo {
    /// Create new stage info
    pub fn new(name: &str) -> Self {
        Self {
            name: name.to_string(),
            state: StageState::NotStarted,
            attempt: 1,
        }
    }

    /// Get display symbol for this stage state
    pub fn symbol(&self) -> &str {
        match self.state {
            StageState::NotStarted => "○",
            StageState::Running => "🔄",
            StageState::Completed => "✓",
            StageState::Failed => "✗",
            StageState::Reentered => "↩",
        }
    }

    /// Get display string with attempt number if > 1
    pub fn display(&self) -> String {
        if self.attempt > 1 {
            format!("{} ({})", self.symbol(), self.attempt)
        } else {
            self.symbol().to_string()
        }
    }
}

/// Task data for rendering
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct TaskRow {
    pub slug: String,
    pub status: String,
    pub stage: Option<String>,
    pub priority: String,
    pub language: String,
    pub branch: String,
    /// Stage lifecycle information (research → plan → implement → review → validate → accept)
    pub stages: Vec<StageInfo>,
}

impl TaskRow {
    /// Create a new task row with empty stage history
    pub fn new(slug: &str, status: &str, priority: &str, language: &str, branch: &str) -> Self {
        let stages = vec![
            StageInfo::new("research"),
            StageInfo::new("plan"),
            StageInfo::new("implement"),
            StageInfo::new("review"),
            StageInfo::new("validate"),
            StageInfo::new("accept"),
        ];

        Self {
            slug: slug.to_string(),
            status: status.to_string(),
            stage: None,
            priority: priority.to_string(),
            language: language.to_string(),
            branch: branch.to_string(),
            stages,
        }
    }

    /// Apply a stage lifecycle event to update state
    pub fn apply_stage_event(
        &mut self,
        stage_name: &str,
        event_state: StageState,
        attempt: u32,
    ) -> Result<(), PluginError> {
        self.stages
            .iter_mut()
            .find(|s| s.name == stage_name)
            .map_or_else(
                || {
                    Err(PluginError::InvalidState(format!(
                        "Stage '{}' not found",
                        stage_name
                    )))
                },
                |stage| {
                    stage.state = event_state;
                    stage.attempt = attempt;
                    Ok(())
                },
            )
    }

    /// Get current stage display string
    pub fn stage_display(&self) -> String {
        self.stages
            .iter()
            .map(|s| s.display())
            .collect::<Vec<_>>()
            .join(" ")
    }

    /// Update task stage field from HostMessage (IPC event)
    ///
    /// This method processes stage lifecycle events from the orchestrator
    /// and updates the task's stage field and stages vector accordingly.
    ///
    /// # Arguments
    ///
    /// * `msg` - HostMessage containing stage update
    ///
    /// # Returns
    ///
    /// * `Ok(())` if update successful
    /// * `Err(PluginError)` if bead_id mismatch or invalid stage format
    pub fn update_from_ipc(&mut self, msg: &HostMessage) -> Result<(), PluginError> {
        match msg {
            // Stage started - set stage field to running stage
            HostMessage::StageStarted { bead_id, stage, .. } => {
                self.validate_bead_id(bead_id)?;
                self.stage = Some(stage.clone());
                self.apply_stage_event(stage, StageState::Running, 1)
            }

            // Stage completed - mark stage as complete
            HostMessage::StageCompleted {
                bead_id, stage, ..
            } => {
                self.validate_bead_id(bead_id)?;
                self.stage = Some(stage.clone());
                self.apply_stage_event(stage, StageState::Completed, 1)
            }

            // Stage failed - mark stage as failed with detail
            HostMessage::StageFailed {
                bead_id,
                stage,
                feedback,
                ..
            } => {
                self.validate_bead_id(bead_id)?;
                let stage_with_detail = format!("{}: {}", stage, feedback);
                self.stage = Some(stage_with_detail.clone());
                self.apply_stage_event(stage, StageState::Failed, 1)?;
                // Store detailed failure in stage field
                self.stage = Some(stage_with_detail);
                Ok(())
            }

            // Stage reentry - reset to earlier stage
            HostMessage::StageReentry {
                bead_id,
                to_stage,
                attempt,
                ..
            } => {
                self.validate_bead_id(bead_id)?;
                self.stage = Some(to_stage.clone());
                self.apply_stage_event(to_stage, StageState::Reentered, *attempt)
            }

            // Validation ran - update validate stage with result
            HostMessage::ValidationRan {
                bead_id, passed, ..
            } => {
                self.validate_bead_id(bead_id)?;
                let stage_name = "validate";
                let state = if *passed {
                    StageState::Completed
                } else {
                    StageState::Failed
                };
                self.apply_stage_event(stage_name, state, 1)
            }

            // Recursion exhausted - mark last stage as failed
            HostMessage::RecursionExhausted {
                bead_id,
                last_stage,
                ..
            } => {
                self.validate_bead_id(bead_id)?;
                let stage_str = stage_kind_to_string(last_stage);
                self.stage = Some(format!(
                    "{}: Recursion exhausted after 15 attempts",
                    stage_str
                ));
                self.apply_stage_event(&stage_str, StageState::Failed, 15)
            }

            // Non-stage events are ignored
            _ => Ok(()),
        }
    }

    /// Validate that bead_id matches this task's slug
    fn validate_bead_id(&self, bead_id: &str) -> Result<(), PluginError> {
        match bead_id == self.slug {
            true => Ok(()),
            false => Err(PluginError::InvalidState(format!(
                "Bead ID mismatch: expected {}, got {}",
                self.slug, bead_id
            ))),
        }
    }
}

/// Convert stage kind string to display format
///
/// This helper function maps stage names from IPC messages
/// to the format used in TaskRow stages vector.
fn stage_kind_to_string(stage: &str) -> String {
    stage.to_string()
}

/// Map (status, stage) tuple to stage progression symbol
///
/// This function determines the appropriate symbol to display
/// for a stage based on the task's overall status and current stage.
///
/// # Arguments
///
/// * `status` - Task status (created/in_progress/failed/passed/integrated)
/// * `stage` - Optional current stage name with optional detail
///
/// # Returns
///
/// Stage progression symbol: ◐ (running), ● (complete), ✗ (failed), ○ (pending)
pub fn stage_symbol_from_status(status: &str, stage: Option<&str>) -> char {
    // Extract stage name without detail (before colon if present)
    let stage_name = stage.and_then(|s| s.split(':').next());

    match (status, stage_name) {
        // Passed/Integrated = all complete
        ("passed" | "integrated", _) => '●',

        // Failed stage
        ("failed", Some(_)) => '✗',

        // Currently running stage
        ("in_progress", Some(stage_name)) => {
            // Map stage name to running symbol
            match stage_name {
                "research" | "plan" | "implement" | "review" | "validate" | "accept" => '◐',
                _ => '?', // Unknown stage
            }
        }

        // Created = not started
        ("created", _) => '○',

        // Default: unknown state
        _ => '?',
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_stage_symbol_returns_running_for_in_progress_stage() {
        let result = stage_symbol_from_status("in_progress", Some("implement"));
        assert_eq!(result, '◐');
    }

    #[test]
    fn test_stage_symbol_returns_complete_for_passed_status() {
        let result = stage_symbol_from_status("passed", None);
        assert_eq!(result, '●');
    }

    #[test]
    fn test_stage_symbol_returns_failed_for_failed_status_with_stage() {
        let result = stage_symbol_from_status("failed", Some("validate: 3 tests failed"));
        assert_eq!(result, '✗');
    }

    #[test]
    fn test_stage_symbol_returns_pending_for_created_status() {
        let result = stage_symbol_from_status("created", None);
        assert_eq!(result, '○');
    }

    #[test]
    fn test_stage_symbol_returns_question_mark_for_unknown_stage_name() {
        let result = stage_symbol_from_status("in_progress", Some("unknown-stage"));
        assert_eq!(result, '?');
    }

    #[test]
    fn test_stage_symbol_extracts_stage_name_before_colon() {
        let result = stage_symbol_from_status("in_progress", Some("implement: writing code"));
        assert_eq!(result, '◐');
    }

    #[test]
    fn test_task_row_update_from_ipc_stage_started() {
        let mut task = TaskRow::new("bd-3a0a.8", "created", "P0", "Rust", "task/bd-3a0a.8");
        let msg = HostMessage::StageStarted {
            bead_id: "bd-3a0a.8".to_string(),
            stage: "implement".to_string(),
            attempt: 1,
            timestamp: 1739097600,
        };

        let result = task.update_from_ipc(&msg);
        assert!(result.is_ok());
        assert_eq!(task.stage, Some("implement".to_string()));
    }

    #[test]
    fn test_task_row_update_from_ipc_stage_completed() {
        let mut task = TaskRow::new("bd-3a0a.8", "in_progress", "P0", "Rust", "task/bd-3a0a.8");
        task.stage = Some("implement".to_string());

        let msg = HostMessage::StageCompleted {
            bead_id: "bd-3a0a.8".to_string(),
            stage: "implement".to_string(),
            artifact_ref: Some("artifacts/code.rs".to_string()),
            timestamp: 1739097660,
        };

        let result = task.update_from_ipc(&msg);
        assert!(result.is_ok());
        assert_eq!(task.stage, Some("implement".to_string()));
        assert_eq!(task.stages[2].state, StageState::Completed); // implement is index 2
    }

    #[test]
    fn test_task_row_update_from_ipc_stage_failed() {
        let mut task = TaskRow::new("bd-3a0a.8", "in_progress", "P0", "Rust", "task/bd-3a0a.8");
        task.stage = Some("validate".to_string());

        let msg = HostMessage::StageFailed {
            bead_id: "bd-3a0a.8".to_string(),
            stage: "validate".to_string(),
            feedback: "3 tests failed".to_string(),
            severity: "minor".to_string(),
            timestamp: 1739097720,
        };

        let result = task.update_from_ipc(&msg);
        assert!(result.is_ok());
        assert_eq!(task.stage, Some("validate: 3 tests failed".to_string()));
        assert_eq!(task.stages[4].state, StageState::Failed); // validate is index 4
    }

    #[test]
    fn test_task_row_update_from_ipc_bead_id_mismatch() {
        let mut task = TaskRow::new("bd-3a0a.8", "created", "P0", "Rust", "task/bd-3a0a.8");
        let msg = HostMessage::StageStarted {
            bead_id: "bd-3a0a.9".to_string(), // Different bead ID
            stage: "implement".to_string(),
            attempt: 1,
            timestamp: 1739097600,
        };

        let result = task.update_from_ipc(&msg);
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("Bead ID mismatch"));
    }

    #[test]
    fn test_task_row_update_from_ipc_is_idempotent() {
        let mut task = TaskRow::new("bd-3a0a.8", "in_progress", "P0", "Rust", "task/bd-3a0a.8");
        task.stage = Some("implement".to_string());

        let msg = HostMessage::StageStarted {
            bead_id: "bd-3a0a.8".to_string(),
            stage: "implement".to_string(),
            attempt: 1,
            timestamp: 1739097600,
        };

        // Update twice with same event
        let result1 = task.update_from_ipc(&msg);
        let result2 = task.update_from_ipc(&msg);

        assert!(result1.is_ok());
        assert!(result2.is_ok());
        assert_eq!(task.stage, Some("implement".to_string()));
    }

    // Additional tests from original second test module

    #[test]
    fn test_plugin_creation() {
        let plugin = OyaPlugin::new();
        assert!(plugin.is_ok());
    }

    #[test]
    fn test_size_serialization() {
        let size = Size { rows: 24, cols: 80 };
        let json = serde_json::to_string(&size).expect("serialization should succeed");
        let decoded: Size = serde_json::from_str(&json).expect("deserialization should succeed");
        assert_eq!(decoded.rows, 24);
        assert_eq!(decoded.cols, 80);
    }

    #[test]
    fn test_plugin_state() {
        assert_ne!(PluginState::Running, PluginState::Starting);
        assert_ne!(PluginState::Running, PluginState::Error);
    }

    #[test]
    fn test_key_modifiers() {
        let mods = KeyModifiers {
            shift: true,
            ctrl: false,
            alt: false,
        };
        assert!(mods.shift);
        assert!(!mods.ctrl);
    }

    #[test]
    fn test_sample_beads() {
        let plugin = OyaPlugin::new().expect("plugin creation should succeed");
        assert!(!plugin.tasks.is_empty());
        assert_eq!(plugin.tasks[0].slug, "task-3ax5");
    }
}

/// Plugin state machine
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum PluginState {
    /// Plugin starting
    Starting,
    /// Running normally
    Running,
    /// Help overlay is active
    HelpOverlay,
    /// Error state
    Error,
    /// Shutting down
    ShuttingDown,
}

impl OyaPlugin {
    /// Create a new OYA plugin instance
    ///
    /// # Errors
    ///
    /// Returns an error if layout calculation fails
    pub fn new() -> PluginResult<Self> {
        // Default terminal size (will be updated on first event)
        let size = Size { rows: 24, cols: 80 };

        // Calculate initial layout
        let layout = Layout::calculate_for_terminal(size.rows, size.cols)
            .map_err(|e| PluginError::LayoutError(e.to_string()))?;

        let renderer = Renderer::new();

        // Create placeholder task data for rendering
        let mut tasks = vec![
            TaskRow::new("task-3ax5", "in_progress", "P1", "Rust", "task/task-3ax5"),
            TaskRow::new("task-1xvj", "created", "P1", "Rust", "task/task-1xvj"),
            TaskRow::new("task-1k71", "created", "P2", "Rust", "task/task-1k71"),
        ];

        // Set some stage states for demo
        let _ = tasks[0].apply_stage_event("research", StageState::Completed, 1);
        let _ = tasks[0].apply_stage_event("plan", StageState::Completed, 1);
        let _ = tasks[0].apply_stage_event("implement", StageState::Running, 1);

        Ok(Self {
            layout,
            size,
            renderer,
            focused_pane: crate::layout::PaneType::BeadList,
            state: PluginState::Starting,
            tasks,
            selected_index: 0,
            ipc: None,
            status_message: None,
        })
    }

    /// Start the plugin
    ///
    /// # Errors
    ///
    /// Returns an error if initialization fails
    pub fn start(&mut self, info: PluginInfo) -> PluginResult<String> {
        self.size = info.size;

        // Recalculate layout for actual terminal size
        self.layout = Layout::calculate_for_terminal(self.size.rows, self.size.cols)
            .map_err(|e| PluginError::LayoutError(e.to_string()))?;

        self.state = PluginState::Running;
        self.connect_ipc(&info.config)?;
        let _ = self.refresh_tasks();

        // Render initial UI
        match self.render()? {
            Some(rendered) => Ok(rendered),
            None => Ok(String::from("OYA UI Plugin started")),
        }
    }

    /// Handle a plugin event
    ///
    /// # Errors
    ///
    /// Returns an error if event handling fails
    pub fn handle_event(&mut self, event: PluginEvent) -> PluginResult<Option<String>> {
        match event {
            PluginEvent::Start { info } => {
                let rendered = self.start(info)?;
                Ok(Some(rendered))
            }
            PluginEvent::Resize { size } => {
                self.size = size;
                self.layout = Layout::calculate_for_terminal(self.size.rows, self.size.cols)
                    .map_err(|e| PluginError::LayoutError(e.to_string()))?;
                self.render()
            }
            PluginEvent::Key { key, modifiers } => {
                self.handle_key(key, modifiers)?;
                self.render()
            }
            PluginEvent::Mouse { event: _ } => {
                // Mouse events not implemented yet
                Ok(None)
            }
            PluginEvent::Timer => {
                // Timer events not implemented yet
                Ok(None)
            }
        }
    }

    /// Handle keyboard input
    ///
    /// # Errors
    ///
    /// Returns an error if key handling fails
    fn handle_key(&mut self, key: char, _modifiers: KeyModifiers) -> PluginResult<()> {
        // Handle help overlay toggle/close
        if key == '?' || key == '\x1b' {
            // ESC key
            return self.toggle_help_overlay().map(|_| ());
        }

        // Ignore other keys when help overlay is active
        if self.state == PluginState::HelpOverlay {
            return Ok(());
        }

        match key {
            // Quit
            'q' | 'Q' => {
                self.state = PluginState::ShuttingDown;
            }
            // Navigate between panes with Tab
            '\t' => {
                self.cycle_focus();
            }
            // Vim-style navigation
            'j' | 'J' => {
                self.move_selection(1)?;
            }
            'k' | 'K' => {
                self.move_selection(-1)?;
            }
            'g' | 'G' => {
                let _ = self.refresh_tasks();
            }
            'r' | 'R' => {
                let _ = self.run_pipeline_for_selected(false);
            }
            'a' | 'A' => {
                let _ = self.approve_selected(false);
            }
            'b' | 'B' => {
                let _ = self.run_pipeline_for_all(false);
            }
            _ => {
                // Other keys ignored
            }
        }

        Ok(())
    }

    /// Cycle focus between panes
    fn cycle_focus(&mut self) {
        self.focused_pane = match self.focused_pane {
            crate::layout::PaneType::BeadList => crate::layout::PaneType::BeadDetail,
            crate::layout::PaneType::BeadDetail => crate::layout::PaneType::PipelineView,
            crate::layout::PaneType::PipelineView => crate::layout::PaneType::WorkflowGraph,
            crate::layout::PaneType::WorkflowGraph => crate::layout::PaneType::BeadList,
        };
    }

    /// Move selection in bead list
    ///
    /// # Errors
    ///
    /// Returns an error if movement fails
    fn move_selection(&mut self, direction: i32) -> PluginResult<()> {
        let len = self.tasks.len();

        if len == 0 {
            return Ok(());
        }

        let new_index = if direction > 0 {
            self.selected_index.saturating_add(1)
        } else {
            self.selected_index.saturating_sub(1)
        };

        // Wrap around
        self.selected_index = if new_index >= len { 0 } else { new_index };

        Ok(())
    }

    /// Handle incoming stage update from orchestrator
    ///
    /// # Errors
    ///
    /// Returns an error if the update cannot be applied
    pub fn handle_stage_update(&mut self, message: HostMessage) -> PluginResult<()> {
        match message {
            HostMessage::PhaseProgress {
                bead_id,
                phase_id,
                progress,
                current_step,
            } => {
                let stage_state = if progress >= 100 {
                    StageState::Completed
                } else {
                    StageState::Running
                };
                if let Err(err) = self.update_task_stage(&bead_id, &phase_id, stage_state, 1) {
                    self.status_message = Some(format!(
                        "{bead_id}: phase '{phase_id}' {progress}% ({current_step}) [{err}]"
                    ));
                    return Ok(());
                }
                self.status_message = Some(format!(
                    "{bead_id}: {phase_id} {progress}% ({current_step})"
                ));
            }

            HostMessage::BeadStateChanged {
                bead_id,
                from_state,
                to_state,
                ..
            } => {
                if let Some(task) = self.tasks.iter_mut().find(|task| task.slug == bead_id) {
                    task.status = to_state.clone();
                }
                self.status_message = Some(format!("{bead_id}: state {from_state} -> {to_state}"));
            }

            HostMessage::SystemAlert {
                level,
                message,
                component,
                ..
            } => {
                let component_prefix = component
                    .as_deref()
                    .map_or_else(String::new, |c| format!("{c}: "));
                self.status_message = Some(format!("{level:?}: {component_prefix}{message}"));
            }

            // Ignore other message types
            _ => return Ok(()),
        }

        Ok(())
    }

    /// Update task stage state
    ///
    /// # Errors
    ///
    /// Returns an error if the task is not found or stage update fails
    fn update_task_stage(
        &mut self,
        bead_id: &str,
        stage_name: &str,
        state: StageState,
        attempt: u32,
    ) -> PluginResult<()> {
        self.tasks
            .iter_mut()
            .find(|task| task.slug == bead_id)
            .map_or_else(
                || {
                    Err(PluginError::InvalidState(format!(
                        "Task '{}' not found for stage update",
                        bead_id
                    )))
                },
                |task| task.apply_stage_event(stage_name, state, attempt),
            )
    }

    /// Render the UI
    ///
    /// # Errors
    ///
    /// Returns an error if rendering fails
    fn render(&mut self) -> PluginResult<Option<String>> {
        if self.state == PluginState::ShuttingDown {
            return Ok(None);
        }

        let rendered = self.renderer.render_layout(
            &self.layout,
            &self.tasks,
            self.selected_index,
            self.focused_pane,
            self.status_message.as_deref(),
        );

        Ok(Some(rendered))
    }

    /// Run the plugin main loop
    ///
    /// # Errors
    ///
    /// Returns an error if the loop fails
    pub fn run(&mut self) -> PluginResult<()> {
        // In a real Zellij plugin, this would:
        // 1. Listen for events from stdin (Zellij protocol)
        // 2. Process events
        // 3. Write rendered output to stdout
        // 4. Loop until shutdown

        // For now, this is a simplified version
        Ok(())
    }

    fn connect_ipc(&mut self, config: &serde_json::Value) -> PluginResult<()> {
        let address = config
            .get("ipc_address")
            .and_then(|value| value.as_str())
            .unwrap_or("127.0.0.1:5555");

        match IpcClient::connect(address) {
            Ok(client) => {
                self.ipc = Some(client);
                self.status_message = Some(format!("IPC connected to {address}"));
                Ok(())
            }
            Err(err) => {
                self.ipc = None;
                self.status_message = Some(format!("IPC unavailable: {err}"));
                Ok(())
            }
        }
    }

    fn refresh_tasks(&mut self) -> PluginResult<()> {
        let ipc = match self.ipc.as_mut() {
            Some(ipc) => ipc,
            None => {
                self.status_message = Some("IPC not connected".to_string());
                return Ok(());
            }
        };

        match ipc.request(GuestMessage::GetTaskList) {
            Ok(HostMessage::TaskList { tasks }) => {
                self.tasks = tasks.into_iter().map(task_summary_to_row).collect();
                self.selected_index = self.selected_index.min(self.tasks.len().saturating_sub(1));
                self.status_message = Some("Tasks refreshed".to_string());
                Ok(())
            }
            Ok(message) => {
                self.status_message = Some(format!("Unexpected response: {message:?}"));
                Ok(())
            }
            Err(err) => {
                self.status_message = Some(format!("IPC error: {err}"));
                Ok(())
            }
        }
    }

    fn run_pipeline_for_selected(&mut self, dry_run: bool) -> PluginResult<()> {
        let slug = match self.tasks.get(self.selected_index) {
            Some(task) => task.slug.clone(),
            None => {
                self.status_message = Some("No task selected".to_string());
                return Ok(());
            }
        };

        self.send_task_command(GuestMessage::RunPipeline { slug, dry_run })
    }

    fn run_pipeline_for_all(&mut self, dry_run: bool) -> PluginResult<()> {
        if self.tasks.is_empty() {
            self.status_message = Some("No tasks to run".to_string());
            return Ok(());
        }

        let slugs = self.tasks.iter().map(|task| task.slug.clone()).collect();

        self.send_task_command(GuestMessage::RunPipelineBatch { slugs, dry_run })
    }

    fn approve_selected(&mut self, force: bool) -> PluginResult<()> {
        let slug = match self.tasks.get(self.selected_index) {
            Some(task) => task.slug.clone(),
            None => {
                self.status_message = Some("No task selected".to_string());
                return Ok(());
            }
        };

        self.send_task_command(GuestMessage::ApproveTask { slug, force })
    }

    fn send_task_command(&mut self, message: GuestMessage) -> PluginResult<()> {
        let ipc = match self.ipc.as_mut() {
            Some(ipc) => ipc,
            None => {
                self.status_message = Some("IPC not connected".to_string());
                return Ok(());
            }
        };

        match ipc.request(message) {
            Ok(HostMessage::TaskUpdated {
                slug,
                status,
                message,
            }) => {
                self.status_message = Some(format!("{slug}: {status} ({message})"));
                let _ = self.refresh_tasks();
                Ok(())
            }
            Ok(HostMessage::Error { message, .. }) => {
                self.status_message = Some(format!("Task error: {message}"));
                Ok(())
            }
            Ok(HostMessage::TaskBatchUpdated { updated, failed }) => {
                let total = updated.len().saturating_add(failed.len());
                self.status_message = Some(format!(
                    "Batch complete: {}/{} updated",
                    updated.len(),
                    total
                ));
                let _ = self.refresh_tasks();
                Ok(())
            }
            Ok(other) => {
                self.status_message = Some(format!("Unexpected response: {other:?}"));
                Ok(())
            }
            Err(err) => {
                self.status_message = Some(format!("IPC error: {err}"));
                Ok(())
            }
        }
    }

    /// Get keybindings for a specific pane type
    ///
    /// Returns a vector of (key, action description) tuples.
    /// Empty vector if no keybindings defined (no error).
    fn get_keybindings_for_pane(
        &self,
        pane_type: crate::layout::PaneType,
    ) -> Vec<(char, &'static str)> {
        match pane_type {
            crate::layout::PaneType::BeadList => vec![
                ('j', "Move down"),
                ('k', "Move up"),
                ('\t', "Switch pane"),
                ('g', "Refresh tasks"),
                ('r', "Run pipeline"),
                ('a', "Approve task"),
                ('b', "Batch run"),
            ],
            crate::layout::PaneType::BeadDetail => vec![
                ('\t', "Switch pane"),
                ('r', "Run pipeline"),
                ('a', "Approve task"),
            ],
            crate::layout::PaneType::PipelineView => {
                vec![('\t', "Switch pane"), ('g', "Refresh tasks")]
            }
            crate::layout::PaneType::WorkflowGraph => vec![('\t', "Switch pane")],
        }
    }

    /// Open help overlay with context-sensitive keybindings
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - Plugin is not in Running state
    /// - Terminal is too small (< 10 rows x 40 cols)
    fn open_help_overlay(&mut self) -> PluginResult<String> {
        // Check precondition: must be in Running state
        if self.state != PluginState::Running {
            return Err(PluginError::InvalidState(
                "Cannot open help overlay: plugin must be in Running state".to_string(),
            ));
        }

        // Check precondition: terminal minimum size
        const MIN_ROWS: usize = 10;
        const MIN_COLS: usize = 40;
        if self.size.rows < MIN_ROWS || self.size.cols < MIN_COLS {
            return Err(PluginError::TerminalTooSmall {
                rows: self.size.rows,
                cols: self.size.cols,
            });
        }

        // Transition to HelpOverlay state
        self.state = PluginState::HelpOverlay;

        // Get keybindings for current pane
        let keybindings = self.get_keybindings_for_pane(self.focused_pane);

        // Render overlay
        self.renderer
            .render_help_overlay(
                self.size.rows,
                self.size.cols,
                &keybindings,
                self.focused_pane,
            )
            .map_err(|e| PluginError::RenderError(e.to_string()))
    }

    /// Close help overlay and restore previous state
    ///
    /// # Errors
    ///
    /// Returns an error if plugin is not in HelpOverlay state
    fn close_help_overlay(&mut self) -> PluginResult<String> {
        // Check precondition: must be in HelpOverlay state
        if self.state != PluginState::HelpOverlay {
            return Err(PluginError::InvalidState(
                "Cannot close help overlay: overlay is not open".to_string(),
            ));
        }

        // Transition back to Running state
        self.state = PluginState::Running;

        // Render full UI (without overlay)
        match self.render()? {
            Some(rendered) => Ok(rendered),
            None => Ok(String::from("Help overlay closed")),
        }
    }

    /// Toggle help overlay (open if closed, close if open)
    ///
    /// # Errors
    ///
    /// Propagates errors from open_help_overlay or close_help_overlay
    fn toggle_help_overlay(&mut self) -> PluginResult<String> {
        match self.state {
            PluginState::Running => self.open_help_overlay(),
            PluginState::HelpOverlay => self.close_help_overlay(),
            _ => Err(PluginError::InvalidState(
                "Cannot toggle help overlay: invalid state".to_string(),
            )),
        }
    }

    // ========================================================================
    // STATE PERSISTENCE METHODS
    // ========================================================================

    /// Get a reference to the tasks list (for state snapshot creation)
    pub fn tasks_ref(&self) -> &[TaskRow] {
        &self.tasks
    }

    /// Get the current selected index (for state snapshot creation)
    pub fn selected_index(&self) -> usize {
        self.selected_index
    }

    /// Get the current focused pane (for state snapshot creation)
    pub fn focused_pane(&self) -> crate::layout::PaneType {
        self.focused_pane
    }

    /// Get the current plugin state (for state snapshot creation)
    pub fn plugin_state(&self) -> PluginState {
        self.state
    }

    /// Get the current status message (for state snapshot creation)
    pub fn status_message(&self) -> Option<&str> {
        self.status_message.as_deref()
    }

    /// Restore plugin state from a snapshot
    ///
    /// # Arguments
    ///
    /// * `snapshot` - State snapshot to restore
    ///
    /// # Errors
    ///
    /// Returns `Err(PluginError)` if restoration fails
    pub fn restore_from_snapshot(
        &mut self,
        snapshot: crate::state::StateSnapshot,
    ) -> PluginResult<()> {
        // Validate snapshot
        let mut snapshot = snapshot;
        snapshot
            .validate()
            .map_err(|e| PluginError::InvalidState(e.to_string()))?;

        // Restore tasks
        self.tasks = snapshot.tasks;

        // Restore selected index (already validated and clamped)
        self.selected_index = snapshot.selected_index;

        // Restore focused pane
        self.focused_pane = snapshot.focused_pane;

        // Restore plugin state
        self.state = snapshot.plugin_state;

        // Restore status message
        self.status_message = snapshot.status_message;

        // IPC client is not restored (must be re-established)
        // Reset to None to force reconnection
        self.ipc = None;

        Ok(())
    }

    /// Create a snapshot of current plugin state
    ///
    /// # Returns
    ///
    /// State snapshot
    pub fn create_snapshot(&self) -> crate::state::StateSnapshot {
        use std::time::{SystemTime, UNIX_EPOCH};

        let timestamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map_or_else(|_| 0, |d| d.as_secs());

        crate::state::StateSnapshot {
            version: crate::state::STATE_VERSION,
            tasks: self.tasks.clone(),
            selected_index: self.selected_index,
            focused_pane: self.focused_pane,
            plugin_state: self.state,
            status_message: self.status_message.clone(),
            timestamp,
        }
    }

    /// Get the default state file path
    ///
    /// Uses `$XDG_DATA_HOME/oya/zellij-plugin-state.json` or `$HOME/.local/share/oya/...`
    pub fn default_state_file_path() -> std::path::PathBuf {
        // Try XDG_DATA_HOME first
        if let Ok(xdg_data_home) = std::env::var("XDG_DATA_HOME") {
            let path = std::path::PathBuf::from(xdg_data_home)
                .join("oya")
                .join("zellij-plugin-state.json");
            return path;
        }

        // Fallback to $HOME/.local/share
        if let Ok(home) = std::env::var("HOME") {
            let path = std::path::PathBuf::from(home)
                .join(".local")
                .join("share")
                .join("oya")
                .join("zellij-plugin-state.json");
            return path;
        }

        // Final fallback to /tmp
        std::path::PathBuf::from("/tmp/oya-zellij-state.json")
    }
}

// ========================================================================
// DROP TRAIT FOR AUTO-SAVE ON SHUTDOWN
// ========================================================================

impl Drop for OyaPlugin {
    fn drop(&mut self) {
        // Attempt to save state on shutdown
        // Ignore errors since we're in a destructor
        let state_manager = crate::state::StateManager::default();
        let _ = state_manager.save_state(self);
    }
}

fn task_summary_to_row(summary: TaskSummary) -> TaskRow {
    let mut row = TaskRow::new(
        &summary.slug,
        &summary.status,
        &summary.priority,
        &summary.language,
        &summary.branch,
    );
    row.stage = summary.stage;
    row
}
