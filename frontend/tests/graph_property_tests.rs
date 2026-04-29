//! Property-based tests for graph operations.
//!
//! Tests invariants over arbitrary inputs using proptest strategies:
//! 1. Node position update idempotency (zero delta)
//! 2. Viewport zoom clamping
//! 3. Connection addition idempotency (duplicate rejection)
//! 4. Node removal completeness (orphan connection cleanup)
//! 5. Workflow serde round-trip
//! 6. Topological sort validity for DAGs
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic, clippy::float_cmp)]

use oya_frontend::graph::{
    calc, layout::DagLayout, Connection, NodeId, PortName, Viewport, Workflow,
};
use petgraph::algo::toposort;
use petgraph::Graph;
use proptest::prelude::*;
use std::collections::HashMap;

// ===========================================================================
// Strategies
// ===========================================================================

fn compatible_node_pair_strategy() -> impl Strategy<Value = (&'static str, &'static str)> {
    prop_oneof![
        Just(("run", "run")),
        Just(("run", "condition")),
        Just(("http-handler", "run")),
        Just(("http-handler", "service-call")),
        Just(("service-call", "run")),
        Just(("service-call", "sleep")),
        Just(("run", "sleep")),
        Just(("sleep", "run")),
    ]
}

// ===========================================================================
// 1. Node position update is idempotent with zero delta
// ===========================================================================

proptest! {
    #[test]
    fn prop_node_position_idempotent_zero_delta(
        x in -10000.0f32..10000.0,
        y in -10000.0f32..10000.0,
    ) {
        // update_node_position applies grid snapping (round to nearest 10).
        // The invariant is that calling it twice with (0,0) produces identical
        // results: the operation is idempotent after the first snap.
        let (snapped_x, snapped_y) = calc::update_node_position(x, y, 0.0, 0.0);

        let (double_snapped_x, double_snapped_y) =
            calc::update_node_position(snapped_x, snapped_y, 0.0, 0.0);

        prop_assert_eq!((snapped_x, snapped_y), (double_snapped_x, double_snapped_y),
            "Grid-snapped position must be stable: ({}, {}) vs ({}, {})",
            snapped_x, snapped_y, double_snapped_x, double_snapped_y);
    }
}

proptest! {
    #[test]
    fn prop_node_position_grid_aligned_unchanged_by_zero_delta(
        grid_x in -1000i32..1000,
        grid_y in -1000i32..1000,
    ) {
        // Positions already on the 10-unit grid must not move with zero delta.
        let x = (grid_x * 10) as f32;
        let y = (grid_y * 10) as f32;

        let (result_x, result_y) = calc::update_node_position(x, y, 0.0, 0.0);

        prop_assert_eq!((x, y), (result_x, result_y),
            "Grid-aligned position ({}, {}) should not move, got ({}, {})",
            x, y, result_x, result_y);
    }
}

// ===========================================================================
// 2. Viewport zoom clamping stays within bounds
// ===========================================================================

proptest! {
    #[test]
    fn prop_zoom_clamped_after_arbitrary_delta(
        current_zoom in 0.1f32..5.0,
        delta in -2.0f32..2.0,
    ) {
        let result = calc::calculate_zoom_delta(delta, current_zoom);

        prop_assert!(result >= 0.1, "Zoom {} is below minimum 0.1", result);
        prop_assert!(result <= 5.0, "Zoom {} is above maximum 5.0", result);
        prop_assert!(result.is_finite(), "Zoom {} is not finite", result);
    }
}

proptest! {
    #[test]
    fn prop_zoom_clamped_for_nan_and_infinity(
        delta in prop_oneof![Just(f32::NAN), Just(f32::INFINITY), Just(f32::NEG_INFINITY)],
        current_zoom in 0.1f32..5.0,
    ) {
        let result = calc::calculate_zoom_delta(delta, current_zoom);

        prop_assert!(result >= 0.1, "Zoom {} is below minimum for non-finite delta", result);
        prop_assert!(result <= 5.0, "Zoom {} is above maximum for non-finite delta", result);
    }
}

