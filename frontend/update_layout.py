import re

content = open("src/graph/layout.rs").read()

new_layout = """use crate::graph::{NodeId, Workflow};
use petgraph::algo::toposort;
use petgraph::graph::NodeIndex;
use petgraph::Graph;
use std::collections::{HashMap, HashSet};

pub struct DagLayout {
    pub layer_spacing: f32,
    pub node_spacing: f32,
}

impl Default for DagLayout {
    fn default() -> Self {
        Self {
            layer_spacing: 140.0,
            node_spacing: 60.0,
        }
    }
}

impl DagLayout {
    pub fn apply(&self, workflow: &mut Workflow) {
        if workflow.nodes.is_empty() {
            return;
        }

        let mut graph = Graph::<NodeId, ()>::new();
        let mut index_map = HashMap::new();
        let mut reverse_map = HashMap::new();

        // Add nodes to petgraph
        for node in &workflow.nodes {
            let idx = graph.add_node(node.id);
            index_map.insert(node.id, idx);
            reverse_map.insert(idx, node.id);
        }

        // Add edges
        for conn in &workflow.connections {
            if let (Some(&src), Some(&tgt)) =
                (index_map.get(&conn.source), index_map.get(&conn.target))
            {
                graph.add_edge(src, tgt, ());
            }
        }

        // 1. Cycle Removal / Toposort
        let sorted_indices = match toposort(&graph, None) {
            Ok(indices) => indices,
            Err(_) => {
                // If cyclic, try to layout what we can or return
                return;
            }
        };

        // 2. Layer Assignment (Longest Path Layering)
        let mut layers: HashMap<NodeIndex, usize> = HashMap::new();
        for &node_idx in &sorted_indices {
            let mut layer = 0;
            for parent in graph.neighbors_directed(node_idx, petgraph::Direction::Incoming) {
                if let Some(&parent_layer) = layers.get(&parent) {
                    layer = layer.max(parent_layer + 1);
                }
            }
            layers.insert(node_idx, layer);
        }

        // Group by layer
        let mut nodes_by_layer: Vec<Vec<NodeIndex>> = Vec::new();
        for (&node_idx, &layer) in &layers {
            while nodes_by_layer.len() <= layer {
                nodes_by_layer.push(Vec::new());
            }
            nodes_by_layer[layer].push(node_idx);
        }

        // 3. Crossing minimization (Barycenter heuristic)
        for _ in 0..4 {
            for layer_idx in 1..nodes_by_layer.len() {
                let mut barycenters: Vec<(NodeIndex, f32)> = nodes_by_layer[layer_idx]
                    .iter()
                    .map(|&node| {
                        let mut sum = 0.0;
                        let mut count = 0.0;
                        for parent in graph.neighbors_directed(node, petgraph::Direction::Incoming) {
                            if let Some(pos) = nodes_by_layer[layer_idx - 1]
                                .iter()
                                .position(|&n| n == parent)
                            {
                                sum += pos as f32;
                                count += 1.0;
                            }
                        }
                        let barycenter = if count > 0.0 { sum / count } else { 0.0 };
                        (node, barycenter)
                    })
                    .collect();

                barycenters.sort_by(|a, b| a.1.partial_cmp(&b.1).unwrap());
                nodes_by_layer[layer_idx] = barycenters.into_iter().map(|(n, _)| n).collect();
            }
        }

        // 4. Coordinate Assignment
        for (layer, nodes) in nodes_by_layer.iter().enumerate() {
            let layer_width = (nodes.len() as f32) * (240.0 + self.node_spacing) - self.node_spacing;
            let start_x = -layer_width / 2.0;

            for (i, &node_idx) in nodes.iter().enumerate() {
                if let Some(node_id) = reverse_map.get(&node_idx) {
                    if let Some(node) = workflow.nodes.iter_mut().find(|n| n.id == *node_id) {
                        node.x = start_x + (i as f32) * (240.0 + self.node_spacing);
                        node.y = (layer as f32) * self.layer_spacing;
                    }
                }
            }
        }
    }
}
"""

with open("src/graph/layout.rs", "w") as f:
    f.write(new_layout)
