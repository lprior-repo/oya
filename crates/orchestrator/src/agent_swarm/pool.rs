//! Agent pool for managing a collection of agents.

use std::collections::HashMap;
use std::sync::Arc;

use itertools::Itertools;
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
    pub const fn for_testing() -> Self {
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
    /// Number of beads currently assigned to agents.
    pub beads_assigned: usize,
    /// Number of beads completed.
    pub beads_completed: usize,
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
    /// Number of beads completed.
    beads_completed: Arc<RwLock<usize>>,
    /// Assignment history for sticky mode (bead_id -> worker_id).
    assignment_history: Arc<RwLock<HashMap<String, String>>>,
}

impl AgentPool {
    /// Create a new agent pool.
    #[must_use]
    pub fn new(config: PoolConfig) -> Self {
        let agents = Arc::new(RwLock::new(HashMap::new()));
        let health_monitor = HealthMonitor::new(config.health_config.clone(), Arc::clone(&agents));
        let beads_completed = Arc::new(RwLock::new(0));
        let assignment_history = Arc::new(RwLock::new(HashMap::new()));

        Self {
            agents,
            health_monitor,
            config,
            beads_completed,
            assignment_history,
        }
    }

    /// Register a new agent in the pool.
    ///
    /// # Errors
    ///
    /// Returns an error if agent is already registered or pool is at capacity.
    #[tracing::instrument(skip(self, agent))]
    #[allow(clippy::unreachable)]
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
    /// Returns an error if agent is not found.
    #[tracing::instrument(skip(self))]
    #[allow(clippy::unreachable)]
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
            .collect_vec()
    }

    /// Get all agents in the pool.
    pub async fn all_agents(&self) -> Vec<AgentHandle> {
        let agents = self.agents.read().await;

        agents.values().cloned().collect_vec()
    }

    /// Get agents with a specific capability.
    pub async fn get_agents_with_capability(&self, capability: &str) -> Vec<AgentHandle> {
        let agents = self.agents.read().await;

        agents
            .values()
            .filter(|a| a.has_capability(capability) && a.is_available())
            .cloned()
            .collect_vec()
    }

    /// Assign a bead to an available agent using sticky mode.
    ///
    /// Sticky mode behavior:
    /// - Prefer previous worker if they are idle
    /// - Fall back to first idle worker if previous is busy/unavailable
    /// - Always returns a worker if any idle worker exists (never blocks)
    ///
    /// Returns the agent ID that was assigned.
    ///
    /// # Errors
    ///
    /// Returns an error if no agents are available.
    #[tracing::instrument(skip(self), fields(bead_id))]
    pub async fn assign_bead(&self, bead_id: &str) -> AgentSwarmResult<String> {
        // Get previous worker assignment from history
        let previous_worker = {
            let history = self.assignment_history.read().await;
            history.get(bead_id).cloned()
        };

        // Get available agent IDs
        let available_agent_ids: Vec<String> = {
            let agents = self.agents.read().await;
            agents
                .values()
                .filter(|a| a.is_available())
                .map(|a| a.id().to_string())
                .collect_vec()
        };

        if available_agent_ids.is_empty() {
            return Err(AgentSwarmError::NoAgentsAvailable);
        }

        // Select worker based on sticky logic
        let selected_worker_id = match previous_worker {
            // Sticky hit: previous worker exists and is idle
            Some(prev_id) if available_agent_ids.iter().any(|id| *id == prev_id) => {
                tracing::debug!(
                    bead_id = %bead_id,
                    worker_id = %prev_id,
                    "Sticky hit: assigning to previous worker"
                );
                prev_id
            }

            // Fallback: previous worker not idle or not in available list
            _ => {
                // Select first available worker (deterministic ordering)
                let fallback_worker = available_agent_ids
                    .iter()
                    .min_by(|a, b| a.cmp(b))
                    .cloned()
                    .ok_or_else(|| AgentSwarmError::assignment_failed(
                        bead_id,
                        "failed to select agent from available list",
                    ))?;

                match previous_worker.as_ref() {
                    Some(prev_id) => {
                        tracing::debug!(
                            bead_id = %bead_id,
                            previous_worker_id = %prev_id,
                            fallback_worker_id = %fallback_worker,
                            "Sticky fallback: previous worker busy/unavailable"
                        );
                    }
                    None => {
                        tracing::debug!(
                            bead_id = %bead_id,
                            fallback_worker_id = %fallback_worker,
                            "No previous assignment: using first available worker"
                        );
                    }
                }

                fallback_worker
            }
        };

        // Assign bead to selected worker
        {
            let mut agents = self.agents.write().await;
            let agent = agents
                .get_mut(&selected_worker_id)
                .ok_or_else(|| AgentSwarmError::agent_not_found(&selected_worker_id))?;

            if !agent.assign_bead(bead_id) {
                return Err(AgentSwarmError::assignment_failed(
                    bead_id,
                    "agent state changed during assignment",
                ));
            }

            // Record assignment in history
            {
                let mut history = self.assignment_history.write().await;
                history.insert(bead_id.to_string(), selected_worker_id.clone());
            }

            tracing::debug!(
                agent_id = %selected_worker_id,
                bead_id = %bead_id,
                "Bead assigned to agent"
            );

            Ok(selected_worker_id)
        }
    }

    /// Assign a bead to a specific agent.
    ///
    /// # Errors
    ///
    /// Returns an error if agent is not found or unavailable.
    #[tracing::instrument(skip(self), fields(bead_id, agent_id))]
    #[allow(clippy::unreachable)]
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
    /// Returns an error if agent is not found.
    #[tracing::instrument(skip(self))]
    #[allow(clippy::unreachable)]
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

            let mut count = self.beads_completed.write().await;
            *count = count.saturating_add(1);
        }

        Ok(())
    }

    /// Release a bead from an agent without completing it.
    ///
    /// # Errors
    ///
    /// Returns an error if agent is not found.
    #[tracing::instrument(skip(self))]
    #[allow(clippy::unreachable)]
    pub async fn release_bead(&self, agent_id: &str) -> AgentSwarmResult<()> {
        let mut agents = self.agents.write().await;

        let agent = agents
            .get_mut(agent_id)
            .ok_or_else(|| AgentSwarmError::agent_not_found(agent_id))?;

        agent.release_bead();

        tracing::debug!(
            agent_id = %agent_id,
            "Bead released from agent"
        );

        Ok(())
    }

    /// Get pool statistics.
    #[tracing::instrument(skip(self))]
    #[allow(clippy::unreachable)]
    pub async fn stats(&self) -> PoolStats {
        let agents = self.agents.read().await;

        let (idle, working, unhealthy, shutting_down, terminated) = agents
            .values()
            .fold(
                (0, 0, 0, 0, 0),
                |(idle, working, unhealthy, shutting_down, terminated), agent| {
                    match agent.state() {
                        AgentState::Idle => (idle + 1, working, unhealthy, shutting_down, terminated),
                        AgentState::Working => (idle, working + 1, unhealthy, shutting_down, terminated),
                        AgentState::Unhealthy => (idle, working, unhealthy + 1, shutting_down, terminated),
                        AgentState::ShuttingDown => (idle, working, unhealthy, shutting_down + 1, terminated),
                        AgentState::Terminated => (idle, working, unhealthy, shutting_down, terminated + 1),
                    }
                },
            );

        let beads_assigned = working;
        let beads_completed = *self.beads_completed.read().await;

        PoolStats {
            total: agents.len(),
            idle,
            working,
            unhealthy,
            shutting_down,
            terminated,
            beads_assigned,
            beads_completed,
        }
    }

    /// Record a heartbeat for an agent.
    ///
    /// # Errors
    ///
    /// Returns an error if agent is not found.
    #[tracing::instrument(skip(self), fields(agent_id))]
    #[allow(clippy::unreachable)]
    pub async fn record_heartbeat(&self, agent_id: &str) -> AgentSwarmResult<()> {
        let mut agents = self.agents.write().await;

        let agent = agents
            .get_mut(agent_id)
            .ok_or_else(|| AgentSwarmError::agent_not_found(agent_id))?;

        agent.record_heartbeat();

        tracing::debug!(
            agent_id = %agent_id,
            "Heartbeat recorded"
        );

        Ok(())
    }

    /// Shutdown an agent (mark as shutting down).
    ///
    /// # Errors
    ///
    /// Returns an error if agent is not found.
    #[tracing::instrument(skip(self), fields(agent_id))]
    #[allow(clippy::unreachable)]
    pub async fn shutdown_agent(&self, agent_id: &str) -> AgentSwarmResult<()> {
        let mut agents = self.agents.write().await;

        let agent = agents
            .get_mut(agent_id)
            .ok_or_else(|| AgentSwarmError::agent_not_found(agent_id))?;

        agent.shutdown();

        tracing::info!(
            agent_id = %agent_id,
            "Agent shutting down"
        );

        Ok(())
    }

    /// Terminate an agent (mark as terminated).
    ///
    /// # Errors
    ///
    /// Returns an error if agent is not found.
    #[tracing::instrument(skip(self), fields(agent_id))]
    #[allow(clippy::unreachable)]
    pub async fn terminate_agent(&self, agent_id: &str) -> AgentSwarmResult<()> {
        let mut agents = self.agents.write().await;

        let agent = agents
            .get_mut(agent_id)
            .ok_or_else(|| AgentSwarmError::agent_not_found(agent_id))?;

        agent.terminate();

        tracing::info!(
            agent_id = %agent_id,
            "Agent terminated"
        );

        Ok(())
    }

}

#[cfg(test)]
mod tests {
    use super::*;

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
    fn test_pool_stats_default() {
        let stats = PoolStats::default();
        assert_eq!(stats.total, 0);
        assert_eq!(stats.idle, 0);
    }
}
