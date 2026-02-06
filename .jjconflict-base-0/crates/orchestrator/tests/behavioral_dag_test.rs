//! Behavioral tests for WorkflowDAG - Martin Fowler style
//!
//! These tests document WHAT the WorkflowDAG does, not HOW it does it.
//! Following Martin Fowler's testing principles:
//! - Test behavior, not implementation
//! - Tests should survive refactoring
//! - Test names describe behavior
//! - Focus on: given input X → expect output Y
//!
//! The WorkflowDAG is responsible for:
//! - Tracking beads as nodes in a directed graph
//! - Tracking dependencies as edges between beads
//! - Preventing duplicate nodes
//! - Preventing duplicate edges
//! - Querying nodes and edges
//! - Providing iterators over the graph structure
//!
//! CRITICAL: This code is SHIT and needs to be ruthlessly tested.
//! Tests will expose weaknesses, inconsistencies, and missing functionality.

#![deny(clippy::unwrap_used)]
#![deny(clippy::expect_used)]
#![deny(clippy::panic)]

use orchestrator::dag::{DependencyType, WorkflowDAG};

// ============================================================================
// BEHAVIOR: Graph Initialization
// ============================================================================

#[test]
fn should_create_empty_graph() {
    // GIVEN: No prior state
    let dag = WorkflowDAG::new();

    // THEN: Graph should be empty
    assert_eq!(dag.node_count(), 0, "New DAG should have zero nodes");
    assert_eq!(dag.edge_count(), 0, "New DAG should have zero edges");

    // THEN: Iterators should yield nothing
    let nodes: Vec<_> = dag.nodes().collect();
    assert!(nodes.is_empty(), "Nodes iterator should be empty");

    let edges: Vec<_> = dag.edges().collect();
    assert!(edges.is_empty(), "Edges iterator should be empty");
}

// ============================================================================
// BEHAVIOR: Node Addition
// ============================================================================

#[test]
fn should_add_single_node_to_empty_graph() {
    // GIVEN: An empty DAG
    let mut dag = WorkflowDAG::new();

    // WHEN: A node is added
    let bead_id = "bead-001";
    let result = dag.add_node(bead_id.to_string());

    // THEN: The operation should succeed
    assert!(result.is_ok(), "Adding a node should succeed");

    // THEN: The node should be tracked
    assert_eq!(dag.node_count(), 1, "Node count should be 1");

    // THEN: The node should be retrievable
    let nodes: Vec<_> = dag.nodes().collect();
    assert_eq!(nodes.len(), 1, "Should retrieve exactly one node");
    assert!(
        nodes.iter().any(|n| n.as_str() == bead_id),
        "Retrieved node should match added node"
    );
}

#[test]
fn should_prevent_duplicate_node_addition() {
    // GIVEN: A DAG with one node
    let mut dag = WorkflowDAG::new();
    let bead_id = "duplicate-test";
    let first_add = dag.add_node(bead_id.to_string());

    // THEN: First addition should succeed
    assert!(first_add.is_ok(), "First node addition should succeed");

    // WHEN: Attempting to add the same node again
    let second_add = dag.add_node(bead_id.to_string());

    // THEN: The operation should fail
    assert!(second_add.is_err(), "Duplicate node should be rejected");

    // THEN: Graph state should be unchanged
    assert_eq!(dag.node_count(), 1, "Node count should remain 1");

    let nodes: Vec<_> = dag.nodes().collect();
    assert_eq!(nodes.len(), 1, "Only one unique node should exist");
}

#[test]
fn should_add_multiple_distinct_nodes() {
    // GIVEN: An empty DAG
    let mut dag = WorkflowDAG::new();

    // WHEN: Multiple distinct nodes are added
    let beads = ["bead-a", "bead-b", "bead-c"];
    for bead in &beads {
        let result = dag.add_node(bead.to_string());
        assert!(result.is_ok(), "Adding distinct nodes should succeed");
    }

    // THEN: All nodes should be tracked
    assert_eq!(dag.node_count(), 3, "All three nodes should be added");

    // THEN: All nodes should be retrievable
    let nodes: Vec<_> = dag.nodes().collect();
    assert_eq!(nodes.len(), 3, "Should retrieve all three nodes");

    for bead in &beads {
        assert!(
            nodes.iter().any(|n| n.as_str() == *bead),
            "Node '{}' should be retrievable",
            bead
        );
    }
}

