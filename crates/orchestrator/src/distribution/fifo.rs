//! FIFO (First-In-First-Out) distribution strategy.

use super::strategy::{DistributionContext, DistributionStrategy};

/// FIFO distribution strategy.
///
/// Selects beads in the order they appear (first ready = first executed).
/// Selects the first available agent.
#[derive(Debug, Clone, Default)]
pub struct FifoStrategy;

impl FifoStrategy {
    /// Create a new FIFO strategy.
    #[must_use]
    pub fn new() -> Self {
        Self
    }
}

impl DistributionStrategy for FifoStrategy {
    fn select_bead(&self, ready_beads: &[String], _ctx: &DistributionContext) -> Option<String> {
        ready_beads.first().cloned()
    }

    fn select_agent(
        &self,
        _bead_id: &str,
        agents: &[String],
        _ctx: &DistributionContext,
    ) -> Option<String> {
        agents.first().cloned()
    }

    fn name(&self) -> &'static str {
        "fifo"
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::distribution::strategy::{AgentMetadata, BeadMetadata};

    #[test]
    fn test_fifo_strategy_name() {
        let strategy = FifoStrategy::new();
        assert_eq!(strategy.name(), "fifo");
    }

    #[test]
    fn test_fifo_select_bead_empty() {
        let strategy = FifoStrategy::new();
        let ctx = DistributionContext::new();

        let result = strategy.select_bead(&[], &ctx);
        assert!(result.is_none());
    }

    #[test]
    fn test_fifo_select_bead_single() {
        let strategy = FifoStrategy::new();
        let ctx = DistributionContext::new();

        let beads = vec!["bead-1".to_string()];
        let result = strategy.select_bead(&beads, &ctx);
        assert_eq!(result, Some("bead-1".to_string()));
    }

    #[test]
    fn test_fifo_select_bead_multiple() {
        let strategy = FifoStrategy::new();
        let ctx = DistributionContext::new();

        let beads = vec![
            "bead-1".to_string(),
            "bead-2".to_string(),
            "bead-3".to_string(),
        ];
        let result = strategy.select_bead(&beads, &ctx);
        assert_eq!(result, Some("bead-1".to_string()));
    }

    #[test]
    fn test_fifo_select_agent_empty() {
        let strategy = FifoStrategy::new();
        let ctx = DistributionContext::new();

        let result = strategy.select_agent("bead-1", &[], &ctx);
        assert!(result.is_none());
    }

    #[test]
    fn test_fifo_select_agent_single() {
        let strategy = FifoStrategy::new();
        let ctx = DistributionContext::new();

        let agents = vec!["agent-1".to_string()];
        let result = strategy.select_agent("bead-1", &agents, &ctx);
        assert_eq!(result, Some("agent-1".to_string()));
    }

    #[test]
    fn test_fifo_select_agent_multiple() {
        let strategy = FifoStrategy::new();
        let ctx = DistributionContext::new();

        let agents = vec![
            "agent-1".to_string(),
            "agent-2".to_string(),
            "agent-3".to_string(),
        ];
        let result = strategy.select_agent("bead-1", &agents, &ctx);
        assert_eq!(result, Some("agent-1".to_string()));
    }

    #[test]
    fn test_fifo_ignores_priority() {
        let strategy = FifoStrategy::new();
        let ctx = DistributionContext::new()
            .with_bead(BeadMetadata::new("bead-1").with_priority(1))
            .with_bead(BeadMetadata::new("bead-2").with_priority(100))
            .with_bead(BeadMetadata::new("bead-3").with_priority(50));

        // FIFO should return first in list regardless of priority
        let beads = vec![
            "bead-1".to_string(),
            "bead-2".to_string(),
            "bead-3".to_string(),
        ];
        let result = strategy.select_bead(&beads, &ctx);
        assert_eq!(result, Some("bead-1".to_string()));
    }

    #[test]
    fn test_fifo_ignores_agent_load() {
        let strategy = FifoStrategy::new();
        let ctx = DistributionContext::new()
            .with_agent(AgentMetadata::new("agent-1").with_load(0.9))
            .with_agent(AgentMetadata::new("agent-2").with_load(0.1))
            .with_agent(AgentMetadata::new("agent-3").with_load(0.5));

        // FIFO should return first in list regardless of load
        let agents = vec![
            "agent-1".to_string(),
            "agent-2".to_string(),
            "agent-3".to_string(),
        ];
        let result = strategy.select_agent("bead-1", &agents, &ctx);
        assert_eq!(result, Some("agent-1".to_string()));
    }

    #[test]
    fn test_fifo_validate_always_ok() {
        let strategy = FifoStrategy::new();
        assert!(strategy.validate().is_ok());
    }

    #[test]
    fn test_fifo_consistent_selection() {
        let strategy = FifoStrategy::new();
        let ctx = DistributionContext::new();

        let beads = vec![
            "bead-a".to_string(),
            "bead-b".to_string(),
            "bead-c".to_string(),
        ];

        // Multiple calls should return same result
        for _ in 0..10 {
            let result = strategy.select_bead(&beads, &ctx);
            assert_eq!(result, Some("bead-a".to_string()));
        }
    }

    #[test]
    fn test_fifo_order_preserved() {
        let strategy = FifoStrategy::new();
        let ctx = DistributionContext::new();

        // Test that order of input determines selection
        let beads_order1 = vec!["x".to_string(), "y".to_string(), "z".to_string()];
        let beads_order2 = vec!["z".to_string(), "x".to_string(), "y".to_string()];

        assert_eq!(
            strategy.select_bead(&beads_order1, &ctx),
            Some("x".to_string())
        );
        assert_eq!(
            strategy.select_bead(&beads_order2, &ctx),
            Some("z".to_string())
        );
    }
}
