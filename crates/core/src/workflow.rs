//! Workflow type for representing DAG-based task execution.

use crate::{Slug, Task};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use std::fmt;

/// A workflow represents a collection of tasks with dependencies.
///
/// Workflows are Directed Acyclic Graphs (DAGs) where tasks are nodes
/// and dependencies are edges.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Workflow {
    /// Unique identifier for the workflow.
    pub id: Slug,
    /// Human-readable name.
    pub name: String,
    /// Detailed description.
    pub description: String,
    /// Map of task IDs to tasks.
    pub tasks: HashMap<String, Task>,
    /// Dependencies between tasks (`task_id` -> set of dependent `task_ids`).
    pub dependencies: HashMap<String, HashSet<String>>,
    /// Creation timestamp.
    pub created_at: DateTime<Utc>,
    /// Last update timestamp.
    pub updated_at: DateTime<Utc>,
}

impl Workflow {
    /// Create a new workflow.
    ///
    /// # Errors
    /// Returns an error if the slug is invalid.
    pub fn new(
        id: impl TryInto<Slug, Error = crate::OyaError>,
        name: impl Into<String>,
        description: impl Into<String>,
    ) -> Result<Self, crate::OyaError> {
        let id = id.try_into()?;
        let now = Utc::now();
        Ok(Self {
            id,
            name: name.into(),
            description: description.into(),
            tasks: HashMap::new(),
            dependencies: HashMap::new(),
            created_at: now,
            updated_at: now,
        })
    }

    /// Add a task to the workflow.
    ///
    /// # Errors
    /// Returns an error if a task with the same ID already exists.
    pub fn add_task(&mut self, task: Task) -> Result<(), crate::OyaError> {
        let task_id = task.id.as_str().to_string();
        if self.tasks.contains_key(&task_id) {
            return Err(crate::OyaError::validation(
                "workflow",
                format!("task {task_id} already exists"),
            ));
        }
        self.tasks.insert(task_id, task);
        self.updated_at = Utc::now();
        Ok(())
    }

    /// Add a dependency between two tasks.
    ///
    /// `from_task_id` must complete before `to_task_id` can execute.
    /// In other words, `to_task_id` depends on `from_task_id`.
    ///
    /// # Errors
    /// Returns an error if:
    /// - Either task doesn't exist
    /// - The dependency would create a cycle
    /// - The dependency already exists
    pub fn add_dependency(
        &mut self,
        from_task_id: impl AsRef<str>,
        to_task_id: impl AsRef<str>,
    ) -> Result<(), crate::OyaError> {
        let from = from_task_id.as_ref();
        let to = to_task_id.as_ref();

        // Check both tasks exist
        if !self.tasks.contains_key(from) {
            return Err(crate::OyaError::not_found("task", from));
        }
        if !self.tasks.contains_key(to) {
            return Err(crate::OyaError::not_found("task", to));
        }

        // Check if dependency already exists (to depends on from)
        if self
            .dependencies
            .get(to)
            .is_some_and(|deps| deps.contains(from))
        {
            return Err(crate::OyaError::validation(
                "workflow",
                format!("dependency {from} -> {to} already exists"),
            ));
        }

        // Add dependency: to depends on from
        self.dependencies
            .entry(to.to_string())
            .or_default()
            .insert(from.to_string());

        // Check for cycles
        let cycle_check = self.check_cycles();

        // Remove dependency if cycle detected
        if cycle_check.is_err() {
            if let Some(deps) = self.dependencies.get_mut(to) {
                deps.remove(from);
                if deps.is_empty() {
                    self.dependencies.remove(to);
                }
            }
            return cycle_check;
        }

        self.updated_at = Utc::now();
        Ok(())
    }

    /// Check if the workflow has any cycles.
    ///
    /// # Errors
    /// Returns an error if a cycle is detected.
    fn check_cycles(&self) -> Result<(), crate::OyaError> {
        let mut visited = HashSet::new();
        let mut rec_stack = HashSet::new();

        for task_id in self.tasks.keys() {
            if !visited.contains(task_id)
                && self.dfs_check_cycle(task_id, &mut visited, &mut rec_stack)?
            {
                return Err(crate::OyaError::validation(
                    "workflow",
                    "cycle detected in task dependencies",
                ));
            }
        }

        Ok(())
    }

    /// DFS helper for cycle detection.
    fn dfs_check_cycle(
        &self,
        task_id: &str,
        visited: &mut HashSet<String>,
        rec_stack: &mut HashSet<String>,
    ) -> Result<bool, crate::OyaError> {
        visited.insert(task_id.to_string());
        rec_stack.insert(task_id.to_string());

        if let Some(deps) = self.dependencies.get(task_id) {
            for dep_id in deps {
                if !visited.contains(dep_id) {
                    if self.dfs_check_cycle(dep_id, visited, rec_stack)? {
                        return Ok(true);
                    }
                } else if rec_stack.contains(dep_id) {
                    return Ok(true);
                }
            }
        }

        rec_stack.remove(task_id);
        Ok(false)
    }

