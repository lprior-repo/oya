//! # Merge Queue
//!
//! Parallel task merging and conflict resolution for OYA.

#![forbid(unsafe_code)]
#![forbid(clippy::unwrap_used)]
#![forbid(clippy::panic)]
#![deny(clippy::expect_used)]

pub use core::error::{Error, Result};

/// Queue management module
pub mod queue {
    //! Merge queue implementation
}

/// Conflict resolution module
pub mod conflict {
    //! Conflict detection and resolution
}
