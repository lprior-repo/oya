//! Round-robin distribution strategy.

use std::sync::atomic::{AtomicUsize, Ordering};

use super::strategy::{DistributionContext, DistributionStrategy};

/// Round-robin distribution strategy.
///
/// Distributes beads and agents in a rotating fashion for fairness.
#[derive(Debug)]
pub struct RoundRobinStrategy {
    /// Current bead index.
    bead_index: AtomicUsize,
    /// Current agent index.
    agent_index: AtomicUsize,
}

impl Default for RoundRobinStrategy {
    fn default() -> Self {
        Self::new()
    }
}

impl RoundRobinStrategy {
    /// Create a new round-robin strategy.
    #[must_use]
    pub fn new() -> Self {
        Self {
            bead_index: AtomicUsize::new(0),
            agent_index: AtomicUsize::new(0),
        }
    }

    /// Reset the bead index.
    pub fn reset_bead_index(&self) {
        self.bead_index.store(0, Ordering::SeqCst);
    }

    /// Reset the agent index.
    pub fn reset_agent_index(&self) {
        self.agent_index.store(0, Ordering::SeqCst);
    }

    /// Reset both indices.
    pub fn reset(&self) {
        self.reset_bead_index();
        self.reset_agent_index();
    }

    /// Get current bead index.
    #[must_use]
    pub fn current_bead_index(&self) -> usize {
        self.bead_index.load(Ordering::SeqCst)
    }

    /// Get current agent index.
    #[must_use]
    pub fn current_agent_index(&self) -> usize {
        self.agent_index.load(Ordering::SeqCst)
    }
}

impl DistributionStrategy for RoundRobinStrategy {
    fn select_bead(&self, ready_beads: &[String], _ctx: &DistributionContext) -> Option<String> {
        if ready_beads.is_empty() {
            return None;
        }

        let index = self.bead_index.fetch_add(1, Ordering::SeqCst) % ready_beads.len();
        ready_beads.get(index).cloned()
    }

    fn select_agent(
        &self,
        _bead_id: &str,
        agents: &[String],
        _ctx: &DistributionContext,
    ) -> Option<String> {
        if agents.is_empty() {
            return None;
        }

        let index = self.agent_index.fetch_add(1, Ordering::SeqCst) % agents.len();
        agents.get(index).cloned()
    }

    fn name(&self) -> &'static str {
        "round_robin"
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_round_robin_strategy_name() {
        let strategy = RoundRobinStrategy::new();
        assert_eq!(strategy.name(), "round_robin");
    }

    #[test]
    fn test_round_robin_select_bead_empty() {
        let strategy = RoundRobinStrategy::new();
        let ctx = DistributionContext::new();

        let result = strategy.select_bead(&[], &ctx);
        assert!(result.is_none());
    }

    #[test]
    fn test_round_robin_select_bead_single() {
        let strategy = RoundRobinStrategy::new();
        let ctx = DistributionContext::new();

        let beads = vec!["bead-1".to_string()];

        // Should always return the only bead
        for _ in 0..5 {
            let result = strategy.select_bead(&beads, &ctx);
            assert_eq!(result, Some("bead-1".to_string()));
        }
    }

    #[test]
    fn test_round_robin_select_bead_rotates() {
        let strategy = RoundRobinStrategy::new();
        let ctx = DistributionContext::new();

        let beads = vec![
            "bead-a".to_string(),
            "bead-b".to_string(),
            "bead-c".to_string(),
        ];

        // Should rotate through beads
        assert_eq!(
            strategy.select_bead(&beads, &ctx),
            Some("bead-a".to_string())
        );
        assert_eq!(
            strategy.select_bead(&beads, &ctx),
            Some("bead-b".to_string())
        );
        assert_eq!(
            strategy.select_bead(&beads, &ctx),
            Some("bead-c".to_string())
        );
        // Wraps around
        assert_eq!(
            strategy.select_bead(&beads, &ctx),
            Some("bead-a".to_string())
        );
        assert_eq!(
            strategy.select_bead(&beads, &ctx),
            Some("bead-b".to_string())
        );
    }

    #[test]
    fn test_round_robin_select_agent_empty() {
        let strategy = RoundRobinStrategy::new();
        let ctx = DistributionContext::new();

        let result = strategy.select_agent("bead-1", &[], &ctx);
        assert!(result.is_none());
    }

    #[test]
    fn test_round_robin_select_agent_rotates() {
        let strategy = RoundRobinStrategy::new();
        let ctx = DistributionContext::new();

        let agents = vec![
            "agent-1".to_string(),
            "agent-2".to_string(),
            "agent-3".to_string(),
        ];

        // Should rotate through agents
        assert_eq!(
            strategy.select_agent("b", &agents, &ctx),
            Some("agent-1".to_string())
        );
        assert_eq!(
            strategy.select_agent("b", &agents, &ctx),
            Some("agent-2".to_string())
        );
        assert_eq!(
            strategy.select_agent("b", &agents, &ctx),
            Some("agent-3".to_string())
        );
        // Wraps around
        assert_eq!(
            strategy.select_agent("b", &agents, &ctx),
            Some("agent-1".to_string())
        );
    }

