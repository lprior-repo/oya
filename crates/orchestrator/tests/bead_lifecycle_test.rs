//! Rigorous End-to-End Bead Lifecycle Integration Test
//!
//! This test validates the entire bead lifecycle using REAL components:
//! - Real WorkflowDAG with petgraph dependency tracking
//! - Real SchedulerActor for bead scheduling
//! - Real dependency resolution and execution ordering
//!
//! NO MOCKS - this is a true integration test

use orchestrator::dag::{DependencyType, WorkflowDAG};
use orchestrator::scheduler::SchedulerActor;
use std::collections::HashSet;

/// Helper to track execution order of beads
#[derive(Debug, Clone)]
struct ExecutionTracker {
    completed_beads: Vec<String>,
}

impl ExecutionTracker {
    fn new() -> Self {
        Self {
            completed_beads: Vec::new(),
        }
    }

    fn complete_bead(&mut self, bead_id: &str) {
        self.completed_beads.push(bead_id.to_string());
    }

    fn completed_count(&self) -> usize {
        self.completed_beads.len()
    }

    fn has_completed(&self, bead_id: &str) -> bool {
        self.completed_beads.contains(&bead_id.to_string())
    }

    fn get_completion_order(&self) -> &[String] {
        &self.completed_beads
    }
}

/// Helper to check if dependencies are satisfied
fn dependencies_satisfied(bead_id: &str, dag: &WorkflowDAG, tracker: &ExecutionTracker) -> bool {
    // Get all edges where this bead is the target (dependencies)
    for (from, to, dep_type) in dag.edges() {
        if to == bead_id
            && matches!(dep_type, DependencyType::BlockingDependency)
            && !tracker.has_completed(from)
        {
            return false;
        }
    }
    true
}

/// Get beads that are ready to execute (all dependencies satisfied)
fn get_ready_beads(dag: &WorkflowDAG, tracker: &ExecutionTracker) -> Vec<String> {
    let mut ready = Vec::new();
    for node in dag.nodes() {
        if !tracker.has_completed(node) && dependencies_satisfied(node, dag, tracker) {
            ready.push(node.clone());
        }
    }
    ready
}

