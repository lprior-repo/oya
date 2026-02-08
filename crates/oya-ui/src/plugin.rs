// Plugin module - Core plugin trait and event handling

use crate::layout::Layout;
use crate::render::Renderer;
use serde::{Deserialize, Serialize};
use std::io::{self, Write};
use thiserror::Error;

/// Terminal size
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct Size {
    pub rows: u16,
    pub cols: u16,
}

/// Plugin information provided by Zellij at startup
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PluginInfo {
    pub size: Size,
    pub plugin_id: String,
    pub zellij_version: String,
}

/// Events that can be received from Zellij
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum PluginEvent {
    Resize(Size),
    Key(KeyEvent),
    Tick,
}

/// Keyboard events
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KeyEvent {
    pub key: Key,
    pub ctrl: bool,
}

/// Supported keys
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Key {
    Char(char),
    Up,
    Down,
}

/// Errors that can occur in plugin operations
#[derive(Debug, Error)]
pub enum PluginError {
    #[error("IO error: {0}")]
    Io(#[from] io::Error),
    #[error("Render error: {0}")]
    Render(String),
}

/// Result type for plugin operations
pub type PluginResult<T> = Result<T, PluginError>;

/// Core plugin trait
pub trait ZellijPlugin {
    fn new(info: PluginInfo) -> PluginResult<Self> where Self: Sized;
    fn render(&mut self, rows: usize, cols: usize) -> PluginResult<String>;
    fn update(&mut self, event: PluginEvent) -> PluginResult<()>;
    fn layout(&self) -> &Layout;
}

/// Main OYA UI plugin implementation
pub struct OyaPlugin {
    info: PluginInfo,
    layout: Layout,
    renderer: Renderer,
    size: Size,
}

impl ZellijPlugin for OyaPlugin {
    fn new(info: PluginInfo) -> PluginResult<Self> {
        let layout = Layout::new_3_pane();
        let renderer = Renderer::new();
        let size = info.size;

        Ok(Self {
            info,
            layout,
            renderer,
            size,
        })
    }

    fn render(&mut self, rows: usize, cols: usize) -> PluginResult<String> {
        self.size.rows = rows as u16;
        self.size.cols = cols as u16;
        self.renderer
            .render_layout(&self.layout, rows, cols)
            .map_err(|e| PluginError::Render(e.to_string()))
    }

    fn update(&mut self, event: PluginEvent) -> PluginResult<()> {
        match event {
            PluginEvent::Resize(size) => {
                self.size = size;
            }
            PluginEvent::Key(_) => {
                // Keyboard handling not implemented yet
            }
            PluginEvent::Tick => {
                // Timer ticks not implemented yet
            }
        }
        Ok(())
    }

    fn layout(&self) -> &Layout {
        &self.layout
    }
}

impl OyaPlugin {
    /// Write output to stdout
    pub fn write_output(&self, output: &str) -> PluginResult<()> {
        let stdout = io::stdout();
        let mut handle = stdout.lock();
        writeln!(handle, "{}", output)?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used)]
    #![allow(clippy::expect_used)]

    use super::*;

    #[test]
    fn test_plugin_creation() {
        let info = PluginInfo {
            size: Size {
                rows: 24,
                cols: 80,
            },
            plugin_id: "test".to_string(),
            zellij_version: "0.40.0".to_string(),
        };

        let plugin = OyaPlugin::new(info).expect("Failed to create plugin");
        assert_eq!(plugin.size.rows, 24);
        assert_eq!(plugin.size.cols, 80);
    }

    #[test]
    fn test_render_output() {
        let info = PluginInfo {
            size: Size { rows: 24, cols: 80 },
            plugin_id: "test".to_string(),
            zellij_version: "0.40.0".to_string(),
        };

        let mut plugin = OyaPlugin::new(info).expect("Failed to create plugin");
        let output = plugin.render(24, 80).expect("Failed to render");
        assert!(!output.is_empty());
    }

    #[test]
    fn test_resize_event() {
        let info = PluginInfo {
            size: Size { rows: 24, cols: 80 },
            plugin_id: "test".to_string(),
            zellij_version: "0.40.0".to_string(),
        };

        let mut plugin = OyaPlugin::new(info).expect("Failed to create plugin");
        let event = PluginEvent::Resize(Size { rows: 48, cols: 160 });

        plugin.update(event).expect("Failed to handle resize");
        assert_eq!(plugin.size.rows, 48);
        assert_eq!(plugin.size.cols, 160);
    }
}