// ============================================================================
// BEHAVIOR: Edge Creation
// ============================================================================

#[test]
fn should_create_edge_between_two_nodes() {
    // GIVEN: A DAG with two nodes
    let mut dag = WorkflowDAG::new();
    let from_bead = "source-bead";
    let to_bead = "target-bead";

    let add_from = dag.add_node(from_bead.to_string());
    let add_to = dag.add_node(to_bead.to_string());

    assert!(add_from.is_ok(), "Source node should add");
    assert!(add_to.is_ok(), "Target node should add");

    // WHEN: An edge is created from source to target
    let result = dag.add_edge(
        from_bead.to_string(),
        to_bead.to_string(),
        DependencyType::BlockingDependency,
    );

    // THEN: The operation should succeed
    assert!(result.is_ok(), "Creating edge should succeed");

    // THEN: The edge should be tracked
    assert_eq!(dag.edge_count(), 1, "Edge count should be 1");

    // THEN: The edge should be retrievable
    let edges: Vec<_> = dag.edges().collect();
    assert_eq!(edges.len(), 1, "Should retrieve exactly one edge");
    assert_eq!(edges[0].0, from_bead, "Edge source should match");
    assert_eq!(edges[0].1, to_bead, "Edge target should match");
    assert_eq!(
        *edges[0].2,
        DependencyType::BlockingDependency,
        "Edge type should match"
    );
}

#[test]
fn should_create_multiple_edges_from_same_source() {
    // GIVEN: A DAG with one source and two target nodes
    let mut dag = WorkflowDAG::new();
    let source = "root-bead";
    let target1 = "branch-1";
    let target2 = "branch-2";

    dag.add_node(source.to_string()).ok();
    dag.add_node(target1.to_string()).ok();
    dag.add_node(target2.to_string()).ok();

    // WHEN: Multiple edges are created from the same source
    let edge1 = dag.add_edge(
        source.to_string(),
        target1.to_string(),
        DependencyType::BlockingDependency,
    );
    let edge2 = dag.add_edge(
        source.to_string(),
        target2.to_string(),
        DependencyType::PreferredOrder,
    );

    // THEN: Both edges should be created
    assert!(edge1.is_ok(), "First edge should succeed");
    assert!(edge2.is_ok(), "Second edge should succeed");

    // THEN: Both edges should be tracked
    assert_eq!(dag.edge_count(), 2, "Edge count should be 2");

    let edges: Vec<_> = dag.edges().collect();
    assert_eq!(edges.len(), 2, "Both edges should be retrievable");
}

#[test]
fn should_create_multiple_edges_to_same_target() {
    // GIVEN: A DAG with two source nodes and one target
    let mut dag = WorkflowDAG::new();
    let source1 = "input-1";
    let source2 = "input-2";
    let target = "merge-bead";

    dag.add_node(source1.to_string()).ok();
    dag.add_node(source2.to_string()).ok();
    dag.add_node(target.to_string()).ok();

    // WHEN: Multiple edges are created to the same target
    let edge1 = dag.add_edge(
        source1.to_string(),
        target.to_string(),
        DependencyType::BlockingDependency,
    );
    let edge2 = dag.add_edge(
        source2.to_string(),
        target.to_string(),
        DependencyType::BlockingDependency,
    );

    // THEN: Both edges should be created
    assert!(edge1.is_ok(), "First edge should succeed");
    assert!(edge2.is_ok(), "Second edge should succeed");

    // THEN: Both edges should be tracked
    assert_eq!(dag.edge_count(), 2, "Edge count should be 2");
}

