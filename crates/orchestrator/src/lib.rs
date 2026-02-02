//! # Orchestrator
//!
//! Agent swarm coordination and task distribution for OYA.
//!
//! This crate provides the core orchestration capabilities for managing
//! workflow DAGs and coordinating bead execution across agent swarms.
//!
//! # Architecture
//!
//! The orchestrator uses ractor actors for message-passing concurrency:
//!
//! - **SchedulerActor**: Manages workflow DAGs and bead scheduling
//! - **Supervision**: Automatic restart on panic with exponential backoff
//! - **EventBus integration**: Subscribes to bead completion events
//! - **Graceful shutdown**: Checkpoint coordination within 30s window

#![forbid(unsafe_code)]
#![forbid(clippy::unwrap_used)]
#![forbid(clippy::panic)]
#![deny(clippy::expect_used)]

pub use oya_core::{Error, Result};

/// Actor-based concurrency for the orchestrator.
///
/// Provides ractor actors with supervision for managing workflows.
pub mod actors;

/// Orchestrator module for managing agent swarms
pub mod agent_swarm {}

/// Task distribution module
pub mod distribution {}

/// Scheduler actor for managing workflow DAGs and bead scheduling
pub mod scheduler;

/// Graceful shutdown handling with signal management and checkpoint coordination
pub mod shutdown;

/// Workflow DAG module for managing bead dependencies
pub mod dag;
