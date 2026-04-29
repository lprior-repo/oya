//! Execution engine type definitions.
//!
//! This module contains all the type definitions for the execution engine:
//! - `ExecutionOutcome`
//! - `NodeOutput`
//! - `SharedContext`
//! - `ExecutionMetadata`
//! - `ExecutionConfig`
//! - `NodeExecutionConfig`
//! - `ExecutionContext`
//! - `ExecutionPlan`
//! - `ExecutionResult`

pub mod config;
pub mod context;
pub mod context_state;
pub mod output;
pub mod plan;

pub use config::{ExecutionConfig, NodeExecutionConfig};
pub use context::{ExecutionMetadata, SharedContext};
pub use context_state::ExecutionContext;
pub use output::{ExecutionOutcome, ExecutionResult, NodeOutput};
pub use plan::ExecutionPlan;
