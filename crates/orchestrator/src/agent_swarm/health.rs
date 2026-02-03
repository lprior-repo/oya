//! Health monitoring for agent swarm.

use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;

use tokio::sync::RwLock;
use tokio::time::interval;

use super::error::{AgentSwarmError, AgentSwarmResult};
use super::handle::{AgentHandle, AgentState};

/// Configuration for health monitoring.
#[derive(Debug, Clone)]
pub struct HealthConfig {
    /// Interval between health checks.
    pub check_interval: Duration,
    /// Timeout for considering an agent unhealthy.
    pub heartbeat_timeout: Duration,
    /// Number of consecutive failures before marking unhealthy.
    pub max_failures: u32,
}

impl Default for HealthConfig {
    fn default() -> Self {
        Self {
            check_interval: Duration::from_secs(10),
            heartbeat_timeout: Duration::from_secs(30),
            max_failures: 3,
        }
    }
}

impl HealthConfig {
    /// Create a new health config with custom values.
    #[must_use]
    pub const fn new(
        check_interval: Duration,
        heartbeat_timeout: Duration,
        max_failures: u32,
    ) -> Self {
        Self {
            check_interval,
            heartbeat_timeout,
            max_failures,
        }
    }

    /// Create a config for testing with shorter intervals.
    #[must_use]
    pub fn for_testing() -> Self {
        Self {
            check_interval: Duration::from_millis(100),
            heartbeat_timeout: Duration::from_millis(500),
            max_failures: 2,
        }
    }
}

/// Result of a health check.
#[derive(Debug, Clone)]
pub struct HealthCheckResult {
    /// Agent ID that was checked.
    pub agent_id: String,
    /// Whether the agent is healthy.
    pub is_healthy: bool,
    /// Current agent state.
    pub state: AgentState,
    /// Time since last heartbeat.
    pub time_since_heartbeat: Duration,
    /// Number of consecutive failures.
    pub failure_count: u32,
}

/// Health monitor for tracking agent health.
#[derive(Debug)]
pub struct HealthMonitor {
    /// Health configuration.
    config: HealthConfig,
    /// Agents being monitored (shared with pool).
    agents: Arc<RwLock<HashMap<String, AgentHandle>>>,
    /// Whether monitoring is active.
    active: Arc<RwLock<bool>>,
}

impl HealthMonitor {
    /// Create a new health monitor.
    #[must_use]
    pub fn new(config: HealthConfig, agents: Arc<RwLock<HashMap<String, AgentHandle>>>) -> Self {
        Self {
            config,
            agents,
            active: Arc::new(RwLock::new(false)),
        }
    }

    /// Check health of a single agent.
    ///
    /// Returns the health check result.
    pub async fn check_agent(&self, agent_id: &str) -> AgentSwarmResult<HealthCheckResult> {
        let mut agents = self.agents.write().await;

        let agent = agents
            .get_mut(agent_id)
            .ok_or_else(|| AgentSwarmError::agent_not_found(agent_id))?;

        let time_since_heartbeat = agent.time_since_heartbeat();
        let is_timeout = agent.is_heartbeat_timeout(self.config.heartbeat_timeout);

        let was_healthy = agent.state() != AgentState::Unhealthy;
        let became_unhealthy = if is_timeout {
            agent.record_health_failure()
        } else {
            false
        };

        // Get current state after potential update
        let state = agent.state();
        let is_healthy = state != AgentState::Unhealthy && state != AgentState::Terminated;

        // Log state change
        if was_healthy && became_unhealthy {
            tracing::warn!(
                agent_id = %agent_id,
                time_since_heartbeat_ms = %time_since_heartbeat.as_millis(),
                "Agent became unhealthy due to heartbeat timeout"
            );
        }

        Ok(HealthCheckResult {
            agent_id: agent_id.to_string(),
            is_healthy,
            state,
            time_since_heartbeat,
            failure_count: 0, // Would need to expose from AgentHandle
        })
    }

    /// Check health of all agents.
    ///
    /// Returns results for all agents.
    pub async fn check_all(&self) -> Vec<HealthCheckResult> {
        let agent_ids: Vec<String> = {
            let agents = self.agents.read().await;
            agents.keys().cloned().collect()
        };

        let mut results = Vec::with_capacity(agent_ids.len());

        for agent_id in agent_ids {
            if let Ok(result) = self.check_agent(&agent_id).await {
                results.push(result);
            }
        }

        results
    }

    /// Get all unhealthy agents.
    pub async fn get_unhealthy_agents(&self) -> Vec<String> {
        let agents = self.agents.read().await;

        agents
            .iter()
            .filter(|(_, agent)| agent.state() == AgentState::Unhealthy)
            .map(|(id, _)| id.clone())
            .collect()
    }

    /// Get all healthy and available agents.
    pub async fn get_available_agents(&self) -> Vec<String> {
        let agents = self.agents.read().await;

        agents
            .iter()
            .filter(|(_, agent)| agent.is_available())
            .map(|(id, _)| id.clone())
            .collect()
    }

