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
use crate::state::StateManager;
use crate::timer::{RefreshTimer, TimerConfig};
use crate::{command::parse_command, command::ParsedCommand};
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
    /// Last configured IPC address for reconnect attempts
    ipc_address: String,
    /// Last IPC connect attempt timestamp (unix ms)
    last_ipc_connect_attempt_ms: Option<u64>,
    /// Status message shown in the UI
    status_message: Option<String>,
    /// Auto-save timer for periodic state saves
    auto_save_timer: Option<RefreshTimer>,
    /// Last successful save timestamp (unix seconds)
    last_save_timestamp: Option<u64>,
    /// Last timer tick timestamp (unix milliseconds)
    last_timer_tick_ms: Option<u64>,
    /// Input mode for modal keyboard behavior
    input_mode: InputMode,
    /// Command buffer used while in command mode
    command_buffer: String,
    /// Search buffer used while in search mode
    search_buffer: String,
    /// Last search pattern for n/N navigation
    last_search_pattern: Option<String>,
    /// Original unfiltered tasks when command filter is active
    unfiltered_tasks: Vec<TaskRow>,
    /// Pending g prefix in normal mode for gg support
    pending_g: bool,
    /// Visual mode selection anchor index
    visual_anchor: Option<usize>,
    /// Horizontal scroll offset for detail panes (BeadDetail, PipelineView, WorkflowGraph)
    horizontal_scroll: usize,
}

/// Input mode state machine for keyboard handling.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum InputMode {
    Normal,
    Command,
    Search,
    Visual,
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

/// Substep state within a stage
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum SubStepState {
    /// Substep not started
    NotStarted,
    /// Substep currently running
    Running,
    /// Substep completed successfully
    Completed,
    /// Substep failed
    Failed,
}

/// A substep within a pipeline stage
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct SubStep {
    /// Substep name
    pub name: String,
    /// Current state of the substep
    pub state: SubStepState,
}

impl SubStep {
    /// Create a new substep
    #[must_use]
    pub fn new(name: &str, state: SubStepState) -> Self {
        Self {
            name: name.to_string(),
            state,
        }
    }

    /// Get display symbol for this substep state
    #[must_use]
    pub const fn symbol(&self) -> &'static str {
        match self.state {
            SubStepState::NotStarted => "○",
            SubStepState::Running => "●",
            SubStepState::Completed => "✓",
            SubStepState::Failed => "✗",
        }
    }

    /// Get display string with symbol and name
    #[must_use]
    pub fn display(&self) -> String {
        format!("{} {}", self.symbol(), self.name)
    }
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
    /// Substeps within this stage
    pub substeps: Vec<SubStep>,
}

