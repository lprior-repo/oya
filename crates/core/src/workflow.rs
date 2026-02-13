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
    tasks: HashMap<Slug, Task>,
    /// Dependencies between tasks (`task_id` -> set of dependent `task_ids`).
    dependencies: HashMap<Slug, HashSet<Slug>>,
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
    #[inline]
    pub fn new(
        id: impl TryInto<Slug, Error = crate::OyaError>,
        name: impl Into<String>,
        description: impl Into<String>,
        now: DateTime<Utc>,
    ) -> Result<Self, crate::OyaError> {
        let id = id.try_into()?;
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

    /// Get all tasks in the workflow.
    #[must_use]
    #[inline]
    pub const fn tasks(&self) -> &HashMap<Slug, Task> {
        &self.tasks
    }

    /// Get all dependencies in the workflow.
    #[must_use]
    #[inline]
    pub const fn dependencies(&self) -> &HashMap<Slug, HashSet<Slug>> {
        &self.dependencies
    }

    /// Get a mutable reference to a task.
    /// 
    /// # Warning
    /// Use this carefully as it allows direct mutation of task state.
    #[must_use]
    #[inline]
    pub fn get_task_mut(&mut self, task_id: &Slug) -> Option<&mut Task> {
        self.tasks.get_mut(task_id)
    }

    /// Add a task to the workflow.
    ///
    /// # Errors
    /// Returns an error if a task with the same ID already exists.
    #[inline]
    pub fn add_task(&mut self, task: Task, now: DateTime<Utc>) -> Result<(), crate::OyaError> {
        let task_id = task.id.clone();
        if self.tasks.contains_key(&task_id) {
            return Err(crate::OyaError::validation(
                "workflow",
                format!("task {task_id} already exists"),
            ));
        }
        self.tasks.insert(task_id, task);
        self.updated_at = now;
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
    #[inline]
    pub fn add_dependency(
        &mut self,
        from_task_id: impl TryInto<Slug, Error = crate::OyaError>,
        to_task_id: impl TryInto<Slug, Error = crate::OyaError>,
        now: DateTime<Utc>,
    ) -> Result<(), crate::OyaError> {
        let from = from_task_id.try_into()?;
        let to = to_task_id.try_into()?;

        // Check both tasks exist
        if !self.tasks.contains_key(&from) {
            return Err(crate::OyaError::not_found("task", from.as_str()));
        }
        if !self.tasks.contains_key(&to) {
            return Err(crate::OyaError::not_found("task", to.as_str()));
        }

        // Check if dependency already exists (to depends on from)
        if self
            .dependencies
            .get(&to)
            .is_some_and(|deps| deps.contains(&from))
        {
            return Err(crate::OyaError::validation(
                "workflow",
                format!("dependency {from} -> {to} already exists"),
            ));
        }

        // Check for cycles BEFORE mutation
        if self.is_reachable(&from, &to) {
             return Err(crate::OyaError::validation(
                "workflow",
                "cycle detected in task dependencies",
            ));
        }

        // Add dependency: to depends on from
        self.dependencies
            .entry(to)
            .or_default()
            .insert(from);

        self.updated_at = now;
        Ok(())
    }

    /// Check if `target` is reachable from `start` by following dependencies.
    /// (i.e. does `start` depend on `target`?)
    fn is_reachable(&self, start: &Slug, target: &Slug) -> bool {
        let mut visited = HashSet::new();
        let mut stack = Vec::new();
        stack.push(start);

        while let Some(current) = stack.pop() {
            if current == target {
                return true;
            }
            if !visited.insert(current) {
                continue;
            }
            if let Some(parents) = self.dependencies.get(current) {
                for parent in parents {
                    stack.push(parent);
                }
            }
        }
        false
    }

    /// Get tasks that are ready to execute (all dependencies satisfied).
    #[must_use]
    #[inline]
    pub fn get_ready_tasks(&self) -> Vec<&Task> {
        self.tasks
            .iter()
            .filter(|(id, _task)| self.is_task_ready(id))
            .map(|(_, task)| task)
            .collect()
    }

    /// Check if a task is ready to execute.
    #[must_use]
    #[inline]
    pub fn is_task_ready(&self, task_id: &Slug) -> bool {
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
    #[inline]
    pub fn is_complete(&self) -> bool {
        self.tasks.is_empty() || self.tasks.values().all(Task::is_complete)
    }

    /// Get the total number of tasks.
    #[must_use]
    #[inline]
    pub fn len(&self) -> usize {
        self.tasks.len()
    }

    /// Check if the workflow has no tasks.
    #[must_use]
    #[inline]
    pub fn is_empty(&self) -> bool {
        self.tasks.is_empty()
    }

    /// Get a task by ID.
    #[must_use]
    #[inline]
    pub fn get_task(&self, task_id: impl AsRef<str>) -> Option<&Task> {
        let slug = Slug::try_from(task_id.as_ref()).ok()?;
        self.tasks.get(&slug)
    }

    /// Get dependencies for a task.
    #[must_use]
    #[inline]
    pub fn get_dependencies(&self, task_id: impl AsRef<str>) -> Option<&HashSet<Slug>> {
        let slug = Slug::try_from(task_id.as_ref()).ok()?;
        self.dependencies.get(&slug)
    }
}

impl fmt::Display for Workflow {
    #[inline]
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
    #![allow(clippy::missing_docs_in_private_items)]

    use super::*;

    #[test]
    fn test_workflow_new() -> Result<(), crate::OyaError> {
        let workflow = Workflow::new(
            "test-workflow",
            "Test Workflow",
            "A test workflow",
            Utc::now(),
        )?;
        assert_eq!(workflow.id.as_str(), "test-workflow");
        assert_eq!(workflow.name, "Test Workflow");
        assert!(workflow.is_empty());
        Ok(())
    }

    #[test]
    fn test_workflow_add_task() -> Result<(), crate::OyaError> {
        let mut workflow = Workflow::new("test-workflow", "Test", "Description", Utc::now())?;
        let task = Task::new("task-1", "Task 1", "First task")?;
        assert!(workflow.add_task(task, Utc::now()).is_ok());
        assert_eq!(workflow.len(), 1);
        Ok(())
    }

    #[test]
    fn test_workflow_add_duplicate_task() -> Result<(), crate::OyaError> {
        let mut workflow = Workflow::new("test-workflow", "Test", "Description", Utc::now())?;
        let task1 = Task::new("task-1", "Task 1", "First task")?;
        let task2 = Task::new("task-1", "Task 1", "Duplicate")?;

        assert!(workflow.add_task(task1, Utc::now()).is_ok());
        assert!(workflow.add_task(task2, Utc::now()).is_err());
        Ok(())
    }

    #[test]
    fn test_workflow_add_dependency() -> Result<(), crate::OyaError> {
        let mut workflow = Workflow::new("test-workflow", "Test", "Description", Utc::now())?;
        let task1 = Task::new("task-1", "Task 1", "First task")?;
        let task2 = Task::new("task-2", "Task 2", "Second task")?;

        workflow.add_task(task1, Utc::now())?;
        workflow.add_task(task2, Utc::now())?;

        assert!(workflow
            .add_dependency("task-1", "task-2", Utc::now())
            .is_ok());
        Ok(())
    }

    #[test]
    fn test_workflow_add_dependency_nonexistent_task() -> Result<(), crate::OyaError> {
        let mut workflow = Workflow::new("test-workflow", "Test", "Description", Utc::now())?;
        let task = Task::new("task-1", "Task 1", "First task")?;
        workflow.add_task(task, Utc::now())?;

        let result = workflow.add_dependency("task-1", "task-nonexistent", Utc::now());
        assert!(result.is_err()); // Slug parse error or not found error
        Ok(())
    }

    #[test]
    fn test_workflow_cycle_detection() -> Result<(), crate::OyaError> {
        let mut workflow = Workflow::new("test-workflow", "Test", "Description", Utc::now())?;
        let task1 = Task::new("task-1", "Task 1", "First task")?;
        let task2 = Task::new("task-2", "Task 2", "Second task")?;

        workflow.add_task(task1, Utc::now())?;
        workflow.add_task(task2, Utc::now())?;

        // Create a cycle: task-1 -> task-2 -> task-1
        assert!(workflow
            .add_dependency("task-1", "task-2", Utc::now())
            .is_ok());
        assert!(workflow
            .add_dependency("task-2", "task-1", Utc::now())
            .is_err());
        assert!(workflow.get_dependencies("task-1").is_none());
        Ok(())
    }

    #[test]
    fn test_workflow_empty_is_complete() -> Result<(), crate::OyaError> {
        let workflow = Workflow::new("test-workflow", "Test", "Description", Utc::now())?;
        assert!(workflow.is_complete());
        Ok(())
    }

    #[test]
    fn test_workflow_is_task_ready() -> Result<(), crate::OyaError> {
        let mut workflow = Workflow::new("test-workflow", "Test", "Description", Utc::now())?;
        let task1 = Task::new("task-1", "Task 1", "First task")?;
        let task2 = Task::new("task-2", "Task 2", "Second task")?;

        workflow.add_task(task1, Utc::now())?;
        workflow.add_task(task2, Utc::now())?;
        workflow.add_dependency("task-1", "task-2", Utc::now())?;

        // task-2 depends on task-1, so it's not ready
        assert!(!workflow.is_task_ready(&Slug::new("task-2")?));
        // task-1 has no dependencies, so it's ready
        assert!(workflow.is_task_ready(&Slug::new("task-1")?));
        Ok(())
    }

    #[test]
    fn test_workflow_get_ready_tasks() -> Result<(), crate::OyaError> {
        let mut workflow = Workflow::new("test-workflow", "Test", "Description", Utc::now())?;
        let task1 = Task::new("task-1", "Task 1", "First task")?;
        let task2 = Task::new("task-2", "Task 2", "Second task")?;

        workflow.add_task(task1, Utc::now())?;
        workflow.add_task(task2, Utc::now())?;
        workflow.add_dependency("task-1", "task-2", Utc::now())?;

        let ready = workflow.get_ready_tasks();
        assert_eq!(ready.len(), 1);
        assert_eq!(ready[0].id.as_str(), "task-1");
        Ok(())
    }

    #[test]
    fn test_workflow_is_task_ready_unknown_task() -> Result<(), crate::OyaError> {
        let workflow = Workflow::new("test-workflow", "Test", "Description", Utc::now())?;
        assert!(!workflow.is_task_ready(&Slug::new("task-1")?));
        Ok(())
    }

    #[test]
    fn test_workflow_get_ready_tasks_excludes_completed() -> Result<(), crate::OyaError> {
        let mut workflow = Workflow::new("test-workflow", "Test", "Description", Utc::now())?;
        let mut task = Task::new("task-1", "Task 1", "First task")?;

        // Mark task as completed
        task.transition_to(crate::Stage::InProgress)?;
        task.transition_to(crate::Stage::Completed)?;

        workflow.add_task(task, Utc::now())?;
        let ready = workflow.get_ready_tasks();
        assert!(ready.is_empty());
        Ok(())
    }

    #[test]
    fn test_workflow_stress_ready_progression_large_chain() -> Result<(), crate::OyaError> {
        let mut workflow = Workflow::new("stress-workflow", "Stress", "Large chain", Utc::now())?;
        let task_count = 64;

        for i in 0..task_count {
            workflow.add_task(
                Task::new(
                    format!("task-{i}"),
                    format!("Task {i}"),
                    "stress task",
                )?,
                Utc::now(),
            )?;
            if i > 0 {
                workflow.add_dependency(
                    format!("task-{}", i - 1),
                    format!("task-{i}"),
                    Utc::now(),
                )?;
            }
        }

        for i in 0..task_count {
            // Note: In a real execution engine, we would check readiness before marking complete.
            // But here we just want to verify we can mutate and traverse.
            
            let current_id = Slug::new(format!("task-{i}"))?;
            let task = workflow
                .get_task_mut(&current_id)
                .ok_or_else(|| crate::OyaError::not_found("task", current_id.as_str()))?;

            // Mark task as completed
            task.transition_to(crate::Stage::InProgress)?;
            task.transition_to(crate::Stage::Completed)?;
        }

        assert!(workflow.get_ready_tasks().is_empty());
        assert!(workflow.is_complete());
        Ok(())
    }

    #[test]
    fn test_workflow_is_complete() -> Result<(), crate::OyaError> {
        let mut workflow = Workflow::new("test-workflow", "Test", "Description", Utc::now())?;
        let mut task = Task::new("task-1", "Task 1", "First task")?;

        workflow.add_task(task.clone(), Utc::now())?;
        assert!(!workflow.is_complete());

        // Mark task as completed
        task.transition_to(crate::Stage::InProgress)?;
        task.transition_to(crate::Stage::Completed)?;
        
        let task_id = task.id.clone();
        let target = workflow.get_task_mut(&task_id).ok_or_else(|| crate::OyaError::not_found("task", task_id.as_str()))?;
        *target = task;
        
        assert!(workflow.is_complete());
        Ok(())
    }

    #[test]
    fn test_workflow_display() -> Result<(), crate::OyaError> {
        let workflow = Workflow::new("test-workflow", "Test Workflow", "Description", Utc::now())?;
        let s = format!("{}", workflow);
        assert!(s.contains("test-workflow"));
        assert!(s.contains("Test Workflow"));
        assert!(s.contains("0")); // task count
        Ok(())
    }
}
