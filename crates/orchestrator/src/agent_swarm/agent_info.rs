#![deny(clippy::unwrap_used)]
#![deny(clippy::expect_used)]
#![deny(clippy::panic)]
#![warn(clippy::pedantic)]
#![warn(clippy::nursery)]
#![forbid(unsafe_code)]

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Agent state representing the lifecycle phase of an agent.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum AgentState {
    /// Agent is idle and available for work.
    Idle,
    /// Agent is actively processing a bead.
    Working,
    /// Agent health checks are failing.
    Unhealthy,
    /// Agent is shutting down gracefully.
    ShuttingDown,
    /// Agent has terminated.
    Terminated,
}

impl AgentState {
    /// Returns all valid agent states.
    pub fn all_states() -> impl Iterator<Item = Self> {
        [
            Self::Idle,
            Self::Working,
            Self::Unhealthy,
            Self::ShuttingDown,
            Self::Terminated,
        ]
        .into_iter()
    }

    /// Checks if the agent state is terminal.
    pub fn is_terminal(self) -> bool {
        matches!(self, Self::ShuttingDown | Self::Terminated)
    }

    /// Checks if the agent can accept new work.
    pub fn can_accept_work(self) -> bool {
        matches!(self, Self::Idle)
    }
}

/// Agent capabilities advertised by the agent.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentCapability {
    /// Unique capability identifier.
    pub id: String,
    /// Human-readable description.
    pub description: String,
    /// Version of the capability.
    pub version: String,
}

impl AgentCapability {
    /// Creates a new agent capability.
    pub fn new(id: String, description: String, version: String) -> Result<Self, AgentInfoError> {
        if id.is_empty() {
            return Err(AgentInfoError::EmptyIdentifier);
        }

        if description.is_empty() {
            return Err(AgentInfoError::EmptyDescription);
        }

        Ok(Self {
            id,
            description,
            version,
        })
    }
}

/// Agent workload history tracking completed beads.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkloadHistory {
    /// Total beads completed.
    pub beads_completed: u64,
    /// Total operations executed.
    pub operations_executed: u64,
    /// Total execution time in seconds.
    pub total_execution_secs: f64,
    /// Average execution time per operation.
    pub avg_execution_secs: Option<f64>,
    /// History of recent operations.
    pub recent_operations: Vec<OperationRecord>,
}

impl WorkloadHistory {
    /// Creates an empty workload history.
    pub fn new() -> Self {
        Self {
            beads_completed: 0,
            operations_executed: 0,
            total_execution_secs: 0.0,
            avg_execution_secs: None,
            recent_operations: Vec::new(),
        }
    }

    /// Records a completed operation.
    pub fn record_operation(&mut self, duration_secs: f64) -> Result<(), AgentInfoError> {
        if duration_secs < 0.0 {
            return Err(AgentInfoError::NegativeDuration);
        }

        self.operations_executed += 1;
        self.total_execution_secs += duration_secs;

        self.recent_operations.push(OperationRecord {
            timestamp: Utc::now(),
            duration_secs,
        });

        if self.recent_operations.len() > 100 {
            self.recent_operations.remove(0);
        }

        self.avg_execution_secs = Some(self.total_execution_secs / self.operations_executed as f64);
        Ok(())
    }

    /// Records completion of a bead.
    pub fn record_bead_completion(&mut self) -> Result<(), AgentInfoError> {
        self.beads_completed += 1;
        self.record_operation(0.0)
    }
}

/// Single operation record in workload history.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OperationRecord {
    /// Timestamp of operation.
    pub timestamp: DateTime<Utc>,
    /// Duration in seconds.
    pub duration_secs: f64,
}

