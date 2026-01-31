//! # Orchestrator
//!
//! Agent swarm coordination and task distribution for OYA.

#![forbid(unsafe_code)]
#![forbid(clippy::unwrap_used)]
#![forbid(clippy::panic)]
#![deny(clippy::expect_used)]

pub use core::error::{Error, Result};

/// Orchestrator module for managing agent swarms
pub mod agent_swarm {
    //! Agent swarm coordination
}

/// Task distribution module
pub mod distribution {
    //! Task distribution across agents
}
