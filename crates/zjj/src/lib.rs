#![deny(clippy::unwrap_used)]
#![deny(clippy::expect_used)]
#![deny(clippy::panic)]

//! # oya-zjj
//!
//! ZJJ (Zellij + Jujutsu) workspace isolation for OYA.
//!
//! This crate provides:
//! - Jujutsu workspace management
//! - Zellij session and tab management
//! - Beads issue tracking integration
//! - Contract validation
//! - Hooks system
//! - File watching

pub mod beads;
pub mod contracts;
pub mod error;
pub mod hints;
pub mod hooks;
pub mod introspection;
pub mod jj;
pub mod watcher;
pub mod zellij;

// Re-export commonly used items
pub use error::{Error, Result};
