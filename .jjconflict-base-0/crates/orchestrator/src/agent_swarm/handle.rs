//! Agent handle for managing individual agents.

use std::time::Duration;

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

/// State of an agent in the pool.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AgentState {
    /// Agent is idle and ready for work.
    #[default]
    Idle,
    /// Agent is currently working on a bead.
    Working,
    /// Agent failed health check.
    Unhealthy,
    /// Agent is shutting down.
    ShuttingDown,
    /// Agent has terminated.
    Terminated,
}

impl AgentState {
    /// Check if agent is available for work.
    #[must_use]
    pub const fn is_available(&self) -> bool {
        matches!(self, Self::Idle)
    }

    /// Check if agent is active (not terminated or shutting down).
    #[must_use]
    pub const fn is_active(&self) -> bool {
        matches!(self, Self::Idle | Self::Working | Self::Unhealthy)
    }

    /// Check if agent can accept new work.
    #[must_use]
    pub const fn can_accept_work(&self) -> bool {
        matches!(self, Self::Idle)
    }
}

impl std::fmt::Display for AgentState {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Idle => write!(f, "idle"),
            Self::Working => write!(f, "working"),
            Self::Unhealthy => write!(f, "unhealthy"),
            Self::ShuttingDown => write!(f, "shutting_down"),
            Self::Terminated => write!(f, "terminated"),
        }
    }
}

/// Handle to an agent in the pool.
#[derive(Debug, Clone)]
pub struct AgentHandle {
    /// Unique agent identifier.
    id: String,
    /// Current state.
    state: AgentState,
    /// Current bead being worked on.
    current_bead: Option<String>,
    /// Last heartbeat timestamp.
    last_heartbeat: DateTime<Utc>,
    /// When the agent registered.
    registered_at: DateTime<Utc>,
    /// Agent capabilities.
    capabilities: Vec<String>,
    /// Consecutive health check failures.
    health_failures: u32,
    /// Maximum allowed health failures before marking unhealthy.
    max_health_failures: u32,
}

impl AgentHandle {
    /// Create a new agent handle.
    #[must_use]
    pub fn new(id: impl Into<String>) -> Self {
        let now = Utc::now();
        Self {
            id: id.into(),
            state: AgentState::Idle,
            current_bead: None,
            last_heartbeat: now,
            registered_at: now,
            capabilities: Vec::new(),
            health_failures: 0,
            max_health_failures: 3,
        }
    }

    /// Create an agent handle with capabilities.
    #[must_use]
    pub fn with_capabilities(mut self, capabilities: Vec<String>) -> Self {
        self.capabilities = capabilities;
        self
    }

    /// Set max health failures threshold.
    #[must_use]
    pub const fn with_max_health_failures(mut self, max: u32) -> Self {
        self.max_health_failures = max;
        self
    }

    /// Get the agent ID.
    #[must_use]
    pub fn id(&self) -> &str {
        &self.id
    }

    /// Get the current state.
    #[must_use]
    pub const fn state(&self) -> AgentState {
        self.state
    }

    /// Get the current bead being worked on.
    #[must_use]
    pub fn current_bead(&self) -> Option<&str> {
        self.current_bead.as_deref()
    }

    /// Get the last heartbeat timestamp.
    #[must_use]
    pub const fn last_heartbeat(&self) -> DateTime<Utc> {
        self.last_heartbeat
    }

    /// Get agent capabilities.
    #[must_use]
    pub fn capabilities(&self) -> &[String] {
        &self.capabilities
    }

    /// Check if agent is available for work.
    #[must_use]
    pub const fn is_available(&self) -> bool {
        self.state.is_available()
    }

    /// Check if agent has a specific capability.
    #[must_use]
    pub fn has_capability(&self, capability: &str) -> bool {
        self.capabilities.iter().any(|c| c == capability)
    }

    /// Record a heartbeat.
    pub fn record_heartbeat(&mut self) {
        self.last_heartbeat = Utc::now();
        self.health_failures = 0;

        // If agent was unhealthy, return to previous state
        if self.state == AgentState::Unhealthy {
            self.state = if self.current_bead.is_some() {
                AgentState::Working
            } else {
                AgentState::Idle
            };
        }
    }

    /// Assign a bead to this agent.
    ///
    /// Returns `true` if assignment succeeded, `false` if agent is unavailable.
    pub fn assign_bead(&mut self, bead_id: impl Into<String>) -> bool {
        if !self.state.can_accept_work() {
            return false;
        }

        self.current_bead = Some(bead_id.into());
        self.state = AgentState::Working;
        true
    }

    /// Complete the current bead.
    pub fn complete_bead(&mut self) {
        self.current_bead = None;
        if self.state == AgentState::Working {
            self.state = AgentState::Idle;
        }
    }

