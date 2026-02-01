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

/// 8-state lifecycle for beads (from nuoc design).
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
}
