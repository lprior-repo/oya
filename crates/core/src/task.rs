//! Minimal Task type for oya-core.
//!
//! Note: The full Task implementation with pipeline status tracking
//! is in oya-pipeline crate. This is a minimal Task type for core workflows.

use crate::Slug;
use serde::{Deserialize, Serialize};

/// A minimal task in a workflow.
///
/// This is a simplified version for core workflow management.
/// For full task lifecycle management with pipeline stages,
/// use oya-pipeline::Task instead.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Task {
    /// Unique identifier for the task.
    pub id: Slug,
    /// Human-readable name.
    pub name: String,
    /// Detailed description.
    pub description: String,
}

impl Task {
    /// Create a new task.
    ///
    /// # Errors
    /// Returns an error if the slug is invalid.
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
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_task_new() {
        let task = Task::new("test-task", "Test Task", "A test task").expect("Failed to create task");
        assert_eq!(task.id.as_str(), "test-task");
        assert_eq!(task.name, "Test Task");
        assert_eq!(task.description, "A test task");
    }

    #[test]
    fn test_task_invalid_slug() {
        let result = Task::new("Invalid Task!", "Test", "Test");
        assert!(result.is_err());
    }
}
