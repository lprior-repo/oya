//! Checkpoint management for workflow state.
//!
//! This module provides compression and serialization for checkpointing workflow state.

pub mod compression;

pub use compression::{compress, decompress, compression_ratio, space_savings, CompressionError, CompressionLevel};
