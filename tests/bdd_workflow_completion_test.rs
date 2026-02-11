//! BDD Integration Tests for Workflow Completion
//!
//! This test module validates the end-to-end workflow completion behavior
//! using Behavior-Driven Development (BDD) style scenarios.
//!
//! ## Test Scenario
//!
//! GIVEN workflow WHEN all beads complete THEN workflow marked complete
//!
//! This test verifies that when all tasks (beads) in a workflow complete
//! successfully, the workflow status transitions to Complete.

#![deny(clippy::unwrap_used)]
#![deny(clippy::expect_used)]
#![deny(clippy::panic)]

use oya_core::{ExecutionEngine, Stage, Task, Workflow};

/// BDD Test: Workflow completion when all tasks succeed
///
/// GIVEN a workflow with multiple tasks
/// WHEN all tasks complete successfully
/// THEN the workflow is marked as complete
#[test]
fn bdd_workflow_completion_all_tasks_succeed() -> Result<(), Box<dyn std::error::Error>> {
    // GIVEN: A workflow with multiple tasks
    let mut workflow = Workflow::new("workflow-test", "Test Workflow", "BDD test workflow")?;

    let task1 = Task::new("task-1", "Task 1", "First task")?;
    let task2 = Task::new("task-2", "Task 2", "Second task")?;
    let task3 = Task::new("task-3", "Task 3", "Third task")?;

    workflow.add_task(task1)?;
    workflow.add_task(task2)?;
    workflow.add_task(task3)?;

    // Add dependencies: task-1 -> task-2 -> task-3
    workflow.add_dependency("task-1", "task-2")?;
    workflow.add_dependency("task-2", "task-3")?;

    // Verify workflow is not complete initially
    assert!(
        !workflow.is_complete(),
        "Workflow should not be complete before any tasks are done"
    );

    // WHEN: All tasks complete successfully
    for task_id in ["task-1", "task-2", "task-3"] {
        let task = workflow
            .get_task(task_id)
            .ok_or(format!("{task_id} not found"))?;
        let mut task = task.clone();
        task.complete_current_stage(); // Pending -> InProgress
        task.complete_current_stage(); // InProgress -> Completed
        workflow.tasks.insert(task_id.to_string(), task);
    }

    // THEN: Workflow is marked as complete
    assert!(
        workflow.is_complete(),
        "Workflow should be complete when all tasks are done"
    );

    Ok(())
}

/// BDD Test: Workflow completion with parallel independent tasks
///
/// GIVEN a workflow with parallel independent tasks
/// WHEN all tasks complete successfully
/// THEN the workflow is marked as complete
#[test]
fn bdd_workflow_completion_parallel_tasks() -> Result<(), Box<dyn std::error::Error>> {
    // GIVEN: A workflow with parallel independent tasks
    let mut workflow = Workflow::new(
        "workflow-parallel",
        "Parallel Workflow",
        "Test parallel tasks",
    )?;

    let task1 = Task::new("task-a", "Task A", "Independent task A")?;
    let task2 = Task::new("task-b", "Task B", "Independent task B")?;
    let task3 = Task::new("task-c", "Task C", "Independent task C")?;

    workflow.add_task(task1)?;
    workflow.add_task(task2)?;
    workflow.add_task(task3)?;

    // No dependencies - all tasks are independent

    // WHEN: All tasks complete successfully
    for task_id in ["task-a", "task-b", "task-c"] {
        let task = workflow
            .get_task(task_id)
            .ok_or(format!("{task_id} not found"))?;
        let mut task = task.clone();
        for _stage in Stage::all() {
            task.complete_current_stage();
        }
        workflow.tasks.insert(task_id.to_string(), task);
    }

    // THEN: Workflow is marked as complete
    assert!(
        workflow.is_complete(),
        "Parallel workflow should be complete when all tasks are done"
    );

    Ok(())
}

