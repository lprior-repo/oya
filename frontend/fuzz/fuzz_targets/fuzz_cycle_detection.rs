#![no_main]
use libfuzzer_sys::fuzz_target;
use oya_frontend::graph::{NodeId, PortName, Workflow};

// Helper to create a node
fn make_node(id: NodeId, deps: Vec<NodeId>) -> oya_frontend::graph::Node {
    use oya_frontend::graph::{RunConfig, WorkflowNode};
    let mut node = oya_frontend::graph::Node::from_workflow_node(
        format!("node_{}", id.0.to_string()[..8]),
        WorkflowNode::Run(RunConfig::default()),
        0.0,
        0.0,
    );
    node.id = id;
    node.config = serde_json::json!({"dependencies": deps});
    node
}

// Deserialize a graph from bytes: [node_count, edge_list]
fn deserialize_graph(data: &[u8]) -> Option<(Vec<NodeId>, Vec<(NodeId, NodeId)>)> {
    if data.len() < 4 {
        return None;
    }

    let node_count = u32::from_le_bytes([data[0], data[1], data[2], data[3]]) as usize;
    if node_count == 0 || node_count > 1000 {
        return None;
    }

    let nodes: Vec<NodeId> = (0..node_count).map(|_| NodeId::new()).collect();

    // Create cyclic graph: force edges to create cycles
    let mut edges = Vec::new();
    let edge_start = 4;
    let edge_end = std::cmp::min(data.len(), edge_start + (node_count * 10 * 8));

    for chunk in data[edge_start..edge_end].chunks(16) {
        if chunk.len() < 16 {
            continue;
        }

        let source_idx = u32::from_le_bytes([chunk[0], chunk[1], chunk[2], chunk[3]]) as usize;
        let target_idx = u32::from_le_bytes([chunk[4], chunk[5], chunk[6], chunk[7]]) as usize;

        if source_idx < node_count && target_idx < node_count {
            // Force some cycles by making edges point backwards
            if source_idx > 0 && (source_idx % 2 == 0) {
                edges.push((nodes[source_idx], nodes[source_idx - 1]));
            } else {
                edges.push((nodes[source_idx], nodes[target_idx]));
            }
        }
    }

    // Ensure at least one cycle exists for testing
    if nodes.len() >= 3 {
        edges.push((nodes[0], nodes[1]));
        edges.push((nodes[1], nodes[2]));
        edges.push((nodes[2], nodes[0])); // Creates cycle 0->1->2->0
    }

    Some((nodes, edges))
}

fuzz_target!(|data: &[u8]| {
    // Deserialize random graph with forced cycles
    let Some((nodes, edges)) = deserialize_graph(data) else {
        return;
    };

    // Create workflow
    let mut workflow = Workflow::new();

    // Create nodes with dependencies
    for node_id in &nodes {
        let deps: Vec<NodeId> = edges
            .iter()
            .filter(|(source, _)| *source == *node_id)
            .map(|(_, target)| *target)
            .collect();
        workflow.nodes.push(make_node(*node_id, deps));
    }

    // Add connections
    for (source, target) in &edges {
        let _ = workflow.add_connection_checked(
            *source,
            *target,
            &PortName::from("main"),
            &PortName::from("main"),
        );
    }

    // Try to prepare run - MUST NOT PANIC
    // For cyclic graphs, should return CycleDetected error
    // BUG: Currently just excludes cyclic nodes silently
    workflow.prepare_run();

    // Verify: Cyclic nodes should not be silently excluded
    // Expected: Should return error with cycle information
});
