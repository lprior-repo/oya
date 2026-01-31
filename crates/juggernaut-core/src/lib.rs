//! # juggernaut-core
//!
//! Core types, errors, and Railway-Oriented primitives for Juggernaut SDLC Factory.
//!
//! ## Zero Panic Guarantee
//!
//! This crate enforces:
//! - `#![forbid(clippy::unwrap_used)]`
//! - `#![deny(clippy::expect_used)]`
//! - `#![forbid(clippy::panic)]`

#![forbid(clippy::unwrap_used)]
#![deny(clippy::expect_used)]
#![forbid(clippy::panic)]

pub mod error;
pub mod id;
pub mod result_ext;
pub mod state;

pub use error::{JuggernautError, Result};
pub use id::{BeadId, EventId, PhaseId, WorkflowId};
pub use result_ext::ResultExt;
pub use state::BeadState;
