#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
//! Property-based tests for DAG invariants.
//!
//! ## Phase 5 - Property Tests
//!
//! ∀ dag: toposort(dag) -> all edges respect order
//! ∀ bead: ready state -> no incomplete blocking deps

use im::HashSet;
use proptest::prelude::*;
use proptest::string::string_regex;

use orchestrator::dag::{DependencyType, WorkflowDAG};

fn bead_id_strategy() -> impl Strategy<Value = String> {
    "bead-[a-z0-9]{3,8}"
}

fn build_linear_dag(size: usize) -> Result<WorkflowDAG, String> {
    let mut dag = WorkflowDAG::new();
    for i in 0..size {
        dag.add_node(format!("node-{i}"))
            .map_err(|e| format!("{e:?}"))?;
    }
    for i in 0..size.saturating_sub(1) {
        dag.add_edge(
            &format!("node-{i}"),
            &format!("node-{}", i + 1),
            DependencyType::BlockingDependency,
        )
        .map_err(|e| format!("{e:?}"))?;
    }
    Ok(dag)
}

proptest! {
    #[test]
    fn prop_toposort_respects_edge_order(size in 2usize..20) {
        let dag = build_linear_dag(size).map_err(|e| TestCaseError::fail(e))?;
        let sorted = dag.topological_sort()
            .map_err(|e| TestCaseError::fail(format!("{e:?}")))?;

        prop_assert_eq!(sorted.len(), size);

        for i in 0..size.saturating_sub(1) {
            let pos_curr = sorted.iter().position(|id| id == &format!("node-{i}")).ok_or_else(|| TestCaseError::fail("node missing"))?;
            let pos_next = sorted.iter().position(|id| id == &format!("node-{}", i+1)).ok_or_else(|| TestCaseError::fail("node missing"))?;
            prop_assert!(pos_curr < pos_next);
        }
    }

    #[test]
    fn prop_ready_beads_have_no_blocking_deps(size in 1usize..10, completed_count in 0usize..10) {
        let dag = build_linear_dag(size).map_err(|e| TestCaseError::fail(e))?;
        let completed: HashSet<String> = (0..completed_count.min(size))
            .map(|i| format!("node-{i}"))
            .collect();

        let ready = dag.get_ready_beads(&completed);

        for bead_id in ready {
            let deps = dag.get_dependencies(&bead_id)
                .map_err(|e| TestCaseError::fail(format!("{e:?}")))?;

            for dep in deps {
                prop_assert!(completed.contains(&dep), "Ready bead {bead_id} has incomplete dep {dep}");
            }
        }
    }
}