proptest! {
    #[test]
    fn prop_zoom_clamped_for_invalid_current_zoom(
        delta in -1.0f32..1.0,
        bad_zoom in prop_oneof![Just(f32::NAN), Just(0.0f32), Just(-1.0f32), Just(f32::INFINITY)],
    ) {
        let result = calc::calculate_zoom_delta(delta, bad_zoom);

        // For bad current_zoom the function returns a fallback (1.0 or clamped)
        prop_assert!(result >= 0.1, "Zoom {} is below minimum for bad current_zoom", result);
        prop_assert!(result <= 5.0, "Zoom {} is above maximum for bad current_zoom", result);
        prop_assert!(result.is_finite(), "Zoom must be finite even with bad inputs");
    }
}

// ===========================================================================
// 3. Connection addition is idempotent (duplicate rejected)
// ===========================================================================

proptest! {
    #[test]
    fn prop_duplicate_connection_rejected(
        node_types in compatible_node_pair_strategy(),
    ) {
        let mut wf = Workflow::new();
        let src = wf.add_node(node_types.0, 0.0, 0.0);
        let tgt = wf.add_node(node_types.1, 100.0, 0.0);
        let port = PortName::from("main");

        // First add should succeed
        let first = wf.add_connection_checked(src, tgt, &port, &port);
        prop_assert!(first.is_ok(), "First connection should succeed: {:?}", first);

        // Second add (duplicate) should be rejected
        let duplicate = wf.add_connection_checked(src, tgt, &port, &port);
        prop_assert!(
            matches!(duplicate, Err(oya_frontend::graph::ConnectivityConnectionError::Duplicate)),
            "Duplicate connection must be rejected, got: {:?}",
            duplicate
        );

        // Connection count must remain 1
        prop_assert_eq!(wf.connections.len(), 1, "Only one connection should exist");
    }
}

// ===========================================================================
// 4. Node removal is complete (orphan connections cleaned up)
// ===========================================================================

proptest! {
    #[test]
    fn prop_node_removal_cleans_all_connections(
        node_count in 3usize..8,
        remove_index in 0usize..6,
    ) {
        // Clamp remove_index to node_count
        let remove_index = remove_index.min(node_count - 1);

        let mut wf = Workflow::new();
        let mut ids: Vec<NodeId> = Vec::new();
        for _ in 0..node_count {
            ids.push(wf.add_node("run", 0.0, 0.0));
        }

        // Chain connections: 0->1->2->...->(n-1)
        let port = PortName::from("main");
        for i in 0..node_count - 1 {
            let _ = wf.add_connection_checked(ids[i], ids[i + 1], &port, &port);
        }

        let removed_id = ids[remove_index];
        wf.remove_node(removed_id);

        // The node itself must be gone
        prop_assert!(
            !wf.nodes.iter().any(|n| n.id == removed_id),
            "Removed node must not be in workflow"
        );

        // No connection may reference the removed node as source or target
        let orphan_count = wf.connections.iter()
            .filter(|c| c.source == removed_id || c.target == removed_id)
            .count();
        prop_assert_eq!(
            orphan_count, 0,
            "All connections referencing removed node must be deleted"
        );

        // Node count must decrease by exactly 1
        prop_assert_eq!(wf.nodes.len(), node_count - 1);
    }
}

proptest! {
    #[test]
    fn prop_removing_all_nodes_leaves_empty_connections(
        node_count in 1usize..5,
    ) {
        let mut wf = Workflow::new();
        let mut ids: Vec<NodeId> = Vec::new();
        for _ in 0..node_count {
            ids.push(wf.add_node("run", 0.0, 0.0));
        }

        let port = PortName::from("main");
        for i in 0..node_count.saturating_sub(1) {
            let _ = wf.add_connection_checked(ids[i], ids[i + 1], &port, &port);
        }

        for id in ids {
            wf.remove_node(id);
        }

        prop_assert!(wf.nodes.is_empty(), "All nodes must be removed");
        prop_assert!(wf.connections.is_empty(), "All connections must be removed when no nodes remain");
    }
}

