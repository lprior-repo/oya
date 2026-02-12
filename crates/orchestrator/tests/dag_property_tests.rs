//! Property-based tests for DAG invariants.
//!
//! This module tests the property described in bead src-as6n:
//!
//! ## Phase 5 - Property Tests
//!
//! ∀ dag: toposort(dag) -> all edges respect order

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

type DagParams = (usize, usize);

fn dag_params_strategy() -> impl Strategy<Value = DagParams> {
    (2usize..15, 0usize..30)
}

fn build_dag(node_count: usize, edge_count: usize) -> Result<WorkflowDAG, String> {
    let mut dag = WorkflowDAG::new();
    
    for i in 0..node_count {
        dag.add_node(format!("node-{}", i))
            .map_err(|e| format!("Failed to add node: {:?}", e))?;
    }
    
    let all_edges: Vec<(usize, usize)> = (0..node_count)
        .flat_map(|from| (from + 1..node_count).map(move |to| (from, to)))
        .collect();
    
    for i in 0..edge_count {
        if let Some(&(from, to)) = all_edges.get(i % all_edges.len().max(1)) {
            dag.add_edge(
                format!("node-{}", from),
                format!("node-{}", to),
                DependencyType::BlockingDependency,
            )
            .map_err(|_| format!("Failed to add edge {} -> {}", from, to))?;
        }
    }
    
    Ok(dag)
}

fn get_edges(node_count: usize, edge_count: usize) -> Vec<(usize, usize)> {
    let all_edges: Vec<(usize, usize)> = (0..node_count)
        .flat_map(|from| (from + 1..node_count).map(move |to| (from, to)))
        .collect();
    
    (0..edge_count)
        .map(|i| all_edges[i % all_edges.len().max(1)])
        .collect()
}

fn verify_edge_order(sorted: &[String], edges: &[(usize, usize)]) -> Result<(), String> {
    let positions: StdHashMap<String, usize> = sorted
        .iter()
        .enumerate()
        .map(|(i, id)| (id.clone(), i))
        .collect();
    
    for &(from, to) in edges {
        let from_node = format!("node-{}", from);
        let to_node = format!("node-{}", to);
        
        let from_pos = positions
            .get(&from_node)
            .ok_or_else(|| format!("Node {} missing from sorted result", from_node))?;
        let to_pos = positions
            .get(&to_node)
            .ok_or_else(|| format!("Node {} missing from sorted result", to_node))?;
        
        if from_pos >= to_pos {
            return Err(format!(
                "Edge order violated: {} (pos {}) should come before {} (pos {})",
                from_node, from_pos, to_node, to_pos
            ));
        }
    }
    
    Ok(())
}

