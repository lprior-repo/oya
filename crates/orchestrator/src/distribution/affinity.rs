//! Affinity-based distribution strategy.

use super::error::{DistributionError, DistributionResult};
use super::strategy::{DistributionContext, DistributionStrategy};

/// Affinity mode for agent selection.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum AffinityMode {
    /// Hard affinity - must match, fail if no match.
    Hard,
    /// Soft affinity - prefer match, fall back if no match.
    #[default]
    Soft,
}

/// Affinity-based distribution strategy.
///
/// Selects agents based on capability matching and preferred agent lists.
#[derive(Debug, Clone)]
pub struct AffinityStrategy {
    /// Affinity mode (hard or soft).
    mode: AffinityMode,
    /// Weight for capability matching (0.0 - 1.0).
    capability_weight: f64,
    /// Weight for preferred agent matching (0.0 - 1.0).
    preference_weight: f64,
    /// Weight for load balancing (0.0 - 1.0).
    load_weight: f64,
}

impl Default for AffinityStrategy {
    fn default() -> Self {
        Self {
            mode: AffinityMode::Soft,
            capability_weight: 0.4,
            preference_weight: 0.4,
            load_weight: 0.2,
        }
    }
}

impl AffinityStrategy {
    /// Create a new affinity strategy with default settings.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Set the affinity mode.
    #[must_use]
    pub const fn with_mode(mut self, mode: AffinityMode) -> Self {
        self.mode = mode;
        self
    }

    /// Set the capability weight.
    #[must_use]
    pub fn with_capability_weight(mut self, weight: f64) -> Self {
        self.capability_weight = weight.clamp(0.0, 1.0);
        self
    }

    /// Set the preference weight.
    #[must_use]
    pub fn with_preference_weight(mut self, weight: f64) -> Self {
        self.preference_weight = weight.clamp(0.0, 1.0);
        self
    }

    /// Set the load weight.
    #[must_use]
    pub fn with_load_weight(mut self, weight: f64) -> Self {
        self.load_weight = weight.clamp(0.0, 1.0);
        self
    }

    /// Create a hard affinity strategy.
    #[must_use]
    pub fn hard() -> Self {
        Self::new().with_mode(AffinityMode::Hard)
    }

    /// Create a soft affinity strategy.
    #[must_use]
    pub fn soft() -> Self {
        Self::new().with_mode(AffinityMode::Soft)
    }

    /// Calculate the capability score for an agent.
    fn capability_score(&self, agent_id: &str, bead_id: &str, ctx: &DistributionContext) -> f64 {
        let bead = match ctx.get_bead(bead_id) {
            Some(b) => b,
            None => return 1.0, // No requirements = full score
        };

        if bead.required_capabilities.is_empty() {
            return 1.0;
        }

        let agent = match ctx.get_agent(agent_id) {
            Some(a) => a,
            None => return 0.0, // No metadata = no capabilities
        };

        let matches = bead
            .required_capabilities
            .iter()
            .filter(|cap| agent.has_capability(cap))
            .count();

        matches as f64 / bead.required_capabilities.len() as f64
    }

    /// Calculate the preference score for an agent.
    fn preference_score(&self, agent_id: &str, bead_id: &str, ctx: &DistributionContext) -> f64 {
        let bead = match ctx.get_bead(bead_id) {
            Some(b) => b,
            None => return 0.5, // Neutral if no preferences
        };

        if bead.preferred_agents.is_empty() {
            return 0.5; // Neutral
        }

        if bead.preferred_agents.contains(&agent_id.to_string()) {
            1.0 // Preferred
        } else {
            0.0 // Not preferred
        }
    }

    /// Calculate the load score for an agent (lower load = higher score).
    fn load_score(&self, agent_id: &str, ctx: &DistributionContext) -> f64 {
        let load = ctx
            .get_agent(agent_id)
            .map(|a| a.load)
            .filter(|load| load.is_finite())
            .unwrap_or(0.5);
        1.0 - load
    }

    /// Calculate the total affinity score for an agent.
    fn affinity_score(&self, agent_id: &str, bead_id: &str, ctx: &DistributionContext) -> f64 {
        let cap_score = self.capability_score(agent_id, bead_id, ctx);
        let pref_score = self.preference_score(agent_id, bead_id, ctx);
        let load_score = self.load_score(agent_id, ctx);

        (cap_score * self.capability_weight)
            + (pref_score * self.preference_weight)
            + (load_score * self.load_weight)
    }