// ===========================================================================
// 5. Workflow serde round-trip preserves data
// ===========================================================================

proptest! {
    #[test]
    fn prop_workflow_serde_roundtrip(
        node_count in 0usize..6,
    ) {
        let mut wf = Workflow::new();
        for i in 0..node_count {
            wf.add_node("run", (i * 100) as f32, (i * 50) as f32);
        }

        // Add some valid connections between compatible nodes
        let port = PortName::from("main");
        let ids: Vec<NodeId> = wf.nodes.iter().map(|n| n.id).collect();
        for i in 0..ids.len().saturating_sub(1) {
            let _ = wf.add_connection_checked(ids[i], ids[i + 1], &port, &port);
        }

        // Set a non-default viewport
        wf.viewport = Viewport { x: 42.0, y: -17.5, zoom: 1.5 };

        let json = serde_json::to_string(&wf).expect("Serialization must succeed");
        let deserialized: Workflow = serde_json::from_str(&json).expect("Deserialization must succeed");

        // Nodes and connections are preserved (skip fields with `#[serde(skip)]`)
        prop_assert_eq!(wf.nodes.len(), deserialized.nodes.len(),
            "Node count must survive round-trip");
        prop_assert_eq!(wf.connections.len(), deserialized.connections.len(),
            "Connection count must survive round-trip");
        prop_assert_eq!(wf.viewport, deserialized.viewport,
            "Viewport must survive round-trip");

        // Verify node IDs are preserved
        for (original, round_tripped) in wf.nodes.iter().zip(deserialized.nodes.iter()) {
            prop_assert_eq!(original.id, round_tripped.id,
                "Node IDs must survive round-trip");
            prop_assert_eq!(&original.name, &round_tripped.name,
                "Node names must survive round-trip");
            prop_assert!((original.x - round_tripped.x).abs() < f32::EPSILON,
                "Node x must survive round-trip");
            prop_assert!((original.y - round_tripped.y).abs() < f32::EPSILON,
                "Node y must survive round-trip");
        }
    }
}

proptest! {
    #[test]
    fn prop_connection_serde_roundtrip_preserves_ports(
        node_types in compatible_node_pair_strategy(),
    ) {
        let mut wf = Workflow::new();
        let src = wf.add_node(node_types.0, 0.0, 0.0);
        let tgt = wf.add_node(node_types.1, 100.0, 0.0);
        let src_port = PortName::from("main");
        let tgt_port = PortName::from("main");

        let _ = wf.add_connection_checked(src, tgt, &src_port, &tgt_port);

        let original_conn = wf.connections.first().cloned();
        prop_assert!(original_conn.is_some(), "Connection should have been added");

        let conn = original_conn.unwrap();
        let json = serde_json::to_string(&conn).expect("Connection serialization must succeed");
        let round_tripped: Connection = serde_json::from_str(&json)
            .expect("Connection deserialization must succeed");

        prop_assert_eq!(conn.source, round_tripped.source, "Source must survive round-trip");
        prop_assert_eq!(conn.target, round_tripped.target, "Target must survive round-trip");
        prop_assert_eq!(conn.source_port, round_tripped.source_port, "Source port must survive round-trip");
        prop_assert_eq!(conn.target_port, round_tripped.target_port, "Target port must survive round-trip");
    }
}

// ===========================================================================
// 6. Topological sort is valid for any DAG
// ===========================================================================

