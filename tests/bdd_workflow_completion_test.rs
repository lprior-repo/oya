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

// ═══════════════════════════════════════════════════════════════════════════════
// BDD TESTS: LINEAR WORKFLOW SEQUENTIAL EXECUTION
// ═══════════════════════════════════════════════════════════════════════════════

/// BDD Test: Linear workflow executes tasks in strict sequential order
///
/// GIVEN a linear workflow with chain A → B → C
/// WHEN executing the workflow
/// THEN tasks complete in dependency order (A, then B, then C)
#[test]
fn bdd_linear_workflow_sequential_execution_order() -> Result<(), Box<dyn std::error::Error>> {
    // GIVEN: A linear workflow with chain A → B → C
    let mut workflow = Workflow::new(
        "linear-sequential",
        "Linear Sequential Workflow",
        "Tests sequential execution order",
    )?;

    let task_a = Task::new("task-a", "Task A", "First task in chain")?;
    let task_b = Task::new("task-b", "Task B", "Second task in chain")?;
    let task_c = Task::new("task-c", "Task C", "Third task in chain")?;

    workflow.add_task(task_a)?;
    workflow.add_task(task_b)?;
    workflow.add_task(task_c)?;

    workflow.add_dependency("task-a", "task-b")?;
    workflow.add_dependency("task-b", "task-c")?;

    let engine = ExecutionEngine::new();

    // WHEN: Executing the workflow
    let result = engine.execute_workflow(&workflow)?;

    // THEN: Tasks complete in dependency order
    assert_eq!(result.succeeded.len(), 3, "All 3 tasks should succeed");
    assert_eq!(
        result.succeeded,
        vec!["task-a", "task-b", "task-c"],
        "Tasks must complete in sequential dependency order"
    );
    assert!(result.failed.is_empty(), "No tasks should fail");

    Ok(())
}

/// BDD Test: Linear workflow respects dependency chain - B cannot start before A
///
/// GIVEN a linear workflow A → B
/// WHEN checking ready tasks before A completes
/// THEN only A is ready, B is blocked
#[test]
fn bdd_linear_workflow_dependency_blocks_downstream() -> Result<(), Box<dyn std::error::Error>> {
    // GIVEN: A linear workflow A → B
    let mut workflow = Workflow::new(
        "linear-block-test",
        "Linear Block Test",
        "Tests that downstream tasks are blocked",
    )?;

    let task_a = Task::new("task-a", "Task A", "Upstream task")?;
    let task_b = Task::new("task-b", "Task B", "Downstream task")?;

    workflow.add_task(task_a)?;
    workflow.add_task(task_b)?;

    workflow.add_dependency("task-a", "task-b")?;

    let engine = ExecutionEngine::new();
    let state = engine.parse_workflow(&workflow)?;

    // WHEN: Checking ready tasks before any execution
    let ready = engine.get_ready_tasks(&workflow, &state);

    // THEN: Only A is ready, B is blocked by dependency
    assert_eq!(ready.len(), 1, "Only one task should be ready");
    assert_eq!(
        ready.first(),
        Some(&"task-a".to_string()),
        "Only task-a (no dependencies) should be ready"
    );
    assert!(
        !ready.contains(&"task-b".to_string()),
        "task-b should be blocked by dependency on task-a"
    );

    Ok(())
}