/// Agent health metrics.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HealthMetrics {
    /// Current health score (0.0 - 1.0).
    pub health_score: f64,
    /// Number of consecutive health failures.
    pub health_failures: u32,
    /// Maximum allowed health failures before marking unhealthy.
    pub max_health_failures: u32,
    /// Last successful health check timestamp.
    pub last_successful_check: Option<DateTime<Utc>>,
    /// Last failed health check timestamp.
    pub last_failed_check: Option<DateTime<Utc>>,
    /// Health check interval in seconds.
    pub check_interval_secs: u64,
}

impl HealthMetrics {
    /// Creates new health metrics.
    pub fn new(max_health_failures: u32, check_interval_secs: u64) -> Result<Self, AgentInfoError> {
        if max_health_failures == 0 {
            return Err(AgentInfoError::ZeroMaxFailures);
        }

        if check_interval_secs == 0 {
            return Err(AgentInfoError::ZeroCheckInterval);
        }

        Ok(Self {
            health_score: 1.0,
            health_failures: 0,
            max_health_failures,
            last_successful_check: Some(Utc::now()),
            last_failed_check: None,
            check_interval_secs,
        })
    }

    /// Records a successful health check.
    pub fn record_success(&mut self) -> Result<(), AgentInfoError> {
        self.health_failures = 0;
        self.health_score = 1.0;
        self.last_successful_check = Some(Utc::now());
        self.last_failed_check = None;
        Ok(())
    }

    /// Records a failed health check.
    pub fn record_failure(&mut self) -> Result<(), AgentInfoError> {
        self.health_failures += 1;

        if self.health_failures >= self.max_health_failures {
            self.health_score = 0.0;
        } else {
            self.health_score =
                1.0 - (self.health_failures as f64 / self.max_health_failures as f64);
        }

        self.last_failed_check = Some(Utc::now());
        Ok(())
    }

    /// Checks if the agent is healthy.
    pub fn is_healthy(&self) -> bool {
        self.health_failures < self.max_health_failures && self.health_score > 0.0
    }
}

/// Agent information containing comprehensive agent data.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentInfo {
    /// Unique agent identifier.
    pub id: String,
    /// Current agent state.
    pub state: AgentState,
    /// Optional bead currently assigned to the agent.
    pub current_bead: Option<String>,
    /// Agent capabilities.
    pub capabilities: Vec<AgentCapability>,
    /// Agent workload history.
    pub workload_history: WorkloadHistory,
    /// Agent health metrics.
    pub health_metrics: HealthMetrics,
    /// Agent metadata/custom fields.
    pub custom_metadata: HashMap<String, String>,
    /// Agent registration timestamp.
    pub registered_at: DateTime<Utc>,
    /// Last heartbeat timestamp.
    pub last_heartbeat: DateTime<Utc>,
    /// Agent uptime in seconds.
    pub uptime_secs: u64,
}

impl AgentInfo {
    /// Creates a new agent info structure.
    ///
    /// # Arguments
    ///
    /// * `agent_id` - Unique identifier for the agent
    /// * `capabilities` - List of agent capabilities
    /// * `max_health_failures` - Maximum health check failures before unhealthy
    /// * `check_interval_secs` - Health check interval in seconds
    ///
    /// # Errors
    ///
    /// Returns `AgentInfoError` if required fields are empty or invalid.
    ///
    /// # Examples
    ///
    /// ```
    /// use oya::agent_info::{AgentInfo, AgentCapability, AgentState};
    /// use chrono::Utc;
    ///
    /// let capabilities = vec![
    ///     AgentCapability::new("code-generation".to_string(), "Generates code".to_string(), "1.0".to_string()).unwrap(),
    ///     AgentCapability::new("testing".to_string(), "Runs tests".to_string(), "1.0".to_string()).unwrap(),
    /// ];
    ///
    /// let agent = AgentInfo::new(
    ///     "agent-001".to_string(),
    ///     capabilities,
    ///     3,
    ///     30,
    /// )
    /// .unwrap();
    /// ```
    pub fn new(
        agent_id: String,
        capabilities: Vec<AgentCapability>,
        max_health_failures: u32,
        check_interval_secs: u64,
    ) -> Result<Self, AgentInfoError> {
        Self::validate_agent_id(&agent_id)?;

        if capabilities.is_empty() {
            return Err(AgentInfoError::EmptyCapabilities);
        }

        let capabilities: Vec<_> = capabilities
            .into_iter()
            .map(|cap| Self::validate_capability(cap))
            .collect::<Result<Vec<_>, _>>()?;

        let health_metrics = HealthMetrics::new(max_health_failures, check_interval_secs)?;

        Ok(Self {
            id: agent_id,
            state: AgentState::Idle,
            current_bead: None,
            capabilities,
            workload_history: WorkloadHistory::new(),
            health_metrics,
            custom_metadata: HashMap::new(),
            registered_at: Utc::now(),
            last_heartbeat: Utc::now(),
            uptime_secs: 0,
        })
    }

