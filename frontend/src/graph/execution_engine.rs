//! Pure execution engine functions.
//!
//! This module implements the contract-specified pure functions for workflow execution:
//! - `prepare_execution`: Creates an execution plan from a workflow
//! - `execute_workflow`: Runs a workflow with shared context and config
//! - `execute_node`: Executes a single node
//!
//! All functions are pure (no side effects) and follow Data -> Calc -> Actions architecture.

use super::graph_ops;
use super::{Node, NodeId, Workflow, WorkflowExecutionError};
use crate::graph::execution_types::ExecutionPlan;
use std::collections::{HashMap, HashSet};

// ============================================================================
// Execution Functions (Contract Section 2)
// ============================================================================

/// Prepare an execution plan from a workflow.
///
/// **Preconditions:**
/// - P1: `workflow.nodes` is not empty
/// - P2: All node IDs are unique
/// - P3: All connection source/target references are valid nodes
///
/// **Postconditions:**
/// - R1: Returns `ExecutionPlan` with topologically sorted nodes
/// - R2: `ExecutionPlan.execution_order` contains all nodes
/// - R3: `ExecutionPlan.entry_nodes` contains all nodes with in-degree 0
/// - R4: `ExecutionPlan.cycles` contains detected cycles (empty if DAG)
/// - R5: `ExecutionPlan.input_map` maps each node to its input ports
/// - R6: If cycles detected, plan uses iterative execution strategy
///
/// **Side Effects:** None (pure function)
#[must_use]
pub fn prepare_execution(workflow: &Workflow) -> Result<ExecutionPlan, WorkflowExecutionError> {
    // Validate preconditions
    if workflow.nodes.is_empty() {
        return Err(WorkflowExecutionError::EmptyWorkflow);
    }

    // Check unique node IDs using HashSet for O(1) lookups instead of sort+dedup
    let mut seen_ids: HashSet<NodeId> = HashSet::with_capacity(workflow.nodes.len());
    for node in &workflow.nodes {
        if !seen_ids.insert(node.id) {
            return Err(WorkflowExecutionError::InvalidConfig {
                node_id: node.id,
                error: "Duplicate node IDs detected".to_string(),
            });
        }
    }

    // Build node lookup map once, used throughout (eliminates redundant HashMap build)
    let node_map = graph_ops::build_node_lookup(&workflow.nodes);

    // Validate all connection references exist
    for conn in &workflow.connections {
        if !node_map.contains_key(&conn.source) {
            return Err(WorkflowExecutionError::NodeNotFound {
                connection_id: conn.id,
                referenced_node: conn.source,
            });
        }
        if !node_map.contains_key(&conn.target) {
            return Err(WorkflowExecutionError::NodeNotFound {
                connection_id: conn.id,
                referenced_node: conn.target,
            });
        }
    }

    // Build adjacency map and calculate in-degrees in a single pass over connections
    let mut adjacency: HashMap<NodeId, Vec<NodeId>> = HashMap::new();
    let mut in_degree: HashMap<NodeId, usize> =
        node_map.keys().copied().map(|id| (id, 0)).collect();

    for conn in &workflow.connections {
        if node_map.contains_key(&conn.source) && node_map.contains_key(&conn.target) {
            adjacency.entry(conn.source).or_default().push(conn.target);
            if let Some(deg) = in_degree.get_mut(&conn.target) {
                *deg += 1;
            }
        }
    }

    // Find entry nodes (in-degree 0)
    let entry_nodes: Vec<NodeId> = in_degree
        .iter()
        .filter(|(_, &deg)| deg == 0)
        .map(|(&id, _)| id)
        .collect();

    // Topological sort using Kahn's algorithm with pre-built adjacency map
    // Optimized: adjacency map gives O(k) lookup per node instead of O(m) scan
    let mut execution_order = Vec::with_capacity(workflow.nodes.len());
    let mut queue: Vec<NodeId> = entry_nodes.clone();
    let mut local_in_degree = in_degree.clone();

    while let Some(node_id) = queue.pop() {
        execution_order.push(node_id);

        // Use pre-built adjacency map for O(k) lookup instead of O(m) scan
        if let Some(targets) = adjacency.get(&node_id) {
            for target in targets {
                if let Some(deg) = local_in_degree.get_mut(target) {
                    *deg -= 1;
                    if *deg == 0 {
                        queue.push(*target);
                    }
                }
            }
        }
    }

    // Check for cycles (nodes not in execution order) using HashSet for O(1) lookup
    let mut cycles: Vec<Vec<NodeId>> = Vec::new();
    if execution_order.len() != node_map.len() {
        let execution_set: HashSet<NodeId> = execution_order.iter().copied().collect();
        let remaining: Vec<NodeId> = node_map
            .keys()
            .filter(|id| !execution_set.contains(id))
            .copied()
            .collect();
        if !remaining.is_empty() {
            cycles.push(remaining);
        }
    }

    // Build input map by inverting the adjacency map (single pass) instead of
    // scanning all connections per node (O(n*m) -> O(n+m))
    let mut input_map: HashMap<NodeId, Vec<NodeId>> = HashMap::new();
    for (&source, targets) in &adjacency {
        for &target in targets {
            input_map.entry(target).or_default().push(source);
        }
    }
    // Ensure all nodes appear in input_map, even those with no inputs
    for &node_id in node_map.keys() {
        input_map.entry(node_id).or_default();
    }

    Ok(ExecutionPlan {
        execution_order,
        entry_nodes,
        cycles,
        input_map,
    })
}
