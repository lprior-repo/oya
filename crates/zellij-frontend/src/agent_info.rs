//! Agent info struct for zellij-frontend UI display
//!
//! Provides immutable data structures for displaying agent information
//! in the zellij terminal UI.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use thiserror::Error;

/// Agent state for UI display
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum AgentState {
    Idle,
    Working,
    Unhealthy,
    ShuttingDown,
    Terminated,
}

impl AgentState {
    #[must_use]
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Idle => "idle",
            Self::Working => "working",
            Self::Unhealthy => "unhealthy",
            Self::ShuttingDown => "shutting_down",
            Self::Terminated => "terminated",
        }
    }

    #[must_use]
    pub fn display_color(self) -> &'static str {
        match self {
            Self::Idle => "green",
            Self::Working => "blue",
            Self::Unhealthy => "red",
            Self::ShuttingDown => "yellow",
            Self::Terminated => "gray",
        }
    }

    #[must_use]
    pub fn is_terminal(self) -> bool {
        matches!(self, Self::ShuttingDown | Self::Terminated)
    }

    #[must_use]
    pub fn can_accept_work(self) -> bool {
        matches!(self, Self::Idle)
    }
}

impl std::fmt::Display for AgentState {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

/// Agent capability for UI display
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AgentCapability {
    pub id: String,
    pub description: String,
    pub version: String,
}

impl AgentCapability {
    pub fn new(id: String, description: String, version: String) -> Result<Self, AgentInfoError> {
        if id.is_empty() {
            return Err(AgentInfoError::EmptyCapabilityId);
        }
        if description.is_empty() {
            return Err(AgentInfoError::EmptyCapabilityDescription);
        }
        Ok(Self {
            id,
            description,
            version,
        })
    }
}

/// Agent info for UI display
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentInfo {
    pub id: String,
    pub state: AgentState,
    pub current_bead: Option<String>,
    pub capabilities: Vec<AgentCapability>,
    pub uptime_secs: u64,
    pub health_score: f64,
    pub beads_completed: u64,
    pub last_heartbeat: DateTime<Utc>,
    pub metadata: HashMap<String, String>,
}

impl AgentInfo {
    pub fn new(
        id: String,
        capabilities: Vec<AgentCapability>,
        health_score: f64,
    ) -> Result<Self, AgentInfoError> {
        if id.is_empty() {
            return Err(AgentInfoError::EmptyAgentId);
        }
        if capabilities.is_empty() {
            return Err(AgentInfoError::EmptyCapabilities);
        }
        if !(0.0..=1.0).contains(&health_score) {
            return Err(AgentInfoError::InvalidHealthScore);
        }
        Ok(Self {
            id,
            state: AgentState::Idle,
            current_bead: None,
            capabilities,
            uptime_secs: 0,
            health_score,
            beads_completed: 0,
            last_heartbeat: Utc::now(),
            metadata: HashMap::new(),
        })
    }

    pub fn with_state(mut self, state: AgentState) -> Self {
        self.state = state;
        self
    }

    pub fn with_current_bead(mut self, bead: Option<String>) -> Self {
        self.current_bead = bead;
        self
    }

    pub fn with_uptime(mut self, secs: u64) -> Self {
        self.uptime_secs = secs;
        self
    }

    pub fn with_beads_completed(mut self, count: u64) -> Self {
        self.beads_completed = count;
        self
    }

    pub fn add_metadata(&mut self, key: String, value: String) -> Result<(), AgentInfoError> {
        if key.is_empty() {
            return Err(AgentInfoError::EmptyMetadataKey);
        }
        if value.is_empty() {
            return Err(AgentInfoError::EmptyMetadataValue);
        }
        self.metadata.insert(key, value);
        Ok(())
    }

    #[must_use]
    pub fn format_uptime(&self) -> String {
        let hours = self.uptime_secs / 3600;
        let minutes = (self.uptime_secs % 3600) / 60;
        let seconds = self.uptime_secs % 60;
        if hours > 0 {
            format!("{}h {}m {}s", hours, minutes, seconds)
        } else if minutes > 0 {
            format!("{}m {}s", minutes, seconds)
        } else {
            format!("{}s", seconds)
        }
    }

    #[must_use]
    pub fn format_health(&self) -> String {
        format!("{:.0}%", self.health_score * 100.0)
    }