    /// Validates agent identifier.
    fn validate_agent_id(agent_id: &str) -> Result<(), AgentInfoError> {
        if agent_id.is_empty() {
            return Err(AgentInfoError::EmptyIdentifier);
        }

        if agent_id.len() > 64 {
            return Err(AgentInfoError::IdentifierTooLong);
        }

        Ok(())
    }

    /// Validates agent capability.
    fn validate_capability(capability: AgentCapability) -> Result<AgentCapability, AgentInfoError> {
        capability.new(
            capability.id.clone(),
            capability.description.clone(),
            capability.version.clone(),
        )
    }

    /// Records a heartbeat from the agent.
    ///
    /// Updates the last_heartbeat timestamp and calculates uptime.
    pub fn record_heartbeat(&mut self) -> Result<(), AgentInfoError> {
        let now = Utc::now();

        if now < self.last_heartbeat {
            return Err(AgentInfoError::InvalidHeartbeatTime);
        }

        self.uptime_secs = (now - self.registered_at).num_seconds() as u64;
        self.last_heartbeat = now;

        // Reset health failures on successful heartbeat
        self.health_metrics.record_success()?;
        self.state = AgentState::Idle;

        Ok(())
    }

    /// Assigns a bead to the agent.
    ///
    /// # Arguments
    ///
    /// * `bead_id` - ID of the bead to assign
    ///
    /// # Errors
    ///
    /// Returns `AgentInfoError` if:
    /// - The bead ID is empty
    /// - The agent is not in Idle state
    /// - The agent is terminal (ShuttingDown or Terminated)
    ///
    /// # Examples
    ///
    /// ```
    /// # use oya::agent_info::AgentInfo;
    /// # let mut agent = AgentInfo::new("agent-001".to_string(), vec![], 3, 30).unwrap();
    /// agent.assign_bead("bead-123".to_string()).unwrap();
    /// assert_eq!(agent.current_bead, Some("bead-123".to_string()));
    /// ```
    pub fn assign_bead(&mut self, bead_id: String) -> Result<(), AgentInfoError> {
        Self::validate_bead_id(&bead_id)?;

        if !self.state.can_accept_work() {
            return Err(AgentInfoError::AgentNotAvailable(self.state));
        }

        if self.state.is_terminal() {
            return Err(AgentInfoError::AgentTerminal(self.state));
        }

        self.current_bead = Some(bead_id);
        self.state = AgentState::Working;
        Ok(())
    }

    /// Completes the current bead assignment.
    ///
    /// Records the workload and returns the bead ID.
    pub fn complete_bead(&mut self) -> Result<String, AgentInfoError> {
        let bead_id = self
            .current_bead
            .take()
            .ok_or_else(|| AgentInfoError::NoActiveBead)?;

        self.workload_history.record_bead_completion()?;
        self.state = AgentState::Idle;

        Ok(bead_id)
    }

