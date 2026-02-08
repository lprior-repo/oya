//! Core types for the events crate.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use ulid::Ulid;

/// Unique identifier for a bead.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct BeadId(Ulid);

impl BeadId {
    /// Create a new random bead ID.
    pub fn new() -> Self {
        Self(Ulid::new())
    }

    /// Create from a ULID.
    pub fn from_ulid(ulid: Ulid) -> Self {
        Self(ulid)
    }

    /// Get the inner ULID.
    pub fn as_ulid(&self) -> Ulid {
        self.0
    }
}

impl Default for BeadId {
    fn default() -> Self {
        Self::new()
    }
}

impl std::fmt::Display for BeadId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl std::str::FromStr for BeadId {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Ulid::from_str(s)
            .map(BeadId)
            .map_err(|e| format!("Invalid BeadId: {}", e))
    }
}

impl TryFrom<&str> for BeadId {
    type Error = String;

    fn try_from(s: &str) -> Result<Self, Self::Error> {
        s.parse()
    }
}

impl TryFrom<String> for BeadId {
    type Error = String;

    fn try_from(s: String) -> Result<Self, Self::Error> {
        s.parse()
    }
}

/// Unique identifier for an event.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct EventId(Ulid);

impl EventId {
    /// Create a new random event ID.
    pub fn new() -> Self {
        Self(Ulid::new())
    }

    /// Create from a ULID.
    pub fn from_ulid(ulid: Ulid) -> Self {
        Self(ulid)
    }

    /// Get the inner ULID.
    pub fn as_ulid(&self) -> Ulid {
        self.0
    }
}

impl Default for EventId {
    fn default() -> Self {
        Self::new()
    }
}

impl std::fmt::Display for EventId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

/// Unique identifier for a phase.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct PhaseId(Ulid);

impl PhaseId {
    /// Create a new random phase ID.
    pub fn new() -> Self {
        Self(Ulid::new())
    }

    /// Create from a ULID.
    pub fn from_ulid(ulid: Ulid) -> Self {
        Self(ulid)
    }

    /// Get the inner ULID.
    pub fn as_ulid(&self) -> Ulid {
        self.0
    }
}

impl Default for PhaseId {
    fn default() -> Self {
        Self::new()
    }
}

impl std::fmt::Display for PhaseId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

/// 8-state lifecycle for beads.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum BeadState {
    /// Waiting for dependencies.
    Pending,
    /// Ready to be claimed.
    Scheduled,
    /// Claimed, about to run.
    Ready,
    /// Actively executing.
    Running,
    /// Paused by user.
    Suspended,
    /// Waiting after failure.
    BackingOff,
    /// System pause (resource constraint).
    Paused,
    /// Terminal: success or failure.
    Completed,
}

impl BeadState {
    /// Check if this is a terminal state.
    pub fn is_terminal(&self) -> bool {
        matches!(self, Self::Completed)
    }

    /// Check if transition to target state is valid.
    pub fn can_transition_to(&self, target: BeadState) -> bool {
        use BeadState::*;
        matches!(
            (self, target),
            // From Pending
            (Pending, Scheduled)
                | (Pending, Completed) // Can be cancelled
                // From Scheduled
                | (Scheduled, Ready)
                | (Scheduled, Pending) // Unscheduled
                | (Scheduled, Completed) // Cancelled
                // From Ready
                | (Ready, Running)
                | (Ready, Scheduled) // Unclaimed
                | (Ready, Completed) // Cancelled
                // From Running
                | (Running, Suspended)
                | (Running, BackingOff)
                | (Running, Paused)
                | (Running, Completed)
                // From Suspended
                | (Suspended, Running)
                | (Suspended, Completed) // Cancelled
                // From BackingOff
                | (BackingOff, Running) // Retry
                | (BackingOff, Completed) // Give up
                // From Paused
                | (Paused, Running)
                | (Paused, Completed) // Cancelled
        )
    }

    /// Get valid transitions from this state.
    pub fn valid_transitions(&self) -> Vec<BeadState> {
        use BeadState::*;
        match self {
            Pending => vec![Scheduled, Completed],
            Scheduled => vec![Ready, Pending, Completed],
            Ready => vec![Running, Scheduled, Completed],
            Running => vec![Suspended, BackingOff, Paused, Completed],
            Suspended => vec![Running, Completed],
            BackingOff => vec![Running, Completed],
            Paused => vec![Running, Completed],
            Completed => vec![],
        }
    }
}

impl std::fmt::Display for BeadState {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let s = match self {
            Self::Pending => "pending",
            Self::Scheduled => "scheduled",
            Self::Ready => "ready",
            Self::Running => "running",
            Self::Suspended => "suspended",
            Self::BackingOff => "backing_off",
            Self::Paused => "paused",
            Self::Completed => "completed",
        };
        write!(f, "{s}")
    }
}