// ============================================================================
// BEHAVIOR: Error Handling - Invalid Edges
// ============================================================================

#[test]
fn should_reject_edge_with_nonexistent_source() {
    // GIVEN: A DAG with one node
    let mut dag = WorkflowDAG::new();
    dag.add_node("existing-node".to_string()).ok();

    // WHEN: Attempting to create an edge from a nonexistent node
    let result = dag.add_edge(
        "nonexistent-source".to_string(),
        "existing-node".to_string(),
        DependencyType::BlockingDependency,
    );

    // THEN: The operation should fail
    assert!(result.is_err(), "Edge with nonexistent source should fail");

    // THEN: Graph state should be unchanged
    assert_eq!(dag.edge_count(), 0, "No edges should be created");
}

#[test]
fn should_reject_edge_with_nonexistent_target() {
    // GIVEN: A DAG with one node
    let mut dag = WorkflowDAG::new();
    dag.add_node("existing-node".to_string()).ok();

    // WHEN: Attempting to create an edge to a nonexistent node
    let result = dag.add_edge(
        "existing-node".to_string(),
        "nonexistent-target".to_string(),
        DependencyType::BlockingDependency,
    );

    // THEN: The operation should fail
    assert!(result.is_err(), "Edge with nonexistent target should fail");

    // THEN: Graph state should be unchanged
    assert_eq!(dag.edge_count(), 0, "No edges should be created");
}

#[test]
fn should_reject_edge_when_both_nodes_nonexistent() {
    // GIVEN: An empty DAG
    let mut dag = WorkflowDAG::new();

    // WHEN: Attempting to create an edge between two nonexistent nodes
    let result = dag.add_edge(
        "ghost-source".to_string(),
        "ghost-target".to_string(),
        DependencyType::BlockingDependency,
    );

    // THEN: The operation should fail
    assert!(result.is_err(), "Edge with nonexistent nodes should fail");
}

// ============================================================================
// BEHAVIOR: Edge Types
// ============================================================================

#[test]
fn should_support_blocking_dependency_type() {
    // GIVEN: A DAG with two nodes
    let mut dag = WorkflowDAG::new();
    dag.add_node("from".to_string()).ok();
    dag.add_node("to".to_string()).ok();

    // WHEN: An edge is created with BlockingDependency type
    let result = dag.add_edge(
        "from".to_string(),
        "to".to_string(),
        DependencyType::BlockingDependency,
    );

    // THEN: The edge should be created with correct type
    assert!(result.is_ok(), "BlockingDependency edge should succeed");

    let edges: Vec<_> = dag.edges().collect();
    assert_eq!(
        *edges[0].2,
        DependencyType::BlockingDependency,
        "Edge type should be BlockingDependency"
    );
}

#[test]
fn should_support_preferred_order_type() {
    // GIVEN: A DAG with two nodes
    let mut dag = WorkflowDAG::new();
    dag.add_node("from".to_string()).ok();
    dag.add_node("to".to_string()).ok();

    // WHEN: An edge is created with PreferredOrder type
    let result = dag.add_edge(
        "from".to_string(),
        "to".to_string(),
        DependencyType::PreferredOrder,
    );

    // THEN: The edge should be created with correct type
    assert!(result.is_ok(), "PreferredOrder edge should succeed");

    let edges: Vec<_> = dag.edges().collect();
    assert_eq!(
        *edges[0].2,
        DependencyType::PreferredOrder,
        "Edge type should be PreferredOrder"
    );
}

