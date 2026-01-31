#![deny(clippy::unwrap_used)]
#![deny(clippy::expect_used)]
#![deny(clippy::panic)]

//! # oya-factory
//!
//! CI/CD pipeline and task management for OYA.
//!
//! This crate provides:
//! - Task creation and management
//! - CI/CD pipeline stages
//! - Audit trail
//! - Repository detection
//! - Worktree management
//! - Process execution

pub mod audit;
pub mod domain;
pub mod error;
pub mod persistence;
pub mod process;
pub mod repo;
pub mod stages;
pub mod worktree;

// Re-export commonly used items
pub use error::{Error, Result};
