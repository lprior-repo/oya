//! Idempotent execution primitives for workflow phases.
//!
//! This module provides the foundation for deterministic, idempotent
//! workflow execution through UUID v5-based key generation.
//!
//! # Architecture
//!
//! Idempotent execution relies on three components:
//!
//! 1. **Input Hashing** ([`hash`]) - SHA-256 hashing of arbitrary input data
//! 2. **Namespace Generation** - UUID v5 namespace from bead_id (future)
//! 3. **Key Generation** - UUID v5 from namespace + input hash (future)
//!
//! # Example
//!
//! ```ignore
//! use oya_workflow::idempotent::{hash_input, hash_serializable};
//! use serde::Serialize;
//!
//! // Hash raw bytes
//! let data = b"workflow input";
//! let hash = hash_input(data);
//!
//! // Hash structured data
//! #[derive(Serialize)]
//! struct TaskInput {
//!     bead_id: String,
//!     phase: String,
//! }
//!
//! let input = TaskInput {
//!     bead_id: "bead-123".to_string(),
//!     phase: "build".to_string(),
//! };
//!
//! let hash = hash_serializable(&input).expect("serialization failed");
//! ```

pub mod hash;

pub use hash::{hash_input, hash_serializable};
