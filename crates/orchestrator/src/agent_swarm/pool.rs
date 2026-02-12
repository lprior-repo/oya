//! Agent pool for managing a collection of agents.

use std::collections::HashMap;
use std::sync::Arc;

use tokio::sync::RwLock;

use super::error::{AgentSwarmError, AgentSwarmResult};
use super::handle::AgentHandle;
use super::health::{HealthConfig, HealthMonitor};

/// Agent pool for managing a collection of agents.
#[derive(Clone)]
pub struct AgentPool {
    agents: Arc<RwLock<HashMap<String, AgentHandle>>>,
    health_monitor: HealthMonitor,
}

impl AgentPool {
    /// Create a new agent pool.
    #[must_use]
    pub fn new(config: PoolConfig) -> Self {
        let agents = Arc::new(RwLock::new(HashMap::new()));
        let health_monitor = HealthMonitor::new(config.health_config, agents.clone());
        Self {
            agents,
            health_monitor,
        }
    }

    /// Register an agent in the pool.
    pub async fn register_agent(&self, agent: AgentHandle) -> AgentSwarmResult<()> {
        self.agents.write().await.insert(agent.id().to_string(), agent);
        Ok(())
    }

    /// Unregister an agent from the pool.
    pub async fn unregister_agent(&self, agent_id: &str) -> AgentSwarmResult<()> {
        self.agents.write().await.remove(agent_id);
        Ok(())
    }

    /// Get a specific agent by ID.
    pub async fn get_agent(&self, agent_id: &str) -> Option<AgentHandle> {
        self.agents.read().await.get(agent_id).cloned()
    }

    /// Get all agents in the pool.
    pub async fn all_agents(&self) -> Vec<AgentHandle> {
        self.agents.read().await.values().cloned().collect()
    }

    /// Assign a bead to an available agent.
    pub async fn assign_bead(&self, bead_id: &str) -> AgentSwarmResult<String> {
        let mut agents = self.agents.write().await;

        // Sort agent IDs for deterministic assignment (round-robin based on ID order)
        let mut agent_ids: Vec<String> = agents.keys().cloned().collect();
        agent_ids.sort();

        for agent_id in &agent_ids {
            if let Some(agent) = agents.get_mut(agent_id) {
                if agent.state().is_idle() && agent.assign_bead(bead_id) {
                    return Ok(agent_id.clone());
                }
            }
        }
        Err(AgentSwarmError::NoAgentsAvailable)
    }

    /// Assign a bead to a specific agent.
    pub async fn assign_bead_to_agent(
        &self,
        bead_id: &str,
        agent_id: &str,
    ) -> AgentSwarmResult<()> {
        let mut agents = self.agents.write().await;
        let res = if let Some(agent) = agents.get_mut(agent_id) {
            if agent.assign_bead(bead_id) {
                Ok(())
            } else {
                Err(AgentSwarmError::unavailable(agent_id, "cannot accept work"))
            }
        } else {
            Err(AgentSwarmError::unavailable(agent_id, "cannot accept work"))
        };
        drop(agents);
        res
    }

    /// Complete a bead assigned to an agent.
    pub async fn complete_bead(&self, agent_id: &str) -> AgentSwarmResult<()> {
        if let Some(agent) = self.agents.write().await.get_mut(agent_id) {
            agent.complete_bead();
        }
        Ok(())
    }

    /// Record a heartbeat for a specific agent.
    pub async fn record_heartbeat(&self, agent_id: &str) -> AgentSwarmResult<()> {
        let mut agents = self.agents.write().await;
        if let Some(agent) = agents.get_mut(agent_id) {
            agent.record_heartbeat();
            Ok(())
        } else {
            Err(AgentSwarmError::agent_not_found(agent_id))
        }
    }

    /// Shutdown a specific agent.
    pub async fn shutdown_agent(&self, agent_id: &str) -> AgentSwarmResult<()> {
        if let Some(agent) = self.agents.write().await.get_mut(agent_id) {
            agent.shutdown();
        }
        Ok(())
    }

    /// Get pool statistics.
    pub async fn stats(&self) -> PoolStats {
        let agents = self.agents.read().await;
        let mut stats = PoolStats::default();

        for agent in agents.values() {
            stats.total = stats.total.wrapping_add(1);
            match agent.state() {
                super::handle::AgentState::Idle => stats.idle = stats.idle.wrapping_add(1),
                super::handle::AgentState::Working => stats.working = stats.working.wrapping_add(1),
                super::handle::AgentState::Unhealthy => {
                    stats.unhealthy = stats.unhealthy.wrapping_add(1);
                }
                super::handle::AgentState::ShuttingDown => {
                    stats.shutting_down = stats.shutting_down.wrapping_add(1);
                }
                super::handle::AgentState::Terminated => {
                    stats.terminated = stats.terminated.wrapping_add(1);
                }
            }
        }

        drop(agents);
        stats
    }

    /// Get the health monitor for this pool.
    #[must_use]
    pub const fn health_monitor(&self) -> &HealthMonitor {
        &self.health_monitor
    }
}

impl Default for AgentPool {
    fn default() -> Self {
        Self::new(PoolConfig::default())
    }
}

/// Configuration for the agent pool.
#[derive(Debug, Clone)]
pub struct PoolConfig {
    /// Maximum number of agents in the pool
    pub max_agents: usize,
    /// Health monitoring configuration
    pub health_config: HealthConfig,
}

impl Default for PoolConfig {
    fn default() -> Self {
        Self {
            max_agents: 100,
            health_config: HealthConfig::for_testing(),
        }
    }
}

impl PoolConfig {
    /// Create a new pool configuration.
    #[must_use]
    pub const fn new(max_agents: usize, health_config: HealthConfig) -> Self {
        Self {
            max_agents,
            health_config,
        }
    }

    /// Create a configuration suitable for testing.
    #[must_use]
    pub const fn for_testing() -> Self {
        Self {
            max_agents: 10,
            health_config: HealthConfig::for_testing(),
        }
    }
}

/// Statistics about the agent pool.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct PoolStats {
    /// Total number of agents
    pub total: usize,
    /// Number of idle agents
    pub idle: usize,
    /// Number of working agents
    pub working: usize,
    /// Number of unhealthy agents
    pub unhealthy: usize,
    /// Number of agents shutting down
    pub shutting_down: usize,
    /// Number of terminated agents
    pub terminated: usize,
    /// Number of beads assigned
    pub beads_assigned: usize,
    /// Number of beads completed
    pub beads_completed: usize,
}

#[test]
fn test_pool_config_default() {
    let config = PoolConfig::default();
    assert_eq!(config.max_agents, 100);
}

#[test]
fn test_pool_config_new() {
    let config = PoolConfig::new(50, HealthConfig::default());
    assert_eq!(config.max_agents, 50);
}

#[test]
fn test_pool_config_for_testing() {
    let config = PoolConfig::for_testing();
    assert_eq!(config.max_agents, 10);
}

#[test]
fn test_pool_stats_default() {}
