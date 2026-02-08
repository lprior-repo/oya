#![forbid(unsafe_code)]

//! OYA Core - Shared domain types and utilities.
//!
//! This crate provides foundational types for the entire OYA system,
//! including error handling, functional utilities, and common domain models.

pub mod error;
pub mod execution;
pub mod slug;
pub mod stage;
pub mod task;
pub mod visualization;
pub mod workflow;

// Re-export commonly used types
pub use error::{OyaError, OyaResult};
pub use execution::{ExecutionEngine, WorkflowResult, WorkflowState};
pub use slug::Slug;
pub use stage::Stage;
pub use task::Task;
pub use visualization::WorkflowVisualization;
pub use workflow::Workflow;
