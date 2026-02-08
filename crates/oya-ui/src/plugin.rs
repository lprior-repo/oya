// Plugin module - Zellij plugin implementation for OYA UI
//
// This module implements the Zellij plugin protocol, handling:
// - Plugin initialization and sizing
// - Event processing (keyboard input, resize, etc.)
// - Basic UI rendering
//
// NOTE: IPC integration with oya-orchestrator will be added in a future bead

use crate::layout::Layout;
use crate::render::Renderer;
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
    /// Sample bead data (placeholder)
    sample_beads: Vec<SampleBead>,
    /// Currently selected bead index
    selected_index: usize,
}

/// Sample bead data for placeholder rendering
#[derive(Debug, Clone)]
struct SampleBead {
    id: String,
    title: String,
    state: String,
    priority: u8,
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
    /// Returns an error if layout calculation fails
    pub fn new() -> PluginResult<Self> {
        // Default terminal size (will be updated on first event)
        let size = Size { rows: 24, cols: 80 };

        // Calculate initial layout
        let layout = Layout::calculate_for_terminal(size.rows, size.cols)
            .map_err(|e| PluginError::LayoutError(e.to_string()))?;

        let renderer = Renderer::new();

        // Create sample bead data for placeholder rendering
        let sample_beads = vec![
            SampleBead {
                id: "src-3ax5".to_string(),
                title: "Create Zellij WASM plugin scaffold".to_string(),
                state: "in_progress".to_string(),
                priority: 1,
            },
            SampleBead {
                id: "src-1xvj".to_string(),
                title: "Implement IPC client integration".to_string(),
                state: "open".to_string(),
                priority: 1,
            },
            SampleBead {
                id: "src-1k71".to_string(),
                title: "Add BeadList component with real data".to_string(),
                state: "open".to_string(),
                priority: 2,
            },
        ];

        Ok(Self {
            layout,
            size,
            renderer,
            focused_pane: crate::layout::PaneType::BeadList,
            state: PluginState::Starting,
            sample_beads,
            selected_index: 0,
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

    /// Move selection in bead list
    ///
    /// # Errors
    ///
    /// Returns an error if movement fails
    fn move_selection(&mut self, direction: i32) -> PluginResult<()> {
        let len = self.sample_beads.len();

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
            &self.sample_beads,
            self.selected_index,
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

    #[test]
    fn test_sample_beads() {
        let plugin = OyaPlugin::new().unwrap();
        assert!(!plugin.sample_beads.is_empty());
        assert_eq!(plugin.sample_beads[0].id, "src-3ax5");
    }
}
