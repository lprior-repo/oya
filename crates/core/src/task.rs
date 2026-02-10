//! Task type for workflow execution.
//!
//! Represents a unit of work in a workflow DAG.

#![deny(clippy::unwrap_used)]
#![deny(clippy::expect_used)]
#![deny(clippy::panic)]
#![warn(clippy::pedantic)]
#![warn(clippy::nursery)]
#![forbid(unsafe_code)]

use crate::Slug;
use serde::{Deserialize, Serialize};

/// Execution stages for a task.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Stage {
    /// Task is pending execution.
    Pending,
    /// Task is currently running.
    InProgress,
    /// Task has completed successfully.
    Completed,
    /// Task has failed.
    Failed,
    /// Task was rolled back.
    RolledBack,
}

impl Stage {
    /// Get all possible stages.
    #[must_use]
    pub const fn all() -> &'static [Self] {
        &[
            Self::Pending,
            Self::InProgress,
            Self::Completed,
            Self::Failed,
            Self::RolledBack,
        ]
    }
}

/// A task in a workflow.
///
/// Tasks are nodes in the workflow DAG with dependencies between them.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Task {
    /// Unique identifier for the task.
    pub id: Slug,
    /// Human-readable title.
    pub title: String,
    /// Detailed description.
    pub description: String,
    /// Current execution stage.
    pub current_stage: Stage,
}

impl Task {
    /// Create a new task.
    ///
    /// # Errors
    /// Returns an error if the slug is invalid.
    pub fn new(
        id: impl TryInto<Slug, Error = crate::Error>,
        title: impl Into<String>,
        description: impl Into<String>,
    ) -> Result<Self, crate::Error> {
        let id = id.try_into()?;
        Ok(Self {
            id,
            title: title.into(),
            description: description.into(),
            current_stage: Stage::Pending,
        })
    }

    /// Check if the task is complete.
    #[must_use]
    pub fn is_complete(&self) -> bool {
        self.current_stage == Stage::Completed
    }

    /// Mark the current stage as complete and advance to the next stage.
    pub const fn complete_current_stage(&mut self) {
        self.current_stage = match self.current_stage {
            Stage::Pending => Stage::InProgress,
            Stage::InProgress => Stage::Completed,
            Stage::Completed => Stage::Completed,
            Stage::Failed => Stage::Failed,
            Stage::RolledBack => Stage::RolledBack,
        };
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_task_creation() {
        let task = Task::new("task-1", "Test Task", "A test task");
        assert!(task.is_ok());
        let task = task.unwrap();
        assert_eq!(task.id.as_str(), "task-1");
        assert_eq!(task.title, "Test Task");
        assert_eq!(task.description, "A test task");
        assert_eq!(task.current_stage, Stage::Pending);
    }

    #[test]
    fn test_task_is_complete() {
        let mut task = Task::new("task-1", "Test", "Test").unwrap();
        assert!(!task.is_complete());
        task.current_stage = Stage::Completed;
        assert!(task.is_complete());
    }

    #[test]
    fn test_task_complete_current_stage() {
        let mut task = Task::new("task-1", "Test", "Test").unwrap();
        assert_eq!(task.current_stage, Stage::Pending);

        task.complete_current_stage();
        assert_eq!(task.current_stage, Stage::InProgress);

        task.complete_current_stage();
        assert_eq!(task.current_stage, Stage::Completed);

        task.complete_current_stage();
        assert_eq!(task.current_stage, Stage::Completed);
    }

    #[test]
    fn test_stage_all() {
        let stages = Stage::all();
        assert_eq!(stages.len(), 5);
        assert!(stages.contains(&Stage::Pending));
        assert!(stages.contains(&Stage::InProgress));
        assert!(stages.contains(&Stage::Completed));
        assert!(stages.contains(&Stage::Failed));
        assert!(stages.contains(&Stage::RolledBack));
    }
}
