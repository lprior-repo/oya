#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
//! Property-based tests for DAG ready state invariant (src-yaw2).
//!
//! ## Phase 5 - Property Tests
//!
//! ∀ bead: ready state -> no incomplete blocking deps
//!
//! Invariant: If a bead is in "ready" state, it must have no incomplete
//! blocking dependencies. This means all blocking dependencies must be
//! in the completed set.

use im::HashSet;
use orchestrator::dag::{BeadId, DependencyType, WorkflowDAG};

fn build_linear_dag(size: usize) -> Result<WorkflowDAG, String> {
    let mut dag = WorkflowDAG::new();
    for i in 0..size {
        dag.add_node(format!("bead-{}", i))
            .map_err(|e| e.to_string())?;
    }
    for i in 0..size.saturating_sub(1) {
        dag.add_edge(
            &format!("bead-{}", i),
            &format!("bead-{}", i + 1),
            DependencyType::BlockingDependency,
        )
        .map_err(|e| e.to_string())?;
    }
    Ok(dag)
}

fn build_diamond_dag() -> Result<WorkflowDAG, String> {
    let mut dag = WorkflowDAG::new();
    dag.add_node("bead-a").map_err(|e| e.to_string())?;
    dag.add_node("bead-b").map_err(|e| e.to_string())?;
    dag.add_node("bead-c").map_err(|e| e.to_string())?;
    dag.add_node("bead-d").map_err(|e| e.to_string())?;

    dag.add_edge("bead-a", "bead-b", DependencyType::BlockingDependency)
        .map_err(|e| e.to_string())?;
    dag.add_edge("bead-a", "bead-c", DependencyType::BlockingDependency)
        .map_err(|e| e.to_string())?;
    dag.add_edge("bead-b", "bead-d", DependencyType::BlockingDependency)
        .map_err(|e| e.to_string())?;
    dag.add_edge("bead-c", "bead-d", DependencyType::BlockingDependency)
        .map_err(|e| e.to_string())?;

    Ok(dag)
}

#[test]
fn prop_single_bead_no_deps_is_ready() -> Result<(), Box<dyn std::error::Error>> {
    let mut dag = WorkflowDAG::new();
    dag.add_node("bead-1")?;

    let completed = HashSet::new();
    let ready = dag.get_ready_beads(&completed);

    assert_eq!(ready, vec!["bead-1".to_string()]);
    assert!(dag.is_ready("bead-1", &completed)?);

    Ok(())
}

#[test]
fn prop_bead_with_complete_dep_is_ready() -> Result<(), Box<dyn std::error::Error>> {
    let mut dag = WorkflowDAG::new();
    dag.add_node("bead-a")?;
    dag.add_node("bead-b")?;
    dag.add_edge("bead-a", "bead-b", DependencyType::BlockingDependency)?;

    let mut completed = HashSet::new();
    completed.insert("bead-a".to_string());

    let ready = dag.get_ready_beads(&completed);

    assert!(ready.contains(&"bead-b".to_string()));
    assert!(dag.is_ready("bead-b", &completed)?);

    let deps = dag.get_dependencies("bead-b")?;
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
    dag.add_node("bead-a")?;
    dag.add_node("bead-b")?;
    dag.add_edge("bead-a", "bead-b", DependencyType::BlockingDependency)?;

    let completed = HashSet::new();
    let ready = dag.get_ready_beads(&completed);

    assert!(!ready.contains(&"bead-b".to_string()));
    assert!(!dag.is_ready("bead-b", &completed)?);

    Ok(())
}

#[test]
fn prop_linear_dag_ready_respects_deps() -> Result<(), Box<dyn std::error::Error>> {
    let dag = build_linear_dag(5).map_err(|e| format!("Failed to build DAG: {}", e))?;

    for completed_count in 0..=5 {
        let completed: HashSet<BeadId> = (0..completed_count)
            .map(|i| format!("bead-{}", i))
            .collect();

        let ready = dag.get_ready_beads(&completed);

        for bead_id in &ready {
            assert!(
                dag.is_ready(bead_id, &completed)?,
                "get_ready_beads returned {} but is_ready returned false",
                bead_id
            );

            let deps = dag.get_dependencies(bead_id)?;
            for dep_id in deps {
                assert!(
                    completed.contains(&dep_id),
                    "Ready bead '{}' has incomplete blocking dependency '{}'",
                    bead_id,
                    dep_id
                );
            }
        }
    }

    Ok(())
}

#[test]
fn prop_diamond_dag_ready_respects_deps() -> Result<(), Box<dyn std::error::Error>> {
    let dag = build_diamond_dag().map_err(|e| format!("Failed to build DAG: {}", e))?;

    let test_cases: Vec<HashSet<BeadId>> = vec![
        HashSet::new(),
        vec!["bead-a".to_string()].into_iter().collect(),
        vec!["bead-a".to_string(), "bead-b".to_string()]
            .into_iter()
            .collect(),
        vec!["bead-a".to_string(), "bead-c".to_string()]
            .into_iter()
            .collect(),
        vec![
            "bead-a".to_string(),
            "bead-b".to_string(),
            "bead-c".to_string(),
        ]
        .into_iter()
        .collect(),
    ];

    for completed in test_cases {
        let ready = dag.get_ready_beads(&completed);

        for bead_id in &ready {
            assert!(
                dag.is_ready(bead_id, &completed)?,
                "get_ready_beads returned {} but is_ready returned false",
                bead_id
            );

            let deps = dag.get_dependencies(bead_id)?;
            for dep_id in deps {
                assert!(
                    completed.contains(&dep_id),
                    "Ready bead '{}' has incomplete blocking dependency '{}'",
                    bead_id,
                    dep_id
                );
            }
        }
    }

    Ok(())
}

#[test]
fn prop_blocked_and_ready_are_disjoint() -> Result<(), Box<dyn std::error::Error>> {
    let dag = build_diamond_dag().map_err(|e| format!("Failed to build DAG: {}", e))?;

    let test_cases: Vec<HashSet<BeadId>> = vec![
        HashSet::new(),
        vec!["bead-a".to_string()].into_iter().collect(),
        vec!["bead-a".to_string(), "bead-b".to_string()]
            .into_iter()
            .collect(),
    ];

    for completed in test_cases {
        let ready = dag.get_ready_beads(&completed);
        let blocked = dag.get_blocked_nodes(&completed);

        for bead_id in &ready {
            assert!(
                !blocked.contains(bead_id),
                "Bead '{}' is both ready and blocked",
                bead_id
            );
        }
    }

    Ok(())
}

#[test]
fn prop_completing_bead_never_causes_blocking() -> Result<(), Box<dyn std::error::Error>> {
    let dag = build_diamond_dag().map_err(|e| format!("Failed to build DAG: {}", e))?;

    let completed_before: HashSet<BeadId> = vec!["bead-a".to_string()].into_iter().collect();
    let ready_before = dag.get_ready_beads(&completed_before);

    let mut completed_after = completed_before.clone();
    completed_after.insert("bead-b".to_string());

    let ready_after = dag.get_ready_beads(&completed_after);

    for bead_id in &ready_before {
        if !completed_after.contains(bead_id) {
            assert!(
                ready_after.contains(bead_id) || completed_after.contains(bead_id),
                "Bead '{}' was ready before but not ready after completing another bead",
                bead_id
            );
        }
    }

    Ok(())
}
