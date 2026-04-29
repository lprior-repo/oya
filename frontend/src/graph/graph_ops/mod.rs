//! Shared graph operations used across multiple modules.
//!
//! This module centralizes common graph algorithms and lookup patterns
//! to eliminate duplication across `connectivity`, `execution`,
//! `execution_engine`, and `validation_checks`.

#![allow(clippy::implicit_hasher)]

mod mutations;
mod queries;

pub use mutations::*;
pub use queries::*;

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used, clippy::panic, clippy::float_cmp)]
mod tests;
