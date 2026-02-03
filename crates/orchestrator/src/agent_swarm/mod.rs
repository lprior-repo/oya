//! Agent swarm module for managing distributed agents.
//!
//! This module provides:
//! - `AgentPool`: Manages a collection of agents
//! - `AgentHandle`: Represents a single agent with state
//! - `HealthMonitor`: Monitors agent health via heartbeats
//! - Message types for agent communication
//!
//! # Example
//!
//! ```ignore
//! use orchestrator::agent_swarm::{AgentPool, AgentHandle, PoolConfig};
//!
//! let pool = AgentPool::new(PoolConfig::default());
//!
//! // Register agents
//! pool.register_agent(AgentHandle::new("agent-1")).await?;
//! pool.register_agent(AgentHandle::new("agent-2")).await?;
//!
//! // Assign work
//! let agent_id = pool.assign_bead("bead-123").await?;
//!
//! // Complete work
//! pool.complete_bead(&agent_id).await?;
//! ```

mod error;
mod handle;
mod health;
mod messages;
mod pool;

pub use error::{AgentSwarmError, AgentSwarmResult};
pub use handle::{AgentHandle, AgentState};
pub use health::{HealthCheckResult, HealthConfig, HealthMonitor};
pub use messages::{AgentMessage, AgentResponse};
pub use pool::{AgentPool, PoolConfig, PoolStats};
