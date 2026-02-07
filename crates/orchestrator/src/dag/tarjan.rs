//! Tarjan's Strongly Connected Components (SCC) algorithm
//!
//! This module implements Tarjan's algorithm for finding SCCs in a directed graph.
//! An SCC is a maximal subgraph where every node can reach every other node.
//! SCCs are used for cycle detection and graph analysis.
//!
//! # Algorithm Overview
//!
//! Tarjan's algorithm uses DFS with two key values per node:
//! - `index`: Discovery order (0, 1, 2, ...)
//! - `low_link`: Smallest index reachable from this node
//!
//! A node is a root of an SCC when its `low_link` equals its `index`.
//!
//! # Complexity
//!
//! - Time: O(V + E) where V = vertices, E = edges
//! - Space: O(V) for the stack and index maps

use im::HashMap;
use petgraph::graph::{DiGraph, NodeIndex};
use petgraph::visit::EdgeRef;

use crate::dag::{BeadId, DependencyType, WorkflowDAG};

/// Index assigned to a node during DFS
type DiscoveryIndex = usize;

/// Low-link value for a node
type LowLink = usize;

/// Tarjan SCC algorithm state
#[derive(Debug)]
pub struct TarjanState {
    /// Next discovery index to assign
    next_index: DiscoveryIndex,
    /// Map from node index to discovery index
    indices: HashMap<NodeIndex, DiscoveryIndex>,
    /// Map from node index to low-link value
    low_links: HashMap<NodeIndex, LowLink>,
    /// Stack of nodes currently being explored
    stack: Vec<NodeIndex>,
    /// Track which nodes are on the stack
    on_stack: HashMap<NodeIndex, bool>,
}

impl TarjanState {
    /// Create a new Tarjan algorithm state
    #[must_use]
    pub fn new() -> Self {
        Self {
            next_index: 0,
            indices: HashMap::new(),
            low_links: HashMap::new(),
            stack: Vec::new(),
            on_stack: HashMap::new(),
        }
    }

    /// Get the next discovery index and increment
    fn next_index(&mut self) -> DiscoveryIndex {
        let idx = self.next_index;
        self.next_index += 1;
        idx
    }

    /// Check if a node has been visited
    fn is_visited(&self, node: NodeIndex) -> bool {
        self.indices.contains_key(&node)
    }

    /// Check if a node is on the stack
    fn is_on_stack(&self, node: NodeIndex) -> bool {
        self.on_stack.get(&node).copied().unwrap_or(false)
    }

    /// Mark a node as on the stack
    fn push_to_stack(&mut self, node: NodeIndex) {
        self.stack.push(node);
        self.on_stack.insert(node, true);
    }

    /// Pop nodes from stack until we reach the given node
    fn pop_stack_to(&mut self, node: NodeIndex) -> Vec<NodeIndex> {
        let mut scc = Vec::new();

        while let Some(top) = self.stack.pop() {
            self.on_stack.insert(top, false);
            scc.push(top);

            if top == node {
                break;
            }
        }

        scc
    }

    /// Visit a node during DFS (recursive helper)
    fn visit(
        &mut self,
        graph: &DiGraph<BeadId, DependencyType>,
        node: NodeIndex,
    ) -> Vec<Vec<NodeIndex>> {
        let mut sccs = Vec::new();

        // Assign discovery index and low-link
        let index = self.next_index();
        self.indices.insert(node, index);
        self.low_links.insert(node, index);
        self.push_to_stack(node);

        // Visit all neighbors
        for edge in graph.edges_directed(node, petgraph::Direction::Outgoing) {
            let neighbor = edge.target();

            if !self.is_visited(neighbor) {
                // Recurse on unvisited neighbor
                let neighbor_sccs = self.visit(graph, neighbor);
                sccs.extend(neighbor_sccs);

                // Update low-link if neighbor can reach smaller index
                let neighbor_low = *self.low_links.get(&neighbor).unwrap_or(&index);
                let current_low = self.low_links.get(&node).copied().unwrap_or(index);
                if neighbor_low < current_low {
                    self.low_links.insert(node, neighbor_low);
                }
            } else if self.is_on_stack(neighbor) {
                // Back edge to node on stack - update low-link
                let neighbor_index = *self.indices.get(&neighbor).unwrap_or(&index);
                let current_low = self.low_links.get(&node).copied().unwrap_or(index);
                if neighbor_index < current_low {
                    self.low_links.insert(node, neighbor_index);
                }
            }
            // Cross edges to already processed nodes are ignored
        }

        // Check if node is root of an SCC
        let current_index = *self.indices.get(&node).unwrap_or(&index);
        let current_low = self.low_links.get(&node).copied().unwrap_or(index);

        if current_low == current_index {
            // Root of SCC - pop stack to collect SCC members
            let scc_nodes = self.pop_stack_to(node);
            sccs.push(scc_nodes);
        }

        sccs
    }
}