    #[must_use]
    pub fn summary(&self) -> AgentSummary {
        AgentSummary {
            id: self.id.clone(),
            state: self.state,
            current_bead: self.current_bead.clone(),
            health_score: self.health_score,
        }
    }
}

/// Summary view for compact display
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AgentSummary {
    pub id: String,
    pub state: AgentState,
    pub current_bead: Option<String>,
    pub health_score: f64,
}

#[derive(Debug, Error, Clone, PartialEq, Eq)]
pub enum AgentInfoError {
    #[error("agent ID cannot be empty")]
    EmptyAgentId,
    #[error("capabilities list cannot be empty")]
    EmptyCapabilities,
    #[error("capability ID cannot be empty")]
    EmptyCapabilityId,
    #[error("capability description cannot be empty")]
    EmptyCapabilityDescription,
    #[error("health score must be between 0.0 and 1.0")]
    InvalidHealthScore,
    #[error("metadata key cannot be empty")]
    EmptyMetadataKey,
    #[error("metadata value cannot be empty")]
    EmptyMetadataValue,
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_capability() -> Result<AgentCapability, AgentInfoError> {
        AgentCapability::new(
            "test-cap".to_string(),
            "Test capability".to_string(),
            "1.0".to_string(),
        )
    }

    #[test]
    fn test_agent_capability_new() -> Result<(), AgentInfoError> {
        let cap = AgentCapability::new(
            "code-gen".to_string(),
            "Generates code".to_string(),
            "1.0".to_string(),
        )?;
        assert_eq!(cap.id, "code-gen");
        assert_eq!(cap.description, "Generates code");
        assert_eq!(cap.version, "1.0");
        Ok(())
    }

    #[test]
    fn test_agent_capability_empty_id() {
        let result = AgentCapability::new(String::new(), "desc".to_string(), "1.0".to_string());
        assert!(matches!(result, Err(AgentInfoError::EmptyCapabilityId)));
    }

    #[test]
    fn test_agent_capability_empty_description() {
        let result = AgentCapability::new("id".to_string(), String::new(), "1.0".to_string());
        assert!(matches!(
            result,
            Err(AgentInfoError::EmptyCapabilityDescription)
        ));
    }

    #[test]
    fn test_agent_state_as_str() {
        assert_eq!(AgentState::Idle.as_str(), "idle");
        assert_eq!(AgentState::Working.as_str(), "working");
        assert_eq!(AgentState::Unhealthy.as_str(), "unhealthy");
    }

    #[test]
    fn test_agent_state_is_terminal() {
        assert!(!AgentState::Idle.is_terminal());
        assert!(!AgentState::Working.is_terminal());
        assert!(AgentState::ShuttingDown.is_terminal());
        assert!(AgentState::Terminated.is_terminal());
    }

    #[test]
    fn test_agent_state_can_accept_work() {
        assert!(AgentState::Idle.can_accept_work());
        assert!(!AgentState::Working.can_accept_work());
        assert!(!AgentState::Unhealthy.can_accept_work());
    }

    #[test]
    fn test_agent_info_new() -> Result<(), AgentInfoError> {
        let agent = AgentInfo::new("agent-001".to_string(), vec![test_capability()?], 0.95)?;
        assert_eq!(agent.id, "agent-001");
        assert_eq!(agent.state, AgentState::Idle);
        assert!(agent.current_bead.is_none());
        assert_eq!(agent.capabilities.len(), 1);
        Ok(())
    }

    #[test]
    fn test_agent_info_empty_id() -> Result<(), AgentInfoError> {
        let result = AgentInfo::new(String::new(), vec![test_capability()?], 0.95);
        assert!(matches!(result, Err(AgentInfoError::EmptyAgentId)));
        Ok(())
    }

    #[test]
    fn test_agent_info_empty_capabilities() {
        let result = AgentInfo::new("agent-001".to_string(), vec![], 0.95);
        assert!(matches!(result, Err(AgentInfoError::EmptyCapabilities)));
    }

    #[test]
    fn test_agent_info_invalid_health_score() -> Result<(), AgentInfoError> {
        let result = AgentInfo::new("agent-001".to_string(), vec![test_capability()?], 1.5);
        assert!(matches!(result, Err(AgentInfoError::InvalidHealthScore)));

        let result = AgentInfo::new("agent-001".to_string(), vec![test_capability()?], -0.1);
        assert!(matches!(result, Err(AgentInfoError::InvalidHealthScore)));
        Ok(())
    }

    #[test]
    fn test_agent_info_with_state() -> Result<(), AgentInfoError> {
        let agent = AgentInfo::new("agent-001".to_string(), vec![test_capability()?], 0.95)?
            .with_state(AgentState::Working);
        assert_eq!(agent.state, AgentState::Working);
        Ok(())
    }