    /// Start background health checking.
    ///
    /// Returns a handle that can be used to stop monitoring.
    pub fn start_background_check(&self) -> tokio::task::JoinHandle<()> {
        let agents = Arc::clone(&self.agents);
        let active = Arc::clone(&self.active);
        let config = self.config.clone();

        tokio::spawn(async move {
            {
                let mut is_active = active.write().await;
                *is_active = true;
            }

            let mut check_interval = interval(config.check_interval);

            loop {
                check_interval.tick().await;

                // Check if we should stop
                {
                    let is_active = active.read().await;
                    if !*is_active {
                        break;
                    }
                }

                // Get agent IDs to check
                let agent_ids: Vec<String> = {
                    let agents_guard = agents.read().await;
                    agents_guard
                        .iter()
                        .filter(|(_, agent)| agent.state().is_active())
                        .map(|(id, _)| id.clone())
                        .collect()
                };

                // Check each agent
                for agent_id in agent_ids {
                    let mut agents_guard = agents.write().await;

                    if let Some(agent) = agents_guard.get_mut(&agent_id) {
                        if agent.is_heartbeat_timeout(config.heartbeat_timeout) {
                            let became_unhealthy = agent.record_health_failure();
                            if became_unhealthy {
                                tracing::warn!(
                                    agent_id = %agent_id,
                                    "Agent marked unhealthy by background monitor"
                                );
                            }
                        }
                    }
                }
            }

            tracing::info!("Background health monitor stopped");
        })
    }

    /// Stop background health checking.
    pub async fn stop(&self) {
        let mut is_active = self.active.write().await;
        *is_active = false;
    }

    /// Check if monitoring is active.
    pub async fn is_active(&self) -> bool {
        let is_active = self.active.read().await;
        *is_active
    }

    /// Get the health configuration.
    #[must_use]
    pub fn config(&self) -> &HealthConfig {
        &self.config
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn create_test_agents() -> Arc<RwLock<HashMap<String, AgentHandle>>> {
        let mut agents = HashMap::new();
        agents.insert("agent-1".to_string(), AgentHandle::new("agent-1"));
        agents.insert("agent-2".to_string(), AgentHandle::new("agent-2"));
        Arc::new(RwLock::new(agents))
    }

    #[tokio::test]
    async fn test_health_monitor_new() {
        let agents = create_test_agents();
        let config = HealthConfig::for_testing();
        let monitor = HealthMonitor::new(config, agents);

        assert!(!monitor.is_active().await);
    }

    #[tokio::test]
    async fn test_check_agent_healthy() {
        let agents = create_test_agents();
        let config = HealthConfig::for_testing();
        let monitor = HealthMonitor::new(config, agents);

        let result = monitor.check_agent("agent-1").await;
        assert!(result.is_ok());

        if let Ok(r) = result {
            assert!(r.is_healthy);
            assert_eq!(r.state, AgentState::Idle);
        }
    }

    #[tokio::test]
    async fn test_check_agent_not_found() {
        let agents = create_test_agents();
        let config = HealthConfig::for_testing();
        let monitor = HealthMonitor::new(config, agents);

        let result = monitor.check_agent("nonexistent").await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_check_all() {
        let agents = create_test_agents();
        let config = HealthConfig::for_testing();
        let monitor = HealthMonitor::new(config, agents);

        let results = monitor.check_all().await;
        assert_eq!(results.len(), 2);
        assert!(results.iter().all(|r| r.is_healthy));
    }

    #[tokio::test]
    async fn test_get_available_agents() {
        let agents = create_test_agents();

        // Make one agent working
        {
            let mut agents_guard = agents.write().await;
            if let Some(agent) = agents_guard.get_mut("agent-1") {
                agent.assign_bead("bead-1");
            }
        }

        let config = HealthConfig::for_testing();
        let monitor = HealthMonitor::new(config, agents);

        let available = monitor.get_available_agents().await;
        assert_eq!(available.len(), 1);
        assert!(available.contains(&"agent-2".to_string()));
    }

    #[tokio::test]
    async fn test_get_unhealthy_agents() {
        let agents = create_test_agents();

        // Make one agent unhealthy
        {
            let mut agents_guard = agents.write().await;
            if let Some(agent) = agents_guard.get_mut("agent-1") {
                // Force unhealthy by recording max failures
                agent.record_health_failure();
                agent.record_health_failure();
                agent.record_health_failure();
            }
        }

        let config = HealthConfig::for_testing();
        let monitor = HealthMonitor::new(config, agents);

        let unhealthy = monitor.get_unhealthy_agents().await;
        assert_eq!(unhealthy.len(), 1);
        assert!(unhealthy.contains(&"agent-1".to_string()));
    }

    #[tokio::test]
    async fn test_background_monitor_lifecycle() {
        let agents = create_test_agents();
        let config = HealthConfig::for_testing();
        let monitor = HealthMonitor::new(config, agents);

        let handle = monitor.start_background_check();

        // Give it time to start
        tokio::time::sleep(Duration::from_millis(50)).await;
        assert!(monitor.is_active().await);

        // Stop it
        monitor.stop().await;

        // Give it time to stop
        tokio::time::sleep(Duration::from_millis(150)).await;
        assert!(!monitor.is_active().await);

        // Clean up
        handle.abort();
    }
}