impl StageInfo {
    /// Create new stage info
    pub fn new(name: &str) -> Self {
        Self {
            name: name.to_string(),
            state: StageState::NotStarted,
            attempt: 1,
            substeps: Vec::new(),
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

        let snapshot = crate::state::StateSnapshot {
            version: 999,
            tasks: vec![],
            selected_index: 0,
            focused_pane: crate::layout::PaneType::BeadList,
            plugin_state: PluginState::Running,
            status_message: None,
            timestamp: 0,
        };

        let result = plugin.restore_from_snapshot(snapshot);
        assert!(result.is_err());
        let error_msg = result.unwrap_err().to_string();
        assert!(error_msg.contains("version") || error_msg.contains("incompatible"));
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
    fn test_connect_ipc_gracefully_degrades_on_invalid_address(
    ) -> Result<(), Box<dyn std::error::Error>> {
        let mut plugin = OyaPlugin::new()?;
        let config = serde_json::json!({ "ipc_address": "not-an-address" });

        let result = plugin.connect_ipc(&config);

        assert!(result.is_ok());
        assert!(plugin.ipc.is_none());
        assert!(plugin
            .status_message
            .as_deref()
            .is_some_and(|msg| msg.contains("IPC unavailable")));
        Ok(())
    }

    #[test]
    fn test_refresh_tasks_reports_action_specific_skip_when_throttled(
    ) -> Result<(), Box<dyn std::error::Error>> {
        let mut plugin = OyaPlugin::new()?;
        plugin.ipc = None;
        plugin.last_ipc_connect_attempt_ms = Some(OyaPlugin::now_ms());

        let result = plugin.refresh_tasks();

        assert!(result.is_ok());
        assert_eq!(
            plugin.status_message.as_deref(),
            Some("Refresh skipped: IPC reconnect throttled")
        );
        Ok(())
    }

    #[test]
    fn test_send_task_command_reports_action_specific_skip_when_throttled(
    ) -> Result<(), Box<dyn std::error::Error>> {
        let mut plugin = OyaPlugin::new()?;
        plugin.ipc = None;
        plugin.last_ipc_connect_attempt_ms = Some(OyaPlugin::now_ms());

        let result = plugin.send_task_command(GuestMessage::RunPipeline {
            slug: "task-3ax5".to_string(),
            dry_run: false,
        });

        assert!(result.is_ok());
        assert_eq!(
            plugin.status_message.as_deref(),
            Some("Run pipeline skipped: IPC reconnect throttled")
        );
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

        // Set to Running state to allow toggle
        plugin.state = PluginState::Running;

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
        // Design: Plugin starts in Starting state, must be Running to open help overlay
        plugin.state = PluginState::Running;
        plugin.size = Size { rows: 5, cols: 10 };
        let result = plugin.open_help_overlay();
        assert!(result.is_err());
        // Design: Help overlay should validate terminal size (minimum 10x40)
        // The error message should clearly communicate this constraint
        let error_msg = result.unwrap_err().to_string();
        assert!(
            error_msg.contains("Terminal") || error_msg.contains("small"),
            "Expected error about terminal size being too small, got: {}",
            error_msg
        );

        // Test invalid state error
        // Design: Help overlay can only be opened from Running state
        plugin.state = PluginState::ShuttingDown;
        let result = plugin.open_help_overlay();
        assert!(result.is_err());
        let error_msg = result.unwrap_err().to_string();
        assert!(
            error_msg.contains("invalid state"),
            "Expected 'invalid state' error, got: {}",
            error_msg
        );

        Ok(())
    }

    #[test]
    fn test_get_keybindings_for_all_panes() -> Result<(), Box<dyn std::error::Error>> {
        let plugin = OyaPlugin::new()?;

        // Test BeadList keybindings
        let bead_list_bindings = plugin.get_keybindings_for_pane(crate::layout::PaneType::BeadList);
        assert!(!bead_list_bindings.is_empty());
        assert!(bead_list_bindings
            .iter()
            .any(|&(key, _): &(char, _)| key == '?'));
        assert!(bead_list_bindings
            .iter()
            .any(|&(key, _): &(char, _)| key == '\x1b'));

        // Test BeadDetail keybindings
        let bead_detail_bindings =
            plugin.get_keybindings_for_pane(crate::layout::PaneType::BeadDetail);
        assert!(!bead_detail_bindings.is_empty());
        assert!(bead_detail_bindings
            .iter()
            .any(|&(key, _): &(char, _)| key == '?'));
        assert!(bead_detail_bindings
            .iter()
            .any(|&(key, _): &(char, _)| key == '\x1b'));

        // Test PipelineView keybindings
        let pipeline_view_bindings =
            plugin.get_keybindings_for_pane(crate::layout::PaneType::PipelineView);
        assert!(!pipeline_view_bindings.is_empty());
        assert!(pipeline_view_bindings
            .iter()
            .any(|&(key, _): &(char, _)| key == '?'));
        assert!(pipeline_view_bindings
            .iter()
            .any(|&(key, _): &(char, _)| key == '\x1b'));

        // Test WorkflowGraph keybindings
        let workflow_graph_bindings =
            plugin.get_keybindings_for_pane(crate::layout::PaneType::WorkflowGraph);
        assert!(!workflow_graph_bindings.is_empty());
        assert!(workflow_graph_bindings
            .iter()
            .any(|&(key, _): &(char, _)| key == '?'));
        assert!(workflow_graph_bindings
            .iter()
            .any(|&(key, _): &(char, _)| key == '\x1b'));

        Ok(())
    }

    #[test]
    fn test_multiple_saves_update_timestamp() -> Result<(), Box<dyn std::error::Error>> {
        let mut plugin = OyaPlugin::new()?;

        let result1 = plugin.save_state_now();
        if result1.is_ok() {
            let timestamp1 = plugin.last_save_timestamp().unwrap();
            std::thread::sleep(std::time::Duration::from_millis(10));
            let result2 = plugin.save_state_now();
            if result2.is_ok() {
                let timestamp2 = plugin.last_save_timestamp().unwrap();
                assert!(timestamp2 > timestamp1);
            }
        }

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

        let temp_dir = std::env::temp_dir().join("oya-test-state-roundtrip");
        let _ = fs::remove_dir_all(&temp_dir);
        fs::create_dir_all(&temp_dir)?;

        let state_file = temp_dir.join("test-state.json");
        let mut plugin1 = OyaPlugin::new()?;
        plugin1.selected_index = 2;
        plugin1.focused_pane = crate::layout::PaneType::PipelineView;
        plugin1.status_message = Some("Test roundtrip message".to_string());

        let state_manager = crate::state::StateManager::new(state_file.clone(), 1_048_576)?;
        let save_result = state_manager.save_state(&plugin1);

        if save_result.is_ok() {
            let mut plugin2 = OyaPlugin::new()?;
            let load_result = state_manager.load_state();
            assert!(load_result.is_ok(), "Load should succeed");

            let snapshot_option = load_result?;
            let mut snapshot = snapshot_option.ok_or("No snapshot found")?;
            assert!(snapshot.validate().is_ok(), "Snapshot should be valid");

            let restore_result = plugin2.restore_from_snapshot(snapshot);
            assert!(restore_result.is_ok(), "Restore should succeed");

            assert_eq!(plugin2.selected_index, 2);
            assert_eq!(plugin2.focused_pane, crate::layout::PaneType::PipelineView);
            assert_eq!(
                plugin2.status_message,
                Some("Test roundtrip message".to_string())
            );

            let _ = fs::remove_dir_all(&temp_dir);
        }

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
            ipc_address: "127.0.0.1:5555".to_string(),
            last_ipc_connect_attempt_ms: None,
            status_message: None,
            auto_save_timer: None,
            last_save_timestamp: None,
            last_timer_tick_ms: None,
            input_mode: InputMode::Normal,
            command_buffer: String::new(),
            search_buffer: String::new(),
            last_search_pattern: None,
            unfiltered_tasks: Vec::new(),
            pending_g: false,
            visual_anchor: None,
            horizontal_scroll: 0,
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
        // Handle help overlay keys.
        if key == '?' {
            return self.toggle_help_overlay().map(|_| ());
        }

        if self.state == PluginState::HelpOverlay {
            if key == '\x1b' {
                return self.toggle_help_overlay().map(|_| ());
            }
            return Ok(());
        }

        if key == '\x1b' {
            self.input_mode = InputMode::Normal;
            self.pending_g = false;
            self.command_buffer.clear();
            self.search_buffer.clear();
            self.visual_anchor = None;
            self.status_message = Some("Normal mode".to_string());
            return Ok(());
        }

        if modifiers.ctrl && modifiers.shift {
            match key {
                'g' | 'G' => {
                    self.focused_pane = crate::layout::PaneType::WorkflowGraph;
                    self.status_message = Some("Focus: Workflow Graph".to_string());
                    return Ok(());
                }
                'l' | 'L' => {
                    self.focused_pane = crate::layout::PaneType::BeadList;
                    self.status_message = Some("Focus: Bead List".to_string());
                    return Ok(());
                }
                'a' | 'A' => {
                    let _ = self.approve_selected(false);
                    return Ok(());
                }
                _ => {}
            }
        }

        match self.input_mode {
            InputMode::Command => return self.handle_command_mode_key(key),
            InputMode::Search => return self.handle_search_mode_key(key),
            InputMode::Visual => {
                return self.handle_visual_mode_key(key);
            }
            InputMode::Normal => {}
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
            // Mode entry
            ':' => {
                self.input_mode = InputMode::Command;
                self.command_buffer.clear();
                self.status_message = Some("Command mode: :".to_string());
            }
            '/' => {
                self.input_mode = InputMode::Search;
                self.search_buffer.clear();
                self.status_message = Some("Search mode: /".to_string());
            }
            'v' | 'V' => {
                self.input_mode = InputMode::Visual;
                self.visual_anchor = Some(self.selected_index);
                self.status_message = Some("Visual mode".to_string());
            }
            // Neovim-style pane and list navigation
            'h' | 'H' => {
                self.cycle_focus_backward();
            }
            'l' | 'L' => {
                self.cycle_focus();
            }
            'j' | 'J' => {
                if self.focused_pane == crate::layout::PaneType::BeadList {
                    self.move_selection(1)?;
                }
                self.pending_g = false;
            }
            'k' | 'K' => {
                if self.focused_pane == crate::layout::PaneType::BeadList {
                    self.move_selection(-1)?;
                }
                self.pending_g = false;
            }
            'g' => {
                if self.pending_g {
                    self.selected_index = 0;
                    self.pending_g = false;
                } else {
                    self.pending_g = true;
                }
            }
            'G' => {
                if !self.tasks.is_empty() {
                    self.selected_index = self.tasks.len().saturating_sub(1);
                }
                self.pending_g = false;
            }
            'n' => {
                self.move_to_next_search_match(true);
                self.pending_g = false;
            }
            'N' => {
                self.move_to_next_search_match(false);
                self.pending_g = false;
            }
            // Existing actions
            'r' | 'R' => {
                let _ = self.run_pipeline_for_selected(false);
                self.pending_g = false;
            }
            'a' | 'A' => {
                let _ = self.approve_selected(false);
                self.pending_g = false;
            }
            'b' | 'B' => {
                let _ = self.run_pipeline_for_all(false);
                self.pending_g = false;
            }
            // keep refresh key
            'x' | 'X' => {
                let _ = self.refresh_tasks();
                self.pending_g = false;
            }
            _ => {
                // Other keys ignored
                self.pending_g = false;
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
            crate::layout::PaneType::WorkflowGraph => crate::layout::PaneType::AgentView,
            crate::layout::PaneType::AgentView => crate::layout::PaneType::BeadList,
        };
    }

    fn cycle_focus_backward(&mut self) {
        self.focused_pane = match self.focused_pane {
            crate::layout::PaneType::BeadList => crate::layout::PaneType::AgentView,
            crate::layout::PaneType::BeadDetail => crate::layout::PaneType::BeadList,
            crate::layout::PaneType::PipelineView => crate::layout::PaneType::BeadDetail,
            crate::layout::PaneType::WorkflowGraph => crate::layout::PaneType::PipelineView,
            crate::layout::PaneType::AgentView => crate::layout::PaneType::WorkflowGraph,
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

        let last = len.saturating_sub(1);
        self.selected_index = if direction > 0 {
            if self.selected_index >= last {
                0
            } else {
                self.selected_index.saturating_add(1)
            }
        } else if self.selected_index == 0 {
            last
        } else {
            self.selected_index.saturating_sub(1)
        };

        Ok(())
    }

    fn handle_visual_mode_key(&mut self, key: char) -> PluginResult<()> {
        match key {
            'v' | 'V' => {
                self.input_mode = InputMode::Normal;
                self.visual_anchor = None;
                self.status_message = Some("Normal mode".to_string());
            }
            'j' | 'J' => {
                self.move_selection(1)?;
            }
            'k' | 'K' => {
                self.move_selection(-1)?;
            }
            'h' | 'H' => self.cycle_focus_backward(),
            'l' | 'L' => self.cycle_focus(),
            _ => {}
        }

        Ok(())
    }

    fn handle_command_mode_key(&mut self, key: char) -> PluginResult<()> {
        match key {
            '\n' | '\r' => {
                let command_text = format!(":{}", self.command_buffer.trim());
                if command_text == ":" {
                    self.status_message = Some("No command entered".to_string());
                } else {
                    match parse_command(&command_text) {
                        Ok(parsed) => self.apply_parsed_command(parsed),
                        Err(err) => {
                            self.status_message = Some(format!("Command error: {err}"));
                        }
                    }
                }

                self.command_buffer.clear();
                self.input_mode = InputMode::Normal;
                Ok(())
            }
            '\x08' | '\x7f' => {
                self.command_buffer.pop();
                self.status_message = Some(format!("Command mode: :{}", self.command_buffer));
                Ok(())
            }
            c if !c.is_control() => {
                self.command_buffer.push(c);
                self.status_message = Some(format!("Command mode: :{}", self.command_buffer));
                Ok(())
            }
            _ => Ok(()),
        }
    }

    fn handle_search_mode_key(&mut self, key: char) -> PluginResult<()> {
        match key {
            '\n' | '\r' => {
                let pattern = self.search_buffer.trim().to_string();
                if pattern.is_empty() {
                    self.status_message = Some("Search cancelled".to_string());
                } else {
                    self.last_search_pattern = Some(pattern.clone());
                    self.move_to_next_search_match(true);
                }

                self.search_buffer.clear();
                self.input_mode = InputMode::Normal;
                Ok(())
            }
            '\x08' | '\x7f' => {
                self.search_buffer.pop();
                self.status_message = Some(format!("Search mode: /{}", self.search_buffer));
                Ok(())
            }
            c if !c.is_control() => {
                self.search_buffer.push(c);
                self.status_message = Some(format!("Search mode: /{}", self.search_buffer));
                Ok(())
            }
            _ => Ok(()),
        }
    }

    fn apply_parsed_command(&mut self, command: ParsedCommand) {
        match command {
            ParsedCommand::Filter { pattern } => {
                if self.unfiltered_tasks.is_empty() {
                    self.unfiltered_tasks = self.tasks.clone();
                }

                self.tasks = self
                    .unfiltered_tasks
                    .iter()
                    .filter(|task| Self::task_matches(task, &pattern))
                    .cloned()
                    .collect();

                self.selected_index = self.selected_index.min(self.tasks.len().saturating_sub(1));
                self.status_message = Some(format!(
                    "Filter applied: '{}' ({} tasks)",
                    pattern,
                    self.tasks.len()
                ));
            }
            ParsedCommand::ClearFilter => {
                if !self.unfiltered_tasks.is_empty() {
                    self.tasks = self.unfiltered_tasks.clone();
                    self.unfiltered_tasks.clear();
                    self.selected_index =
                        self.selected_index.min(self.tasks.len().saturating_sub(1));
                }
                self.status_message = Some("Filter cleared".to_string());
            }
            ParsedCommand::Refresh => {
                if !self.unfiltered_tasks.is_empty() {
                    self.unfiltered_tasks.clear();
                }
                let _ = self.refresh_tasks();
            }
            ParsedCommand::Help => {
                self.state = PluginState::Running;
                let _ = self.toggle_help_overlay();
            }
            ParsedCommand::Export { path } => {
                let export_result = self.export_tasks_to_file(&path);
                match export_result {
                    Ok(count) => {
                        self.status_message = Some(format!("Exported {count} tasks to '{path}'"));
                    }
                    Err(err) => {
                        self.status_message = Some(format!("Export error: {err}"));
                    }
                }
            }
        }
    }

    fn move_to_next_search_match(&mut self, forward: bool) {
        let pattern = match self.last_search_pattern.as_deref() {
            Some(pattern) if !pattern.is_empty() => pattern.to_lowercase(),
            _ => {
                self.status_message = Some("No active search pattern".to_string());
                return;
            }
        };

        if self.tasks.is_empty() {
            self.status_message = Some("No tasks available".to_string());
            return;
        }

        let len = self.tasks.len();

        #[allow(clippy::indexing_slicing, clippy::arithmetic_side_effects)]
        for step in 1..=len {
            let idx = if forward {
                (self.selected_index + step) % len
            } else {
                (self.selected_index + len - (step % len)) % len
            };
            // idx is always in [0, len) due to modulo operation
            if Self::task_matches(&self.tasks[idx], &pattern) {
                self.selected_index = idx;
                self.status_message = Some(format!("Search match: {}", self.tasks[idx].slug));
                return;
            }
        }

        self.status_message = Some("No matches found".to_string());
    }

    fn task_matches(task: &TaskRow, pattern: &str) -> bool {
        let pattern = pattern.to_lowercase();
        task.slug.to_lowercase().contains(&pattern)
            || task.status.to_lowercase().contains(&pattern)
            || task.priority.to_lowercase().contains(&pattern)
            || task.language.to_lowercase().contains(&pattern)
            || task
                .stage
                .as_ref()
                .is_some_and(|stage| stage.to_lowercase().contains(&pattern))
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

        self.ipc_address = address.to_string();

        match IpcClient::connect(address) {
            Ok(client) => {
                self.ipc = Some(client);
                self.last_ipc_connect_attempt_ms = None;
                self.status_message = Some(format!("IPC connected to {address}"));
                Ok(())
            }
            Err(err) => {
                self.ipc = None;
                self.last_ipc_connect_attempt_ms = Some(Self::now_ms());
                self.status_message = Some(format!("IPC unavailable: {err}"));
                Ok(())
            }
        }
    }

    fn refresh_tasks(&mut self) -> PluginResult<()> {
        if !self.ensure_ipc_connected_for_action("Refresh") {
            return Ok(());
        }

        let ipc = match self.ipc.as_mut() {
            Some(ipc) => ipc,
            None => return Ok(()),
        };

        match ipc.request(GuestMessage::GetTaskList) {
            Ok(HostMessage::TaskList { tasks }) => {
                self.tasks = tasks.into_iter().map(task_summary_to_row).collect();
                self.unfiltered_tasks.clear();
                self.selected_index = self.selected_index.min(self.tasks.len().saturating_sub(1));
                self.status_message = Some("Tasks refreshed".to_string());
                Ok(())
            }
            Ok(message) => {
                self.status_message = Some(format!("Unexpected response: {message:?}"));
                Ok(())
            }
            Err(err) => {
                self.ipc = None;
                self.status_message = Some(format!("IPC error: {err}"));
                Ok(())
            }
        }
    }

    fn export_tasks_to_file(&self, path: &str) -> PluginResult<usize> {
        use std::fs::File;
        use std::io::Write;

        let json = serde_json::to_string_pretty(&self.tasks)
            .map_err(|e| PluginError::StateSaveError(format!("JSON serialization failed: {e}")))?;

        let mut file = File::create(path)
            .map_err(|e| PluginError::StateSaveError(format!("File creation failed: {e}")))?;

        file.write_all(json.as_bytes())
            .map_err(|e| PluginError::StateSaveError(format!("Write failed: {e}")))?;

        Ok(self.tasks.len())
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
        let action = Self::action_name_for_message(&message);
        if !self.ensure_ipc_connected_for_action(action) {
            return Ok(());
        }

        let ipc = match self.ipc.as_mut() {
            Some(ipc) => ipc,
            None => return Ok(()),
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
                self.ipc = None;
                self.status_message = Some(format!("IPC error: {err}"));
                Ok(())
            }
        }
    }

    fn action_name_for_message(message: &GuestMessage) -> &'static str {
        match message {
            GuestMessage::RunPipeline { .. } => "Run pipeline",
            GuestMessage::RunPipelineBatch { .. } => "Batch run",
            GuestMessage::ApproveTask { .. } => "Approve task",
            _ => "Action",
        }
    }

    fn ensure_ipc_connected_for_action(&mut self, action: &str) -> bool {
        if self.ensure_ipc_connected() {
            return true;
        }

        let detail = self
            .status_message
            .clone()
            .unwrap_or_else(|| "IPC unavailable".to_string());
        self.status_message = Some(format!("{action} skipped: {detail}"));
        false
    }

    fn ensure_ipc_connected(&mut self) -> bool {
        if self.ipc.is_some() {
            return true;
        }

        const IPC_RECONNECT_COOLDOWN_MS: u64 = 250;
        let now_ms = Self::now_ms();
        let should_throttle = self
            .last_ipc_connect_attempt_ms
            .is_some_and(|last_attempt_ms| {
                now_ms.saturating_sub(last_attempt_ms) < IPC_RECONNECT_COOLDOWN_MS
            });

        if should_throttle {
            self.status_message = Some("IPC reconnect throttled".to_string());
            return false;
        }

        self.last_ipc_connect_attempt_ms = Some(now_ms);

        match IpcClient::connect(&self.ipc_address) {
            Ok(client) => {
                self.ipc = Some(client);
                self.last_ipc_connect_attempt_ms = None;
                self.status_message = Some(format!("IPC reconnected to {}", self.ipc_address));
                true
            }
            Err(err) => {
                self.status_message = Some(format!("IPC unavailable: {err}"));
                false
            }
        }
    }

    fn now_ms() -> u64 {
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map_or_else(|_| 0, |d| d.as_millis() as u64)
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
                ('?', "Help"),
                ('\x1b', "Escape"),
                ('j', "Move down"),
                ('k', "Move up"),
                ('h', "Previous pane"),
                ('l', "Next pane"),
                ('\t', "Switch pane"),
                ('G', "Go bottom"),
                ('g', "g-prefix / gg top"),
                ('/', "Search mode"),
                (':', "Command mode"),
                ('n', "Next search match"),
                ('N', "Prev search match"),
                ('r', "Run pipeline"),
                ('a', "Approve task"),
                ('b', "Batch run"),
                ('x', "Refresh tasks"),
            ],
            crate::layout::PaneType::BeadDetail => vec![
                ('?', "Help"),
                ('\x1b', "Escape"),
                ('h', "Previous pane"),
                ('l', "Next pane"),
                ('\t', "Switch pane"),
                ('r', "Run pipeline"),
                ('a', "Approve task"),
            ],
            crate::layout::PaneType::PipelineView => {
                vec![
                    ('?', "Help"),
                    ('\x1b', "Escape"),
                    ('h', "Previous pane"),
                    ('l', "Next pane"),
                    ('\t', "Switch pane"),
                    ('x', "Refresh tasks"),
                ]
            }
            crate::layout::PaneType::WorkflowGraph => {
                vec![
                    ('?', "Help"),
                    ('\x1b', "Escape"),
                    ('h', "Previous pane"),
                    ('l', "Next pane"),
                    ('\t', "Switch pane"),
                ]
            }
            crate::layout::PaneType::AgentView => {
                vec![
                    ('?', "Help"),
                    ('\x1b', "Escape"),
                    ('h', "Previous pane"),
                    ('l', "Next pane"),
                    ('\t', "Switch pane"),
                ]
            }
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

    /// Get the current input mode.
    pub fn input_mode(&self) -> InputMode {
        self.input_mode
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
        self.input_mode = InputMode::Normal;
        self.command_buffer.clear();
        self.search_buffer.clear();
        self.last_search_pattern = None;
        self.unfiltered_tasks.clear();
        self.pending_g = false;
        self.visual_anchor = None;

        // IPC client is not restored (must be re-established)
        // Reset to None to force reconnection
        self.ipc = None;
        self.last_ipc_connect_attempt_ms = None;

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
