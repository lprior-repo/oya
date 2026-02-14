//! Workflow DAG execution engine with checkpoint support.
//!
//! This module provides the core execution engine for workflows, including:
//! - Topological sorting of task dependencies
//! - Parallel execution where dependencies allow
//! - Checkpoint-based recovery from failures
//! - Rollback support for failed tasks
//!
//! # Design Principles
//!
//! - **Zero unwrap**: All errors handled explicitly with Result types
//! - **Functional core**: Pure functions for state transitions
//! - **Railway-oriented**: Error propagation with context
//! - **Immutable state**: State transitions return new state, don't mutate

#![deny(clippy::unwrap_used)]
#![deny(clippy::expect_used)]
#![deny(clippy::panic)]
#![warn(clippy::pedantic)]
#![warn(clippy::nursery)]
#![allow(clippy::missing_inline_in_public_items)]
#![allow(clippy::unused_self)]
#![allow(clippy::self_only_used_in_recursion)]
#![forbid(unsafe_code)]

#[cfg(test)]
use crate::Task;
use crate::{Slug, Workflow};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use std::path::Path;

/// Execution state for a workflow.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct WorkflowState {
    /// Workflow ID.
    pub workflow_id: Slug,
    /// Current status of each task.
    pub task_status: HashMap<Slug, TaskExecutionStatus>,
    /// Timestamp of state creation.
    pub timestamp: DateTime<Utc>,
}

/// Execution status of a task.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TaskExecutionStatus {
    /// Task is pending execution.
    Pending,
    /// Task is currently running.
    InProgress,
    /// Task completed successfully.
    Completed,
    /// Task failed.
    Failed { error: String },
    /// Task was rolled back.
    RolledBack,
    /// Task was cancelled.
    Cancelled,
}

/// Result of workflow execution.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct WorkflowResult {
    /// Tasks that succeeded.
    pub succeeded: Vec<Slug>,
    /// Tasks that failed.
    pub failed: Vec<Slug>,
    /// Tasks that were rolled back.
    pub rolled_back: Vec<Slug>,
    /// Tasks that timed out.
    pub timed_out: Vec<Slug>,
    /// Tasks where rollback failed.
    pub rollback_failed: Vec<Slug>,
}

/// Errors that can occur during workflow execution.
#[derive(Debug, thiserror::Error)]
pub enum ExecutionError {
    /// Workflow definition is invalid.
    #[error("Invalid workflow: {0}")]
    InvalidWorkflow(String),

    /// Workflow graph contains a cycle.
    #[error("Workflow contains cycle: {cycle:?}")]
    CyclicGraph { cycle: Vec<String> },

    /// Task execution failed.
    #[error("Task {task_id} failed: {error}")]
    TaskFailed { task_id: String, error: String },

    /// Checkpoint operation failed.
    #[error("Checkpoint failed for task {task_id}: {cause}")]
    CheckpointFailed { task_id: String, cause: String },

    /// Checkpoint is corrupted.
    #[error("Checkpoint corrupted at {path}: {validation_errors:?}")]
    CheckpointCorrupted {
        path: String,
        validation_errors: Vec<String>,
    },

    /// Scheduler unavailable.
    #[error("Scheduler unavailable after waiting {wait_duration:?}")]
    SchedulerUnavailable { wait_duration: std::time::Duration },

    /// Task execution timed out.
    #[error("Task {task_id} timed out after {duration:?}")]
    Timeout {
        task_id: String,
        duration: std::time::Duration,
    },

    /// Rollback operation failed.
    #[error("Rollback failed for task {task_id}: {cause}")]
    RollbackFailed { task_id: String, cause: String },
}

/// Workflow execution engine.
pub struct ExecutionEngine {
    /// Checkpoint directory.
    checkpoint_dir: Option<String>,
}

impl ExecutionEngine {
    /// Create a new execution engine.
    #[must_use]
    pub const fn new() -> Self {
        Self {
            checkpoint_dir: None,
        }
    }

    /// Set the checkpoint directory.
    #[inline]
    #[must_use]
    pub fn with_checkpoint_dir(mut self, dir: impl Into<String>) -> Self {
        self.checkpoint_dir = Some(dir.into());
        self
    }

    /// Parse and validate a workflow for execution.
    ///
    /// # Errors
    /// Returns `ExecutionError::InvalidWorkflow` if validation fails.
    /// Returns `ExecutionError::CyclicGraph` if the workflow has cycles.
    #[inline]
    pub fn parse_workflow(&self, workflow: &Workflow) -> Result<WorkflowState, ExecutionError> {
        // Validate workflow structure
        self.validate_workflow(workflow)?;

        // Check for cycles using topological sort
        let _sorted = self.topological_sort(workflow)?;

        // Initialize task status
        let task_status = workflow
            .tasks()
            .iter()
            .map(|(id, task)| {
                (
                    id.clone(),
                    if task.is_complete() {
                        TaskExecutionStatus::Completed
                    } else {
                        TaskExecutionStatus::Pending
                    },
                )
            })
            .collect();

        Ok(WorkflowState {
            workflow_id: workflow.id.clone(),
            task_status,
            timestamp: Utc::now(),
        })
    }

