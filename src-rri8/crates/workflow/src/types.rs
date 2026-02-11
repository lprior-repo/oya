//! Core types for the workflow engine.

use std::sync::Arc;
use std::time::Duration;

use chrono::{DateTime, Utc};
use itertools::Itertools;
use serde::{Deserialize, Serialize};
use ulid::Ulid;

/// Unique identifier for a workflow.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct WorkflowId(Ulid);

impl WorkflowId {
    /// Create a new random workflow ID.
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

impl Default for WorkflowId {
    fn default() -> Self {
        Self::new()
    }
}

impl std::fmt::Display for WorkflowId {
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

/// Phase definition in a workflow.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Phase {
    /// Unique phase identifier.
    pub id: PhaseId,
    /// Human-readable phase name.
    pub name: String,
    /// Maximum execution time.
    pub timeout: Duration,
    /// Number of retry attempts on failure.
    pub retries: u32,
    /// Optional description.
    pub description: Option<String>,
    /// Phase-specific configuration (JSON).
    pub config: Option<serde_json::Value>,
}

impl Phase {
    /// Create a new phase with the given name.
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            id: PhaseId::new(),
            name: name.into(),
            timeout: Duration::from_secs(300), // 5 minutes default
            retries: 3,
            description: None,
            config: None,
        }
    }

    /// Set the timeout.
    pub fn with_timeout(mut self, timeout: Duration) -> Self {
        self.timeout = timeout;
        self
    }

    /// Set the number of retries.
    pub fn with_retries(mut self, retries: u32) -> Self {
        self.retries = retries;
        self
    }

    /// Set the description.
    pub fn with_description(mut self, desc: impl Into<String>) -> Self {
        self.description = Some(desc.into());
        self
    }

    /// Set phase-specific configuration.
    pub fn with_config(mut self, config: serde_json::Value) -> Self {
        self.config = Some(config);
        self
    }
}

/// Workflow state.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum WorkflowState {
    /// Workflow has not started.
    Pending,
    /// Workflow is currently executing.
    Running,
    /// Workflow is paused (can be resumed).
    Paused,
    /// Workflow completed successfully.
    Completed,
    /// Workflow failed.
    Failed,
    /// Workflow was cancelled.
    Cancelled,
}

impl WorkflowState {
    /// Check if the workflow is in a terminal state.
    pub fn is_terminal(&self) -> bool {
        matches!(self, Self::Completed | Self::Failed | Self::Cancelled)
    }

    /// Check if the workflow can transition to the given state.
    pub fn can_transition_to(&self, target: WorkflowState) -> bool {
        use WorkflowState::*;
        matches!(
            (self, target),
            (Pending, Running)
                | (Running, Paused)
                | (Running, Completed)
                | (Running, Failed)
                | (Running, Cancelled)
                | (Paused, Running)
                | (Paused, Cancelled)
        )
    }
}

impl std::fmt::Display for WorkflowState {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let s = match self {
            Self::Pending => "pending",
            Self::Running => "running",
            Self::Paused => "paused",
            Self::Completed => "completed",
            Self::Failed => "failed",
            Self::Cancelled => "cancelled",
        };
        write!(f, "{s}")
    }
}

/// Workflow definition.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Workflow {
    /// Unique workflow identifier.
    pub id: WorkflowId,
    /// Human-readable name.
    pub name: String,
    /// Ordered list of phases.
    pub phases: Vec<Phase>,
    /// Current phase index (0-based).
    pub current_phase: usize,
    /// Workflow state.
    pub state: WorkflowState,
    /// Creation timestamp.
    pub created_at: DateTime<Utc>,
    /// Last update timestamp.
    pub updated_at: DateTime<Utc>,
    /// Optional metadata.
    pub metadata: Option<serde_json::Value>,
}

impl Workflow {
    /// Create a new workflow with the given name.
    pub fn new(name: impl Into<String>) -> Self {
        let now = Utc::now();
        Self {
            id: WorkflowId::new(),
            name: name.into(),
            phases: Vec::new(),
            current_phase: 0,
            state: WorkflowState::Pending,
            created_at: now,
            updated_at: now,
            metadata: None,
        }
    }

    /// Add a phase to the workflow.
    pub fn add_phase(mut self, phase: Phase) -> Self {
        self.phases.push(phase);
        self
    }

    /// Add multiple phases.
    pub fn with_phases(mut self, phases: impl IntoIterator<Item = Phase>) -> Self {
        self.phases.extend(phases);
        self
    }

