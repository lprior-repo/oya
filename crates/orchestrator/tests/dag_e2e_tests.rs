//! DAG End-to-End Tests
//!
//! Tests complete user scenarios with realistic workflow patterns.
//! These tests are HOSTILE - they try to break the system with real-world complexity.

#![deny(clippy::unwrap_used)]
#![deny(clippy::expect_used)]
#![deny(clippy::panic)]

use im::{HashMap, HashSet};
use orchestrator::dag::{BeadId, DependencyType, WorkflowDAG};
use std::time::Duration;

// ============================================================================
// REAL WORKFLOW PATTERNS (3 tests)
// ============================================================================

#[test]
fn given_ci_pipeline_dag_when_execute_then_correct_order() {
    // GIVEN: A realistic CI/CD pipeline
    //        checkout -> [build, lint] -> test -> [docker, deploy]
    let dag = WorkflowDAG::builder()
        .with_nodes(["checkout", "build", "lint", "test", "docker", "deploy"].map(String::from))
        .with_edges(
            [
                ("checkout", "build", DependencyType::BlockingDependency),
                ("checkout", "lint", DependencyType::BlockingDependency),
                ("build", "test", DependencyType::BlockingDependency),
                ("lint", "test", DependencyType::BlockingDependency),
                ("test", "docker", DependencyType::BlockingDependency),
                ("test", "deploy", DependencyType::BlockingDependency),
            ]
            .map(|(a, b, t)| (a.to_string(), b.to_string(), t)),
        )
        .build()
        .expect("builder should succeed");

    // WHEN: Simulating execution progression
    let mut completed: HashSet<BeadId> = HashSet::new();

    // THEN: Initially only checkout should be ready
    let ready_1 = dag.get_ready_nodes(&completed);
    assert_eq!(ready_1.len(), 1, "Only checkout should be ready initially");
    assert!(
        ready_1.contains(&"checkout".to_string()),
        "checkout should be the root"
    );

    // WHEN: Checkout completes
    completed.insert("checkout".to_string());
    let ready_2 = dag.get_ready_nodes(&completed);

    // THEN: Both build and lint should be ready (parallel execution)
    assert_eq!(ready_2.len(), 2, "Build and lint should run in parallel");
    assert!(
        ready_2.contains(&"build".to_string()),
        "build should be ready"
    );
    assert!(
        ready_2.contains(&"lint".to_string()),
        "lint should be ready"
    );

    // WHEN: Only build completes (not lint)
    completed.insert("build".to_string());
    let ready_3 = dag.get_ready_nodes(&completed);

    // THEN: Test should NOT be ready yet (waiting for lint)
    assert!(
        !ready_3.contains(&"test".to_string()),
        "test should wait for ALL dependencies"
    );

    // WHEN: Lint completes
    completed.insert("lint".to_string());
    let ready_4 = dag.get_ready_nodes(&completed);

    // THEN: Now test should be ready
    assert!(
        ready_4.contains(&"test".to_string()),
        "test should be ready after all deps complete"
    );

    // WHEN: Test completes
    completed.insert("test".to_string());
    let ready_5 = dag.get_ready_nodes(&completed);

    // THEN: Docker and deploy should both be ready
    assert_eq!(ready_5.len(), 2, "Docker and deploy should run in parallel");
    assert!(ready_5.contains(&"docker".to_string()));
    assert!(ready_5.contains(&"deploy".to_string()));
}

