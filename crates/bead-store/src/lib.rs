#![deny(clippy::unwrap_used)]
#![deny(clippy::expect_used)]
#![deny(clippy::panic)]
#![warn(clippy::pedantic)]
#![warn(clippy::nursery)]
#![forbid(unsafe_code)]

//! BeadStore - Persistent storage for bead tracking.
//!
//! This crate provides a functional core/imperative shell architecture for
//! managing bead state with persistence, queries, and atomic operations.
//!
//! # Architecture
//!
//! - **Core** (`BeadStoreCore`): Pure, synchronous, immutable operations
//! - **Shell** (`BeadStore`): Async I/O, persistence, concurrency
//!
//! # Example
//!
//! ```no_run
//! use bead_store::{BeadStore, BeadRecord, BeadStatus, BeadId};
//! use std::path::PathBuf;
//!
//! # #[tokio::main]
//! # async fn main() -> Result<(), Box<dyn std::error::Error>> {
//! let store = BeadStore::new(PathBuf::from(".oya/beads.jsonl")).await?;
//!
//! // Query beads
//! let beads = store.list_beads().await?;
//!
//! // Update bead status
//! let bead_id = BeadId::new("bead-123");
//! if let Some(mut bead) = store.get_bead(&bead_id).await? {
//!     bead.status = BeadStatus::Closed;
//!     store.update_bead(bead).await?;
//! }
//! # Ok(())
//! # }
//! ```

pub mod error;
pub mod store;
pub mod types;

// Re-export common types for convenience
pub use error::StoreError;
pub use store::{BeadStore, BeadStoreCore};
pub use types::{BeadId, BeadRecord, BeadStatus};