#[test]
fn should_support_mixed_edge_types() {
    // GIVEN: A DAG with three nodes
    let mut dag = WorkflowDAG::new();
    dag.add_node("root".to_string()).ok();
    dag.add_node("blocking-target".to_string()).ok();
    dag.add_node("preferred-target".to_string()).ok();

    // WHEN: Edges of both types are created
    let edge1 = dag.add_edge(
        "root".to_string(),
        "blocking-target".to_string(),
        DependencyType::BlockingDependency,
    );
    let edge2 = dag.add_edge(
        "root".to_string(),
        "preferred-target".to_string(),
        DependencyType::PreferredOrder,
    );

    // THEN: Both edges should be created
    assert!(edge1.is_ok(), "BlockingDependency edge should succeed");
    assert!(edge2.is_ok(), "PreferredOrder edge should succeed");

    // THEN: Both edge types should be preserved
    let edges: Vec<_> = dag.edges().collect();
    assert_eq!(edges.len(), 2, "Both edges should exist");

    let blocking_edge = edges
        .iter()
        .find(|e| *e.2 == DependencyType::BlockingDependency);
    let preferred_edge = edges
        .iter()
        .find(|e| *e.2 == DependencyType::PreferredOrder);

    assert!(
        blocking_edge.is_some(),
        "Should have BlockingDependency edge"
    );
    assert!(preferred_edge.is_some(), "Should have PreferredOrder edge");
}

// ============================================================================
// BEHAVIOR: Node Iteration
// ============================================================================

#[test]
fn should_iterate_over_all_nodes() {
    // GIVEN: A DAG with multiple nodes
    let mut dag = WorkflowDAG::new();
    let beads = vec!["a", "b", "c", "d", "e"];

    for bead in &beads {
        dag.add_node(bead.to_string()).ok();
    }

    // WHEN: Iterating over nodes
    let nodes: Vec<_> = dag.nodes().collect();

    // THEN: All nodes should be included
    assert_eq!(nodes.len(), beads.len(), "All nodes should be iterated");

    for bead in &beads {
        assert!(
            nodes.iter().any(|n| n.as_str() == *bead),
            "Node '{}' should be in iterator",
            bead
        );
    }
}

#[test]
fn should_return_empty_iterator_for_empty_graph() {
    // GIVEN: An empty DAG
    let dag = WorkflowDAG::new();

    // WHEN: Iterating over nodes
    let nodes: Vec<_> = dag.nodes().collect();

    // THEN: Iterator should yield nothing
    assert!(nodes.is_empty(), "Empty DAG should have no nodes");
}

// ============================================================================
// BEHAVIOR: Edge Iteration
// ============================================================================

#[test]
fn should_iterate_over_all_edges() {
    // GIVEN: A DAG with multiple edges
    let mut dag = WorkflowDAG::new();

    dag.add_node("a".to_string()).ok();
    dag.add_node("b".to_string()).ok();
    dag.add_node("c".to_string()).ok();

    dag.add_edge(
        "a".to_string(),
        "b".to_string(),
        DependencyType::BlockingDependency,
    )
    .ok();
    dag.add_edge(
        "b".to_string(),
        "c".to_string(),
        DependencyType::PreferredOrder,
    )
    .ok();

    // WHEN: Iterating over edges
    let edges: Vec<_> = dag.edges().collect();

    // THEN: All edges should be included
    assert_eq!(edges.len(), 2, "All edges should be iterated");

    // THEN: Each edge should have correct components
    let edge1 = edges.iter().find(|e| e.0 == "a");
    assert!(
        edge1.is_some(),
        "Failed to find edge from 'a': no edge found in iterator"
    );
    if let Some(e1) = edge1 {
        assert_eq!(e1.1, "b", "Edge 'a' should point to 'b'");
    }

    let edge2 = edges.iter().find(|e| e.0 == "b");
    assert!(
        edge2.is_some(),
        "Failed to find edge from 'b': no edge found in iterator"
    );
    if let Some(e2) = edge2 {
        assert_eq!(e2.1, "c", "Edge 'b' should point to 'c'");
    }
}

#[test]
fn should_return_empty_edge_iterator_for_empty_graph() {
    // GIVEN: An empty DAG
    let dag = WorkflowDAG::new();

    // WHEN: Iterating over edges
    let edges: Vec<_> = dag.edges().collect();

    // THEN: Iterator should yield nothing
    assert!(edges.is_empty(), "Empty DAG should have no edges");
}