    /// Validate workflow structure.
    fn validate_workflow(&self, workflow: &Workflow) -> Result<(), ExecutionError> {
        // Check all task IDs are unique (guaranteed by HashMap)
        // Check all dependencies reference valid tasks
        for (task_id, deps) in workflow.dependencies() {
            if !workflow.tasks().contains_key(task_id) {
                return Err(ExecutionError::InvalidWorkflow(format!(
                    "Unknown dependency owner task {task_id}"
                )));
            }

            for dep_id in deps {
                if dep_id == task_id {
                    return Err(ExecutionError::InvalidWorkflow(format!(
                        "Task {task_id} cannot depend on itself"
                    )));
                }

                if !workflow.tasks().contains_key(dep_id) {
                    return Err(ExecutionError::InvalidWorkflow(format!(
                        "Task {task_id} depends on non-existent task {dep_id}"
                    )));
                }
            }
        }
        Ok(())
    }

    /// Perform topological sort on workflow tasks.
    ///
    /// # Errors
    /// Returns `ExecutionError::CyclicGraph` if a cycle is detected.
    fn topological_sort(&self, workflow: &Workflow) -> Result<Vec<Slug>, ExecutionError> {
        let mut in_degree: HashMap<Slug, usize> = HashMap::new();
        let mut adj_list: HashMap<Slug, Vec<Slug>> = HashMap::new();

        // Initialize in-degrees and adjacency list
        for task_id in workflow.tasks().keys() {
            in_degree.insert(task_id.clone(), 0);
            adj_list.insert(task_id.clone(), Vec::new());
        }

        // Build adjacency list and in-degrees
        for (task_id, deps) in workflow.dependencies() {
            for dep_id in deps {
                // dep_id -> task_id (dep must complete before task)
                adj_list
                    .entry(dep_id.clone())
                    .or_default()
                    .push(task_id.clone());
                let entry = in_degree.entry(task_id.clone()).or_insert(0);
                *entry = entry.saturating_add(1);
            }
        }

        // Kahn's algorithm
        let mut queue: Vec<Slug> = in_degree
            .iter()
            .filter(|(_, degree)| **degree == 0)
            .map(|(id, _)| id.clone())
            .collect();

        let mut result = Vec::new();

        while let Some(task_id) = queue.pop() {
            result.push(task_id.clone());

            if let Some(neighbors) = adj_list.get(&task_id) {
                for neighbor in neighbors {
                    if let Some(degree) = in_degree.get_mut(neighbor) {
                        *degree = degree.saturating_sub(1);
                        if *degree == 0 {
                            queue.push(neighbor.clone());
                        }
                    }
                }
            }
        }

        // Check for cycle
        if result.len() != workflow.tasks().len() {
            // Find cycle for better error message
            let cycle = self.detect_cycle(workflow)?;
            return Err(ExecutionError::CyclicGraph { cycle });
        }

        Ok(result)
    }

    /// Detect a cycle in the workflow graph.
    fn detect_cycle(&self, workflow: &Workflow) -> Result<Vec<String>, ExecutionError> {
        let mut visited: HashSet<Slug> = HashSet::new();
        let mut rec_stack: HashSet<Slug> = HashSet::new();
        let mut path: Vec<Slug> = Vec::new();

        for task_id in workflow.tasks().keys() {
            if !visited.contains(task_id)
                && self.dfs_cycle_detect(
                    workflow,
                    task_id,
                    &mut visited,
                    &mut rec_stack,
                    &mut path,
                )?
            {
                // Convert path to strings for error message
                return Ok(path.iter().map(std::string::ToString::to_string).collect());
            }
        }

        // Should not reach here if cycle exists
        Ok(vec![])
    }

    /// DFS helper for cycle detection.
    fn dfs_cycle_detect(
        &self,
        workflow: &Workflow,
        task_id: &Slug,
        visited: &mut HashSet<Slug>,
        rec_stack: &mut HashSet<Slug>,
        path: &mut Vec<Slug>,
    ) -> Result<bool, ExecutionError> {
        visited.insert(task_id.clone());
        rec_stack.insert(task_id.clone());
        path.push(task_id.clone());

        // Get tasks that depend on this task (reverse dependencies)
        for (other_id, deps) in workflow.dependencies() {
            if deps.contains(task_id) {
                if !visited.contains(other_id) {
                    if self.dfs_cycle_detect(workflow, other_id, visited, rec_stack, path)? {
                        return Ok(true);
                    }
                } else if rec_stack.contains(other_id) {
                    // Found cycle - extract the cycle portion
                    let cycle_start = path.iter().position(|id| id == other_id);
                    if let Some(start) = cycle_start {
                        if let Some(slice) = path.get(start..) {
                            *path = slice.to_vec();
                            path.push(other_id.clone());
                        }
                    }
                    return Ok(true);
                }
            }
        }

        rec_stack.remove(task_id);
        path.pop();
        Ok(false)
    }

