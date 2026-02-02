//! # Orchestrator
//!
//! Agent swarm coordination and task distribution for OYA.

#![forbid(unsafe_code)]
#![forbid(clippy::unwrap_used)]
#![forbid(clippy::panic)]
#![deny(clippy::expect_used)]

pub use oya_core::{Error, Result};

/// Orchestrator module for managing agent swarms
pub mod agent_swarm {}

/// Task distribution module
pub mod distribution {}

/// Scheduler actor for managing workflow DAGs and bead scheduling
pub mod scheduler;
