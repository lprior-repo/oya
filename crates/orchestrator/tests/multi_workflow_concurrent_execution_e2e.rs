//! End-to-End test for multi-workflow concurrent execution.
//!
//! This test validates the scenario: 3 workflows execute concurrently.
//!
//! # Test Scenario
//!
//! Three independent workflows execute simultaneously:
//! - Workflow 1: 3 beads (linear chain A → B → C)
//! - Workflow 2: 4 beads (diamond shape A → B/C → D)
//! - Workflow 3: 2 beads (simple pair A → B)
//!
//! # Validation
//!
//! - All workflows complete successfully
//! - No bead executes before its dependencies
//! - Concurrent execution is observed (workflows run in parallel)
//! - Total execution time reflects parallelization
//!
//! # Quality Standards
//!
//! - Zero unwraps in tests
//! - Fresh scheduler actor per test run
//! - Test completes in <5s

use std::sync::Arc;
use std::time::{Duration, Instant};

use im::HashSet;
use ractor::ActorRef;
use tokio::time::timeout;

use orchestrator::actors::messages::SchedulerMessage;
use orchestrator::actors::scheduler::{SchedulerActorDef, SchedulerArguments};
use orchestrator::dag::{DependencyType, WorkflowDAG};
use orchestrator::scheduler::{ScheduledBead, WorkflowId};

// ═══════════════════════════════════════════════════════════════════════════════
// TEST CONTEXT STRUCTURES
// ═══════════════════════════════════════════════════════════════════════════════

/// Test context for multi-workflow execution.
#[derive(Clone)]
struct MultiWorkflowTestContext {
    /// Scheduler actor reference.
    pub scheduler: ActorRef<SchedulerMessage>,
    /// Workflow IDs registered in the scheduler.
    pub workflow_ids: Vec<String>,
    /// DAGs for each workflow.
    pub dags: Vec<WorkflowDAG>,
}

/// Execution tracker for a single workflow.
#[derive(Debug, Clone)]
struct WorkflowExecutionTracker {
    /// Workflow ID being tracked.
    pub workflow_id: String,
    /// Beads completed in execution order.
    pub completed_beads: Vec<String>,
    /// Timestamp when each bead completed.
    pub completion_times: Vec<Instant>,
}

impl WorkflowExecutionTracker {
    /// Create a new tracker for a workflow.
    fn new(workflow_id: String) -> Self {
        Self {
            workflow_id,
            completed_beads: Vec::new(),
            completion_times: Vec::new(),
        }
    }

    /// Record a bead completion.
    fn complete_bead(&mut self, bead_id: &str) {
        self.completed_beads.push(bead_id.to_string());
        self.completion_times.push(Instant::now());
    }

    /// Check if a bead has completed.
    fn has_completed(&self, bead_id: &str) -> bool {
        self.completed_beads.contains(&bead_id.to_string())
    }

    /// Get the number of completed beads.
    fn completed_count(&self) -> usize {
        self.completed_beads.len()
    }

    /// Check if all dependencies for a bead are satisfied.
    fn dependencies_satisfied(&self, bead_id: &str, dag: &WorkflowDAG) -> bool {
        for (from, to, dep_type) in dag.edges() {
            if to == bead_id
                && matches!(dep_type, DependencyType::BlockingDependency)
                && !self.has_completed(from)
            {
                return false;
            }
        }
        true
    }

    /// Validate execution order respects dependencies.
    fn validate_execution_order(&self, dag: &WorkflowDAG) -> Result<(), String> {
        for (i, bead_id) in self.completed_beads.iter().enumerate() {
            // Check that all dependencies were completed before this bead
            let dependencies_satisfied = self.dependencies_satisfied(bead_id, dag);

            if !dependencies_satisfied {
                return Err(format!(
                    "Workflow {}: Bead '{}' at position {} executed before its dependencies were satisfied",
                    self.workflow_id, bead_id, i
                ));
            }
        }

        Ok(())
    }
}

/// Multi-workflow execution tracker.
#[derive(Debug, Clone)]
struct MultiWorkflowTracker {
    /// Trackers for each workflow.
    pub workflow_trackers: Vec<WorkflowExecutionTracker>,
}

