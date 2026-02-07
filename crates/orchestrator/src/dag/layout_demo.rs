//! DAG Layout Memoization Demo
//!
//! This demonstrates the performance improvement achieved by memoizing
//! spring force layout calculations.

#![deny(clippy::unwrap_used)]
#![deny(clippy::expect_used)]
#![deny(clippy::panic)]
#![warn(clippy::pedantic)]
#![warn(clippy::nursery)]
#![forbid(unsafe_code)]

use crate::dag::{DependencyType, MemoizedLayout, WorkflowDAG};
use std::collections::HashMap;
use std::time::Duration;

/// Run the demo to show memoization benefits
pub fn run_demo() {
    println!("=== DAG Layout Memoization Demo ===\n");

    // Create test scenarios
    demo_basic_usage();
    demo_performance_comparison();
    demo_cache_behavior();
    demo_realistic_workflow();
}

fn demo_basic_usage() {
    println!("1. Basic Usage Example\n");

    // Create a simple DAG
    let mut dag = WorkflowDAG::new();
    dag.add_node("build".to_string()).unwrap();
    dag.add_node("test".to_string()).unwrap();
    dag.add_node("deploy".to_string()).unwrap();
    dag.add_dependency(
        "build".to_string(),
        "test".to_string(),
        DependencyType::BlockingDependency,
    )
    .unwrap();
    dag.add_dependency(
        "test".to_string(),
        "deploy".to_string(),
        DependencyType::BlockingDependency,
    )
    .unwrap();

    // Create memoized layout
    let layout = dag
        .create_memoized_layout(0.1, 100.0)
        .expect("Should create layout");

    // Get node positions
    let positions = layout.compute_node_positions();
    println!("Node positions:");
    for (node, pos) in positions {
        println!("  {}: ({:.2}, {:.2})", node, pos.x, pos.y);
    }

    // Get edge forces
    let forces = layout.compute_edge_forces();
    println!("\nEdge forces:");
    for ((from, to), (source_force, target_force)) in forces {
        println!(
            "  {} -> {}: source: {:.2}, target: {:.2}",
            from,
            to,
            source_force.magnitude(),
            target_force.magnitude()
        );
    }

    // Get edge paths for rendering
    let paths = layout.compute_edge_paths(15.0);
    println!("\nEdge paths for rendering:");
    for ((from, to), path) in paths {
        println!("  {} -> {}: length: {:.2}", from, to, path.length);
    }
}

fn demo_performance_comparison() {
    println!("\n2. Performance Comparison\n");

    // Create a larger test DAG
    let dag = create_sample_workflow_dag(50);

    // Test different configurations
    let configs = [
        (0.05, 50.0, "Loose springs"),
        (0.1, 100.0, "Medium springs"),
        (0.2, 150.0, "Tight springs"),
    ];

    for (stiffness, rest_length, description) in configs {
        println!("\nTesting: {}", description);

        let layout = dag
            .create_memoized_layout(stiffness, rest_length)
            .expect("Should create layout");

        // Cold cache (first computation)
        let cold_start = std::time::Instant::now();
        let _cold_positions = layout.compute_node_positions();
        let cold_time = cold_start.elapsed();

        // Warm cache (second computation - should be much faster)
        let warm_start = std::time::Instant::now();
        let _warm_positions = layout.compute_node_positions();
        let warm_time = warm_start.elapsed();

        let speedup = cold_time.as_nanos() as f64 / warm_time.as_nanos() as f64;

        println!("  Cold cache: {:?}", cold_time);
        println!("  Warm cache: {:?}", warm_time);
        println!("  Speedup: {:.1}x", speedup);
    }
}

fn demo_cache_behavior() {
    println!("\n3. Cache Behavior\n");

    let mut dag = create_sample_workflow_dag(20);

    // Create initial layout
    let mut layout = dag
        .create_memoized_layout(0.1, 100.0)
        .expect("Should create layout");

    // First access - cold cache
    let start = std::time::Instant::now();
    let positions1 = layout.compute_node_positions();
    let cold_time = start.elapsed();
    println!("First access (cold cache): {:?}", cold_time);

    // Second access - warm cache
    let start = std::time::Instant::now();
    let positions2 = layout.compute_node_positions();
    let warm_time = start.elapsed();
    println!("Second access (warm cache): {:?}", warm_time);

    // Verify positions are the same (cached)
    assert_eq!(positions1.len(), positions2.len());
    println!(
        "Positions match: {}",
        positions1
            .iter()
            .zip(&positions2)
            .all(|(a, b)| a.0 == b.0 && a.1 == b.1)
    );

    // Add a new node to the DAG
    dag.add_node("new-node".to_string()).unwrap();
    dag.add_dependency(
        "node-19".to_string(),
        "new-node".to_string(),
        DependencyType::BlockingDependency,
    )
    .unwrap();

    // Invalidate cache and recompute
    layout.invalidate_cache();

    let start = std::time::Instant::now();
    let positions3 = layout.compute_node_positions();
    let recompute_time = start.elapsed();
    println!("After cache invalidation: {:?}", recompute_time);

    // Verify positions changed
    assert!(positions3.len() == positions1.len() + 1);
    println!("After adding node, positions count: {}", positions3.len());
}

