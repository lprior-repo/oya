//! Priority-based distribution strategy.

use super::strategy::{DistributionContext, DistributionStrategy};

/// Priority-based distribution strategy.
///
/// Selects beads with highest priority first.
/// Selects agents with lowest load.
#[derive(Debug, Clone)]
pub struct PriorityStrategy {
    /// Default priority for beads without metadata.
    default_priority: i32,
    /// Whether to prefer agents with matching capabilities.
    capability_matching: bool,
}

impl Default for PriorityStrategy {
    fn default() -> Self {
        Self {
            default_priority: 0,
            capability_matching: true,
        }
    }
}

impl PriorityStrategy {
    /// Create a new priority strategy.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Set the default priority for beads without metadata.
    #[must_use]
    pub const fn with_default_priority(mut self, priority: i32) -> Self {
        self.default_priority = priority;
        self
    }

    /// Enable or disable capability matching for agent selection.
    #[must_use]
    pub const fn with_capability_matching(mut self, enabled: bool) -> Self {
        self.capability_matching = enabled;
        self
    }

    /// Get the priority for a bead.
    fn get_priority(&self, bead_id: &str, ctx: &DistributionContext) -> i32 {
        ctx.get_bead(bead_id)
            .map(|b| b.priority)
            .unwrap_or(self.default_priority)
    }

    /// Get the load for an agent.
    fn get_load(&self, agent_id: &str, ctx: &DistributionContext) -> f64 {
        ctx.get_agent(agent_id).map(|a| a.load).unwrap_or(0.5)
    }

    /// Check if agent has required capabilities for bead.
    fn agent_matches_bead(&self, agent_id: &str, bead_id: &str, ctx: &DistributionContext) -> bool {
        if !self.capability_matching {
            return true;
        }

        let bead = match ctx.get_bead(bead_id) {
            Some(b) => b,
            None => return true, // No metadata = no requirements
        };

        if bead.required_capabilities.is_empty() {
            return true;
        }

        let agent = match ctx.get_agent(agent_id) {
            Some(a) => a,
            None => return false, // No agent metadata but bead has requirements
        };

        // Agent must have all required capabilities
        bead.required_capabilities
            .iter()
            .all(|cap| agent.has_capability(cap))
    }
}

impl DistributionStrategy for PriorityStrategy {
    fn select_bead(&self, ready_beads: &[String], ctx: &DistributionContext) -> Option<String> {
        if ready_beads.is_empty() {
            return None;
        }

        // Find bead with highest priority
        ready_beads
            .iter()
            .max_by_key(|bead_id| self.get_priority(bead_id, ctx))
            .cloned()
    }

    fn select_agent(
        &self,
        bead_id: &str,
        agents: &[String],
        ctx: &DistributionContext,
    ) -> Option<String> {
        if agents.is_empty() {
            return None;
        }

        // Filter agents that match capability requirements
        let matching_agents: Vec<_> = agents
            .iter()
            .filter(|agent_id| self.agent_matches_bead(agent_id, bead_id, ctx))
            .collect();

        if matching_agents.is_empty() {
            let has_requirements = ctx
                .get_bead(bead_id)
                .map(|bead| !bead.required_capabilities.is_empty())
                .unwrap_or(false);
            if self.capability_matching && has_requirements {
                return None;
            }

            // Fall back to any agent if no matches
            return agents
                .iter()
                .min_by(|a, b| {
                    self.get_load(a, ctx)
                        .partial_cmp(&self.get_load(b, ctx))
                        .unwrap_or(std::cmp::Ordering::Equal)
                })
                .cloned();
        }

        // Select agent with lowest load among matching agents
        matching_agents
            .into_iter()
            .min_by(|a, b| {
                self.get_load(a, ctx)
                    .partial_cmp(&self.get_load(b, ctx))
                    .unwrap_or(std::cmp::Ordering::Equal)
            })
            .cloned()
    }

    fn name(&self) -> &'static str {
        "priority"
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::distribution::strategy::{AgentMetadata, BeadMetadata};

    #[test]
    fn test_priority_strategy_name() {
        let strategy = PriorityStrategy::new();
        assert_eq!(strategy.name(), "priority");
    }

