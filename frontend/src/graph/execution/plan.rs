use super::super::graph_ops;
use super::super::NodeId;
use super::super::Workflow;
use super::super::WorkflowExecutionError;

use std::cmp::Ordering;
use std::collections::HashMap;
use std::collections::HashSet;

// ===========================================================================
// Execution Plan Preparation (Topological Sort & Validation)
// ===========================================================================

impl Workflow {
    /// Compare two node IDs by execution priority (x, then y, then name).
    ///
    /// Accepts a pre-built lookup map to avoid repeated O(n) linear scans
    /// during sorting, turning each comparison from O(n) to O(1).
    pub(super) fn compare_execution_priority(
        node_map: &HashMap<NodeId, &super::super::Node>,
        a: NodeId,
        b: NodeId,
    ) -> Ordering {
        let node_a = node_map.get(&a);
        let node_b = node_map.get(&b);

        match (node_a, node_b) {
            (Some(left), Some(right)) => left
                .x
                .partial_cmp(&right.x)
                .unwrap_or(Ordering::Equal)
                .then_with(|| left.y.partial_cmp(&right.y).unwrap_or(Ordering::Equal))
                .then_with(|| left.name.cmp(&right.name)),
            _ => Ordering::Equal,
        }
    }

    /// Checks if all nodes are connected (no isolated subgraphs).
    ///
    /// This ensures graph connectivity invariant - all nodes must be
    /// reachable from at least one entry node.
    ///
    /// Optimized to build adjacency maps once (O(n+m)) instead of scanning
    /// connections per node (O(n*m)).
    pub(super) fn verify_graph_connectivity(&self) -> Result<(), WorkflowExecutionError> {
        if self.nodes.is_empty() {
            return Ok(());
        }

        let node_ids = graph_ops::collect_node_ids(&self.nodes);
        let outgoing = graph_ops::build_outgoing_adjacency(&self.connections, &node_ids);
        let (has_incoming, _) = graph_ops::build_connection_membership(&self.connections);

        // Find a starting node (any node with no incoming edges, or first node)
        let entry_node =
            self.nodes.iter().find(|node| !has_incoming.contains(&node.id)).map(|node| node.id);

        let start_node = if let Some(node) = entry_node {
            node
        } else if let Some(first_node) = self.nodes.first() {
            first_node.id
        } else {
            return Err(WorkflowExecutionError::InvalidWorkflowState {
                reason: "verify_graph_connectivity: graph should have at least one node"
                    .to_string(),
            });
        };

        let visited = graph_ops::find_reachable(&[start_node], &outgoing);

        if visited.len() != node_ids.len() {
            let unreachable: Vec<NodeId> = node_ids.difference(&visited).copied().collect();

            if !unreachable.is_empty() {
                return Err(WorkflowExecutionError::InvalidWorkflowState {
                    reason: format!(
                        "graph connectivity violation: isolated subgraph detected with {} unreachable nodes: {:?}",
                        unreachable.len(),
                        unreachable
                    ),
                });
            }
        }

        Ok(())
    }

    /// Validates that all dependencies reference existing nodes.
    pub(super) fn validate_dependencies_exist(&self) -> Result<(), WorkflowExecutionError> {
        let node_ids = graph_ops::collect_node_ids(&self.nodes);

        let mut missing_deps: Vec<(NodeId, NodeId)> = Vec::new();

        for conn in &self.connections {
            if !node_ids.contains(&conn.source) {
                continue;
            }
            if !node_ids.contains(&conn.target) {
                missing_deps.push((conn.source, conn.target));
            }
        }

        if !missing_deps.is_empty() {
            // Group by source node
            let mut nodes_with_missing: HashMap<NodeId, Vec<NodeId>> = HashMap::new();

            for (source, missing) in &missing_deps {
                nodes_with_missing.entry(*source).or_default().push(*missing);
            }

            let nodes: Vec<NodeId> = nodes_with_missing.keys().copied().collect();
            let all_missing: Vec<NodeId> = missing_deps.iter().map(|(_, m)| *m).collect();

            return Err(WorkflowExecutionError::UnresolvedDependencies {
                nodes,
                missing_deps: all_missing,
            });
        }

        Ok(())
    }

    /// Checks for duplicate connections in the connection list.
    pub(super) fn check_duplicate_connections(&self) -> Result<(), WorkflowExecutionError> {
        let mut seen: HashMap<(NodeId, NodeId), Vec<uuid::Uuid>> = HashMap::new();

        for conn in &self.connections {
            let key = (conn.source, conn.target);
            seen.entry(key).or_default().push(conn.id);
        }

        for ((source, target), ids) in &seen {
            if ids.len() > 1 {
                return Err(WorkflowExecutionError::InvalidWorkflowState {
                    reason: format!(
                        "duplicate connection from node {} to node {} ({} connections)",
                        source,
                        target,
                        ids.len()
                    ),
                });
            }
        }

        Ok(())
    }

