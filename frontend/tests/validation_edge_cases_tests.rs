//! Edge-case tests for workflow validation.
//!
//! Covers: empty workflow, single node, disconnected components,
//! cycles in validation context, and large-node-count workflows.
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic, clippy::float_cmp)]

use oya_frontend::graph::{
    validate_workflow, Connection, Node, NodeCategory, NodeId, ValidationSeverity, Workflow,
};
use uuid::Uuid;

// ===========================================================================
// Empty Workflow
// ===========================================================================

#[test]
fn given_empty_workflow_when_validated_then_reports_no_entry_point() {
    let workflow = Workflow { nodes: vec![], connections: vec![], ..Default::default() };

    let result = validate_workflow(&workflow);

    assert!(!result.valid, "Empty workflow should not be valid");
    assert!(result.has_errors());
    assert!(
        result.issues.iter().any(|i| i.message.contains("entry point")),
        "Should report missing entry point"
    );
}

// ===========================================================================
// Single Node Variants
// ===========================================================================

#[test]
fn given_single_entry_node_when_validated_then_valid() {
    let node = Node {
        id: NodeId::new(),
        name: "HTTP Handler".to_string(),
        category: NodeCategory::Entry,
        ..Default::default()
    };
    let workflow = Workflow { nodes: vec![node], connections: vec![], ..Default::default() };

    let result = validate_workflow(&workflow);

    assert!(result.valid, "Single entry node should be valid");
    assert_eq!(result.error_count(), 0);
}

#[test]
fn given_single_durable_node_when_validated_then_no_entry_point_error() {
    let node = Node {
        id: NodeId::new(),
        name: "Handler".to_string(),
        category: NodeCategory::Durable,
        ..Default::default()
    };
    let workflow = Workflow { nodes: vec![node], connections: vec![], ..Default::default() };

    let result = validate_workflow(&workflow);

    assert!(!result.valid);
    assert!(result.has_errors());
    assert!(
        result.issues.iter().any(|i| i.message.contains("entry point")),
        "Single non-entry node should report missing entry point"
    );
}

#[test]
fn given_single_state_node_when_validated_then_no_entry_point_error() {
    let node = Node {
        id: NodeId::new(),
        name: "State".to_string(),
        category: NodeCategory::State,
        ..Default::default()
    };
    let workflow = Workflow { nodes: vec![node], connections: vec![], ..Default::default() };

    let result = validate_workflow(&workflow);
    assert!(!result.valid);
}

#[test]
fn given_single_flow_node_when_validated_then_no_entry_point_error() {
    let node = Node {
        id: NodeId::new(),
        name: "Router".to_string(),
        category: NodeCategory::Flow,
        ..Default::default()
    };
    let workflow = Workflow { nodes: vec![node], connections: vec![], ..Default::default() };

    let result = validate_workflow(&workflow);
    assert!(!result.valid);
}

#[test]
fn given_single_timing_node_when_validated_then_no_entry_point_error() {
    let node = Node {
        id: NodeId::new(),
        name: "Delay".to_string(),
        category: NodeCategory::Timing,
        ..Default::default()
    };
    let workflow = Workflow { nodes: vec![node], connections: vec![], ..Default::default() };

    let result = validate_workflow(&workflow);
    assert!(!result.valid);
}

#[test]
fn given_single_signal_node_when_validated_then_no_entry_point_error() {
    let node = Node {
        id: NodeId::new(),
        name: "Signal".to_string(),
        category: NodeCategory::Signal,
        ..Default::default()
    };
    let workflow = Workflow { nodes: vec![node], connections: vec![], ..Default::default() };

    let result = validate_workflow(&workflow);
    assert!(!result.valid);
}

// ===========================================================================
// Disconnected Components
// ===========================================================================

#[test]
fn given_two_disconnected_entry_nodes_when_validated_then_valid() {
    let entry1 = Node {
        id: NodeId::new(),
        name: "Entry 1".to_string(),
        category: NodeCategory::Entry,
        ..Default::default()
    };
    let entry2 = Node {
        id: NodeId::new(),
        name: "Entry 2".to_string(),
        category: NodeCategory::Entry,
        ..Default::default()
    };
    let workflow =
        Workflow { nodes: vec![entry1, entry2], connections: vec![], ..Default::default() };

    let result = validate_workflow(&workflow);
    assert!(result.valid, "Two entry nodes with no connections should be valid");
}

#[test]
fn given_disconnected_subgraph_when_validated_then_warns_about_orphan() {
    let entry_id = NodeId::new();
    let connected_id = NodeId::new();
    let orphan_id = NodeId::new();

    let workflow = Workflow {
        nodes: vec![
            Node {
                id: entry_id,
                name: "Entry".to_string(),
                category: NodeCategory::Entry,
                ..Default::default()
            },
            Node {
                id: connected_id,
                name: "Connected".to_string(),
                category: NodeCategory::Durable,
                ..Default::default()
            },
            Node {
                id: orphan_id,
                name: "Orphan".to_string(),
                category: NodeCategory::Durable,
                ..Default::default()
            },
        ],
        connections: vec![Connection {
            id: Uuid::new_v4(),
            source: entry_id,
            target: connected_id,
            source_port: "out".into(),
            target_port: "in".into(),
        }],
        ..Default::default()
    };

    let result = validate_workflow(&workflow);

    // Should be valid (warnings only, no errors) but have warnings about orphan
    let orphan_warnings: Vec<_> = result
        .issues
        .iter()
        .filter(|i| i.node_id == Some(orphan_id) && i.severity == ValidationSeverity::Warning)
        .collect();
    assert!(!orphan_warnings.is_empty(), "Orphan node should produce at least one warning");
}