impl Default for TarjanState {
    fn default() -> Self {
        Self::new()
    }
}

/// Find all strongly connected components using Tarjan's algorithm
///
/// # Arguments
///
/// * `dag` - The workflow DAG to analyze
///
/// # Returns
///
/// Vector of SCCs, where each SCC is a vector of BeadIds
///
/// # Examples
///
/// ```
/// use orchestrator::dag::{tarjan_scc, WorkflowDAG, DependencyType};
///
/// # fn main() -> Result<(), Box<dyn std::error::Error>> {
/// let mut dag = WorkflowDAG::new();
/// dag.add_node("a".to_string())?;
/// dag.add_node("b".to_string())?;
/// dag.add_node("c".to_string())?;
/// dag.add_edge("a".to_string(), "b".to_string(), DependencyType::BlockingDependency)?;
/// dag.add_edge("b".to_string(), "c".to_string(), DependencyType::BlockingDependency)?;
/// dag.add_edge("c".to_string(), "a".to_string(), DependencyType::BlockingDependency)?;
///
/// let sccs = tarjan_scc(&dag);
/// assert_eq!(sccs.len(), 1); // One SCC with all three nodes
/// # Ok(())
/// # }
/// ```
pub fn tarjan_scc(dag: &WorkflowDAG) -> Vec<Vec<BeadId>> {
    // Build a local graph representation from the DAG
    let node_list: Vec<BeadId> = dag.nodes().cloned().collect();
    let mut local_graph = DiGraph::new();
    let mut index_map: HashMap<BeadId, NodeIndex> = HashMap::new();

    for bead_id in &node_list {
        let idx = local_graph.add_node(bead_id.clone());
        index_map.insert(bead_id.clone(), idx);
    }

    // Add edges
    for (from, to, _dep_type) in dag.edges() {
        if let (Some(&from_idx), Some(&to_idx)) = (index_map.get(from), index_map.get(to)) {
            local_graph.add_edge(from_idx, to_idx, DependencyType::BlockingDependency);
        }
    }

    // Run Tarjan's algorithm
    let mut state = TarjanState::new();
    let mut all_sccs = Vec::new();

    // Visit all unvisited nodes
    // Collect unvisited nodes first to avoid borrow checker issues
    let unvisited: Vec<_> = local_graph
        .node_indices()
        .filter(|node| !state.is_visited(*node))
        .collect();

    // Now process each unvisited node with mutable state
    for node in unvisited {
        let sccs = state.visit(&local_graph, node);
        all_sccs.extend(sccs);
    }

    // Convert NodeIndices to BeadIds
    all_sccs
        .into_iter()
        .map(|scc_nodes| {
            scc_nodes
                .into_iter()
                .filter_map(|idx| local_graph.node_weight(idx).cloned())
                .collect()
        })
        .collect()
}

