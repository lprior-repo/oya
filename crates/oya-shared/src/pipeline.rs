//! Pipeline types for UI/backend communication
//!
//! These types represent pipeline stages and their execution state,
//! designed for efficient serialization across the Tauri IPC boundary.

use rkyv::{Archive, Deserialize as RkyvDeserialize, Serialize as RkyvSerialize};
use serde::{Deserialize, Serialize};

/// Status of an individual pipeline stage
#[derive(
    Debug,
    Clone,
    Copy,
    PartialEq,
    Eq,
    Serialize,
    Deserialize,
    Archive,
    RkyvSerialize,
    RkyvDeserialize,
    Default,
)]
#[serde(rename_all = "snake_case")]
pub enum StageStatus {
    /// Stage has not started yet
    #[default]
    Pending,
    /// Stage is currently running
    Running,
    /// Stage completed successfully
    Passed,
    /// Stage failed
    Failed,
    /// Stage was skipped (dependency failed)
    Skipped,
}

impl StageStatus {
    /// Check if stage is in a terminal state
    #[must_use]
    pub const fn is_terminal(&self) -> bool {
        matches!(self, Self::Passed | Self::Failed | Self::Skipped)
    }

    /// Check if stage passed
    #[must_use]
    pub const fn is_passed(&self) -> bool {
        matches!(self, Self::Passed)
    }

    /// Check if stage failed
    #[must_use]
    pub const fn is_failed(&self) -> bool {
        matches!(self, Self::Failed)
    }

    /// Get CSS class for styling
    #[must_use]
    pub const fn css_class(&self) -> &'static str {
        match self {
            Self::Pending => "stage-pending",
            Self::Running => "stage-running",
            Self::Passed => "stage-passed",
            Self::Failed => "stage-failed",
            Self::Skipped => "stage-skipped",
        }
    }

    /// Get icon character for display
    #[must_use]
    pub const fn icon(&self) -> &'static str {
        match self {
            Self::Pending => "○",
            Self::Running => "◉",
            Self::Passed => "●",
            Self::Failed => "✕",
            Self::Skipped => "⊘",
        }
    }
}

impl std::fmt::Display for StageStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Pending => write!(f, "pending"),
            Self::Running => write!(f, "running"),
            Self::Passed => write!(f, "passed"),
            Self::Failed => write!(f, "failed"),
            Self::Skipped => write!(f, "skipped"),
        }
    }
}

/// Information about a single pipeline stage
#[derive(Debug, Clone, Serialize, Deserialize, Archive, RkyvSerialize, RkyvDeserialize)]
pub struct StageInfo {
    /// Stage name (e.g., "implement", "unit-test")
    pub name: String,
    /// Human-readable gate description
    pub gate: String,
    /// Current execution status
    pub status: StageStatus,
    /// Number of retry attempts allowed
    pub retries: u32,
    /// Stages this depends on
    pub depends_on: Vec<String>,
    /// Duration in milliseconds (if completed)
    pub duration_ms: Option<u64>,
    /// Error message (if failed)
    pub error: Option<String>,
}

impl StageInfo {
    /// Create a new pending stage
    #[must_use]
    pub fn new(name: impl Into<String>, gate: impl Into<String>, retries: u32) -> Self {
        Self {
            name: name.into(),
            gate: gate.into(),
            status: StageStatus::Pending,
            retries,
            depends_on: Vec::new(),
            duration_ms: None,
            error: None,
        }
    }

    /// Add dependencies
    #[must_use]
    pub fn with_depends_on(self, deps: Vec<String>) -> Self {
        Self {
            depends_on: deps,
            ..self
        }
    }

    /// Set status
    #[must_use]
    pub fn with_status(self, status: StageStatus) -> Self {
        Self { status, ..self }
    }

    /// Check if this stage can run (all dependencies passed)
    #[must_use]
    pub fn can_run(&self, stages: &[StageInfo]) -> bool {
        if !matches!(self.status, StageStatus::Pending) {
            return false;
        }

        self.depends_on.iter().all(|dep| {
            stages
                .iter()
                .find(|s| s.name == *dep)
                .is_some_and(|s| s.status.is_passed())
        })
    }
}

