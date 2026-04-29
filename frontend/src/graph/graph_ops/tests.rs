//! Tests for graph_ops — queries and mutations.

use super::*;
use crate::graph::{Connection, Node, NodeId, PortName};
use std::collections::{HashMap, HashSet};
use uuid::Uuid;

fn make_connection(source: NodeId, target: NodeId) -> Connection {
    Connection {
        id: Uuid::new_v4(),
        source,
        target,
        source_port: PortName::from("main"),
        target_port: PortName::from("main"),
    }
}

// --- build_node_lookup ---

#[test]
fn given_nodes_when_building_lookup_then_all_ids_are_mapped() {
    let a = NodeId::new();
    let b = NodeId::new();
    let nodes = vec![
        Node::from_workflow_node("a".into(), crate::graph::WorkflowNode::default(), 0.0, 0.0),
        Node::from_workflow_node("b".into(), crate::graph::WorkflowNode::default(), 0.0, 0.0),
    ];
    // Override IDs for the test
    let mut nodes = nodes;
    nodes[0].id = a;
    nodes[1].id = b;

    let lookup = build_node_lookup(&nodes);
    assert!(lookup.contains_key(&a));
    assert!(lookup.contains_key(&b));
    assert_eq!(lookup.len(), 2);
}

// --- collect_node_ids ---

#[test]
fn given_nodes_when_collecting_ids_then_all_ids_are_present() {
    let a = NodeId::new();
    let b = NodeId::new();
    let mut nodes =
        vec![Node::from_workflow_node("a".into(), crate::graph::WorkflowNode::default(), 0.0, 0.0)];
    nodes[0].id = a;

    let ids = collect_node_ids(&nodes);
    assert!(ids.contains(&a));
    assert!(!ids.contains(&b));
}

// --- build_outgoing_adjacency ---

#[test]
fn given_connections_when_building_outgoing_adjacency_then_map_is_correct() {
    let a = NodeId::new();
    let b = NodeId::new();
    let c = NodeId::new();
    let valid: HashSet<NodeId> = [a, b, c].into_iter().collect();

    let connections = vec![make_connection(a, b), make_connection(b, c)];
    let adj = build_outgoing_adjacency(&connections, &valid);

    assert_eq!(adj.get(&a), Some(&vec![b]));
    assert_eq!(adj.get(&b), Some(&vec![c]));
    assert_eq!(adj.get(&c), None);
}

// --- build_reverse_adjacency ---

#[test]
fn given_connections_when_building_reverse_adjacency_then_map_is_correct() {
    let a = NodeId::new();
    let b = NodeId::new();
    let valid: HashSet<NodeId> = [a, b].into_iter().collect();

    let connections = vec![make_connection(a, b)];
    let rev = build_reverse_adjacency(&connections, &valid);

    assert_eq!(rev.get(&b), Some(&vec![a]));
    assert_eq!(rev.get(&a), None);
}

// --- build_adjacency_with_in_degree ---

#[test]
fn given_connections_when_building_adjacency_with_in_degree_then_degrees_are_correct() {
    let a = NodeId::new();
    let b = NodeId::new();
    let c = NodeId::new();
    let valid: HashSet<NodeId> = [a, b, c].into_iter().collect();

    let connections = vec![make_connection(a, b), make_connection(a, c)];
    let (adj, in_deg) = build_adjacency_with_in_degree(&connections, &valid);

    assert_eq!(in_deg.get(&a), Some(&0));
    assert_eq!(in_deg.get(&b), Some(&1));
    assert_eq!(in_deg.get(&c), Some(&1));
    assert_eq!(adj.get(&a), Some(&vec![b, c]));
}

// --- build_connection_membership ---

#[test]
fn given_connections_when_building_membership_then_sets_are_correct() {
    let a = NodeId::new();
    let b = NodeId::new();
    let connections = vec![make_connection(a, b)];
    let (incoming, outgoing) = build_connection_membership(&connections);

    assert!(outgoing.contains(&a));
    assert!(!outgoing.contains(&b));
    assert!(incoming.contains(&b));
    assert!(!incoming.contains(&a));
}

