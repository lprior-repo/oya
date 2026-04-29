// Sugiyama layered graph layout algorithm for DAG visualization
// Port this directly to Rust for Dioxus
//
// Algorithm steps:
// 1. Layer assignment (longest path layering)
// 2. Crossing minimization (barycenter heuristic)
// 3. Position assignment (Brandes-Köpf)
// 4. Edge routing (orthogonal or Bezier)

use std::collections::{HashMap, HashSet, VecDeque};

#[derive(Debug, Clone)]
pub struct Position {
    pub x: f64,
    pub y: f64,
}

#[derive(Debug, Clone)]
pub struct Node {
    pub id: String,
    pub width: f64,
    pub height: f64,
}

#[derive(Debug, Clone)]
pub struct Edge {
    pub source: String,
    pub target: String,
}

pub struct LayeredLayout {
    pub node_spacing: f64,  // horizontal spacing between nodes
    pub layer_spacing: f64, // vertical spacing between layers
}

impl Default for LayeredLayout {
    fn default() -> Self {
        Self {
            node_spacing: 60.0,
            layer_spacing: 140.0,
        }
    }
}

impl LayeredLayout {
    pub fn layout(&self, nodes: &[Node], edges: &[Edge]) -> HashMap<String, Position> {
        if nodes.is_empty() {
            return HashMap::new();
        }

        // Build adjacency lists
        let mut adj: HashMap<String, Vec<String>> = HashMap::new();
        let mut in_degree: HashMap<String, usize> = HashMap::new();

        for node in nodes {
            adj.entry(node.id.clone()).or_insert_with(Vec::new);
            in_degree.entry(node.id.clone()).or_insert(0);
        }

        for edge in edges {
            adj.entry(edge.source.clone())
                .or_insert_with(Vec::new)
                .push(edge.target.clone());
            *in_degree.entry(edge.target.clone()).or_insert(0) += 1;
        }

        // Step 1: Layer assignment (topological sort + longest path)
        let layers = self.assign_layers(nodes, &adj, &in_degree);

        // Step 2: Crossing minimization (simplified barycenter)
        let ordered_layers = self.minimize_crossings(&layers, edges);

        // Step 3: Position assignment
        self.assign_positions(nodes, &ordered_layers)
    }

    fn assign_layers(
        &self,
        nodes: &[Node],
        adj: &HashMap<String, Vec<String>>,
        in_degree: &HashMap<String, usize>,
    ) -> Vec<Vec<String>> {
        let mut layers: Vec<Vec<String>> = Vec::new();
        let mut node_layer: HashMap<String, usize> = HashMap::new();
        let mut queue: VecDeque<String> = VecDeque::new();
        let mut degree = in_degree.clone();

        // Find root nodes (in_degree = 0)
        for (node, &deg) in &degree {
            if deg == 0 {
                queue.push_back(node.clone());
            }
        }

        // If no roots, just pick first node (cyclic or disconnected)
        if queue.is_empty() && !nodes.is_empty() {
            queue.push_back(nodes[0].id.clone());
        }

        // Topological sort with layer tracking
        while let Some(node) = queue.pop_front() {
            let layer = node_layer.get(&node).copied().map_or(0, |layer| layer);

            while layers.len() <= layer {
                layers.push(Vec::new());
            }
            layers[layer].push(node.clone());

            if let Some(neighbors) = adj.get(&node) {
                for neighbor in neighbors {
                    let neighbor_layer = layer + 1;
                    node_layer
                        .entry(neighbor.clone())
                        .and_modify(|l| *l = (*l).max(neighbor_layer))
                        .or_insert(neighbor_layer);

                    if let Some(d) = degree.get_mut(neighbor) {
                        *d = d.saturating_sub(1);
                        if *d == 0 {
                            queue.push_back(neighbor.clone());
                        }
                    }
                }
            }
        }

        // Add any orphaned nodes to final layer
        let all_layered: HashSet<String> = layers.iter().flat_map(|l| l.iter()).cloned().collect();
        let orphans: Vec<String> = nodes
            .iter()
            .map(|n| n.id.clone())
            .filter(|id| !all_layered.contains(id))
            .collect();

        if !orphans.is_empty() {
            layers.push(orphans);
        }

        layers
    }

    fn minimize_crossings(&self, layers: &[Vec<String>], edges: &[Edge]) -> Vec<Vec<String>> {
        // Simplified barycenter heuristic
        // For each layer, order nodes by average position of neighbors in previous layer
        let mut ordered = layers.to_vec();

        // Build edge map
        let mut edge_map: HashMap<String, Vec<String>> = HashMap::new();
        for edge in edges {
            edge_map
                .entry(edge.source.clone())
                .or_insert_with(Vec::new)
                .push(edge.target.clone());
        }

        // Multiple passes for better results
        for _ in 0..4 {
            for layer_idx in 1..ordered.len() {
                let prev_layer = &ordered[layer_idx - 1];
                let curr_layer = &ordered[layer_idx];

                // Calculate barycenter for each node
                let mut barycenters: Vec<(String, f64)> = curr_layer
                    .iter()
                    .map(|node| {
                        let positions: Vec<f64> = prev_layer
                            .iter()
                            .enumerate()
                            .filter(|(_, prev_node)| {
                                edge_map
                                    .get(*prev_node)
                                    .map(|targets| targets.contains(node))
                                    .is_some_and(|contains| contains)
                            })
                            .map(|(idx, _)| idx as f64)
                            .collect();

                        let barycenter = if positions.is_empty() {
                            0.0
                        } else {
                            positions.iter().sum::<f64>() / positions.len() as f64
                        };

                        (node.clone(), barycenter)
                    })
                    .collect();

                // Sort by barycenter
                barycenters.sort_by(|a, b| match a.1.partial_cmp(&b.1) {
                    Some(ordering) => ordering,
                    None => std::cmp::Ordering::Equal,
                });
                ordered[layer_idx] = barycenters.into_iter().map(|(id, _)| id).collect();
            }
        }

        ordered
    }