/// Bead specification.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BeadSpec {
    /// Bead title.
    pub title: String,
    /// Description.
    pub description: Option<String>,
    /// Dependencies (other bead IDs).
    pub dependencies: Vec<BeadId>,
    /// Priority (lower = higher priority).
    pub priority: u32,
    /// Estimated complexity.
    pub complexity: Complexity,
    /// Labels/tags.
    pub labels: Vec<String>,
    /// Additional metadata.
    pub metadata: Option<serde_json::Value>,
}

impl BeadSpec {
    /// Create a new bead spec with the given title.
    pub fn new(title: impl Into<String>) -> Self {
        Self {
            title: title.into(),
            description: None,
            dependencies: Vec::new(),
            priority: 100,
            complexity: Complexity::Medium,
            labels: Vec::new(),
            metadata: None,
        }
    }

    /// Set the description.
    pub fn with_description(mut self, desc: impl Into<String>) -> Self {
        self.description = Some(desc.into());
        self
    }

    /// Add a dependency.
    pub fn with_dependency(mut self, dep: BeadId) -> Self {
        self.dependencies.push(dep);
        self
    }

    /// Set dependencies.
    pub fn with_dependencies(mut self, deps: Vec<BeadId>) -> Self {
        self.dependencies = deps;
        self
    }

    /// Set priority.
    pub fn with_priority(mut self, priority: u32) -> Self {
        self.priority = priority;
        self
    }

    /// Set complexity.
    pub fn with_complexity(mut self, complexity: Complexity) -> Self {
        self.complexity = complexity;
        self
    }

    /// Add a label.
    pub fn with_label(mut self, label: impl Into<String>) -> Self {
        self.labels.push(label.into());
        self
    }
}

/// Complexity levels for routing.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Complexity {
    /// Simple task, skip some phases.
    Simple,
    /// Medium complexity.
    Medium,
    /// Complex task, full phases.
    Complex,
}

impl std::fmt::Display for Complexity {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let s = match self {
            Self::Simple => "simple",
            Self::Medium => "medium",
            Self::Complex => "complex",
        };
        write!(f, "{s}")
    }
}

/// Phase output (simplified for events).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PhaseOutput {
    /// Whether the phase succeeded.
    pub success: bool,
    /// Output data.
    pub data: Vec<u8>,
    /// Message.
    pub message: Option<String>,
}

impl PhaseOutput {
    /// Create a successful output.
    pub fn success(data: Vec<u8>) -> Self {
        Self {
            success: true,
            data,
            message: None,
        }
    }

    /// Create a failed output.
    pub fn failure(message: impl Into<String>) -> Self {
        Self {
            success: false,
            data: Vec::new(),
            message: Some(message.into()),
        }
    }
}

/// Bead result (terminal state info).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BeadResult {
    /// Whether the bead succeeded.
    pub success: bool,
    /// Final output.
    pub output: Option<Vec<u8>>,
    /// Error message if failed.
    pub error: Option<String>,
    /// Total duration in milliseconds.
    pub duration_ms: u64,
}

impl BeadResult {
    /// Create a successful result.
    pub fn success(output: Vec<u8>, duration_ms: u64) -> Self {
        Self {
            success: true,
            output: Some(output),
            error: None,
            duration_ms,
        }
    }

    /// Create a failed result.
    pub fn failure(error: impl Into<String>, duration_ms: u64) -> Self {
        Self {
            success: false,
            output: None,
            error: Some(error.into()),
            duration_ms,
        }
    }
}

/// State transition record.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StateTransition {
    /// From state.
    pub from: BeadState,
    /// To state.
    pub to: BeadState,
    /// Timestamp.
    pub timestamp: DateTime<Utc>,
    /// Reason for transition.
    pub reason: Option<String>,
}

impl StateTransition {
    /// Create a new state transition.
    pub fn new(from: BeadState, to: BeadState) -> Self {
        Self {
            from,
            to,
            timestamp: Utc::now(),
            reason: None,
        }
    }