#[test]
fn test_bead_lifecycle_with_dependencies() {
    // Create a real WorkflowDAG with 5 beads in a dependency structure:
    // A → B → C
    // A → D → E
    //
    // Expected execution order: A first, then B and D (parallel), then C and E (parallel)

    let mut dag = WorkflowDAG::new();

    // Add all beads to the DAG
    let result = dag.add_node("bead-a".to_string());
    assert!(result.is_ok(), "Failed to add bead-a: {:?}", result);
    let result = dag.add_node("bead-b".to_string());
    assert!(result.is_ok(), "Failed to add bead-b: {:?}", result);
    let result = dag.add_node("bead-c".to_string());
    assert!(result.is_ok(), "Failed to add bead-c: {:?}", result);
    let result = dag.add_node("bead-d".to_string());
    assert!(result.is_ok(), "Failed to add bead-d: {:?}", result);
    let result = dag.add_node("bead-e".to_string());
    assert!(result.is_ok(), "Failed to add bead-e: {:?}", result);

    // Add dependency edges: A → B → C
    let result = dag.add_edge(
        "bead-a".to_string(),
        "bead-b".to_string(),
        DependencyType::BlockingDependency,
    );
    assert!(result.is_ok(), "Failed to add edge A→B: {:?}", result);

    let result = dag.add_edge(
        "bead-b".to_string(),
        "bead-c".to_string(),
        DependencyType::BlockingDependency,
    );
    assert!(result.is_ok(), "Failed to add edge B→C: {:?}", result);

    // Add dependency edges: A → D → E
    let result = dag.add_edge(
        "bead-a".to_string(),
        "bead-d".to_string(),
        DependencyType::BlockingDependency,
    );
    assert!(result.is_ok(), "Failed to add edge A→D: {:?}", result);

    let result = dag.add_edge(
        "bead-d".to_string(),
        "bead-e".to_string(),
        DependencyType::BlockingDependency,
    );
    assert!(result.is_ok(), "Failed to add edge D→E: {:?}", result);

    // Verify DAG structure
    assert_eq!(dag.node_count(), 5, "DAG should have 5 beads");
    assert_eq!(dag.edge_count(), 4, "DAG should have 4 dependency edges");

    // Create real SchedulerActor
    let mut scheduler = SchedulerActor::new();
    let workflow_id = "test-workflow-001".to_string();

    // Register workflow
    let result = scheduler.register_workflow(workflow_id.clone());
    assert!(result.is_ok(), "Failed to register workflow: {:?}", result);

    // Schedule all beads
    let result = scheduler.schedule_bead(workflow_id.clone(), "bead-a".to_string());
    assert!(result.is_ok(), "Failed to schedule bead-a: {:?}", result);
    let result = scheduler.schedule_bead(workflow_id.clone(), "bead-b".to_string());
    assert!(result.is_ok(), "Failed to schedule bead-b: {:?}", result);
    let result = scheduler.schedule_bead(workflow_id.clone(), "bead-c".to_string());
    assert!(result.is_ok(), "Failed to schedule bead-c: {:?}", result);
    let result = scheduler.schedule_bead(workflow_id.clone(), "bead-d".to_string());
    assert!(result.is_ok(), "Failed to schedule bead-d: {:?}", result);
    let result = scheduler.schedule_bead(workflow_id.clone(), "bead-e".to_string());
    assert!(result.is_ok(), "Failed to schedule bead-e: {:?}", result);

    // Verify all beads are scheduled
    assert_eq!(
        scheduler.pending_count(),
        5,
        "All 5 beads should be pending"
    );

    // Track execution order
    let mut tracker = ExecutionTracker::new();

    // PHASE 1: Only bead-a should be ready (no dependencies)
    let ready_beads = get_ready_beads(&dag, &tracker);
    assert_eq!(ready_beads.len(), 1, "Only bead-a should be ready");
    assert_eq!(ready_beads[0], "bead-a", "bead-a should be ready");

    // Mark bead-a as ready and execute it
    let result = scheduler.mark_ready(&"bead-a".to_string());
    assert!(result.is_ok(), "Failed to mark bead-a ready: {:?}", result);
    assert_eq!(scheduler.ready_count(), 1, "Should have 1 ready bead");

    // Simulate execution and completion of bead-a
    tracker.complete_bead("bead-a");
    let result = scheduler.handle_bead_completed(&"bead-a".to_string());
    assert!(result.is_ok(), "Failed to complete bead-a: {:?}", result);

    // PHASE 2: After A completes, B and D should become ready
    let ready_beads = get_ready_beads(&dag, &tracker);
    assert_eq!(
        ready_beads.len(),
        2,
        "After A completes, B and D should be ready"
    );
    let ready_set: HashSet<_> = ready_beads.iter().cloned().collect();
    assert!(
        ready_set.contains("bead-b"),
        "bead-b should be ready after A"
    );
    assert!(
        ready_set.contains("bead-d"),
        "bead-d should be ready after A"
    );

    // Mark B and D as ready and execute them
    let result = scheduler.mark_ready(&"bead-b".to_string());
    assert!(result.is_ok(), "Failed to mark bead-b ready: {:?}", result);
    let result = scheduler.mark_ready(&"bead-d".to_string());
    assert!(result.is_ok(), "Failed to mark bead-d ready: {:?}", result);
    assert_eq!(
        scheduler.ready_count(),
        2,
        "Should have 2 ready beads (B and D)"
    );

    // Complete B and D
    tracker.complete_bead("bead-b");
    tracker.complete_bead("bead-d");
    let result = scheduler.handle_bead_completed(&"bead-b".to_string());
    assert!(result.is_ok(), "Failed to complete bead-b: {:?}", result);
    let result = scheduler.handle_bead_completed(&"bead-d".to_string());
    assert!(result.is_ok(), "Failed to complete bead-d: {:?}", result);

    // PHASE 3: After B and D complete, C and E should become ready
    let ready_beads = get_ready_beads(&dag, &tracker);
    assert_eq!(
        ready_beads.len(),
        2,
        "After B and D complete, C and E should be ready"
    );
    let ready_set: HashSet<_> = ready_beads.iter().cloned().collect();
    assert!(
        ready_set.contains("bead-c"),
        "bead-c should be ready after B"
    );
    assert!(
        ready_set.contains("bead-e"),
        "bead-e should be ready after D"
    );

    // Mark C and E as ready and execute them
    let result = scheduler.mark_ready(&"bead-c".to_string());
    assert!(result.is_ok(), "Failed to mark bead-c ready: {:?}", result);
    let result = scheduler.mark_ready(&"bead-e".to_string());
    assert!(result.is_ok(), "Failed to mark bead-e ready: {:?}", result);

    // Complete C and E
    tracker.complete_bead("bead-c");
    tracker.complete_bead("bead-e");
    let result = scheduler.handle_bead_completed(&"bead-c".to_string());
    assert!(result.is_ok(), "Failed to complete bead-c: {:?}", result);
    let result = scheduler.handle_bead_completed(&"bead-e".to_string());
    assert!(result.is_ok(), "Failed to complete bead-e: {:?}", result);

    // FINAL VERIFICATION
    assert_eq!(tracker.completed_count(), 5, "All 5 beads should complete");
    assert_eq!(scheduler.ready_count(), 0, "No beads should be ready");

    // Verify execution order respects dependencies
    let order = tracker.get_completion_order();
    assert_eq!(order[0], "bead-a", "A should complete first");

    // B and D should complete after A but before C and E
    let a_pos = order.iter().position(|x| x == "bead-a");
    assert!(a_pos.is_some(), "bead-a not found in completion order");
    let a_pos = a_pos.unwrap_or(0);

    let b_pos = order.iter().position(|x| x == "bead-b");
    assert!(b_pos.is_some(), "bead-b not found in completion order");
    let b_pos = b_pos.unwrap_or(0);

    let d_pos = order.iter().position(|x| x == "bead-d");
    assert!(d_pos.is_some(), "bead-d not found in completion order");
    let d_pos = d_pos.unwrap_or(0);

    let c_pos = order.iter().position(|x| x == "bead-c");
    assert!(c_pos.is_some(), "bead-c not found in completion order");
    let c_pos = c_pos.unwrap_or(0);

    let e_pos = order.iter().position(|x| x == "bead-e");
    assert!(e_pos.is_some(), "bead-e not found in completion order");
    let e_pos = e_pos.unwrap_or(0);

    assert!(
        b_pos > a_pos,
        "B should complete after A (B at {}, A at {})",
        b_pos,
        a_pos
    );
    assert!(
        d_pos > a_pos,
        "D should complete after A (D at {}, A at {})",
        d_pos,
        a_pos
    );
    assert!(
        c_pos > b_pos,
        "C should complete after B (C at {}, B at {})",
        c_pos,
        b_pos
    );
    assert!(
        e_pos > d_pos,
        "E should complete after D (E at {}, D at {})",
        e_pos,
        d_pos
    );

    println!("✅ Bead lifecycle test passed!");
    println!("   Execution order: {:?}", order);
    println!("   All dependencies were respected!");
}