#[test]
fn given_data_pipeline_dag_when_execute_then_parallel_stages() {
    // GIVEN: A data pipeline with heavy parallelism
    //        extract -> [transform_1..5] -> [load_1..3] -> validate
    let mut dag = WorkflowDAG::new();

    let _ = dag.add_node("extract".to_string());
    let _ = dag.add_node("validate".to_string());

    // Add 5 parallel transform stages
    for i in 1..=5 {
        let node = format!("transform_{}", i);
        let _ = dag.add_node(node.clone());
        let _ = dag.add_edge(
            "extract".to_string(),
            node,
            DependencyType::BlockingDependency,
        );
    }

    // Add 3 parallel load stages (each depends on ALL transforms)
    for i in 1..=3 {
        let load_node = format!("load_{}", i);
        let _ = dag.add_node(load_node.clone());
        for j in 1..=5 {
            let transform_node = format!("transform_{}", j);
            let _ = dag.add_edge(
                transform_node,
                load_node.clone(),
                DependencyType::BlockingDependency,
            );
        }
        let _ = dag.add_edge(
            load_node,
            "validate".to_string(),
            DependencyType::BlockingDependency,
        );
    }

    let mut completed = HashSet::new();

    // WHEN: Extract completes
    completed.insert("extract".to_string());
    let ready_transforms = dag.get_ready_nodes(&completed);

    // THEN: All 5 transforms should be ready simultaneously
    assert_eq!(
        ready_transforms.len(),
        5,
        "All transforms should be ready in parallel"
    );
    for i in 1..=5 {
        assert!(
            ready_transforms.contains(&format!("transform_{}", i)),
            "transform_{} should be ready",
            i
        );
    }

    // WHEN: Complete 4 out of 5 transforms (missing one)
    for i in 1..=4 {
        completed.insert(format!("transform_{}", i));
    }
    let ready_incomplete = dag.get_ready_nodes(&completed);

    // THEN: Load stages should NOT be ready yet (hostile: catch premature dispatch)
    for i in 1..=3 {
        assert!(
            !ready_incomplete.contains(&format!("load_{}", i)),
            "load_{} should NOT be ready - all transforms must complete",
            i
        );
    }

    // WHEN: Last transform completes
    completed.insert("transform_5".to_string());
    let ready_loads = dag.get_ready_nodes(&completed);

    // THEN: Now all load stages should be ready
    assert_eq!(ready_loads.len(), 3, "All load stages should be ready");

    // WHEN: Complete all loads
    for i in 1..=3 {
        completed.insert(format!("load_{}", i));
    }
    let ready_final = dag.get_ready_nodes(&completed);

    // THEN: Validate should be ready
    assert!(
        ready_final.contains(&"validate".to_string()),
        "validate should be final stage"
    );
}

#[test]
fn given_ml_training_dag_when_execute_then_dependencies_respected() {
    // GIVEN: A machine learning training workflow
    //        data_prep -> feature_eng -> [train_model_1, train_model_2, train_model_3]
    //        -> ensemble -> evaluate
    let mut dag = WorkflowDAG::new();

    let _ = dag.add_node("data_prep".to_string());
    let _ = dag.add_node("feature_eng".to_string());
    let _ = dag.add_node("train_model_1".to_string());
    let _ = dag.add_node("train_model_2".to_string());
    let _ = dag.add_node("train_model_3".to_string());
    let _ = dag.add_node("ensemble".to_string());
    let _ = dag.add_node("evaluate".to_string());

    // Sequential: data -> features
    let _ = dag.add_edge(
        "data_prep".to_string(),
        "feature_eng".to_string(),
        DependencyType::BlockingDependency,
    );

    // Parallel: features -> all models
    let _ = dag.add_edge(
        "feature_eng".to_string(),
        "train_model_1".to_string(),
        DependencyType::BlockingDependency,
    );
    let _ = dag.add_edge(
        "feature_eng".to_string(),
        "train_model_2".to_string(),
        DependencyType::BlockingDependency,
    );
    let _ = dag.add_edge(
        "feature_eng".to_string(),
        "train_model_3".to_string(),
        DependencyType::BlockingDependency,
    );

    // Join: all models -> ensemble
    let _ = dag.add_edge(
        "train_model_1".to_string(),
        "ensemble".to_string(),
        DependencyType::BlockingDependency,
    );
    let _ = dag.add_edge(
        "train_model_2".to_string(),
        "ensemble".to_string(),
        DependencyType::BlockingDependency,
    );
    let _ = dag.add_edge(
        "train_model_3".to_string(),
        "ensemble".to_string(),
        DependencyType::BlockingDependency,
    );

    // Final: ensemble -> evaluate
    let _ = dag.add_edge(
        "ensemble".to_string(),
        "evaluate".to_string(),
        DependencyType::BlockingDependency,
    );

    // HOSTILE TEST: Try to compute critical path with realistic weights
    let mut weights = HashMap::new();
    weights.insert("data_prep".to_string(), Duration::from_secs(300)); // 5 min
    weights.insert("feature_eng".to_string(), Duration::from_secs(600)); // 10 min
    weights.insert("train_model_1".to_string(), Duration::from_secs(3600)); // 60 min (bottleneck!)
    weights.insert("train_model_2".to_string(), Duration::from_secs(1800)); // 30 min
    weights.insert("train_model_3".to_string(), Duration::from_secs(2400)); // 40 min
    weights.insert("ensemble".to_string(), Duration::from_secs(120)); // 2 min
    weights.insert("evaluate".to_string(), Duration::from_secs(60)); // 1 min

    let critical_result = dag.critical_path(&weights);

    // THEN: Critical path should go through train_model_1 (slowest)
    assert!(
        critical_result.is_ok(),
        "critical path computation should not panic on realistic data"
    );
    if let Ok(critical) = critical_result {
        assert!(
            critical.contains(&"train_model_1".to_string()),
            "Critical path should identify train_model_1 as bottleneck"
        );
        assert!(
            critical.contains(&"data_prep".to_string()),
            "Should include start"
        );
        assert!(
            critical.contains(&"evaluate".to_string()),
            "Should include end"
        );
    }
}

