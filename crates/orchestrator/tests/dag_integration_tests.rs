//! DAG Integration Tests
//!
//! Tests DAG operations across multiple components and real-world scenarios.
//! Focus: Multi-step workflows, algorithm correctness, cross-cutting concerns

#![deny(clippy::unwrap_used)]
#![deny(clippy::expect_used)]
#![deny(clippy::panic)]

use orchestrator::dag::{DependencyType, WorkflowDAG};
use std::collections::{HashMap, HashSet};
use std::time::Duration;

// ============================================================================
// TOPOLOGICAL SORT INTEGRATION (6 tests)
// ============================================================================

#[test]
fn given_empty_dag_when_toposort_then_empty_vec() {
    // GIVEN: An empty DAG
    let dag = WorkflowDAG::new();

    // WHEN: Performing topological sort
    let result = dag.topological_sort();

    // THEN: Should return empty vector
    assert!(result.is_ok(), "toposort on empty DAG should succeed");
    if let Ok(sorted) = result {
        assert_eq!(sorted.len(), 0, "empty DAG should produce empty ordering");
    }
}

#[test]
fn given_linear_chain_when_toposort_then_correct_order() {
    // GIVEN: A linear chain a -> b -> c -> d
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

    // WHEN: Performing topological sort
    let result = dag.topological_sort();

    // THEN: Order should respect all dependencies
    assert!(result.is_ok(), "toposort should succeed on acyclic DAG");
    if let Ok(sorted) = result {
        let pos_a = sorted.iter().position(|x| x == "a");
        let pos_b = sorted.iter().position(|x| x == "b");
        let pos_c = sorted.iter().position(|x| x == "c");
        let pos_d = sorted.iter().position(|x| x == "d");

        assert!(pos_a < pos_b, "a must come before b");
        assert!(pos_b < pos_c, "b must come before c");
        assert!(pos_c < pos_d, "c must come before d");
    }
}

#[test]
fn given_diamond_when_toposort_then_valid_order() {
    // GIVEN: A diamond DAG: a -> b,c -> d
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
        "a".to_string(),
        "c".to_string(),
        DependencyType::BlockingDependency,
    );
    let _ = dag.add_edge(
        "b".to_string(),
        "d".to_string(),
        DependencyType::BlockingDependency,
    );
    let _ = dag.add_edge(
        "c".to_string(),
        "d".to_string(),
        DependencyType::BlockingDependency,
    );

    // WHEN: Performing topological sort
    let result = dag.topological_sort();

    // THEN: a must come first, d must come last, b/c can be in any order
    assert!(result.is_ok(), "toposort should succeed");
    if let Ok(sorted) = result {
        assert_eq!(sorted.len(), 4, "should have all 4 nodes");
        assert_eq!(&sorted[0], "a", "a should be first (no dependencies)");
        assert_eq!(&sorted[3], "d", "d should be last (depends on all)");
    }
}

#[test]
fn given_disconnected_components_when_toposort_then_all_included() {
    // GIVEN: Two disconnected components: (a -> b) and (c -> d)
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
        "c".to_string(),
        "d".to_string(),
        DependencyType::BlockingDependency,
    );

    // WHEN: Performing topological sort
    let result = dag.topological_sort();

    // THEN: All nodes should be included in some valid order
    assert!(result.is_ok(), "toposort should succeed");
    if let Ok(sorted) = result {
        assert_eq!(sorted.len(), 4, "all nodes should be included");
        // Verify component 1 ordering
        let pos_a = sorted.iter().position(|x| x == "a");
        let pos_b = sorted.iter().position(|x| x == "b");
        assert!(pos_a < pos_b, "a must come before b within component 1");
        // Verify component 2 ordering
        let pos_c = sorted.iter().position(|x| x == "c");
        let pos_d = sorted.iter().position(|x| x == "d");
        assert!(pos_c < pos_d, "c must come before d within component 2");
    }
}

#[test]
fn given_cycle_when_toposort_then_error() {
    // GIVEN: A DAG with a cycle: a -> b -> c -> a
    let mut dag = WorkflowDAG::new();
    let _ = dag.add_node("a".to_string());
    let _ = dag.add_node("b".to_string());
    let _ = dag.add_node("c".to_string());
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
        "a".to_string(),
        DependencyType::BlockingDependency,
    );

    // WHEN: Performing topological sort
    let result = dag.topological_sort();

    // THEN: Should return error indicating cycle
    assert!(result.is_err(), "toposort should fail on cyclic graph");
}

