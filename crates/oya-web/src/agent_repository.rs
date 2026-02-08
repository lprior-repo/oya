//! Agent repository layer for agent data access.
//!
//! This module provides a repository pattern implementation for accessing agent data.
//! It abstracts the data source and provides a clean interface for the web layer.
//!
//! # Architecture
//!
//! - `AgentRepository`: Trait defining repository operations
//! - `InMemoryAgentRepository`: In-memory implementation for development
//! - Future: Database-backed implementations can be added
//!
//! # Example
//!
//! ```ignore
//! use oya_web::agent_repository::{AgentRepository, InMemoryAgentRepository};
//!
//! let repo = InMemoryAgentRepository::new();
//!
//! // Register an agent
//! let agent = repo.register_agent("agent-001".to_string()).await?;
//!
//! // Get all agents
//! let agents = repo.get_all_agents().await?;
//! ```

#![deny(clippy::unwrap_used)]
#![deny(clippy::panic)]
#![warn(clippy::pedantic)]
#![warn(clippy::nursery)]
#![forbid(unsafe_code)]

use chrono::Utc;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use thiserror::Error;
use tokio::sync::RwLock;

/// Agent state representing the lifecycle phase.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum RepositoryAgentState {
    /// Agent is idle and available for work.
    Idle,
    /// Agent is actively processing a bead.
    Working,
    /// Agent health checks are failing.
    Unhealthy,
    /// Agent is shutting down gracefully.
    ShuttingDown,
    /// Agent has terminated.
    Terminated,
}

impl From<RepositoryAgentState> for String {
    fn from(state: RepositoryAgentState) -> Self {
        match state {
            RepositoryAgentState::Idle => "idle".to_string(),
            RepositoryAgentState::Working => "working".to_string(),
            RepositoryAgentState::Unhealthy => "unhealthy".to_string(),
            RepositoryAgentState::ShuttingDown => "shutting_down".to_string(),
            RepositoryAgentState::Terminated => "terminated".to_string(),
        }
    }
}

impl std::fmt::Display for RepositoryAgentState {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let s: String = (*self).into();
        write!(f, "{}", s)
    }
}

/// Agent data stored in the repository.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RepositoryAgent {
    /// Unique agent identifier.
    pub id: String,
    /// Current agent state.
    pub state: RepositoryAgentState,
    /// Optional bead currently assigned to the agent.
    pub current_bead: Option<String>,
    /// Agent uptime in seconds.
    pub uptime_secs: u64,
    /// Total number of beads completed.
    pub beads_completed: u64,
    /// Total number of operations executed.
    pub operations_executed: u64,
    /// Current health score (0.0 - 1.0).
    pub health_score: f64,
    /// ISO 8601 timestamp of last heartbeat.
    pub last_heartbeat: String,
    /// Agent registration timestamp.
    pub registered_at: String,
}

impl RepositoryAgent {
    /// Creates a new repository agent entry.
    #[must_use]
    pub fn new(agent_id: String) -> Self {
        let now = Utc::now().to_rfc3339();
        Self {
            id: agent_id,
            state: RepositoryAgentState::Idle,
            current_bead: None,
            uptime_secs: 0,
            beads_completed: 0,
            operations_executed: 0,
            health_score: 1.0,
            last_heartbeat: now.clone(),
            registered_at: now,
        }
    }

    /// Updates the agent's heartbeat timestamp.
    pub fn record_heartbeat(&mut self) -> Result<(), AgentRepositoryError> {
        let now = Utc::now().to_rfc3339();
        self.last_heartbeat = now;
        self.health_score = 1.0;
        Ok(())
    }

    /// Assigns a bead to the agent.
    pub fn assign_bead(&mut self, bead_id: String) -> Result<(), AgentRepositoryError> {
        if !matches!(self.state, RepositoryAgentState::Idle) {
            return Err(AgentRepositoryError::AgentNotAvailable {
                agent_id: self.id.clone(),
                state: self.state,
            });
        }

        self.current_bead = Some(bead_id);
        self.state = RepositoryAgentState::Working;
        Ok(())
    }