    /// Release the current bead without completing it.
    pub fn release_bead(&mut self) {
        self.current_bead = None;
        if self.state == AgentState::Working {
            self.state = AgentState::Idle;
        }
    }

    /// Record a health check failure.
    ///
    /// Returns `true` if agent should be marked unhealthy.
    pub fn record_health_failure(&mut self) -> bool {
        self.health_failures = self.health_failures.saturating_add(1);

        if self.health_failures >= self.max_health_failures {
            self.state = AgentState::Unhealthy;
            return true;
        }

        false
    }

    /// Check if heartbeat has timed out.
    #[must_use]
    pub fn is_heartbeat_timeout(&self, timeout: Duration) -> bool {
        let elapsed = Utc::now()
            .signed_duration_since(self.last_heartbeat)
            .to_std()
            .unwrap_or(Duration::ZERO);

        elapsed > timeout
    }

    /// Get duration since last heartbeat.
    #[must_use]
    pub fn time_since_heartbeat(&self) -> Duration {
        Utc::now()
            .signed_duration_since(self.last_heartbeat)
            .to_std()
            .unwrap_or(Duration::ZERO)
    }

    /// Mark agent as shutting down.
    pub fn shutdown(&mut self) {
        self.state = AgentState::ShuttingDown;
    }

    /// Mark agent as terminated.
    pub fn terminate(&mut self) {
        self.state = AgentState::Terminated;
        self.current_bead = None;
    }

    /// Get uptime since registration.
    #[must_use]
    pub fn uptime(&self) -> Duration {
        Utc::now()
            .signed_duration_since(self.registered_at)
            .to_std()
            .unwrap_or(Duration::ZERO)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_agent_handle_new() {
        let handle = AgentHandle::new("agent-1");
        assert_eq!(handle.id(), "agent-1");
        assert_eq!(handle.state(), AgentState::Idle);
        assert!(handle.is_available());
        assert!(handle.current_bead().is_none());
    }

    #[test]
    fn test_agent_state_transitions() {
        assert!(AgentState::Idle.is_available());
        assert!(!AgentState::Working.is_available());
        assert!(!AgentState::Unhealthy.is_available());

        assert!(AgentState::Idle.is_active());
        assert!(AgentState::Working.is_active());
        assert!(!AgentState::Terminated.is_active());
    }

    #[test]
    fn test_assign_bead() {
        let mut handle = AgentHandle::new("agent-1");

        assert!(handle.assign_bead("bead-1"));
        assert_eq!(handle.state(), AgentState::Working);
        assert_eq!(handle.current_bead(), Some("bead-1"));

        // Can't assign when already working
        assert!(!handle.assign_bead("bead-2"));
    }

    #[test]
    fn test_complete_bead() {
        let mut handle = AgentHandle::new("agent-1");
        handle.assign_bead("bead-1");

        handle.complete_bead();

        assert_eq!(handle.state(), AgentState::Idle);
        assert!(handle.current_bead().is_none());
    }

    #[test]
    fn test_heartbeat() {
        let mut handle = AgentHandle::new("agent-1");
        let initial = handle.last_heartbeat();

        std::thread::sleep(std::time::Duration::from_millis(10));
        handle.record_heartbeat();

        assert!(handle.last_heartbeat() > initial);
    }

    #[test]
    fn test_health_failures() {
        let mut handle = AgentHandle::new("agent-1").with_max_health_failures(2);

        assert!(!handle.record_health_failure()); // 1st failure
        assert!(handle.record_health_failure()); // 2nd failure -> unhealthy

        assert_eq!(handle.state(), AgentState::Unhealthy);
    }

    #[test]
    fn test_heartbeat_recovers_from_unhealthy() {
        let mut handle = AgentHandle::new("agent-1").with_max_health_failures(1);

        handle.record_health_failure();
        assert_eq!(handle.state(), AgentState::Unhealthy);

        handle.record_heartbeat();
        assert_eq!(handle.state(), AgentState::Idle);
    }

    #[test]
    fn test_capabilities() {
        let handle = AgentHandle::new("agent-1")
            .with_capabilities(vec!["rust".to_string(), "python".to_string()]);

        assert!(handle.has_capability("rust"));
        assert!(handle.has_capability("python"));
        assert!(!handle.has_capability("java"));
    }

    #[test]
    fn test_shutdown() {
        let mut handle = AgentHandle::new("agent-1");

        handle.shutdown();
        assert_eq!(handle.state(), AgentState::ShuttingDown);
        assert!(!handle.state().can_accept_work());

        handle.terminate();
        assert_eq!(handle.state(), AgentState::Terminated);
    }
}
