#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
//! BDD Test: Diamond DAG Join Ready Detection
//!
//! Tests that a join node in a diamond-shaped DAG becomes ready
//! only when ALL its dependencies are complete.
//!
//! ## Diamond Pattern
//!
//! ```text
//!     A
//!    / \
//!   B   C
//!    \ /
//!     D
//! ```
//!
//! D is the "join node" with two incoming edges (B→D, C→D).
//! D should only become ready when BOTH B AND C are complete.

use im::HashSet;
use orchestrator::dag::{DependencyType, WorkflowDAG};

/// BDD Test: Join node becomes ready only when all dependencies complete
///
/// GIVEN a diamond-shaped DAG: A → [B, C] → D
/// WHEN progressively completing beads
/// THEN the join node D becomes ready only when ALL dependencies complete
#[test]
fn bdd_diamond_dag_join_ready_all_deps_required() -> Result<(), Box<dyn std::error::Error>> {
    // GIVEN: A diamond-shaped DAG
    //     A
    //    / \
    //   B   C
    //    \ /
    //     D
    let mut dag = WorkflowDAG::new();
    dag.add_node("bead-a")?;
    dag.add_node("bead-b")?;
    dag.add_node("bead-c")?;
    dag.add_node("bead-d")?;

    dag.add_edge("bead-a", "bead-b", DependencyType::BlockingDependency)?;
    dag.add_edge("bead-a", "bead-c", DependencyType::BlockingDependency)?;
    dag.add_edge("bead-b", "bead-d", DependencyType::BlockingDependency)?;
    dag.add_edge("bead-c", "bead-d", DependencyType::BlockingDependency)?;

    // Initial state: nothing completed
    let completed = HashSet::new();
    let ready = dag.get_ready_beads(&completed);

    // THEN: Only A is ready (root node, no dependencies)
    assert_eq!(ready, vec!["bead-a".to_string()]);

    // Verify D is NOT ready initially
    assert!(!dag.is_ready("bead-d", &completed)?);

    Ok(())
}

/// BDD Test: Join node blocked by single incomplete dependency
///
/// GIVEN a diamond DAG where only one branch is complete
/// WHEN B is complete but C is not
/// THEN D remains blocked (not ready)
#[test]
fn bdd_diamond_dag_join_blocked_by_incomplete_branch() -> Result<(), Box<dyn std::error::Error>> {
    // GIVEN: Diamond DAG
    let mut dag = WorkflowDAG::new();
    dag.add_node("bead-a")?;
    dag.add_node("bead-b")?;
    dag.add_node("bead-c")?;
    dag.add_node("bead-d")?;

    dag.add_edge("bead-a", "bead-b", DependencyType::BlockingDependency)?;
    dag.add_edge("bead-a", "bead-c", DependencyType::BlockingDependency)?;
    dag.add_edge("bead-b", "bead-d", DependencyType::BlockingDependency)?;
    dag.add_edge("bead-c", "bead-d", DependencyType::BlockingDependency)?;

    // WHEN: A and B are complete, but C is not
    let mut completed = HashSet::new();
    completed.insert("bead-a".to_string());
    completed.insert("bead-b".to_string());

    let ready = dag.get_ready_beads(&completed);

    // THEN: C is ready (A complete), but D is NOT ready (C incomplete)
    assert!(ready.contains(&"bead-c".to_string()));
    assert!(!ready.contains(&"bead-d".to_string()));

    // Verify D is explicitly blocked
    assert!(!dag.is_ready("bead-d", &completed)?);

    Ok(())
}

/// BDD Test: Join node becomes ready when all branches complete
///
/// GIVEN a diamond DAG where all branches are complete
/// WHEN both B and C are complete
/// THEN D becomes ready
#[test]
fn bdd_diamond_dag_join_ready_when_all_branches_complete() -> Result<(), Box<dyn std::error::Error>>
{
    // GIVEN: Diamond DAG
    let mut dag = WorkflowDAG::new();
    dag.add_node("bead-a")?;
    dag.add_node("bead-b")?;
    dag.add_node("bead-c")?;
    dag.add_node("bead-d")?;

    dag.add_edge("bead-a", "bead-b", DependencyType::BlockingDependency)?;
    dag.add_edge("bead-a", "bead-c", DependencyType::BlockingDependency)?;
    dag.add_edge("bead-b", "bead-d", DependencyType::BlockingDependency)?;
    dag.add_edge("bead-c", "bead-d", DependencyType::BlockingDependency)?;

    // WHEN: A, B, and C are all complete
    let mut completed = HashSet::new();
    completed.insert("bead-a".to_string());
    completed.insert("bead-b".to_string());
    completed.insert("bead-c".to_string());

    let ready = dag.get_ready_beads(&completed);

    // THEN: D is now ready
    assert_eq!(ready, vec!["bead-d".to_string()]);
    assert!(dag.is_ready("bead-d", &completed)?);

    Ok(())
}

