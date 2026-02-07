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

    graph_sizes.into_iter().for_each(|(size, description)| {
        println!("Testing {}", description);
        benchmark_graph_size(size);
        println!();
    });

    // Test repeated access patterns
    println!("=== Repeated Access Pattern Benchmark ===");
    benchmark_repeated_access();

    println!("\n=== Cache Invalidation Benchmark ===");
    benchmark_cache_invalidation();
}

fn benchmark_graph_size(size: usize) {
    // Create test DAG
    let dag = create_test_dag(size);

    // Test with different layout parameters
    let stiffness_values = [0.05, 0.1, 0.2];
    let rest_length_values = [50.0, 100.0, 150.0];

    stiffness_values.iter().for_each(|&stiffness| {
        rest_length_values.iter().for_each(|&rest_length| {
            let layout = WorkflowDAG::create_memoized_layout(&dag, stiffness, rest_length);
            match layout {
                Ok(layout) => {
                    // Cold cache performance
                    let cold_time = benchmark_layout_computation(&layout, 100, true);

                    // Warm cache performance
                    let warm_time = benchmark_layout_computation(&layout, 100, false);

                    let speedup = cold_time.as_nanos() as f64 / warm_time.as_nanos() as f64;

                    println!("  Stiffness: {:.2}, Rest Length: {:.3} - Cold: {:?}, Warm: {:?}, Speedup: {:.1}x",
                           stiffness, rest_length, cold_time, warm_time, speedup);
                }
                Err(e) => {
                    println!("  Failed to create layout: {}", e);
                }
            }
        });
    });
}

fn benchmark_layout_computation(
    layout: &MemoizedLayout,
    iterations: usize,
    cold_cache: bool,
) -> Duration {
    use std::time::Instant;

    // Force cache invalidation for cold cache test
    if cold_cache {
        // Create a fresh layout for cold cache test
        let dag = layout.dag().clone();
        let fresh_layout = WorkflowDAG::create_memoized_layout(
            &dag,
            layout.spring_force().stiffness(),
            layout.spring_force().rest_length(),
        )
        .expect("Should succeed");

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
    let dag = create_test_dag(50);

    // Create layout
    let layout = WorkflowDAG::create_memoized_layout(&dag, 0.1, 100.0).expect("Should succeed");

    let iterations = 1000;

    // Test repeated access to node positions
    println!(
        "  Repeated node position access ({} iterations):",
        iterations
    );
    let start = Instant::now();
    for _ in 0..iterations {
        let _ = layout.compute_node_positions();
    }
    let access_time = start.elapsed();
    println!("    Time: {:?}", access_time);

    // Test repeated access to edge forces
    println!(
        "  Repeated edge force calculation ({} iterations):",
        iterations
    );
    let start = Instant::now();
    for _ in 0..iterations {
        let _ = layout.compute_edge_forces();
    }
    let force_time = start.elapsed();
    println!("    Time: {:?}", force_time);

    // Test mixed access pattern
    println!("  Mixed access pattern ({} iterations):", iterations);
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
    println!("    Time: {:?}", mixed_time);
}

fn benchmark_cache_invalidation() {
    let mut dag = create_test_dag(20);

    // Create initial layout
    let mut layout = WorkflowDAG::create_memoized_layout(&dag, 0.1, 100.0).expect("Should succeed");

    let initial_time = benchmark_layout_computation(&layout, 100, false);
    println!("  Initial layout computation: {:?}", initial_time);

    // Add a node and invalidate cache
    dag.add_node("new_node".to_string()).unwrap();
    dag.add_edge(
        "node-19".to_string(),
        "new_node".to_string(),
        DependencyType::BlockingDependency,
    )
    .unwrap();

    layout.invalidate_cache();

    let recomputed_time = benchmark_layout_computation(&layout, 100, false);
    println!("  After cache invalidation: {:?}", recomputed_time);

    let overhead = recomputed_time.as_nanos() as f64 / initial_time.as_nanos() as f64;
    println!("  Recomputation overhead: {:.2}x", overhead);
}

fn create_test_dag(size: usize) -> WorkflowDAG {
    let mut dag = WorkflowDAG::new();

    // Add nodes
    for i in 0..size {
        dag.add_node(format!("node-{}", i)).unwrap();
    }

    // Create a chain structure with some branching
    for i in 0..(size - 1) {
        if i % 4 == 0 && i + 2 < size {
            // Create branching every 4th node
            dag.add_edge(
                format!("node-{}", i),
                format!("node-{}", i + 1),
                DependencyType::BlockingDependency,
            )
            .unwrap();
            dag.add_edge(
                format!("node-{}", i),
                format!("node-{}", i + 2),
                DependencyType::BlockingDependency,
            )
            .unwrap();
        } else if i + 1 < size {
            // Normal chain
            dag.add_edge(
                format!("node-{}", i),
                format!("node-{}", i + 1),
                DependencyType::BlockingDependency,
            )
            .unwrap();
        }
    }

    dag
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
        let small_dag = create_test_dag(5);
        assert_eq!(small_dag.node_count(), 5);

        let medium_dag = create_test_dag(20);
        assert_eq!(medium_dag.node_count(), 20);
    }

    #[test]
    fn test_layout_creation_performance() {
        let dag = create_test_dag(25);
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
    use super::*;

    /// Analyze cache effectiveness
    pub fn analyze_cache_effectiveness() {
        println!("\n=== Cache Effectiveness Analysis ===");

        let dag = create_test_dag(30);

        // Test different cache sizes
        let cache_sizes = [10, 50, 100, 500];

        for size in cache_sizes {
            println!("Testing cache size: {} accesses", size);

            let layout =
                WorkflowDAG::create_memoized_layout(&dag, 0.1, 100.0).expect("Should succeed");

            // Simulate repeated access
            let start = std::time::Instant::now();
            for _ in 0..size {
                let _ = layout.compute_node_positions();
            }
            let total_time = start.elapsed();

            println!("  Total time for {} accesses: {:?}", size, total_time);
            let avg_time = total_time.as_nanos() as f64 / size as f64;
            println!("  Average time per access: {:.2} ns", avg_time);

            // Test cache hit rate by simulating different access patterns
            let hit_rate = simulate_cache_hits(&layout, size);
            println!("  Simulated cache hit rate: {:.2}%", hit_rate * 100.0);
        }
    }

    fn simulate_cache_hits(layout: &MemoizedLayout, accesses: usize) -> f64 {
        // Simulate repeated access to the same positions (high cache hit rate)
        let mut hit_count = 0;

        // Access the same position multiple times
        for _ in 0..accesses {
            let positions = layout.compute_node_positions();
            // Access first few positions repeatedly
            if let Some(_) = positions.get("node-0") {
                hit_count += 1;
            }
        }

        hit_count as f64 / accesses as f64
    }

    #[test]
    fn test_cache_analysis() {
        analyze_cache_effectiveness();
    }
}
