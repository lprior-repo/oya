#![deny(clippy::unwrap_used)]
#![deny(clippy::expect_used)]
#![deny(clippy::panic)]

//! # oya-OYA
//!
//! CI/CD pipeline and task management for OYA.
//!
//! This crate provides:
//! - Task creation and management with type-state builders
//! - CI/CD pipeline stages with composable execution
//! - Audit trail for all operations
//! - Repository detection and language inference
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

pub mod actor;
pub mod ai_integration;
pub mod audit;
pub mod bead_worker;
pub mod builder;
pub mod codegen;
pub mod domain;
pub mod error;
pub mod execution;
pub mod functional;
pub mod persistence;
pub mod pipeline;
pub mod process;
pub mod process_pool;
pub mod quality_gates;
pub mod repo;
pub mod retry;
pub mod stages;
pub mod types;
pub mod worker_registry;
pub mod workspace;

// Re-export commonly used items
pub use ai_integration::{AIStageExecutor, OYAPhaseContextBuilder, StagePhaseMapping};
pub use codegen::{
    BeadSpec, FunctionRequirement, generate_from_bead, parse_bead_spec, simple_function_spec,
    spec_to_prompt, validate_functional_code,
};
pub use error::{Error, Result};
pub use functional::{
    ForbiddenPattern, FunctionalAudit, audit_functional_style, format_violations_report,
    generate_functional_module, has_critical_violations,
};
pub use process_pool::{
    ProcessConfig, ProcessResult, WorkerProcess, run_command, run_command_in_dir, spawn_and_wait,
};
pub use quality_gates::{FunctionalGate, QualityGateResult, enforce_functional_quality};
pub use workspace::{WorkspaceInfo, WorkspaceManager};
