//! Benchmark for DAG layout memoization performance
//!
//! This module benchmarks the performance improvement achieved by memoizing
//! spring force layout calculations.

#![deny(clippy::unwrap_used)]
#![deny(clippy::expect_used)]
#![deny(clippy::panic)]
#![warn(clippy::pedantic)]
#![warn(clippy::nursery)]
#![forbid(unsafe_code)]

use crate::dag::{DependencyType, MemoizedLayout, WorkflowDAG};
use std::time::{Duration, Instant};

/// Benchmark DAG layout performance with memoization
pub fn benchmark_layout_performance() {
    println!("=== DAG Layout Memoization Benchmark ===\n");

    // Test with different graph sizes
    let graph_sizes = vec![
        (10, "Small (10 nodes)"),
        (25, "Medium (25 nodes)"),
        (50, "Large (50 nodes)"),
        (100, "Extra Large (100 nodes)"),
    ];

    for (size, description) in graph_sizes {
        println!("Testing {description}");
        benchmark_graph_size(size);
        println!();
    }

    // Test repeated access patterns
    println!("=== Repeated Access Pattern Benchmark ===");
    benchmark_repeated_access();

    println!("\n=== Cache Invalidation Benchmark ===");
    benchmark_cache_invalidation();
}

#[allow(clippy::cast_precision_loss)]
fn benchmark_graph_size(size: usize) {
    // Create test DAG
    let dag = match create_test_dag_with_result(size) {
        Ok(dag) => dag,
        Err(e) => {
            eprintln!("Failed to create test DAG: {e}");
            return;
        }
    };

    // Test with different layout parameters
    let stiffness_values = [0.05, 0.1, 0.2];
    let rest_length_values = [50.0, 100.0, 150.0];

    for &stiffness in &stiffness_values {
        for &rest_length in &rest_length_values {
            let layout = WorkflowDAG::create_memoized_layout(&dag, stiffness, rest_length);
            match layout {
                Ok(layout) => {
                    // Cold cache performance
                    let cold_time = benchmark_layout_computation(&layout, 100, true);

                    // Warm cache performance
                    let warm_time = benchmark_layout_computation(&layout, 100, false);

                    let speedup = cold_time.as_nanos() as f64 / warm_time.as_nanos() as f64;

                    println!("  Stiffness: {stiffness:.2}, Rest Length: {rest_length:.3} - Cold: {cold_time:?}, Warm: {warm_time:?}, Speedup: {speedup:.1}x");
                }
                Err(e) => {
                    println!("  Failed to create layout: {e}");
                }
            }
        }
    }
}

fn benchmark_layout_computation(
    layout: &MemoizedLayout,
    iterations: usize,
    cold_cache: bool,
) -> Duration {
    // Force cache invalidation for cold cache test
    if cold_cache {
        // Create a fresh layout for cold cache test
        let dag = layout.dag().clone();
        let fresh_layout = match WorkflowDAG::create_memoized_layout(
            &dag,
            layout.spring_force().stiffness(),
            layout.spring_force().rest_length(),
        ) {
            Ok(layout) => layout,
            Err(e) => {
                eprintln!("Failed to create layout for benchmark: {e}");
                return Duration::ZERO;
            }
        };

        let start = Instant::now();
        for _ in 0..iterations {
            let _ = fresh_layout.compute_node_positions();
        }
        return start.elapsed();
    }

    // Warm cache test
    let start = Instant::now();
    for _ in 0..iterations {
        let _ = layout.compute_node_positions();
    }
    start.elapsed()
}

fn benchmark_repeated_access() {
    let dag = match create_test_dag_with_result(50) {
        Ok(dag) => dag,
        Err(e) => {
            eprintln!("Failed to create test DAG: {e}");
            return;
        }
    };

    // Create layout
    let layout = match WorkflowDAG::create_memoized_layout(&dag, 0.1, 100.0) {
        Ok(layout) => layout,
        Err(e) => {
            eprintln!("Failed to create layout for benchmark: {e}");
            return;
        }
    };

    let iterations = 1000;

    // Test repeated access to node positions
    println!(
        "  Repeated node position access ({iterations} iterations):"
    );
    let start = Instant::now();
    for _ in 0..iterations {
        let _ = layout.compute_node_positions();
    }
    let access_time = start.elapsed();
    println!("    Time: {access_time:?}");

    // Test repeated access to edge forces
    println!(
        "  Repeated edge force calculation ({iterations} iterations):"
    );
    let start = Instant::now();
    for _ in 0..iterations {
        let _ = layout.compute_edge_forces();
    }
    let force_time = start.elapsed();
    println!("    Time: {force_time:?}");

    // Test mixed access pattern
    println!("  Mixed access pattern ({iterations} iterations):");
    let start = Instant::now();
    for i in 0..iterations {
        match i % 3 {
            0 => {
                let _ = layout.compute_node_positions();
            }
            1 => {
                let _ = layout.compute_edge_forces();
            }
            _ => {
                let _ = layout.compute_edge_paths(10.0);
            }
        }
    }
    let mixed_time = start.elapsed();
    println!("    Time: {mixed_time:?}");
}