    /// Reports a failed bead execution.
    ///
    /// Returns the bead ID that failed.
    pub fn report_bead_failure(&mut self) -> Result<Option<String>, AgentInfoError> {
        let bead_id = self.current_bead.take();

        self.state = if self.health_metrics.is_healthy() {
            AgentState::Working
        } else {
            AgentState::Unhealthy
        };

        Ok(bead_id)
    }

    /// Updates the agent state.
    ///
    /// # Arguments
    ///
    /// * `state` - New state to set
    ///
    /// # Errors
    ///
    /// Returns `AgentInfoError` if attempting to transition to an invalid state.
    pub fn update_state(&mut self, new_state: AgentState) -> Result<(), AgentInfoError> {
        if !self.is_valid_state_transition(self.state, new_state) {
            return Err(AgentInfoError::InvalidStateTransition {
                from: self.state,
                to: new_state,
            });
        }

        self.state = new_state;
        Ok(())
    }

    /// Validates a state transition.
    fn is_valid_state_transition(&self, from: AgentState, to: AgentState) -> bool {
        match (from, to) {
            (AgentState::Idle, AgentState::Working) => true,
            (AgentState::Working, AgentState::Idle) => true,
            (AgentState::Idle, AgentState::ShuttingDown) => true,
            (AgentState::Working, AgentState::ShuttingDown) => true,
            (AgentState::ShuttingDown, AgentState::Terminated) => true,
            (AgentState::Unhealthy, AgentState::Idle) => true,
            (AgentState::Idle, AgentState::Unhealthy) => true,
            (AgentState::Working, AgentState::Unhealthy) => true,
            _ => false,
        }
    }

    /// Adds a custom metadata field.
    ///
    /// # Arguments
    ///
    /// * `key` - Metadata key
    /// * `value` - Metadata value
    ///
    /// # Errors
    ///
    /// Returns `AgentInfoError` if key or value is empty.
    pub fn add_metadata(&mut self, key: String, value: String) -> Result<(), AgentInfoError> {
        if key.is_empty() {
            return Err(AgentInfoError::EmptyMetadataKey);
        }

        if value.is_empty() {
            return Err(AgentInfoError::EmptyMetadataValue);
        }

        self.custom_metadata.insert(key, value);
        Ok(())
    }

    /// Gets a custom metadata value.
    ///
    /// Returns `None` if the key doesn't exist.
    pub fn get_metadata(&self, key: &str) -> Option<&String> {
        self.custom_metadata.get(key)
    }

    /// Gets agent statistics.
    pub fn stats(&self) -> AgentStats {
        AgentStats {
            id: self.id.clone(),
            state: self.state,
            uptime_secs: self.uptime_secs,
            beads_completed: self.workload_history.beads_completed,
            operations_executed: self.workload_history.operations_executed,
            avg_execution_secs: self.workload_history.avg_execution_secs,
            health_score: self.health_metrics.health_score,
            health_failures: self.health_metrics.health_failures,
        }
    }

    /// Converts agent info to a simplified view for API responses.
    pub fn to_api_response(&self) -> AgentApiResponse {
        AgentApiResponse {
            id: self.id.clone(),
            state: self.state,
            current_bead: self.current_bead.clone(),
            capabilities: self.capabilities.iter().map(|c| c.id.clone()).collect(),
            uptime_secs: self.uptime_secs,
            health_score: self.health_metrics.health_score,
        }
    }
}

/// Agent statistics summary.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentStats {
    /// Agent ID.
    pub id: String,
    /// Current state.
    pub state: AgentState,
    /// Uptime in seconds.
    pub uptime_secs: u64,
    /// Total beads completed.
    pub beads_completed: u64,
    /// Total operations executed.
    pub operations_executed: u64,
    /// Average execution time per operation.
    pub avg_execution_secs: Option<f64>,
    /// Current health score.
    pub health_score: f64,
    /// Health failure count.
    pub health_failures: u32,
}

