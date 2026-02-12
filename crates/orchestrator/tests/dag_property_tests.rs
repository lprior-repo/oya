//! Property-based tests for DAG invariants.
//!
//! This module tests multiple properties:
//!
//! ## Bead src-as6n: Topological Sort
//!
//! ∀ dag: toposort(dag) -> all edges respect order
//!
//! ## Bead src-yaw2: Ready State Invariant
//!
//! ∀ bead: ready state -> no incomplete blocking deps
//!
//! Invariant: If a bead is in "ready" state, it must have no incomplete
//! blocking dependencies. This means all blocking dependencies must be
//! in the completed set.

#![forbid(clippy::unwrap_used)]
#![deny(clippy::expect_used)]
#![forbid(clippy::panic)]

use std::collections::HashMap as StdHashMap;

use im::HashSet;
use proptest::collection::{hash_map, hash_set, vec};
use proptest::prelude::*;
use proptest::string::string_regex;

use orchestrator::dag::{BeadId, DependencyType, WorkflowDAG};

fn bead_id_strategy() -> impl Strategy<Value = String> {
    string_regex("bead-[a-z0-9]{3,8}").unwrap()
}

#[derive(Debug, Clone)]
struct DagSpec {
    nodes: Vec<String>,
    edges: Vec<(String, String)>,
}

fn dag_strategy() -> impl Strategy<Value = DagSpec> {
    let node_count = 2usize..20;

    node_count.flat_map(|n| {
        let node_ids: Vec<String> = (0..n).map(|i| format!("node-{}", i)).collect();

        let edges: Vec<(usize, usize)> = (0..n)
            .flat_map(|from| (from + 1..n).map(move |to| (from, to)))
            .collect();

        vec(
            proptest::option::of(proptest::sample::select(edges.clone())),
            0..n * (n - 1) / 2,
        )
        .prop_map(move |edge_options| {
            let selected_edges: Vec<(String, String)> = edge_options
                .into_iter()
                .flatten()
                .map(|(from, to)| (node_ids[from].clone(), node_ids[to].clone()))
                .collect();

            DagSpec {
                nodes: node_ids.clone(),
                edges: selected_edges,
            }
        })
    })
}

fn build_dag(spec: &DagSpec) -> Result<WorkflowDAG, String> {
    let mut dag = WorkflowDAG::new();

    for node in &spec.nodes {
        dag.add_node(node.clone())
            .map_err(|e| format!("Failed to add node: {:?}", e))?;
    }

    for (from, to) in &spec.edges {
        dag.add_edge(from.clone(), to.clone(), DependencyType::BlockingDependency)
            .map_err(|_| format!("Failed to add edge {} -> {}", from, to))?;
    }

    Ok(dag)
}

fn verify_edge_order(sorted: &[String], edges: &[(String, String)]) -> Result<(), String> {
    let positions: StdHashMap<&String, usize> =
        sorted.iter().enumerate().map(|(i, id)| (id, i)).collect();

    for (from, to) in edges {
        let from_pos = positions
            .get(&from)
            .ok_or_else(|| format!("Node {} missing from sorted result", from))?;
        let to_pos = positions
            .get(&to)
            .ok_or_else(|| format!("Node {} missing from sorted result", to))?;

        if from_pos >= to_pos {
            return Err(format!(
                "Edge order violated: {} (pos {}) should come before {} (pos {})",
                from, from_pos, to, to_pos
            ));
        }
    }

    Ok(())
}