/// Find cycles in the graph using SCCs
///
/// # Arguments
///
/// * `dag` - The workflow DAG to analyze
///
/// # Returns
///
/// Vector of cycles, where each cycle is a vector of BeadIds
///
/// # Note
///
/// An SCC represents a cycle if:
/// - It has more than one node, OR
/// - It has one node with a self-loop
///
/// # Examples
///
/// ```
/// use orchestrator::dag::{find_cycles_tarjan, WorkflowDAG, DependencyType};
///
/// # fn main() -> Result<(), Box<dyn std::error::Error>> {
/// let mut dag = WorkflowDAG::new();
/// dag.add_node("a".to_string())?;
/// dag.add_node("b".to_string())?;
/// dag.add_edge("a".to_string(), "b".to_string(), DependencyType::BlockingDependency)?;
/// dag.add_edge("b".to_string(), "a".to_string(), DependencyType::BlockingDependency)?;
///
/// let cycles = find_cycles_tarjan(&dag);
/// assert_eq!(cycles.len(), 1);
/// # Ok(())
/// # }
/// ```
pub fn find_cycles_tarjan(dag: &WorkflowDAG) -> Vec<Vec<BeadId>> {
    let sccs = tarjan_scc(dag);

    sccs.into_iter()
        .filter(|scc| {
            // SCC is a cycle if it has multiple nodes
            if scc.len() > 1 {
                return true;
            }

            // OR single node with self-loop
            if scc.len() == 1 {
                let bead_id = &scc[0];

                // Check if there's a self-loop in the DAG
                return dag
                    .edges()
                    .any(|(from, to, _dep_type)| from == to && from == bead_id);
            }

            false
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use im::HashSet;

    // Helper to create a DAG from edges
    fn dag_from_edges(edges: &[(BeadId, BeadId)]) -> WorkflowDAG {
        let mut dag = WorkflowDAG::new();

        // Add all unique nodes
        let mut nodes: HashSet<BeadId> = HashSet::new();
        for (from, to) in edges {
            nodes.insert(from.clone());
            nodes.insert(to.clone());
        }

        for node in nodes {
            let _ = dag.add_node(node);
        }

        // Add all edges
        for (from, to) in edges {
            let _ = dag.add_edge(from.clone(), to.clone(), DependencyType::BlockingDependency);
        }

        dag
    }

    #[test]
    fn test_tarjan_scc_empty_graph() {
        let dag = WorkflowDAG::new();
        let sccs = tarjan_scc(&dag);
        assert_eq!(sccs.len(), 0);
    }

    #[test]
    fn test_tarjan_scc_single_node() {
        let mut dag = WorkflowDAG::new();
        let _ = dag.add_node("a".to_string());

        let sccs = tarjan_scc(&dag);

        // Single node forms its own SCC
        assert_eq!(sccs.len(), 1);
        assert_eq!(sccs[0].len(), 1);
        assert!(sccs[0].contains(&"a".to_string()));
    }

    #[test]
    fn test_tarjan_scc_two_node_cycle() {
        let dag = dag_from_edges(&[
            ("a".to_string(), "b".to_string()),
            ("b".to_string(), "a".to_string()),
        ]);

        let sccs = tarjan_scc(&dag);

        // Both nodes in one SCC
        assert_eq!(sccs.len(), 1);
        assert_eq!(sccs[0].len(), 2);
        assert!(sccs[0].contains(&"a".to_string()));
        assert!(sccs[0].contains(&"b".to_string()));
    }

    #[test]
    fn test_tarjan_scc_three_node_cycle() {
        let dag = dag_from_edges(&[
            ("a".to_string(), "b".to_string()),
            ("b".to_string(), "c".to_string()),
            ("c".to_string(), "a".to_string()),
        ]);

        let sccs = tarjan_scc(&dag);

        // All three nodes in one SCC
        assert_eq!(sccs.len(), 1);
        assert_eq!(sccs[0].len(), 3);
        assert!(sccs[0].contains(&"a".to_string()));
        assert!(sccs[0].contains(&"b".to_string()));
        assert!(sccs[0].contains(&"c".to_string()));
    }

    #[test]
    fn test_tarjan_scc_dag_returns_singletons() {
        // A DAG should return each node as its own SCC
        let dag = dag_from_edges(&[
            ("a".to_string(), "b".to_string()),
            ("b".to_string(), "c".to_string()),
            ("c".to_string(), "d".to_string()),
        ]);

        let sccs = tarjan_scc(&dag);

        // Each node is its own SCC
        assert_eq!(sccs.len(), 4);

        // Each SCC should have exactly one node
        for scc in &sccs {
            assert_eq!(scc.len(), 1);
        }

        // All nodes should be present
        let all_nodes: Vec<&BeadId> = sccs.iter().flat_map(|scc| scc.iter()).collect();
        assert_eq!(all_nodes.len(), 4);
    }

    #[test]
    fn test_find_cycles_empty_graph() {
        let dag = WorkflowDAG::new();
        let cycles = find_cycles_tarjan(&dag);
        assert_eq!(cycles.len(), 0);
    }

    #[test]
    fn test_find_cycles_dag_no_cycles() {
        let dag = dag_from_edges(&[
            ("a".to_string(), "b".to_string()),
            ("b".to_string(), "c".to_string()),
            ("c".to_string(), "d".to_string()),
        ]);

        let cycles = find_cycles_tarjan(&dag);

        // DAG has no cycles (singletons are not cycles)
        assert_eq!(cycles.len(), 0);
    }

    #[test]
    fn test_find_cycles_two_node_cycle() {
        let dag = dag_from_edges(&[
            ("a".to_string(), "b".to_string()),
            ("b".to_string(), "a".to_string()),
        ]);

        let cycles = find_cycles_tarjan(&dag);

        assert_eq!(cycles.len(), 1);
        assert_eq!(cycles[0].len(), 2);
    }

    #[test]
    fn test_find_cycles_three_node_cycle() {
        let dag = dag_from_edges(&[
            ("a".to_string(), "b".to_string()),
            ("b".to_string(), "c".to_string()),
            ("c".to_string(), "a".to_string()),
        ]);

        let cycles = find_cycles_tarjan(&dag);

        assert_eq!(cycles.len(), 1);
        assert_eq!(cycles[0].len(), 3);
    }

    #[test]
    fn test_tarjan_all_nodes_visited() {
        // Ensure all nodes are visited exactly once (invariant check)
        let dag = dag_from_edges(&[
            ("a".to_string(), "b".to_string()),
            ("b".to_string(), "c".to_string()),
            ("c".to_string(), "a".to_string()),
            ("d".to_string(), "e".to_string()),
        ]);

        let sccs = tarjan_scc(&dag);

        // Count total nodes in SCCs
        let total_nodes: usize = sccs.iter().map(|scc| scc.len()).sum();

        // Should equal graph node count
        assert_eq!(total_nodes, dag.node_count());

        // Each node should appear exactly once
        let mut all_nodes = Vec::new();
        for scc in &sccs {
            all_nodes.extend(scc.clone());
        }

        let mut sorted_nodes = all_nodes.clone();
        sorted_nodes.sort();
        sorted_nodes.dedup();

        assert_eq!(all_nodes.len(), sorted_nodes.len());
    }

    #[test]
    fn test_tarjan_sccs_are_maximal() {
        // Ensure SCCs are maximal (can't add any more nodes)
        let dag = dag_from_edges(&[
            ("a".to_string(), "b".to_string()),
            ("b".to_string(), "c".to_string()),
            ("c".to_string(), "a".to_string()),
            ("d".to_string(), "a".to_string()),
        ]);

        let sccs = tarjan_scc(&dag);

        // Should have 2 SCCs: {a,b,c} and {d}
        assert_eq!(sccs.len(), 2);

        // Find the 3-node SCC
        let large_scc = sccs.iter().find(|scc| scc.len() == 3).unwrap();

        // Verify it's maximal: all nodes can reach each other
        // a <-> b <-> c <-> a
        assert!(large_scc.contains(&"a".to_string()));
        assert!(large_scc.contains(&"b".to_string()));
        assert!(large_scc.contains(&"c".to_string()));
    }
}
