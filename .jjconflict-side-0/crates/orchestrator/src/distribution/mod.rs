//! Task distribution strategies for the orchestrator.
//!
//! This module provides various strategies for distributing beads to agents:
//!
//! - `FifoStrategy`: First-in-first-out ordering
//! - `PriorityStrategy`: Priority-based selection with load balancing
//! - `RoundRobinStrategy`: Fair rotation between agents
//! - `AffinityStrategy`: Capability and preference-based matching
//!
//! # Example
//!
//! ```ignore
//! use orchestrator::distribution::{
//!     DistributionStrategy, FifoStrategy, PriorityStrategy,
//!     DistributionContext, BeadMetadata, AgentMetadata,
//! };
//!
//! // Create a priority-based strategy
//! let strategy = PriorityStrategy::new();
//!
//! // Build context with metadata
//! let ctx = DistributionContext::new()
//!     .with_bead(BeadMetadata::new("bead-1").with_priority(10))
//!     .with_bead(BeadMetadata::new("bead-2").with_priority(5))
//!     .with_agent(AgentMetadata::new("agent-1").with_load(0.3));
//!
//! // Select next bead to execute
//! let beads = vec!["bead-1".to_string(), "bead-2".to_string()];
//! let next = strategy.select_bead(&beads, &ctx);
//! assert_eq!(next, Some("bead-1".to_string())); // Higher priority
//!
//! // Select agent to execute it
//! let agents = vec!["agent-1".to_string()];
//! let agent = strategy.select_agent("bead-1", &agents, &ctx);
//! ```

mod affinity;
mod error;
mod fifo;
mod priority;
mod round_robin;
mod strategy;

pub use affinity::{AffinityMode, AffinityStrategy};
pub use error::{DistributionError, DistributionResult};
pub use fifo::FifoStrategy;
pub use priority::PriorityStrategy;
pub use round_robin::RoundRobinStrategy;
pub use strategy::{AgentMetadata, BeadMetadata, DistributionContext, DistributionStrategy};

/// Create a boxed distribution strategy by name.
///
/// # Supported names
///
/// - `"fifo"` - First-in-first-out
/// - `"priority"` - Priority-based
/// - `"round_robin"` - Round-robin
/// - `"affinity"` - Affinity-based (soft mode)
/// - `"affinity_hard"` - Affinity-based (hard mode)
///
/// # Returns
///
/// Returns `None` if the strategy name is not recognized.
#[must_use]
pub fn create_strategy(name: &str) -> Option<Box<dyn DistributionStrategy>> {
    match name {
        "fifo" => Some(Box::new(FifoStrategy::new())),
        "priority" => Some(Box::new(PriorityStrategy::new())),
        "round_robin" => Some(Box::new(RoundRobinStrategy::new())),
        "affinity" => Some(Box::new(AffinityStrategy::soft())),
        "affinity_hard" => Some(Box::new(AffinityStrategy::hard())),
        _ => None,
    }
}

/// Get a list of available strategy names.
#[must_use]
pub fn available_strategies() -> &'static [&'static str] {
    &[
        "fifo",
        "priority",
        "round_robin",
        "affinity",
        "affinity_hard",
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_create_strategy_fifo() {
        let strategy = create_strategy("fifo");
        assert!(strategy.is_some());
        assert_eq!(strategy.map(|s| s.name()), Some("fifo"));
    }

    #[test]
    fn test_create_strategy_priority() {
        let strategy = create_strategy("priority");
        assert!(strategy.is_some());
        assert_eq!(strategy.map(|s| s.name()), Some("priority"));
    }

    #[test]
    fn test_create_strategy_round_robin() {
        let strategy = create_strategy("round_robin");
        assert!(strategy.is_some());
        assert_eq!(strategy.map(|s| s.name()), Some("round_robin"));
    }

    #[test]
    fn test_create_strategy_affinity() {
        let strategy = create_strategy("affinity");
        assert!(strategy.is_some());
        assert_eq!(strategy.map(|s| s.name()), Some("affinity"));
    }

    #[test]
    fn test_create_strategy_affinity_hard() {
        let strategy = create_strategy("affinity_hard");
        assert!(strategy.is_some());
        assert_eq!(strategy.map(|s| s.name()), Some("affinity_hard"));
    }

    #[test]
    fn test_create_strategy_unknown() {
        let strategy = create_strategy("unknown");
        assert!(strategy.is_none());
    }

    #[test]
    fn test_available_strategies() {
        let strategies = available_strategies();
        assert!(strategies.contains(&"fifo"));
        assert!(strategies.contains(&"priority"));
        assert!(strategies.contains(&"round_robin"));
        assert!(strategies.contains(&"affinity"));
        assert!(strategies.contains(&"affinity_hard"));
    }

    #[test]
    fn test_strategy_trait_object() {
        let strategies: Vec<Box<dyn DistributionStrategy>> = vec![
            Box::new(FifoStrategy::new()),
            Box::new(PriorityStrategy::new()),
            Box::new(RoundRobinStrategy::new()),
            Box::new(AffinityStrategy::new()),
        ];

        let ctx = DistributionContext::new();
        let beads = vec!["b1".to_string(), "b2".to_string()];

        for strategy in &strategies {
            let result = strategy.select_bead(&beads, &ctx);
            assert!(result.is_some());
        }
    }

    #[test]
    fn test_all_strategies_validate() {
        for name in available_strategies() {
            if let Some(strategy) = create_strategy(name) {
                // All default strategies should validate
                assert!(
                    strategy.validate().is_ok(),
                    "Strategy {} should validate",
                    name
                );
            }
        }
    }

    #[test]
    fn test_strategy_interchangeability() {
        let ctx = DistributionContext::new()
            .with_bead(BeadMetadata::new("bead-1").with_priority(10))
            .with_bead(BeadMetadata::new("bead-2").with_priority(5))
            .with_agent(AgentMetadata::new("agent-1").with_load(0.3))
            .with_agent(AgentMetadata::new("agent-2").with_load(0.7));

        let beads = vec!["bead-1".to_string(), "bead-2".to_string()];
        let agents = vec!["agent-1".to_string(), "agent-2".to_string()];

        // All strategies should work with the same context
        for name in available_strategies() {
            if let Some(strategy) = create_strategy(name) {
                let bead = strategy.select_bead(&beads, &ctx);
                assert!(bead.is_some(), "Strategy {} should select a bead", name);

                let agent = strategy.select_agent("bead-1", &agents, &ctx);
                assert!(agent.is_some(), "Strategy {} should select an agent", name);
            }
        }
    }

    #[test]
    fn test_empty_inputs() {
        let ctx = DistributionContext::new();
        let empty_beads: Vec<String> = vec![];
        let empty_agents: Vec<String> = vec![];

        for name in available_strategies() {
            if let Some(strategy) = create_strategy(name) {
                let bead = strategy.select_bead(&empty_beads, &ctx);
                assert!(
                    bead.is_none(),
                    "Strategy {} should return None for empty beads",
                    name
                );

                let agent = strategy.select_agent("bead", &empty_agents, &ctx);
                assert!(
                    agent.is_none(),
                    "Strategy {} should return None for empty agents",
                    name
                );
            }
        }
    }
}
