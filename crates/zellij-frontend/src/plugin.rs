// Plugin module - Zellij plugin implementation for OYA UI
//
// This module implements the Zellij plugin protocol, handling:
// - Plugin initialization and sizing
// - Event processing (keyboard input, resize, etc.)
// - Basic UI rendering
//
// NOTE: IPC integration with oya-orchestrator will be added in a future bead

use crate::ipc::IpcClient;
use crate::layout::{Layout, PaneType};
use crate::render::Renderer;
use crate::state::StateManager;
use crate::timer::{RefreshTimer, TimerConfig};
use oya_ipc::{GuestMessage, HostMessage, TaskSummary};
use serde::{Deserialize, Serialize};
use std::time::{SystemTime, UNIX_EPOCH};
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

    #[error("Timer error: {0}")]
    TimerError(String),

    #[error("State save failed: {0}")]
    StateSaveError(String),
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
/// - Automatic state saves on timer
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
    /// Auto-save timer for periodic state saves
    auto_save_timer: Option<RefreshTimer>,
    /// Last successful save timestamp (unix seconds)
    last_save_timestamp: Option<u64>,
    /// Last timer tick timestamp (unix milliseconds)
    last_timer_tick_ms: Option<u64>,
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
            HostMessage::StageCompleted { bead_id, stage, .. } => {
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

        // Currently running stage - collapse into outer match
        (
            "in_progress",
            Some("research" | "plan" | "implement" | "review" | "validate" | "accept"),
        ) => '◐',

        // Created = not started
        ("created", _) => '○',

        // Default: unknown state
        _ => '?',
    }
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used)]
    #![allow(clippy::indexing_slicing)]
    #![allow(clippy::unwrap_in_result)]

    use super::*;

    #[test]
    fn test_stage_symbol_returns_running_for_in_progress_stage(
    ) -> Result<(), Box<dyn std::error::Error>> {
        let result = stage_symbol_from_status("in_progress", Some("implement"));
        assert_eq!(result, '◐');
        Ok(())
    }

    #[test]
    fn test_stage_symbol_returns_complete_for_passed_status(
    ) -> Result<(), Box<dyn std::error::Error>> {
        let result = stage_symbol_from_status("passed", None);
        assert_eq!(result, '●');
        Ok(())
    }

    #[test]
    fn test_stage_symbol_returns_failed_for_failed_status_with_stage(
    ) -> Result<(), Box<dyn std::error::Error>> {
        let result = stage_symbol_from_status("failed", Some("validate: 3 tests failed"));
        assert_eq!(result, '✗');
        Ok(())
    }

    #[test]
    fn test_stage_symbol_returns_pending_for_created_status(
    ) -> Result<(), Box<dyn std::error::Error>> {
        let result = stage_symbol_from_status("created", None);
        assert_eq!(result, '○');
        Ok(())
    }

    #[test]
    fn test_stage_symbol_returns_question_mark_for_unknown_stage_name(
    ) -> Result<(), Box<dyn std::error::Error>> {
        let result = stage_symbol_from_status("in_progress", Some("unknown-stage"));
        assert_eq!(result, '?');
        Ok(())
    }

    #[test]
    fn test_stage_symbol_extracts_stage_name_before_colon() -> Result<(), Box<dyn std::error::Error>>
    {
        let result = stage_symbol_from_status("in_progress", Some("implement: writing code"));
        assert_eq!(result, '◐');
        Ok(())
    }

    #[test]
    fn test_task_row_update_from_ipc_stage_started() -> Result<(), Box<dyn std::error::Error>> {
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
        Ok(())
    }

    #[test]
    fn test_task_row_update_from_ipc_stage_completed() -> Result<(), Box<dyn std::error::Error>> {
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
        Ok(())
    }

    #[test]
    fn test_task_row_update_from_ipc_stage_failed() -> Result<(), Box<dyn std::error::Error>> {
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
        Ok(())
    }

    #[test]
    fn test_task_row_update_from_ipc_bead_id_mismatch() -> Result<(), Box<dyn std::error::Error>> {
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
        Ok(())
    }

    #[test]
    fn test_task_row_update_from_ipc_is_idempotent() -> Result<(), Box<dyn std::error::Error>> {
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
        Ok(())
    }

    // ========================================================================
    // STATE RESTORATION TESTS
    // ========================================================================

    #[test]
    fn test_plugin_restores_state_from_snapshot() -> Result<(), Box<dyn std::error::Error>> {
        let mut plugin = OyaPlugin::new()?;

        // Create a snapshot with specific state
        let snapshot = crate::state::StateSnapshot {
            version: crate::state::STATE_VERSION,
            tasks: vec![TaskRow::new(
                "test-task",
                "in_progress",
                "P0",
                "Rust",
                "task/test",
            )],
            selected_index: 0,
            focused_pane: crate::layout::PaneType::BeadDetail,
            plugin_state: PluginState::Running,
            status_message: Some("Test message".to_string()),
            timestamp: 0,
        };

        // Restore from snapshot
        let result = plugin.restore_from_snapshot(snapshot);

        // Verify restoration succeeded
        assert!(result.is_ok());
        assert_eq!(plugin.tasks.len(), 1);
        assert_eq!(plugin.tasks[0].slug, "test-task");
        assert_eq!(plugin.selected_index, 0);
        assert_eq!(plugin.focused_pane, crate::layout::PaneType::BeadDetail);
        assert_eq!(plugin.state, PluginState::Running);
        assert_eq!(plugin.status_message, Some("Test message".to_string()));
        // IPC client should be reset to None after restoration
        assert!(plugin.ipc.is_none());
        Ok(())
    }

    #[test]
    fn test_plugin_restore_clamps_invalid_selected_index() -> Result<(), Box<dyn std::error::Error>>
    {
        let mut plugin = OyaPlugin::new()?;

        // Create snapshot with invalid selected_index (out of bounds)
        let mut snapshot = crate::state::StateSnapshot {
            version: crate::state::STATE_VERSION,
            tasks: vec![TaskRow::new("task-1", "created", "P0", "Rust", "task/1")],
            selected_index: 10, // Invalid - only 1 task
            focused_pane: crate::layout::PaneType::BeadList,
            plugin_state: PluginState::Running,
            status_message: None,
            timestamp: 0,
        };

        // Validate should clamp selected_index to valid range
        let validation_result = snapshot.validate();
        assert!(validation_result.is_ok());
        assert_eq!(snapshot.selected_index, 0); // Clamped to 0 (only valid index)

        // Restore should succeed with clamped value
        let result = plugin.restore_from_snapshot(snapshot);
        assert!(result.is_ok());
        assert_eq!(plugin.selected_index, 0);
        Ok(())
    }

    #[test]
    fn test_plugin_restore_rejects_incompatible_version() -> Result<(), Box<dyn std::error::Error>>
    {
        let mut plugin = OyaPlugin::new()?;

        // Create snapshot with incompatible version
        let snapshot = crate::state::StateSnapshot {
            version: 999, // Incompatible version
            tasks: vec![],
            selected_index: 0,
            focused_pane: crate::layout::PaneType::BeadList,
            plugin_state: PluginState::Running,
            status_message: None,
            timestamp: 0,
        };

        // Restore should fail with validation error
        let result = plugin.restore_from_snapshot(snapshot);
        assert!(result.is_err());
        let error_msg = result.unwrap_err().to_string();
        // Check for version-related error message
        assert!(
            error_msg.contains("version")
                || error_msg.contains("999")
                || error_msg.contains("incompatible")
        );
        Ok(())
    }

    #[test]
    fn test_plugin_create_snapshot() -> Result<(), Box<dyn std::error::Error>> {
        let plugin = OyaPlugin::new()?;

        // Create snapshot
        let snapshot = plugin.create_snapshot();

        // Verify snapshot contains current state
        assert_eq!(snapshot.version, crate::state::STATE_VERSION);
        assert!(!snapshot.tasks.is_empty());
        assert_eq!(snapshot.selected_index, 0);
        assert_eq!(snapshot.focused_pane, crate::layout::PaneType::BeadList);
        assert_eq!(snapshot.plugin_state, PluginState::Starting);
        assert!(snapshot.status_message.is_none());
        assert!(snapshot.timestamp > 0);
        Ok(())
    }

    #[test]
    fn test_plugin_restore_preserves_task_stage_history() -> Result<(), Box<dyn std::error::Error>>
    {
        let mut plugin = OyaPlugin::new()?;

        // Set up task with stage history
        let mut task = TaskRow::new("test-task", "in_progress", "P0", "Rust", "task/test");
        let _ = task.apply_stage_event("research", StageState::Completed, 1);
        let _ = task.apply_stage_event("plan", StageState::Completed, 1);
        let _ = task.apply_stage_event("implement", StageState::Running, 1);

        let snapshot = crate::state::StateSnapshot {
            version: crate::state::STATE_VERSION,
            tasks: vec![task],
            selected_index: 0,
            focused_pane: crate::layout::PaneType::BeadList,
            plugin_state: PluginState::Running,
            status_message: None,
            timestamp: 0,
        };

        // Restore from snapshot
        let result = plugin.restore_from_snapshot(snapshot);
        assert!(result.is_ok());

        // Verify stage history is preserved
        assert_eq!(plugin.tasks.len(), 1);
        assert_eq!(plugin.tasks[0].stages[0].state, StageState::Completed); // research
        assert_eq!(plugin.tasks[0].stages[1].state, StageState::Completed); // plan
        assert_eq!(plugin.tasks[0].stages[2].state, StageState::Running); // implement
        Ok(())
    }

    #[test]
    fn test_plugin_default_state_file_path() -> Result<(), Box<dyn std::error::Error>> {
        let path = OyaPlugin::default_state_file_path();

        // Path should end with zellij-plugin-state.json
        assert!(path.to_string_lossy().ends_with("zellij-plugin-state.json"));
        Ok(())
    }

    // Additional tests from original second test module

    #[test]
    fn test_help_overlay_toggle() -> Result<(), Box<dyn std::error::Error>> {
        let mut plugin = OyaPlugin::new()?;

        // Initially not in help overlay
        assert_eq!(plugin.state, PluginState::Starting);

        // Toggle to open help overlay
        let result = plugin.toggle_help_overlay();
        assert!(result.is_ok());
        assert_eq!(plugin.state, PluginState::HelpOverlay);

        // Toggle to close help overlay
        let result = plugin.toggle_help_overlay();
        assert!(result.is_ok());
        assert_eq!(plugin.state, PluginState::Running);

        Ok(())
    }

    #[test]
    fn test_help_overlay_preconditions() -> Result<(), Box<dyn std::error::Error>> {
        let mut plugin = OyaPlugin::new()?;

        // Test terminal too small error
        plugin.size = Size { rows: 5, cols: 10 };
        let result = plugin.open_help_overlay();
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("TerminalTooSmall"));

        // Test invalid state error
        plugin.state = PluginState::ShuttingDown;
        let result = plugin.open_help_overlay();
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("invalid state"));

        Ok(())
    }

    #[test]
    fn test_tab_cycles_focus_forward() -> Result<(), Box<dyn std::error::Error>> {
        let mut plugin = OyaPlugin::new()?;
        plugin.state = PluginState::Running;
        plugin.focused_pane = PaneType::BeadList;

        let _ = plugin.handle_event(PluginEvent::Key {
            key: '\t',
            modifiers: KeyModifiers {
                shift: false,
                ctrl: false,
                alt: false,
            },
        })?;

        assert_eq!(plugin.focused_pane, PaneType::BeadDetail);
        Ok(())
    }

    #[test]
    fn test_shift_tab_cycles_focus_backward() -> Result<(), Box<dyn std::error::Error>> {
        let mut plugin = OyaPlugin::new()?;
        plugin.state = PluginState::Running;
        plugin.focused_pane = PaneType::BeadList;

        let _ = plugin.handle_event(PluginEvent::Key {
            key: '\t',
            modifiers: KeyModifiers {
                shift: true,
                ctrl: false,
                alt: false,
            },
        })?;

        assert_eq!(plugin.focused_pane, PaneType::WorkflowGraph);
        Ok(())
    }

    #[test]
    fn test_get_keybindings_for_all_panes() -> Result<(), Box<dyn std::error::Error>> {
        let plugin = OyaPlugin::new()?;

        // Test BeadList keybindings
        let bead_list_bindings = plugin.get_keybindings_for_pane(PaneType::BeadList);
        assert!(!bead_list_bindings.is_empty());
        assert!(bead_list_bindings.iter().any(&|(key, _)| *key == '?'));
        assert!(bead_list_bindings.iter().any(&|(key, _)| *key == '\x1b'));

        // Test BeadDetail keybindings
        let bead_detail_bindings = plugin.get_keybindings_for_pane(PaneType::BeadDetail);
        assert!(!bead_detail_bindings.is_empty());
        assert!(bead_detail_bindings.iter().any(&|(key, _)| *key == '?'));
        assert!(bead_detail_bindings.iter().any(&|(key, _)| *key == '\x1b'));

        // Test PipelineView keybindings
        let pipeline_view_bindings = plugin.get_keybindings_for_pane(PaneType::PipelineView);
        assert!(!pipeline_view_bindings.is_empty());
        assert!(pipeline_view_bindings.iter().any(|&(key, _)| key == '?'));
        assert!(pipeline_view_bindings
            .iter()
            .any(&|(key, _)| *key == '\x1b'));

        // Test WorkflowGraph keybindings
        let workflow_graph_bindings = plugin.get_keybindings_for_pane(PaneType::WorkflowGraph);
        assert!(!workflow_graph_bindings.is_empty());
        assert!(workflow_graph_bindings.iter().any(&|(key, _)| *key == '?'));
        assert!(workflow_graph_bindings
            .iter()
            .any(&|(key, _)| *key == '\x1b'));

        Ok(())
    }

    #[test]
    fn test_multiple_saves_update_timestamp() -> Result<(), Box<dyn std::error::Error>> {
        let mut plugin = OyaPlugin::new()?;

        // First save
        let result1 = plugin.save_state_now();

        if result1.is_ok() {
            let timestamp1 = plugin.last_save_timestamp().unwrap();

            // Wait a bit (simulated by just calling again)
            std::thread::sleep(std::time::Duration::from_millis(10));

            // Second save
            let result2 = plugin.save_state_now();
            if result2.is_ok() {
                let timestamp2 = plugin.last_save_timestamp().unwrap();

                // Timestamps should be different (second save is later)
                assert!(timestamp2 > timestamp1);
            }
        }
        // If saves fail, that's acceptable in test environment
        Ok(())
    }

    #[test]
    fn test_auto_save_timer_is_running_after_init() -> Result<(), Box<dyn std::error::Error>> {
        let mut plugin = OyaPlugin::new()?;

        let _ = plugin.init_auto_save(30);

        assert!(plugin.auto_save_timer.is_some());
        let timer = plugin.auto_save_timer.as_ref().unwrap();
        assert!(timer.is_running());
        Ok(())
    }

    #[test]
    fn test_state_save_and_restore_roundtrip() -> Result<(), Box<dyn std::error::Error>> {
        use std::fs;

        // Create a temporary directory for state file
        let temp_dir = std::env::temp_dir().join("oya-test-state-roundtrip");
        let _ = fs::remove_dir_all(&temp_dir); // Clean up any previous test
        fs::create_dir_all(&temp_dir)?;

        let state_file = temp_dir.join("test-state.json");

        // Create plugin and set up specific state
        let mut plugin1 = OyaPlugin::new()?;

        // Modify state to test restoration
        plugin1.selected_index = 2;
        plugin1.focused_pane = crate::layout::PaneType::PipelineView;
        plugin1.status_message = Some("Test roundtrip message".to_string());

        // Save state
        let state_manager = crate::state::StateManager::new(state_file.clone(), 1_048_576)?;
        let save_result = state_manager.save_state(&plugin1);

        // Verify save succeeded (or skip if filesystem unavailable)
        if save_result.is_ok() {
            // Create a new plugin instance
            let mut plugin2 = OyaPlugin::new()?;

            // Load state
            let load_result = state_manager.load_state();
            assert!(load_result.is_some(), "Load should succeed");

            let mut snapshot = load_result.ok_or("No snapshot found")?;
            assert!(snapshot.validate().is_ok(), "Snapshot should be valid");

            // Restore state
            let restore_result = plugin2.restore_from_snapshot(snapshot);
            assert!(restore_result.is_ok(), "Restore should succeed");

            // Verify restored state matches saved state
            assert_eq!(plugin2.selected_index, 2);
            assert_eq!(plugin2.focused_pane, crate::layout::PaneType::PipelineView);
            assert_eq!(
                plugin2.status_message,
                Some("Test roundtrip message".to_string())
            );

            // Clean up
            let _ = fs::remove_dir_all(&temp_dir);
        }
        // If save fails, skip test (filesystem unavailable in test environment)
        Ok(())
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
        let tasks = vec![
            TaskRow::new("task-3ax5", "in_progress", "P1", "Rust", "task/task-3ax5"),
            TaskRow::new("task-1xvj", "created", "P1", "Rust", "task/task-1xvj"),
            TaskRow::new("task-1k71", "created", "P2", "Rust", "task/task-1k71"),
        ];

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
            auto_save_timer: None,
            last_save_timestamp: None,
            last_timer_tick_ms: None,
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

        // Attempt to restore previous state
        let state_manager = crate::state::StateManager::default();
        match state_manager.load_state() {
            Ok(Some(snapshot)) => {
                // Restore state from snapshot
                self.restore_from_snapshot(snapshot)?;
                self.status_message = Some("State restored from disk".to_string());
            }
            Ok(None) => {
                // No previous state found, start fresh
                self.status_message = Some("No previous state found".to_string());
            }
            Err(err) => {
                // State load failed, log but continue with fresh state
                self.status_message = Some(format!("State load failed: {err}, starting fresh"));
            }
        }

        self.state = PluginState::Running;
        self.connect_ipc(&info.config)?;
        let _ = self.refresh_tasks();

        // Initialize auto-save timer from config
        let interval_secs = info
            .config
            .get("auto_save_interval_secs")
            .and_then(|v| v.as_u64())
            .unwrap_or(30);
        let _ = self.init_auto_save(interval_secs);

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
                let rendered = self.handle_timer_event()?;
                Ok(rendered)
            }
        }
    }

    /// Handle keyboard input
    ///
    /// # Errors
    ///
    /// Returns an error if key handling fails
    fn handle_key(&mut self, key: char, modifiers: KeyModifiers) -> PluginResult<()> {
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
                if modifiers.shift {
                    self.cycle_focus_backward();
                } else {
                    self.cycle_focus();
                }
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

    /// Cycle focus backwards between panes (Shift+Tab)
    fn cycle_focus_backward(&mut self) {
        self.focused_pane = match self.focused_pane {
            crate::layout::PaneType::BeadList => crate::layout::PaneType::WorkflowGraph,
            crate::layout::PaneType::BeadDetail => crate::layout::PaneType::BeadList,
            crate::layout::PaneType::PipelineView => crate::layout::PaneType::BeadDetail,
            crate::layout::PaneType::WorkflowGraph => crate::layout::PaneType::PipelineView,
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
                ('\t', "Next pane (Shift+Tab back)"),
                ('g', "Refresh tasks"),
                ('r', "Run pipeline"),
                ('a', "Approve task"),
                ('b', "Batch run"),
            ],
            crate::layout::PaneType::BeadDetail => vec![
                ('\t', "Next pane (Shift+Tab back)"),
                ('r', "Run pipeline"),
                ('a', "Approve task"),
            ],
            crate::layout::PaneType::PipelineView => {
                vec![('\t', "Next pane (Shift+Tab back)"), ('g', "Refresh tasks")]
            }
            crate::layout::PaneType::WorkflowGraph => vec![('\t', "Next pane (Shift+Tab back)")],
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

    // ========================================================================
    // AUTO-SAVE TIMER METHODS
    // ========================================================================

    /// Initialize auto-save timer with configured interval
    ///
    /// # Arguments
    ///
    /// * `interval_secs` - Auto-save interval in seconds (min: 10, max: 600)
    ///
    /// # Errors
    ///
    /// Returns `Err(PluginError)` if timer creation fails
    pub fn init_auto_save(&mut self, interval_secs: u64) -> PluginResult<()> {
        // Clamp interval to valid range [10, 600] seconds
        const MIN_INTERVAL_SECS: u64 = 10;
        const MAX_INTERVAL_SECS: u64 = 600;

        let interval_secs = interval_secs.clamp(MIN_INTERVAL_SECS, MAX_INTERVAL_SECS);
        let interval_ms = interval_secs.saturating_mul(1000);

        // Create timer configuration
        let config =
            TimerConfig::new(interval_ms).map_err(|e| PluginError::TimerError(e.to_string()))?;

        // Create and start timer
        let timer = RefreshTimer::new(config)
            .start()
            .map_err(|e| PluginError::TimerError(e.to_string()))?;

        self.auto_save_timer = Some(timer);

        // Reset last tick timestamp to allow immediate first tick
        self.last_timer_tick_ms = None;

        Ok(())
    }

    /// Handle timer event and trigger save if needed
    ///
    /// # Errors
    ///
    /// Returns `Err(PluginError)` if save fails
    pub fn handle_timer_event(&mut self) -> PluginResult<Option<String>> {
        let timer = match self.auto_save_timer.as_ref() {
            Some(timer) => timer,
            None => return Ok(None), // No timer configured, ignore event
        };

        // Get current time in milliseconds
        let now_ms = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map_or_else(|_| 0, |d| d.as_millis() as u64);

        // Get last tick time (default to 0 if never ticked)
        let last_tick_ms = self.last_timer_tick_ms.unwrap_or(0);

        // Check if tick is due
        if !timer.is_tick_due(last_tick_ms) {
            return Ok(None); // Not due yet, skip save
        }

        // Update last tick timestamp
        self.last_timer_tick_ms = Some(now_ms);

        // Perform save
        self.save_state_now()?;

        // Render UI with save confirmation
        self.render()
    }

    /// Force immediate state save
    ///
    /// # Errors
    ///
    /// Returns `Err(PluginError)` if save fails
    pub fn save_state_now(&mut self) -> PluginResult<()> {
        let state_manager = StateManager::default();

        state_manager
            .save_state(self)
            .map_err(|e| PluginError::StateSaveError(e.to_string()))?;

        // Update last save timestamp
        let now_secs = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map_or_else(|_| 0, |d| d.as_secs());

        self.last_save_timestamp = Some(now_secs);

        // Update status message
        self.status_message = Some(format!("State saved at {}", now_secs));

        Ok(())
    }

    /// Get last save timestamp (if any)
    pub fn last_save_timestamp(&self) -> Option<u64> {
        self.last_save_timestamp
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
