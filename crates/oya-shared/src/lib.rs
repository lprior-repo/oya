//! Shared types for oya-ui and oya-tauri
//!
//! This crate provides high-performance serializable types used across
//! the Tauri frontend (WASM) and backend (native Rust). All types support
//! both standard serde serialization and rkyv zero-copy deserialization.
//!
//! # Performance
//!
//! - `serde`: Used for JSON/bincode serialization (~5-8μs per 1KB)
//! - `rkyv`: Zero-copy deserialization (~0μs, direct memory access)
//!
//! # Usage
//!
//! ```rust
//! use oya_shared::{Bead, BeadStatus, BeadPriority};
//!
//! let bead = Bead::new("bead-1", "Fix login bug")
//!     .with_status(BeadStatus::Running)
//!     .with_priority(BeadPriority::High);
//! ```

mod bead;
mod event;
mod graph;
mod pipeline;

pub use bead::{Bead, BeadFilters, BeadPriority, BeadStatus};
pub use event::{BeadEvent, StreamChunk, StreamEnded};
pub use graph::{
    Edge, EdgeState, EdgeStyle, EdgeType, Graph, Node, NodeId, NodeShape, NodeState, Position,
};
pub use pipeline::{PipelineState, StageEvent, StageInfo, StageStatus};
