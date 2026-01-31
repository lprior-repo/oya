//! # juggernaut-workflow
//!
//! Intra-bead workflow engine with checkpoints, rewind, and journal replay.

#![forbid(clippy::unwrap_used)]
#![deny(clippy::expect_used)]
#![forbid(clippy::panic)]

pub use juggernaut_core::{JuggernautError, Result};

// TODO: Implement Phase, Workflow, Journal, WorkflowEngine
