//! Agent pool for managing a collection of agents.

use std::collections::HashMap;
use std::sync::Arc;

use tokio::sync::RwLock;

use super::error::{AgentSwarmError, AgentSwarmResult};
use super::handle::{AgentHandle, AgentState};
use super::health::{HealthConfig, HealthMonitor};

/// Configuration for the agent pool.
#[derive(Debug, Clone)]
pub struct PoolConfig {
    /// Maximum number of agents in the pool.
    pub max_agents: usize,
    /// Health monitoring configuration.
    pub health_config: HealthConfig,
}

impl Default for PoolConfig {
    fn default() -> Self {
        Self {
            max_agents: 100,
            health_config: HealthConfig::default(),
        }
    }
}

impl PoolConfig {
    /// Create a new pool config.
    #[must_use]
    pub const fn new(max_agents: usize, health_config: HealthConfig) -> Self {
        Self {
            max_agents,
            health_config,
        }
    }

    /// Create a config for testing.
    #[must_use]
    pub fn for_testing() -> Self {
        Self {
            max_agents: 10,
            health_config: HealthConfig::for_testing(),
        }
    }
}

/// Statistics about the agent pool.
#[derive(Debug, Clone, Default)]
pub struct PoolStats {
    /// Total number of agents.
    pub total: usize,
    /// Number of idle agents.
    pub idle: usize,
    /// Number of working agents.
    pub working: usize,
    /// Number of unhealthy agents.
    pub unhealthy: usize,
    /// Number of shutting down agents.
    pub shutting_down: usize,
    /// Number of terminated agents.
    pub terminated: usize,
}

/// Agent pool for managing multiple agents.
#[derive(Debug)]
pub struct AgentPool {
    /// All agents in the pool.
    agents: Arc<RwLock<HashMap<String, AgentHandle>>>,
    /// Health monitor.
    health_monitor: HealthMonitor,
    /// Pool configuration.
    config: PoolConfig,
}

impl AgentPool {
    /// Create a new agent pool.
    #[must_use]
    pub fn new(config: PoolConfig) -> Self {
        let agents = Arc::new(RwLock::new(HashMap::new()));
        let health_monitor = HealthMonitor::new(config.health_config.clone(), Arc::clone(&agents));

        Self {
            agents,
            health_monitor,
            config,
        }
    }

    /// Register a new agent in the pool.
    ///
    /// # Errors
    ///
    /// Returns an error if the agent is already registered or pool is at capacity.
    pub async fn register_agent(&self, agent: AgentHandle) -> AgentSwarmResult<()> {
        let mut agents = self.agents.write().await;

        if agents.len() >= self.config.max_agents {
            return Err(AgentSwarmError::PoolCapacityExceeded {
                current: agents.len(),
                max: self.config.max_agents,
            });
        }

        let agent_id = agent.id().to_string();

        if agents.contains_key(&agent_id) {
            return Err(AgentSwarmError::already_registered(&agent_id));
        }

        tracing::info!(agent_id = %agent_id, "Agent registered");
        agents.insert(agent_id, agent);

        Ok(())
    }

    /// Unregister an agent from the pool.
    ///
    /// # Errors
    ///
    /// Returns an error if the agent is not found.
    pub async fn unregister_agent(&self, agent_id: &str) -> AgentSwarmResult<AgentHandle> {
        let mut agents = self.agents.write().await;

        agents
            .remove(agent_id)
            .ok_or_else(|| AgentSwarmError::agent_not_found(agent_id))
    }

    /// Get a reference to an agent by ID.
    ///
    /// Returns None if agent not found.
    pub async fn get_agent(&self, agent_id: &str) -> Option<AgentHandle> {
        let agents = self.agents.read().await;
        agents.get(agent_id).cloned()
    }

    /// Get all available agents (idle and healthy).
    pub async fn get_available_agents(&self) -> Vec<AgentHandle> {
        let agents = self.agents.read().await;

        agents
            .values()
            .filter(|a| a.is_available())
            .cloned()
            .collect()
    }

    /// Get all agents in the pool.
    pub async fn all_agents(&self) -> Vec<AgentHandle> {
        let agents = self.agents.read().await;

        agents.values().cloned().collect()
    }

