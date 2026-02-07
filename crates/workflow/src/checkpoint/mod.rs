//! Checkpoint management for workflow state.
//!
//! This module provides compression and serialization for checkpointing workflow state,
//! as well as automatic checkpoint timer functionality.

pub mod auto;
pub mod compression;

pub use auto::{
    start_auto_checkpoint, AutoCheckpointConfig, AutoCheckpointTimer, StateProvider,
    DEFAULT_AUTO_CHECKPOINT_INTERVAL,
};
pub use compression::{compress, decompress, compression_ratio, space_savings, CompressionError, CompressionLevel};
