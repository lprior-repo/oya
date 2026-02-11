#![forbid(unsafe_code)]
#![cfg_attr(test, allow(clippy::expect_used))]
#![cfg_attr(test, allow(clippy::unwrap_used))]
#![cfg_attr(test, allow(clippy::panic))]

//! OYA Core - Shared domain types and utilities.
//!
//! This crate provides foundational types for the entire OYA system,
//! including error handling, functional utilities, and common domain models.

pub mod error;
pub mod execution;
pub mod result;
pub mod slug;
pub mod task;
pub mod visualization;
pub mod workflow;

// Re-export commonly used types
pub use error::Error;
pub use execution::{ExecutionEngine, WorkflowResult, WorkflowState};
pub use result::Result;
pub use slug::Slug;
pub use task::{Stage, Task};
pub use visualization::WorkflowVisualization;
pub use workflow::Workflow;

// Type aliases for convenience
pub type OyaError = Error;
pub type OyaResult<T> = Result<T>;