    /// Get agents with a specific capability.
    pub async fn get_agents_with_capability(&self, capability: &str) -> Vec<AgentHandle> {
        let agents = self.agents.read().await;

        agents
            .values()
            .filter(|a| a.has_capability(capability) && a.is_available())
            .cloned()
            .collect()
    }

    /// Assign a bead to an available agent.
    ///
    /// Returns the agent ID that was assigned.
    ///
    /// # Errors
    ///
    /// Returns an error if no agents are available.
    pub async fn assign_bead(&self, bead_id: &str) -> AgentSwarmResult<String> {
        let mut agents = self.agents.write().await;

        // Find first available agent
        let agent = agents
            .values_mut()
            .find(|a| a.is_available())
            .ok_or(AgentSwarmError::NoAgentsAvailable)?;

        let agent_id = agent.id().to_string();

        if !agent.assign_bead(bead_id) {
            return Err(AgentSwarmError::assignment_failed(
                bead_id,
                "agent state changed during assignment",
            ));
        }

        tracing::debug!(
            agent_id = %agent_id,
            bead_id = %bead_id,
            "Bead assigned to agent"
        );

        Ok(agent_id)
    }

    /// Assign a bead to a specific agent.
    ///
    /// # Errors
    ///
    /// Returns an error if the agent is not found or unavailable.
    pub async fn assign_bead_to_agent(
        &self,
        bead_id: &str,
        agent_id: &str,
    ) -> AgentSwarmResult<()> {
        let mut agents = self.agents.write().await;

        let agent = agents
            .get_mut(agent_id)
            .ok_or_else(|| AgentSwarmError::agent_not_found(agent_id))?;

        if !agent.is_available() {
            return Err(AgentSwarmError::unavailable(
                agent_id,
                format!("agent is in state: {}", agent.state()),
            ));
        }

        if !agent.assign_bead(bead_id) {
            return Err(AgentSwarmError::assignment_failed(
                bead_id,
                "agent rejected assignment",
            ));
        }

        tracing::debug!(
            agent_id = %agent_id,
            bead_id = %bead_id,
            "Bead assigned to specific agent"
        );

        Ok(())
    }

    /// Mark a bead as completed on an agent.
    ///
    /// # Errors
    ///
    /// Returns an error if the agent is not found.
    pub async fn complete_bead(&self, agent_id: &str) -> AgentSwarmResult<()> {
        let mut agents = self.agents.write().await;

        let agent = agents
            .get_mut(agent_id)
            .ok_or_else(|| AgentSwarmError::agent_not_found(agent_id))?;

        let bead_id = agent.current_bead().map(String::from);
        agent.complete_bead();

        if let Some(bead_id) = bead_id {
            tracing::debug!(
                agent_id = %agent_id,
                bead_id = %bead_id,
                "Bead completed"
            );
        }

        Ok(())
    }

    /// Release a bead from an agent without completing it.
    ///
    /// # Errors
    ///
    /// Returns an error if the agent is not found.
    pub async fn release_bead(&self, agent_id: &str) -> AgentSwarmResult<Option<String>> {
        let mut agents = self.agents.write().await;

        let agent = agents
            .get_mut(agent_id)
            .ok_or_else(|| AgentSwarmError::agent_not_found(agent_id))?;

        let bead_id = agent.current_bead().map(String::from);
        agent.release_bead();

        if let Some(ref bead_id) = bead_id {
            tracing::debug!(
                agent_id = %agent_id,
                bead_id = %bead_id,
                "Bead released"
            );
        }

        Ok(bead_id)
    }

    /// Record a heartbeat for an agent.
    ///
    /// # Errors
    ///
    /// Returns an error if the agent is not found.
    pub async fn record_heartbeat(&self, agent_id: &str) -> AgentSwarmResult<()> {
        let mut agents = self.agents.write().await;

        let agent = agents
            .get_mut(agent_id)
            .ok_or_else(|| AgentSwarmError::agent_not_found(agent_id))?;

        agent.record_heartbeat();
        Ok(())
    }

    /// Shutdown an agent gracefully.
    ///
    /// # Errors
    ///
    /// Returns an error if the agent is not found.
    pub async fn shutdown_agent(&self, agent_id: &str) -> AgentSwarmResult<()> {
        let mut agents = self.agents.write().await;

        let agent = agents
            .get_mut(agent_id)
            .ok_or_else(|| AgentSwarmError::agent_not_found(agent_id))?;

        agent.shutdown();
        tracing::info!(agent_id = %agent_id, "Agent shutdown initiated");

        Ok(())
    }