#[test]
fn given_two_isolated_clusters_when_validated_then_unreachable_cluster_warned() {
    let entry_a = NodeId::new();
    let node_a = NodeId::new();
    let node_b = NodeId::new();
    let node_c = NodeId::new();

    // Cluster 1: entry → A (connected)
    // Cluster 2: B → C (no entry, disconnected)
    let workflow = Workflow {
        nodes: vec![
            Node {
                id: entry_a,
                name: "Entry".to_string(),
                category: NodeCategory::Entry,
                ..Default::default()
            },
            Node {
                id: node_a,
                name: "A".to_string(),
                category: NodeCategory::Durable,
                ..Default::default()
            },
            Node {
                id: node_b,
                name: "B".to_string(),
                category: NodeCategory::Durable,
                ..Default::default()
            },
            Node {
                id: node_c,
                name: "C".to_string(),
                category: NodeCategory::Durable,
                ..Default::default()
            },
        ],
        connections: vec![
            Connection {
                id: Uuid::new_v4(),
                source: entry_a,
                target: node_a,
                source_port: "out".into(),
                target_port: "in".into(),
            },
            Connection {
                id: Uuid::new_v4(),
                source: node_b,
                target: node_c,
                source_port: "out".into(),
                target_port: "in".into(),
            },
        ],
        ..Default::default()
    };

    let result = validate_workflow(&workflow);

    let unreachable_b = result.issues.iter().any(|i| i.node_id == Some(node_b));
    let unreachable_c = result.issues.iter().any(|i| i.node_id == Some(node_c));
    assert!(unreachable_b || unreachable_c, "Nodes in disconnected cluster should be flagged");
}

// ===========================================================================
// Cycles in Validation Context
// ===========================================================================

#[test]
fn given_self_referencing_node_when_validated_then_does_not_panic() {
    let node_id = NodeId::new();
    let node = Node {
        id: node_id,
        name: "Self-ref".to_string(),
        category: NodeCategory::Entry,
        ..Default::default()
    };

    let workflow = Workflow {
        nodes: vec![node],
        connections: vec![Connection {
            id: Uuid::new_v4(),
            source: node_id,
            target: node_id,
            source_port: "out".into(),
            target_port: "in".into(),
        }],
        ..Default::default()
    };

    // Must not panic
    let result = validate_workflow(&workflow);
    let _ = result.issues.len();
}

#[test]
fn given_simple_cycle_when_validated_then_does_not_panic() {
    let a = NodeId::new();
    let b = NodeId::new();

    let workflow = Workflow {
        nodes: vec![
            Node {
                id: a,
                name: "A".to_string(),
                category: NodeCategory::Entry,
                ..Default::default()
            },
            Node {
                id: b,
                name: "B".to_string(),
                category: NodeCategory::Durable,
                ..Default::default()
            },
        ],
        connections: vec![
            Connection {
                id: Uuid::new_v4(),
                source: a,
                target: b,
                source_port: "out".into(),
                target_port: "in".into(),
            },
            Connection {
                id: Uuid::new_v4(),
                source: b,
                target: a,
                source_port: "out".into(),
                target_port: "in".into(),
            },
        ],
        ..Default::default()
    };

    let result = validate_workflow(&workflow);
    let _ = result.issues.len();
}

#[test]
fn given_three_node_cycle_when_validated_then_does_not_panic() {
    let a = NodeId::new();
    let b = NodeId::new();
    let c = NodeId::new();

    let workflow = Workflow {
        nodes: vec![
            Node {
                id: a,
                name: "A".to_string(),
                category: NodeCategory::Entry,
                ..Default::default()
            },
            Node {
                id: b,
                name: "B".to_string(),
                category: NodeCategory::Durable,
                ..Default::default()
            },
            Node {
                id: c,
                name: "C".to_string(),
                category: NodeCategory::Durable,
                ..Default::default()
            },
        ],
        connections: vec![
            Connection {
                id: Uuid::new_v4(),
                source: a,
                target: b,
                source_port: "out".into(),
                target_port: "in".into(),
            },
            Connection {
                id: Uuid::new_v4(),
                source: b,
                target: c,
                source_port: "out".into(),
                target_port: "in".into(),
            },
            Connection {
                id: Uuid::new_v4(),
                source: c,
                target: a,
                source_port: "out".into(),
                target_port: "in".into(),
            },
        ],
        ..Default::default()
    };

    let result = validate_workflow(&workflow);
    let _ = result.issues.len();
}

// ===========================================================================
// Connection Edge Cases
// ===========================================================================