#[test]
fn given_complex_dag_when_toposort_kahn_vs_dfs_then_both_valid() {
    // GIVEN: A complex DAG with multiple valid orderings
    let mut dag = WorkflowDAG::new();
    for i in 0..10 {
        let _ = dag.add_node(format!("node-{}", i));
    }
    // Create complex dependency structure
    let _ = dag.add_edge(
        "node-0".to_string(),
        "node-2".to_string(),
        DependencyType::BlockingDependency,
    );
    let _ = dag.add_edge(
        "node-1".to_string(),
        "node-2".to_string(),
        DependencyType::BlockingDependency,
    );
    let _ = dag.add_edge(
        "node-2".to_string(),
        "node-5".to_string(),
        DependencyType::BlockingDependency,
    );
    let _ = dag.add_edge(
        "node-3".to_string(),
        "node-6".to_string(),
        DependencyType::BlockingDependency,
    );
    let _ = dag.add_edge(
        "node-4".to_string(),
        "node-6".to_string(),
        DependencyType::BlockingDependency,
    );

    // WHEN: Using both algorithms
    let dfs_result = dag.topological_sort();
    let kahn_result = dag.topological_sort_kahn();

    // THEN: Both should succeed and produce valid orderings
    assert!(dfs_result.is_ok(), "DFS toposort should succeed");
    assert!(kahn_result.is_ok(), "Kahn toposort should succeed");

    if let (Ok(dfs_sorted), Ok(kahn_sorted)) = (dfs_result, kahn_result) {
        assert_eq!(dfs_sorted.len(), 10, "DFS should include all nodes");
        assert_eq!(kahn_sorted.len(), 10, "Kahn should include all nodes");

        // Both should respect the dependency: node-0/1 -> node-2 -> node-5
        let dfs_pos_2 = dfs_sorted.iter().position(|x| x == "node-2");
        let dfs_pos_5 = dfs_sorted.iter().position(|x| x == "node-5");
        assert!(dfs_pos_2 < dfs_pos_5, "DFS: node-2 before node-5");

        let kahn_pos_2 = kahn_sorted.iter().position(|x| x == "node-2");
        let kahn_pos_5 = kahn_sorted.iter().position(|x| x == "node-5");
        assert!(kahn_pos_2 < kahn_pos_5, "Kahn: node-2 before node-5");
    }
}

// ============================================================================
// CRITICAL PATH INTEGRATION (3 tests)
// ============================================================================

#[test]
fn given_uniform_weights_when_critical_path_then_longest_chain() {
    // GIVEN: A DAG with uniform weights (all 1 second)
    //        Structure: a -> b -> c (chain of 3)
    //                   a -> d (direct path)
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
        "a".to_string(),
        "d".to_string(),
        DependencyType::BlockingDependency,
    );

    let mut weights = HashMap::new();
    weights.insert("a".to_string(), Duration::from_secs(1));
    weights.insert("b".to_string(), Duration::from_secs(1));
    weights.insert("c".to_string(), Duration::from_secs(1));
    weights.insert("d".to_string(), Duration::from_secs(1));

    // WHEN: Computing critical path
    let result = dag.critical_path(&weights);

    // THEN: Should return the longest chain (a -> b -> c)
    assert!(result.is_ok(), "critical path should succeed");
    if let Ok(path) = result {
        assert!(
            path.contains(&"a".to_string()),
            "critical path should include a"
        );
        assert!(
            path.contains(&"b".to_string()),
            "critical path should include b"
        );
        assert!(
            path.contains(&"c".to_string()),
            "critical path should include c"
        );
        assert_eq!(path.len(), 3, "critical path should be longest chain");
    }
}

