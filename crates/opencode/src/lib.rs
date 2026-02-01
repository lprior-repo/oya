#![deny(clippy::unwrap_used)]
#![deny(clippy::expect_used)]
#![deny(clippy::panic)]

//! # oya-opencode
//!
//! OpenCode AI bridge for OYA - Execute AI coding tasks via opencode CLI.
//!
//! This crate provides integration with the [OpenCode](https://opencode.ai) CLI
//! for AI-powered code generation, analysis, and modification.
//!
//! ## Features
//!
//! - Execute prompts via the opencode CLI
//! - Stream responses for real-time feedback
//! - Phase-based execution with context management
//! - Configurable models and agent modes
//!
//! ## Example
//!
//! ```ignore
//! use oya_opencode::{OpencodeClient, PhaseContext, AIExecutor};
//!
//! // Create a client
//! let client = OpencodeClient::new()?;
//!
//! // Execute a simple prompt
//! let result = client.execute("Create a hello world function in Rust").await?;
//! println!("Output: {}", result.output);
//!
//! // Or use phase-based execution
//! let executor = AIExecutor::new(Arc::new(client));
//! let ctx = PhaseContext::new("implement", "Create a new feature")
//!     .with_input(PhaseInput::text("Build a CLI parser"))
//!     .with_constraint("Use clap");
//! let output = executor.execute(&ctx).await?;
//! ```

pub mod acp_schemas;
pub mod client;
pub mod config;
pub mod error;
pub mod executor;
pub mod types;

// Re-export commonly used items
pub use acp_schemas::{
    AcpError, AcpMessage, AgentPart, CacheStats, FilePart, FilePartSource, LspPosition, LspRange,
    MessagePart, PartBase, PatchPart, ReasoningPart, SnapshotPart, StepFinishPart, StepStartPart,
    TextPart, TimeRange, ToolPart, ToolState, ToolTimeRange,
};
pub use client::OpencodeClient;
pub use config::{AgentMode, OpencodeConfig};
pub use error::{Error, Result};
pub use executor::{
    AIExecutor, PhaseContext, PhaseHandler, PhaseInput, PhaseOutput, PhaseRegistry,
};
pub use types::{
    ChunkType, CommandExecution, ExecutionResult, ModificationType, ModifiedFile, StreamChunk,
    TokenUsage,
};
