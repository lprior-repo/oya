//! Distribution strategy trait and context.

use std::collections::HashMap;

use super::error::DistributionResult;

/// Context provided to distribution strategies for decision-making.
#[derive(Debug, Clone, Default)]
pub struct DistributionContext {
    /// Bead metadata (bead_id -> metadata).
    pub bead_metadata: HashMap<String, BeadMetadata>,
    /// Agent metadata (agent_id -> metadata).
    pub agent_metadata: HashMap<String, AgentMetadata>,
    /// Current workflow ID being processed.
    pub workflow_id: Option<String>,
    /// Custom context data.
    pub custom: HashMap<String, String>,
}

/// Metadata about a bead for distribution decisions.
#[derive(Debug, Clone, Default)]
pub struct BeadMetadata {
    /// Bead ID.
    pub id: String,
    /// Priority level (higher = more urgent).
    pub priority: i32,
    /// Required capabilities.
    pub required_capabilities: Vec<String>,
    /// Preferred agent IDs.
    pub preferred_agents: Vec<String>,
    /// When the bead became ready.
    pub ready_at: Option<chrono::DateTime<chrono::Utc>>,
    /// Number of times this bead has been retried.
    pub retry_count: u32,
    /// Estimated execution time in seconds.
    pub estimated_duration_secs: Option<u64>,
    /// Custom metadata.
    pub custom: HashMap<String, String>,
}

/// Metadata about an agent for distribution decisions.
#[derive(Debug, Clone, Default)]
pub struct AgentMetadata {
    /// Agent ID.
    pub id: String,
    /// Agent capabilities.
    pub capabilities: Vec<String>,
    /// Current load (0.0 - 1.0).
    pub load: f64,
    /// Number of beads completed.
    pub beads_completed: u64,
    /// Average execution time in seconds.
    pub avg_execution_secs: Option<f64>,
    /// Custom metadata.
    pub custom: HashMap<String, String>,
}

impl DistributionContext {
    /// Create a new empty context.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Add bead metadata.
    #[must_use]
    pub fn with_bead(mut self, metadata: BeadMetadata) -> Self {
        self.bead_metadata.insert(metadata.id.clone(), metadata);
        self
    }

    /// Add agent metadata.
    #[must_use]
    pub fn with_agent(mut self, metadata: AgentMetadata) -> Self {
        self.agent_metadata.insert(metadata.id.clone(), metadata);
        self
    }

    /// Set workflow ID.
    #[must_use]
    pub fn with_workflow(mut self, workflow_id: impl Into<String>) -> Self {
        self.workflow_id = Some(workflow_id.into());
        self
    }

    /// Get bead metadata by ID.
    #[must_use]
    pub fn get_bead(&self, bead_id: &str) -> Option<&BeadMetadata> {
        self.bead_metadata.get(bead_id)
    }

    /// Get agent metadata by ID.
    #[must_use]
    pub fn get_agent(&self, agent_id: &str) -> Option<&AgentMetadata> {
        self.agent_metadata.get(agent_id)
    }
}

impl BeadMetadata {
    /// Create new bead metadata with just an ID.
    #[must_use]
    pub fn new(id: impl Into<String>) -> Self {
        Self {
            id: id.into(),
            ..Default::default()
        }
    }

    /// Set priority.
    #[must_use]
    pub const fn with_priority(mut self, priority: i32) -> Self {
        self.priority = priority;
        self
    }

    /// Add required capability.
    #[must_use]
    pub fn with_capability(mut self, capability: impl Into<String>) -> Self {
        self.required_capabilities.push(capability.into());
        self
    }

    /// Set preferred agents.
    #[must_use]
    pub fn with_preferred_agents(mut self, agents: Vec<String>) -> Self {
        self.preferred_agents = agents;
        self
    }

    /// Set retry count.
    #[must_use]
    pub const fn with_retry_count(mut self, count: u32) -> Self {
        self.retry_count = count;
        self
    }
}

impl AgentMetadata {
    /// Create new agent metadata with just an ID.
    #[must_use]
    pub fn new(id: impl Into<String>) -> Self {
        Self {
            id: id.into(),
            ..Default::default()
        }
    }

