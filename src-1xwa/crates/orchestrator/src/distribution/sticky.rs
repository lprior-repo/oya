//! Sticky distribution strategy with fallback logic.
//!
//! This strategy implements "soft sticky mode" which:
//! - Prefers to assign beads to the same worker that previously handled them
//! - Falls back to any idle worker if the previous worker is unavailable (busy/dead)
//! - Always returns a worker if any idle worker exists (never blocks)
//!
//! # Architecture
//!
//! The sticky strategy retrieves previous worker assignments from the distribution
//! context and uses them to inform agent selection:
//!
//! 1. **Sticky Hit**: Previous worker is idle → assign to them
//! 2. **Fallback (Busy)**: Previous worker is busy → assign to least-loaded idle worker
//! 3. **Fallback (Dead)**: Previous worker not in list → assign to least-loaded idle worker
//! 4. **No History**: No previous assignment → assign to least-loaded idle worker
//!
//! # Example
//!
//! ```ignore
//! use orchestrator::distribution::{DistributionStrategy, StickyStrategy};
//! use orchestrator::distribution::strategy::{DistributionContext, AgentMetadata, BeadMetadata};
//!
//! let strategy = StickyStrategy::new();
//!
//! let ctx = DistributionContext::new()
//!     .with_bead(BeadMetadata::new("bead-1"))
//!     .with_agent(AgentMetadata::new("worker-a").with_load(0.0))
//!     .with_agent(AgentMetadata::new("worker-b").with_load(0.5));
//!
//! // Set previous worker in context custom data
//! let mut ctx = ctx.clone();
//! ctx.custom.insert("previous_worker".to_string(), "worker-a".to_string());
//!
//! let agents = vec!["worker-a".to_string(), "worker-b".to_string()];
//! let selected = strategy.select_agent("bead-1", &agents, &ctx);
//!
//! assert_eq!(selected, Some("worker-a".to_string())); // Sticky hit!
//! ```

use std::f64;

use super::error::{DistributionError, DistributionResult};
use super::strategy::{DistributionContext, DistributionStrategy};

/// Sticky distribution strategy.
///
/// Implements soft sticky mode with automatic fallback to idle workers.
#[derive(Debug, Clone)]
pub struct StickyStrategy {
    /// Weight for sticky preference (0.0 - 1.0).
    ///
    /// Higher values give more priority to assigning beads to their previous workers.
    sticky_weight: f64,

    /// Weight for load balancing (0.0 - 1.0).
    ///
    /// Higher values give more priority to distributing load evenly.
    load_weight: f64,
}

impl Default for StickyStrategy {
    fn default() -> Self {
        Self {
            sticky_weight: 0.7,
            load_weight: 0.3,
        }
    }
}

impl StickyStrategy {
    /// Create a new sticky strategy with default weights.
    ///
    /// Default weights:
    /// - `sticky_weight`: 0.7 (prefer previous worker)
    /// - `load_weight`: 0.3 (balance load as secondary factor)
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Set the sticky weight.
    ///
    /// # Arguments
    ///
    /// * `weight` - Weight for sticky preference (clamped to [0.0, 1.0])
    #[must_use]
    pub const fn with_sticky_weight(mut self, weight: f64) -> Self {
        self.sticky_weight = weight;
        self
    }

    /// Set the load weight.
    ///
    /// # Arguments
    ///
    /// * `weight` - Weight for load balancing (clamped to [0.0, 1.0])
    #[must_use]
    pub const fn with_load_weight(mut self, weight: f64) -> Self {
        self.load_weight = weight;
        self
    }

    /// Extract the previous worker for a bead from the distribution context.
    ///
    /// # Arguments
    ///
    /// * `bead_id` - The bead ID to look up
    /// * `ctx` - The distribution context containing custom data
    ///
    /// # Returns
    ///
    /// * `Some(worker_id)` if a previous worker is recorded
    /// * `None` if no previous assignment exists
    fn get_previous_worker(&self, bead_id: &str, ctx: &DistributionContext) -> Option<String> {
        // Check custom context for "previous_worker" key
        if let Some(worker_id) = ctx.custom.get("previous_worker") {
            return Some(worker_id.clone());
        }

        // Check bead metadata for preferred_agents (contains previous worker)
        if let Some(bead) = ctx.get_bead(bead_id) {
            if let Some(prev_worker) = bead.preferred_agents.first() {
                return Some(prev_worker.clone());
            }
        }

        None
    }

