//! Structural validations for workflows.

use crate::graph::graph_ops;
use crate::graph::{NodeCategory, NodeId, ValidationIssue, Workflow};

use std::collections::HashSet;

// ===========================================================================
// Structural Validations (Reachability, Entry Points, Orphans)
// ===========================================================================

pub fn validate_entry_points(workflow: &Workflow, issues: &mut Vec<ValidationIssue>) {
    if !workflow.nodes.iter().any(|n| n.category == NodeCategory::Entry) {
        issues.push(ValidationIssue::error(
            "Workflow has no entry point (e.g., HTTP Handler, Kafka Handler)".to_string(),
        ));
    }
}

pub fn validate_reachability(workflow: &Workflow, issues: &mut Vec<ValidationIssue>) {
    if workflow.nodes.is_empty() || workflow.connections.is_empty() {
        return;
    }

    let entry_ids: HashSet<NodeId> =
        workflow.nodes.iter().filter(|n| n.category == NodeCategory::Entry).map(|n| n.id).collect();

    if entry_ids.is_empty() {
        return;
    }

    let node_ids = graph_ops::collect_node_ids(&workflow.nodes);
    let outgoing = graph_ops::build_outgoing_adjacency(&workflow.connections, &node_ids);
    let start_ids: Vec<NodeId> = entry_ids.iter().copied().collect();
    let reachable = graph_ops::find_reachable(&start_ids, &outgoing);

    for node in &workflow.nodes {
        if !reachable.contains(&node.id) && node.category != NodeCategory::Entry {
            issues.push(ValidationIssue::warning_for_node(
                format!("Node '{}' is not reachable from any entry point", node.name),
                node.id,
            ));
        }
    }
}

/// Single-pass connection scan: build incoming/outgoing sets once (O(n+m))
/// instead of scanning all connections per node (O(n*m)).
pub fn validate_orphan_nodes(workflow: &Workflow, issues: &mut Vec<ValidationIssue>) {
    let (has_incoming, has_outgoing) =
        graph_ops::build_connection_membership(&workflow.connections);

    for node in &workflow.nodes {
        if node.category == NodeCategory::Entry {
            continue;
        }

        let incoming = has_incoming.contains(&node.id);
        let outgoing = has_outgoing.contains(&node.id);

        if !incoming && !outgoing && workflow.nodes.len() > 1 {
            issues.push(ValidationIssue::warning_for_node(
                format!("Node '{}' is not connected to anything", node.name),
                node.id,
            ));
        } else if !incoming && workflow.nodes.len() > 1 {
            issues.push(ValidationIssue::warning_for_node(
                format!("Node '{}' has no incoming connections", node.name),
                node.id,
            ));
        }
    }
}
