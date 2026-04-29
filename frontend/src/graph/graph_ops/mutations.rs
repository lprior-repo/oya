//! Graph operations requiring mutable access to node storage.

use crate::graph::{Node, NodeId};
use std::collections::HashMap;

/// Build a `NodeId -> &mut Node` lookup map from a mutable node slice.
#[must_use]
pub fn build_node_lookup_mut(nodes: &mut [Node]) -> HashMap<NodeId, &mut Node> {
    nodes.iter_mut().map(|n| (n.id, n)).collect()
}