    /// Terminate an agent.
    ///
    /// # Errors
    ///
    /// Returns an error if the agent is not found.
    pub async fn terminate_agent(&self, agent_id: &str) -> AgentSwarmResult<()> {
        let mut agents = self.agents.write().await;

        let agent = agents
            .get_mut(agent_id)
            .ok_or_else(|| AgentSwarmError::agent_not_found(agent_id))?;

        agent.terminate();
        tracing::info!(agent_id = %agent_id, "Agent terminated");

        Ok(())
    }

    /// Get pool statistics.
    pub async fn stats(&self) -> PoolStats {
        let agents = self.agents.read().await;

        let mut stats = PoolStats {
            total: agents.len(),
            ..Default::default()
        };

        for agent in agents.values() {
            match agent.state() {
                AgentState::Idle => stats.idle += 1,
                AgentState::Working => stats.working += 1,
                AgentState::Unhealthy => stats.unhealthy += 1,
                AgentState::ShuttingDown => stats.shutting_down += 1,
                AgentState::Terminated => stats.terminated += 1,
            }
        }

        stats
    }

    /// Get the number of agents in the pool.
    pub async fn len(&self) -> usize {
        let agents = self.agents.read().await;
        agents.len()
    }

    /// Check if the pool is empty.
    pub async fn is_empty(&self) -> bool {
        let agents = self.agents.read().await;
        agents.is_empty()
    }

    /// Get the health monitor.
    #[must_use]
    pub fn health_monitor(&self) -> &HealthMonitor {
        &self.health_monitor
    }

    /// Start background health monitoring.
    pub fn start_health_monitoring(&self) -> tokio::task::JoinHandle<()> {
        self.health_monitor.start_background_check()
    }

    /// Stop background health monitoring.
    pub async fn stop_health_monitoring(&self) {
        self.health_monitor.stop().await;
    }

    /// Shutdown all agents gracefully.
    pub async fn shutdown_all(&self) {
        let agent_ids: Vec<String> = {
            let agents = self.agents.read().await;
            agents.keys().cloned().collect()
        };

        for agent_id in agent_ids {
            let _ = self.shutdown_agent(&agent_id).await;
        }
    }

