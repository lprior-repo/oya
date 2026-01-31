#![deny(clippy::unwrap_used)]
#![deny(clippy::expect_used)]
#![deny(clippy::panic)]

//! # oya-intent
//!
//! Intent/KIRK system for OYA.
//!
//! This crate provides:
//! - Configuration management
//! - Intent specification
//! - Type-safe domain types
//! - Prelude with common imports

pub mod config;
pub mod error;
pub mod prelude;
pub mod types;

// Re-export commonly used items
pub use error::{IntentError, IntentResult};