    #[test]
    fn test_agent_info_with_current_bead() -> Result<(), AgentInfoError> {
        let agent = AgentInfo::new("agent-001".to_string(), vec![test_capability()?], 0.95)?
            .with_current_bead(Some("bead-123".to_string()));
        assert_eq!(agent.current_bead, Some("bead-123".to_string()));
        Ok(())
    }

    #[test]
    fn test_agent_info_with_uptime() -> Result<(), AgentInfoError> {
        let agent = AgentInfo::new("agent-001".to_string(), vec![test_capability()?], 0.95)?
            .with_uptime(3665);
        assert_eq!(agent.uptime_secs, 3665);
        Ok(())
    }

    #[test]
    fn test_agent_info_format_uptime() -> Result<(), AgentInfoError> {
        let agent = AgentInfo::new("agent-001".to_string(), vec![test_capability()?], 0.95)?
            .with_uptime(3665);
        assert_eq!(agent.format_uptime(), "1h 1m 5s");

        let agent = AgentInfo::new("agent-001".to_string(), vec![test_capability()?], 0.95)?
            .with_uptime(125);
        assert_eq!(agent.format_uptime(), "2m 5s");

        let agent = AgentInfo::new("agent-001".to_string(), vec![test_capability()?], 0.95)?
            .with_uptime(30);
        assert_eq!(agent.format_uptime(), "30s");
        Ok(())
    }

    #[test]
    fn test_agent_info_format_health() -> Result<(), AgentInfoError> {
        let agent = AgentInfo::new("agent-001".to_string(), vec![test_capability()?], 0.95)?;
        assert_eq!(agent.format_health(), "95%");

        let agent = AgentInfo::new("agent-001".to_string(), vec![test_capability()?], 1.0)?;
        assert_eq!(agent.format_health(), "100%");

        let agent = AgentInfo::new("agent-001".to_string(), vec![test_capability()?], 0.0)?;
        assert_eq!(agent.format_health(), "0%");
        Ok(())
    }

    #[test]
    fn test_agent_info_add_metadata() -> Result<(), AgentInfoError> {
        let mut agent = AgentInfo::new("agent-001".to_string(), vec![test_capability()?], 0.95)?;

        let result = agent.add_metadata("env".to_string(), "production".to_string());
        assert!(result.is_ok());
        assert_eq!(agent.metadata.get("env"), Some(&"production".to_string()));
        Ok(())
    }

    #[test]
    fn test_agent_info_add_metadata_empty_key() -> Result<(), AgentInfoError> {
        let mut agent = AgentInfo::new("agent-001".to_string(), vec![test_capability()?], 0.95)?;

        let result = agent.add_metadata(String::new(), "value".to_string());
        assert!(matches!(result, Err(AgentInfoError::EmptyMetadataKey)));
        Ok(())
    }

    #[test]
    fn test_agent_info_add_metadata_empty_value() -> Result<(), AgentInfoError> {
        let mut agent = AgentInfo::new("agent-001".to_string(), vec![test_capability()?], 0.95)?;

        let result = agent.add_metadata("key".to_string(), String::new());
        assert!(matches!(result, Err(AgentInfoError::EmptyMetadataValue)));
        Ok(())
    }

    #[test]
    fn test_agent_info_summary() -> Result<(), AgentInfoError> {
        let agent = AgentInfo::new("agent-001".to_string(), vec![test_capability()?], 0.95)?
            .with_state(AgentState::Working)
            .with_current_bead(Some("bead-123".to_string()));

        let summary = agent.summary();
        assert_eq!(summary.id, "agent-001");
        assert_eq!(summary.state, AgentState::Working);
        assert_eq!(summary.current_bead, Some("bead-123".to_string()));
        assert!((summary.health_score - 0.95).abs() < f64::EPSILON);
        Ok(())
    }

    #[test]
    fn test_agent_state_display() {
        assert_eq!(format!("{}", AgentState::Idle), "idle");
        assert_eq!(format!("{}", AgentState::Working), "working");
        assert_eq!(format!("{}", AgentState::Unhealthy), "unhealthy");
    }

    #[test]
    fn test_agent_state_display_color() {
        assert_eq!(AgentState::Idle.display_color(), "green");
        assert_eq!(AgentState::Working.display_color(), "blue");
        assert_eq!(AgentState::Unhealthy.display_color(), "red");
    }
}
