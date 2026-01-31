//! Chunking module - re-exports from chunking_adapter
//!
//! This module provides backwards compatibility by re-exporting types
//! from the chunking_adapter module, which wraps the contextual-chunker crate.
//!
//! All chunking logic now lives in the contextual-chunker workspace crate,
//! with doc_transformer-specific extensions in chunking_adapter.

// Re-export types for backwards compatibility
// Note: These are used by tests, benchmarks, and lib code, not the binary
#[allow(unused_imports)]
pub use crate::chunking_adapter::{chunk_all, Chunk, ChunksResult};

// Re-export ChunkLevel from the underlying crate
#[allow(unused_imports)]
pub use contextual_chunker::ChunkLevel;
