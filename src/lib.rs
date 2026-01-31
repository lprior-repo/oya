//! Factory Core Library
//!
//! Core functionality for the Factory CI/CD pipeline.
//! Implements Railway-Oriented Programming with zero panics.

pub mod audit;
pub mod domain;
pub mod error;
pub mod persistence;
pub mod process;
pub mod repo;
pub mod stages;
pub mod worktree;

// Re-export commonly used types
pub use domain::{Language, Priority, Slug, Stage, Task, TaskStatus};
pub use error::{Error, Result};