    /// Completes the current bead assignment.
    pub fn complete_bead(&mut self) -> Result<(), AgentRepositoryError> {
        if self.current_bead.is_none() {
            return Err(AgentRepositoryError::NoActiveBead {
                agent_id: self.id.clone(),
            });
        }

        self.current_bead = None;
        self.beads_completed += 1;
        self.state = RepositoryAgentState::Idle;
        Ok(())
    }
}

/// Repository errors.
#[derive(Debug, Error, Clone, PartialEq)]
pub enum AgentRepositoryError {
    /// Agent not found.
    #[error("Agent not found: {agent_id}")]
    AgentNotFound { agent_id: String },

    /// Agent already exists.
    #[error("Agent already exists: {agent_id}")]
    AgentAlreadyExists { agent_id: String },

    /// Agent not available for work.
    #[error("Agent not available: {agent_id} (state: {state:?})")]
    AgentNotAvailable {
        agent_id: String,
        state: RepositoryAgentState,
    },

    /// No active bead to complete.
    #[error("No active bead for agent: {agent_id}")]
    NoActiveBead { agent_id: String },

    /// Repository capacity exceeded.
    #[error("Repository capacity exceeded: max {max_agents} agents")]
    CapacityExceeded { max_agents: usize },

    /// Invalid agent ID.
    #[error("Invalid agent ID: {reason}")]
    InvalidAgentId { reason: String },
}

/// Repository trait for agent data access.
///
/// This trait defines the interface for agent data operations.
/// Implementations can use different backends (in-memory, database, etc.).
#[async_trait::async_trait]
pub trait AgentRepository: Send + Sync {
    /// Registers a new agent in the repository.
    ///
    /// # Errors
    ///
    /// Returns an error if the agent already exists or capacity is exceeded.
    async fn register_agent(
        &self,
        agent_id: String,
    ) -> Result<RepositoryAgent, AgentRepositoryError>;

    /// Gets an agent by ID.
    ///
    /// # Errors
    ///
    /// Returns an error if the agent is not found.
    async fn get_agent(&self, agent_id: &str) -> Result<RepositoryAgent, AgentRepositoryError>;

    /// Gets all agents in the repository.
    ///
    /// # Errors
    ///
    /// Returns an error if the repository is inaccessible.
    async fn get_all_agents(&self) -> Result<Vec<RepositoryAgent>, AgentRepositoryError>;

    /// Updates an agent's heartbeat.
    ///
    /// # Errors
    ///
    /// Returns an error if the agent is not found.
    async fn update_heartbeat(
        &self,
        agent_id: &str,
    ) -> Result<RepositoryAgent, AgentRepositoryError>;

    /// Assigns a bead to an agent.
    ///
    /// # Errors
    ///
    /// Returns an error if the agent is not found or not available.
    async fn assign_bead(
        &self,
        agent_id: &str,
        bead_id: String,
    ) -> Result<RepositoryAgent, AgentRepositoryError>;

    /// Completes the current bead assignment for an agent.
    ///
    /// # Errors
    ///
    /// Returns an error if the agent is not found or has no active bead.
    async fn complete_bead(&self, agent_id: &str) -> Result<RepositoryAgent, AgentRepositoryError>;

    /// Gets repository statistics.
    ///
    /// # Errors
    ///
    /// Returns an error if statistics cannot be computed.
    async fn get_stats(&self) -> Result<AgentRepositoryStats, AgentRepositoryError>;
}

/// Repository statistics.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentRepositoryStats {
    /// Total number of agents.
    pub total_agents: usize,
    /// Number of idle agents.
    pub idle_agents: usize,
    /// Number of working agents.
    pub working_agents: usize,
    /// Number of unhealthy agents.
    pub unhealthy_agents: usize,
    /// Total beads completed across all agents.
    pub total_beads_completed: u64,
    /// Total operations executed across all agents.
    pub total_operations_executed: u64,
    /// Average health score across all agents.
    pub average_health_score: f64,
}