    /// Get the pool configuration.
    #[must_use]
    pub fn config(&self) -> &PoolConfig {
        &self.config
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_pool_new() {
        let pool = AgentPool::new(PoolConfig::for_testing());
        assert!(pool.is_empty().await);
        assert_eq!(pool.len().await, 0);
    }

    #[tokio::test]
    async fn test_register_agent() {
        let pool = AgentPool::new(PoolConfig::for_testing());

        let agent = AgentHandle::new("agent-1");
        let result = pool.register_agent(agent).await;
        assert!(result.is_ok());

        assert_eq!(pool.len().await, 1);
    }

    #[tokio::test]
    async fn test_register_duplicate() {
        let pool = AgentPool::new(PoolConfig::for_testing());

        let agent1 = AgentHandle::new("agent-1");
        let _ = pool.register_agent(agent1).await;

        let agent2 = AgentHandle::new("agent-1");
        let result = pool.register_agent(agent2).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_pool_capacity() {
        let mut config = PoolConfig::for_testing();
        config.max_agents = 2;
        let pool = AgentPool::new(config);

        let _ = pool.register_agent(AgentHandle::new("agent-1")).await;
        let _ = pool.register_agent(AgentHandle::new("agent-2")).await;

        let result = pool.register_agent(AgentHandle::new("agent-3")).await;
        assert!(result.is_err());

        if let Err(AgentSwarmError::PoolCapacityExceeded { current, max }) = result {
            assert_eq!(current, 2);
            assert_eq!(max, 2);
        }
    }

    #[tokio::test]
    async fn test_unregister_agent() {
        let pool = AgentPool::new(PoolConfig::for_testing());

        let _ = pool.register_agent(AgentHandle::new("agent-1")).await;
        assert_eq!(pool.len().await, 1);

        let result = pool.unregister_agent("agent-1").await;
        assert!(result.is_ok());
        assert!(pool.is_empty().await);
    }

    #[tokio::test]
    async fn test_get_agent() {
        let pool = AgentPool::new(PoolConfig::for_testing());

        let _ = pool.register_agent(AgentHandle::new("agent-1")).await;

        let agent = pool.get_agent("agent-1").await;
        assert!(agent.is_some());

        let agent = pool.get_agent("nonexistent").await;
        assert!(agent.is_none());
    }

    #[tokio::test]
    async fn test_assign_bead() {
        let pool = AgentPool::new(PoolConfig::for_testing());

        let _ = pool.register_agent(AgentHandle::new("agent-1")).await;

        let result = pool.assign_bead("bead-1").await;
        assert!(result.is_ok());
        assert_eq!(result.ok(), Some("agent-1".to_string()));

        // Agent is now working, no more available
        let result = pool.assign_bead("bead-2").await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_assign_bead_to_specific_agent() {
        let pool = AgentPool::new(PoolConfig::for_testing());

        let _ = pool.register_agent(AgentHandle::new("agent-1")).await;
        let _ = pool.register_agent(AgentHandle::new("agent-2")).await;

        let result = pool.assign_bead_to_agent("bead-1", "agent-2").await;
        assert!(result.is_ok());

        // agent-1 should still be available
        let available = pool.get_available_agents().await;
        assert_eq!(available.len(), 1);
        assert_eq!(available[0].id(), "agent-1");
    }

    #[tokio::test]
    async fn test_complete_bead() {
        let pool = AgentPool::new(PoolConfig::for_testing());

        let _ = pool.register_agent(AgentHandle::new("agent-1")).await;
        let _ = pool.assign_bead("bead-1").await;

        let result = pool.complete_bead("agent-1").await;
        assert!(result.is_ok());

        // Agent should be available again
        let available = pool.get_available_agents().await;
        assert_eq!(available.len(), 1);
    }

    #[tokio::test]
    async fn test_release_bead() {
        let pool = AgentPool::new(PoolConfig::for_testing());

        let _ = pool.register_agent(AgentHandle::new("agent-1")).await;
        let _ = pool.assign_bead("bead-1").await;

        let result = pool.release_bead("agent-1").await;
        assert!(result.is_ok());
        assert_eq!(result.ok().flatten(), Some("bead-1".to_string()));

        // Agent should be available again
        let available = pool.get_available_agents().await;
        assert_eq!(available.len(), 1);
    }

    #[tokio::test]
    async fn test_pool_stats() {
        let pool = AgentPool::new(PoolConfig::for_testing());

        let _ = pool.register_agent(AgentHandle::new("agent-1")).await;
        let _ = pool.register_agent(AgentHandle::new("agent-2")).await;
        let _ = pool.register_agent(AgentHandle::new("agent-3")).await;

        let _ = pool.assign_bead("bead-1").await;

        let stats = pool.stats().await;
        assert_eq!(stats.total, 3);
        assert_eq!(stats.working, 1);
        assert_eq!(stats.idle, 2);
    }

    #[tokio::test]
    async fn test_shutdown_agent() {
        let pool = AgentPool::new(PoolConfig::for_testing());

        let _ = pool.register_agent(AgentHandle::new("agent-1")).await;

        let result = pool.shutdown_agent("agent-1").await;
        assert!(result.is_ok());

        let agent = pool.get_agent("agent-1").await;
        if let Some(a) = agent {
            assert_eq!(a.state(), AgentState::ShuttingDown);
        }
    }

    #[tokio::test]
    async fn test_get_agents_with_capability() {
        let pool = AgentPool::new(PoolConfig::for_testing());

        let agent1 = AgentHandle::new("agent-1").with_capabilities(vec!["rust".to_string()]);
        let agent2 = AgentHandle::new("agent-2").with_capabilities(vec!["python".to_string()]);
        let agent3 = AgentHandle::new("agent-3")
            .with_capabilities(vec!["rust".to_string(), "python".to_string()]);

        let _ = pool.register_agent(agent1).await;
        let _ = pool.register_agent(agent2).await;
        let _ = pool.register_agent(agent3).await;

        let rust_agents = pool.get_agents_with_capability("rust").await;
        assert_eq!(rust_agents.len(), 2);

        let python_agents = pool.get_agents_with_capability("python").await;
        assert_eq!(python_agents.len(), 2);

        let java_agents = pool.get_agents_with_capability("java").await;
        assert!(java_agents.is_empty());
    }
}
