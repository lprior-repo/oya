//! Task type for workflow execution.
//!
//! Represents a unit of work in a workflow DAG.

#![deny(clippy::unwrap_used)]
#![deny(clippy::expect_used)]
#![deny(clippy::panic)]
#![warn(clippy::pedantic)]
#![warn(clippy::nursery)]
#![forbid(unsafe_code)]
#![cfg_attr(test, allow(clippy::expect_used))]
#![cfg_attr(test, allow(clippy::unwrap_used))]
#![cfg_attr(test, allow(clippy::panic))]

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
    #[inline]
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
    /// Human-readable name.
    pub name: String,
    /// Detailed description.
    pub description: String,
    /// Current execution stage.
    current_stage: Stage,
}

impl Task {
    /// Create a new task.
    ///
    /// # Errors
    /// Returns an error if slug is invalid.
    #[inline]
    pub fn new(
        id: impl TryInto<Slug, Error = crate::Error>,
        name: impl Into<String>,
        description: impl Into<String>,
    ) -> Result<Self, crate::Error> {
        let id = id.try_into()?;
        Ok(Self {
            id,
            name: name.into(),
            description: description.into(),
            current_stage: Stage::Pending,
        })
    }

    /// Get the current stage of the task.
    #[must_use]
    #[inline]
    pub const fn stage(&self) -> &Stage {
        &self.current_stage
    }

    /// Check if the task is complete.
    #[must_use]
    #[inline]
    pub fn is_complete(&self) -> bool {
        self.current_stage == Stage::Completed
    }

    /// Transition the task to a new stage.
    ///
    /// # Errors
    /// Returns an error if the transition is invalid.
    #[inline]
    pub fn transition_to(&mut self, new_stage: Stage) -> Result<(), crate::Error> {
        // Define valid transitions
        match (&self.current_stage, &new_stage) {
            // Identity transition is always allowed (no-op)
            (s1, s2) if s1 == s2 => Ok(()),
            
            // Pending -> InProgress
            (Stage::Pending, Stage::InProgress) => {
                self.current_stage = new_stage;
                Ok(())
            }
            
            // InProgress -> Completed | Failed | RolledBack
            (Stage::InProgress, Stage::Completed | Stage::Failed | Stage::RolledBack) => {
                self.current_stage = new_stage;
                Ok(())
            }
            
            // Failed -> Pending (Retry) | RolledBack
            (Stage::Failed, Stage::Pending | Stage::RolledBack) => {
                self.current_stage = new_stage;
                Ok(())
            }
            
            // RolledBack -> Pending (Retry)
            (Stage::RolledBack, Stage::Pending) => {
                self.current_stage = new_stage;
                Ok(())
            }
            
            // Completed tasks are final (unless we explicitly allow re-opening, which we don't for now)
            (Stage::Completed, _) => Err(crate::Error::validation(
                "task_transition",
                format!("cannot transition from Completed to {new_stage:?}"),
            )),
            
            // Any other transition is invalid
            (from, to) => Err(crate::Error::validation(
                "task_transition",
                format!("invalid transition from {from:?} to {to:?}"),
            )),
        }
    }

    /// Mark the current stage as complete and advance to the next logical stage.
    ///
    /// This is a helper for simple workflows.
    #[inline]
    pub fn complete_current_stage(&mut self) {
        let next = match self.current_stage {
            Stage::Pending => Stage::InProgress,
            Stage::InProgress | Stage::Completed => Stage::Completed,
            Stage::Failed => Stage::Failed, // Stuck state
            Stage::RolledBack => Stage::RolledBack, // Stuck state
        };
        // We ignore the error here because complete_current_stage is "forceful" or legacy,
        // but ideally it should return Result. For now, we just apply if valid.
        let _ = self.transition_to(next);
    }
}

#[cfg(test)]
mod tests {

    use super::*;

    #[test]
    fn test_task_creation() {
        let task = Task::new("task-1", "Test Task", "A test task");
        assert!(task.is_ok());
        let task = match task {
            Ok(t) => t,
            Err(e) => panic!("Task creation failed: {e}"),
        };
        assert_eq!(task.id.as_str(), "task-1");
        assert_eq!(task.name, "Test Task");
        assert_eq!(task.description, "A test task");
        assert_eq!(task.current_stage, Stage::Pending);
    }

    #[test]
    fn test_task_is_complete() {
        let mut task = Task::new("task-1", "Test", "Test").unwrap();
        assert!(!task.is_complete());
        
        // Use transition_to instead of direct assignment
        task.transition_to(Stage::InProgress).unwrap();
        task.transition_to(Stage::Completed).unwrap();
        
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
    fn test_invalid_transitions() {
        let mut task = Task::new("task-1", "Test", "Test").unwrap();
        
        // Pending -> Completed is invalid (must go through InProgress)
        assert!(task.transition_to(Stage::Completed).is_err());
        
        // Pending -> Failed is invalid
        assert!(task.transition_to(Stage::Failed).is_err());
        
        task.transition_to(Stage::InProgress).unwrap();
        task.transition_to(Stage::Completed).unwrap();
        
        // Completed -> InProgress is invalid
        assert!(task.transition_to(Stage::InProgress).is_err());
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