/// BDD Test: Progressive join readiness through diamond execution
///
/// GIVEN a diamond DAG
/// WHEN executing beads in valid order
/// THEN join readiness progresses correctly at each step
#[test]
fn bdd_diamond_dag_progressive_join_readiness() -> Result<(), Box<dyn std::error::Error>> {
    // GIVEN: Diamond DAG
    let mut dag = WorkflowDAG::new();
    dag.add_node("bead-a")?;
    dag.add_node("bead-b")?;
    dag.add_node("bead-c")?;
    dag.add_node("bead-d")?;

    dag.add_edge("bead-a", "bead-b", DependencyType::BlockingDependency)?;
    dag.add_edge("bead-a", "bead-c", DependencyType::BlockingDependency)?;
    dag.add_edge("bead-b", "bead-d", DependencyType::BlockingDependency)?;
    dag.add_edge("bead-c", "bead-d", DependencyType::BlockingDependency)?;

    // Step 0: Nothing complete
    let completed = HashSet::new();
    let ready = dag.get_ready_beads(&completed);
    assert_eq!(ready, vec!["bead-a".to_string()]);
    assert!(!dag.is_ready("bead-d", &completed)?);

    // Step 1: A complete
    let mut completed = HashSet::new();
    completed.insert("bead-a".to_string());
    let ready = dag.get_ready_beads(&completed);
    assert!(ready.contains(&"bead-b".to_string()));
    assert!(ready.contains(&"bead-c".to_string()));
    assert!(!ready.contains(&"bead-d".to_string()));
    assert!(!dag.is_ready("bead-d", &completed)?);

    // Step 2: B complete (but not C)
    completed.insert("bead-b".to_string());
    let ready = dag.get_ready_beads(&completed);
    assert!(ready.contains(&"bead-c".to_string()));
    assert!(!ready.contains(&"bead-d".to_string()));
    assert!(!dag.is_ready("bead-d", &completed)?);

    // Step 3: C complete (both branches now done)
    completed.insert("bead-c".to_string());
    let ready = dag.get_ready_beads(&completed);
    assert_eq!(ready, vec!["bead-d".to_string()]);
    assert!(dag.is_ready("bead-d", &completed)?);

    // Step 4: D complete (workflow done)
    completed.insert("bead-d".to_string());
    let ready = dag.get_ready_beads(&completed);
    assert!(ready.is_empty());

    Ok(())
}

/// BDD Test: Join node with asymmetric completion order
///
/// GIVEN a diamond DAG
/// WHEN completing branches in different order (C first, then B)
/// THEN D still becomes ready only when both complete
#[test]
fn bdd_diamond_dag_asymmetric_completion_order() -> Result<(), Box<dyn std::error::Error>> {
    // GIVEN: Diamond DAG
    let mut dag = WorkflowDAG::new();
    dag.add_node("bead-a")?;
    dag.add_node("bead-b")?;
    dag.add_node("bead-c")?;
    dag.add_node("bead-d")?;

    dag.add_edge("bead-a", "bead-b", DependencyType::BlockingDependency)?;
    dag.add_edge("bead-a", "bead-c", DependencyType::BlockingDependency)?;
    dag.add_edge("bead-b", "bead-d", DependencyType::BlockingDependency)?;
    dag.add_edge("bead-c", "bead-d", DependencyType::BlockingDependency)?;

    // Complete A first
    let mut completed = HashSet::new();
    completed.insert("bead-a".to_string());

    // WHEN: Complete C first (asymmetric order)
    completed.insert("bead-c".to_string());
    let ready = dag.get_ready_beads(&completed);

    // THEN: B is still ready, but D is NOT ready (B incomplete)
    assert!(ready.contains(&"bead-b".to_string()));
    assert!(!ready.contains(&"bead-d".to_string()));
    assert!(!dag.is_ready("bead-d", &completed)?);

    // Complete B now
    completed.insert("bead-b".to_string());
    let ready = dag.get_ready_beads(&completed);

    // THEN: D becomes ready
    assert_eq!(ready, vec!["bead-d".to_string()]);
    assert!(dag.is_ready("bead-d", &completed)?);

    Ok(())
}