    /// Add a reason.
    pub fn with_reason(mut self, reason: impl Into<String>) -> Self {
        self.reason = Some(reason.into());
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_bead_id() {
        let id1 = BeadId::new();
        let id2 = BeadId::new();
        assert_ne!(id1, id2);
    }

    #[test]
    fn test_bead_state_transitions() {
        assert!(BeadState::Pending.can_transition_to(BeadState::Scheduled));
        assert!(BeadState::Running.can_transition_to(BeadState::Completed));
        assert!(!BeadState::Completed.can_transition_to(BeadState::Running));
        assert!(!BeadState::Pending.can_transition_to(BeadState::Running));
    }

    #[test]
    fn test_bead_state_valid_transitions() {
        let transitions = BeadState::Running.valid_transitions();
        assert!(transitions.contains(&BeadState::Completed));
        assert!(transitions.contains(&BeadState::Suspended));
        assert!(!transitions.contains(&BeadState::Pending));
    }

    #[test]
    fn test_bead_spec_builder() {
        let dep = BeadId::new();
        let spec = BeadSpec::new("Test task")
            .with_description("A test")
            .with_dependency(dep)
            .with_priority(50)
            .with_complexity(Complexity::Simple)
            .with_label("test");

        assert_eq!(spec.title, "Test task");
        assert_eq!(spec.priority, 50);
        assert_eq!(spec.complexity, Complexity::Simple);
        assert!(spec.dependencies.contains(&dep));
    }

    // ==========================================================================
    // BeadState::is_terminal BEHAVIORAL TESTS
    // ==========================================================================

    #[test]
    fn should_return_true_for_completed_state() {
        assert!(
            BeadState::Completed.is_terminal(),
            "Completed should be terminal"
        );
    }

    #[test]
    fn should_return_false_for_all_non_terminal_states() {
        let non_terminal_states = [
            BeadState::Pending,
            BeadState::Scheduled,
            BeadState::Ready,
            BeadState::Running,
            BeadState::Suspended,
            BeadState::BackingOff,
            BeadState::Paused,
        ];

        for state in non_terminal_states {
            assert!(!state.is_terminal(), "{:?} should NOT be terminal", state);
        }
    }

    // ==========================================================================
    // ID Types from_ulid/as_ulid BEHAVIORAL TESTS
    // ==========================================================================

    #[test]
    fn should_roundtrip_bead_id_through_ulid() {
        let original_ulid = Ulid::new();
        let bead_id = BeadId::from_ulid(original_ulid);
        let extracted_ulid = bead_id.as_ulid();

        assert_eq!(
            original_ulid, extracted_ulid,
            "BeadId should roundtrip through ULID"
        );
    }

    #[test]
    fn should_roundtrip_event_id_through_ulid() {
        let original_ulid = Ulid::new();
        let event_id = EventId::from_ulid(original_ulid);
        let extracted_ulid = event_id.as_ulid();

        assert_eq!(
            original_ulid, extracted_ulid,
            "EventId should roundtrip through ULID"
        );
    }

    #[test]
    fn should_roundtrip_phase_id_through_ulid() {
        let original_ulid = Ulid::new();
        let phase_id = PhaseId::from_ulid(original_ulid);
        let extracted_ulid = phase_id.as_ulid();

        assert_eq!(
            original_ulid, extracted_ulid,
            "PhaseId should roundtrip through ULID"
        );
    }

    #[test]
    fn should_preserve_bead_id_identity_through_from_ulid() {
        let ulid = Ulid::new();
        let id1 = BeadId::from_ulid(ulid);
        let id2 = BeadId::from_ulid(ulid);

        assert_eq!(id1, id2, "Same ULID should produce equal BeadIds");
    }

    #[test]
    fn should_preserve_event_id_identity_through_from_ulid() {
        let ulid = Ulid::new();
        let id1 = EventId::from_ulid(ulid);
        let id2 = EventId::from_ulid(ulid);

        assert_eq!(id1, id2, "Same ULID should produce equal EventIds");
    }

    #[test]
    fn should_preserve_phase_id_identity_through_from_ulid() {
        let ulid = Ulid::new();
        let id1 = PhaseId::from_ulid(ulid);
        let id2 = PhaseId::from_ulid(ulid);

        assert_eq!(id1, id2, "Same ULID should produce equal PhaseIds");
    }

    // ==========================================================================
    // Display Implementation BEHAVIORAL TESTS
    // ==========================================================================

    #[test]
    fn should_display_bead_id_as_ulid_string() {
        let ulid = Ulid::new();
        let bead_id = BeadId::from_ulid(ulid);
        let displayed = format!("{}", bead_id);

        assert_eq!(
            displayed,
            ulid.to_string(),
            "BeadId display should match ULID display"
        );
    }

    #[test]
    fn should_display_event_id_as_ulid_string() {
        let ulid = Ulid::new();
        let event_id = EventId::from_ulid(ulid);
        let displayed = format!("{}", event_id);

        assert_eq!(
            displayed,
            ulid.to_string(),
            "EventId display should match ULID display"
        );
    }

    #[test]
    fn should_display_phase_id_as_ulid_string() {
        let ulid = Ulid::new();
        let phase_id = PhaseId::from_ulid(ulid);
        let displayed = format!("{}", phase_id);

        assert_eq!(
            displayed,
            ulid.to_string(),
            "PhaseId display should match ULID display"
        );
    }

    #[test]
    fn should_display_all_bead_states_correctly() {
        assert_eq!(format!("{}", BeadState::Pending), "pending");
        assert_eq!(format!("{}", BeadState::Scheduled), "scheduled");
        assert_eq!(format!("{}", BeadState::Ready), "ready");
        assert_eq!(format!("{}", BeadState::Running), "running");
        assert_eq!(format!("{}", BeadState::Suspended), "suspended");
        assert_eq!(format!("{}", BeadState::BackingOff), "backing_off");
        assert_eq!(format!("{}", BeadState::Paused), "paused");
        assert_eq!(format!("{}", BeadState::Completed), "completed");
    }

    #[test]
    fn should_display_all_complexity_levels_correctly() {
        assert_eq!(format!("{}", Complexity::Simple), "simple");
        assert_eq!(format!("{}", Complexity::Medium), "medium");
        assert_eq!(format!("{}", Complexity::Complex), "complex");
    }

    // ==========================================================================
    // PhaseOutput BEHAVIORAL TESTS
    // ==========================================================================

    #[test]
    fn should_create_success_phase_output_with_correct_fields() {
        let data = vec![1, 2, 3];
        let output = PhaseOutput::success(data.clone());

        assert!(output.success, "success output should have success=true");
        assert_eq!(output.data, data, "success output should preserve data");
        assert!(
            output.message.is_none(),
            "success output should have no message"
        );
    }

    #[test]
    fn should_create_failure_phase_output_with_correct_fields() {
        let output = PhaseOutput::failure("something went wrong");

        assert!(!output.success, "failure output should have success=false");
        assert!(
            output.data.is_empty(),
            "failure output should have empty data"
        );
        assert_eq!(
            output.message,
            Some("something went wrong".to_string()),
            "failure output should have message"
        );
    }

    // ==========================================================================
    // BeadResult BEHAVIORAL TESTS
    // ==========================================================================

    #[test]
    fn should_create_success_bead_result_with_correct_fields() {
        let output = vec![4, 5, 6];
        let result = BeadResult::success(output.clone(), 1000);

        assert!(result.success, "success result should have success=true");
        assert_eq!(
            result.output,
            Some(output),
            "success result should have output"
        );
        assert!(
            result.error.is_none(),
            "success result should have no error"
        );
        assert_eq!(
            result.duration_ms, 1000,
            "success result should preserve duration"
        );
    }

    #[test]
    fn should_create_failure_bead_result_with_correct_fields() {
        let result = BeadResult::failure("task failed", 500);

        assert!(!result.success, "failure result should have success=false");
        assert!(
            result.output.is_none(),
            "failure result should have no output"
        );
        assert_eq!(
            result.error,
            Some("task failed".to_string()),
            "failure result should have error"
        );
        assert_eq!(
            result.duration_ms, 500,
            "failure result should preserve duration"
        );
    }

    // ==========================================================================
    // StateTransition BEHAVIORAL TESTS
    // ==========================================================================

    #[test]
    fn should_create_state_transition_with_correct_states() {
        let transition = StateTransition::new(BeadState::Pending, BeadState::Scheduled);

        assert_eq!(transition.from, BeadState::Pending);
        assert_eq!(transition.to, BeadState::Scheduled);
        assert!(transition.reason.is_none());
    }

    #[test]
    fn should_add_reason_to_state_transition() {
        let transition = StateTransition::new(BeadState::Running, BeadState::Completed)
            .with_reason("task completed successfully");

        assert_eq!(
            transition.reason,
            Some("task completed successfully".to_string())
        );
    }

    // ==========================================================================
    // BeadSpec Builder BEHAVIORAL TESTS
    // ==========================================================================

    #[test]
    fn should_set_dependencies_via_with_dependencies() {
        let dep1 = BeadId::new();
        let dep2 = BeadId::new();
        let spec = BeadSpec::new("Test").with_dependencies(vec![dep1, dep2]);

        assert_eq!(spec.dependencies.len(), 2);
        assert!(spec.dependencies.contains(&dep1));
        assert!(spec.dependencies.contains(&dep2));
    }

    #[test]
    fn should_replace_dependencies_when_using_with_dependencies() {
        let dep1 = BeadId::new();
        let dep2 = BeadId::new();
        let spec = BeadSpec::new("Test")
            .with_dependency(dep1)
            .with_dependencies(vec![dep2]); // Should replace, not add

        assert_eq!(spec.dependencies.len(), 1);
        assert!(!spec.dependencies.contains(&dep1));
        assert!(spec.dependencies.contains(&dep2));
    }
}
