//! Checkpoint management for workflow state.
//!
//! This module provides compression and serialization for checkpointing workflow state,
//! as well as automatic checkpoint timer functionality.

pub mod auto;
pub mod compression;
pub mod manager;
pub mod restore;
pub mod serialize;
pub mod storage;

pub use auto::{
    start_auto_checkpoint, AutoCheckpointConfig, AutoCheckpointTimer, StateProvider,
    DEFAULT_AUTO_CHECKPOINT_INTERVAL,
};
pub use compression::{compress, compression_ratio, decompress, space_savings, CompressionLevel};
pub use manager::{CheckpointDecision, CheckpointManager, CheckpointStrategy};
pub use restore::{restore_checkpoint, CheckpointId, RestoreError, RestoreResult};
pub use serialize::{serialize_state, SerializeError, SerializeResult, CHECKPOINT_VERSION};
pub use storage::{
    CheckpointMetadata, CheckpointStorage, CompressionConfig as StorageCompressionConfig,
    StorageStats,
};