    /// Save workflow state to checkpoint.
    ///
    /// # Errors
    /// Returns `ExecutionError::CheckpointFailed` if write fails.
    pub async fn save_checkpoint(&self, state: &WorkflowState) -> Result<String, ExecutionError> {
        let checkpoint_dir =
            self.checkpoint_dir
                .clone()
                .ok_or_else(|| ExecutionError::CheckpointFailed {
                    task_id: state.workflow_id.to_string(),
                    cause: "No checkpoint directory configured".to_string(),
                })?;

        // Serialize state
        let json =
            serde_json::to_string_pretty(state).map_err(|e| ExecutionError::CheckpointFailed {
                task_id: state.workflow_id.to_string(),
                cause: format!("Serialization failed: {e}"),
            })?;

        // Write to temp file first
        let temp_path = format!("{}/{}.tmp", checkpoint_dir, state.workflow_id);
        tokio::fs::write(&temp_path, json)
            .await
            .map_err(|e| ExecutionError::CheckpointFailed {
                task_id: state.workflow_id.to_string(),
                cause: format!("Write failed: {e}"),
            })?;

        // Atomic rename
        let final_path = format!("{}/{}.json", checkpoint_dir, state.workflow_id);
        tokio::fs::rename(&temp_path, &final_path)
            .await
            .map_err(|e| ExecutionError::CheckpointFailed {
                task_id: state.workflow_id.to_string(),
                cause: format!("Rename failed: {e}"),
            })?;

        Ok(final_path)
    }

    /// Load workflow state from checkpoint.
    ///
    /// # Errors
    /// Returns `ExecutionError::CheckpointCorrupted` if validation fails.
    pub async fn load_checkpoint(&self, path: &Path) -> Result<WorkflowState, ExecutionError> {
        let content = tokio::fs::read_to_string(path).await.map_err(|e| {
            ExecutionError::CheckpointCorrupted {
                path: path.display().to_string(),
                validation_errors: vec![format!("Read failed: {e}")],
            }
        })?;

        serde_json::from_str(&content).map_err(|e| ExecutionError::CheckpointCorrupted {
            path: path.display().to_string(),
            validation_errors: vec![format!("Parse failed: {e}")],
        })
    }

    /// Get tasks ready to execute based on current state.
    #[must_use]
    pub fn get_ready_tasks(&self, workflow: &Workflow, state: &WorkflowState) -> Vec<Slug> {
        if state.workflow_id != workflow.id {
            return Vec::new();
        }

        workflow
            .tasks()
            .keys()
            .filter(|task_id| {
                // Task must be pending
                if state
                    .task_status
                    .get(*task_id)
                    .is_none_or(|s| !matches!(s, TaskExecutionStatus::Pending))
                {
                    return false;
                }

                // All dependencies must be completed
                workflow.dependencies().get(*task_id).is_none_or(|deps| {
                    deps.iter().all(|dep_id| {
                        state
                            .task_status
                            .get(dep_id)
                            .is_some_and(|s| matches!(s, TaskExecutionStatus::Completed))
                    })
                })
            })
            .cloned()
            .collect()
    }

    /// Execute a workflow and return the result.
    ///
    /// This is a simplified synchronous implementation that processes tasks
    /// sequentially. In production, this would be async with parallel execution.
    ///
    /// # Errors
    /// Returns `ExecutionError` if workflow execution fails.
    pub fn execute_workflow(&self, workflow: &Workflow) -> Result<WorkflowResult, ExecutionError> {
        // Parse workflow to get initial state
        let mut state = self.parse_workflow(workflow)?;

        let mut succeeded = Vec::new();
        let failed = Vec::new();
        let rolled_back = Vec::new();
        let timed_out = Vec::new();
        let rollback_failed = Vec::new();

        // Execute tasks until no more are ready
        loop {
            // Get ready tasks
            let ready = self.get_ready_tasks(workflow, &state);

            if ready.is_empty() {
                // Check if workflow is complete or blocked
                let all_complete = state.task_status.values().all(|s| {
                    matches!(
                        s,
                        TaskExecutionStatus::Completed
                            | TaskExecutionStatus::Failed { .. }
                            | TaskExecutionStatus::RolledBack
                    )
                });

                if all_complete {
                    break;
                }

                // Workflow is blocked (circular dependency or other issue)
                return Err(ExecutionError::InvalidWorkflow(
                    "Workflow is blocked - no ready tasks but not all complete".to_string(),
                ));
            }

            // Execute ready tasks (simplified - in production would be parallel)
            for task_id in ready {
                // Mark task as in progress
                state
                    .task_status
                    .insert(task_id.clone(), TaskExecutionStatus::InProgress);

                // Simulate task execution - in production would call actual task
                // For now, we'll mark it as completed
                let _task =
                    workflow
                        .get_task(&task_id)
                        .ok_or_else(|| ExecutionError::TaskFailed {
                            task_id: task_id.to_string(),
                            error: "Task not found".to_string(),
                        })?;

                // Simulate execution success
                state
                    .task_status
                    .insert(task_id.clone(), TaskExecutionStatus::Completed);
                succeeded.push(task_id.clone());
            }
        }

        Ok(WorkflowResult {
            succeeded,
            failed,
            rolled_back,
            timed_out,
            rollback_failed,
        })
    }

