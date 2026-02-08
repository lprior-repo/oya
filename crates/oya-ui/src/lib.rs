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

// Zero-panic policy - enforced by workspace
// These lints are inherited from workspace.lints.clippy in Cargo.toml

pub mod components;
pub mod layout;
pub mod plugin;
pub mod render;

// Re-exports for convenience
pub use layout::{Layout, Pane, PaneType};
pub use plugin::{OyaPlugin, PluginEvent, PluginInfo, Size};
pub use render::Renderer;