    /// Check if a worker is idle based on load threshold.
    ///
    /// # Arguments
    ///
    /// * `worker_id` - The worker ID to check
    /// * `ctx` - The distribution context containing agent metadata
    ///
    /// # Returns
    ///
    /// * `Some(true)` if the worker is idle (load < 0.5)
    /// * `Some(false)` if the worker is busy (load >= 0.5)
    /// * `None` if worker metadata is not found
    fn is_worker_idle(&self, worker_id: &str, ctx: &DistributionContext) -> Option<bool> {
        ctx.get_agent(worker_id).map(|agent| agent.load < 0.5)
    }

    /// Calculate the idle score for a worker (higher = more idle).
    ///
    /// # Arguments
    ///
    /// * `worker_id` - The worker ID to score
    /// * `ctx` - The distribution context containing agent metadata
    ///
    /// # Returns
    ///
    /// * Score in range [0.0, 1.0], where 1.0 = completely idle (load = 0.0)
    fn idle_score(&self, worker_id: &str, ctx: &DistributionContext) -> f64 {
        ctx.get_agent(worker_id)
            .map(|agent| 1.0 - agent.load)
            .filter(|score| score.is_finite())
            .unwrap_or(0.5)
    }

    /// Select the least-loaded worker from a list of candidates.
    ///
    /// # Arguments
    ///
    /// * `candidates` - List of worker IDs to consider
    /// * `ctx` - The distribution context
    ///
    /// # Returns
    ///
    /// * `Some(worker_id)` with the least-loaded worker
    /// * `None` if candidates is empty
    #[allow(clippy::cast_precision_loss)]
    fn select_least_loaded(
        &self,
        candidates: &[String],
        ctx: &DistributionContext,
    ) -> Option<String> {
        candidates
            .iter()
            .min_by(|a, b| {
                let load_a = ctx.get_agent(a).map_or(1.0, |agent| agent.load);
                let load_b = ctx.get_agent(b).map_or(1.0, |agent| agent.load);
                load_a
                    .partial_cmp(&load_b)
                    .unwrap_or(std::cmp::Ordering::Equal)
            })
            .cloned()
    }

    /// Calculate the combined score for a worker (sticky + load).
    ///
    /// # Arguments
    ///
    /// * `worker_id` - The worker ID to score
    /// * `is_previous` - Whether this worker previously handled the bead
    /// * `ctx` - The distribution context
    ///
    /// # Returns
    ///
    /// * Combined score in range [0.0, 1.0]
    fn combined_score(&self, worker_id: &str, is_previous: bool, ctx: &DistributionContext) -> f64 {
        let sticky_score = if is_previous { 1.0 } else { 0.0 };
        let load_score = self.idle_score(worker_id, ctx);

        (sticky_score * self.sticky_weight) + (load_score * self.load_weight)
    }
}

impl DistributionStrategy for StickyStrategy {
    fn select_bead(&self, ready_beads: &[String], ctx: &DistributionContext) -> Option<String> {
        if ready_beads.is_empty() {
            return None;
        }

        // Select bead with highest priority (retry count as tiebreaker)
        ready_beads
            .iter()
            .max_by_key(|bead_id| {
                let bead = ctx.get_bead(bead_id);
                let priority = bead.map_or(0, |b| b.priority);
                let retry = bead.map_or(0, |b| b.retry_count);
                (priority, retry)
            })
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

        // Get previous worker assignment
        let previous_worker = self.get_previous_worker(bead_id, ctx);

        match previous_worker {
            // Case 1: Previous worker exists
            Some(prev_worker) => {
                // Check if previous worker is in the available agents list
                if agents.iter().any(|a| a == &prev_worker) {
                    // Check if previous worker is idle
                    match self.is_worker_idle(&prev_worker, ctx) {
                        Some(true) => {
                            // STICKY HIT: Previous worker is idle, assign to them
                            Some(prev_worker)
                        }
                        Some(false) => {
                            // FALLBACK (Busy): Previous worker is busy, select least-loaded idle worker
                            self.select_least_loaded(agents, ctx)
                        }
                        None => {
                            // FALLBACK (No metadata): Previous worker metadata missing, select least-loaded
                            self.select_least_loaded(agents, ctx)
                        }
                    }
                } else {
                    // FALLBACK (Dead): Previous worker not in agents list, select least-loaded
                    self.select_least_loaded(agents, ctx)
                }
            }
            // Case 2: No previous assignment
            None => {
                // Select least-loaded worker
                self.select_least_loaded(agents, ctx)
            }
        }
    }

