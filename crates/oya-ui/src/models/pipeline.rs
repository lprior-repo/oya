//! Pipeline types for UI
//!
//! These types mirror the types in oya-shared for use in the WASM frontend.
//! We duplicate them here because oya-shared uses rkyv which doesn't compile to WASM.

use serde::{Deserialize, Serialize};

/// Status of an individual pipeline stage
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
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

    /// Get color for the stage
    #[must_use]
    pub const fn color(&self) -> &'static str {
        match self {
            Self::Pending => "#6b7280", // gray-500
            Self::Running => "#3b82f6", // blue-500
            Self::Passed => "#22c55e",  // green-500
            Self::Failed => "#ef4444",  // red-500
            Self::Skipped => "#a855f7", // purple-500
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
#[derive(Debug, Clone, Serialize, Deserialize)]
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
}

/// Overall pipeline state
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
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
}

/// Event emitted when a stage changes state
#[derive(Debug, Clone, Serialize, Deserialize)]
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_stage_status_display() {
        assert_eq!(StageStatus::Pending.to_string(), "pending");
        assert_eq!(StageStatus::Running.to_string(), "running");
        assert_eq!(StageStatus::Passed.to_string(), "passed");
        assert_eq!(StageStatus::Failed.to_string(), "failed");
        assert_eq!(StageStatus::Skipped.to_string(), "skipped");
    }

    #[test]
    fn test_stage_status_terminal() {
        assert!(!StageStatus::Pending.is_terminal());
        assert!(!StageStatus::Running.is_terminal());
        assert!(StageStatus::Passed.is_terminal());
        assert!(StageStatus::Failed.is_terminal());
        assert!(StageStatus::Skipped.is_terminal());
    }

    #[test]
    fn test_pipeline_completion() {
        let state = PipelineState {
            stages: vec![
                StageInfo {
                    name: "a".to_string(),
                    gate: "Gate A".to_string(),
                    status: StageStatus::Passed,
                    retries: 1,
                    depends_on: vec![],
                    duration_ms: Some(100),
                    error: None,
                },
                StageInfo {
                    name: "b".to_string(),
                    gate: "Gate B".to_string(),
                    status: StageStatus::Pending,
                    retries: 1,
                    depends_on: vec!["a".to_string()],
                    duration_ms: None,
                    error: None,
                },
            ],
            current_stage: None,
            total_duration_ms: 100,
            all_passed: false,
            first_failure: None,
        };

        assert_eq!(state.completion_percent(), 50);
        assert_eq!(state.passed_count(), 1);
    }
}