/// In-memory implementation of the agent repository.
///
/// This implementation stores agents in a `HashMap` protected by a `RwLock`.
/// It is suitable for development and testing but not for production use.
#[derive(Debug, Clone)]
pub struct InMemoryAgentRepository {
    /// Agent storage.
    agents: Arc<RwLock<HashMap<String, RepositoryAgent>>>,
    /// Maximum number of agents allowed.
    max_agents: usize,
}

impl InMemoryAgentRepository {
    /// Creates a new in-memory repository.
    #[must_use]
    pub fn new() -> Self {
        Self {
            agents: Arc::new(RwLock::new(HashMap::new())),
            max_agents: 100,
        }
    }

    /// Creates a new in-memory repository with a custom capacity.
    #[must_use]
    pub fn with_capacity(max_agents: usize) -> Self {
        Self {
            agents: Arc::new(RwLock::new(HashMap::new())),
            max_agents,
        }
    }

    /// Validates an agent ID.
    fn validate_agent_id(&self, agent_id: &str) -> Result<(), AgentRepositoryError> {
        if agent_id.is_empty() {
            return Err(AgentRepositoryError::InvalidAgentId {
                reason: "Agent ID cannot be empty".to_string(),
            });
        }

        if agent_id.len() > 64 {
            return Err(AgentRepositoryError::InvalidAgentId {
                reason: "Agent ID too long (max 64 characters)".to_string(),
            });
        }

        Ok(())
    }
}

impl Default for InMemoryAgentRepository {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait::async_trait]
impl AgentRepository for InMemoryAgentRepository {
    async fn register_agent(
        &self,
        agent_id: String,
    ) -> Result<RepositoryAgent, AgentRepositoryError> {
        self.validate_agent_id(&agent_id)?;

        let mut agents = self.agents.write().await;

        if agents.len() >= self.max_agents {
            return Err(AgentRepositoryError::CapacityExceeded {
                max_agents: self.max_agents,
            });
        }

        if agents.contains_key(&agent_id) {
            return Err(AgentRepositoryError::AgentAlreadyExists { agent_id });
        }

        let agent = RepositoryAgent::new(agent_id.clone());
        agents.insert(agent_id.clone(), agent.clone());

        Ok(agent)
    }

    async fn get_agent(&self, agent_id: &str) -> Result<RepositoryAgent, AgentRepositoryError> {
        let agents = self.agents.read().await;
        agents
            .get(agent_id)
            .cloned()
            .ok_or_else(|| AgentRepositoryError::AgentNotFound {
                agent_id: agent_id.to_string(),
            })
    }

    async fn get_all_agents(&self) -> Result<Vec<RepositoryAgent>, AgentRepositoryError> {
        let agents = self.agents.read().await;
        let agent_list = agents.values().cloned().collect::<Vec<_>>();
        Ok(agent_list)
    }

    async fn update_heartbeat(
        &self,
        agent_id: &str,
    ) -> Result<RepositoryAgent, AgentRepositoryError> {
        let mut agents = self.agents.write().await;
        let agent =
            agents
                .get_mut(agent_id)
                .ok_or_else(|| AgentRepositoryError::AgentNotFound {
                    agent_id: agent_id.to_string(),
                })?;

        agent.record_heartbeat()?;
        Ok(agent.clone())
    }

    async fn assign_bead(
        &self,
        agent_id: &str,
        bead_id: String,
    ) -> Result<RepositoryAgent, AgentRepositoryError> {
        let mut agents = self.agents.write().await;
        let agent =
            agents
                .get_mut(agent_id)
                .ok_or_else(|| AgentRepositoryError::AgentNotFound {
                    agent_id: agent_id.to_string(),
                })?;

        agent.assign_bead(bead_id)?;
        Ok(agent.clone())
    }

    async fn complete_bead(&self, agent_id: &str) -> Result<RepositoryAgent, AgentRepositoryError> {
        let mut agents = self.agents.write().await;
        let agent =
            agents
                .get_mut(agent_id)
                .ok_or_else(|| AgentRepositoryError::AgentNotFound {
                    agent_id: agent_id.to_string(),
                })?;

        agent.complete_bead()?;
        Ok(agent.clone())
    }