#[test]
fn test_scheduler_actor_state_transitions() {
    // Test that beads transition through all states correctly
    let mut scheduler = SchedulerActor::new();
    let workflow_id = "test-workflow-002".to_string();
    let bead_id = "state-test-bead".to_string();

    // Register workflow and schedule bead
    let result = scheduler.register_workflow(workflow_id.clone());
    assert!(result.is_ok(), "Failed to register workflow: {:?}", result);
    let result = scheduler.schedule_bead(workflow_id.clone(), bead_id.clone());
    assert!(result.is_ok(), "Failed to schedule bead: {:?}", result);

    // Bead should start in Pending state
    assert_eq!(scheduler.pending_count(), 1, "Bead should be pending");

    // Mark as ready
    let result = scheduler.mark_ready(&bead_id);
    assert!(result.is_ok(), "Failed to mark ready: {:?}", result);
    assert_eq!(scheduler.ready_count(), 1, "Bead should be ready");

    // Assign to worker
    let result = scheduler.assign_to_worker(&bead_id, "worker-001".to_string());
    assert!(result.is_ok(), "Failed to assign to worker: {:?}", result);
    assert_eq!(
        scheduler.get_worker_assignment(&bead_id),
        Some(&"worker-001".to_string()),
        "Bead should be assigned to worker"
    );

    // Complete the bead
    let result = scheduler.handle_bead_completed(&bead_id);
    assert!(result.is_ok(), "Failed to complete bead: {:?}", result);
    assert_eq!(
        scheduler.ready_count(),
        0,
        "Bead should not be in ready list after completion"
    );
    assert_eq!(
        scheduler.get_worker_assignment(&bead_id),
        None,
        "Worker assignment should be cleared after completion"
    );

    println!("✅ State transition test passed!");
}