/// Simplified API response for agents.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentApiResponse {
    /// Agent ID.
    pub id: String,
    /// Current state.
    pub state: AgentState,
    /// Optional current bead.
    pub current_bead: Option<String>,
    /// List of capability IDs.
    pub capabilities: Vec<String>,
    /// Uptime in seconds.
    pub uptime_secs: u64,
    /// Current health score.
    pub health_score: f64,
}

/// Agent information errors.
#[derive(Debug, Error, Clone, PartialEq)]
pub enum AgentInfoError {
    #[error("agent identifier is empty")]
    EmptyIdentifier,

    #[error("agent identifier is too long (max 64 characters)")]
    IdentifierTooLong,

    #[error("agent capabilities list is empty")]
    EmptyCapabilities,

    #[error("agent capability has empty identifier")]
    EmptyCapabilityIdentifier,

    #[error("agent capability has empty description")]
    EmptyCapabilityDescription,

    #[error("health metrics: max health failures cannot be zero")]
    ZeroMaxFailures,

    #[error("health metrics: check interval cannot be zero")]
    ZeroCheckInterval,

    #[error("workload history: duration cannot be negative")]
    NegativeDuration,

    #[error("invalid heartbeat timestamp: cannot be in the future")]
    InvalidHeartbeatTime,

    #[error("bead ID is empty")]
    EmptyBeadId,

    #[error("cannot assign bead: agent is not available (state: {0:?})")]
    AgentNotAvailable(AgentState),

    #[error("cannot assign bead: agent is terminal (state: {0:?})")]
    AgentTerminal(AgentState),

    #[error("no active bead to complete")]
    NoActiveBead,

    #[error("invalid state transition from {from:?} to {to:?}")]
    InvalidStateTransition { from: AgentState, to: AgentState },

    #[error("metadata key is empty")]
    EmptyMetadataKey,

    #[error("metadata value is empty")]
    EmptyMetadataValue,
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Duration;

    fn create_test_agent() -> AgentInfo {
        let capabilities = vec![
            AgentCapability::new(
                "code-generation".to_string(),
                "Generates code".to_string(),
                "1.0".to_string(),
            )
            .unwrap(),
            AgentCapability::new(
                "testing".to_string(),
                "Runs tests".to_string(),
                "1.0".to_string(),
            )
            .unwrap(),
        ];

        AgentInfo::new("agent-001".to_string(), capabilities, 3, 30).unwrap()
    }

    #[test]
    fn test_agent_creation() {
        let capabilities =
            vec![
                AgentCapability::new("test".to_string(), "desc".to_string(), "1.0".to_string())
                    .unwrap(),
            ];

        let agent = AgentInfo::new("agent-001".to_string(), capabilities, 3, 30).unwrap();

        assert_eq!(agent.id, "agent-001");
        assert_eq!(agent.state, AgentState::Idle);
        assert_eq!(agent.current_bead, None);
        assert_eq!(agent.capabilities.len(), 1);
        assert_eq!(agent.health_metrics.health_score, 1.0);
    }

    #[test]
    fn test_agent_creation_empty_id() {
        let capabilities =
            vec![
                AgentCapability::new("test".to_string(), "desc".to_string(), "1.0".to_string())
                    .unwrap(),
            ];

        let result = AgentInfo::new("".to_string(), capabilities, 3, 30);
        assert_eq!(result, Err(AgentInfoError::EmptyIdentifier));
    }

    #[test]
    fn test_agent_creation_no_capabilities() {
        let result = AgentInfo::new("agent-001".to_string(), vec![], 3, 30);
        assert_eq!(result, Err(AgentInfoError::EmptyCapabilities));
    }

    #[test]
    fn test_assign_bead() {
        let mut agent = create_test_agent();
        agent.assign_bead("bead-123".to_string()).unwrap();

        assert_eq!(agent.current_bead, Some("bead-123".to_string()));
        assert_eq!(agent.state, AgentState::Working);
    }