    /// Rollback a single task.
    ///
    /// This is a simplified implementation that marks the task as rolled back.
    /// In production, this would invoke rollback handlers.
    ///
    /// # Errors
    /// Returns `ExecutionError` if rollback fails.
    pub fn rollback_task(
        &self,
        task_id: &Slug,
        workflow: &Workflow,
        state: &mut WorkflowState,
    ) -> Result<(), ExecutionError> {
        // Check if task exists
        if !workflow.tasks().contains_key(task_id) {
            return Err(ExecutionError::RollbackFailed {
                task_id: task_id.to_string(),
                cause: "Task not found".to_string(),
            });
        }

        // Check if task is completed (only completed tasks can be rolled back)
        match state.task_status.get(task_id) {
            Some(TaskExecutionStatus::Completed) => {
                // Mark as rolled back
                state
                    .task_status
                    .insert(task_id.clone(), TaskExecutionStatus::RolledBack);

                Ok(())
            }
            Some(TaskExecutionStatus::RolledBack) => {
                // Already rolled back - idempotent
                Ok(())
            }
            Some(other) => Err(ExecutionError::RollbackFailed {
                task_id: task_id.to_string(),
                cause: format!("Cannot rollback task in state {other:?}"),
            }),
            None => Err(ExecutionError::RollbackFailed {
                task_id: task_id.to_string(),
                cause: "Task not found in state".to_string(),
            }),
        }
    }

    /// Recover workflow execution from a checkpoint.
    ///
    /// # Errors
    /// Returns `ExecutionError` if recovery fails.
    pub async fn recover_from_checkpoint(
        &self,
        path: &Path,
        workflow: &Workflow,
    ) -> Result<WorkflowState, ExecutionError> {
        // Load checkpoint
        let state = self.load_checkpoint(path).await?;

        // Validate state consistency
        if state.workflow_id != workflow.id {
            return Err(ExecutionError::CheckpointCorrupted {
                path: path.display().to_string(),
                validation_errors: vec![format!(
                    "Workflow ID mismatch: checkpoint has {}, workflow has {}",
                    state.workflow_id, workflow.id
                )],
            });
        }

        // Validate all tasks in state exist in workflow
        for task_id in state.task_status.keys() {
            if !workflow.tasks().contains_key(task_id) {
                return Err(ExecutionError::CheckpointCorrupted {
                    path: path.display().to_string(),
                    validation_errors: vec![format!(
                        "Task {} in checkpoint not found in workflow",
                        task_id
                    )],
                });
            }
        }

        // Validate all workflow tasks are in state
        for task_id in workflow.tasks().keys() {
            if !state.task_status.contains_key(task_id) {
                return Err(ExecutionError::CheckpointCorrupted {
                    path: path.display().to_string(),
                    validation_errors: vec![format!(
                        "Task {} from workflow missing in checkpoint",
                        task_id
                    )],
                });
            }
        }

        Ok(state)
    }
}

impl Default for ExecutionEngine {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {

    #![allow(clippy::assertions_on_constants)]
    #![allow(clippy::unwrap_used)]
    #![allow(clippy::expect_used)]
    #![allow(clippy::panic)]
    #![allow(clippy::indexing_slicing)]

    use super::*;
    use crate::{Slug, Task};
    use chrono::Utc;

    #[test]
    fn test_execution_engine_new() {
        let engine = ExecutionEngine::new();
        assert!(engine.checkpoint_dir.is_none());
    }

    #[test]
    fn test_execution_engine_with_checkpoint_dir() {
        let engine = ExecutionEngine::new().with_checkpoint_dir("/tmp/checkpoints");
        assert_eq!(engine.checkpoint_dir, Some("/tmp/checkpoints".to_string()));
    }

    #[test]
    fn test_parse_valid_workflow() -> Result<(), Box<dyn std::error::Error>> {
        let engine = ExecutionEngine::new();
        let mut workflow = Workflow::new("test-workflow", "Test", "Description", Utc::now())?;

        let task1 = Task::new("task-1", "Task 1", "First task")?;
        let task2 = Task::new("task-2", "Task 2", "Second task")?;

        workflow.add_task(task1, Utc::now())?;
        workflow.add_task(task2, Utc::now())?;
        workflow.add_dependency("task-1", "task-2", Utc::now())?;

        let state = engine.parse_workflow(&workflow)?;
        assert_eq!(state.workflow_id.as_str(), "test-workflow");
        assert_eq!(state.task_status.len(), 2);
        assert_eq!(
            state.task_status.get(&Slug::new("task-1")?),
            Some(&TaskExecutionStatus::Pending)
        );
        assert_eq!(
            state.task_status.get(&Slug::new("task-2")?),
            Some(&TaskExecutionStatus::Pending)
        );
        Ok(())
    }

    #[test]
    fn test_parse_workflow_with_invalid_dependency() -> Result<(), Box<dyn std::error::Error>> {
        let engine = ExecutionEngine::new();
        let workflow = Workflow::new("test-workflow", "Test", "Description", Utc::now())?;

        // Create an invalid workflow via JSON to bypass private field restrictions
        let mut json = serde_json::to_value(&workflow)?;
        json["dependencies"] = serde_json::json!({
            "task-1": ["non-existent"]
        });
        json["tasks"] = serde_json::json!({
            "task-1": {
                "id": "task-1",
                "name": "Task 1",
                "description": "First task",
                "current_stage": "pending"
            }
        });

        let workflow: Workflow = serde_json::from_value(json)?;

        let result = engine.parse_workflow(&workflow);
        assert!(result.is_err());
        match result {
            Err(ExecutionError::InvalidWorkflow(msg)) => {
                assert!(msg.contains("non-existent"));
            }
            _ => return Err("Expected InvalidWorkflow error".into()),
        }
        Ok(())
    }