/// BDD Test: Linear workflow with 5-task chain executes in order
///
/// GIVEN a linear workflow with 5 tasks in chain A → B → C → D → E
/// WHEN executing the workflow
/// THEN all tasks complete in strict sequential order
#[test]
fn bdd_linear_workflow_five_task_chain() -> Result<(), Box<dyn std::error::Error>> {
    // GIVEN: A linear workflow with 5 tasks in chain
    let mut workflow = Workflow::new(
        "linear-five-chain",
        "Linear Five Chain",
        "Tests 5-task linear chain execution",
    )?;

    let tasks = [
        ("task-a", "Task A"),
        ("task-b", "Task B"),
        ("task-c", "Task C"),
        ("task-d", "Task D"),
        ("task-e", "Task E"),
    ];

    for (id, name) in tasks {
        let task = Task::new(id, name, "Chain task")?;
        workflow.add_task(task)?;
    }

    workflow.add_dependency("task-a", "task-b")?;
    workflow.add_dependency("task-b", "task-c")?;
    workflow.add_dependency("task-c", "task-d")?;
    workflow.add_dependency("task-d", "task-e")?;

    let engine = ExecutionEngine::new();

    // WHEN: Executing the workflow
    let result = engine.execute_workflow(&workflow)?;

    // THEN: All tasks complete in strict sequential order
    assert_eq!(result.succeeded.len(), 5, "All 5 tasks should succeed");
    assert_eq!(
        result.succeeded,
        vec!["task-a", "task-b", "task-c", "task-d", "task-e"],
        "Tasks must complete in strict sequential order"
    );
    assert!(result.failed.is_empty(), "No tasks should fail");

    Ok(())
}

/// BDD Test: Linear workflow - dependency resolution after completion
///
/// GIVEN a linear workflow A → B → C where A has completed
/// WHEN checking ready tasks
/// THEN only B is ready (C still blocked by B)
#[test]
fn bdd_linear_workflow_partial_completion_unblocks_next() -> Result<(), Box<dyn std::error::Error>>
{
    use oya_core::execution::{TaskExecutionStatus, WorkflowState};

    // GIVEN: A linear workflow A → B → C
    let mut workflow = Workflow::new(
        "linear-partial",
        "Linear Partial",
        "Tests partial completion unblocks next",
    )?;

    let task_a = Task::new("task-a", "Task A", "First task")?;
    let task_b = Task::new("task-b", "Task B", "Second task")?;
    let task_c = Task::new("task-c", "Task C", "Third task")?;

    workflow.add_task(task_a)?;
    workflow.add_task(task_b)?;
    workflow.add_task(task_c)?;

    workflow.add_dependency("task-a", "task-b")?;
    workflow.add_dependency("task-b", "task-c")?;

    let engine = ExecutionEngine::new();

    // Simulate A completed
    let state = WorkflowState {
        workflow_id: "linear-partial".to_string(),
        task_status: std::collections::HashMap::from([
            ("task-a".to_string(), TaskExecutionStatus::Completed),
            ("task-b".to_string(), TaskExecutionStatus::Pending),
            ("task-c".to_string(), TaskExecutionStatus::Pending),
        ]),
        timestamp: chrono::Utc::now(),
    };

    // WHEN: Checking ready tasks after A completed
    let ready = engine.get_ready_tasks(&workflow, &state);

    // THEN: Only B is ready (C still blocked by B)
    assert_eq!(ready.len(), 1, "Only one task should be ready");
    assert_eq!(
        ready.first(),
        Some(&"task-b".to_string()),
        "Only task-b should be ready after task-a completes"
    );
    assert!(
        !ready.contains(&"task-c".to_string()),
        "task-c should still be blocked by task-b"
    );

    Ok(())
}

/// BDD Test: Linear workflow - all tasks blocked until chain starts
///
/// GIVEN a linear workflow A → B → C with all tasks pending
/// WHEN A is blocked (not pending)
/// THEN no tasks are ready
#[test]
fn bdd_linear_workflow_blocked_start_blocks_all() -> Result<(), Box<dyn std::error::Error>> {
    use oya_core::execution::{TaskExecutionStatus, WorkflowState};

    // GIVEN: A linear workflow A → B → C
    let mut workflow = Workflow::new(
        "linear-blocked-start",
        "Linear Blocked Start",
        "Tests blocked start blocks entire chain",
    )?;

    let task_a = Task::new("task-a", "Task A", "First task")?;
    let task_b = Task::new("task-b", "Task B", "Second task")?;
    let task_c = Task::new("task-c", "Task C", "Third task")?;

    workflow.add_task(task_a)?;
    workflow.add_task(task_b)?;
    workflow.add_task(task_c)?;

    workflow.add_dependency("task-a", "task-b")?;
    workflow.add_dependency("task-b", "task-c")?;

    let engine = ExecutionEngine::new();

    // Simulate A is in-progress (not completed, not pending)
    let state = WorkflowState {
        workflow_id: "linear-blocked-start".to_string(),
        task_status: std::collections::HashMap::from([
            ("task-a".to_string(), TaskExecutionStatus::InProgress),
            ("task-b".to_string(), TaskExecutionStatus::Pending),
            ("task-c".to_string(), TaskExecutionStatus::Pending),
        ]),
        timestamp: chrono::Utc::now(),
    };

    // WHEN: Checking ready tasks while A is in-progress
    let ready = engine.get_ready_tasks(&workflow, &state);

    // THEN: No tasks are ready (A in progress, B and C blocked)
    assert!(
        ready.is_empty(),
        "No tasks should be ready when head of chain is in-progress"
    );

    Ok(())
}