proptest! {
    #[test]
    fn prop_toposort_respects_all_edge_orders(params in dag_params_strategy()) {
        let (node_count, edge_count) = params;
        let dag = match build_dag(node_count, edge_count) {
            Ok(d) => d,
            Err(_) => return Err(TestCaseError::reject("Invalid DAG")),
        };
        
        let edges = get_edges(node_count, edge_count);
        
        let sorted = dag
            .topological_sort()
            .map_err(|e| TestCaseError::fail(format!("Toposort failed: {:?}", e)))?;
        
        if let Err(e) = verify_edge_order(&sorted, &edges) {
            return Err(TestCaseError::fail(e));
        }
    }
    
    #[test]
    fn prop_toposort_kahn_respects_all_edge_orders(params in dag_params_strategy()) {
        let (node_count, edge_count) = params;
        let dag = match build_dag(node_count, edge_count) {
            Ok(d) => d,
            Err(_) => return Err(TestCaseError::reject("Invalid DAG")),
        };
        
        let edges = get_edges(node_count, edge_count);
        
        let sorted = dag
            .topological_sort_kahn()
            .map_err(|e| TestCaseError::fail(format!("Toposort Kahn failed: {:?}", e)))?;
        
        if let Err(e) = verify_edge_order(&sorted, &edges) {
            return Err(TestCaseError::fail(e));
        }
    }
    
    #[test]
    fn prop_toposort_contains_all_nodes(params in dag_params_strategy()) {
        let (node_count, _) = params;
        let dag = match build_dag(node_count, 0) {
            Ok(d) => d,
            Err(_) => return Err(TestCaseError::reject("Invalid DAG")),
        };
        
        let sorted = dag
            .topological_sort()
            .map_err(|e| TestCaseError::fail(format!("Toposort failed: {:?}", e)))?;
        
        prop_assert_eq!(sorted.len(), node_count);
        
        let sorted_set: std::collections::HashSet<&String> = sorted.iter().collect();
        for i in 0..node_count {
            let node = format!("node-{}", i);
            prop_assert!(sorted_set.contains(&node), "Missing node {}", node);
        }
    }
    
    #[test]
    fn prop_toposort_no_duplicates(params in dag_params_strategy()) {
        let (node_count, edge_count) = params;
        let dag = match build_dag(node_count, edge_count) {
            Ok(d) => d,
            Err(_) => return Err(TestCaseError::reject("Invalid DAG")),
        };
        
        let sorted = dag
            .topological_sort()
            .map_err(|e| TestCaseError::fail(format!("Toposort failed: {:?}", e)))?;
        
        let mut seen = std::collections::HashSet::new();
        for node in &sorted {
            prop_assert!(seen.insert(node), "Duplicate node {}", node);
        }
    }
    
    #[test]
    fn prop_toposort_empty_dag() {
        let dag = WorkflowDAG::new();
        let sorted = dag
            .topological_sort()
            .map_err(|e| TestCaseError::fail(format!("Toposort failed: {:?}", e)))?;
        
        prop_assert!(sorted.is_empty());
    }
    
    #[test]
    fn prop_toposort_single_node(bead_id in bead_id_strategy()) {
        let mut dag = WorkflowDAG::new();
        dag.add_node(bead_id.clone())
            .map_err(|e| TestCaseError::fail(format!("Add node failed: {:?}", e)))?;
        
        let sorted = dag
            .topological_sort()
            .map_err(|e| TestCaseError::fail(format!("Toposort failed: {:?}", e)))?;
        
        prop_assert_eq!(sorted.len(), 1);
        prop_assert_eq!(&sorted[0], &bead_id);
    }
    
    #[test]
    fn prop_toposort_linear_chain(chain_size in 2usize..15) {
        let mut dag = WorkflowDAG::new();
        
        for i in 0..chain_size {
            dag.add_node(format!("node-{}", i))
                .map_err(|e| TestCaseError::fail(format!("Add node failed: {:?}", e)))?;
        }
        
        for i in 0..(chain_size - 1) {
            dag.add_edge(
                format!("node-{}", i),
                format!("node-{}", i + 1),
                DependencyType::BlockingDependency,
            )
            .map_err(|e| TestCaseError::fail(format!("Add edge failed: {:?}", e)))?;
        }
        
        let sorted = dag
            .topological_sort()
            .map_err(|e| TestCaseError::fail(format!("Toposort failed: {:?}", e)))?;
        
        prop_assert_eq!(sorted.len(), chain_size);
        
        for (expected_pos, node) in sorted.iter().enumerate() {
            prop_assert_eq!(node, &format!("node-{}", expected_pos));
        }
    }
    
    #[test]
    fn prop_toposort_diamond_dag() {
        let mut dag = WorkflowDAG::new();
        
        dag.add_node("a".to_string())
            .map_err(|e| TestCaseError::fail(format!("Add node failed: {:?}", e)))?;
        dag.add_node("b".to_string())
            .map_err(|e| TestCaseError::fail(format!("Add node failed: {:?}", e)))?;
        dag.add_node("c".to_string())
            .map_err(|e| TestCaseError::fail(format!("Add node failed: {:?}", e)))?;
        dag.add_node("d".to_string())
            .map_err(|e| TestCaseError::fail(format!("Add node failed: {:?}", e)))?;
        
        dag.add_edge("a".to_string(), "b".to_string(), DependencyType::BlockingDependency)
            .map_err(|e| TestCaseError::fail(format!("Add edge failed: {:?}", e)))?;
        dag.add_edge("a".to_string(), "c".to_string(), DependencyType::BlockingDependency)
            .map_err(|e| TestCaseError::fail(format!("Add edge failed: {:?}", e)))?;
        dag.add_edge("b".to_string(), "d".to_string(), DependencyType::BlockingDependency)
            .map_err(|e| TestCaseError::fail(format!("Add edge failed: {:?}", e)))?;
        dag.add_edge("c".to_string(), "d".to_string(), DependencyType::BlockingDependency)
            .map_err(|e| TestCaseError::fail(format!("Add edge failed: {:?}", e)))?;
        
        let sorted = dag
            .topological_sort()
            .map_err(|e| TestCaseError::fail(format!("Toposort failed: {:?}", e)))?;
        
        prop_assert_eq!(sorted.len(), 4);
        
        let pos_a = sorted.iter().position(|x| x == "a").ok_or_else(|| TestCaseError::fail("Missing a"))?;
        let pos_b = sorted.iter().position(|x| x == "b").ok_or_else(|| TestCaseError::fail("Missing b"))?;
        let pos_c = sorted.iter().position(|x| x == "c").ok_or_else(|| TestCaseError::fail("Missing c"))?;
        let pos_d = sorted.iter().position(|x| x == "d").ok_or_else(|| TestCaseError::fail("Missing d"))?;
        
        prop_assert!(pos_a < pos_b);
        prop_assert!(pos_a < pos_c);
        prop_assert!(pos_b < pos_d);
        prop_assert!(pos_c < pos_d);
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

proptest! {
    #[test]
    fn prop_ready_bead_has_no_incomplete_blocking_deps(
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

            prop_assert!(is_ready);

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
