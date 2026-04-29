//! Read-only graph queries: lookups, adjacency maps, traversal, and
//! topological sort.

use crate::graph::{Connection, Node, NodeId};
use std::collections::{HashMap, HashSet};

// ===========================================================================
// Node Lookup
// ===========================================================================

/// Build a `NodeId -> &Node` lookup map from a node slice.
///
/// O(n) construction, then O(1) lookups. Use this instead of repeated
/// `nodes.iter().find(|n| n.id == id)` calls which are O(n) each.
#[must_use]
pub fn build_node_lookup(nodes: &[Node]) -> HashMap<NodeId, &Node> {
    nodes.iter().map(|n| (n.id, n)).collect()
}

/// Collect all node IDs into a `HashSet`.
///
/// Replaces the repeated `nodes.iter().map(|n| n.id).collect()` pattern.
#[must_use]
pub fn collect_node_ids(nodes: &[Node]) -> HashSet<NodeId> {
    nodes.iter().map(|n| n.id).collect()
}

// ===========================================================================
// Adjacency Maps
// ===========================================================================

/// Build an outgoing adjacency map: `source -> [targets]`.
///
/// Only includes connections where both endpoints exist in `valid_node_ids`.
#[must_use]
pub fn build_outgoing_adjacency(
    connections: &[Connection],
    valid_node_ids: &HashSet<NodeId>,
) -> HashMap<NodeId, Vec<NodeId>> {
    let mut adjacency: HashMap<NodeId, Vec<NodeId>> = HashMap::new();
    for conn in connections {
        if valid_node_ids.contains(&conn.source) && valid_node_ids.contains(&conn.target) {
            adjacency.entry(conn.source).or_default().push(conn.target);
        }
    }
    adjacency
}

/// Build a reverse adjacency map: `target -> [sources]`.
///
/// Only includes connections where both endpoints exist in `valid_node_ids`.
#[must_use]
pub fn build_reverse_adjacency(
    connections: &[Connection],
    valid_node_ids: &HashSet<NodeId>,
) -> HashMap<NodeId, Vec<NodeId>> {
    let mut reverse: HashMap<NodeId, Vec<NodeId>> = HashMap::new();
    for conn in connections {
        if valid_node_ids.contains(&conn.source) && valid_node_ids.contains(&conn.target) {
            reverse.entry(conn.target).or_default().push(conn.source);
        }
    }
    reverse
}

/// Build outgoing adjacency map with in-degree counts.
///
/// Returns both the adjacency map and a map of `node_id -> in_degree`.
/// In-degrees default to 0 for all nodes in `valid_node_ids`.
#[must_use]
pub fn build_adjacency_with_in_degree(
    connections: &[Connection],
    valid_node_ids: &HashSet<NodeId>,
) -> (HashMap<NodeId, Vec<NodeId>>, HashMap<NodeId, usize>) {
    let mut adjacency: HashMap<NodeId, Vec<NodeId>> = HashMap::new();
    let mut in_degree: HashMap<NodeId, usize> = valid_node_ids.iter().map(|&id| (id, 0)).collect();

    for conn in connections {
        if valid_node_ids.contains(&conn.source) && valid_node_ids.contains(&conn.target) {
            adjacency.entry(conn.source).or_default().push(conn.target);
            if let Some(deg) = in_degree.get_mut(&conn.target) {
                *deg += 1;
            }
        }
    }

    (adjacency, in_degree)
}

/// Build incoming/outgoing membership sets from connections.
///
/// Returns `(has_incoming, has_outgoing)` sets for O(1) membership checks.
/// Used by orphan node detection and entry node finding.
#[must_use]
pub fn build_connection_membership(
    connections: &[Connection],
) -> (HashSet<NodeId>, HashSet<NodeId>) {
    let mut has_incoming: HashSet<NodeId> = HashSet::new();
    let mut has_outgoing: HashSet<NodeId> = HashSet::new();

    for conn in connections {
        has_outgoing.insert(conn.source);
        has_incoming.insert(conn.target);
    }

    (has_incoming, has_outgoing)
}