// --- find_reachable ---

#[test]
fn given_linear_chain_when_finding_reachable_then_all_downstream_found() {
    let a = NodeId::new();
    let b = NodeId::new();
    let c = NodeId::new();

    let mut outgoing = HashMap::new();
    outgoing.insert(a, vec![b]);
    outgoing.insert(b, vec![c]);

    let reachable = find_reachable(&[a], &outgoing);
    assert!(reachable.contains(&a));
    assert!(reachable.contains(&b));
    assert!(reachable.contains(&c));
    assert_eq!(reachable.len(), 3);
}

#[test]
fn given_disconnected_graph_when_finding_reachable_then_only_connected_found() {
    let a = NodeId::new();
    let b = NodeId::new();
    let c = NodeId::new();

    let mut outgoing = HashMap::new();
    outgoing.insert(a, vec![b]);
    // c is not connected

    let reachable = find_reachable(&[a], &outgoing);
    assert!(reachable.contains(&a));
    assert!(reachable.contains(&b));
    assert!(!reachable.contains(&c));
}

// --- path_exists ---

#[test]
fn given_direct_edge_when_checking_path_exists_then_true() {
    let a = NodeId::new();
    let b = NodeId::new();
    let connections = vec![make_connection(a, b)];

    assert!(path_exists(&connections, a, b));
    assert!(!path_exists(&connections, b, a));
}

#[test]
fn given_transitive_path_when_checking_path_exists_then_true() {
    let a = NodeId::new();
    let b = NodeId::new();
    let c = NodeId::new();
    let connections = vec![make_connection(a, b), make_connection(b, c)];

    assert!(path_exists(&connections, a, c));
}

#[test]
fn given_same_node_no_self_loop_when_checking_path_exists_then_false() {
    let a = NodeId::new();
    assert!(!path_exists(&[], a, a));
}

#[test]
fn given_self_loop_when_checking_path_exists_from_node_to_itself_then_true() {
    let a = NodeId::new();
    let connections = vec![make_connection(a, a)];
    assert!(path_exists(&connections, a, a));
}

// --- topological_sort ---

#[test]
fn given_linear_chain_when_topsort_then_order_follows_dependencies() {
    let a = NodeId::new();
    let b = NodeId::new();
    let c = NodeId::new();
    let node_ids: HashSet<NodeId> = [a, b, c].into_iter().collect();

    let mut adj = HashMap::new();
    adj.insert(a, vec![b]);
    adj.insert(b, vec![c]);

    let mut in_deg = HashMap::new();
    in_deg.insert(a, 0);
    in_deg.insert(b, 1);
    in_deg.insert(c, 1);

    let result = topological_sort(&node_ids, &adj, &in_deg, |_, _| std::cmp::Ordering::Equal);

    let order = result.expect("linear chain should produce valid order");
    let pos_a = order.iter().position(|&id| id == a).expect("a");
    let pos_b = order.iter().position(|&id| id == b).expect("b");
    let pos_c = order.iter().position(|&id| id == c).expect("c");
    assert!(pos_a < pos_b);
    assert!(pos_b < pos_c);
}

#[test]
fn given_cycle_when_topsort_then_remaining_nodes_returned() {
    let a = NodeId::new();
    let b = NodeId::new();
    let node_ids: HashSet<NodeId> = [a, b].into_iter().collect();

    let mut adj = HashMap::new();
    adj.insert(a, vec![b]);
    adj.insert(b, vec![a]);

    let mut in_deg = HashMap::new();
    in_deg.insert(a, 1);
    in_deg.insert(b, 1);

    let result = topological_sort(&node_ids, &adj, &in_deg, |_, _| std::cmp::Ordering::Equal);

    assert!(result.is_err());
    let remaining = result.err().expect("cycle should remain");
    assert_eq!(remaining.len(), 2);
}
