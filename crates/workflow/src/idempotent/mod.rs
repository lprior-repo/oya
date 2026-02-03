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
//! 2. **Namespace Generation** ([`namespace`]) - UUID v5 namespace from bead_id
//! 3. **Key Generation** ([`keys`]) - UUID v5 from namespace + input hash
//! 4. **Type Safety** ([`types`]) - IdempotencyKey wrapper type
//!
//! # Example
//!
//! ```ignore
//! use oya_workflow::idempotent::{
//!     hash_input, hash_serializable, namespace_from_bead, idempotency_key,
//! };
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
//! # fn main() -> Result<(), Box<dyn std::error::Error>> {
//! let hash = hash_serializable(&input)?;
//!
//! // Generate namespace from bead_id
//! let namespace = namespace_from_bead("bead-123");
//!
//! // Generate idempotency key
//! let key = idempotency_key("bead-123", &input)?;
//! # Ok(())
//! # }
//! ```

pub mod hash;
pub mod keys;
pub mod namespace;
pub mod types;

pub use hash::{hash_input, hash_serializable};
pub use keys::{idempotency_key, idempotency_key_from_bytes, idempotency_key_from_json};
pub use namespace::namespace_from_bead;
pub use types::IdempotencyKey;