/// Overall pipeline state
#[derive(Debug, Clone, Serialize, Deserialize, Archive, RkyvSerialize, RkyvDeserialize)]
pub struct PipelineState {
    /// All stages in execution order
    pub stages: Vec<StageInfo>,
    /// Index of currently running stage (if any)
    pub current_stage: Option<usize>,
    /// Total duration so far in milliseconds
    pub total_duration_ms: u64,
    /// Whether the entire pipeline passed
    pub all_passed: bool,
    /// Index of first failed stage (if any)
    pub first_failure: Option<usize>,
}

impl PipelineState {
    /// Create a new pipeline state from stage info
    #[must_use]
    pub fn new(stages: Vec<StageInfo>) -> Self {
        Self {
            stages,
            current_stage: None,
            total_duration_ms: 0,
            all_passed: false,
            first_failure: None,
        }
    }

    /// Get the standard 9-stage pipeline
    #[must_use]
    pub fn standard() -> Self {
        let stages = vec![
            StageInfo::new("implement", "Code compiles", 5),
            StageInfo::new("unit-test", "All tests pass", 3)
                .with_depends_on(vec!["implement".to_string()]),
            StageInfo::new("coverage", "80% coverage", 5)
                .with_depends_on(vec!["unit-test".to_string()]),
            StageInfo::new("lint", "Code formatted", 3)
                .with_depends_on(vec!["implement".to_string()]),
            StageInfo::new("static", "Static analysis passes", 3)
                .with_depends_on(vec!["lint".to_string()]),
            StageInfo::new("integration", "Integration tests pass", 3)
                .with_depends_on(vec!["unit-test".to_string(), "static".to_string()]),
            StageInfo::new("security", "No vulnerabilities", 2)
                .with_depends_on(vec!["coverage".to_string(), "static".to_string()]),
            StageInfo::new("review", "Code review passes", 3).with_depends_on(vec![
                "lint".to_string(),
                "static".to_string(),
                "unit-test".to_string(),
            ]),
            StageInfo::new("accept", "Ready for merge", 1).with_depends_on(vec![
                "integration".to_string(),
                "security".to_string(),
                "review".to_string(),
            ]),
        ];

        Self::new(stages)
    }

    /// Get number of passed stages
    #[must_use]
    pub fn passed_count(&self) -> usize {
        self.stages.iter().filter(|s| s.status.is_passed()).count()
    }

    /// Get number of failed stages
    #[must_use]
    pub fn failed_count(&self) -> usize {
        self.stages.iter().filter(|s| s.status.is_failed()).count()
    }

    /// Get completion percentage
    #[must_use]
    pub fn completion_percent(&self) -> u8 {
        if self.stages.is_empty() {
            return 100;
        }
        let terminal = self
            .stages
            .iter()
            .filter(|s| s.status.is_terminal())
            .count();
        ((terminal * 100) / self.stages.len()) as u8
    }

    /// Find stage by name
    #[must_use]
    pub fn find_stage(&self, name: &str) -> Option<&StageInfo> {
        self.stages.iter().find(|s| s.name == name)
    }

    /// Find mutable stage by name
    ///
    /// # Deprecated
    ///
    /// This method is deprecated in favor of immutable update patterns.
    /// Use `with_updated_stage` instead to create a new pipeline state.
    #[deprecated(
        since = "0.1.0",
        note = "Use `with_updated_stage` for immutable updates"
    )]
    pub fn find_stage_mut(&mut self, name: &str) -> Option<&mut StageInfo> {
        self.stages.iter_mut().find(|s| s.name == name)
    }

    /// Update a stage's status
    ///
    /// # Deprecated
    ///
    /// This method is deprecated in favor of immutable update patterns.
    /// Use `with_updated_stage` instead to create a new pipeline state.
    #[deprecated(
        since = "0.1.0",
        note = "Use `with_updated_stage` for immutable updates"
    )]
    #[allow(deprecated)]
    pub fn update_stage(
        &mut self,
        name: &str,
        status: StageStatus,
        duration_ms: Option<u64>,
        error: Option<String>,
    ) {
        if let Some(stage) = self.find_stage_mut(name) {
            stage.status = status;
            stage.duration_ms = duration_ms;
            stage.error = error;
        }

        // Update derived state
        self.all_passed = self.stages.iter().all(|s| s.status.is_passed());
        self.first_failure = self.stages.iter().position(|s| s.status.is_failed());
        self.total_duration_ms = self.stages.iter().filter_map(|s| s.duration_ms).sum();
    }

    /// Returns a new pipeline state with the specified stage updated
    #[must_use]
    pub fn with_updated_stage(
        self,
        name: &str,
        status: StageStatus,
        duration_ms: Option<u64>,
        error: Option<String>,
    ) -> Self {
        let mut stages = self.stages.clone();
        if let Some(stage) = stages.iter_mut().find(|s| s.name == name) {
            stage.status = status;
            stage.duration_ms = duration_ms;
            stage.error = error;
        }

        let all_passed = stages.iter().all(|s| s.status.is_passed());
        let first_failure = stages.iter().position(|s| s.status.is_failed());
        let total_duration_ms = stages.iter().filter_map(|s| s.duration_ms).sum();

        Self {
            stages,
            all_passed,
            first_failure,
            total_duration_ms,
            ..self
        }
    }

    /// Get the next stage that can run
    #[must_use]
    pub fn next_runnable(&self) -> Option<&StageInfo> {
        let stages = &self.stages;
        self.stages.iter().find(|s| s.can_run(stages))
    }
}

