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
    /// IPC client for orchestrator communication
    ipc: Option<IpcClient>,
    /// Status message shown in the UI
    status_message: Option<String>,
}

/// Task data for rendering
#[derive(Debug, Clone)]
pub struct TaskRow {
    pub slug: String,
    pub status: String,
    pub stage: Option<String>,
    pub priority: String,
    pub language: String,
    pub branch: String,
}

/// Plugin state machine
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
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
            TaskRow {
                slug: "task-3ax5".to_string(),
                status: "in_progress".to_string(),
                stage: Some("implement".to_string()),
                priority: "P1".to_string(),
                language: "Rust".to_string(),
                branch: "task/task-3ax5".to_string(),
            },
            TaskRow {
                slug: "task-1xvj".to_string(),
                status: "created".to_string(),
                stage: None,
                priority: "P1".to_string(),
                language: "Rust".to_string(),
                branch: "task/task-1xvj".to_string(),
            },
            TaskRow {
                slug: "task-1k71".to_string(),
                status: "created".to_string(),
                stage: None,
                priority: "P2".to_string(),
                language: "Rust".to_string(),
                branch: "task/task-1k71".to_string(),
            },
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
    fn get_keybindings_for_pane(&self, pane_type: crate::layout::PaneType) -> Vec<(char, &'static str)> {
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
            crate::layout::PaneType::PipelineView => vec![
                ('\t', "Switch pane"),
                ('g', "Refresh tasks"),
            ],
            crate::layout::PaneType::WorkflowGraph => vec![
                ('\t', "Switch pane"),
            ],
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
}

fn task_summary_to_row(summary: TaskSummary) -> TaskRow {
    TaskRow {
        slug: summary.slug,
        status: summary.status,
        stage: summary.stage,
        priority: summary.priority,
        language: summary.language,
        branch: summary.branch,
    }
}

#[cfg(test)]
#[allow(clippy::expect_used)]
#[allow(clippy::indexing_slicing)]
mod tests {
    #![allow(clippy::unwrap_used)]
    #![allow(clippy::expect_used)]

    use super::*;

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
        let plugin = OyaPlugin::new().unwrap();
        assert!(!plugin.tasks.is_empty());
        assert_eq!(plugin.tasks[0].slug, "task-3ax5");
    }
}