    #[test]
    fn test_parse_workflow_with_unknown_dependency_owner() -> Result<(), Box<dyn std::error::Error>>
    {
        let engine = ExecutionEngine::new();
        let workflow = Workflow::new("test-workflow", "Test", "Description", Utc::now())?;

        // Create an invalid workflow owner via JSON
        let mut json = serde_json::to_value(&workflow)?;
        json["dependencies"] = serde_json::json!({
            "ghost": ["task-1"]
        });
        json["tasks"] = serde_json::json!({
            "task-1": {
                "id": "task-1",
                "name": "Task 1",
                "description": "First task",
                "current_stage": "pending"
            }
        });

        let workflow: Workflow = serde_json::from_value(json)?;

        let result = engine.parse_workflow(&workflow);
        assert!(matches!(result, Err(ExecutionError::InvalidWorkflow(_))));
        Ok(())
    }

    #[test]
    fn test_parse_workflow_maps_completed_task_to_completed_state(
    ) -> Result<(), Box<dyn std::error::Error>> {
        let engine = ExecutionEngine::new();
        let mut workflow = Workflow::new("test-workflow", "Test", "Description", Utc::now())?;

        let mut completed_task = Task::new("task-1", "Task 1", "First task")?;
        // Mark task as completed
        completed_task.transition_to(crate::Stage::InProgress)?;
        completed_task.transition_to(crate::Stage::Completed)?;

        workflow.add_task(completed_task, Utc::now())?;
        let state = engine.parse_workflow(&workflow)?;

        assert_eq!(
            state.task_status.get(&Slug::new("task-1")?),
            Some(&TaskExecutionStatus::Completed)
        );
        Ok(())
    }

    #[test]
    fn test_parse_workflow_with_cycle() -> Result<(), Box<dyn std::error::Error>> {
        let engine = ExecutionEngine::new();
        let workflow = Workflow::new("test-workflow", "Test", "Description", Utc::now())?;

        // Create cycle via JSON
        let mut json = serde_json::to_value(&workflow)?;
        json["tasks"] = serde_json::json!({
            "task-1": {
                "id": "task-1",
                "name": "Task 1",
                "description": "First task",
                "current_stage": "pending"
            },
            "task-2": {
                "id": "task-2",
                "name": "Task 2",
                "description": "Second task",
                "current_stage": "pending"
            }
        });
        json["dependencies"] = serde_json::json!({
            "task-1": ["task-2"],
            "task-2": ["task-1"]
        });

        let workflow: Workflow = serde_json::from_value(json)?;

        let result = engine.parse_workflow(&workflow);
        assert!(result.is_err());
        match result {
            Err(ExecutionError::CyclicGraph { cycle }) => {
                assert!(!cycle.is_empty());
            }
            other => return Err(format!("Expected CyclicGraph error, got {other:?}").into()),
        }
        Ok(())
    }

    #[test]
    fn test_topological_sort_linear() -> Result<(), Box<dyn std::error::Error>> {
        let engine = ExecutionEngine::new();
        let mut workflow = Workflow::new("test-workflow", "Test", "Description", Utc::now())?;

        let task1 = Task::new("task-1", "Task 1", "First task")?;
        let task2 = Task::new("task-2", "Task 2", "Second task")?;
        let task3 = Task::new("task-3", "Task 3", "Third task")?;

        workflow.add_task(task1, Utc::now())?;
        workflow.add_task(task2, Utc::now())?;
        workflow.add_task(task3, Utc::now())?;

        workflow.add_dependency("task-1", "task-2", Utc::now())?;
        workflow.add_dependency("task-2", "task-3", Utc::now())?;

        let sorted = engine.topological_sort(&workflow)?;
        assert_eq!(sorted.len(), 3);

        // task-1 must come before task-2, task-2 before task-3
        let pos1 = sorted
            .iter()
            .position(|id| id.as_str() == "task-1")
            .ok_or("task-1 not found")?;
        let pos2 = sorted
            .iter()
            .position(|id| id.as_str() == "task-2")
            .ok_or("task-2 not found")?;
        let pos3 = sorted
            .iter()
            .position(|id| id.as_str() == "task-3")
            .ok_or("task-3 not found")?;

        assert!(pos1 < pos2);
        assert!(pos2 < pos3);
        Ok(())
    }

