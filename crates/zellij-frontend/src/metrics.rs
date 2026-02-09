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
}
