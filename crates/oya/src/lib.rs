//! # Oya CLI Tool
//!
//! This is the main CLI library for the Oya workflow orchestration tool.
//!
//! ## Architecture
//!
//! The Oya CLI is organized into command modules:
//! - `commands::storm`: Orchestration command for executing bead workflows
//!
//! ## Function Contract
//!
//! All commands follow design-by-contract principles with:
//! - Explicit preconditions (validated before execution)
//! - Guaranteed postconditions (deterministic outputs)
//! - Enforced invariants (database immutability, resource cleanup)
//!
//! ## Error Handling
//!
//! All errors use Result<T, Error> with specific exit codes.

#![deny(clippy::unwrap_used)]
#![deny(clippy::expect_used)]
#![deny(clippy::panic)]
#![warn(clippy::pedantic)]
#![warn(clippy::nursery)]
#![forbid(unsafe_code)]

pub mod commands;

pub use oya_core::{Error, Result};