    #[test]
    fn test_assign_bead_unavailable() {
        let mut agent = create_test_agent();
        agent.state = AgentState::Working;

        let result = agent.assign_bead("bead-123".to_string());
        assert_eq!(
            result,
            Err(AgentInfoError::AgentNotAvailable(AgentState::Working))
        );
    }

    #[test]
    fn test_complete_bead() {
        let mut agent = create_test_agent();
        agent.current_bead = Some("bead-123".to_string());
        agent.state = AgentState::Working;

        let bead_id = agent.complete_bead().unwrap();

        assert_eq!(bead_id, "bead-123");
        assert_eq!(agent.current_bead, None);
        assert_eq!(agent.state, AgentState::Idle);
        assert_eq!(agent.workload_history.beads_completed, 1);
    }

    #[test]
    fn test_record_heartbeat() {
        let mut agent = create_test_agent();

        agent.record_heartbeat().unwrap();

        assert!(agent.uptime_secs > 0);
        assert_eq!(agent.state, AgentState::Idle);
    }

    #[test]
    fn test_health_metrics() {
        let mut metrics = HealthMetrics::new(3, 30).unwrap();

        metrics.record_success().unwrap();
        assert!(metrics.is_healthy());

        metrics.record_failure().unwrap();
        metrics.record_failure().unwrap();
        assert!(!metrics.is_healthy());
        assert_eq!(metrics.health_failures, 2);
    }

    #[test]
    fn test_state_transitions() {
        let mut agent = create_test_agent();

        assert!(agent.update_state(AgentState::Working).is_ok());
        assert!(agent.update_state(AgentState::Idle).is_ok());
    }

    #[test]
    fn test_invalid_state_transition() {
        let mut agent = create_test_agent();

        let result = agent.update_state(AgentState::Terminated);
        assert_eq!(
            result,
            Err(AgentInfoError::InvalidStateTransition {
                from: AgentState::Idle,
                to: AgentState::Terminated
            })
        );
    }

    #[test]
    fn test_metadata() {
        let mut agent = create_test_agent();

        agent
            .add_metadata("env".to_string(), "production".to_string())
            .unwrap();
        agent
            .add_metadata("version".to_string(), "2.0".to_string())
            .unwrap();

        assert_eq!(agent.get_metadata("env"), Some(&"production".to_string()));
        assert_eq!(agent.get_metadata("version"), Some(&"2.0".to_string()));
        assert_eq!(agent.get_metadata("nonexistent"), None);
    }

    #[test]
    fn test_workload_history() {
        let mut history = WorkloadHistory::new();

        history.record_operation(1.5).unwrap();
        history.record_operation(2.0).unwrap();
        history.record_bead_completion().unwrap();

        assert_eq!(history.beads_completed, 1);
        assert_eq!(history.operations_executed, 2);
        assert_eq!(history.avg_execution_secs, Some(1.75));
    }

    #[test]
    fn test_agent_to_api_response() {
        let agent = create_test_agent();
        let response = agent.to_api_response();

        assert_eq!(response.id, "agent-001");
        assert_eq!(response.state, AgentState::Idle);
        assert_eq!(response.capabilities.len(), 2);
    }

    #[test]
    fn test_negative_duration() {
        let mut history = WorkloadHistory::new();

        let result = history.record_operation(-1.0);
        assert_eq!(result, Err(AgentInfoError::NegativeDuration));
    }

    #[test]
    fn test_zero_max_failures() {
        let result = HealthMetrics::new(0, 30);
        assert_eq!(result, Err(AgentInfoError::ZeroMaxFailures));
    }

    #[test]
    fn test_zero_check_interval() {
        let result = HealthMetrics::new(3, 0);
        assert_eq!(result, Err(AgentInfoError::ZeroCheckInterval));
    }

    #[test]
    fn test_empty_bead_id() {
        let mut agent = create_test_agent();
        let result = agent.assign_bead("".to_string());
        assert_eq!(result, Err(AgentInfoError::EmptyBeadId));
    }
}