    #[test]
    fn test_round_robin_bead_and_agent_independent() {
        let strategy = RoundRobinStrategy::new();
        let ctx = DistributionContext::new();

        let beads = vec!["b1".to_string(), "b2".to_string()];
        let agents = vec!["a1".to_string(), "a2".to_string(), "a3".to_string()];

        // Bead selection
        assert_eq!(strategy.select_bead(&beads, &ctx), Some("b1".to_string()));
        // Agent selection (independent counter)
        assert_eq!(
            strategy.select_agent("x", &agents, &ctx),
            Some("a1".to_string())
        );
        // Bead advances independently
        assert_eq!(strategy.select_bead(&beads, &ctx), Some("b2".to_string()));
        // Agent advances independently
        assert_eq!(
            strategy.select_agent("x", &agents, &ctx),
            Some("a2".to_string())
        );
    }

    #[test]
    fn test_round_robin_reset_bead_index() {
        let strategy = RoundRobinStrategy::new();
        let ctx = DistributionContext::new();

        let beads = vec!["a".to_string(), "b".to_string(), "c".to_string()];

        // Advance index
        let _ = strategy.select_bead(&beads, &ctx);
        let _ = strategy.select_bead(&beads, &ctx);
        assert_eq!(strategy.current_bead_index(), 2);

        // Reset
        strategy.reset_bead_index();
        assert_eq!(strategy.current_bead_index(), 0);
        assert_eq!(strategy.select_bead(&beads, &ctx), Some("a".to_string()));
    }

    #[test]
    fn test_round_robin_reset_agent_index() {
        let strategy = RoundRobinStrategy::new();
        let ctx = DistributionContext::new();

        let agents = vec!["a".to_string(), "b".to_string()];

        // Advance index
        let _ = strategy.select_agent("x", &agents, &ctx);
        assert_eq!(strategy.current_agent_index(), 1);

        // Reset
        strategy.reset_agent_index();
        assert_eq!(strategy.current_agent_index(), 0);
        assert_eq!(
            strategy.select_agent("x", &agents, &ctx),
            Some("a".to_string())
        );
    }

    #[test]
    fn test_round_robin_reset_both() {
        let strategy = RoundRobinStrategy::new();
        let ctx = DistributionContext::new();

        let beads = vec!["b1".to_string(), "b2".to_string()];
        let agents = vec!["a1".to_string(), "a2".to_string()];

        // Advance both
        let _ = strategy.select_bead(&beads, &ctx);
        let _ = strategy.select_agent("x", &agents, &ctx);

        // Reset both
        strategy.reset();
        assert_eq!(strategy.current_bead_index(), 0);
        assert_eq!(strategy.current_agent_index(), 0);
    }

    #[test]
    fn test_round_robin_handles_changing_list_size() {
        let strategy = RoundRobinStrategy::new();
        let ctx = DistributionContext::new();

        // Start with 3 beads
        let beads_3 = vec!["a".to_string(), "b".to_string(), "c".to_string()];
        let _ = strategy.select_bead(&beads_3, &ctx); // index becomes 1
        let _ = strategy.select_bead(&beads_3, &ctx); // index becomes 2

        // Now use 2 beads - index 2 % 2 = 0
        let beads_2 = vec!["x".to_string(), "y".to_string()];
        let result = strategy.select_bead(&beads_2, &ctx); // index 2 % 2 = 0
        assert_eq!(result, Some("x".to_string()));
    }

    #[test]
    fn test_round_robin_fairness_distribution() {
        let strategy = RoundRobinStrategy::new();
        let ctx = DistributionContext::new();

        let agents = vec![
            "agent-1".to_string(),
            "agent-2".to_string(),
            "agent-3".to_string(),
        ];

        let mut counts = std::collections::HashMap::new();

        // Select 300 times
        for _ in 0..300 {
            if let Some(agent) = strategy.select_agent("bead", &agents, &ctx) {
                *counts.entry(agent).or_insert(0) += 1;
            }
        }

        // Each agent should be selected exactly 100 times
        assert_eq!(counts.get("agent-1"), Some(&100));
        assert_eq!(counts.get("agent-2"), Some(&100));
        assert_eq!(counts.get("agent-3"), Some(&100));
    }

    #[test]
    fn test_round_robin_validate_ok() {
        let strategy = RoundRobinStrategy::new();
        assert!(strategy.validate().is_ok());
    }

    #[test]
    fn test_round_robin_default() {
        let strategy = RoundRobinStrategy::default();
        assert_eq!(strategy.current_bead_index(), 0);
        assert_eq!(strategy.current_agent_index(), 0);
    }

    #[test]
    fn test_round_robin_large_index_overflow() {
        let strategy = RoundRobinStrategy::new();
        let ctx = DistributionContext::new();

        // Set a high starting index
        strategy.bead_index.store(usize::MAX - 1, Ordering::SeqCst);

        let beads = vec!["a".to_string(), "b".to_string(), "c".to_string()];

        // Should handle overflow gracefully
        let result1 = strategy.select_bead(&beads, &ctx);
        let result2 = strategy.select_bead(&beads, &ctx);
        let result3 = strategy.select_bead(&beads, &ctx);

        // Results should be valid (even if order changes due to overflow)
        assert!(result1.is_some());
        assert!(result2.is_some());
        assert!(result3.is_some());
    }
}
