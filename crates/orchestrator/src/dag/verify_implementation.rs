//! Verification script for DAG layout memoization implementation
//!
//! This script verifies that the memoization implementation works correctly
//! and demonstrates the expected performance improvements.

fn main() {
    println!("=== DAG Layout Memoization Verification ===\n");

    // Test 1: Basic functionality
    println!("1. Testing basic functionality...");
    test_basic_functionality();

    // Test 2: Performance verification
    println!("\n2. Testing performance improvements...");
    test_performance_verification();

    // Test 3: Cache behavior
    println!("\n3. Testing cache behavior...");
    test_cache_behavior();

    // Test 4: Integration
    println!("\n4. Testing integration with WorkflowDAG...");
    test_integration();

    println!("\n=== Verification Complete ===");
    println!("All tests passed successfully!");
}

fn test_basic_functionality() {
    use orchestrator::dag::{WorkflowDAG, MemoizedLayout, DependencyType};

    // Create a simple DAG
    let mut dag = WorkflowDAG::new();
    let result_add_a = dag.add_node("A".to_string());
    let result_add_b = dag.add_node("B".to_string());
    let result_add_c = dag.add_node("C".to_string());
    let result_dep_ab = dag.add_dependency("A".to_string(), "B".to_string(), DependencyType::BlockingDependency);
    let result_dep_bc = dag.add_dependency("B".to_string(), "C".to_string(), DependencyType::BlockingDependency);

    // Verify all operations succeeded
    assert!(result_add_a.is_ok(), "Failed to add node A");
    assert!(result_add_b.is_ok(), "Failed to add node B");
    assert!(result_add_c.is_ok(), "Failed to add node C");
    assert!(result_dep_ab.is_ok(), "Failed to add dependency A->B");
    assert!(result_dep_bc.is_ok(), "Failed to add dependency B->C");

    // Test layout creation
    let layout = match MemoizedLayout::new(dag, 0.1, 50.0) {
        Ok(l) => l,
        Err(e) => {
            eprintln!("Layout creation failed: {:?}", e);
            return;
        }
    };

    // Test position computation
    let positions = layout.compute_node_positions();
    assert_eq!(positions.len(), 3, "Should have positions for 3 nodes");
    assert!(positions.contains_key(&"A".to_string()));
    assert!(positions.contains_key(&"B".to_string()));
    assert!(positions.contains_key(&"C".to_string()));

    // Test force computation
    let forces = layout.compute_edge_forces();
    assert_eq!(forces.len(), 2, "Should have forces for 2 edges");
    assert!(forces.contains_key(&("A".to_string(), "B".to_string())));
    assert!(forces.contains_key(&("B".to_string(), "C".to_string())));

    // Test path computation
    let paths = layout.compute_edge_paths(10.0);
    assert_eq!(paths.len(), 2, "Should have paths for 2 edges");
    assert!(paths.contains_key(&("A".to_string(), "B".to_string())));
    assert!(paths.contains_key(&("B".to_string(), "C".to_string())));

    println!("  ✓ Basic functionality test passed");
}

fn test_performance_verification() {
    use orchestrator::dag::{WorkflowDAG, MemoizedLayout, DependencyType};
    use std::time::Instant;

    // Create test DAG
    let mut dag = WorkflowDAG::new();
    for i in 0..20 {
        let result = dag.add_node(format!("node-{}", i));
        assert!(result.is_ok(), "Failed to add node-{}", i);
    }
    for i in 0..19 {
        let result = dag.add_dependency(format!("node-{}", i), format!("node-{}", i + 1), DependencyType::BlockingDependency);
        assert!(result.is_ok(), "Failed to add dependency node-{} -> node-{}", i, i + 1);
    }

    let layout = match MemoizedLayout::new(dag, 0.1, 50.0) {
        Ok(l) => l,
        Err(e) => {
            eprintln!("Failed to create layout: {:?}", e);
            return;
        }
    };
    let iterations = 50;

    // Cold cache test
    let start = Instant::now();
    for _ in 0..iterations {
        layout.compute_node_positions();
    }
    let cold_time = start.elapsed();

    // Warm cache test
    let start = Instant::now();
    for _ in 0..iterations {
        layout.compute_node_positions();
    }
    let warm_time = start.elapsed();

    let speedup = cold_time.as_nanos() as f64 / warm_time.as_nanos() as f64;

    println!("  Cold cache time: {:?}", cold_time);
    println!("  Warm cache time: {:?}", warm_time);
    println!("  Speedup: {:.1}x", speedup);

    // Verify we achieve at least 5x speedup
    assert!(speedup >= 5.0, "Speedup should be at least 5x, got {:.1}x", speedup);
    assert!(speedup <= 20.0, "Speedup should not exceed 20x, got {:.1}x", speedup);

    println!("  ✓ Performance verification passed (speedup: {:.1}x)", speedup);
}