    /// Set metadata.
    pub fn with_metadata(mut self, metadata: serde_json::Value) -> Self {
        self.metadata = Some(metadata);
        self
    }

    /// Get the current phase, if any.
    pub fn current_phase(&self) -> Option<&Phase> {
        self.phases.get(self.current_phase)
    }

    /// Check if all phases are complete.
    pub fn is_complete(&self) -> bool {
        self.current_phase >= self.phases.len()
    }

    /// Advance to the next phase.
    pub fn advance(&mut self) {
        if self.current_phase < self.phases.len() {
            self.current_phase += 1;
            self.updated_at = Utc::now();
        }
    }

    /// Get progress as a fraction (0.0 to 1.0).
    pub fn progress(&self) -> f64 {
        if self.phases.is_empty() {
            return 1.0;
        }
        self.current_phase as f64 / self.phases.len() as f64
    }
}

/// Checkpoint for rewind capability.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Checkpoint {
    /// Phase ID this checkpoint is for.
    pub phase_id: PhaseId,
    /// Timestamp when checkpoint was created.
    pub timestamp: DateTime<Utc>,
    /// Serialized state data.
    pub state: Arc<Vec<u8>>,
    /// Serialized input data.
    pub inputs: Arc<Vec<u8>>,
    /// Serialized output data (if phase completed).
    pub outputs: Option<Arc<Vec<u8>>>,
}

impl Checkpoint {
    /// Create a new checkpoint.
    pub fn new(phase_id: PhaseId, state: Vec<u8>, inputs: Vec<u8>) -> Self {
        Self {
            phase_id,
            timestamp: Utc::now(),
            state: Arc::new(state),
            inputs: Arc::new(inputs),
            outputs: None,
        }
    }

    /// Add output data to the checkpoint.
    pub fn with_outputs(mut self, outputs: Vec<u8>) -> Self {
        self.outputs = Some(Arc::new(outputs));
        self
    }
}

/// Journal entry for replay.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum JournalEntry {
    /// Phase execution started.
    PhaseStarted {
        phase_id: PhaseId,
        phase_name: String,
        timestamp: DateTime<Utc>,
    },
    /// Phase completed successfully.
    PhaseCompleted {
        phase_id: PhaseId,
        phase_name: String,
        output: Arc<Vec<u8>>,
        timestamp: DateTime<Utc>,
    },
    /// Phase failed.
    PhaseFailed {
        phase_id: PhaseId,
        phase_name: String,
        error: String,
        timestamp: DateTime<Utc>,
    },
    /// Checkpoint was created.
    CheckpointCreated {
        phase_id: PhaseId,
        timestamp: DateTime<Utc>,
    },
    /// Rewind was initiated.
    RewindInitiated {
        to_phase: PhaseId,
        reason: String,
        timestamp: DateTime<Utc>,
    },
    /// Workflow state changed.
    StateChanged {
        from: WorkflowState,
        to: WorkflowState,
        timestamp: DateTime<Utc>,
    },
}

impl JournalEntry {
    /// Create a phase started entry.
    pub fn phase_started(phase_id: PhaseId, phase_name: impl Into<String>) -> Self {
        Self::PhaseStarted {
            phase_id,
            phase_name: phase_name.into(),
            timestamp: Utc::now(),
        }
    }

    /// Create a phase completed entry.
    pub fn phase_completed(
        phase_id: PhaseId,
        phase_name: impl Into<String>,
        output: Vec<u8>,
    ) -> Self {
        Self::PhaseCompleted {
            phase_id,
            phase_name: phase_name.into(),
            output: Arc::new(output),
            timestamp: Utc::now(),
        }
    }

    /// Create a phase failed entry.
    pub fn phase_failed(
        phase_id: PhaseId,
        phase_name: impl Into<String>,
        error: impl Into<String>,
    ) -> Self {
        Self::PhaseFailed {
            phase_id,
            phase_name: phase_name.into(),
            error: error.into(),
            timestamp: Utc::now(),
        }
    }

    /// Create a checkpoint created entry.
    pub fn checkpoint_created(phase_id: PhaseId) -> Self {
        Self::CheckpointCreated {
            phase_id,
            timestamp: Utc::now(),
        }
    }

    /// Create a rewind initiated entry.
    pub fn rewind_initiated(to_phase: PhaseId, reason: impl Into<String>) -> Self {
        Self::RewindInitiated {
            to_phase,
            reason: reason.into(),
            timestamp: Utc::now(),
        }
    }