    fn assign_positions(
        &self,
        nodes: &[Node],
        layers: &[Vec<String>],
    ) -> HashMap<String, Position> {
        let mut positions = HashMap::new();
        let node_map: HashMap<String, &Node> = nodes.iter().map(|n| (n.id.clone(), n)).collect();

        for (layer_idx, layer) in layers.iter().enumerate() {
            let layer_width = layer.len() as f64 * (240.0 + self.node_spacing);
            let start_x = -layer_width / 2.0;

            for (node_idx, node_id) in layer.iter().enumerate() {
                let x = start_x + node_idx as f64 * (240.0 + self.node_spacing);
                let y = layer_idx as f64 * self.layer_spacing;

                positions.insert(node_id.clone(), Position { x, y });
            }
        }

        positions
    }
}

// Simpler force-directed layout for comparison
pub struct ForceDirectedLayout {
    pub iterations: usize,
    pub spring_length: f64,
    pub spring_strength: f64,
    pub repulsion_strength: f64,
}

impl Default for ForceDirectedLayout {
    fn default() -> Self {
        Self {
            iterations: 50,
            spring_length: 200.0,
            spring_strength: 0.1,
            repulsion_strength: 3000.0,
        }
    }
}

impl ForceDirectedLayout {
    pub fn layout(
        &self,
        nodes: &[Node],
        edges: &[Edge],
        initial: &HashMap<String, Position>,
    ) -> HashMap<String, Position> {
        let mut positions = initial.clone();
        let mut velocities: HashMap<String, Position> = HashMap::new();

        // Initialize missing positions
        for node in nodes {
            positions
                .entry(node.id.clone())
                .or_insert(Position { x: 0.0, y: 0.0 });
            velocities.insert(node.id.clone(), Position { x: 0.0, y: 0.0 });
        }

        for _ in 0..self.iterations {
            let mut forces: HashMap<String, Position> = HashMap::new();

            // Repulsion between all nodes
            for i in 0..nodes.len() {
                for j in (i + 1)..nodes.len() {
                    let id1 = &nodes[i].id;
                    let id2 = &nodes[j].id;

                    if let (Some(p1), Some(p2)) = (positions.get(id1), positions.get(id2)) {
                        let dx = p2.x - p1.x;
                        let dy = p2.y - p1.y;
                        let dist = (dx * dx + dy * dy).sqrt().max(1.0);
                        let force = self.repulsion_strength / (dist * dist);

                        let fx = (dx / dist) * force;
                        let fy = (dy / dist) * force;

                        forces
                            .entry(id1.clone())
                            .or_insert(Position { x: 0.0, y: 0.0 })
                            .x -= fx;
                        forces
                            .entry(id1.clone())
                            .or_insert(Position { x: 0.0, y: 0.0 })
                            .y -= fy;
                        forces
                            .entry(id2.clone())
                            .or_insert(Position { x: 0.0, y: 0.0 })
                            .x += fx;
                        forces
                            .entry(id2.clone())
                            .or_insert(Position { x: 0.0, y: 0.0 })
                            .y += fy;
                    }
                }
            }

            // Spring attraction for edges
            for edge in edges {
                if let (Some(p1), Some(p2)) =
                    (positions.get(&edge.source), positions.get(&edge.target))
                {
                    let dx = p2.x - p1.x;
                    let dy = p2.y - p1.y;
                    let dist = (dx * dx + dy * dy).sqrt().max(1.0);
                    let force = self.spring_strength * (dist - self.spring_length);

                    let fx = (dx / dist) * force;
                    let fy = (dy / dist) * force;

                    forces
                        .entry(edge.source.clone())
                        .or_insert(Position { x: 0.0, y: 0.0 })
                        .x += fx;
                    forces
                        .entry(edge.source.clone())
                        .or_insert(Position { x: 0.0, y: 0.0 })
                        .y += fy;
                    forces
                        .entry(edge.target.clone())
                        .or_insert(Position { x: 0.0, y: 0.0 })
                        .x -= fx;
                    forces
                        .entry(edge.target.clone())
                        .or_insert(Position { x: 0.0, y: 0.0 })
                        .y -= fy;
                }
            }

            // Apply forces with damping
            let damping = 0.8;
            for node in nodes {
                if let Some(force) = forces.get(&node.id) {
                    let Some(vel) = velocities.get_mut(&node.id) else {
                        continue;
                    };
                    vel.x = (vel.x + force.x) * damping;
                    vel.y = (vel.y + force.y) * damping;

                    let Some(pos) = positions.get_mut(&node.id) else {
                        continue;
                    };
                    pos.x += vel.x;
                    pos.y += vel.y;
                }
            }
        }

        positions
    }
}
