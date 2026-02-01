#![deny(clippy::unwrap_used)]
#![deny(clippy::expect_used)]
#![deny(clippy::panic)]

//! # oya-factory
//!
//! CI/CD pipeline and task management for OYA.
//!
//! This crate provides:
//! - Task creation and management with type-state builders
//! - CI/CD pipeline stages with composable execution
//! - Audit trail for all operations
//! - Repository detection and language inference
//! - Worktree management (JJ/git)
//! - Process execution with timeout support
//! - Retry logic with exponential backoff
//! - Validated types (NonEmpty, Bounded, etc.)
//!
//! # Design Principles
//!
//! - **Railway-Oriented Programming**: All errors are explicit Result types
//! - **No panics**: `unwrap()`, `expect()`, and `panic!()` are forbidden
//! - **Type-state builders**: Required fields enforced at compile time
//! - **Functional composition**: Pipelines are built from composable stages

pub mod audit;
pub mod builder;
pub mod domain;
pub mod error;
pub mod persistence;
pub mod pipeline;
pub mod process;
pub mod repo;
pub mod retry;
pub mod stages;
pub mod types;
pub mod worktree;

// Re-export commonly used items
pub use error::{Error, Result};