    /// Create a state changed entry.
    pub fn state_changed(from: WorkflowState, to: WorkflowState) -> Self {
        Self::StateChanged {
            from,
            to,
            timestamp: Utc::now(),
        }
    }

    /// Get the timestamp of this entry.
    pub fn timestamp(&self) -> DateTime<Utc> {
        match self {
            Self::PhaseStarted { timestamp, .. }
            | Self::PhaseCompleted { timestamp, .. }
            | Self::PhaseFailed { timestamp, .. }
            | Self::CheckpointCreated { timestamp, .. }
            | Self::RewindInitiated { timestamp, .. }
            | Self::StateChanged { timestamp, .. } => *timestamp,
        }
    }
}

/// Journal for workflow replay.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Journal {
    /// Journal entries in chronological order.
    entries: Vec<JournalEntry>,
}

impl Journal {
    /// Create a new empty journal.
    pub fn new() -> Self {
        Self::default()
    }

    /// Append an entry to the journal.
    pub fn append(&mut self, entry: JournalEntry) {
        self.entries.push(entry);
    }

    /// Get all entries.
    pub fn entries(&self) -> &[JournalEntry] {
        &self.entries
    }

    /// Get entries for a specific phase.
    pub fn entries_for_phase(&self, phase_id: PhaseId) -> Vec<&JournalEntry> {
        self.entries
            .iter()
            .filter(|e| match e {
                JournalEntry::PhaseStarted { phase_id: id, .. }
                | JournalEntry::PhaseCompleted { phase_id: id, .. }
                | JournalEntry::PhaseFailed { phase_id: id, .. }
                | JournalEntry::CheckpointCreated { phase_id: id, .. } => *id == phase_id,
                JournalEntry::RewindInitiated { to_phase, .. } => *to_phase == phase_id,
                JournalEntry::StateChanged { .. } => false,
            })
            .collect_vec()
    }

    /// Get the number of entries.
    pub fn len(&self) -> usize {
        self.entries.len()
    }

    /// Check if the journal is empty.
    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    /// Clear all entries.
    pub fn clear(&mut self) {
        self.entries.clear();
    }
}

/// Phase execution context.
#[derive(Debug, Clone)]
pub struct PhaseContext {
    /// Workflow ID.
    pub workflow_id: WorkflowId,
    /// Phase being executed.
    pub phase: Phase,
    /// Attempt number (1-based).
    pub attempt: u32,
    /// Inputs from previous phase (if any).
    pub previous_output: Option<Arc<Vec<u8>>>,
    /// Global workflow metadata.
    pub metadata: Option<serde_json::Value>,
}

impl PhaseContext {
    /// Create a new phase context.
    pub fn new(workflow_id: WorkflowId, phase: Phase) -> Self {
        Self {
            workflow_id,
            phase,
            attempt: 1,
            previous_output: None,
            metadata: None,
        }
    }

    /// Set the attempt number.
    pub fn with_attempt(mut self, attempt: u32) -> Self {
        self.attempt = attempt;
        self
    }

    /// Set the previous output.
    pub fn with_previous_output(mut self, output: Vec<u8>) -> Self {
        self.previous_output = Some(Arc::new(output));
        self
    }

    /// Set the metadata.
    pub fn with_metadata(mut self, metadata: serde_json::Value) -> Self {
        self.metadata = Some(metadata);
        self
    }
}

/// Phase execution output.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PhaseOutput {
    /// Whether the phase succeeded.
    pub success: bool,
    /// Output data (serialized).
    pub data: Arc<Vec<u8>>,
    /// Optional message.
    pub message: Option<String>,
    /// Artifacts produced (paths or identifiers).
    pub artifacts: Vec<String>,
    /// Duration of execution.
    pub duration_ms: u64,
}

impl PhaseOutput {
    /// Create a successful output.
    pub fn success(data: Vec<u8>) -> Self {
        Self {
            success: true,
            data: Arc::new(data),
            message: None,
            artifacts: Vec::new(),
            duration_ms: 0,
        }
    }

    /// Create a failed output.
    pub fn failure(message: impl Into<String>) -> Self {
        Self {
            success: false,
            data: Arc::new(Vec::new()),
            message: Some(message.into()),
            artifacts: Vec::new(),
            duration_ms: 0,
        }
    }

