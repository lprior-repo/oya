//! Tauri command handlers
//!
//! High-performance command handlers optimized for:
//! - Batched operations to reduce IPC overhead
//! - Cache-first reads with async refresh
//! - Non-blocking file I/O

mod beads;
mod health;
mod streams;

pub use beads::*;
pub use health::*;
pub use streams::*;
