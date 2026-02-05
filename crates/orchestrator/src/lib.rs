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
//!
//! # Restate Feature Parity
//!
//! The orchestrator implements Restate-style durable execution:
//!
//! - **Messaging**: Durable message channels with exactly-once delivery
//! - **Virtual Objects**: Stateful K/V entities with isolation
//! - **Timers**: Durable scheduled task execution

#![forbid(unsafe_code)]
#![forbid(clippy::unwrap_used)]
#![forbid(clippy::panic)]
#![deny(clippy::expect_used)]

pub use oya_core::{Error, Result};

/// Actor-based concurrency for the orchestrator.
///
/// Provides ractor actors with supervision for managing workflows.
pub mod actors;

/// Orchestrator module for managing agent swarms.
///
/// Provides agent pool management, health monitoring, and message types
/// for coordinating distributed agents.
pub mod agent_swarm;

/// Workflow DAG module for managing bead dependencies
pub mod dag;

/// Task distribution strategies for assigning beads to agents.
///
/// Provides pluggable strategies including FIFO, priority-based,
/// round-robin, and affinity-based distribution.
pub mod distribution;

/// Durable message passing for service-to-service communication.
///
/// Provides Restate-style durable message channels with exactly-once
/// delivery semantics for reliable inter-workflow communication.
pub mod messaging;

/// Persistence layer for workflows, beads, and checkpoints.
pub mod persistence;

/// Replay and recovery engine for orchestrator state.
pub mod replay;

/// Scheduler actor for managing workflow DAGs and bead scheduling
pub mod scheduler;

/// Supervision tree helpers for tier-1 supervisors.
pub mod supervision;

/// Graceful shutdown handling with signal management and checkpoint coordination
pub mod shutdown;

/// Durable timers for scheduled task execution.
///
/// Provides Restate-style durable timers that survive restarts
/// and ensure scheduled work is executed.
pub mod timers;

/// Virtual Objects for stateful entity management.
///
/// Provides Restate-style Virtual Objects - stateful entities that
/// maintain isolated key-value state and handle messages.
pub mod virtual_objects;