fn demo_realistic_workflow() {
    println!("\n4. Realistic Workflow Example\n");

    // Create a realistic CI/CD workflow DAG
    let workflow_dag = create_ci_cd_workflow();

    // Create layout with appropriate parameters
    let layout = workflow_dag
        .create_memoized_layout(0.15, 120.0)
        .expect("Should create layout");

    // Compute positions and paths
    let positions = layout.compute_node_positions();
    let paths = layout.compute_edge_paths(20.0);

    // Simulate repeated access (like in a UI that updates frequently)
    println!("Simulating UI updates with repeated layout access...");

    let updates = 100;
    let start = std::time::Instant::now();

    for _ in 0..updates {
        // Simulate UI redrawing with layout
        let _current_positions = layout.compute_node_positions();
        let _current_paths = layout.compute_edge_paths(20.0);
    }

    let total_time = start.elapsed();
    let avg_time = total_time.as_nanos() as f64 / updates as f64;

    println!("Total time for {} UI updates: {:?}", updates, total_time);
    println!("Average time per update: {:.2} ns", avg_time);

    // Show workflow structure
    println!("\nWorkflow structure:");
    let beads = workflow_dag.nodes().collect::<Vec<_>>();
    for bead in beads {
        let ready_beads = workflow_dag.get_ready_beads(&im::HashSet::new());
        let ready_status = if ready_beads.contains(bead) {
            "READY"
        } else {
            "BLOCKED"
        };
        println!("  {}: {}", bead, ready_status);
    }
}

// Helper functions
fn create_sample_workflow_dag(size: usize) -> WorkflowDAG {
    let mut dag = WorkflowDAG::new();

    // Add nodes
    for i in 0..size {
        dag.add_node(format!("node-{}", i)).unwrap();
    }

    // Create dependencies in a realistic pattern
    for i in 0..size {
        if i % 5 == 0 && i + 1 < size {
            // Branch every 5th node
            dag.add_dependency(
                format!("node-{}", i),
                format!("node-{}", i + 1),
                DependencyType::BlockingDependency,
            )
            .unwrap();
            dag.add_dependency(
                format!("node-{}", i),
                format!("node-{}", i + 2),
                DependencyType::BlockingDependency,
            )
            .unwrap();
        } else if i + 1 < size {
            // Linear chain
            dag.add_dependency(
                format!("node-{}", i),
                format!("node-{}", i + 1),
                DependencyType::BlockingDependency,
            )
            .unwrap();
        }
    }

    dag
}

fn create_ci_cd_workflow() -> WorkflowDAG {
    let mut dag = WorkflowDAG::new();

    // Define CI/CD stages
    let stages = vec![
        "setup",
        "lint",
        "test",
        "build",
        "security",
        "deploy-staging",
        "deploy-prod",
    ];

    // Add all stages as nodes
    for stage in stages {
        dag.add_node(stage.to_string()).unwrap();
    }

    // Define dependencies
    let dependencies = vec![
        ("setup", "lint"),
        ("setup", "test"),
        ("lint", "test"),
        ("test", "build"),
        ("build", "security"),
        ("security", "deploy-staging"),
        ("deploy-staging", "deploy-prod"),
    ];

    for (from, to) in dependencies {
        dag.add_dependency(
            from.to_string(),
            to.to_string(),
            DependencyType::BlockingDependency,
        )
        .unwrap();
    }

    dag
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_demo_functions() {
        // Test that demo functions run without errors
        demo_basic_usage();
        demo_performance_comparison();
        demo_cache_behavior();
        demo_realistic_workflow();
    }

    #[test]
    fn test_ci_cd_workflow_creation() {
        let workflow = create_ci_cd_workflow();
        assert_eq!(workflow.node_count(), 7);

        // Check some key dependencies exist
        assert!(
            workflow
                .get_dependencies(&"test".to_string())
                .unwrap()
                .contains(&"lint".to_string())
        );
        assert!(
            workflow
                .get_dependencies(&"deploy-prod".to_string())
                .unwrap()
                .contains(&"deploy-staging".to_string())
        );
    }
}