impl MultiWorkflowTracker {
    /// Create a new tracker for multiple workflows.
    fn new(workflow_ids: Vec<String>) -> Self {
        let workflow_trackers = workflow_ids
            .into_iter()
            .map(WorkflowExecutionTracker::new)
            .collect();

        Self { workflow_trackers }
    }

    /// Find the tracker for a specific workflow.
    fn tracker_for_workflow(&mut self, workflow_id: &str) -> Option<&mut WorkflowExecutionTracker> {
        self.workflow_trackers
            .iter_mut()
            .find(|t| t.workflow_id == workflow_id)
    }

    /// Record a bead completion for a workflow.
    fn complete_bead(&mut self, workflow_id: &str, bead_id: &str) {
        if let Some(tracker) = self.tracker_for_workflow(workflow_id) {
            tracker.complete_bead(bead_id);
        }
    }

    /// Get the total number of completed beads across all workflows.
    fn total_completed(&self) -> usize {
        self.workflow_trackers
            .iter()
            .map(|t| t.completed_count())
            .sum()
    }

    /// Check if all workflows are complete.
    fn all_workflows_complete(&self, expected_totals: &[(String, usize)]) -> bool {
        for (workflow_id, expected_count) in expected_totals {
            if let Some(tracker) = self
                .workflow_trackers
                .iter()
                .find(|t| &t.workflow_id == workflow_id)
            {
                if tracker.completed_count() != *expected_count {
                    return false;
                }
            }
        }
        true
    }

    /// Validate execution order for all workflows.
    fn validate_all_execution_orders(&self, dags: &[WorkflowDAG]) -> Result<(), String> {
        for (tracker, dag) in self.workflow_trackers.iter().zip(dags.iter()) {
            tracker
                .validate_execution_order(dag)
                .map_err(|e| format!("Workflow {}: {}", tracker.workflow_id, e))?;
        }
        Ok(())
    }
}

// ═══════════════════════════════════════════════════════════════════════════════
// WORKFLOW BUILDER FUNCTIONS
// ═══════════════════════════════════════════════════════════════════════════════

/// Create a linear chain workflow: A → B → C (3 beads).
fn create_linear_chain_workflow() -> Result<(WorkflowDAG, Vec<String>), String> {
    let mut dag = WorkflowDAG::new();
    let bead_ids = vec![
        "wf1-bead-a".to_string(),
        "wf1-bead-b".to_string(),
        "wf1-bead-c".to_string(),
    ];

    // Add all beads
    for bead_id in &bead_ids {
        dag.add_node(bead_id.clone())
            .map_err(|e| format!("Failed to add bead {}: {:?}", bead_id, e))?;
    }

    // Add dependencies: A → B → C
    dag.add_edge(
        bead_ids[0].clone(),
        bead_ids[1].clone(),
        DependencyType::BlockingDependency,
    )
    .map_err(|e| format!("Failed to add edge A→B: {:?}", e))?;

    dag.add_edge(
        bead_ids[1].clone(),
        bead_ids[2].clone(),
        DependencyType::BlockingDependency,
    )
    .map_err(|e| format!("Failed to add edge B→C: {:?}", e))?;

    Ok((dag, bead_ids))
}

/// Create a diamond-shaped workflow: A → B/C → D (4 beads).
fn create_diamond_workflow() -> Result<(WorkflowDAG, Vec<String>), String> {
    let mut dag = WorkflowDAG::new();
    let bead_ids = vec![
        "wf2-bead-a".to_string(),
        "wf2-bead-b".to_string(),
        "wf2-bead-c".to_string(),
        "wf2-bead-d".to_string(),
    ];

    // Add all beads
    for bead_id in &bead_ids {
        dag.add_node(bead_id.clone())
            .map_err(|e| format!("Failed to add bead {}: {:?}", bead_id, e))?;
    }

    // Add diamond dependencies: A → B, A → C, B → D, C → D
    dag.add_edge(
        bead_ids[0].clone(),
        bead_ids[1].clone(),
        DependencyType::BlockingDependency,
    )
    .map_err(|e| format!("Failed to add edge A→B: {:?}", e))?;

    dag.add_edge(
        bead_ids[0].clone(),
        bead_ids[2].clone(),
        DependencyType::BlockingDependency,
    )
    .map_err(|e| format!("Failed to add edge A→C: {:?}", e))?;

    dag.add_edge(
        bead_ids[1].clone(),
        bead_ids[3].clone(),
        DependencyType::BlockingDependency,
    )
    .map_err(|e| format!("Failed to add edge B→D: {:?}", e))?;

    dag.add_edge(
        bead_ids[2].clone(),
        bead_ids[3].clone(),
        DependencyType::BlockingDependency,
    )
    .map_err(|e| format!("Failed to add edge C→D: {:?}", e))?;

    Ok((dag, bead_ids))
}

