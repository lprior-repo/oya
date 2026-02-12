//! Metrics aggregation for orchestrator data
//!
//! Provides immutable data structures for collecting and displaying
//! agent pool and individual agent metrics.

use rpds::Vector;

/// Individual agent statistics
#[derive(Debug, Clone, PartialEq)]
pub struct AgentMetrics {
    pub id: String,
    pub state: String,
    pub uptime_secs: u64,
    pub beads_completed: u64,
    pub operations_executed: u64,
    pub avg_execution_secs: Option<f64>,
    pub health_score: f64,
}

/// Pool-wide statistics
#[derive(Debug, Clone, PartialEq)]
pub struct PoolMetrics {
    pub total: usize,
    pub idle: usize,
    pub working: usize,
    pub unhealthy: usize,
    pub shutting_down: usize,
    pub terminated: usize,
}

/// Pool statistics from IPC HostMessage::AgentPoolStats
#[derive(Debug, Clone, Default, PartialEq)]
pub struct PoolStats {
    pub total_agents: usize,
    pub active_agents: usize,
    pub idle_agents: usize,
    pub beads_assigned: usize,
    pub beads_completed: usize,
}

impl PoolStats {
    pub fn new(
        total_agents: usize,
        active_agents: usize,
        idle_agents: usize,
        beads_assigned: usize,
        beads_completed: usize,
    ) -> Self {
        Self {
            total_agents,
            active_agents,
            idle_agents,
            beads_assigned,
            beads_completed,
        }
    }

    pub fn utilization_percent(&self) -> u8 {
        if self.total_agents == 0 {
            return 0;
        }
        let percent = (self.active_agents * 100) / self.total_agents;
        percent.min(100) as u8
    }
}

/// Aggregated metrics from the orchestrator
#[derive(Debug, Clone, PartialEq)]
pub struct MetricsSnapshot {
    pub pool: PoolMetrics,
    pub agents: Vector<AgentMetrics>,
    pub timestamp: i64,
}

impl MetricsSnapshot {
    /// Create a new metrics snapshot
    pub fn new(pool: PoolMetrics, agents: Vector<AgentMetrics>, timestamp: i64) -> Self {
        Self {
            pool,
            agents,
            timestamp,
        }
    }

    /// Format metrics for Zellij display
    pub fn format_for_zellij(&self) -> String {
        let mut output = String::new();

        output.push_str("┌─ Orchestrator Metrics ─────────────┐\n");
        output.push_str(&format!(
            "│ Total: {} │ Idle: {} │ Working: {} │\n",
            self.pool.total, self.pool.idle, self.pool.working
        ));
        output.push_str("├────────────────────────────────────┤\n");

        for agent in self.agents.iter() {
            output.push_str(&format!(
                "│ {:<12} {:<6} {:>3}% │\n",
                agent.id, agent.state, agent.health_score as i32
            ));
        }

        output.push_str("└────────────────────────────────────┘\n");

        output
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Utc;

    #[test]
    fn test_create_metrics_snapshot() {
        let pool = PoolMetrics {
            total: 5,
            idle: 2,
            working: 2,
            unhealthy: 1,
            shutting_down: 0,
            terminated: 0,
        };

        let agents = Vector::from_iter(vec![AgentMetrics {
            id: "agent-1".to_string(),
            state: "working".to_string(),
            uptime_secs: 3600,
            beads_completed: 10,
            operations_executed: 50,
            avg_execution_secs: Some(1.5),
            health_score: 95.0,
        }]);

        let snapshot = MetricsSnapshot::new(pool.clone(), agents, Utc::now().timestamp());

        assert_eq!(snapshot.pool, pool);
        assert_eq!(snapshot.agents.len(), 1);
    }

    #[test]
    fn test_format_for_zellij() {
        let pool = PoolMetrics {
            total: 2,
            idle: 1,
            working: 1,
            unhealthy: 0,
            shutting_down: 0,
            terminated: 0,
        };

        let agents = Vector::from_iter(vec![AgentMetrics {
            id: "agent-1".to_string(),
            state: "idle".to_string(),
            uptime_secs: 3600,
            beads_completed: 10,
            operations_executed: 50,
            avg_execution_secs: Some(1.5),
            health_score: 95.0,
        }]);

        let snapshot = MetricsSnapshot::new(pool, agents, 0);

        let output = snapshot.format_for_zellij();

        assert!(output.contains("Orchestrator Metrics"));
        assert!(output.contains("Total: 2"));
        assert!(output.contains("agent-1"));
        assert!(output.contains("95%"));
    }

    #[test]
    fn test_empty_agents_list() {
        let pool = PoolMetrics {
            total: 0,
            idle: 0,
            working: 0,
            unhealthy: 0,
            shutting_down: 0,
            terminated: 0,
        };

        let agents = Vector::new();
        let snapshot = MetricsSnapshot::new(pool, agents, 0);

        let output = snapshot.format_for_zellij();

        assert!(output.contains("Total: 0"));
    }

    #[test]
    fn test_pool_stats_default() {
        let stats = PoolStats::default();
        assert_eq!(stats.total_agents, 0);
        assert_eq!(stats.active_agents, 0);
        assert_eq!(stats.idle_agents, 0);
        assert_eq!(stats.beads_assigned, 0);
        assert_eq!(stats.beads_completed, 0);
    }

    #[test]
    fn test_pool_stats_new() {
        let stats = PoolStats::new(10, 7, 3, 15, 42);
        assert_eq!(stats.total_agents, 10);
        assert_eq!(stats.active_agents, 7);
        assert_eq!(stats.idle_agents, 3);
        assert_eq!(stats.beads_assigned, 15);
        assert_eq!(stats.beads_completed, 42);
    }

    #[test]
    fn test_pool_stats_utilization() {
        let stats = PoolStats::new(10, 5, 5, 0, 0);
        assert_eq!(stats.utilization_percent(), 50);
    }

    #[test]
    fn test_pool_stats_utilization_zero_agents() {
        let stats = PoolStats::default();
        assert_eq!(stats.utilization_percent(), 0);
    }

    #[test]
    fn test_pool_stats_utilization_full() {
        let stats = PoolStats::new(4, 4, 0, 0, 0);
        assert_eq!(stats.utilization_percent(), 100);
    }
}