    #[test]
    fn test_priority_select_bead_empty() {
        let strategy = PriorityStrategy::new();
        let ctx = DistributionContext::new();

        let result = strategy.select_bead(&[], &ctx);
        assert!(result.is_none());
    }

    #[test]
    fn test_priority_select_bead_highest_priority() {
        let strategy = PriorityStrategy::new();
        let ctx = DistributionContext::new()
            .with_bead(BeadMetadata::new("low").with_priority(1))
            .with_bead(BeadMetadata::new("high").with_priority(100))
            .with_bead(BeadMetadata::new("medium").with_priority(50));

        let beads = vec!["low".to_string(), "high".to_string(), "medium".to_string()];
        let result = strategy.select_bead(&beads, &ctx);
        assert_eq!(result, Some("high".to_string()));
    }

    #[test]
    fn test_priority_select_bead_default_priority() {
        let strategy = PriorityStrategy::new().with_default_priority(10);
        let ctx = DistributionContext::new()
            .with_bead(BeadMetadata::new("low").with_priority(5))
            .with_bead(BeadMetadata::new("high").with_priority(20));
        // "no-metadata" will use default priority of 10

        let beads = vec![
            "low".to_string(),
            "no-metadata".to_string(),
            "high".to_string(),
        ];
        let result = strategy.select_bead(&beads, &ctx);
        assert_eq!(result, Some("high".to_string()));
    }

    #[test]
    fn test_priority_select_bead_negative_priorities() {
        let strategy = PriorityStrategy::new();
        let ctx = DistributionContext::new()
            .with_bead(BeadMetadata::new("a").with_priority(-10))
            .with_bead(BeadMetadata::new("b").with_priority(-5))
            .with_bead(BeadMetadata::new("c").with_priority(-20));

        let beads = vec!["a".to_string(), "b".to_string(), "c".to_string()];
        let result = strategy.select_bead(&beads, &ctx);
        assert_eq!(result, Some("b".to_string())); // -5 is highest
    }

    #[test]
    fn test_priority_select_agent_empty() {
        let strategy = PriorityStrategy::new();
        let ctx = DistributionContext::new();

        let result = strategy.select_agent("bead-1", &[], &ctx);
        assert!(result.is_none());
    }

    #[test]
    fn test_priority_select_agent_lowest_load() {
        let strategy = PriorityStrategy::new();
        let ctx = DistributionContext::new()
            .with_agent(AgentMetadata::new("busy").with_load(0.9))
            .with_agent(AgentMetadata::new("idle").with_load(0.1))
            .with_agent(AgentMetadata::new("medium").with_load(0.5));

        let agents = vec!["busy".to_string(), "idle".to_string(), "medium".to_string()];
        let result = strategy.select_agent("bead-1", &agents, &ctx);
        assert_eq!(result, Some("idle".to_string()));
    }

    #[test]
    fn test_priority_select_agent_with_capabilities() {
        let strategy = PriorityStrategy::new().with_capability_matching(true);
        let ctx = DistributionContext::new()
            .with_bead(BeadMetadata::new("rust-bead").with_capability("rust"))
            .with_agent(
                AgentMetadata::new("python-agent")
                    .with_capability("python")
                    .with_load(0.1),
            )
            .with_agent(
                AgentMetadata::new("rust-agent")
                    .with_capability("rust")
                    .with_load(0.5),
            );

        let agents = vec!["python-agent".to_string(), "rust-agent".to_string()];
        let result = strategy.select_agent("rust-bead", &agents, &ctx);
        assert_eq!(result, Some("rust-agent".to_string()));
    }

    #[test]
    fn test_priority_select_agent_multiple_capabilities() {
        let strategy = PriorityStrategy::new();
        let ctx = DistributionContext::new()
            .with_bead(
                BeadMetadata::new("multi-bead")
                    .with_capability("rust")
                    .with_capability("wasm"),
            )
            .with_agent(
                AgentMetadata::new("partial")
                    .with_capability("rust")
                    .with_load(0.1),
            )
            .with_agent(
                AgentMetadata::new("full")
                    .with_capabilities(vec!["rust".to_string(), "wasm".to_string()])
                    .with_load(0.5),
            );

        let agents = vec!["partial".to_string(), "full".to_string()];
        let result = strategy.select_agent("multi-bead", &agents, &ctx);
        assert_eq!(result, Some("full".to_string()));
    }