// ===========================================================================
// Graph Traversal
// ===========================================================================

/// Find all nodes reachable from `start_ids` via outgoing edges.
///
/// Uses an iterative DFS with a visited set. Returns the set of all
/// visited node IDs (including the start IDs themselves).
#[must_use]
pub fn find_reachable(
    start_ids: &[NodeId],
    outgoing: &HashMap<NodeId, Vec<NodeId>>,
) -> HashSet<NodeId> {
    let mut visited = HashSet::new();
    let mut stack: Vec<NodeId> = start_ids.to_vec();

    while let Some(current) = stack.pop() {
        if visited.insert(current) {
            if let Some(targets) = outgoing.get(&current) {
                for target in targets {
                    if !visited.contains(target) {
                        stack.push(*target);
                    }
                }
            }
        }
    }

    visited
}

/// Check if a path exists from `from` to `to` via the given connections.
///
/// Uses an iterative DFS. If `from == to`, returns `true` only if there
/// is an explicit self-loop connection.
#[must_use]
pub fn path_exists(connections: &[Connection], from: NodeId, to: NodeId) -> bool {
    if from == to {
        return connections.iter().any(|conn| conn.source == from && conn.target == from);
    }

    let mut visited = HashSet::new();
    let mut stack = vec![from];

    while let Some(current) = stack.pop() {
        if current == to {
            return true;
        }

        if visited.insert(current) {
            connections
                .iter()
                .filter(|connection| connection.source == current)
                .for_each(|connection| stack.push(connection.target));
        }
    }

    false
}

// ===========================================================================
// Topological Sort (Kahn's Algorithm)
// ===========================================================================

/// Perform topological sort using Kahn's algorithm.
///
/// Returns the topological order on success, or the set of unprocessed
/// node IDs if a cycle was detected.
///
/// # Arguments
/// * `node_ids` - All node IDs to sort
/// * `adjacency` - Pre-built outgoing adjacency map
/// * `in_degree` - Pre-computed in-degree counts
/// * `compare` - Optional comparison function for tie-breaking among nodes
///   with the same in-degree
///
/// # Errors
///
/// Returns `Err(HashSet<NodeId>)` containing all nodes that could not be
/// ordered (i.e., nodes involved in cycles).
pub fn topological_sort<F>(
    node_ids: &HashSet<NodeId>,
    adjacency: &HashMap<NodeId, Vec<NodeId>>,
    in_degree: &HashMap<NodeId, usize>,
    compare: F,
) -> Result<Vec<NodeId>, HashSet<NodeId>>
where
    F: Fn(&NodeId, &NodeId) -> std::cmp::Ordering,
{
    let mut queue = Vec::with_capacity(node_ids.len());
    let mut local_in_degree = in_degree.clone();

    // Start with all nodes that have no dependencies
    for &id in node_ids {
        if local_in_degree.get(&id).is_some_and(|&d| d == 0) {
            queue.push(id);
        }
    }
    queue.sort_by(|a, b| compare(a, b));

    let mut result = Vec::with_capacity(node_ids.len());

    while !queue.is_empty() {
        let id = queue.remove(0);
        result.push(id);

        if let Some(targets) = adjacency.get(&id) {
            for target in targets {
                if let Some(deg) = local_in_degree.get_mut(target) {
                    *deg -= 1;
                    if *deg == 0 {
                        queue.push(*target);
                    }
                }
            }
        }

        queue.sort_by(|a, b| compare(a, b));
    }

    if result.len() == node_ids.len() {
        Ok(result)
    } else {
        let result_set: HashSet<NodeId> = result.iter().copied().collect();
        let remaining: HashSet<NodeId> = node_ids.difference(&result_set).copied().collect();
        Err(remaining)
    }
}