    /// Add a message.
    pub fn with_message(mut self, msg: impl Into<String>) -> Self {
        self.message = Some(msg.into());
        self
    }

    /// Add artifacts.
    pub fn with_artifacts(mut self, artifacts: Vec<String>) -> Self {
        self.artifacts = artifacts;
        self
    }

    /// Set the duration.
    pub fn with_duration_ms(mut self, duration_ms: u64) -> Self {
        self.duration_ms = duration_ms;
        self
    }
}

/// Workflow execution result.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkflowResult {
    /// Workflow ID.
    pub workflow_id: WorkflowId,
    /// Final state.
    pub state: WorkflowState,
    /// Phase outputs in execution order.
    pub phase_outputs: Vec<(PhaseId, PhaseOutput)>,
    /// Total duration in milliseconds.
    pub duration_ms: u64,
    /// Error message if failed.
    pub error: Option<String>,
}

impl WorkflowResult {
    /// Create a successful result.
    pub fn success(workflow_id: WorkflowId, phase_outputs: Vec<(PhaseId, PhaseOutput)>) -> Self {
        let duration_ms = phase_outputs.iter().map(|(_, o)| o.duration_ms).sum();
        Self {
            workflow_id,
            state: WorkflowState::Completed,
            phase_outputs,
            duration_ms,
            error: None,
        }
    }

    /// Create a failed result.
    pub fn failure(
        workflow_id: WorkflowId,
        phase_outputs: Vec<(PhaseId, PhaseOutput)>,
        error: impl Into<String>,
    ) -> Self {
        let duration_ms = phase_outputs.iter().map(|(_, o)| o.duration_ms).sum();
        Self {
            workflow_id,
            state: WorkflowState::Failed,
            phase_outputs,
            duration_ms,
            error: Some(error.into()),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_workflow_id() {
        let id1 = WorkflowId::new();
        let id2 = WorkflowId::new();
        assert_ne!(id1, id2);
    }

    #[test]
    fn test_phase_builder() {
        let phase = Phase::new("build")
            .with_timeout(Duration::from_secs(60))
            .with_retries(5)
            .with_description("Build the project");

        assert_eq!(phase.name, "build");
        assert_eq!(phase.timeout, Duration::from_secs(60));
        assert_eq!(phase.retries, 5);
        assert!(phase.description.is_some());
    }

    #[test]
    fn test_workflow_builder() {
        let workflow = Workflow::new("test-workflow")
            .add_phase(Phase::new("phase1"))
            .add_phase(Phase::new("phase2"));

        assert_eq!(workflow.name, "test-workflow");
        assert_eq!(workflow.phases.len(), 2);
        assert_eq!(workflow.current_phase, 0);
    }

    #[test]
    fn test_workflow_progress() {
        let mut workflow = Workflow::new("test")
            .add_phase(Phase::new("p1"))
            .add_phase(Phase::new("p2"))
            .add_phase(Phase::new("p3"))
            .add_phase(Phase::new("p4"));

        assert_eq!(workflow.progress(), 0.0);

        workflow.advance();
        assert_eq!(workflow.progress(), 0.25);

        workflow.advance();
        assert_eq!(workflow.progress(), 0.5);
    }

    #[test]
    fn test_workflow_state_transitions() {
        assert!(WorkflowState::Pending.can_transition_to(WorkflowState::Running));
        assert!(WorkflowState::Running.can_transition_to(WorkflowState::Completed));
        assert!(!WorkflowState::Completed.can_transition_to(WorkflowState::Running));
        assert!(!WorkflowState::Pending.can_transition_to(WorkflowState::Completed));
    }

    #[test]
    fn test_journal() {
        let mut journal = Journal::new();
        let phase_id = PhaseId::new();

        journal.append(JournalEntry::phase_started(phase_id, "build"));
        journal.append(JournalEntry::phase_completed(
            phase_id,
            "build",
            vec![1, 2, 3],
        ));

        assert_eq!(journal.len(), 2);
        assert_eq!(journal.entries_for_phase(phase_id).len(), 2);
    }

    #[test]
    fn test_phase_output() {
        let output = PhaseOutput::success(vec![1, 2, 3])
            .with_message("Build completed")
            .with_artifacts(vec!["target/release/app".to_string()])
            .with_duration_ms(1500);

        assert!(output.success);
        assert_eq!(*output.data, vec![1, 2, 3]);
        assert_eq!(output.duration_ms, 1500);
    }
}
