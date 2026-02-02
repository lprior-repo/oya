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

/// Scheduler module for dispatching ready beads to queues
pub mod scheduler;

pub use scheduler::{DispatchResult, Dispatcher, QueueStrategy};