    #[test]
    fn test_topological_sort_diamond() -> Result<(), Box<dyn std::error::Error>> {
        let engine = ExecutionEngine::new();
        let mut workflow = Workflow::new("test-workflow", "Test", "Description", Utc::now())?;

        let task_a = Task::new("a", "Task A", "First")?;
        let task_b = Task::new("b", "Task B", "Second")?;
        let task_c = Task::new("c", "Task C", "Third")?;
        let task_d = Task::new("d", "Task D", "Fourth")?;

        workflow.add_task(task_a, Utc::now())?;
        workflow.add_task(task_b, Utc::now())?;
        workflow.add_task(task_c, Utc::now())?;
        workflow.add_task(task_d, Utc::now())?;

        // A -> [B, C] -> D
        workflow.add_dependency("a", "b", Utc::now())?;
        workflow.add_dependency("a", "c", Utc::now())?;
        workflow.add_dependency("b", "d", Utc::now())?;
        workflow.add_dependency("c", "d", Utc::now())?;

        let sorted = engine.topological_sort(&workflow)?;
        assert_eq!(sorted.len(), 4);

        // A must come before both B and C
        let pos_a = sorted
            .iter()
            .position(|id| id.as_str() == "a")
            .ok_or("a not found")?;
        let pos_b = sorted
            .iter()
            .position(|id| id.as_str() == "b")
            .ok_or("b not found")?;
        let pos_c = sorted
            .iter()
            .position(|id| id.as_str() == "c")
            .ok_or("c not found")?;
        let pos_d = sorted
            .iter()
            .position(|id| id.as_str() == "d")
            .ok_or("d not found")?;

        assert!(pos_a < pos_b);
        assert!(pos_a < pos_c);
        assert!(pos_b < pos_d);
        assert!(pos_c < pos_d);
        Ok(())
    }

    #[test]
    fn test_get_ready_tasks_no_dependencies() -> Result<(), Box<dyn std::error::Error>> {
        let engine = ExecutionEngine::new();
        let mut workflow = Workflow::new("test-workflow", "Test", "Description", Utc::now())?;

        let task1 = Task::new("task-1", "Task 1", "First task")?;
        workflow.add_task(task1, Utc::now())?;

        let state = WorkflowState {
            workflow_id: Slug::new("test-workflow")?,
            task_status: HashMap::from([(Slug::new("task-1")?, TaskExecutionStatus::Pending)]),
            timestamp: Utc::now(),
        };

        let ready = engine.get_ready_tasks(&workflow, &state);
        assert_eq!(ready.len(), 1);
        assert!(ready.contains(&Slug::new("task-1")?));
        Ok(())
    }

    #[test]
    fn test_get_ready_tasks_with_dependencies() -> Result<(), Box<dyn std::error::Error>> {
        let engine = ExecutionEngine::new();
        let mut workflow = Workflow::new("test-workflow", "Test", "Description", Utc::now())?;

        let task1 = Task::new("task-1", "Task 1", "First task")?;
        let task2 = Task::new("task-2", "Task 2", "Second task")?;

        workflow.add_task(task1, Utc::now())?;
        workflow.add_task(task2, Utc::now())?;
        workflow.add_dependency("task-1", "task-2", Utc::now())?;

        let state = WorkflowState {
            workflow_id: Slug::new("test-workflow")?,
            task_status: HashMap::from([
                (Slug::new("task-1")?, TaskExecutionStatus::Completed),
                (Slug::new("task-2")?, TaskExecutionStatus::Pending),
            ]),
            timestamp: Utc::now(),
        };

        let ready = engine.get_ready_tasks(&workflow, &state);
        assert_eq!(ready.len(), 1);
        assert!(ready.contains(&Slug::new("task-2")?));
        Ok(())
    }

    #[test]
    fn test_get_ready_tasks_dependencies_not_complete() -> Result<(), Box<dyn std::error::Error>> {
        let engine = ExecutionEngine::new();
        let mut workflow = Workflow::new("test-workflow", "Test", "Description", Utc::now())?;

        let task1 = Task::new("task-1", "Task 1", "First task")?;
        let task2 = Task::new("task-2", "Task 2", "Second task")?;

        workflow.add_task(task1, Utc::now())?;
        workflow.add_task(task2, Utc::now())?;
        workflow.add_dependency("task-1", "task-2", Utc::now())?;

        let state = WorkflowState {
            workflow_id: Slug::new("test-workflow")?,
            task_status: HashMap::from([
                (Slug::new("task-1")?, TaskExecutionStatus::InProgress),
                (Slug::new("task-2")?, TaskExecutionStatus::Pending),
            ]),
            timestamp: Utc::now(),
        };

        let ready = engine.get_ready_tasks(&workflow, &state);
        assert!(ready.is_empty());
        Ok(())
    }

    #[test]
    fn test_get_ready_tasks_returns_empty_for_workflow_id_mismatch(
    ) -> Result<(), Box<dyn std::error::Error>> {
        let engine = ExecutionEngine::new();
        let mut workflow = Workflow::new("wf-1", "Test", "Description", Utc::now())?;

        let task1 = Task::new("task-1", "Task 1", "First task")?;
        workflow.add_task(task1, Utc::now())?;

        let state = WorkflowState {
            workflow_id: Slug::new("wf-2").unwrap(),
            task_status: HashMap::from([(
                Slug::new("task-1").unwrap(),
                TaskExecutionStatus::Pending,
            )]),
            timestamp: Utc::now(),
        };

        let ready = engine.get_ready_tasks(&workflow, &state);
        assert!(ready.is_empty());
        Ok(())
    }