    fn name(&self) -> &'static str {
        "sticky"
    }

    fn validate(&self) -> DistributionResult<()> {
        let total = self.sticky_weight + self.load_weight;
        if (total - 1.0).abs() > 0.01 {
            return Err(DistributionError::configuration(format!(
                "weights should sum to 1.0, got {total}"
            )));
        }

        // Validate weight ranges
        if self.sticky_weight < 0.0 || self.sticky_weight > 1.0 {
            return Err(DistributionError::configuration(format!(
                "sticky_weight must be in [0.0, 1.0], got {}",
                self.sticky_weight
            )));
        }

        if self.load_weight < 0.0 || self.load_weight > 1.0 {
            return Err(DistributionError::configuration(format!(
                "load_weight must be in [0.0, 1.0], got {}",
                self.load_weight
            )));
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::distribution::strategy::{AgentMetadata, BeadMetadata};

    /// Test helper to create a distribution context with agents.
    fn create_context_with_agents(agents: Vec<(&str, f64)>) -> DistributionContext {
        let mut ctx = DistributionContext::new();
        for (id, load) in agents {
            ctx = ctx.with_agent(AgentMetadata::new(id).with_load(load));
        }
        ctx
    }

    /// Test helper to set previous worker in context.
    fn set_previous_worker(mut ctx: DistributionContext, worker_id: &str) -> DistributionContext {
        ctx.custom
            .insert("previous_worker".to_string(), worker_id.to_string());
        ctx
    }

    #[test]
    fn test_sticky_strategy_name() {
        let strategy = StickyStrategy::new();
        assert_eq!(strategy.name(), "sticky");
    }

    #[test]
    fn test_sticky_select_bead_empty() {
        let strategy = StickyStrategy::new();
        let ctx = DistributionContext::new();

        let result = strategy.select_bead(&[], &ctx);
        assert!(result.is_none());
    }

    #[test]
    fn test_sticky_select_bead_by_priority() {
        let strategy = StickyStrategy::new();
        let ctx = DistributionContext::new()
            .with_bead(BeadMetadata::new("low").with_priority(1))
            .with_bead(BeadMetadata::new("high").with_priority(10))
            .with_bead(BeadMetadata::new("medium").with_priority(5));

        let beads = vec!["low".to_string(), "high".to_string(), "medium".to_string()];
        let result = strategy.select_bead(&beads, &ctx);
        assert_eq!(result, Some("high".to_string()));
    }

    #[test]
    fn test_sticky_select_agent_empty() {
        let strategy = StickyStrategy::new();
        let ctx = DistributionContext::new();

        let result = strategy.select_agent("bead", &[], &ctx);
        assert!(result.is_none());
    }

    #[test]
    fn test_sticky_prefer_previous_worker_idle() {
        // SCENARIO 1: Sticky Hit - Previous worker is idle
        let strategy = StickyStrategy::new();
        let ctx = create_context_with_agents(vec![("worker-a", 0.0), ("worker-b", 0.5)]);
        let ctx = set_previous_worker(ctx, "worker-a");

        let agents = vec!["worker-a".to_string(), "worker-b".to_string()];
        let result = strategy.select_agent("bead-1", &agents, &ctx);

        assert_eq!(
            result,
            Some("worker-a".to_string()),
            "Should prefer previous idle worker"
        );
    }

    #[test]
    fn test_sticky_fallback_previous_worker_busy() {
        // SCENARIO 2: Fallback (Busy) - Previous worker is busy
        let strategy = StickyStrategy::new();
        let ctx = create_context_with_agents(vec![("worker-a", 1.0), ("worker-b", 0.0)]);
        let ctx = set_previous_worker(ctx, "worker-a");

        let agents = vec!["worker-a".to_string(), "worker-b".to_string()];
        let result = strategy.select_agent("bead-1", &agents, &ctx);

        assert_eq!(
            result,
            Some("worker-b".to_string()),
            "Should fallback to idle worker when previous busy"
        );
    }

    #[test]
    fn test_sticky_fallback_previous_worker_dead() {
        // SCENARIO 3: Fallback (Dead) - Previous worker not in agents list
        let strategy = StickyStrategy::new();
        let ctx = create_context_with_agents(vec![("worker-b", 0.0), ("worker-c", 0.5)]);
        let ctx = set_previous_worker(ctx, "worker-a"); // worker-a not in agents

        let agents = vec!["worker-b".to_string(), "worker-c".to_string()];
        let result = strategy.select_agent("bead-1", &agents, &ctx);

        assert_eq!(
            result,
            Some("worker-b".to_string()),
            "Should fallback to available worker when previous dead"
        );
    }

    #[test]
    fn test_sticky_no_previous_assignment() {
        // SCENARIO 4: No previous assignment
        let strategy = StickyStrategy::new();
        let ctx = create_context_with_agents(vec![("worker-a", 0.7), ("worker-b", 0.3)]);

        let agents = vec!["worker-a".to_string(), "worker-b".to_string()];
        let result = strategy.select_agent("bead-1", &agents, &ctx);

        assert_eq!(
            result,
            Some("worker-b".to_string()),
            "Should select least-loaded worker when no previous assignment"
        );
    }

    #[test]
    fn test_sticky_validate_ok() {
        let strategy = StickyStrategy::new(); // Default weights sum to 1.0
        assert!(strategy.validate().is_ok());
    }

    #[test]
    fn test_sticky_validate_bad_weights() {
        let strategy = StickyStrategy::new()
            .with_sticky_weight(0.7)
            .with_load_weight(0.5); // Sum = 1.2

        let result = strategy.validate();
        assert!(result.is_err());
    }

    #[test]
    fn test_sticky_validate_negative_weight() {
        let strategy = StickyStrategy::new()
            .with_sticky_weight(-0.5)
            .with_load_weight(1.5);

        let result = strategy.validate();
        assert!(result.is_err());
    }

    #[test]
    fn test_sticky_weight_gt_1() {
        let strategy = StickyStrategy::new()
            .with_sticky_weight(1.5)
            .with_load_weight(-0.5);

        let result = strategy.validate();
        assert!(result.is_err());
    }

    #[test]
    fn test_sticky_get_previous_worker_from_custom() {
        let strategy = StickyStrategy::new();
        let mut ctx = DistributionContext::new();
        ctx.custom
            .insert("previous_worker".to_string(), "worker-a".to_string());

        let previous = strategy.get_previous_worker("bead-1", &ctx);
        assert_eq!(previous, Some("worker-a".to_string()));
    }

    #[test]
    fn test_sticky_get_previous_worker_from_metadata() {
        let strategy = StickyStrategy::new();
        let ctx = DistributionContext::new().with_bead(
            BeadMetadata::new("bead-1").with_preferred_agents(vec!["worker-b".to_string()]),
        );

        let previous = strategy.get_previous_worker("bead-1", &ctx);
        assert_eq!(previous, Some("worker-b".to_string()));
    }

    #[test]
    fn test_sticky_get_previous_worker_none() {
        let strategy = StickyStrategy::new();
        let ctx = DistributionContext::new();

        let previous = strategy.get_previous_worker("bead-1", &ctx);
        assert!(previous.is_none());
    }

    #[test]
    fn test_sticky_is_worker_idle_true() {
        let strategy = StickyStrategy::new();
        let ctx = create_context_with_agents(vec![("worker-a", 0.0)]);

        let idle = strategy.is_worker_idle("worker-a", &ctx);
        assert_eq!(idle, Some(true));
    }

    #[test]
    fn test_sticky_is_worker_idle_false() {
        let strategy = StickyStrategy::new();
        let ctx = create_context_with_agents(vec![("worker-a", 0.8)]);

        let idle = strategy.is_worker_idle("worker-a", &ctx);
        assert_eq!(idle, Some(false));
    }

    #[test]
    fn test_sticky_is_worker_idle_none() {
        let strategy = StickyStrategy::new();
        let ctx = DistributionContext::new();

        let idle = strategy.is_worker_idle("worker-a", &ctx);
        assert_eq!(idle, None);
    }

    #[test]
    fn test_sticky_idle_score_completely_idle() {
        let strategy = StickyStrategy::new();
        let ctx = create_context_with_agents(vec![("worker-a", 0.0)]);

        let score = strategy.idle_score("worker-a", &ctx);
        assert!((score - 1.0).abs() < f64::EPSILON);
    }

    #[test]
    fn test_sticky_idle_score_half_loaded() {
        let strategy = StickyStrategy::new();
        let ctx = create_context_with_agents(vec![("worker-a", 0.5)]);

        let score = strategy.idle_score("worker-a", &ctx);
        assert!((score - 0.5).abs() < f64::EPSILON);
    }

    #[test]
    fn test_sticky_idle_score_completely_loaded() {
        let strategy = StickyStrategy::new();
        let ctx = create_context_with_agents(vec![("worker-a", 1.0)]);

        let score = strategy.idle_score("worker-a", &ctx);
        assert!(score.abs() < f64::EPSILON);
    }

    #[test]
    fn test_sticky_select_least_loaded_single() {
        let strategy = StickyStrategy::new();
        let ctx = create_context_with_agents(vec![("worker-a", 0.5)]);

        let agents = vec!["worker-a".to_string()];
        let result = strategy.select_least_loaded(&agents, &ctx);

        assert_eq!(result, Some("worker-a".to_string()));
    }

    #[test]
    fn test_sticky_select_least_loaded_multiple() {
        let strategy = StickyStrategy::new();
        let ctx = create_context_with_agents(vec![
            ("worker-a", 0.9),
            ("worker-b", 0.1),
            ("worker-c", 0.5),
        ]);

        let agents = vec![
            "worker-a".to_string(),
            "worker-b".to_string(),
            "worker-c".to_string(),
        ];
        let result = strategy.select_least_loaded(&agents, &ctx);

        assert_eq!(result, Some("worker-b".to_string()));
    }

    #[test]
    fn test_sticky_select_least_loaded_empty() {
        let strategy = StickyStrategy::new();
        let ctx = DistributionContext::new();

        let agents: Vec<String> = vec![];
        let result = strategy.select_least_loaded(&agents, &ctx);

        assert!(result.is_none());
    }

    #[test]
    fn test_sticky_combined_score_previous_worker() {
        let strategy = StickyStrategy::new()
            .with_sticky_weight(0.7)
            .with_load_weight(0.3);
        let ctx = create_context_with_agents(vec![("worker-a", 0.2)]);

        let score = strategy.combined_score("worker-a", true, &ctx);
        let expected = (1.0 * 0.7) + (0.8 * 0.3); // sticky=1.0, idle=0.8
        assert!((score - expected).abs() < f64::EPSILON);
    }

    #[test]
    fn test_sticky_combined_score_new_worker() {
        let strategy = StickyStrategy::new()
            .with_sticky_weight(0.7)
            .with_load_weight(0.3);
        let ctx = create_context_with_agents(vec![("worker-b", 0.3)]);

        let score = strategy.combined_score("worker-b", false, &ctx);
        let expected = (0.0 * 0.7) + (0.7 * 0.3); // sticky=0.0, idle=0.7
        assert!((score - expected).abs() < f64::EPSILON);
    }

    #[test]
    fn test_sticky_custom_weights() {
        let strategy = StickyStrategy::new()
            .with_sticky_weight(0.5)
            .with_load_weight(0.5);

        assert!((strategy.sticky_weight - 0.5).abs() < f64::EPSILON);
        assert!((strategy.load_weight - 0.5).abs() < f64::EPSILON);
        assert!(strategy.validate().is_ok());
    }

    #[test]
    fn test_sticky_default_weights() {
        let strategy = StickyStrategy::new();
        assert!((strategy.sticky_weight - 0.7).abs() < f64::EPSILON);
        assert!((strategy.load_weight - 0.3).abs() < f64::EPSILON);
    }

    #[test]
    fn test_sticky_hit_rate_scenario() {
        // Simulate 100 assignments with idle previous worker
        let strategy = StickyStrategy::new();
        let mut sticky_hits = 0;
        let mut total = 0;

        for i in 0..100 {
            let ctx = create_context_with_agents(vec![
                ("worker-a", 0.0), // Always idle
                ("worker-b", 0.5),
            ]);
            let ctx = set_previous_worker(ctx, "worker-a");

            let agents = vec!["worker-a".to_string(), "worker-b".to_string()];
            let result = strategy.select_agent(&format!("bead-{}", i), &agents, &ctx);

            total += 1;
            if result == Some("worker-a".to_string()) {
                sticky_hits += 1;
            }
        }

        let hit_rate = sticky_hits as f64 / total as f64;
        assert!(
            hit_rate > 0.8,
            "Hit rate {}% should be >80%",
            hit_rate * 100.0
        );
    }
}
