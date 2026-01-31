#![deny(clippy::unwrap_used)]
#![deny(clippy::expect_used)]
#![deny(clippy::panic)]

//! # oya-core
//!
//! Core types, errors, and utilities shared across all OYA crates.
//!
//! This crate provides:
//! - Unified error types and Result aliases
//! - Result extension traits for Railway-Oriented Programming

pub mod error;
pub mod result;

// Re-export commonly used items
pub use error::Error;
pub use result::{GenericResultExt, OptionExt, Result, ResultExt};