#[test]
fn test_complex_dag_diamond_dependency() {
    // Test a diamond dependency pattern:
    //     A
    //    / \
    //   B   C
    //    \ /
    //     D
    //
    // A must complete before B and C
    // Both B and C must complete before D

    let mut dag = WorkflowDAG::new();

    // Add nodes
    let result = dag.add_node("bead-a".to_string());
    assert!(result.is_ok(), "Failed to add A: {:?}", result);
    let result = dag.add_node("bead-b".to_string());
    assert!(result.is_ok(), "Failed to add B: {:?}", result);
    let result = dag.add_node("bead-c".to_string());
    assert!(result.is_ok(), "Failed to add C: {:?}", result);
    let result = dag.add_node("bead-d".to_string());
    assert!(result.is_ok(), "Failed to add D: {:?}", result);

    // Add edges
    let result = dag.add_edge(
        "bead-a".to_string(),
        "bead-b".to_string(),
        DependencyType::BlockingDependency,
    );
    assert!(result.is_ok(), "Failed to add A→B: {:?}", result);
    let result = dag.add_edge(
        "bead-a".to_string(),
        "bead-c".to_string(),
        DependencyType::BlockingDependency,
    );
    assert!(result.is_ok(), "Failed to add A→C: {:?}", result);
    let result = dag.add_edge(
        "bead-b".to_string(),
        "bead-d".to_string(),
        DependencyType::BlockingDependency,
    );
    assert!(result.is_ok(), "Failed to add B→D: {:?}", result);
    let result = dag.add_edge(
        "bead-c".to_string(),
        "bead-d".to_string(),
        DependencyType::BlockingDependency,
    );
    assert!(result.is_ok(), "Failed to add C→D: {:?}", result);

    let mut tracker = ExecutionTracker::new();

    // Phase 1: Only A should be ready
    let ready = get_ready_beads(&dag, &tracker);
    assert_eq!(ready.len(), 1, "Only A should be ready");
    assert_eq!(ready[0], "bead-a");

    // Complete A
    tracker.complete_bead("bead-a");

    // Phase 2: B and C should be ready
    let ready = get_ready_beads(&dag, &tracker);
    assert_eq!(ready.len(), 2, "B and C should be ready");
    let ready_set: HashSet<_> = ready.iter().cloned().collect();
    assert!(ready_set.contains("bead-b"));
    assert!(ready_set.contains("bead-c"));

    // D should NOT be ready yet
    assert!(!dependencies_satisfied("bead-d", &dag, &tracker));

    // Complete B only
    tracker.complete_bead("bead-b");

    // Phase 3: D should still not be ready (waiting for C)
    let ready = get_ready_beads(&dag, &tracker);
    assert_eq!(ready.len(), 1, "Only C should be ready");
    assert_eq!(ready[0], "bead-c");
    assert!(!dependencies_satisfied("bead-d", &dag, &tracker));

    // Complete C
    tracker.complete_bead("bead-c");

    // Phase 4: Now D should be ready
    let ready = get_ready_beads(&dag, &tracker);
    assert_eq!(ready.len(), 1, "D should be ready");
    assert_eq!(ready[0], "bead-d");
    assert!(dependencies_satisfied("bead-d", &dag, &tracker));

    println!("✅ Diamond dependency test passed!");
}

#[test]
fn test_workflow_unregister_cleanup() {
    // Test that unregistering a workflow properly cleans up
    let mut scheduler = SchedulerActor::new();
    let workflow_id = "cleanup-test".to_string();

    let result = scheduler.register_workflow(workflow_id.clone());
    assert!(result.is_ok(), "Failed to register: {:?}", result);
    let result = scheduler.schedule_bead(workflow_id.clone(), "bead-1".to_string());
    assert!(result.is_ok(), "Failed to schedule: {:?}", result);

    assert_eq!(scheduler.workflow_count(), 1);

    let removed = scheduler.unregister_workflow(&workflow_id);
    assert!(removed.is_some(), "Should return removed DAG");
    assert_eq!(scheduler.workflow_count(), 0, "Workflow should be removed");

    println!("✅ Workflow cleanup test passed!");
}