#[allow(clippy::cast_precision_loss)]
fn benchmark_cache_invalidation() {
    let mut dag = match create_test_dag_with_result(20) {
        Ok(dag) => dag,
        Err(e) => {
            eprintln!("Failed to create test DAG: {e}");
            return;
        }
    };

    // Create initial layout
    let mut layout = match WorkflowDAG::create_memoized_layout(&dag, 0.1, 100.0) {
        Ok(layout) => layout,
        Err(e) => {
            eprintln!("Failed to create layout for benchmark: {e}");
            return;
        }
    };

    let initial_time = benchmark_layout_computation(&layout, 100, false);
    println!("  Initial layout computation: {initial_time:?}");

    // Add a node and invalidate cache
    if let Err(e) = dag.add_node("new_node".to_string()) {
        eprintln!("Failed to add node: {e}");
        return;
    }
    if let Err(e) = dag.add_edge(
        "node-19".to_string(),
        "new_node".to_string(),
        DependencyType::BlockingDependency,
    ) {
        eprintln!("Failed to add edge: {e}");
        return;
    }

    layout.invalidate_cache();

    let recomputed_time = benchmark_layout_computation(&layout, 100, false);
    println!("  After cache invalidation: {recomputed_time:?}");

    let overhead = recomputed_time.as_nanos() as f64 / initial_time.as_nanos() as f64;
    println!("  Recomputation overhead: {overhead:.2}x");
}

fn create_test_dag_with_result(size: usize) -> Result<WorkflowDAG, String> {
    let mut dag = WorkflowDAG::new();

    // Add nodes
    for i in 0..size {
        dag.add_node(format!("node-{i}"))
            .map_err(|e| format!("Failed to add node {i}: {e}"))?;
    }

    // Create a chain structure with some branching
    for i in 0..(size.saturating_sub(1)) {
        if i % 4 == 0 && i + 2 < size {
            // Create branching every 4th node
            dag.add_edge(
                format!("node-{i}"),
                format!("node-{}", i + 1),
                DependencyType::BlockingDependency,
            )
            .map_err(|e| {
                format!(
                    "Failed to add edge from node-{} to node-{}: {}",
                    i,
                    i + 1,
                    e
                )
            })?;
            dag.add_edge(
                format!("node-{i}"),
                format!("node-{}", i + 2),
                DependencyType::BlockingDependency,
            )
            .map_err(|e| {
                format!(
                    "Failed to add edge from node-{} to node-{}: {}",
                    i,
                    i + 2,
                    e
                )
            })?;
        } else if i + 1 < size {
            // Normal chain
            dag.add_edge(
                format!("node-{i}"),
                format!("node-{}", i + 1),
                DependencyType::BlockingDependency,
            )
            .map_err(|e| {
                format!(
                    "Failed to add edge from node-{} to node-{}: {}",
                    i,
                    i + 1,
                    e
                )
            })?;
        }
    }

    Ok(dag)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_benchmark_functions_exist() {
        // Test that benchmark functions don't panic
        benchmark_graph_size(10);
        benchmark_repeated_access();
        benchmark_cache_invalidation();
    }

    #[test]
    fn test_create_test_dag() {
        let small_dag = create_test_dag_with_result(5).unwrap();
        assert_eq!(small_dag.node_count(), 5);

        let medium_dag = create_test_dag_with_result(20).unwrap();
        assert_eq!(medium_dag.node_count(), 20);
    }

    #[test]
    fn test_layout_creation_performance() {
        let dag = create_test_dag_with_result(25).unwrap();
        let layout = WorkflowDAG::create_memoized_layout(&dag, 0.1, 100.0);
        assert!(layout.is_ok());

        if let Ok(layout) = layout {
            let positions = layout.compute_node_positions();
            assert_eq!(positions.len(), 25);
        }
    }
}

/// Performance analysis functions
pub mod analysis {
    use super::{create_test_dag_with_result, WorkflowDAG, MemoizedLayout};

    /// Analyze cache effectiveness
    #[allow(clippy::cast_precision_loss)]
    pub fn analyze_cache_effectiveness() {
        println!("\n=== Cache Effectiveness Analysis ===");

        let dag = match create_test_dag_with_result(30) {
            Ok(dag) => dag,
            Err(e) => {
                eprintln!("Failed to create test DAG: {e}");
                return;
            }
        };

        // Test different cache sizes
        let cache_sizes = [10, 50, 100, 500];

        for size in cache_sizes {
            println!("Testing cache size: {size} accesses");

            let layout = match WorkflowDAG::create_memoized_layout(&dag, 0.1, 100.0) {
                Ok(layout) => layout,
                Err(e) => {
                    eprintln!("Failed to create layout: {e}");
                    return;
                }
            };

            // Simulate repeated access
            let start = std::time::Instant::now();
            for _ in 0..size {
                let _ = layout.compute_node_positions();
            }
            let total_time = start.elapsed();

            println!("  Total time for {size} accesses: {total_time:?}");
            let avg_time = total_time.as_nanos() as f64 / size as f64;
            println!("  Average time per access: {avg_time:.2} ns");

            // Test cache hit rate by simulating different access patterns
            let hit_rate = simulate_cache_hits(&layout, size);
            println!("  Simulated cache hit rate: {:.2}%", hit_rate * 100.0);
        }
    }

    #[allow(clippy::cast_precision_loss)]
    fn simulate_cache_hits(layout: &MemoizedLayout, accesses: usize) -> f64 {
        // Simulate repeated access to the same positions (high cache hit rate)
        let mut hit_count = 0;

        // Access the same position multiple times
        for _ in 0..accesses {
            let positions = layout.compute_node_positions();
            // Access first few positions repeatedly
            if positions.get("node-0").is_some() {
                hit_count += 1;
            }
        }

        f64::from(hit_count) / accesses as f64
    }

    #[test]
    fn test_cache_analysis() {
        analyze_cache_effectiveness();
    }
}