#[test]
fn given_connection_to_nonexistent_node_when_validated_then_does_not_panic() {
    let entry_id = NodeId::new();
    let ghost_id = NodeId::new();

    let workflow = Workflow {
        nodes: vec![Node {
            id: entry_id,
            name: "Entry".to_string(),
            category: NodeCategory::Entry,
            ..Default::default()
        }],
        connections: vec![Connection {
            id: Uuid::new_v4(),
            source: entry_id,
            target: ghost_id,
            source_port: "out".into(),
            target_port: "in".into(),
        }],
        ..Default::default()
    };

    let result = validate_workflow(&workflow);
    let _ = result.issues.len();
}

#[test]
fn given_connection_from_nonexistent_node_when_validated_then_does_not_panic() {
    let ghost_id = NodeId::new();
    let entry_id = NodeId::new();

    let workflow = Workflow {
        nodes: vec![Node {
            id: entry_id,
            name: "Entry".to_string(),
            category: NodeCategory::Entry,
            ..Default::default()
        }],
        connections: vec![Connection {
            id: Uuid::new_v4(),
            source: ghost_id,
            target: entry_id,
            source_port: "out".into(),
            target_port: "in".into(),
        }],
        ..Default::default()
    };

    let result = validate_workflow(&workflow);
    let _ = result.issues.len();
}

#[test]
fn given_duplicate_connections_when_validated_then_does_not_panic() {
    let entry_id = NodeId::new();
    let target_id = NodeId::new();

    let workflow = Workflow {
        nodes: vec![
            Node {
                id: entry_id,
                name: "Entry".to_string(),
                category: NodeCategory::Entry,
                ..Default::default()
            },
            Node {
                id: target_id,
                name: "Target".to_string(),
                category: NodeCategory::Durable,
                ..Default::default()
            },
        ],
        connections: vec![
            Connection {
                id: Uuid::new_v4(),
                source: entry_id,
                target: target_id,
                source_port: "out".into(),
                target_port: "in".into(),
            },
            Connection {
                id: Uuid::new_v4(),
                source: entry_id,
                target: target_id,
                source_port: "out".into(),
                target_port: "in".into(),
            },
        ],
        ..Default::default()
    };

    let result = validate_workflow(&workflow);
    let _ = result.issues.len();
}

// ===========================================================================
// All Node Categories
// ===========================================================================

#[test]
fn given_all_node_categories_when_validated_then_entry_passes_others_fail() {
    for (name, category, expect_valid) in [
        ("Entry", NodeCategory::Entry, true),
        ("Durable", NodeCategory::Durable, false),
        ("State", NodeCategory::State, false),
        ("Flow", NodeCategory::Flow, false),
        ("Timing", NodeCategory::Timing, false),
        ("Signal", NodeCategory::Signal, false),
    ] {
        let node =
            Node { id: NodeId::new(), name: name.to_string(), category, ..Default::default() };
        let workflow = Workflow { nodes: vec![node], connections: vec![], ..Default::default() };

        let result = validate_workflow(&workflow);
        assert_eq!(
            result.valid, expect_valid,
            "{name}: expected valid={expect_valid}, got valid={}",
            result.valid
        );
    }
}

// ===========================================================================
// ValidationResult Methods
// ===========================================================================

#[test]
fn given_workflow_with_only_warnings_when_validated_then_valid_is_true() {
    let entry_id = NodeId::new();
    let orphan_id = NodeId::new();

    // Entry + orphan produces warnings but no errors
    let workflow = Workflow {
        nodes: vec![
            Node {
                id: entry_id,
                name: "Entry".to_string(),
                category: NodeCategory::Entry,
                ..Default::default()
            },
            Node {
                id: orphan_id,
                name: "Orphan".to_string(),
                category: NodeCategory::Durable,
                ..Default::default()
            },
        ],
        connections: vec![],
        ..Default::default()
    };

    let result = validate_workflow(&workflow);

    assert!(result.valid, "Workflow with only warnings should be valid");
    assert_eq!(result.error_count(), 0);
    assert!(result.warning_count() > 0, "Should have warnings");
}

// ===========================================================================
// Large Workflow
// ===========================================================================

#[test]
fn given_large_linear_workflow_when_validated_then_completes_without_panic() {
    let mut nodes = vec![];
    let mut connections = vec![];

    let entry_id = NodeId::new();
    nodes.push(Node {
        id: entry_id,
        name: "Entry".to_string(),
        category: NodeCategory::Entry,
        ..Default::default()
    });

    let mut prev_id = entry_id;
    for i in 1..100 {
        let id = NodeId::new();
        nodes.push(Node {
            id,
            name: format!("Node {i}"),
            category: NodeCategory::Durable,
            ..Default::default()
        });
        connections.push(Connection {
            id: Uuid::new_v4(),
            source: prev_id,
            target: id,
            source_port: "out".into(),
            target_port: "in".into(),
        });
        prev_id = id;
    }

    let workflow = Workflow { nodes, connections, ..Default::default() };

    let result = validate_workflow(&workflow);
    assert!(result.valid || !result.has_errors(), "Large linear workflow should have no errors");
}