#[test]
fn should_return_empty_edge_iterator_for_graph_with_no_edges() {
    // GIVEN: A DAG with nodes but no edges
    let mut dag = WorkflowDAG::new();
    dag.add_node("isolated-1".to_string()).ok();
    dag.add_node("isolated-2".to_string()).ok();

    // WHEN: Iterating over edges
    let edges: Vec<_> = dag.edges().collect();

    // THEN: Iterator should yield nothing
    assert!(
        edges.is_empty(),
        "Graph with only nodes should have no edges"
    );
}

// ============================================================================
// BEHAVIOR: Count Tracking
// ============================================================================

#[test]
fn should_track_node_count_across_operations() {
    // GIVEN: An empty DAG
    let mut dag = WorkflowDAG::new();

    // WHEN: Nodes are added
    dag.add_node("one".to_string()).ok();
    assert_eq!(dag.node_count(), 1, "Count should be 1 after first add");

    dag.add_node("two".to_string()).ok();
    assert_eq!(dag.node_count(), 2, "Count should be 2 after second add");

    dag.add_node("three".to_string()).ok();
    assert_eq!(dag.node_count(), 3, "Count should be 3 after third add");

    // THEN: Duplicate adds should not increase count
    dag.add_node("one".to_string()).ok();
    assert_eq!(dag.node_count(), 3, "Count should remain 3 after duplicate");
}

#[test]
fn should_track_edge_count_across_operations() {
    // GIVEN: A DAG with nodes
    let mut dag = WorkflowDAG::new();
    dag.add_node("a".to_string()).ok();
    dag.add_node("b".to_string()).ok();
    dag.add_node("c".to_string()).ok();

    // WHEN: Edges are added
    dag.add_edge(
        "a".to_string(),
        "b".to_string(),
        DependencyType::BlockingDependency,
    )
    .ok();
    assert_eq!(dag.edge_count(), 1, "Count should be 1 after first edge");

    dag.add_edge(
        "b".to_string(),
        "c".to_string(),
        DependencyType::BlockingDependency,
    )
    .ok();
    assert_eq!(dag.edge_count(), 2, "Count should be 2 after second edge");

    // THEN: Failed edge adds should not increase count
    dag.add_edge(
        "ghost".to_string(),
        "a".to_string(),
        DependencyType::BlockingDependency,
    )
    .ok();
    assert_eq!(
        dag.edge_count(),
        2,
        "Count should remain 2 after failed add"
    );
}

// ============================================================================
// BEHAVIOR: Complex Graph Structures
// ============================================================================

#[test]
fn should_handle_linear_chain() {
    // GIVEN: Building a linear dependency chain A → B → C → D
    let mut dag = WorkflowDAG::new();

    let nodes = ["a", "b", "c", "d"];
    for node in &nodes {
        dag.add_node(node.to_string()).ok();
    }

    dag.add_edge(
        "a".to_string(),
        "b".to_string(),
        DependencyType::BlockingDependency,
    )
    .ok();
    dag.add_edge(
        "b".to_string(),
        "c".to_string(),
        DependencyType::BlockingDependency,
    )
    .ok();
    dag.add_edge(
        "c".to_string(),
        "d".to_string(),
        DependencyType::BlockingDependency,
    )
    .ok();

    // THEN: All nodes and edges should be tracked
    assert_eq!(dag.node_count(), 4, "Should have 4 nodes");
    assert_eq!(dag.edge_count(), 3, "Should have 3 edges");

    // THEN: All edges should form a chain
    let edges: Vec<_> = dag.edges().collect();
    assert_eq!(edges.len(), 3, "Should iterate all edges");
}