    /// Returns error if workflow has dirty state.
    pub(super) fn check_dirty_state(&self) -> Result<(), WorkflowExecutionError> {
        if !self.execution_queue.is_empty() {
            return Err(WorkflowExecutionError::InvalidWorkflowState {
                reason: "execution_queue is not empty".to_string(),
            });
        }

        // Check if any node is executing (dirty executed state)
        if self.nodes.iter().any(|n| n.executing) {
            return Err(WorkflowExecutionError::InvalidWorkflowState {
                reason: "some nodes are in executing state".to_string(),
            });
        }

        Ok(())
    }

    /// Returns error if workflow is empty.
    pub(super) const fn check_non_empty(&self) -> Result<(), WorkflowExecutionError> {
        if self.nodes.is_empty() {
            return Err(WorkflowExecutionError::EmptyWorkflow);
        }
        Ok(())
    }

    /// Returns error if self-references detected.
    pub(super) fn check_self_references(&self) -> Result<(), WorkflowExecutionError> {
        for conn in &self.connections {
            if conn.source == conn.target {
                return Err(WorkflowExecutionError::CycleDetected {
                    cycle_nodes: vec![conn.source],
                });
            }
        }
        Ok(())
    }

    /// Uses Kahn's algorithm to build the execution queue.
    ///
    /// Returns a vector of node IDs in topological order.
    ///
    /// Optimized to pre-build an adjacency map (source -> targets) and a node
    /// lookup `HashMap` so that each comparison during sort is O(1) instead of O(n),
    /// and finding dependents is O(k) instead of O(m).
    pub(super) fn build_execution_queue(&self) -> Result<Vec<NodeId>, WorkflowExecutionError> {
        let node_ids = graph_ops::collect_node_ids(&self.nodes);

        // Pre-build node lookup for O(1) comparisons during sort
        let node_map = graph_ops::build_node_lookup(&self.nodes);

        // Pre-build adjacency map and in-degrees in a single pass
        let (adjacency, in_degree) =
            graph_ops::build_adjacency_with_in_degree(&self.connections, &node_ids);

        // Delegate to shared topological sort
        graph_ops::topological_sort(&node_ids, &adjacency, &in_degree, |a, b| {
            Self::compare_execution_priority(&node_map, *a, *b)
        })
        .map_err(|remaining| WorkflowExecutionError::CycleDetected {
            cycle_nodes: remaining.into_iter().collect(),
        })
    }

    /// DFS-based cycle detection that returns actual cycle nodes.
    ///
    /// Uses a pre-built reverse adjacency map for O(1) neighbor lookup
    /// instead of scanning all connections per DFS call.
    fn detect_cycle_dfs(
        reverse_adj: &HashMap<NodeId, Vec<NodeId>>,
        start_node: NodeId,
        visited: &mut HashSet<NodeId>,
        path_set: &mut HashSet<NodeId>,
        path: &mut Vec<NodeId>,
    ) -> Option<Vec<NodeId>> {
        visited.insert(start_node);
        path.push(start_node);
        path_set.insert(start_node);

        // Use pre-built reverse adjacency for O(1) lookup instead of O(m) scan
        let dependents =
            reverse_adj.get(&start_node).map(std::vec::Vec::as_slice).unwrap_or_default();

        for &dependent in dependents {
            // Check if dependent is in current path (back edge = cycle)
            if path_set.contains(&dependent) {
                // Found a cycle - extract it
                let cycle_start = path.iter().position(|&n| n == dependent)?;
                let cycle: Vec<NodeId> = path[cycle_start..].to_vec();
                return Some(cycle);
            }
            // Recurse if not yet visited
            if !visited.contains(&dependent) {
                if let Some(cycle) =
                    Self::detect_cycle_dfs(reverse_adj, dependent, visited, path_set, path)
                {
                    return Some(cycle);
                }
            }
            // If node is visited but not in current path, skip it
            // (it was already explored in a previous branch)
        }

        path.pop();
        path_set.remove(&start_node);
        None
    }

    /// Finds a cycle in the workflow graph using DFS.
    ///
    /// Returns a vector of node IDs that form the cycle if one exists.
    /// The cycle nodes are those that are part of the cycle.
    ///
    /// Builds a reverse adjacency map once for O(n+m) complexity instead
    /// of O(n*m) from scanning connections on each DFS step.
    pub(super) fn find_cycle(&self) -> Option<Vec<NodeId>> {
        let node_ids = graph_ops::collect_node_ids(&self.nodes);
        let reverse_adj = graph_ops::build_reverse_adjacency(&self.connections, &node_ids);

        let mut visited = HashSet::new();
        let mut path = Vec::new();
        let mut path_set = HashSet::new();

        for node in &self.nodes {
            if !visited.contains(&node.id) {
                if let Some(cycle) = Self::detect_cycle_dfs(
                    &reverse_adj,
                    node.id,
                    &mut visited,
                    &mut path_set,
                    &mut path,
                ) {
                    return Some(cycle);
                }
            }
        }

        None
    }
}