#[test]
fn given_varied_weights_when_critical_path_then_heaviest_path() {
    // GIVEN: A DAG with varied weights
    //        a(1) -> b(10) (total 11)
    //        a(1) -> c(5) (total 6)
    let mut dag = WorkflowDAG::new();
    let _ = dag.add_node("a".to_string());
    let _ = dag.add_node("b".to_string());
    let _ = dag.add_node("c".to_string());
    let _ = dag.add_edge(
        "a".to_string(),
        "b".to_string(),
        DependencyType::BlockingDependency,
    );
    let _ = dag.add_edge(
        "a".to_string(),
        "c".to_string(),
        DependencyType::BlockingDependency,
    );

    let mut weights = HashMap::new();
    weights.insert("a".to_string(), Duration::from_secs(1));
    weights.insert("b".to_string(), Duration::from_secs(10));
    weights.insert("c".to_string(), Duration::from_secs(5));

    // WHEN: Computing critical path
    let result = dag.critical_path(&weights);

    // THEN: Should return path through b (heavier weight)
    assert!(result.is_ok(), "critical path should succeed");
    if let Ok(path) = result {
        assert!(path.contains(&"a".to_string()), "path should include a");
        assert!(
            path.contains(&"b".to_string()),
            "path should include b (heavier)"
        );
    }
}

#[test]
fn given_parallel_paths_when_critical_path_then_bottleneck_found() {
    // GIVEN: A complex DAG with parallel paths converging
    //        start -> [fast1, slow] -> end
    let mut dag = WorkflowDAG::new();
    let _ = dag.add_node("start".to_string());
    let _ = dag.add_node("fast1".to_string());
    let _ = dag.add_node("slow".to_string());
    let _ = dag.add_node("end".to_string());
    let _ = dag.add_edge(
        "start".to_string(),
        "fast1".to_string(),
        DependencyType::BlockingDependency,
    );
    let _ = dag.add_edge(
        "start".to_string(),
        "slow".to_string(),
        DependencyType::BlockingDependency,
    );
    let _ = dag.add_edge(
        "fast1".to_string(),
        "end".to_string(),
        DependencyType::BlockingDependency,
    );
    let _ = dag.add_edge(
        "slow".to_string(),
        "end".to_string(),
        DependencyType::BlockingDependency,
    );

    let mut weights = HashMap::new();
    weights.insert("start".to_string(), Duration::from_secs(1));
    weights.insert("fast1".to_string(), Duration::from_secs(2));
    weights.insert("slow".to_string(), Duration::from_secs(20)); // Bottleneck
    weights.insert("end".to_string(), Duration::from_secs(1));

    // WHEN: Computing critical path
    let result = dag.critical_path(&weights);

    // THEN: Should identify the bottleneck path through "slow"
    assert!(result.is_ok(), "critical path should succeed");
    if let Ok(path) = result {
        assert!(
            path.contains(&"slow".to_string()),
            "critical path should go through bottleneck"
        );
        assert!(
            path.contains(&"start".to_string()),
            "path should include start"
        );
        assert!(path.contains(&"end".to_string()), "path should include end");
    }
}

// ============================================================================
// CYCLE DETECTION INTEGRATION (3 tests)
// ============================================================================

#[test]
fn given_acyclic_dag_when_has_cycle_then_false() {
    // GIVEN: An acyclic DAG
    let mut dag = WorkflowDAG::new();
    let _ = dag.add_node("a".to_string());
    let _ = dag.add_node("b".to_string());
    let _ = dag.add_node("c".to_string());
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

    // WHEN: Checking for cycles
    let has_cycle = dag.has_cycle();

    // THEN: Should return false
    assert!(!has_cycle, "acyclic DAG should not have cycles");
}

#[test]
fn given_self_loop_when_has_cycle_then_true() {
    // GIVEN: A DAG with a self-loop: a -> a
    let mut dag = WorkflowDAG::new();
    let _ = dag.add_node("a".to_string());
    // Note: petgraph allows self-loops, but DAG semantics should reject them
    let _ = dag.add_edge(
        "a".to_string(),
        "a".to_string(),
        DependencyType::BlockingDependency,
    );

    // WHEN: Checking for cycles
    let has_cycle = dag.has_cycle();

    // THEN: Should return true (self-loop is a cycle)
    assert!(has_cycle, "self-loop should be detected as a cycle");
}

