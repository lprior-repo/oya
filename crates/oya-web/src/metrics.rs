//! Agent metrics calculation logic.
//!
//! This module provides functionality to calculate performance metrics
//! for agents including:
//! - Average uptime
//! - Task completion rates
//! - Health score distribution
//! - Active vs idle agent counts

use crate::agent_repository::AgentSnapshot;
use std::collections::HashMap;

/// Aggregated metrics for all agents
#[derive(Debug, Clone, PartialEq)]
pub struct AgentMetrics {
    pub total_agents: usize,
    pub active_agents: usize,
    pub idle_agents: usize,
    pub unhealthy_agents: usize,
    pub average_uptime_secs: f64,
    pub average_health_score: f64,
    pub status_distribution: HashMap<String, usize>,
    pub capability_counts: HashMap<String, usize>,
}

/// Error type for metrics calculation
#[derive(Debug, thiserror::Error)]
pub enum MetricsError {
    #[error("no agents available for metrics calculation")]
    NoAgents,
    #[error("invalid uptime value: {value}")]
    InvalidUptime { value: u64 },
}

impl AgentMetrics {
    /// Calculate metrics from a collection of agent snapshots
    ///
    /// # Arguments
    ///
    /// * `agents` - Slice of agent snapshots to analyze
    ///
    /// # Returns
    ///
    /// * `Ok(AgentMetrics)` - Calculated metrics
    /// * `Err(MetricsError::NoAgents)` - If agents slice is empty
    pub fn calculate(agents: &[AgentSnapshot]) -> Result<Self, MetricsError> {
        if agents.is_empty() {
            return Err(MetricsError::NoAgents);
        }

        let total_agents = agents.len();
        let mut active_agents = 0;
        let mut idle_agents = 0;
        let mut unhealthy_agents = 0;
        let mut total_uptime = 0u64;
        let mut total_health_score = 0.0;
        let mut status_distribution: HashMap<String, usize> = HashMap::new();
        let mut capability_counts: HashMap<String, usize> = HashMap::new();

        for agent in agents {
            // Count by status
            *status_distribution.entry(agent.status.clone()).or_insert(0) += 1;

            // Categorize agent states
            if agent.status == "active" || agent.status == "working" {
                active_agents += 1;
            } else if agent.status == "idle" {
                idle_agents += 1;
            }

            // Count unhealthy agents
            if agent.health_score < 0.5 {
                unhealthy_agents += 1;
            }

            // Accumulate uptime and health scores
            total_uptime =
                total_uptime
                    .checked_add(agent.uptime_secs)
                    .ok_or(MetricsError::InvalidUptime {
                        value: agent.uptime_secs,
                    })?;
            total_health_score += agent.health_score;

            // Count capabilities
            for capability in &agent.capabilities {
                *capability_counts.entry(capability.clone()).or_insert(0) += 1;
            }
        }

        let average_uptime_secs = if total_agents > 0 {
            total_uptime as f64 / total_agents as f64
        } else {
            0.0
        };

        let average_health_score = if total_agents > 0 {
            total_health_score / total_agents as f64
        } else {
            0.0
        };

        Ok(AgentMetrics {
            total_agents,
            active_agents,
            idle_agents,
            unhealthy_agents,
            average_uptime_secs,
            average_health_score,
            status_distribution,
            capability_counts,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn create_test_agent(
        id: &str,
        status: &str,
        health_score: f64,
        uptime_secs: u64,
        capabilities: Vec<&str>,
    ) -> AgentSnapshot {
        AgentSnapshot {
            id: id.to_string(),
            status: status.to_string(),
            current_bead: None,
            health_score,
            uptime_secs,
            capabilities: capabilities.into_iter().map(String::from).collect(),
        }
    }

    #[test]
    fn test_calculate_metrics_with_empty_agents() {
        let agents = vec![];
        let result = AgentMetrics::calculate(&agents);

        assert!(matches!(result, Err(MetricsError::NoAgents)));
    }

    #[test]
    fn test_calculate_metrics_with_single_agent() {
        let agents = vec![create_test_agent("agent-1", "idle", 1.0, 100, vec!["rust"])];

        let metrics = AgentMetrics::calculate(&agents).expect("metrics should calculate");

        assert_eq!(metrics.total_agents, 1);
        assert_eq!(metrics.active_agents, 0);
        assert_eq!(metrics.idle_agents, 1);
        assert_eq!(metrics.unhealthy_agents, 0);
        assert_eq!(metrics.average_uptime_secs, 100.0);
        assert_eq!(metrics.average_health_score, 1.0);
        assert_eq!(metrics.status_distribution.get("idle"), Some(&1));
        assert_eq!(metrics.capability_counts.get("rust"), Some(&1));
    }

    #[test]
    fn test_calculate_metrics_with_multiple_agents() {
        let agents = vec![
            create_test_agent("agent-1", "active", 1.0, 100, vec!["rust", "python"]),
            create_test_agent("agent-2", "idle", 0.8, 200, vec!["rust"]),
            create_test_agent("agent-3", "working", 0.3, 50, vec!["go"]),
        ];

        let metrics = AgentMetrics::calculate(&agents).expect("metrics should calculate");

        assert_eq!(metrics.total_agents, 3);
        assert_eq!(metrics.active_agents, 2); // active + working
        assert_eq!(metrics.idle_agents, 1);
        assert_eq!(metrics.unhealthy_agents, 1); // health_score < 0.5
        assert_eq!(metrics.average_uptime_secs, (100.0 + 200.0 + 50.0) / 3.0);
        assert_eq!(metrics.average_health_score, (1.0 + 0.8 + 0.3) / 3.0);
    }

    #[test]
    fn test_calculate_metrics_status_distribution() {
        let agents = vec![
            create_test_agent("agent-1", "active", 1.0, 100, vec![]),
            create_test_agent("agent-2", "active", 1.0, 100, vec![]),
            create_test_agent("agent-3", "idle", 1.0, 100, vec![]),
            create_test_agent("agent-4", "terminated", 0.0, 0, vec![]),
        ];

        let metrics = AgentMetrics::calculate(&agents).expect("metrics should calculate");

        assert_eq!(metrics.status_distribution.get("active"), Some(&2));
        assert_eq!(metrics.status_distribution.get("idle"), Some(&1));
        assert_eq!(metrics.status_distribution.get("terminated"), Some(&1));
    }

    #[test]
    fn test_calculate_metrics_capability_counts() {
        let agents = vec![
            create_test_agent("agent-1", "active", 1.0, 100, vec!["rust", "python"]),
            create_test_agent("agent-2", "idle", 1.0, 200, vec!["rust", "go"]),
            create_test_agent("agent-3", "working", 1.0, 50, vec!["python"]),
        ];

        let metrics = AgentMetrics::calculate(&agents).expect("metrics should calculate");

        assert_eq!(metrics.capability_counts.get("rust"), Some(&2));
        assert_eq!(metrics.capability_counts.get("python"), Some(&2));
        assert_eq!(metrics.capability_counts.get("go"), Some(&1));
    }

    #[test]
    fn test_calculate_metrics_unhealthy_threshold() {
        let agents = vec![
            create_test_agent("agent-1", "active", 0.6, 100, vec![]), // healthy
            create_test_agent("agent-2", "idle", 0.5, 100, vec![]),   // healthy (boundary)
            create_test_agent("agent-3", "active", 0.4, 100, vec![]), // unhealthy
            create_test_agent("agent-4", "idle", 0.0, 100, vec![]),   // unhealthy
        ];

        let metrics = AgentMetrics::calculate(&agents).expect("metrics should calculate");

        assert_eq!(metrics.unhealthy_agents, 2);
    }

    #[test]
    fn test_calculate_metrics_average_uptime() {
        let agents = vec![
            create_test_agent("agent-1", "active", 1.0, 0, vec![]),
            create_test_agent("agent-2", "idle", 1.0, 100, vec![]),
            create_test_agent("agent-3", "working", 1.0, 200, vec![]),
        ];

        let metrics = AgentMetrics::calculate(&agents).expect("metrics should calculate");

        assert_eq!(metrics.average_uptime_secs, 100.0);
    }

    #[test]
    fn test_calculate_metrics_average_health_score() {
        let agents = vec![
            create_test_agent("agent-1", "active", 1.0, 100, vec![]),
            create_test_agent("agent-2", "idle", 0.5, 100, vec![]),
            create_test_agent("agent-3", "working", 0.0, 100, vec![]),
        ];

        let metrics = AgentMetrics::calculate(&agents).expect("metrics should calculate");

        assert_eq!(metrics.average_health_score, 0.5);
    }
}