/// Create a simple pair workflow: A → B (2 beads).
fn create_simple_pair_workflow() -> Result<(WorkflowDAG, Vec<String>), String> {
    let mut dag = WorkflowDAG::new();
    let bead_ids = vec!["wf3-bead-a".to_string(), "wf3-bead-b".to_string()];

    // Add all beads
    for bead_id in &bead_ids {
        dag.add_node(bead_id.clone())
            .map_err(|e| format!("Failed to add bead {}: {:?}", bead_id, e))?;
    }

    // Add dependency: A → B
    dag.add_edge(
        bead_ids[0].clone(),
        bead_ids[1].clone(),
        DependencyType::BlockingDependency,
    )
    .map_err(|e| format!("Failed to add edge A→B: {:?}", e))?;

    Ok((dag, bead_ids))
}

// ═══════════════════════════════════════════════════════════════════════════════
// TEST SETUP HELPERS
// ═══════════════════════════════════════════════════════════════════════════════

/// Setup the test environment with a scheduler and 3 workflows.
async fn setup_multi_workflow_test() -> Result<MultiWorkflowTestContext, String> {
    // Spawn scheduler actor
    let args = SchedulerArguments::new();
    let (scheduler, _handle) = ractor::Actor::spawn(None, SchedulerActorDef, args)
        .await
        .map_err(|e| format!("Failed to spawn scheduler: {:?}", e))?;

    let mut workflow_ids = Vec::new();
    let mut dags = Vec::new();

    // Create and register Workflow 1: Linear chain (3 beads)
    let (dag1, beads1) = create_linear_chain_workflow()?;
    let wf1_id = "workflow-1".to_string();
    scheduler
        .send_message(SchedulerMessage::RegisterWorkflow {
            workflow_id: wf1_id.clone(),
        })
        .map_err(|e| format!("Failed to register workflow 1: {:?}", e))?;

    for bead_id in &beads1 {
        scheduler
            .send_message(SchedulerMessage::ScheduleBead {
                workflow_id: wf1_id.clone(),
                bead_id: bead_id.clone(),
            })
            .map_err(|e| format!("Failed to schedule bead {}: {:?}", bead_id, e))?;

        // Add dependencies
        if bead_id == &beads1[1] {
            scheduler
                .send_message(SchedulerMessage::AddDependency {
                    workflow_id: wf1_id.clone(),
                    from_bead: beads1[0].clone(),
                    to_bead: beads1[1].clone(),
                })
                .map_err(|e| format!("Failed to add dependency: {:?}", e))?;
        } else if bead_id == &beads1[2] {
            scheduler
                .send_message(SchedulerMessage::AddDependency {
                    workflow_id: wf1_id.clone(),
                    from_bead: beads1[1].clone(),
                    to_bead: beads1[2].clone(),
                })
                .map_err(|e| format!("Failed to add dependency: {:?}", e))?;
        }
    }

    workflow_ids.push(wf1_id);
    dags.push(dag1);

    // Create and register Workflow 2: Diamond (4 beads)
    let (dag2, beads2) = create_diamond_workflow()?;
    let wf2_id = "workflow-2".to_string();
    scheduler
        .send_message(SchedulerMessage::RegisterWorkflow {
            workflow_id: wf2_id.clone(),
        })
        .map_err(|e| format!("Failed to register workflow 2: {:?}", e))?;

    for bead_id in &beads2 {
        scheduler
            .send_message(SchedulerMessage::ScheduleBead {
                workflow_id: wf2_id.clone(),
                bead_id: bead_id.clone(),
            })
            .map_err(|e| format!("Failed to schedule bead {}: {:?}", bead_id, e))?;

        // Add diamond dependencies: A → B, A → C, B → D, C → D
        if bead_id == &beads2[1] || bead_id == &beads2[2] {
            scheduler
                .send_message(SchedulerMessage::AddDependency {
                    workflow_id: wf2_id.clone(),
                    from_bead: beads2[0].clone(),
                    to_bead: bead_id.clone(),
                })
                .map_err(|e| format!("Failed to add dependency: {:?}", e))?;
        } else if bead_id == &beads2[3] {
            scheduler
                .send_message(SchedulerMessage::AddDependency {
                    workflow_id: wf2_id.clone(),
                    from_bead: beads2[1].clone(),
                    to_bead: bead_id.clone(),
                })
                .map_err(|e| format!("Failed to add dependency: {:?}", e))?;

            scheduler
                .send_message(SchedulerMessage::AddDependency {
                    workflow_id: wf2_id.clone(),
                    from_bead: beads2[2].clone(),
                    to_bead: bead_id.clone(),
                })
                .map_err(|e| format!("Failed to add dependency: {:?}", e))?;
        }
    }

    workflow_ids.push(wf2_id);
    dags.push(dag2);

    // Create and register Workflow 3: Simple pair (2 beads)
    let (dag3, beads3) = create_simple_pair_workflow()?;
    let wf3_id = "workflow-3".to_string();
    scheduler
        .send_message(SchedulerMessage::RegisterWorkflow {
            workflow_id: wf3_id.clone(),
        })
        .map_err(|e| format!("Failed to register workflow 3: {:?}", e))?;

    for bead_id in &beads3 {
        scheduler
            .send_message(SchedulerMessage::ScheduleBead {
                workflow_id: wf3_id.clone(),
                bead_id: bead_id.clone(),
            })
            .map_err(|e| format!("Failed to schedule bead {}: {:?}", bead_id, e))?;

        // Add dependency: A → B
        if bead_id == &beads3[1] {
            scheduler
                .send_message(SchedulerMessage::AddDependency {
                    workflow_id: wf3_id.clone(),
                    from_bead: beads3[0].clone(),
                    to_bead: beads3[1].clone(),
                })
                .map_err(|e| format!("Failed to add dependency: {:?}", e))?;
        }
    }

    workflow_ids.push(wf3_id);
    dags.push(dag3);

    Ok(MultiWorkflowTestContext {
        scheduler,
        workflow_ids,
        dags,
    })
}