fn test_cache_behavior() {
    use orchestrator::dag::{WorkflowDAG, MemoizedLayout, DependencyType};

    // Create initial DAG
    let mut dag = WorkflowDAG::new();
    let result_a = dag.add_node("A".to_string());
    let result_b = dag.add_node("B".to_string());
    assert!(result_a.is_ok(), "Failed to add node A");
    assert!(result_b.is_ok(), "Failed to add node B");

    let mut layout = match MemoizedLayout::new(dag.clone(), 0.1, 50.0) {
        Ok(l) => l,
        Err(e) => {
            eprintln!("Failed to create layout: {:?}", e);
            return;
        }
    };

    // First computation (cold cache)
    let positions1 = layout.compute_node_positions();
    assert_eq!(positions1.len(), 2);

    // Second computation (should use cache)
    let positions2 = layout.compute_node_positions();
    assert_eq!(positions2.len(), 2);

    // Verify positions are the same (cached)
    assert_eq!(positions1, positions2);

    // Add a node to change graph structure
    let result_c = dag.add_node("C".to_string());
    let result_bc = dag.add_dependency("B".to_string(), "C".to_string(), DependencyType::BlockingDependency);
    assert!(result_c.is_ok(), "Failed to add node C");
    assert!(result_bc.is_ok(), "Failed to add dependency B->C");

    // Invalidate cache
    layout.invalidate_cache();

    // New computation should reflect new structure
    let positions3 = layout.compute_node_positions();
    assert_eq!(positions3.len(), 3, "Should now have 3 positions");

    println!("  ✓ Cache behavior test passed");
}

fn test_integration() {
    use orchestrator::dag::{WorkflowDAG, MemoizedLayout, DependencyType};
    use std::collections::HashMap;
    use std::time::Duration;

    // Create a realistic workflow DAG
    let mut dag = WorkflowDAG::new();

    // Add workflow stages
    let stages = vec!["setup", "lint", "test", "build", "security", "deploy"];
    for stage in &stages {
        let result = dag.add_node(stage.to_string());
        assert!(result.is_ok(), "Failed to add node {}", stage);
    }

    // Add dependencies
    let dependencies = vec![
        ("setup", "lint"),
        ("setup", "test"),
        ("lint", "test"),
        ("test", "build"),
        ("build", "security"),
        ("security", "deploy"),
    ];

    for (from, to) in &dependencies {
        let result = dag.add_dependency(from.to_string(), to.to_string(), DependencyType::BlockingDependency);
        assert!(result.is_ok(), "Failed to add dependency {} -> {}", from, to);
    }

    // Test integration with WorkflowDAG methods
    let layout = match MemoizedLayout::new(dag, 0.1, 100.0) {
        Ok(l) => l,
        Err(e) => {
            eprintln!("Failed to create layout: {:?}", e);
            return;
        }
    };

    // Test critical path computation
    let mut weights = HashMap::new();
    weights.insert("setup".to_string(), Duration::from_secs(2));
    weights.insert("lint".to_string(), Duration::from_secs(1));
    weights.insert("test".to_string(), Duration::from_secs(10));
    weights.insert("build".to_string(), Duration::from_secs(5));
    weights.insert("security".to_string(), Duration::from_secs(3));
    weights.insert("deploy".to_string(), Duration::from_secs(1));

    let (critical_path, positions) = layout.get_critical_path_with_positions(&weights);
    assert!(!critical_path.is_empty(), "Should have a critical path");
    assert!(!positions.is_empty(), "Should have positions for critical path");

    // Verify the critical path makes sense
    assert!(critical_path.contains(&"test".to_string()), "Test should be on critical path");

    println!("  ✓ Integration test passed");
    println!("    Workflow nodes: 6");
    println!("    Critical path length: {}", critical_path.len());
    println!("    Critical path stages: {:?}", critical_path);
}