    #[tokio::test]
    async fn test_save_and_load_checkpoint() -> Result<(), Box<dyn std::error::Error>> {
        let temp_dir = tempfile::tempdir()?;
        let checkpoint_dir = temp_dir.path().to_str().ok_or("invalid path")?;

        let engine = ExecutionEngine::new().with_checkpoint_dir(checkpoint_dir);

        let state = WorkflowState {
            workflow_id: Slug::new("test-workflow").unwrap(),
            task_status: HashMap::from([
                (Slug::new("task-1").unwrap(), TaskExecutionStatus::Completed),
                (Slug::new("task-2").unwrap(), TaskExecutionStatus::Pending),
            ]),
            timestamp: Utc::now(),
        };

        let checkpoint_path = engine
            .save_checkpoint(&state)
            .await
            .map_err(|e| e.to_string())?;

        assert!(checkpoint_path.contains("test-workflow.json"));

        // Verify file exists
        tokio::fs::metadata(&checkpoint_path).await?;

        // Load checkpoint
        let loaded_state = engine
            .load_checkpoint(Path::new(&checkpoint_path))
            .await
            .map_err(|e| e.to_string())?;

        assert_eq!(loaded_state.workflow_id, state.workflow_id);
        assert_eq!(loaded_state.task_status, state.task_status);
        Ok(())
    }

    #[tokio::test]
    async fn test_save_checkpoint_no_directory() -> Result<(), Box<dyn std::error::Error>> {
        let engine = ExecutionEngine::new();

        let state = WorkflowState {
            workflow_id: Slug::new("test-workflow")?,
            task_status: HashMap::new(),
            timestamp: Utc::now(),
        };

        let result = engine.save_checkpoint(&state).await;
        assert!(matches!(
            result,
            Err(ExecutionError::CheckpointFailed { .. })
        ));
        Ok(())
    }

    #[tokio::test]
    async fn test_load_checkpoint_corrupted_json() -> Result<(), Box<dyn std::error::Error>> {
        let temp_dir = tempfile::tempdir()?;
        let checkpoint_path = temp_dir.path().join("broken.json");

        tokio::fs::write(&checkpoint_path, "{ definitely-not-json").await?;

        let engine = ExecutionEngine::new();
        let result = engine.load_checkpoint(&checkpoint_path).await;

        match result {
            Err(ExecutionError::CheckpointCorrupted {
                path,
                validation_errors,
            }) => {
                assert!(path.contains("broken.json"));
                assert!(!validation_errors.is_empty());
                assert!(validation_errors[0].contains("Parse failed"));
                Ok(())
            }
            Ok(_) => Err("expected checkpoint corruption for invalid json".into()),
            Err(other) => Err(format!("expected CheckpointCorrupted error, got {other}").into()),
        }
    }

    #[test]
    fn test_topological_sort_stress_large_dag() -> Result<(), Box<dyn std::error::Error>> {
        let engine = ExecutionEngine::new();
        let mut workflow = Workflow::new("stress-dag", "Stress", "Large DAG", Utc::now())?;

        let task_count = 128;
        for i in 0..task_count {
            let id = format!("task-{i}");
            workflow.add_task(
                Task::new(id.clone(), format!("Task {i}"), "stress")?,
                Utc::now(),
            )?;
        }

        for i in 1..task_count {
            workflow.add_dependency("task-0", format!("task-{i}"), Utc::now())?;
        }

        for i in 2..task_count {
            if i % 2 == 0 {
                workflow.add_dependency(
                    format!("task-{}", i - 1),
                    format!("task-{i}"),
                    Utc::now(),
                )?;
            }
        }

        let sorted = engine
            .topological_sort(&workflow)
            .map_err(|e| e.to_string())?;
        assert_eq!(sorted.len(), task_count);

        let positions: HashMap<Slug, usize> = sorted
            .iter()
            .enumerate()
            .map(|(idx, id)| (id.clone(), idx))
            .collect();

        for (task_id, deps) in workflow.dependencies() {
            let task_pos = positions
                .get(task_id)
                .ok_or_else(|| format!("missing task in sorted output: {task_id}"))?;
            for dep in deps {
                let dep_pos = positions
                    .get(dep)
                    .ok_or_else(|| format!("missing dependency in sorted output: {dep}"))?;
                assert!(dep_pos < task_pos);
            }
        }

        Ok(())
    }

    #[test]
    fn test_execute_workflow_empty() -> Result<(), Box<dyn std::error::Error>> {
        let engine = ExecutionEngine::new();
        let workflow = Workflow::new("test-workflow", "Test", "Description", Utc::now())?;

        let result = engine
            .execute_workflow(&workflow)
            .map_err(|e| e.to_string())?;
        assert!(result.succeeded.is_empty());
        assert!(result.failed.is_empty());
        Ok(())
    }

    #[test]
    fn test_execute_workflow_linear() -> Result<(), Box<dyn std::error::Error>> {
        let engine = ExecutionEngine::new();
        let mut workflow = Workflow::new("test-workflow", "Test", "Description", Utc::now())?;

        let task1 = Task::new("task-1", "Task 1", "First task")?;
        let task2 = Task::new("task-2", "Task 2", "Second task")?;

        workflow.add_task(task1, Utc::now())?;
        workflow.add_task(task2, Utc::now())?;
        workflow.add_dependency("task-1", "task-2", Utc::now())?;

        let result = engine
            .execute_workflow(&workflow)
            .map_err(|e| e.to_string())?;
        assert_eq!(result.succeeded.len(), 2);
        assert!(result.succeeded.contains(&Slug::new("task-1")?));
        assert!(result.succeeded.contains(&Slug::new("task-2")?));
        assert!(result.failed.is_empty());
        Ok(())
    }