    #[test]
    fn test_priority_select_agent_no_matching_fallback() {
        let strategy = PriorityStrategy::new();
        let ctx = DistributionContext::new()
            .with_bead(BeadMetadata::new("java-bead").with_capability("java"))
            .with_agent(
                AgentMetadata::new("rust-agent")
                    .with_capability("rust")
                    .with_load(0.9),
            )
            .with_agent(
                AgentMetadata::new("python-agent")
                    .with_capability("python")
                    .with_load(0.1),
            );

        let agents = vec!["rust-agent".to_string(), "python-agent".to_string()];
        let result = strategy.select_agent("java-bead", &agents, &ctx);
        // Falls back to lowest load since no java capability
        assert_eq!(result, Some("python-agent".to_string()));
    }

    #[test]
    fn test_priority_capability_matching_disabled() {
        let strategy = PriorityStrategy::new().with_capability_matching(false);
        let ctx = DistributionContext::new()
            .with_bead(BeadMetadata::new("rust-bead").with_capability("rust"))
            .with_agent(
                AgentMetadata::new("python-agent")
                    .with_capability("python")
                    .with_load(0.1),
            )
            .with_agent(
                AgentMetadata::new("rust-agent")
                    .with_capability("rust")
                    .with_load(0.9),
            );

        let agents = vec!["python-agent".to_string(), "rust-agent".to_string()];
        let result = strategy.select_agent("rust-bead", &agents, &ctx);
        // Ignores capabilities, picks lowest load
        assert_eq!(result, Some("python-agent".to_string()));
    }

    #[test]
    fn test_priority_equal_priorities_stable() {
        let strategy = PriorityStrategy::new();
        let ctx = DistributionContext::new()
            .with_bead(BeadMetadata::new("a").with_priority(10))
            .with_bead(BeadMetadata::new("b").with_priority(10))
            .with_bead(BeadMetadata::new("c").with_priority(10));

        let beads = vec!["a".to_string(), "b".to_string(), "c".to_string()];

        // With equal priorities, selection should be stable
        let result1 = strategy.select_bead(&beads, &ctx);
        let result2 = strategy.select_bead(&beads, &ctx);
        assert_eq!(result1, result2);
    }

    #[test]
    fn test_priority_equal_load_stable() {
        let strategy = PriorityStrategy::new();
        let ctx = DistributionContext::new()
            .with_agent(AgentMetadata::new("a").with_load(0.5))
            .with_agent(AgentMetadata::new("b").with_load(0.5))
            .with_agent(AgentMetadata::new("c").with_load(0.5));

        let agents = vec!["a".to_string(), "b".to_string(), "c".to_string()];

        let result1 = strategy.select_agent("bead", &agents, &ctx);
        let result2 = strategy.select_agent("bead", &agents, &ctx);
        assert_eq!(result1, result2);
    }

    #[test]
    fn test_priority_validate_ok() {
        let strategy = PriorityStrategy::new();
        assert!(strategy.validate().is_ok());
    }

    #[test]
    fn test_priority_bead_no_requirements() {
        let strategy = PriorityStrategy::new();
        let ctx = DistributionContext::new()
            .with_bead(BeadMetadata::new("simple-bead")) // No capabilities required
            .with_agent(AgentMetadata::new("agent-1").with_load(0.9))
            .with_agent(AgentMetadata::new("agent-2").with_load(0.1));

        let agents = vec!["agent-1".to_string(), "agent-2".to_string()];
        let result = strategy.select_agent("simple-bead", &agents, &ctx);
        assert_eq!(result, Some("agent-2".to_string())); // Lowest load
    }

    #[test]
    fn test_priority_agent_no_metadata() {
        let strategy = PriorityStrategy::new();
        let ctx = DistributionContext::new(); // No agent metadata

        let agents = vec!["agent-1".to_string(), "agent-2".to_string()];
        let result = strategy.select_agent("bead-1", &agents, &ctx);
        // Should return some agent (first with 0.0 load)
        assert!(result.is_some());
    }
}