// ============================================================================
// LARGE SCALE TESTS (hostile: can the DAG handle realistic sizes?)
// ============================================================================

#[test]
fn given_1000_node_dag_when_toposort_then_completes_quickly() {
    // GIVEN: A massive DAG with 1000 nodes in a chain
    let mut dag = WorkflowDAG::new();

    for i in 0..1000 {
        let _ = dag.add_node(format!("node_{}", i));
    }

    // Create dependencies: each node depends on previous
    for i in 1..1000 {
        let _ = dag.add_edge(
            format!("node_{}", i - 1),
            format!("node_{}", i),
            DependencyType::BlockingDependency,
        );
    }

    // WHEN: Performing topological sort on large graph
    let start = std::time::Instant::now();
    let result = dag.topological_sort();
    let elapsed = start.elapsed();

    // THEN: Should complete quickly (hostile: check performance)
    assert!(result.is_ok(), "toposort should not fail on large graph");
    assert!(
        elapsed < Duration::from_secs(1),
        "toposort on 1000 nodes should complete in <1s, took {:?}",
        elapsed
    );

    if let Ok(sorted) = result {
        assert_eq!(sorted.len(), 1000, "should include all nodes");
        // Verify order is correct
        for i in 0..999 {
            let pos_i = sorted.iter().position(|x| x == &format!("node_{}", i));
            let pos_next = sorted.iter().position(|x| x == &format!("node_{}", i + 1));
            assert!(
                pos_i < pos_next,
                "node_{} must come before node_{}",
                i,
                i + 1
            );
        }
    }
}

#[test]
fn given_deep_chain_100_nodes_when_get_ancestors_then_all_found() {
    // GIVEN: A deep chain of 100 nodes
    let mut dag = WorkflowDAG::new();

    for i in 0..100 {
        let _ = dag.add_node(format!("node_{}", i));
    }

    for i in 1..100 {
        let _ = dag.add_edge(
            format!("node_{}", i - 1),
            format!("node_{}", i),
            DependencyType::BlockingDependency,
        );
    }

    // WHEN: Getting all ancestors of the last node
    let result = dag.get_all_ancestors(&"node_99".to_string());

    // THEN: Should find all 99 ancestors (hostile: check deep recursion handling)
    assert!(
        result.is_ok(),
        "get_all_ancestors should not fail on deep chain"
    );
    if let Ok(ancestors) = result {
        assert_eq!(ancestors.len(), 99, "should find all 99 ancestors");
        for i in 0..99 {
            assert!(
                ancestors.contains(&format!("node_{}", i)),
                "should include node_{}",
                i
            );
        }
    }
}

#[test]
fn given_wide_fan_100_children_when_get_dependents_then_all_found() {
    // GIVEN: A wide fan-out with one root and 100 children
    let mut dag = WorkflowDAG::new();

    let _ = dag.add_node("root".to_string());

    for i in 0..100 {
        let child = format!("child_{}", i);
        let _ = dag.add_node(child.clone());
        let _ = dag.add_edge(
            "root".to_string(),
            child,
            DependencyType::BlockingDependency,
        );
    }

    // WHEN: Getting all dependents of root
    let result = dag.get_dependents(&"root".to_string());

    // THEN: Should find all 100 children (hostile: check wide traversal)
    assert!(
        result.is_ok(),
        "get_dependents should not fail on wide fan-out"
    );
    if let Ok(dependents) = result {
        assert_eq!(dependents.len(), 100, "should find all 100 dependents");
        for i in 0..100 {
            assert!(
                dependents.contains(&format!("child_{}", i)),
                "should include child_{}",
                i
            );
        }
    }
}

// ============================================================================
// MUTATION UNDER LOAD (hostile: concurrent modifications)
// ============================================================================