/// Event emitted when a stage changes state
#[derive(Debug, Clone, Serialize, Deserialize, Archive, RkyvSerialize, RkyvDeserialize)]
pub struct StageEvent {
    /// Stage name
    pub stage: String,
    /// New status
    pub status: StageStatus,
    /// Duration in milliseconds (if terminal)
    pub duration_ms: Option<u64>,
    /// Error message (if failed)
    pub error: Option<String>,
}

impl StageEvent {
    /// Create a started event
    #[must_use]
    pub fn started(stage: impl Into<String>) -> Self {
        Self {
            stage: stage.into(),
            status: StageStatus::Running,
            duration_ms: None,
            error: None,
        }
    }

    /// Create a passed event
    #[must_use]
    pub fn passed(stage: impl Into<String>, duration_ms: u64) -> Self {
        Self {
            stage: stage.into(),
            status: StageStatus::Passed,
            duration_ms: Some(duration_ms),
            error: None,
        }
    }

    /// Create a failed event
    #[must_use]
    pub fn failed(stage: impl Into<String>, duration_ms: u64, error: impl Into<String>) -> Self {
        Self {
            stage: stage.into(),
            status: StageStatus::Failed,
            duration_ms: Some(duration_ms),
            error: Some(error.into()),
        }
    }

    /// Create a skipped event
    #[must_use]
    pub fn skipped(stage: impl Into<String>) -> Self {
        Self {
            stage: stage.into(),
            status: StageStatus::Skipped,
            duration_ms: None,
            error: None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_stage_status_terminal() {
        assert!(!StageStatus::Pending.is_terminal());
        assert!(!StageStatus::Running.is_terminal());
        assert!(StageStatus::Passed.is_terminal());
        assert!(StageStatus::Failed.is_terminal());
        assert!(StageStatus::Skipped.is_terminal());
    }

    #[test]
    fn test_pipeline_state_standard() {
        let state = PipelineState::standard();
        assert_eq!(state.stages.len(), 9);
        assert_eq!(state.stages[0].name, "implement");
        assert_eq!(state.stages[8].name, "accept");
    }

    #[test]
    fn test_stage_can_run() {
        let state = PipelineState::standard();

        // implement has no deps, should be runnable
        assert!(state.stages[0].can_run(&state.stages));

        // unit-test depends on implement, not runnable yet
        assert!(!state.stages[1].can_run(&state.stages));

        // Mark implement as passed using immutable update
        let state = state.with_updated_stage("implement", StageStatus::Passed, Some(100), None);

        // Now unit-test should be runnable
        assert!(state.stages[1].can_run(&state.stages));
    }

    #[test]
    fn test_completion_percent() {
        let state = PipelineState::standard();
        assert_eq!(state.completion_percent(), 0);

        let state = state.with_updated_stage("implement", StageStatus::Passed, Some(100), None);
        assert_eq!(state.completion_percent(), 11); // 1/9 = 11%

        // Pass all stages
        let state = [
            "unit-test",
            "coverage",
            "lint",
            "static",
            "integration",
            "security",
            "review",
            "accept",
        ]
        .iter()
        .fold(state, |acc, stage| {
            acc.with_updated_stage(stage, StageStatus::Passed, Some(100), None)
        });

        assert_eq!(state.completion_percent(), 100);
    }

    #[test]
    fn test_serialization() {
        let state = PipelineState::standard();
        let json = serde_json::to_string(&state);
        assert!(json.is_ok());

        let restored: Result<PipelineState, _> = serde_json::from_str(&json.unwrap_or_default());
        assert!(restored.is_ok());
    }
}