// ═══════════════════════════════════════════════════════════════════════════════
// PROPERTY TESTS: WORKFLOW COMPLETION
// ═══════════════════════════════════════════════════════════════════════════════

mod property_tests {
    use super::*;
    use oya_core::Stage;
    use proptest::prelude::*;

    // Property: ∀ workflow, if all beads complete -> workflow complete
    //
    // This test generates random workflows with varying numbers of tasks
    // and verifies that completing all tasks always results in a complete workflow.
    proptest! {
        #[test]
        fn prop_all_beads_complete_implies_workflow_complete(
            task_count in 0usize..100,
        ) {
            let mut workflow = Workflow::new(
                "prop-workflow",
                "Property Test Workflow",
                "Tests all beads complete implies workflow complete",
            )
            .map_err(|e| proptest::test_runner::TestCaseError::fail(e.to_string()))?;

            for i in 0..task_count {
                let task = Task::new(
                    format!("task-{i}"),
                    format!("Task {i}"),
                    "Property test task",
                )
                .map_err(|e| proptest::test_runner::TestCaseError::fail(e.to_string()))?;
                workflow
                    .add_task(task)
                    .map_err(|e| proptest::test_runner::TestCaseError::fail(e.to_string()))?;
            }

            if task_count == 0 {
                prop_assert!(workflow.is_complete(), "Empty workflow should be complete");
            } else {
                prop_assert!(
                    !workflow.is_complete(),
                    "Workflow with pending tasks should not be complete"
                );

                for task in workflow.tasks.values_mut() {
                    task.current_stage = Stage::Completed;
                }

                prop_assert!(
                    workflow.is_complete(),
                    "Workflow with all completed tasks should be complete"
                );
            }
        }

        #[test]
        fn prop_workflow_completion_invariant_across_dag_shapes(
            task_count in 1usize..50,
            edge_probability in 0.0f64..1.0,
        ) {
            use std::collections::HashSet;

            let mut workflow = Workflow::new(
                "prop-dag-workflow",
                "DAG Property Test",
                "Tests completion invariant across DAG shapes",
            )
            .map_err(|e| proptest::test_runner::TestCaseError::fail(e.to_string()))?;

            for i in 0..task_count {
                let task = Task::new(
                    format!("task-{i}"),
                    format!("Task {i}"),
                    "DAG task",
                )
                .map_err(|e| proptest::test_runner::TestCaseError::fail(e.to_string()))?;
                workflow
                    .add_task(task)
                    .map_err(|e| proptest::test_runner::TestCaseError::fail(e.to_string()))?;
            }

            let mut added_edges: HashSet<(String, String)> = HashSet::new();
            for i in 0..task_count {
                for j in (i + 1)..task_count {
                    if (j as f64 * edge_probability) as usize > (j - 1) as usize {
                        let from = format!("task-{i}");
                        let to = format!("task-{j}");
                        if added_edges.insert((from.clone(), to.clone())) {
                            let _ = workflow.add_dependency(&from, &to);
                        }
                    }
                }
            }

            prop_assert!(
                !workflow.is_complete(),
                "Workflow with pending tasks should not be complete"
            );

            for task in workflow.tasks.values_mut() {
                task.current_stage = Stage::Completed;
            }

            prop_assert!(
                workflow.is_complete(),
                "Workflow with all completed tasks must be complete regardless of DAG shape"
            );
        }
    }
}
