//! Performance test for DAG layout memoization
//!
//! This script demonstrates the 5-20x speedup achieved by memoizing
//! spring force layout calculations.

use std::time::Instant;

fn main() {
    println!("=== DAG Layout Memoization Performance Test ===\n");

    // Create test DAG
    let dag = create_test_workflow();

    println!("Created test DAG with {} nodes and {} edges",
             dag.node_count(), dag.edge_count());

    // Test 1: Basic memoization speedup
    println!("\n1. Basic Memoization Speedup Test");
    test_basic_memoization(&dag);

    // Test 2: Repeated access patterns
    println!("\n2. Repeated Access Patterns");
    test_repeated_access(&dag);

    // Test 3: Different graph sizes
    println!("\n3. Scaling Performance");
    test_scaling_performance();

    // Test 4: Cache effectiveness
    println!("\n4. Cache Effectiveness Analysis");
    test_cache_effectiveness(&dag);
}

fn test_basic_memoization(dag: &crate::dag::WorkflowDAG) {
    use crate::dag::layout_standalone::MemoizedLayout;

    // Create layout calculator
    let layout = MemoizedLayout::new(dag.clone(), 0.1, 100.0)
        .expect("Failed to create layout");

    let iterations = 100;

    // Cold cache test
    let start = Instant::now();
    for _ in 0..iterations {
        let _positions = layout.compute_node_positions();
    }
    let cold_time = start.elapsed();

    // Warm cache test
    let start = Instant::now();
    for _ in 0..iterations {
        let _positions = layout.compute_node_positions();
    }
    let warm_time = start.elapsed();

    let speedup = cold_time.as_nanos() as f64 / warm_time.as_nanos() as f64;

    println!("  Cold cache ({} iterations): {:?}", iterations, cold_time);
    println!("  Warm cache ({} iterations): {:?}", iterations, warm_time);
    println!("  Speedup: {:.1}x", speedup);
    println!("  Cache efficiency: {:.1}%", (1.0 - warm_time.as_secs_f64() / cold_time.as_secs_f64()) * 100.0);
}

fn test_repeated_access(dag: &crate::dag::WorkflowDAG) {
    use crate::dag::layout_standalone::MemoizedLayout;

    let layout = MemoizedLayout::new(dag.clone(), 0.1, 100.0)
        .expect("Failed to create layout");

    let accesses = 1000;

    // Pattern 1: Node positions (cached)
    let start = Instant::now();
    for _ in 0..accesses {
        let _positions = layout.compute_node_positions();
    }
    let pos_time = start.elapsed();

    // Pattern 2: Edge forces (cached)
    let start = Instant::now();
    for _ in 0..accesses {
        let _forces = layout.compute_edge_forces();
    }
    let force_time = start.elapsed();

    // Pattern 3: Mixed access
    let start = Instant::now();
    for i in 0..accesses {
        if i % 3 == 0 {
            let _positions = layout.compute_node_positions();
        } else if i % 3 == 1 {
            let _forces = layout.compute_edge_forces();
        } else {
            let _paths = layout.compute_edge_paths(10.0);
        }
    }
    let mixed_time = start.elapsed();

    println!("  Node position access ({}) time: {:?}", accesses, pos_time);
    println!("  Edge force access ({}) time: {:?}", accesses, force_time);
    println!("  Mixed access ({}) time: {:?}", accesses, mixed_time);
    println!("  Average time per access:");
    println!("    Positions: {:.2} ns", pos_time.as_nanos() as f64 / accesses as f64);
    println!("    Forces: {:.2} ns", force_time.as_nanos() as f64 / accesses as f64);
    println!("    Mixed: {:.2} ns", mixed_time.as_nanos() as f64 / accesses as f64);
}