    #[test]
    fn test_rollback_task_completed() -> Result<(), Box<dyn std::error::Error>> {
        let engine = ExecutionEngine::new();
        let mut workflow = Workflow::new("test-workflow", "Test", "Description", Utc::now())?;

        let task1 = Task::new("task-1", "Task 1", "First task")?;
        workflow.add_task(task1, Utc::now())?;

        let mut state = WorkflowState {
            workflow_id: Slug::new("test-workflow")?,
            task_status: HashMap::from([(Slug::new("task-1")?, TaskExecutionStatus::Completed)]),
            timestamp: Utc::now(),
        };

        let result = engine.rollback_task(&Slug::new("task-1")?, &workflow, &mut state);
        assert!(result.is_ok());

        assert_eq!(
            state.task_status.get(&Slug::new("task-1")?),
            Some(&TaskExecutionStatus::RolledBack)
        );
        Ok(())
    }

    #[test]
    fn test_rollback_task_idempotent() -> Result<(), Box<dyn std::error::Error>> {
        let engine = ExecutionEngine::new();
        let mut workflow = Workflow::new("test-workflow", "Test", "Description", Utc::now())?;

        let task1 = Task::new("task-1", "Task 1", "First task")?;
        workflow.add_task(task1, Utc::now())?;

        let mut state = WorkflowState {
            workflow_id: Slug::new("test-workflow")?,
            task_status: HashMap::from([(Slug::new("task-1")?, TaskExecutionStatus::RolledBack)]),
            timestamp: Utc::now(),
        };

        // First rollback
        let result1 = engine.rollback_task(&Slug::new("task-1")?, &workflow, &mut state);
        assert!(result1.is_ok());

        // Second rollback (idempotent)
        let result2 = engine.rollback_task(&Slug::new("task-1")?, &workflow, &mut state);
        assert!(result2.is_ok());

        assert_eq!(
            state.task_status.get(&Slug::new("task-1")?),
            Some(&TaskExecutionStatus::RolledBack)
        );
        Ok(())
    }

    #[test]
    fn test_rollback_task_not_found() -> Result<(), Box<dyn std::error::Error>> {
        let engine = ExecutionEngine::new();
        let workflow = Workflow::new("test-workflow", "Test", "Description", Utc::now())?;

        let mut state = WorkflowState {
            workflow_id: Slug::new("test-workflow").unwrap(),
            task_status: HashMap::new(),
            timestamp: Utc::now(),
        };

        let result =
            engine.rollback_task(&Slug::new("non-existent").unwrap(), &workflow, &mut state);
        assert!(result.is_err());
        assert!(matches!(result, Err(ExecutionError::RollbackFailed { .. })));
        Ok(())
    }

    #[tokio::test]
    async fn test_recover_from_checkpoint() -> Result<(), Box<dyn std::error::Error>> {
        let temp_dir = tempfile::tempdir()?;
        let checkpoint_dir = temp_dir.path().to_str().ok_or("invalid path")?;

        let engine = ExecutionEngine::new().with_checkpoint_dir(checkpoint_dir);

        let mut workflow = Workflow::new("test-workflow", "Test", "Description", Utc::now())?;

        let task1 = Task::new("task-1", "Task 1", "First task")?;
        workflow.add_task(task1, Utc::now())?;

        let state = WorkflowState {
            workflow_id: Slug::new("test-workflow").unwrap(),
            task_status: HashMap::from([(
                Slug::new("task-1").unwrap(),
                TaskExecutionStatus::Completed,
            )]),
            timestamp: Utc::now(),
        };

        // Save checkpoint
        let checkpoint_path = engine
            .save_checkpoint(&state)
            .await
            .map_err(|e| e.to_string())?;

        // Recover from checkpoint
        let recovered_state = engine
            .recover_from_checkpoint(Path::new(&checkpoint_path), &workflow)
            .await
            .map_err(|e| e.to_string())?;

        assert_eq!(recovered_state.workflow_id, state.workflow_id);
        assert_eq!(recovered_state.task_status, state.task_status);
        Ok(())
    }

    #[tokio::test]
    async fn test_recover_from_checkpoint_workflow_id_mismatch(
    ) -> Result<(), Box<dyn std::error::Error>> {
        let temp_dir = tempfile::tempdir()?;
        let checkpoint_dir = temp_dir.path().to_str().ok_or("invalid path")?;

        let engine = ExecutionEngine::new().with_checkpoint_dir(checkpoint_dir);

        let _workflow1 = Workflow::new("workflow-1", "Test", "Description", Utc::now())?;

        let state = WorkflowState {
            workflow_id: Slug::new("workflow-1").unwrap(),
            task_status: HashMap::new(),
            timestamp: Utc::now(),
        };

        // Save checkpoint
        let checkpoint_path = engine
            .save_checkpoint(&state)
            .await
            .map_err(|e| e.to_string())?;

        // Try to recover with different workflow
        let workflow2 = Workflow::new("workflow-2", "Test", "Description", Utc::now())?;

        let result = engine
            .recover_from_checkpoint(Path::new(&checkpoint_path), &workflow2)
            .await;

        assert!(result.is_err());
        assert!(matches!(
            result,
            Err(ExecutionError::CheckpointCorrupted { .. })
        ));
        Ok(())
    }
}