#[test]
fn should_handle_diamond_pattern() {
    // GIVEN: Building a diamond dependency pattern:
    //     root
    //    /   \
    //  left   right
    //    \   /
    //     sink
    let mut dag = WorkflowDAG::new();

    let nodes = ["root", "left", "right", "sink"];
    for node in &nodes {
        dag.add_node(node.to_string()).ok();
    }

    dag.add_edge(
        "root".to_string(),
        "left".to_string(),
        DependencyType::BlockingDependency,
    )
    .ok();
    dag.add_edge(
        "root".to_string(),
        "right".to_string(),
        DependencyType::BlockingDependency,
    )
    .ok();
    dag.add_edge(
        "left".to_string(),
        "sink".to_string(),
        DependencyType::BlockingDependency,
    )
    .ok();
    dag.add_edge(
        "right".to_string(),
        "sink".to_string(),
        DependencyType::BlockingDependency,
    )
    .ok();

    // THEN: All nodes and edges should be tracked
    assert_eq!(dag.node_count(), 4, "Should have 4 nodes");
    assert_eq!(dag.edge_count(), 4, "Should have 4 edges");

    // THEN: Edges should form diamond structure
    let edges: Vec<_> = dag.edges().collect();
    assert_eq!(edges.len(), 4, "Should iterate all edges");
}

#[test]
fn should_handle_disconnected_subgraphs() {
    // GIVEN: Building two disconnected chains: A→B and C→D
    let mut dag = WorkflowDAG::new();

    dag.add_node("a".to_string()).ok();
    dag.add_node("b".to_string()).ok();
    dag.add_node("c".to_string()).ok();
    dag.add_node("d".to_string()).ok();

    dag.add_edge(
        "a".to_string(),
        "b".to_string(),
        DependencyType::BlockingDependency,
    )
    .ok();
    dag.add_edge(
        "c".to_string(),
        "d".to_string(),
        DependencyType::BlockingDependency,
    )
    .ok();

    // THEN: All nodes and edges should be tracked
    assert_eq!(dag.node_count(), 4, "Should have 4 nodes");
    assert_eq!(dag.edge_count(), 2, "Should have 2 edges");

    // THEN: Iteration should include all nodes and edges
    let nodes: Vec<_> = dag.nodes().collect();
    assert_eq!(nodes.len(), 4, "Should iterate all nodes");

    let edges: Vec<_> = dag.edges().collect();
    assert_eq!(edges.len(), 2, "Should iterate all edges");
}

// ============================================================================
// BEHAVIOR: Edge Cases and Boundary Conditions
// ============================================================================

#[test]
fn should_handle_empty_string_node_id() {
    // GIVEN: An empty DAG
    let mut dag = WorkflowDAG::new();

    // WHEN: Adding a node with empty string ID
    let result = dag.add_node("".to_string());

    // THEN: The node should be added (empty string is a valid ID)
    // NOTE: This behavior might be undesirable - should we validate IDs?
    assert!(result.is_ok(), "Empty string node should be added");

    let nodes: Vec<_> = dag.nodes().collect();
    assert!(
        nodes.iter().any(|n| n.as_str() == ""),
        "Empty string node should exist"
    );
}

#[test]
fn should_handle_whitespace_node_id() {
    // GIVEN: An empty DAG
    let mut dag = WorkflowDAG::new();

    // WHEN: Adding a node with whitespace-only ID
    let result = dag.add_node("   ".to_string());

    // THEN: The node should be added
    // NOTE: This behavior might be undesirable - should we trim or reject?
    assert!(result.is_ok(), "Whitespace node should be added");

    let nodes: Vec<_> = dag.nodes().collect();
    assert!(
        nodes.iter().any(|n| n.as_str() == "   "),
        "Whitespace node should exist"
    );
}

