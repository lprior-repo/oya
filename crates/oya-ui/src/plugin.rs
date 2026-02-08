// Plugin module - Zellij plugin implementation for OYA UI
//
// This module implements the Zellij plugin protocol, handling:
// - Plugin initialization and sizing
// - Event processing (keyboard input, resize, etc.)
// - IPC communication with oya-orchestrator
// - Message routing between UI and host

use crate::layout::Layout;
use crate::render::Renderer;
use oya_ipc::{GuestMessage, HostMessage, IpcTransport};
use serde::{Deserialize, Serialize};
use std::io::{stdin, stdout};
use thiserror::Error;

/// Plugin errors
#[derive(Debug, Error)]
pub enum PluginError {
    #[error("IPC transport error: {0}")]
    IpcError(String),

    #[error("Render error: {0}")]
    RenderError(String),

    #[error("Invalid plugin state: {0}")]
    InvalidState(String),

    #[error("Layout calculation failed: {0}")]
    LayoutError(String),
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

    /// Message received from host
    HostMessage {
        /// Host message
        message: HostMessage,
    },
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
/// - IPC communication with oya-orchestrator
/// - Event processing and state management
pub struct OyaPlugin {
    /// IPC transport for communicating with host
    transport: IpcTransport<std::io::Stdin, std::io::Stdout>,
    /// Terminal layout
    layout: Layout,
    /// Terminal size
    size: Size,
    /// Renderer for drawing UI
    renderer: Renderer,
    /// Current bead data (from host)
    beads: Vec<oya_ipc::BeadSummary>,
    /// Current bead detail (if selected)
    selected_bead: Option<oya_ipc::BeadDetail>,
    /// Currently selected pane
    focused_pane: crate::layout::PaneType,
    /// Plugin state
    state: PluginState,
}

/// Plugin state machine
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PluginState {
    /// Plugin starting
    Starting,
    /// Running normally
    Running,
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
    /// Returns an error if IPC transport creation fails
    pub fn new() -> PluginResult<Self> {
        let transport = IpcTransport::new(stdin(), stdout());

        // Default terminal size (will be updated on first event)
        let size = Size { rows: 24, cols: 80 };

        // Calculate initial layout
        let layout = Layout::calculate_for_terminal(size.rows, size.cols)
            .map_err(|e| PluginError::LayoutError(e.to_string()))?;

        let renderer = Renderer::new();

        Ok(Self {
            transport,
            layout,
            size,
            renderer,
            beads: Vec::new(),
            selected_bead: None,
            focused_pane: crate::layout::PaneType::BeadList,
            state: PluginState::Starting,
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

        // Request initial bead list from host
        self.send_message(GuestMessage::GetBeadList)
            .map_err(|e| PluginError::IpcError(e.to_string()))?;

        self.state = PluginState::Running;

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
            PluginEvent::HostMessage { message } => {
                self.handle_host_message(message)?;
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
        match key {
            // Quit
            'q' | 'Q' => {
                self.state = PluginState::ShuttingDown;
            }
            // Navigate between panes with Tab
            '\t' => {
                self.cycle_focus();
            }
            // Arrow keys for navigation (simplified)
            'j' | 'J' => {
                self.move_selection(-1)?;
            }
            'k' | 'K' => {
                self.move_selection(1)?;
            }
            // Enter to select bead
            '\n' | '\r' => {
                self.select_bead()?;
            }
            _ => {
                // Other keys ignored for now
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

    /// Move selection in current pane
    ///
    /// # Errors
    ///
    /// Returns an error if movement fails
    fn move_selection(&mut self, _direction: i32) -> PluginResult<()> {
        // TODO: Implement selection movement
        Ok(())
    }

    /// Select current bead and show details
    ///
    /// # Errors
    ///
    /// Returns an error if selection fails
    fn select_bead(&mut self) -> PluginResult<()> {
        // TODO: Implement bead selection and detail request
        Ok(())
    }

    /// Handle message from host
    ///
    /// # Errors
    ///
    /// Returns an error if message handling fails
    fn handle_host_message(&mut self, message: HostMessage) -> PluginResult<()> {
        match message {
            HostMessage::BeadList { beads } => {
                self.beads = beads;
            }
            HostMessage::BeadDetail { bead } => {
                self.selected_bead = Some(bead);
            }
            HostMessage::Error { message: msg } => {
                eprintln!("Host error: {}", msg);
                self.state = PluginState::Error;
            }
            _ => {
                // Other messages ignored for now
            }
        }
        Ok(())
    }

    /// Send message to host
    ///
    /// # Errors
    ///
    /// Returns an error if sending fails
    fn send_message(&mut self, message: GuestMessage) -> Result<(), Box<dyn std::error::Error>> {
        self.transport.send(&message)
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
            &self.beads,
            self.selected_bead.as_ref(),
            self.focused_pane,
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
}

impl Default for OyaPlugin {
    fn default() -> Self {
        Self::new().expect("Failed to create plugin")
    }
}

#[cfg(test)]
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
}