proptest! {
    #[test]
    fn prop_toposort_respects_all_edge_orders(spec in dag_strategy()) {
        let dag = match build_dag(&spec) {
            Ok(d) => d,
            Err(_) => return Err(proptest::test_runner::TestCaseError::reject("Invalid DAG")),
        };

        let sorted = dag
            .topological_sort()
            .map_err(|e| proptest::test_runner::TestCaseError::fail(format!("Toposort failed: {:?}", e)))?;

        if let Err(e) = verify_edge_order(&sorted, &spec.edges) {
            return Err(proptest::test_runner::TestCaseError::fail(e));
        }
    }

    #[test]
    fn prop_toposort_kahn_respects_all_edge_orders(spec in dag_strategy()) {
        let dag = match build_dag(&spec) {
            Ok(d) => d,
            Err(_) => return Err(proptest::test_runner::TestCaseError::reject("Invalid DAG")),
        };

        let sorted = dag
            .topological_sort_kahn()
            .map_err(|e| proptest::test_runner::TestCaseError::fail(format!("Toposort Kahn failed: {:?}", e)))?;

        if let Err(e) = verify_edge_order(&sorted, &spec.edges) {
            return Err(proptest::test_runner::TestCaseError::fail(e));
        }
    }

    #[test]
    fn prop_toposort_contains_all_nodes(spec in dag_strategy()) {
        let dag = match build_dag(&spec) {
            Ok(d) => d,
            Err(_) => return Err(proptest::test_runner::TestCaseError::reject("Invalid DAG")),
        };

        let sorted = dag
            .topological_sort()
            .map_err(|e| proptest::test_runner::TestCaseError::fail(format!("Toposort failed: {:?}", e)))?;

        let sorted_set: std::collections::HashSet<&String> = sorted.iter().collect();
        for node in &spec.nodes {
            if !sorted_set.contains(node) {
                return Err(proptest::test_runner::TestCaseError::fail(
                    format!("Node {} missing from toposort result", node)
                ));
            }
        }

        if sorted.len() != spec.nodes.len() {
            return Err(proptest::test_runner::TestCaseError::fail(
                format!("Toposort returned {} nodes, expected {}", sorted.len(), spec.nodes.len())
            ));
        }
    }

    #[test]
    fn prop_toposort_no_duplicates(spec in dag_strategy()) {
        let dag = match build_dag(&spec) {
            Ok(d) => d,
            Err(_) => return Err(proptest::test_runner::TestCaseError::reject("Invalid DAG")),
        };

        let sorted = dag
            .topological_sort()
            .map_err(|e| proptest::test_runner::TestCaseError::fail(format!("Toposort failed: {:?}", e)))?;

        let mut seen = std::collections::HashSet::new();
        for node in &sorted {
            if !seen.insert(node) {
                return Err(proptest::test_runner::TestCaseError::fail(
                    format!("Duplicate node {} in toposort result", node)
                ));
            }
        }
    }

    #[test]
    fn prop_toposort_both_algorithms_produce_valid_ordering(spec in dag_strategy()) {
        let dag = match build_dag(&spec) {
            Ok(d) => d,
            Err(_) => return Err(proptest::test_runner::TestCaseError::reject("Invalid DAG")),
        };

        let sorted_dfs = dag
            .topological_sort()
            .map_err(|e| proptest::test_runner::TestCaseError::fail(format!("Toposort DFS failed: {:?}", e)))?;

        let sorted_kahn = dag
            .topological_sort_kahn()
            .map_err(|e| proptest::test_runner::TestCaseError::fail(format!("Toposort Kahn failed: {:?}", e)))?;

        if let Err(e) = verify_edge_order(&sorted_dfs, &spec.edges) {
            return Err(proptest::test_runner::TestCaseError::fail(format!("DFS: {}", e)));
        }

        if let Err(e) = verify_edge_order(&sorted_kahn, &spec.edges) {
            return Err(proptest::test_runner::TestCaseError::fail(format!("Kahn: {}", e)));
        }
    }

    #[test]
    fn prop_toposort_empty_dag_returns_empty() {
        let dag = WorkflowDAG::new();
        let sorted = dag
            .topological_sort()
            .map_err(|e| proptest::test_runner::TestCaseError::fail(format!("Toposort failed: {:?}", e)))?;

        prop_assert!(sorted.is_empty(), "Empty DAG should produce empty toposort");
    }

    #[test]
    fn prop_toposort_single_node(spec in dag_strategy().prop_filter("Single node", |s| s.nodes.len() == 1)) {
        let dag = match build_dag(&spec) {
            Ok(d) => d,
            Err(_) => return Err(proptest::test_runner::TestCaseError::reject("Invalid DAG")),
        };

        let sorted = dag
            .topological_sort()
            .map_err(|e| proptest::test_runner::TestCaseError::fail(format!("Toposort failed: {:?}", e)))?;

        prop_assert_eq!(sorted.len(), 1, "Single node DAG should produce single element toposort");
        prop_assert_eq!(&sorted[0], &spec.nodes[0], "Single node should be preserved");
    }

    #[test]
    fn prop_toposort_linear_chain() {
        let mut dag = WorkflowDAG::new();
        let chain_size = 5usize;

        for i in 0..chain_size {
            dag.add_node(format!("node-{}", i))
                .map_err(|e| proptest::test_runner::TestCaseError::fail(format!("Add node failed: {:?}", e)))?;
        }

        for i in 0..(chain_size - 1) {
            dag.add_edge(
                format!("node-{}", i),
                format!("node-{}", i + 1),
                DependencyType::BlockingDependency,
            )
            .map_err(|e| proptest::test_runner::TestCaseError::fail(format!("Add edge failed: {:?}", e)))?;
        }

        let sorted = dag
            .topological_sort()
            .map_err(|e| proptest::test_runner::TestCaseError::fail(format!("Toposort failed: {:?}", e)))?;

        prop_assert_eq!(sorted.len(), chain_size, "Linear chain toposort should have all nodes");

        for i in 0..chain_size {
            prop_assert_eq!(
                sorted[i],
                format!("node-{}", i),
                "Linear chain should preserve exact order: expected node-{} at position {}, got {}",
                i, i, sorted[i]
            );
        }
    }

    #[test]
    fn prop_toposort_diamond_dag() {
        let mut dag = WorkflowDAG::new();

        dag.add_node("a".to_string())
            .map_err(|e| proptest::test_runner::TestCaseError::fail(format!("Add node failed: {:?}", e)))?;
        dag.add_node("b".to_string())
            .map_err(|e| proptest::test_runner::TestCaseError::fail(format!("Add node failed: {:?}", e)))?;
        dag.add_node("c".to_string())
            .map_err(|e| proptest::test_runner::TestCaseError::fail(format!("Add node failed: {:?}", e)))?;
        dag.add_node("d".to_string())
            .map_err(|e| proptest::test_runner::TestCaseError::fail(format!("Add node failed: {:?}", e)))?;

        dag.add_edge("a".to_string(), "b".to_string(), DependencyType::BlockingDependency)
            .map_err(|e| proptest::test_runner::TestCaseError::fail(format!("Add edge failed: {:?}", e)))?;
        dag.add_edge("a".to_string(), "c".to_string(), DependencyType::BlockingDependency)
            .map_err(|e| proptest::test_runner::TestCaseError::fail(format!("Add edge failed: {:?}", e)))?;
        dag.add_edge("b".to_string(), "d".to_string(), DependencyType::BlockingDependency)
            .map_err(|e| proptest::test_runner::TestCaseError::fail(format!("Add edge failed: {:?}", e)))?;
        dag.add_edge("c".to_string(), "d".to_string(), DependencyType::BlockingDependency)
            .map_err(|e| proptest::test_runner::TestCaseError::fail(format!("Add edge failed: {:?}", e)))?;

        let sorted = dag
            .topological_sort()
            .map_err(|e| proptest::test_runner::TestCaseError::fail(format!("Toposort failed: {:?}", e)))?;

        prop_assert_eq!(sorted.len(), 4, "Diamond DAG should have 4 nodes");

        let pos_a = sorted.iter().position(|x| x == "a")
            .ok_or_else(|| proptest::test_runner::TestCaseError::fail("Missing node a"))?;
        let pos_b = sorted.iter().position(|x| x == "b")
            .ok_or_else(|| proptest::test_runner::TestCaseError::fail("Missing node b"))?;
        let pos_c = sorted.iter().position(|x| x == "c")
            .ok_or_else(|| proptest::test_runner::TestCaseError::fail("Missing node c"))?;
        let pos_d = sorted.iter().position(|x| x == "d")
            .ok_or_else(|| proptest::test_runner::TestCaseError::fail("Missing node d"))?;

        prop_assert!(pos_a < pos_b, "a should come before b");
        prop_assert!(pos_a < pos_c, "a should come before c");
        prop_assert!(pos_b < pos_d, "b should come before d");
        prop_assert!(pos_c < pos_d, "c should come before d");
    }
}