proptest! {
    #[test]
    fn prop_toposort_valid_for_chained_dag(
        node_count in 1usize..20,
    ) {
        let mut wf = Workflow::new();
        let mut ids: Vec<NodeId> = Vec::new();
        for _ in 0..node_count {
            ids.push(wf.add_node("run", 0.0, 0.0));
        }

        // Build a chain: 0->1->2->...->(n-1) (a valid DAG)
        let port = PortName::from("main");
        for i in 0..ids.len().saturating_sub(1) {
            let _ = wf.add_connection_checked(ids[i], ids[i + 1], &port, &port);
        }

        let sorted = toposort_from_workflow(&wf);

        prop_assert!(sorted.is_some(), "Chain DAG must be sortable");

        let order = sorted.unwrap();
        prop_assert_eq!(order.len(), node_count, "All nodes must appear in toposort");

        // Verify: every node appears before its dependents
        let position: HashMap<NodeId, usize> = order.iter()
            .enumerate()
            .map(|(pos, id)| (*id, pos))
            .collect();

        for conn in &wf.connections {
            let src_pos = position.get(&conn.source);
            let tgt_pos = position.get(&conn.target);
            prop_assert!(src_pos.is_some(), "Source must be in toposort");
            prop_assert!(tgt_pos.is_some(), "Target must be in toposort");

            if let (Some(&sp), Some(&tp)) = (src_pos, tgt_pos) {
                prop_assert!(sp < tp,
                    "Source {:?} (pos {}) must appear before target {:?} (pos {})",
                    conn.source, sp, conn.target, tp);
            }
        }
    }
}

proptest! {
    #[test]
    fn prop_toposort_valid_for_branching_dag(
        node_count in 4usize..12,
        branch_from in 0usize..3,
    ) {
        let branch_from = branch_from.min(node_count - 2);

        let mut wf = Workflow::new();
        let mut ids: Vec<NodeId> = Vec::new();
        for _ in 0..node_count {
            ids.push(wf.add_node("run", 0.0, 0.0));
        }

        // Build chain
        let port = PortName::from("main");
        for i in 0..ids.len().saturating_sub(1) {
            let _ = wf.add_connection_checked(ids[i], ids[i + 1], &port, &port);
        }

        // Add extra branch: branch_from -> last node (skip connection)
        if branch_from + 2 < node_count {
            let _ = wf.add_connection_checked(ids[branch_from], ids[node_count - 1], &port, &port);
        }

        let sorted = toposort_from_workflow(&wf);
        prop_assert!(sorted.is_some(), "Branching DAG must be sortable");

        let order = sorted.unwrap();
        prop_assert_eq!(order.len(), node_count, "All nodes must appear");

        let position: HashMap<NodeId, usize> = order.iter()
            .enumerate()
            .map(|(pos, id)| (*id, pos))
            .collect();

        for conn in &wf.connections {
            let sp = position[&conn.source];
            let tp = position[&conn.target];
            prop_assert!(sp < tp,
                "Source must precede target in toposort");
        }
    }
}

proptest! {
    #[test]
    fn prop_dag_layout_idempotent(
        node_count in 2usize..8,
    ) {
        let mut wf = Workflow::new();
        let mut ids: Vec<NodeId> = Vec::new();
        for _ in 0..node_count {
            ids.push(wf.add_node("run", 0.0, 0.0));
        }

        let port = PortName::from("main");
        for i in 0..ids.len().saturating_sub(1) {
            let _ = wf.add_connection_checked(ids[i], ids[i + 1], &port, &port);
        }

        let layout = DagLayout::default();
        layout.apply(&mut wf);

        let first_positions: Vec<(f32, f32)> = wf.nodes.iter().map(|n| (n.x, n.y)).collect();

        layout.apply(&mut wf);
        let second_positions: Vec<(f32, f32)> = wf.nodes.iter().map(|n| (n.x, n.y)).collect();

        prop_assert_eq!(first_positions, second_positions,
            "Applying layout twice must produce identical positions");
    }
}

// ===========================================================================
// Helper: perform topological sort via petgraph on a Workflow
// ===========================================================================

fn toposort_from_workflow(workflow: &Workflow) -> Option<Vec<NodeId>> {
    let mut graph = Graph::<NodeId, ()>::new();
    let mut index_map = HashMap::new();

    for node in &workflow.nodes {
        let idx = graph.add_node(node.id);
        index_map.insert(node.id, idx);
    }

    for conn in &workflow.connections {
        if let (Some(&src), Some(&tgt)) = (index_map.get(&conn.source), index_map.get(&conn.target))
        {
            graph.add_edge(src, tgt, ());
        }
    }

    toposort(&graph, None).ok().map(|indices| indices.into_iter().map(|idx| graph[idx]).collect())
}