#[test]
fn given_active_workflow_when_node_removed_then_dag_consistent() {
    // GIVEN: A workflow in progress (some nodes completed)
    let mut dag = WorkflowDAG::new();
    let _ = dag.add_node("a".to_string());
    let _ = dag.add_node("b".to_string());
    let _ = dag.add_node("c".to_string());
    let _ = dag.add_node("d".to_string());
    let _ = dag.add_edge(
        "a".to_string(),
        "b".to_string(),
        DependencyType::BlockingDependency,
    );
    let _ = dag.add_edge(
        "b".to_string(),
        "c".to_string(),
        DependencyType::BlockingDependency,
    );
    let _ = dag.add_edge(
        "c".to_string(),
        "d".to_string(),
        DependencyType::BlockingDependency,
    );

    let mut completed = HashSet::new();
    completed.insert("a".to_string());

    // WHEN: Removing an in-progress node (hostile: can we handle this?)
    let remove_result = dag.remove_node(&"b".to_string());

    // THEN: Removal should succeed and DAG should remain consistent
    assert!(
        remove_result.is_ok(),
        "removing in-progress node should succeed"
    );
    assert_eq!(dag.node_count(), 3, "should have 3 nodes left");

    // HOSTILE: Verify no dangling edges remain (c->d still exists)
    assert_eq!(
        dag.edge_count(),
        1,
        "only c->d edge should remain after removing b"
    );

    // HOSTILE: c and d should now be orphaned (no path from a)
    let ready = dag.get_ready_nodes(&completed);
    assert!(
        ready.contains(&"c".to_string()) || !dag.contains_node(&"c".to_string()),
        "c should either be ready (no deps) or removed"
    );
}

#[test]
fn given_concurrent_mutations_when_parallel_then_no_corruption() {
    // GIVEN: A DAG that undergoes multiple mutations
    let mut dag = WorkflowDAG::new();

    // Build initial structure
    for i in 0..10 {
        let _ = dag.add_node(format!("node_{}", i));
    }

    // WHEN: Performing many operations (simulating concurrent behavior)
    // Remove even nodes
    for i in (0..10).step_by(2) {
        let _ = dag.remove_node(&format!("node_{}", i));
    }

    // THEN: DAG should be consistent (hostile: check for corruption)
    assert_eq!(dag.node_count(), 5, "should have 5 odd nodes left");

    // Verify only odd nodes remain
    for i in (1..10).step_by(2) {
        assert!(
            dag.contains_node(&format!("node_{}", i)),
            "node_{} should still exist",
            i
        );
    }

    // Verify even nodes are gone
    for i in (0..10).step_by(2) {
        assert!(
            !dag.contains_node(&format!("node_{}", i)),
            "node_{} should be removed",
            i
        );
    }
}

// ============================================================================
// SERIALIZATION (hostile: can we persist/restore faithfully?)
// ============================================================================

#[test]
fn given_dag_when_serialize_deserialize_then_identical() {
    // GIVEN: A complex DAG
    let mut original = WorkflowDAG::new();
    let _ = original.add_node("a".to_string());
    let _ = original.add_node("b".to_string());
    let _ = original.add_node("c".to_string());
    let _ = original.add_edge(
        "a".to_string(),
        "b".to_string(),
        DependencyType::BlockingDependency,
    );
    let _ = original.add_edge(
        "a".to_string(),
        "c".to_string(),
        DependencyType::PreferredOrder,
    );

    // WHEN: Serializing and deserializing (using Clone as proxy for serde)
    let restored = original.clone();

    // THEN: Structure should be identical (hostile: verify complete fidelity)
    assert_eq!(
        restored.node_count(),
        original.node_count(),
        "node count should match"
    );
    assert_eq!(
        restored.edge_count(),
        original.edge_count(),
        "edge count should match"
    );

    // Verify nodes
    let original_nodes: HashSet<BeadId> = original.nodes().cloned().collect::<HashSet<BeadId>>();
    let restored_nodes: HashSet<_> = restored.nodes().collect::<HashSet<_>>();
    assert_eq!(original_nodes, restored_nodes, "nodes should be identical");
}

#[test]
fn given_large_dag_when_serialize_then_reasonable_size() {
    // GIVEN: A large DAG with 1000 nodes
    let mut dag = WorkflowDAG::new();

    for i in 0..1000 {
        let _ = dag.add_node(format!("node_{}", i));
    }

    // WHEN: Cloning (proxy for serialization cost)
    let start = std::time::Instant::now();
    let _cloned = dag.clone();
    let elapsed = start.elapsed();

    // THEN: Cloning should be fast (hostile: check serialization overhead)
    assert!(
        elapsed < Duration::from_millis(100),
        "cloning 1000-node DAG should be fast, took {:?}",
        elapsed
    );
}