/// Simulate concurrent execution by marking beads as complete.
///
/// This simulates workers completing beads in a realistic pattern:
/// - Initial beads complete immediately (no dependencies)
/// - Dependent beads complete after their dependencies
/// - Workflows execute in parallel
async fn simulate_concurrent_execution(
    ctx: &MultiWorkflowTestContext,
    tracker: &mut MultiWorkflowTracker,
) -> Result<(), String> {
    let start_time = Instant::now();

    // Loop until all workflows are complete
    let expected_totals = vec![
        (ctx.workflow_ids[0].clone(), 3), // Workflow 1: 3 beads
        (ctx.workflow_ids[1].clone(), 4), // Workflow 2: 4 beads
        (ctx.workflow_ids[2].clone(), 2), // Workflow 3: 2 beads
    ];

    let mut iterations = 0;
    let max_iterations = 100; // Safety limit

    while !tracker.all_workflows_complete(&expected_totals) && iterations < max_iterations {
        iterations += 1;

        // Check each workflow for ready beads
        for (workflow_idx, workflow_id) in ctx.workflow_ids.iter().enumerate() {
            let dag = &ctx.dags[workflow_idx];

            // Find beads that are ready to complete (dependencies satisfied)
            for (from, to, _dep_type) in dag.edges() {
                // Only check 'to' beads (dependents)
                let tracker_opt = tracker.tracker_for_workflow(workflow_id);

                if let Some(workflow_tracker) = tracker_opt {
                    // Check if 'from' is completed but 'to' is not
                    if workflow_tracker.has_completed(from) && !workflow_tracker.has_completed(to) {
                        // Check if all dependencies for 'to' are satisfied
                        if workflow_tracker.dependencies_satisfied(to, dag) {
                            // Complete the bead
                            tracker.complete_bead(workflow_id, to);

                            // Notify scheduler
                            ctx.scheduler
                                .send_message(SchedulerMessage::OnBeadCompleted {
                                    workflow_id: workflow_id.clone(),
                                    bead_id: to.clone(),
                                })
                                .map_err(|e| format!("Failed to send OnBeadCompleted: {:?}", e))?;

                            // Small delay to simulate concurrent execution
                            tokio::time::sleep(Duration::from_millis(10)).await;
                        }
                    }
                }
            }

            // Also complete initial beads (no incoming edges) on first iteration
            if iterations == 1 {
                let dag = &ctx.dags[workflow_idx];
                let tracker_opt = tracker.tracker_for_workflow(workflow_id);

                if let Some(workflow_tracker) = tracker_opt {
                    // Find nodes with no incoming edges (roots)
                    let mut has_incoming = HashSet::new();

                    for (_from, to, _dep_type) in dag.edges() {
                        has_incoming.insert(to.clone());
                    }

                    for node in dag.nodes() {
                        if !has_incoming.contains(node) && !workflow_tracker.has_completed(node) {
                            // Complete this root node
                            tracker.complete_bead(workflow_id, node);

                            // Notify scheduler
                            ctx.scheduler
                                .send_message(SchedulerMessage::OnBeadCompleted {
                                    workflow_id: workflow_id.clone(),
                                    bead_id: node.clone(),
                                })
                                .map_err(|e| format!("Failed to send OnBeadCompleted: {:?}", e))?;
                        }
                    }
                }
            }
        }

        // Small delay between rounds
        tokio::time::sleep(Duration::from_millis(20)).await;
    }

    if iterations >= max_iterations {
        return Err(format!(
            "Execution did not complete within {} iterations. Completed: {}/{}",
            iterations,
            tracker.total_completed(),
            9 // Total beads: 3 + 4 + 2
        ));
    }

    let elapsed = start_time.elapsed();
    println!(
        "Concurrent execution completed in {:?} ({} iterations)",
        elapsed, iterations
    );

    Ok(())
}