    /// Get tasks that are ready to execute (all dependencies satisfied).
    #[must_use]
    pub fn get_ready_tasks(&self) -> Vec<&Task> {
        self.tasks
            .iter()
            .filter(|(id, _task)| self.is_task_ready(id))
            .map(|(_, task)| task)
            .collect()
    }

    /// Check if a task is ready to execute.
    #[must_use]
    pub fn is_task_ready(&self, task_id: &str) -> bool {
        let Some(task) = self.tasks.get(task_id) else {
            return false;
        };

        if task.is_complete() {
            return false;
        }

        // Check if all dependencies of this task are complete
        self.dependencies.get(task_id).is_none_or(|deps| {
            deps.iter()
                .all(|dep_id| self.tasks.get(dep_id).is_some_and(Task::is_complete))
        })
    }

    /// Check if the workflow is complete (all tasks complete).
    ///
    /// An empty workflow is considered complete (vacuously true).
    #[must_use]
    pub fn is_complete(&self) -> bool {
        self.tasks.is_empty() || self.tasks.values().all(Task::is_complete)
    }

    /// Get the total number of tasks.
    #[must_use]
    pub fn len(&self) -> usize {
        self.tasks.len()
    }

    /// Check if the workflow has no tasks.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.tasks.is_empty()
    }

    /// Get a task by ID.
    #[must_use]
    pub fn get_task(&self, task_id: &str) -> Option<&Task> {
        self.tasks.get(task_id)
    }

    /// Get dependencies for a task.
    #[must_use]
    pub fn get_dependencies(&self, task_id: &str) -> Option<&HashSet<String>> {
        self.dependencies.get(task_id)
    }
}

