// oya-ui - Zellij WASM plugin for OYA SDLC visualization
//
// This crate provides a terminal-based UI for visualizing OYA workflows,
// including bead status, pipeline progress, and workflow graphs.
//
// Architecture:
// - Plugin: Main plugin entry point implementing Zellij protocol
// - Layout: 3-pane layout system (BeadList, BeadDetail, WorkflowGraph)
// - IPC: Communication with oya-orchestrator for real-time data
// - Components: UI widgets for rendering different views

#![warn(clippy::unwrap_used)]
#![warn(clippy::expect_used)]
#![warn(clippy::panic)]
#![warn(clippy::unimplemented)]
#![warn(clippy::unreachable)]
#![warn(clippy::indexing_slicing)]
#![warn(clippy::arithmetic_side_effects)]
#![warn(clippy::unwrap_in_result)]

pub mod components;
pub mod layout;
pub mod plugin;
pub mod render;

// Re-exports for convenience
pub use plugin::{OyaPlugin, PluginInfo, PluginEvent, Size};
pub use layout::{Layout, Pane, PaneType};
pub use render::Renderer;