#[test]
fn should_handle_unicode_node_id() {
    // GIVEN: An empty DAG
    let mut dag = WorkflowDAG::new();

    // WHEN: Adding nodes with Unicode characters
    let unicode1 = "日本語";
    let unicode2 = "🚀rocket";
    let unicode3 = "مصر";

    let result1 = dag.add_node(unicode1.to_string());
    let result2 = dag.add_node(unicode2.to_string());
    let result3 = dag.add_node(unicode3.to_string());

    // THEN: All Unicode nodes should be added
    assert!(result1.is_ok(), "Japanese should be added");
    assert!(result2.is_ok(), "Emoji should be added");
    assert!(result3.is_ok(), "Arabic should be added");

    // THEN: All Unicode nodes should be retrievable
    let nodes: Vec<_> = dag.nodes().collect();
    assert!(
        nodes.iter().any(|n| n.as_str() == unicode1),
        "Japanese node should exist"
    );
    assert!(
        nodes.iter().any(|n| n.as_str() == unicode2),
        "Emoji node should exist"
    );
    assert!(
        nodes.iter().any(|n| n.as_str() == unicode3),
        "Arabic node should exist"
    );
}

#[test]
fn should_handle_very_long_node_id() {
    // GIVEN: An empty DAG
    let mut dag = WorkflowDAG::new();

    // WHEN: Adding a node with very long ID
    let long_id = "a".repeat(10000);
    let result = dag.add_node(long_id.clone());

    // THEN: The node should be added
    assert!(result.is_ok(), "Long ID should be added");

    // THEN: Node should be retrievable
    let nodes: Vec<_> = dag.nodes().collect();
    assert!(
        nodes.iter().any(|n| n.as_str() == long_id.as_str()),
        "Long ID node should exist"
    );
}

// ============================================================================
// BEHAVIOR: Special Characters in Node IDs
// ============================================================================

#[test]
fn should_handle_special_characters_in_node_ids() {
    // GIVEN: An empty DAG
    let mut dag = WorkflowDAG::new();

    // WHEN: Adding nodes with special characters
    let special_ids = vec![
        "bead-with-hyphen",
        "bead_with_underscore",
        "bead.with.dots",
        "bead/with/slashes",
        "bead:with:colons",
        "bead@with@ats",
        "bead#with#hashes",
        "bead!with!exclamations",
        "bead$with$dollars",
        "bead%with%percents",
    ];

    for id in &special_ids {
        let result = dag.add_node(id.to_string());
        assert!(result.is_ok(), "Node with '{}' should be added", id);
    }

    // THEN: All special character nodes should be retrievable
    let nodes: Vec<_> = dag.nodes().collect();
    assert_eq!(
        nodes.len(),
        special_ids.len(),
        "All special nodes should exist"
    );

    for id in &special_ids {
        assert!(
            nodes.iter().any(|n| n.as_str() == *id),
            "Node '{}' should be retrievable",
            id
        );
    }
}

// ============================================================================
// BEHAVIOR: Clone Independence
// ============================================================================

#[test]
fn should_provide_independent_clones() {
    // GIVEN: A DAG with nodes and edges
    let mut dag1 = WorkflowDAG::new();
    dag1.add_node("a".to_string()).ok();
    dag1.add_node("b".to_string()).ok();
    dag1.add_edge(
        "a".to_string(),
        "b".to_string(),
        DependencyType::BlockingDependency,
    )
    .ok();

    // WHEN: The DAG is cloned
    let mut dag2 = dag1.clone();

    // THEN: Clone should have same state
    assert_eq!(
        dag1.node_count(),
        dag2.node_count(),
        "Node counts should match"
    );
    assert_eq!(
        dag1.edge_count(),
        dag2.edge_count(),
        "Edge counts should match"
    );

    // WHEN: Modifying the clone
    dag2.add_node("c".to_string()).ok();

    // THEN: Original should be unchanged
    assert_eq!(dag1.node_count(), 2, "Original should remain unchanged");
    assert_eq!(dag2.node_count(), 3, "Clone should have new node");
}

// ============================================================================
// BEHAVIOR: Default Implementation
// ============================================================================

#[test]
fn should_create_empty_graph_via_default() {
    // GIVEN: Using Default trait
    let dag: WorkflowDAG = WorkflowDAG::default();

    // THEN: Should behave the same as new()
    assert_eq!(dag.node_count(), 0, "Default should create empty graph");
    assert_eq!(dag.edge_count(), 0, "Default should have no edges");
}