/// BDD Test: Workflow completion with diamond dependency graph
///
/// GIVEN a workflow with diamond-shaped dependencies (A -> [B, C] -> D)
/// WHEN all tasks complete successfully
/// THEN the workflow is marked as complete
#[test]
fn bdd_workflow_completion_diamond_graph() -> Result<(), Box<dyn std::error::Error>> {
    // GIVEN: A workflow with diamond dependencies
    let mut workflow = Workflow::new(
        "workflow-diamond",
        "Diamond Workflow",
        "Diamond dependency test",
    )?;

    let task_a = Task::new("task-a", "Task A", "Root task")?;
    let task_b = Task::new("task-b", "Task B", "Branch B")?;
    let task_c = Task::new("task-c", "Task C", "Branch C")?;
    let task_d = Task::new("task-d", "Task D", "Final task")?;

    workflow.add_task(task_a)?;
    workflow.add_task(task_b)?;
    workflow.add_task(task_c)?;
    workflow.add_task(task_d)?;

    // Diamond: A -> [B, C] -> D
    workflow.add_dependency("task-a", "task-b")?;
    workflow.add_dependency("task-a", "task-c")?;
    workflow.add_dependency("task-b", "task-d")?;
    workflow.add_dependency("task-c", "task-d")?;

    // WHEN: All tasks complete successfully
    let task_ids = ["task-a", "task-b", "task-c", "task-d"];
    for task_id in task_ids {
        let task = workflow
            .get_task(task_id)
            .ok_or(format!("{task_id} not found"))?;
        let mut task = task.clone();
        task.complete_current_stage(); // Pending -> InProgress
        task.complete_current_stage(); // InProgress -> Completed
        workflow.tasks.insert(task_id.to_string(), task);
    }

    // THEN: Workflow is marked as complete
    assert!(
        workflow.is_complete(),
        "Diamond workflow should be complete when all tasks are done"
    );

    Ok(())
}

/// BDD Test: Workflow completion through execution engine
///
/// GIVEN a workflow with tasks and dependencies
/// WHEN executing the workflow through ExecutionEngine
/// THEN the workflow result shows all tasks succeeded
#[test]
fn bdd_workflow_completion_via_execution_engine() -> Result<(), Box<dyn std::error::Error>> {
    // GIVEN: A workflow with tasks and dependencies
    let mut workflow = Workflow::new("workflow-exec", "Execution Test", "Test execution engine")?;

    let task1 = Task::new("task-1", "Task 1", "First task")?;
    let task2 = Task::new("task-2", "Task 2", "Second task")?;
    let task3 = Task::new("task-3", "Task 3", "Third task")?;

    workflow.add_task(task1)?;
    workflow.add_task(task2)?;
    workflow.add_task(task3)?;

    // Linear dependency chain: task-1 -> task-2 -> task-3
    workflow.add_dependency("task-1", "task-2")?;
    workflow.add_dependency("task-2", "task-3")?;

    let engine = ExecutionEngine::new();

    // WHEN: Executing the workflow through ExecutionEngine
    let result = engine.execute_workflow(&workflow)?;

    // THEN: All tasks succeed
    assert_eq!(result.succeeded.len(), 3, "All 3 tasks should succeed");
    assert!(
        result.succeeded.contains(&"task-1".to_string()),
        "task-1 should succeed"
    );
    assert!(
        result.succeeded.contains(&"task-2".to_string()),
        "task-2 should succeed"
    );
    assert!(
        result.succeeded.contains(&"task-3".to_string()),
        "task-3 should succeed"
    );
    assert!(result.failed.is_empty(), "No tasks should fail");
    assert!(
        result.rolled_back.is_empty(),
        "No tasks should be rolled back"
    );

    Ok(())
}

/// BDD Test: Empty workflow is considered complete
///
/// GIVEN an empty workflow with no tasks
/// WHEN checking workflow completion status
/// THEN the workflow is marked as complete (vacuously true)
#[test]
fn bdd_workflow_completion_empty_workflow() -> Result<(), Box<dyn std::error::Error>> {
    // GIVEN: An empty workflow
    let workflow = Workflow::new("workflow-empty", "Empty Workflow", "Empty workflow test")?;

    // WHEN: Checking workflow completion status
    let is_complete = workflow.is_complete();

    // THEN: Workflow is marked as complete
    assert!(
        is_complete,
        "Empty workflow should be complete (vacuously true)"
    );

    Ok(())
}