impl fmt::Display for Workflow {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "Workflow(id={}, name={}, tasks={})",
            self.id,
            self.name,
            self.tasks.len()
        )
    }
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used)]
    #![allow(clippy::expect_used)]
    #![allow(clippy::panic)]
    #![allow(clippy::indexing_slicing)]
    #![allow(clippy::assertions_on_constants)]
    #![allow(clippy::uninlined_format_args)]
    #![allow(clippy::single_char_pattern)]
    #![allow(clippy::unnecessary_wraps)]
    #![allow(clippy::manual_let_else)]
    #![allow(clippy::struct_field_names)]
    #![allow(clippy::should_implement_trait)]
    #![allow(clippy::if_then_some_else_none)]
    #![allow(clippy::redundant_clone)]
    #![allow(clippy::map_or_none)]
    #![allow(clippy::missing_docs_in_private_items)]

    use super::*;

    #[test]
    fn test_workflow_new() -> Result<(), crate::OyaError> {
        let workflow = Workflow::new("test-workflow", "Test Workflow", "A test workflow")?;
        assert_eq!(workflow.id.as_str(), "test-workflow");
        assert_eq!(workflow.name, "Test Workflow");
        assert!(workflow.is_empty());
        Ok(())
    }

    #[test]
    fn test_workflow_add_task() -> Result<(), crate::OyaError> {
        let mut workflow = Workflow::new("test-workflow", "Test", "Description")?;
        let task = Task::new("task-1", "Task 1", "First task")?;
        assert!(workflow.add_task(task).is_ok());
        assert_eq!(workflow.len(), 1);
        Ok(())
    }

    #[test]
    fn test_workflow_add_duplicate_task() -> Result<(), crate::OyaError> {
        let mut workflow = Workflow::new("test-workflow", "Test", "Description")?;
        let task1 = Task::new("task-1", "Task 1", "First task")?;
        let task2 = Task::new("task-1", "Task 1", "Duplicate")?;

        assert!(workflow.add_task(task1).is_ok());
        assert!(workflow.add_task(task2).is_err());
        Ok(())
    }

    #[test]
    fn test_workflow_add_dependency() -> Result<(), crate::OyaError> {
        let mut workflow = Workflow::new("test-workflow", "Test", "Description")?;
        let task1 = Task::new("task-1", "Task 1", "First task")?;
        let task2 = Task::new("task-2", "Task 2", "Second task")?;

        workflow.add_task(task1)?;
        workflow.add_task(task2)?;

        assert!(workflow.add_dependency("task-1", "task-2").is_ok());
        Ok(())
    }

    #[test]
    fn test_workflow_add_dependency_nonexistent_task() -> Result<(), crate::OyaError> {
        let mut workflow = Workflow::new("test-workflow", "Test", "Description")?;
        let task = Task::new("task-1", "Task 1", "First task")?;
        workflow.add_task(task)?;

        let result = workflow.add_dependency("task-1", "task-nonexistent");
        assert!(result.is_err());
        Ok(())
    }

    #[test]
    fn test_workflow_cycle_detection() -> Result<(), crate::OyaError> {
        let mut workflow = Workflow::new("test-workflow", "Test", "Description")?;
        let task1 = Task::new("task-1", "Task 1", "First task")?;
        let task2 = Task::new("task-2", "Task 2", "Second task")?;

        workflow.add_task(task1)?;
        workflow.add_task(task2)?;

        // Create a cycle: task-1 -> task-2 -> task-1
        assert!(workflow.add_dependency("task-1", "task-2").is_ok());
        assert!(workflow.add_dependency("task-2", "task-1").is_err());
        assert!(workflow.get_dependencies("task-1").is_none());
        Ok(())
    }

    #[test]
    fn test_workflow_empty_is_complete() -> Result<(), crate::OyaError> {
        let workflow = Workflow::new("test-workflow", "Test", "Description")?;
        assert!(workflow.is_complete());
        Ok(())
    }

    #[test]
    fn test_workflow_is_task_ready() -> Result<(), crate::OyaError> {
        let mut workflow = Workflow::new("test-workflow", "Test", "Description")?;
        let task1 = Task::new("task-1", "Task 1", "First task")?;
        let task2 = Task::new("task-2", "Task 2", "Second task")?;

        workflow.add_task(task1)?;
        workflow.add_task(task2)?;
        workflow.add_dependency("task-1", "task-2")?;

        // task-2 depends on task-1, so it's not ready
        assert!(!workflow.is_task_ready("task-2"));
        // task-1 has no dependencies, so it's ready
        assert!(workflow.is_task_ready("task-1"));
        Ok(())
    }

    #[test]
    fn test_workflow_get_ready_tasks() -> Result<(), crate::OyaError> {
        let mut workflow = Workflow::new("test-workflow", "Test", "Description")?;
        let task1 = Task::new("task-1", "Task 1", "First task")?;
        let task2 = Task::new("task-2", "Task 2", "Second task")?;

        workflow.add_task(task1)?;
        workflow.add_task(task2)?;
        workflow.add_dependency("task-1", "task-2")?;

        let ready = workflow.get_ready_tasks();
        assert_eq!(ready.len(), 1);
        assert_eq!(ready[0].id.as_str(), "task-1");
        Ok(())
    }

    #[test]
    fn test_workflow_is_task_ready_unknown_task() -> Result<(), crate::OyaError> {
        let workflow = Workflow::new("test-workflow", "Test", "Description")?;
        assert!(!workflow.is_task_ready("missing-task"));
        Ok(())
    }

    #[test]
    fn test_workflow_get_ready_tasks_excludes_completed() -> Result<(), crate::OyaError> {
        let mut workflow = Workflow::new("test-workflow", "Test", "Description")?;
        let mut task = Task::new("task-1", "Task 1", "First task")?;

        for stage in crate::Stage::all() {
            task.current_stage = stage.clone();
            task.complete_current_stage();
        }

        workflow.add_task(task)?;
        let ready = workflow.get_ready_tasks();
        assert!(ready.is_empty());
        Ok(())
    }

    #[test]
    fn test_workflow_stress_ready_progression_large_chain() -> Result<(), crate::OyaError> {
        let mut workflow = Workflow::new("stress-workflow", "Stress", "Large chain")?;
        let task_count = 64;

        for i in 0..task_count {
            workflow.add_task(Task::new(
                format!("task-{i}"),
                format!("Task {i}"),
                "stress task",
            )?)?;
            if i > 0 {
                workflow.add_dependency(format!("task-{}", i - 1), format!("task-{i}"))?;
            }
        }

        for i in 0..task_count {
            let ready = workflow.get_ready_tasks();
            assert_eq!(ready.len(), 1);
            assert_eq!(ready[0].id.as_str(), format!("task-{i}"));

            let current = format!("task-{i}");
            let task = workflow
                .tasks
                .get_mut(&current)
                .ok_or_else(|| crate::OyaError::not_found("task", current.clone()))?;

            for stage in crate::Stage::all() {
                task.current_stage = stage.clone();
                task.complete_current_stage();
            }
        }

        assert!(workflow.get_ready_tasks().is_empty());
        assert!(workflow.is_complete());
        Ok(())
    }

    #[test]
    fn test_workflow_is_complete() -> Result<(), crate::OyaError> {
        let mut workflow = Workflow::new("test-workflow", "Test", "Description")?;
        let mut task = Task::new("task-1", "Task 1", "First task")?;

        workflow.add_task(task.clone())?;
        assert!(!workflow.is_complete());

        // Complete all stages
        for stage in crate::Stage::all() {
            task.current_stage = stage.clone();
            task.complete_current_stage();
        }
        workflow.tasks.insert(task.id.as_str().to_string(), task);
        assert!(workflow.is_complete());
        Ok(())
    }

    #[test]
    fn test_workflow_display() -> Result<(), crate::OyaError> {
        let workflow = Workflow::new("test-workflow", "Test Workflow", "Description")?;
        let s = format!("{}", workflow);
        assert!(s.contains("test-workflow"));
        assert!(s.contains("Test Workflow"));
        assert!(s.contains("0")); // task count
        Ok(())
    }
}