    /// Add capability.
    #[must_use]
    pub fn with_capability(mut self, capability: impl Into<String>) -> Self {
        self.capabilities.push(capability.into());
        self
    }

    /// Set capabilities.
    #[must_use]
    pub fn with_capabilities(mut self, capabilities: Vec<String>) -> Self {
        self.capabilities = capabilities;
        self
    }

    /// Set load.
    #[must_use]
    pub fn with_load(mut self, load: f64) -> Self {
        self.load = load.clamp(0.0, 1.0);
        self
    }

    /// Set beads completed count.
    #[must_use]
    pub const fn with_beads_completed(mut self, count: u64) -> Self {
        self.beads_completed = count;
        self
    }

    /// Check if agent has a capability.
    #[must_use]
    pub fn has_capability(&self, capability: &str) -> bool {
        self.capabilities.iter().any(|c| c == capability)
    }
}

/// Trait for task distribution strategies.
///
/// Strategies determine which bead to execute next and which agent
/// should execute it.
pub trait DistributionStrategy: Send + Sync {
    /// Select the next bead to execute from ready beads.
    ///
    /// Returns the bead ID to execute, or None if no suitable bead found.
    fn select_bead(&self, ready_beads: &[String], ctx: &DistributionContext) -> Option<String>;

    /// Select an agent to execute a bead.
    ///
    /// Returns the agent ID, or None if no suitable agent found.
    fn select_agent(
        &self,
        bead_id: &str,
        agents: &[String],
        ctx: &DistributionContext,
    ) -> Option<String>;

    /// Get the strategy name.
    fn name(&self) -> &'static str;

    /// Validate strategy configuration.
    fn validate(&self) -> DistributionResult<()> {
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_distribution_context_new() {
        let ctx = DistributionContext::new();
        assert!(ctx.bead_metadata.is_empty());
        assert!(ctx.agent_metadata.is_empty());
        assert!(ctx.workflow_id.is_none());
    }

    #[test]
    fn test_distribution_context_with_bead() {
        let ctx =
            DistributionContext::new().with_bead(BeadMetadata::new("bead-1").with_priority(10));

        assert!(ctx.get_bead("bead-1").is_some());
        assert_eq!(ctx.get_bead("bead-1").map(|b| b.priority), Some(10));
    }

    #[test]
    fn test_distribution_context_with_agent() {
        let ctx =
            DistributionContext::new().with_agent(AgentMetadata::new("agent-1").with_load(0.5));

        assert!(ctx.get_agent("agent-1").is_some());
        let agent = ctx.get_agent("agent-1");
        assert!((agent.map(|a| a.load).unwrap_or(0.0) - 0.5).abs() < f64::EPSILON);
    }

    #[test]
    fn test_bead_metadata_builder() {
        let bead = BeadMetadata::new("b1")
            .with_priority(5)
            .with_capability("rust")
            .with_capability("python")
            .with_retry_count(2);

        assert_eq!(bead.id, "b1");
        assert_eq!(bead.priority, 5);
        assert_eq!(bead.required_capabilities.len(), 2);
        assert_eq!(bead.retry_count, 2);
    }

    #[test]
    fn test_agent_metadata_builder() {
        let agent = AgentMetadata::new("a1")
            .with_capability("rust")
            .with_load(0.75)
            .with_beads_completed(100);

        assert_eq!(agent.id, "a1");
        assert!(agent.has_capability("rust"));
        assert!(!agent.has_capability("python"));
        assert!((agent.load - 0.75).abs() < f64::EPSILON);
        assert_eq!(agent.beads_completed, 100);
    }

    #[test]
    fn test_agent_load_clamped() {
        let agent = AgentMetadata::new("a1").with_load(1.5);
        assert!((agent.load - 1.0).abs() < f64::EPSILON);

        let agent = AgentMetadata::new("a2").with_load(-0.5);
        assert!(agent.load.abs() < f64::EPSILON);
    }

    #[test]
    fn test_context_workflow_id() {
        let ctx = DistributionContext::new().with_workflow("wf-123");
        assert_eq!(ctx.workflow_id, Some("wf-123".to_string()));
    }
}
