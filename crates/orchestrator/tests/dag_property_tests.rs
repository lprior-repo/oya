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
use proptest::collection::vec;
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
    (2usize..20).prop_flat_map(|n| {
        let possible_edges: Vec<(usize, usize)> = (0..n)
            .flat_map(|from| (from + 1..n).map(move |to| (from, to)))
            .collect();

        let node_ids: Vec<String> = (0..n).map(|i| format!("node-{}", i)).collect();

        vec(
            proptest::option::of(proptest::sample::select(possible_edges.clone())),
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
    fn prop_toposort_single_node(bead_id in bead_id_strategy()) {
        let mut dag = WorkflowDAG::new();
        dag.add_node(bead_id.clone())
            .map_err(|e| proptest::test_runner::TestCaseError::fail(format!("Add node failed: {:?}", e)))?;

        let sorted = dag
            .topological_sort()
            .map_err(|e| proptest::test_runner::TestCaseError::fail(format!("Toposort failed: {:?}", e)))?;

        prop_assert_eq!(sorted.len(), 1, "Single node DAG should produce single element toposort");
        prop_assert_eq!(&sorted[0], &bead_id, "Single node should be preserved");
    }

    #[test]
    fn prop_toposort_linear_chain(chain_size in 2usize..20) {
        let mut dag = WorkflowDAG::new();

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

fn build_linear_dag_for_ready(size: usize) -> Result<WorkflowDAG, String> {
    let mut dag = WorkflowDAG::new();
    for i in 0..size {
        dag.add_node(format!("bead-{}", i))
            .map_err(|e| e.to_string())?;
    }
    for i in 0..size.saturating_sub(1) {
        dag.add_edge(
            format!("bead-{}", i),
            format!("bead-{}", i + 1),
            DependencyType::BlockingDependency,
        )
        .map_err(|e| e.to_string())?;
    }
    Ok(dag)
}

fn build_diamond_dag_for_ready() -> Result<WorkflowDAG, String> {
    let mut dag = WorkflowDAG::new();
    dag.add_node("bead-a".to_string())
        .map_err(|e| e.to_string())?;
    dag.add_node("bead-b".to_string())
        .map_err(|e| e.to_string())?;
    dag.add_node("bead-c".to_string())
        .map_err(|e| e.to_string())?;
    dag.add_node("bead-d".to_string())
        .map_err(|e| e.to_string())?;

    dag.add_edge(
        "bead-a".to_string(),
        "bead-b".to_string(),
        DependencyType::BlockingDependency,
    )
    .map_err(|e| e.to_string())?;
    dag.add_edge(
        "bead-a".to_string(),
        "bead-c".to_string(),
        DependencyType::BlockingDependency,
    )
    .map_err(|e| e.to_string())?;
    dag.add_edge(
        "bead-b".to_string(),
        "bead-d".to_string(),
        DependencyType::BlockingDependency,
    )
    .map_err(|e| e.to_string())?;
    dag.add_edge(
        "bead-c".to_string(),
        "bead-d".to_string(),
        DependencyType::BlockingDependency,
    )
    .map_err(|e| e.to_string())?;

    Ok(dag)
}

proptest! {
    /// Property: Ready beads have no incomplete blocking dependencies (src-yaw2)
    ///
    /// ∀ bead in DAG: is_ready(bead, completed) -> all blocking deps ∈ completed
    #[test]
    fn prop_ready_bead_has_no_incomplete_blocking_deps_linear(
        dag_size in 1usize..10,
        completed_count in 0usize..10,
    ) {
        let dag = build_linear_dag_for_ready(dag_size).map_err(|e| TestCaseError::fail(e))?;

        let completed: HashSet<BeadId> = (0..completed_count.min(dag_size))
            .map(|i| format!("bead-{}", i))
            .collect();

        let ready_beads = dag.get_ready_beads(&completed);

        for bead_id in &ready_beads {
            let is_ready = dag.is_ready(bead_id, &completed)
                .map_err(|e| TestCaseError::fail(e.to_string()))?;

            prop_assert!(is_ready, "get_ready_beads returned {} but is_ready returned false", bead_id);

            let deps = dag.get_dependencies(bead_id)
                .map_err(|e| TestCaseError::fail(e.to_string()))?;

            for dep_id in &deps {
                prop_assert!(
                    completed.contains(dep_id),
                    "Ready bead '{}' has incomplete blocking dependency '{}'",
                    bead_id,
                    dep_id
                );
            }
        }
    }

    /// Property: get_blocked_nodes and get_ready_beads are disjoint (src-yaw2)
    ///
    /// ∀ completed set: blocked ∩ ready = ∅
    #[test]
    fn prop_blocked_and_ready_are_disjoint_diamond(
        completed_beads in vec(0usize..4, 0..4),
    ) {
        let dag = build_diamond_dag_for_ready().map_err(|e| TestCaseError::fail(e))?;

        let bead_names = ["bead-a", "bead-b", "bead-c", "bead-d"];
        let completed: HashSet<BeadId> = completed_beads
            .iter()
            .filter_map(|&i| bead_names.get(i).map(|s| s.to_string()))
            .collect();

        let ready = dag.get_ready_beads(&completed);
        let blocked = dag.get_blocked_nodes(&completed);

        for bead_id in &ready {
            prop_assert!(
                !blocked.contains(bead_id),
                "Bead '{}' is both ready and blocked",
                bead_id
            );
        }
    }

    /// Property: is_ready returns true only for beads in get_ready_beads (src-yaw2)
    ///
    /// Consistency between get_ready_beads and is_ready
    #[test]
    fn prop_is_ready_consistent_with_get_ready_beads(
        completed_beads in vec(0usize..4, 0..4),
    ) {
        let dag = build_diamond_dag_for_ready().map_err(|e| TestCaseError::fail(e))?;

        let bead_names = ["bead-a", "bead-b", "bead-c", "bead-d"];
        let completed: HashSet<BeadId> = completed_beads
            .iter()
            .filter_map(|&i| bead_names.get(i).map(|s| s.to_string()))
            .collect();

        let ready = dag.get_ready_beads(&completed);

        for bead_name in &bead_names {
            let bead_id = bead_name.to_string();
            let is_ready = dag.is_ready(&bead_id, &completed)
                .map_err(|e| TestCaseError::fail(e.to_string()))?;

            let in_ready_list = ready.contains(&bead_id);

            if !completed.contains(&bead_id) {
                prop_assert_eq!(
                    is_ready, in_ready_list,
                    "is_ready({}) = {} but get_ready_beads contains? {}",
                    bead_id, is_ready, in_ready_list
                );
            }
        }
    }

    /// Property: Completing a bead never makes another bead blocked (src-yaw2)
    ///
    /// Adding to completed set can only make beads ready, not blocked
    #[test]
    fn prop_completing_bead_never_causes_blocking(
        initial_completed in vec(0usize..4, 0..3),
        new_completed in 0usize..4,
    ) {
        let dag = build_diamond_dag_for_ready().map_err(|e| TestCaseError::fail(e))?;

        let bead_names = ["bead-a", "bead-b", "bead-c", "bead-d"];

        let mut completed_before: HashSet<BeadId> = initial_completed
            .iter()
            .filter_map(|&i| bead_names.get(i).map(|s| s.to_string()))
            .collect();

        let ready_before = dag.get_ready_beads(&completed_before);

        if let Some(&new_bead) = bead_names.get(new_completed) {
            completed_before.insert(new_bead.to_string());
        }

        let ready_after = dag.get_ready_beads(&completed_before);

        for bead_id in &ready_before {
            if !completed_before.contains(bead_id) {
                prop_assert!(
                    ready_after.contains(bead_id) || completed_before.contains(bead_id),
                    "Bead '{}' was ready before but not ready after completing another bead",
                    bead_id
                );
            }
        }
    }

    /// Property: A ready bead has all its blocking dependencies complete (src-yaw2)
    ///
    /// ∀ completed: ∀ bead ∈ get_ready_beads(completed): all blocking deps of bead ∈ completed
    #[test]
    fn prop_ready_implies_all_blocking_deps_complete(
        completed_beads in vec(0usize..4, 0..4),
    ) {
        let dag = build_diamond_dag_for_ready().map_err(|e| TestCaseError::fail(e))?;

        let bead_names = ["bead-a", "bead-b", "bead-c", "bead-d"];
        let completed: HashSet<BeadId> = completed_beads
            .iter()
            .filter_map(|&i| bead_names.get(i).map(|s| s.to_string()))
            .collect();

        let ready_beads = dag.get_ready_beads(&completed);

        for bead_id in &ready_beads {
            let deps = dag.get_dependencies(bead_id)
                .map_err(|e| TestCaseError::fail(e.to_string()))?;

            for dep_id in &deps {
                prop_assert!(
                    completed.contains(dep_id),
                    "Ready bead '{}' has incomplete blocking dependency '{}'",
                    bead_id,
                    dep_id
                );
            }
        }
    }
}

#[test]
fn prop_single_bead_no_deps_is_ready() -> Result<(), Box<dyn std::error::Error>> {
    let mut dag = WorkflowDAG::new();
    dag.add_node("bead-1".to_string())?;

    let completed = HashSet::new();
    let ready = dag.get_ready_beads(&completed);

    assert_eq!(ready, vec!["bead-1".to_string()]);
    assert!(dag.is_ready(&"bead-1".to_string(), &completed)?);

    Ok(())
}

#[test]
fn prop_bead_with_complete_dep_is_ready() -> Result<(), Box<dyn std::error::Error>> {
    let mut dag = WorkflowDAG::new();
    dag.add_node("bead-a".to_string())?;
    dag.add_node("bead-b".to_string())?;
    dag.add_edge(
        "bead-a".to_string(),
        "bead-b".to_string(),
        DependencyType::BlockingDependency,
    )?;

    let mut completed = HashSet::new();
    completed.insert("bead-a".to_string());

    let ready = dag.get_ready_beads(&completed);

    assert!(ready.contains(&"bead-b".to_string()));
    assert!(dag.is_ready(&"bead-b".to_string(), &completed)?);

    let deps = dag.get_dependencies(&"bead-b".to_string())?;
    for dep_id in deps {
        assert!(
            completed.contains(&dep_id),
            "Ready bead has incomplete blocking dep: {}",
            dep_id
        );
    }

    Ok(())
}

#[test]
fn prop_bead_with_incomplete_dep_is_not_ready() -> Result<(), Box<dyn std::error::Error>> {
    let mut dag = WorkflowDAG::new();
    dag.add_node("bead-a".to_string())?;
    dag.add_node("bead-b".to_string())?;
    dag.add_edge(
        "bead-a".to_string(),
        "bead-b".to_string(),
        DependencyType::BlockingDependency,
    )?;

    let completed = HashSet::new();
    let ready = dag.get_ready_beads(&completed);

    assert!(!ready.contains(&"bead-b".to_string()));
    assert!(!dag.is_ready(&"bead-b".to_string(), &completed)?);

    Ok(())
}