/// BDD Test: Blocked nodes list includes join when incomplete
///
/// GIVEN a diamond DAG with partial completion
/// WHEN only one branch is complete
/// THEN D appears in blocked nodes list
#[test]
fn bdd_diamond_dag_join_in_blocked_list() -> Result<(), Box<dyn std::error::Error>> {
    // GIVEN: Diamond DAG
    let mut dag = WorkflowDAG::new();
    dag.add_node("bead-a")?;
    dag.add_node("bead-b")?;
    dag.add_node("bead-c")?;
    dag.add_node("bead-d")?;

    dag.add_edge("bead-a", "bead-b", DependencyType::BlockingDependency)?;
    dag.add_edge("bead-a", "bead-c", DependencyType::BlockingDependency)?;
    dag.add_edge("bead-b", "bead-d", DependencyType::BlockingDependency)?;
    dag.add_edge("bead-c", "bead-d", DependencyType::BlockingDependency)?;

    // WHEN: A and B complete, C incomplete
    let mut completed = HashSet::new();
    completed.insert("bead-a".to_string());
    completed.insert("bead-b".to_string());

    let blocked = dag.get_blocked_nodes(&completed);

    // THEN: D is in blocked list (waiting for C)
    assert!(blocked.contains(&"bead-d".to_string()));
    assert!(!blocked.contains(&"bead-c".to_string()));

    Ok(())
}

/// BDD Test: Multi-way join (3+ incoming edges)
///
/// GIVEN a DAG with 3-way join: A → [B, C, D] → E
/// WHEN completing beads progressively
/// THEN E becomes ready only when ALL dependencies complete
#[test]
fn bdd_diamond_dag_multi_way_join() -> Result<(), Box<dyn std::error::Error>> {
    // GIVEN: Multi-way join DAG
    //     A
    //   / | \
    //  B  C  D
    //   \ | /
    //     E
    let mut dag = WorkflowDAG::new();
    dag.add_node("bead-a")?;
    dag.add_node("bead-b")?;
    dag.add_node("bead-c")?;
    dag.add_node("bead-d")?;
    dag.add_node("bead-e")?;

    dag.add_edge("bead-a", "bead-b", DependencyType::BlockingDependency)?;
    dag.add_edge("bead-a", "bead-c", DependencyType::BlockingDependency)?;
    dag.add_edge("bead-a", "bead-d", DependencyType::BlockingDependency)?;
    dag.add_edge("bead-b", "bead-e", DependencyType::BlockingDependency)?;
    dag.add_edge("bead-c", "bead-e", DependencyType::BlockingDependency)?;
    dag.add_edge("bead-d", "bead-e", DependencyType::BlockingDependency)?;

    // Complete A, B, C (D incomplete)
    let mut completed = HashSet::new();
    completed.insert("bead-a".to_string());
    completed.insert("bead-b".to_string());
    completed.insert("bead-c".to_string());

    // THEN: E is NOT ready (D incomplete)
    assert!(!dag.is_ready("bead-e", &completed)?);
    let blocked = dag.get_blocked_nodes(&completed);
    assert!(blocked.contains(&"bead-e".to_string()));

    // Complete D
    completed.insert("bead-d".to_string());

    // THEN: E is now ready
    assert!(dag.is_ready("bead-e", &completed)?);
    let ready = dag.get_ready_beads(&completed);
    assert_eq!(ready, vec!["bead-e".to_string()]);

    Ok(())
}

/// BDD Test: PreferredOrder dependencies don't block join
///
/// GIVEN a diamond DAG with mixed dependency types
/// WHEN join has PreferredOrder + BlockingDependency
/// THEN only BlockingDependency affects readiness
#[test]
fn bdd_diamond_dag_preferred_order_not_blocking() -> Result<(), Box<dyn std::error::Error>> {
    // GIVEN: Diamond with mixed dependency types
    //     A --blocking--> B
    //     A --preferred-> C
    //     B --blocking--> D
    //     C --preferred-> D
    let mut dag = WorkflowDAG::new();
    dag.add_node("bead-a")?;
    dag.add_node("bead-b")?;
    dag.add_node("bead-c")?;
    dag.add_node("bead-d")?;

    dag.add_edge("bead-a", "bead-b", DependencyType::BlockingDependency)?;
    dag.add_edge("bead-a", "bead-c", DependencyType::PreferredOrder)?;
    dag.add_edge("bead-b", "bead-d", DependencyType::BlockingDependency)?;
    dag.add_edge("bead-c", "bead-d", DependencyType::PreferredOrder)?;

    // WHEN: A and B complete (C incomplete, but C→D is PreferredOrder)
    let mut completed = HashSet::new();
    completed.insert("bead-a".to_string());
    completed.insert("bead-b".to_string());

    // THEN: D IS ready (only blocking dep is B, which is complete)
    assert!(dag.is_ready("bead-d", &completed)?);

    let ready = dag.get_ready_beads(&completed);
    assert!(ready.contains(&"bead-d".to_string()));

    Ok(())
}