#[test]
fn given_2_node_cycle_when_find_cycles_then_returns_cycle() {
    // GIVEN: A 2-node cycle: a <-> b
    let mut dag = WorkflowDAG::new();
    let _ = dag.add_node("a".to_string());
    let _ = dag.add_node("b".to_string());
    let _ = dag.add_edge(
        "a".to_string(),
        "b".to_string(),
        DependencyType::BlockingDependency,
    );
    let _ = dag.add_edge(
        "b".to_string(),
        "a".to_string(),
        DependencyType::BlockingDependency,
    );

    // WHEN: Finding all cycles
    let cycles = dag.find_cycles();

    // THEN: Should return the cycle containing a and b
    assert!(!cycles.is_empty(), "should detect at least one cycle");
    let cycle_nodes: HashSet<String> = cycles[0].iter().cloned().collect();
    assert!(cycle_nodes.contains("a"), "cycle should contain a");
    assert!(cycle_nodes.contains("b"), "cycle should contain b");
}

// ============================================================================
// SUBGRAPH EXTRACTION INTEGRATION (3 tests)
// ============================================================================

#[test]
fn given_node_subset_when_subgraph_then_only_internal_edges() {
    // GIVEN: A DAG with external edges
    //        a -> b -> c -> d
    //        We want subgraph of [b, c]
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

    // WHEN: Extracting subgraph of [b, c]
    let result = dag.subgraph(&["b".to_string(), "c".to_string()]);

    // THEN: Should only include edge b -> c
    assert!(result.is_ok(), "subgraph extraction should succeed");
    if let Ok(sub) = result {
        assert_eq!(sub.node_count(), 2, "should have 2 nodes");
        assert_eq!(sub.edge_count(), 1, "should have 1 edge (b->c only)");
        assert!(sub.contains_node(&"b".to_string()), "should contain b");
        assert!(sub.contains_node(&"c".to_string()), "should contain c");
    }
}

#[test]
fn given_node_when_induced_subgraph_then_includes_all_related() {
    // GIVEN: A DAG with connected and disconnected nodes
    //        a -> b -> c
    //        d (disconnected)
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

    // WHEN: Creating induced subgraph around b
    let result = dag.induced_subgraph(&"b".to_string());

    // THEN: Should include a, b, c (all connected) but not d
    assert!(result.is_ok(), "induced subgraph should succeed");
    if let Ok(sub) = result {
        assert_eq!(sub.node_count(), 3, "should have 3 nodes");
        assert!(
            sub.contains_node(&"a".to_string()),
            "should include ancestor a"
        );
        assert!(
            sub.contains_node(&"b".to_string()),
            "should include b itself"
        );
        assert!(
            sub.contains_node(&"c".to_string()),
            "should include descendant c"
        );
        assert!(
            !sub.contains_node(&"d".to_string()),
            "should not include disconnected d"
        );
    }
}

#[test]
fn given_complex_graph_when_induced_subgraph_then_preserves_structure() {
    // GIVEN: A complex DAG with diamond pattern
    //        a -> b,c -> d -> e
    let mut dag = WorkflowDAG::new();
    let _ = dag.add_node("a".to_string());
    let _ = dag.add_node("b".to_string());
    let _ = dag.add_node("c".to_string());
    let _ = dag.add_node("d".to_string());
    let _ = dag.add_node("e".to_string());
    let _ = dag.add_edge(
        "a".to_string(),
        "b".to_string(),
        DependencyType::BlockingDependency,
    );
    let _ = dag.add_edge(
        "a".to_string(),
        "c".to_string(),
        DependencyType::BlockingDependency,
    );
    let _ = dag.add_edge(
        "b".to_string(),
        "d".to_string(),
        DependencyType::BlockingDependency,
    );
    let _ = dag.add_edge(
        "c".to_string(),
        "d".to_string(),
        DependencyType::BlockingDependency,
    );
    let _ = dag.add_edge(
        "d".to_string(),
        "e".to_string(),
        DependencyType::BlockingDependency,
    );

    // WHEN: Creating induced subgraph around d
    let result = dag.induced_subgraph(&"d".to_string());

    // THEN: Should include entire chain (a,b,c,d,e) with preserved structure
    assert!(result.is_ok(), "induced subgraph should succeed");
    if let Ok(sub) = result {
        assert_eq!(sub.node_count(), 5, "should include all connected nodes");
        // Verify diamond structure is preserved
        let deps_d = sub.get_dependencies(&"d".to_string());
        if let Ok(deps) = deps_d {
            assert_eq!(deps.len(), 2, "d should have 2 dependencies (b and c)");
        }
    }
}