    async fn get_stats(&self) -> Result<AgentRepositoryStats, AgentRepositoryError> {
        let agents = self.agents.read().await;

        let total_agents = agents.len();
        let mut idle_agents = 0;
        let mut working_agents = 0;
        let mut unhealthy_agents = 0;
        let mut total_beads_completed = 0;
        let mut total_operations_executed = 0;
        let mut total_health_score = 0.0;

        for agent in agents.values() {
            match agent.state {
                RepositoryAgentState::Idle => idle_agents += 1,
                RepositoryAgentState::Working => working_agents += 1,
                RepositoryAgentState::Unhealthy => unhealthy_agents += 1,
                RepositoryAgentState::ShuttingDown | RepositoryAgentState::Terminated => {}
            }

            total_beads_completed += agent.beads_completed;
            total_operations_executed += agent.operations_executed;
            total_health_score += agent.health_score;
        }

        let average_health_score = if total_agents > 0 {
            total_health_score / total_agents as f64
        } else {
            0.0
        };

        Ok(AgentRepositoryStats {
            total_agents,
            idle_agents,
            working_agents,
            unhealthy_agents,
            total_beads_completed,
            total_operations_executed,
            average_health_score,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Creates a test repository.
    fn create_test_repo() -> InMemoryAgentRepository {
        InMemoryAgentRepository::with_capacity(10)
    }

    #[tokio::test]
    async fn test_register_agent() {
        let repo = create_test_repo();
        let result = repo.register_agent("agent-001".to_string()).await;

        assert!(matches!(result, Ok(ref agent) if agent.id == "agent-001"));
        assert!(matches!(result, Ok(ref agent) if agent.state == RepositoryAgentState::Idle));
    }

    #[tokio::test]
    async fn test_register_duplicate_agent() {
        let repo = create_test_repo();

        let _ = repo.register_agent("agent-001".to_string()).await;
        let result = repo.register_agent("agent-001".to_string()).await;

        assert!(matches!(
            result,
            Err(AgentRepositoryError::AgentAlreadyExists { agent_id: _ })
        ));
    }

    #[tokio::test]
    async fn test_register_agent_empty_id() {
        let repo = create_test_repo();
        let result = repo.register_agent(String::new()).await;

        assert!(matches!(
            result,
            Err(AgentRepositoryError::InvalidAgentId { .. })
        ));
    }

    #[tokio::test]
    async fn test_register_agent_too_long_id() {
        let repo = create_test_repo();
        let long_id = "a".repeat(65);
        let result = repo.register_agent(long_id).await;

        assert!(matches!(
            result,
            Err(AgentRepositoryError::InvalidAgentId { .. })
        ));
    }

    #[tokio::test]
    async fn test_register_agent_capacity_exceeded() {
        let repo = InMemoryAgentRepository::with_capacity(2);

        let _ = repo.register_agent("agent-001".to_string()).await;
        let _ = repo.register_agent("agent-002".to_string()).await;
        let result = repo.register_agent("agent-003".to_string()).await;

        assert!(matches!(
            result,
            Err(AgentRepositoryError::CapacityExceeded { .. })
        ));
    }

    #[tokio::test]
    async fn test_get_agent() {
        let repo = create_test_repo();

        let _ = repo.register_agent("agent-001".to_string()).await;
        let result = repo.get_agent("agent-001").await;

        assert!(matches!(result, Ok(ref agent) if agent.id == "agent-001"));
    }

    #[tokio::test]
    async fn test_get_agent_not_found() {
        let repo = create_test_repo();
        let result = repo.get_agent("nonexistent").await;

        assert!(matches!(
            result,
            Err(AgentRepositoryError::AgentNotFound { .. })
        ));
    }

    #[tokio::test]
    async fn test_get_all_agents() {
        let repo = create_test_repo();

        let _ = repo.register_agent("agent-001".to_string()).await;
        let _ = repo.register_agent("agent-002".to_string()).await;

        let result = repo.get_all_agents().await;

        assert!(matches!(result, Ok(ref agents) if agents.len() == 2));
    }

    #[tokio::test]
    async fn test_update_heartbeat() {
        let repo = create_test_repo();

        let _ = repo.register_agent("agent-001".to_string()).await;
        let result = repo.update_heartbeat("agent-001").await;

        assert!(matches!(result, Ok(ref agent) if agent.health_score == 1.0));
    }

    #[tokio::test]
    async fn test_assign_bead() {
        let repo = create_test_repo();

        let _ = repo.register_agent("agent-001".to_string()).await;
        let result = repo.assign_bead("agent-001", "bead-123".to_string()).await;

        assert!(
            matches!(result, Ok(ref agent) if agent.current_bead == Some("bead-123".to_string()))
        );
        assert!(matches!(result, Ok(ref agent) if agent.state == RepositoryAgentState::Working));
    }

    #[tokio::test]
    async fn test_assign_bead_not_found() {
        let repo = create_test_repo();
        let result = repo.assign_bead("agent-001", "bead-123".to_string()).await;

        assert!(matches!(
            result,
            Err(AgentRepositoryError::AgentNotFound { .. })
        ));
    }

    #[tokio::test]
    async fn test_assign_bead_not_available() {
        let repo = create_test_repo();

        let _ = repo.register_agent("agent-001".to_string()).await;
        let _ = repo.assign_bead("agent-001", "bead-123".to_string()).await;
        let result = repo.assign_bead("agent-001", "bead-456".to_string()).await;

        assert!(matches!(
            result,
            Err(AgentRepositoryError::AgentNotAvailable { .. })
        ));
    }

    #[tokio::test]
    async fn test_complete_bead() {
        let repo = create_test_repo();

        let _ = repo.register_agent("agent-001".to_string()).await;
        let _ = repo.assign_bead("agent-001", "bead-123".to_string()).await;
        let result = repo.complete_bead("agent-001").await;

        assert!(matches!(result, Ok(ref agent) if agent.current_bead.is_none()));
        assert!(matches!(result, Ok(ref agent) if agent.state == RepositoryAgentState::Idle));
        assert!(matches!(result, Ok(ref agent) if agent.beads_completed == 1));
    }

    #[tokio::test]
    async fn test_complete_bead_no_active() {
        let repo = create_test_repo();

        let _ = repo.register_agent("agent-001".to_string()).await;
        let result = repo.complete_bead("agent-001").await;

        assert!(matches!(
            result,
            Err(AgentRepositoryError::NoActiveBead { .. })
        ));
    }

    #[tokio::test]
    async fn test_get_stats() {
        let repo = create_test_repo();

        let _ = repo.register_agent("agent-001".to_string()).await;
        let _ = repo.register_agent("agent-002".to_string()).await;

        let result = repo.get_stats().await;

        assert!(matches!(result, Ok(ref stats) if stats.total_agents == 2));
        assert!(matches!(result, Ok(ref stats) if stats.idle_agents == 2));
    }

    #[tokio::test]
    async fn test_repository_agent_serialization() {
        let agent = RepositoryAgent::new("agent-001".to_string());
        let json = serde_json::to_string(&agent);
        assert!(json.is_ok());
    }

    #[tokio::test]
    async fn test_repository_stats_serialization() {
        let stats = AgentRepositoryStats {
            total_agents: 10,
            idle_agents: 5,
            working_agents: 3,
            unhealthy_agents: 2,
            total_beads_completed: 100,
            total_operations_executed: 1000,
            average_health_score: 0.95,
        };
        let json = serde_json::to_string(&stats);
        assert!(json.is_ok());
    }

    #[tokio::test]
    async fn test_agent_state_to_string() {
        let state = RepositoryAgentState::Idle;
        let s: String = state.into();
        assert_eq!(s, "idle");

        let state = RepositoryAgentState::Working;
        let s: String = state.into();
        assert_eq!(s, "working");
    }
}