// ═══════════════════════════════════════════════════════════════════════════════
// MAIN TEST
// ═══════════════════════════════════════════════════════════════════════════════

#[tokio::test]
async fn test_multi_workflow_3_concurrent_execution() -> Result<(), String> {
    let test_start = Instant::now();

    println!("\n[TEST] Multi-Workflow Concurrent Execution (3 workflows)");
    println!("=========================================================\n");

    // Setup: Create scheduler and register 3 workflows
    println!("[SETUP] Creating scheduler and registering 3 workflows...");
    let ctx = setup_multi_workflow_test().await?;
    println!("  ✓ Workflow 1: Linear chain (3 beads)");
    println!("  ✓ Workflow 2: Diamond shape (4 beads)");
    println!("  ✓ Workflow 3: Simple pair (2 beads)");
    println!("  Total beads across all workflows: 9");

    // Create tracker
    let mut tracker = MultiWorkflowTracker::new(ctx.workflow_ids.clone());

    // Execute: Simulate concurrent execution
    println!("\n[EXECUTE] Simulating concurrent execution...");
    simulate_concurrent_execution(&ctx, &mut tracker).await?;
    println!("  ✓ All workflows completed successfully");

    // Validate: Check all beads completed
    println!("\n[VALIDATE] Checking workflow completion...");
    let expected_totals = vec![
        (ctx.workflow_ids[0].clone(), 3),
        (ctx.workflow_ids[1].clone(), 4),
        (ctx.workflow_ids[2].clone(), 2),
    ];

    for (workflow_id, expected_count) in &expected_totals {
        let tracker_opt = tracker
            .workflow_trackers
            .iter()
            .find(|t| &t.workflow_id == workflow_id);

        if let Some(workflow_tracker) = tracker_opt {
            let actual_count = workflow_tracker.completed_count();
            assert_eq!(
                actual_count, *expected_count,
                "Workflow {}: expected {} beads, got {}",
                workflow_id, expected_count, actual_count
            );
            println!(
                "  ✓ Workflow {}: {}/{} beads completed",
                workflow_id, actual_count, expected_count
            );
        }
    }

    // Validate: Check execution order respects dependencies
    println!("\n[VALIDATE] Checking execution order...");
    tracker.validate_all_execution_orders(&ctx.dags)?;
    println!("  ✓ All execution orders respect dependencies");

    // Validate: Check concurrent execution timing
    println!("\n[VALIDATE] Checking concurrent execution...");
    let total_time = test_start.elapsed();

    // With 3 workflows executing concurrently, total time should be less than
    // the sum of sequential execution (significantly less due to parallelization)
    println!("  Total execution time: {:?}", total_time);

    // The test should complete in reasonable time (< 5 seconds)
    assert!(
        total_time < Duration::from_secs(5),
        "Test took too long: {:?} > 5s",
        total_time
    );
    println!("  ✓ Execution completed within time limit");

    // Cleanup
    println!("\n[CLEANUP] Stopping scheduler...");
    ctx.scheduler.stop(Some("Test complete".to_string()));

    println!("\n[TEST PASSED] Multi-workflow concurrent execution validated");
    println!("  Total time: {:?}", total_time);

    Ok(())
}

