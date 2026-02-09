#![forbid(unsafe_code)]

//! OYA Core - Shared domain types and utilities.
//!
//! This crate provides foundational types for the entire OYA system,
//! including error handling, functional utilities, and common domain models.

pub mod error;
pub mod execution;
pub mod result;
pub mod slug;
pub mod visualization;
pub mod workflow;

// Re-export commonly used types
pub use error::Error;
pub use execution::{ExecutionEngine, WorkflowResult, WorkflowState};
pub use result::Result;
pub use slug::Slug;
pub use visualization::WorkflowVisualization;
pub use workflow::Workflow;

// Type aliases for convenience
pub type OyaError = Error;
pub type OyaResult<T> = Result<T>;