    /// Check if an agent fully matches capability requirements.
    fn has_all_capabilities(
        &self,
        agent_id: &str,
        bead_id: &str,
        ctx: &DistributionContext,
    ) -> bool {
        (self.capability_score(agent_id, bead_id, ctx) - 1.0).abs() < f64::EPSILON
    }
}

impl DistributionStrategy for AffinityStrategy {
    fn select_bead(&self, ready_beads: &[String], ctx: &DistributionContext) -> Option<String> {
        if ready_beads.is_empty() {
            return None;
        }

        // Select bead with highest priority (retry count as tiebreaker)
        ready_beads
            .iter()
            .max_by_key(|bead_id| {
                let bead = ctx.get_bead(bead_id);
                let priority = bead.map(|b| b.priority).unwrap_or(0);
                let retry = bead.map(|b| b.retry_count).unwrap_or(0);
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

        // In hard mode, filter to only agents with all required capabilities
        let candidates: Vec<_> = if self.mode == AffinityMode::Hard {
            agents
                .iter()
                .filter(|a| self.has_all_capabilities(a, bead_id, ctx))
                .collect()
        } else {
            agents.iter().collect()
        };

        if candidates.is_empty() {
            // In soft mode, fall back to all agents
            if self.mode == AffinityMode::Soft {
                return agents
                    .iter()
                    .max_by(|a, b| {
                        self.affinity_score(a, bead_id, ctx)
                            .partial_cmp(&self.affinity_score(b, bead_id, ctx))
                            .unwrap_or(std::cmp::Ordering::Equal)
                    })
                    .cloned();
            }
            return None;
        }

        // Select candidate with highest affinity score
        candidates
            .into_iter()
            .max_by(|a, b| {
                self.affinity_score(a, bead_id, ctx)
                    .partial_cmp(&self.affinity_score(b, bead_id, ctx))
                    .unwrap_or(std::cmp::Ordering::Equal)
            })
            .cloned()
    }

    fn name(&self) -> &'static str {
        match self.mode {
            AffinityMode::Soft => "affinity",
            AffinityMode::Hard => "affinity_hard",
        }
    }

    fn validate(&self) -> DistributionResult<()> {
        let total = self.capability_weight + self.preference_weight + self.load_weight;
        if (total - 1.0).abs() > 0.01 {
            return Err(DistributionError::configuration(format!(
                "weights should sum to 1.0, got {}",
                total
            )));
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::distribution::strategy::{AgentMetadata, BeadMetadata};

    #[test]
    fn test_affinity_strategy_name() {
        let strategy = AffinityStrategy::new();
        assert_eq!(strategy.name(), "affinity");
    }

    #[test]
    fn test_affinity_select_bead_empty() {
        let strategy = AffinityStrategy::new();
        let ctx = DistributionContext::new();

        let result = strategy.select_bead(&[], &ctx);
        assert!(result.is_none());
    }

    #[test]
    fn test_affinity_select_bead_by_priority() {
        let strategy = AffinityStrategy::new();
        let ctx = DistributionContext::new()
            .with_bead(BeadMetadata::new("low").with_priority(1))
            .with_bead(BeadMetadata::new("high").with_priority(10))
            .with_bead(BeadMetadata::new("medium").with_priority(5));

        let beads = vec!["low".to_string(), "high".to_string(), "medium".to_string()];
        let result = strategy.select_bead(&beads, &ctx);
        assert_eq!(result, Some("high".to_string()));
    }

    #[test]
    fn test_affinity_select_bead_retry_tiebreaker() {
        let strategy = AffinityStrategy::new();
        let ctx = DistributionContext::new()
            .with_bead(BeadMetadata::new("a").with_priority(5).with_retry_count(0))
            .with_bead(BeadMetadata::new("b").with_priority(5).with_retry_count(3))
            .with_bead(BeadMetadata::new("c").with_priority(5).with_retry_count(1));

        let beads = vec!["a".to_string(), "b".to_string(), "c".to_string()];
        let result = strategy.select_bead(&beads, &ctx);
        assert_eq!(result, Some("b".to_string())); // Highest retry count
    }

    #[test]
    fn test_affinity_select_agent_empty() {
        let strategy = AffinityStrategy::new();
        let ctx = DistributionContext::new();

        let result = strategy.select_agent("bead", &[], &ctx);
        assert!(result.is_none());
    }

    #[test]
    fn test_affinity_soft_mode_capability_matching() {
        let strategy = AffinityStrategy::soft();
        let ctx = DistributionContext::new()
            .with_bead(BeadMetadata::new("bead").with_capability("rust"))
            .with_agent(
                AgentMetadata::new("rust-agent")
                    .with_capability("rust")
                    .with_load(0.5),
            )
            .with_agent(
                AgentMetadata::new("python-agent")
                    .with_capability("python")
                    .with_load(0.1),
            );

        let agents = vec!["rust-agent".to_string(), "python-agent".to_string()];
        let result = strategy.select_agent("bead", &agents, &ctx);
        assert_eq!(result, Some("rust-agent".to_string()));
    }

    #[test]
    fn test_affinity_hard_mode_capability_matching() {
        let strategy = AffinityStrategy::hard();
        let ctx = DistributionContext::new()
            .with_bead(BeadMetadata::new("bead").with_capability("rust"))
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
        let result = strategy.select_agent("bead", &agents, &ctx);
        // Hard mode: only rust-agent matches
        assert_eq!(result, Some("rust-agent".to_string()));
    }

    #[test]
    fn test_affinity_hard_mode_no_match() {
        let strategy = AffinityStrategy::hard();
        let ctx = DistributionContext::new()
            .with_bead(BeadMetadata::new("bead").with_capability("java"))
            .with_agent(AgentMetadata::new("rust-agent").with_capability("rust"))
            .with_agent(AgentMetadata::new("python-agent").with_capability("python"));

        let agents = vec!["rust-agent".to_string(), "python-agent".to_string()];
        let result = strategy.select_agent("bead", &agents, &ctx);
        // Hard mode: no match returns None
        assert!(result.is_none());
    }

    #[test]
    fn test_affinity_soft_mode_fallback() {
        let strategy = AffinityStrategy::soft();
        let ctx = DistributionContext::new()
            .with_bead(BeadMetadata::new("bead").with_capability("java"))
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
        let result = strategy.select_agent("bead", &agents, &ctx);
        // Soft mode: falls back to best available (lower load)
        assert!(result.is_some());
    }

    #[test]
    fn test_affinity_preferred_agents() {
        let strategy = AffinityStrategy::soft()
            .with_capability_weight(0.2)
            .with_preference_weight(0.6)
            .with_load_weight(0.2);

        let ctx = DistributionContext::new()
            .with_bead(
                BeadMetadata::new("bead").with_preferred_agents(vec!["preferred".to_string()]),
            )
            .with_agent(AgentMetadata::new("preferred").with_load(0.9))
            .with_agent(AgentMetadata::new("other").with_load(0.1));

        let agents = vec!["preferred".to_string(), "other".to_string()];
        let result = strategy.select_agent("bead", &agents, &ctx);
        // High preference weight should favor preferred agent
        assert_eq!(result, Some("preferred".to_string()));
    }

    #[test]
    fn test_affinity_load_balancing() {
        let strategy = AffinityStrategy::soft()
            .with_capability_weight(0.0)
            .with_preference_weight(0.0)
            .with_load_weight(1.0);

        let ctx = DistributionContext::new()
            .with_agent(AgentMetadata::new("busy").with_load(0.9))
            .with_agent(AgentMetadata::new("idle").with_load(0.1));

        let agents = vec!["busy".to_string(), "idle".to_string()];
        let result = strategy.select_agent("bead", &agents, &ctx);
        // Pure load balancing should pick idle
        assert_eq!(result, Some("idle".to_string()));
    }

    #[test]
    fn test_affinity_validate_ok() {
        let strategy = AffinityStrategy::new(); // Default weights sum to 1.0
        assert!(strategy.validate().is_ok());
    }

    #[test]
    fn test_affinity_validate_bad_weights() {
        let strategy = AffinityStrategy::new()
            .with_capability_weight(0.5)
            .with_preference_weight(0.5)
            .with_load_weight(0.5); // Sum = 1.5

        let result = strategy.validate();
        assert!(result.is_err());
    }

    #[test]
    fn test_affinity_multiple_capabilities() {
        let strategy = AffinityStrategy::hard();
        let ctx = DistributionContext::new()
            .with_bead(
                BeadMetadata::new("bead")
                    .with_capability("rust")
                    .with_capability("wasm"),
            )
            .with_agent(AgentMetadata::new("partial").with_capability("rust"))
            .with_agent(
                AgentMetadata::new("full")
                    .with_capabilities(vec!["rust".to_string(), "wasm".to_string()]),
            );

        let agents = vec!["partial".to_string(), "full".to_string()];
        let result = strategy.select_agent("bead", &agents, &ctx);
        // Only full agent has all required capabilities
        assert_eq!(result, Some("full".to_string()));
    }

    #[test]
    fn test_affinity_no_metadata() {
        let strategy = AffinityStrategy::soft();
        let ctx = DistributionContext::new(); // No metadata

        let agents = vec!["agent-1".to_string(), "agent-2".to_string()];
        let result = strategy.select_agent("bead", &agents, &ctx);
        // Should still return something
        assert!(result.is_some());
    }

    #[test]
    fn test_affinity_mode_constructors() {
        let hard = AffinityStrategy::hard();
        assert_eq!(hard.mode, AffinityMode::Hard);

        let soft = AffinityStrategy::soft();
        assert_eq!(soft.mode, AffinityMode::Soft);
    }

    #[test]
    fn test_affinity_weight_clamping() {
        let strategy = AffinityStrategy::new()
            .with_capability_weight(1.5)
            .with_preference_weight(-0.5)
            .with_load_weight(0.5);

        assert!((strategy.capability_weight - 1.0).abs() < f64::EPSILON);
        assert!(strategy.preference_weight.abs() < f64::EPSILON);
        assert!((strategy.load_weight - 0.5).abs() < f64::EPSILON);
    }

    #[test]
    fn test_affinity_capability_score_no_requirements() {
        let strategy = AffinityStrategy::new();
        let ctx = DistributionContext::new()
            .with_bead(BeadMetadata::new("bead")) // No requirements
            .with_agent(AgentMetadata::new("agent"));

        let score = strategy.capability_score("agent", "bead", &ctx);
        assert!((score - 1.0).abs() < f64::EPSILON);
    }

    #[test]
    fn test_affinity_capability_score_partial_match() {
        let strategy = AffinityStrategy::new();
        let ctx = DistributionContext::new()
            .with_bead(
                BeadMetadata::new("bead")
                    .with_capability("rust")
                    .with_capability("wasm"),
            )
            .with_agent(AgentMetadata::new("agent").with_capability("rust"));

        let score = strategy.capability_score("agent", "bead", &ctx);
        assert!((score - 0.5).abs() < f64::EPSILON); // 1/2 capabilities
    }

    #[test]
    fn test_affinity_preference_score_neutral() {
        let strategy = AffinityStrategy::new();
        let ctx = DistributionContext::new()
            .with_bead(BeadMetadata::new("bead")) // No preferences
            .with_agent(AgentMetadata::new("agent"));

        let score = strategy.preference_score("agent", "bead", &ctx);
        assert!((score - 0.5).abs() < f64::EPSILON);
    }

    #[test]
    fn test_affinity_preference_score_preferred() {
        let strategy = AffinityStrategy::new();
        let ctx = DistributionContext::new()
            .with_bead(BeadMetadata::new("bead").with_preferred_agents(vec!["agent".to_string()]))
            .with_agent(AgentMetadata::new("agent"));

        let score = strategy.preference_score("agent", "bead", &ctx);
        assert!((score - 1.0).abs() < f64::EPSILON);
    }

    #[test]
    fn test_affinity_preference_score_not_preferred() {
        let strategy = AffinityStrategy::new();
        let ctx = DistributionContext::new()
            .with_bead(BeadMetadata::new("bead").with_preferred_agents(vec!["other".to_string()]))
            .with_agent(AgentMetadata::new("agent"));

        let score = strategy.preference_score("agent", "bead", &ctx);
        assert!(score.abs() < f64::EPSILON);
    }
}
