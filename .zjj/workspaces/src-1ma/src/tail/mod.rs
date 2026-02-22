//! Tail TUI module - live pipeline monitoring via Ratatui.
//! Domain model with illegal states unrepresentable (Scott Wlaschin style).

mod app;
mod parser;
mod restate;
mod types;
mod ui;

// Public API - exports only what's needed by consumers
pub use app::run_tail;