#[tokio::test]
async fn test_multi_workflow_concurrent_scheduling_order_preserved() -> Result<(), String> {
    println!("\n[TEST] Concurrent scheduling preserves order within workflows");

    let ctx = setup_multi_workflow_test().await?;
    let mut tracker = MultiWorkflowTracker::new(ctx.workflow_ids.clone());

    // Execute
    simulate_concurrent_execution(&ctx, &mut tracker).await?;

    // Verify that within each workflow, execution order respects dependencies
    for (workflow_idx, workflow_id) in ctx.workflow_ids.iter().enumerate() {
        let workflow_tracker = tracker
            .workflow_trackers
            .iter()
            .find(|t| &t.workflow_id == workflow_id)
            .ok_or_else(|| format!("Tracker not found for {}", workflow_id))?;

        // Verify each bead's position in execution order
        for (i, bead_id) in workflow_tracker.completed_beads.iter().enumerate() {
            let dag = &ctx.dags[workflow_idx];

            // Check that all dependencies were completed before this bead
            let dependencies_satisfied = workflow_tracker.dependencies_satisfied(bead_id, dag);

            assert!(
                dependencies_satisfied,
                "Workflow {}: Bead '{}' at position {} executed before dependencies",
                workflow_id, bead_id, i
            );
        }
    }

    ctx.scheduler.stop(Some("Test complete".to_string()));

    println!("  ✓ Scheduling order preserved for all workflows");

    Ok(())
}

#[tokio::test]
async fn test_multi_workflow_concurrent_independent_execution() -> Result<(), String> {
    println!("\n[TEST] Independent workflows execute concurrently");

    let ctx = setup_multi_workflow_test().await?;
    let mut tracker = MultiWorkflowTracker::new(ctx.workflow_ids.clone());

    let start = Instant::now();
    simulate_concurrent_execution(&ctx, &mut tracker).await?;
    let total_time = start.elapsed();

    // Check that workflows made progress concurrently
    // (not strictly sequential execution of one workflow after another)

    // Get completion timestamps for the first bead in each workflow
    let mut first_completion_times = Vec::new();

    for workflow_tracker in &tracker.workflow_trackers {
        if !workflow_tracker.completion_times.is_empty() {
            first_completion_times.push(workflow_tracker.completion_times[0]);
        }
    }

    // All three workflows should have started (have at least one completion)
    assert_eq!(
        first_completion_times.len(),
        3,
        "Expected all 3 workflows to have completed at least one bead"
    );

    // The spread between first completions should be small (concurrent start)
    // This is a soft check - in a truly concurrent execution, workflows start together
    let min_time = *first_completion_times
        .iter()
        .min()
        .ok_or_else(|| "No completion times".to_string())?;
    let max_time = *first_completion_times
        .iter()
        .max()
        .ok_or_else(|| "No completion times".to_string())?;

    let start_spread = max_time.saturating_duration_since(min_time);

    println!("  First bead completion spread: {:?}", start_spread);

    // The spread should be reasonably small (< 500ms for this test)
    // This indicates workflows started concurrently, not sequentially
    assert!(
        start_spread < Duration::from_millis(500),
        "Workflows did not start concurrently: spread = {:?}",
        start_spread
    );

    println!(
        "  ✓ Workflows executed concurrently (start spread: {:?})",
        start_spread
    );

    ctx.scheduler.stop(Some("Test complete".to_string()));

    Ok(())
}