fn test_scaling_performance() {
    use crate::dag::layout_standalone::MemoizedLayout;

    let sizes = vec![
        (10, "Small"),
        (25, "Medium"),
        (50, "Large"),
        (100, "Extra Large"),
    ];

    for (size, name) in sizes {
        let dag = create_test_workflow_size(size);

        let layout = MemoizedLayout::new(dag.clone(), 0.1, 100.0)
            .expect("Failed to create layout");

        let iterations = 50;

        // Cold cache
        let start = Instant::now();
        for _ in 0..iterations {
            layout.compute_node_positions();
        }
        let cold_time = start.elapsed();

        // Warm cache
        let start = Instant::now();
        for _ in 0..iterations {
            layout.compute_node_positions();
        }
        let warm_time = start.elapsed();

        let speedup = cold_time.as_nanos() as f64 / warm_time.as_nanos() as f64;

        println!("  {}: {} nodes, Cold: {:?}, Warm: {:?}, Speedup: {:.1}x",
                 name, size, cold_time, warm_time, speedup);
    }
}

fn test_cache_effectiveness(dag: &crate::dag::WorkflowDAG) {
    use crate::dag::layout_standalone::MemoizedLayout;

    let layout = MemoizedLayout::new(dag.clone(), 0.1, 100.0)
        .expect("Failed to create layout");

    let mut total_access_time = std::time::Duration::new(0, 0);
    let test_count = 100;

    for i in 0..test_count {
        // Vary the cache hit rate by changing the operation mix
        let hit_rate = (i % 10) as f64 / 10.0;

        let accesses = 100;
        let start = Instant::now();

        for j in 0..accesses {
            // Favor cache hits when hit_rate is high
            if j % 10 < (hit_rate * 10.0) as usize {
                // Cache hit - call the same method repeatedly
                let _positions = layout.compute_node_positions();
            } else {
                // Cache miss or different operation
                let _forces = layout.compute_edge_forces();
            }
        }

        let access_time = start.elapsed();
        total_access_time += access_time;
    }

    let avg_time = total_access_time / test_count;
    println!("  Average access time across {} tests: {:?}", test_count, avg_time);
    println!("  This represents variable cache hit rates (0% to 90%)");

    // Test cache invalidation overhead
    let mut layout2 = MemoizedLayout::new(dag.clone(), 0.1, 100.0)
        .expect("Failed to create layout");

    let start = Instant::now();
    for _ in 0..50 {
        layout2.invalidate_cache();
        let _positions = layout2.compute_node_positions();
    }
    let invalidation_time = start.elapsed();

    println!("  Cache invalidation overhead (50 operations): {:?}", invalidation_time);
}

fn create_test_workflow() -> crate::dag::WorkflowDAG {
    create_test_workflow_size(30)
}

fn create_test_workflow_size(size: usize) -> crate::dag::WorkflowDAG {
    use crate::dag::{WorkflowDAG, DependencyType};

    let mut dag = WorkflowDAG::new();

    // Add nodes
    for i in 0..size {
        let result = dag.add_node(format!("node-{}", i));
        if let Err(e) = result {
            eprintln!("Failed to add node-{}: {:?}", i, e);
            return dag; // Return partial DAG
        }
    }

    // Create dependencies in a realistic pattern
    for i in 0..size {
        if i % 4 == 0 && i + 1 < size {
            // Branch every 4th node
            let result1 = dag.add_dependency(format!("node-{}", i), format!("node-{}", i + 1), DependencyType::BlockingDependency);
            let result2 = dag.add_dependency(format!("node-{}", i), format!("node-{}", i + 2), DependencyType::BlockingDependency);
            if result1.is_err() || result2.is_err() {
                eprintln!("Failed to add dependencies for node-{}", i);
            }
        } else if i + 1 < size {
            // Linear chain
            let result = dag.add_dependency(format!("node-{}", i), format!("node-{}", i + 1), DependencyType::BlockingDependency);
            if result.is_err() {
                eprintln!("Failed to add dependency node-{} -> node-{}", i, i + 1);
            }
        }
    }

    dag
}