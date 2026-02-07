//! Test DAG builder pattern implementation

#![deny(clippy::unwrap_used)]
#![deny(clippy::expect_used)]
#![deny(clippy::panic)]

use orchestrator::dag::{DependencyType, WorkflowDAG};

#[test]
fn test_builder_basic() {
    // GIVEN: A builder with nodes and edges
    let result = WorkflowDAG::builder()
        .with_nodes(["a", "b", "c"].map(String::from))
        .with_edges(
            [
                ("a", "b", DependencyType::BlockingDependency),
                ("b", "c", DependencyType::BlockingDependency),
            ]
            .map(|(a, b, t)| (a.to_string(), b.to_string(), t)),
        )
        .build();

    // THEN: Should successfully build DAG
    let dag = match result {
        Ok(dag) => dag,
        Err(e) => panic!("Builder should succeed but failed with: {:?}", e),
    };
    assert_eq!(dag.node_count(), 3);
    assert_eq!(dag.edge_count(), 2);
}

#[test]
fn test_builder_with_large_dag() {
    // GIVEN: A builder with many nodes
    let result = WorkflowDAG::builder()
        .with_nodes((0..100).map(|i| format!("node_{}", i)))
        .with_edges((0..90).map(|i| {
            (
                format!("node_{}", i),
                format!("node_{}", i + 10),
                DependencyType::BlockingDependency,
            )
        }))
        .build();

    // THEN: Should successfully build large DAG
    let dag = match result {
        Ok(dag) => dag,
        Err(e) => panic!("Builder should succeed for large DAG but failed with: {:?}", e),
    };
    assert_eq!(dag.node_count(), 100);
    assert_eq!(dag.edge_count(), 90);
}

#[test]
fn test_builder_single_edge() {
    // GIVEN: A builder using single edge method
    let result = WorkflowDAG::builder()
        .with_nodes(["a", "b"].map(String::from))
        .with_edge(
            "a".to_string(),
            "b".to_string(),
            DependencyType::BlockingDependency,
        )
        .build();

    // THEN: Should successfully build DAG
    let dag = match result {
        Ok(dag) => dag,
        Err(e) => panic!("Builder should succeed but failed with: {:?}", e),
    };
    assert_eq!(dag.node_count(), 2);
    assert_eq!(dag.edge_count(), 1);
}
