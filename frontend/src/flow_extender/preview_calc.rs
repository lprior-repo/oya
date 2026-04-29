//! Pure functions for computing preview nodes and edges from extension patches.
//!
//! These are extracted from memo computations in main.rs to enable unit testing
//! and reuse without Dioxus signal dependencies.

use crate::flow_extender::{
    ExtensionPatchPreview, PreviewConnection, PreviewEndpoint, PreviewNode,
};
use crate::graph::{Node, NodeId};
use std::collections::HashMap;
use std::fmt::Write;

/// A resolved preview node: (key, node_type, x, y).
pub type PreviewNodeEntry = (String, String, f32, f32);

/// A resolved preview edge: (edge_key, svg_path).
pub type PreviewEdgeEntry = (String, String);

/// Pure function: compute preview nodes from extension patches.
///
/// Iterates over all patches and their proposed nodes, producing a flat list
/// of `(key, node_type, x, y)` tuples where key is `"p{patch_idx}-{temp_id}"`.
pub fn compute_preview_nodes(patches: &[ExtensionPatchPreview]) -> Vec<PreviewNodeEntry> {
    patches
        .iter()
        .enumerate()
        .flat_map(|(patch_idx, patch)| {
            patch.nodes.iter().map(move |node| {
                let mut key = String::with_capacity(32);
                let _ = write!(key, "p{patch_idx}-{}", node.temp_id);
                (key, node.node_type.clone(), node.x, node.y)
            })
        })
        .collect()
}

/// Pure function: compute preview edges from extension patches and existing nodes.
///
/// For each patch, builds a lookup of proposed node positions, then resolves
/// source/target positions for each connection, producing SVG cubic bezier path
/// strings with keys `"p{patch_idx}-e{edge_idx}"`.
pub fn compute_preview_edges(
    patches: &[ExtensionPatchPreview],
    existing_nodes: &HashMap<NodeId, Node>,
) -> Vec<PreviewEdgeEntry> {
    patches
        .iter()
        .enumerate()
        .flat_map(|(patch_idx, patch)| {
            let proposed_lookup = build_proposed_lookup(patch_idx, &patch.nodes);
            let existing_nodes = existing_nodes.clone();

            patch.connections.iter().enumerate().filter_map(move |(edge_idx, edge)| {
                compute_single_edge(patch_idx, edge_idx, edge, &existing_nodes, &proposed_lookup)
            })
        })
        .collect()
}

/// Build the proposed node position lookup for a given patch index.
fn build_proposed_lookup(patch_idx: usize, nodes: &[PreviewNode]) -> HashMap<String, (f32, f32)> {
    nodes
        .iter()
        .map(|node| {
            let mut key = String::with_capacity(32);
            let _ = write!(key, "p{patch_idx}-{}", node.temp_id);
            (key, (node.x, node.y))
        })
        .collect()
}

/// Compute a single preview edge, resolving endpoints to pixel positions.
fn compute_single_edge(
    patch_idx: usize,
    edge_idx: usize,
    edge: &PreviewConnection,
    existing_nodes: &HashMap<NodeId, Node>,
    proposed_lookup: &HashMap<String, (f32, f32)>,
) -> Option<PreviewEdgeEntry> {
    let source = resolve_source_position(&edge.source, patch_idx, existing_nodes, proposed_lookup)?;
    let target = resolve_target_position(&edge.target, patch_idx, existing_nodes, proposed_lookup)?;

    let mut edge_key = String::with_capacity(32);
    let _ = write!(edge_key, "p{patch_idx}-e{edge_idx}");

    let (sx, sy) = source;
    let (tx, ty) = target;

    let mut svg_path = String::with_capacity(80);
    let _ = write!(
        svg_path,
        "M {} {} C {} {}, {} {}, {} {}",
        sx,
        sy,
        f32::midpoint(sx, tx),
        sy,
        f32::midpoint(sx, tx),
        ty,
        tx,
        ty
    );

    Some((edge_key, svg_path))
}

/// Resolve the source endpoint to a position (x + 220.0, y + 34.0).
fn resolve_source_position(
    endpoint: &PreviewEndpoint,
    patch_idx: usize,
    existing_nodes: &HashMap<NodeId, Node>,
    proposed_lookup: &HashMap<String, (f32, f32)>,
) -> Option<(f32, f32)> {
    match endpoint {
        PreviewEndpoint::Existing(node_id) => {
            existing_nodes.get(node_id).map(|node| (node.x + 220.0, node.y + 34.0))
        }
        PreviewEndpoint::Proposed(temp_id) => {
            let mut key = String::with_capacity(32);
            let _ = write!(key, "p{patch_idx}-{temp_id}");
            proposed_lookup.get(&key).copied().map(|(x, y)| (x + 220.0, y + 34.0))
        }
    }
}

/// Resolve the target endpoint to a position (x, y + 34.0).
fn resolve_target_position(
    endpoint: &PreviewEndpoint,
    patch_idx: usize,
    existing_nodes: &HashMap<NodeId, Node>,
    proposed_lookup: &HashMap<String, (f32, f32)>,
) -> Option<(f32, f32)> {
    match endpoint {
        PreviewEndpoint::Existing(node_id) => {
            existing_nodes.get(node_id).map(|node| (node.x, node.y + 34.0))
        }
        PreviewEndpoint::Proposed(temp_id) => {
            let mut key = String::with_capacity(32);
            let _ = write!(key, "p{patch_idx}-{temp_id}");
            proposed_lookup.get(&key).copied().map(|(x, y)| (x, y + 34.0))
        }
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used, clippy::panic, clippy::float_cmp)]
mod tests {
    use super::*;
    use crate::flow_extender::{
        ExtensionPatchPreview, PreviewConnection, PreviewEndpoint, PreviewNode,
    };
    use crate::graph::Node;
    use std::collections::HashMap;

    fn sample_node(id: NodeId, x: f32, y: f32) -> Node {
        Node { id, name: "test-node".to_string(), x, y, ..Default::default() }
    }

    #[test]
    fn empty_patches_produce_no_preview_nodes() {
        let patches: Vec<ExtensionPatchPreview> = vec![];
        let result = compute_preview_nodes(&patches);
        assert!(result.is_empty());
    }

    #[test]
    fn empty_patches_produce_no_preview_edges() {
        let patches: Vec<ExtensionPatchPreview> = vec![];
        let existing = HashMap::new();
        let result = compute_preview_edges(&patches, &existing);
        assert!(result.is_empty());
    }

    #[test]
    fn single_patch_with_one_node_produces_one_preview_node() {
        let patches = vec![ExtensionPatchPreview {
            key: "test-ext".to_string(),
            nodes: vec![PreviewNode {
                temp_id: "n1".to_string(),
                node_type: "handler".to_string(),
                x: 100.0,
                y: 200.0,
            }],
            connections: vec![],
        }];

        let result = compute_preview_nodes(&patches);
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].0, "p0-n1");
        assert_eq!(result[0].1, "handler");
        assert!((result[0].2 - 100.0).abs() < f32::EPSILON);
        assert!((result[0].3 - 200.0).abs() < f32::EPSILON);
    }

    #[test]
    fn multiple_patches_use_correct_key_prefixes() {
        let patches = vec![
            ExtensionPatchPreview {
                key: "ext-a".to_string(),
                nodes: vec![PreviewNode {
                    temp_id: "alpha".to_string(),
                    node_type: "ingress".to_string(),
                    x: 10.0,
                    y: 20.0,
                }],
                connections: vec![],
            },
            ExtensionPatchPreview {
                key: "ext-b".to_string(),
                nodes: vec![
                    PreviewNode {
                        temp_id: "beta".to_string(),
                        node_type: "handler".to_string(),
                        x: 30.0,
                        y: 40.0,
                    },
                    PreviewNode {
                        temp_id: "gamma".to_string(),
                        node_type: "egress".to_string(),
                        x: 50.0,
                        y: 60.0,
                    },
                ],
                connections: vec![],
            },
        ];

        let result = compute_preview_nodes(&patches);
        assert_eq!(result.len(), 3);
        assert_eq!(result[0].0, "p0-alpha");
        assert_eq!(result[1].0, "p1-beta");
        assert_eq!(result[2].0, "p1-gamma");
    }

    #[test]
    fn edge_between_existing_and_proposed_node_produces_one_edge() {
        let existing_id = NodeId::new();
        let mut existing = HashMap::new();
        existing.insert(existing_id, sample_node(existing_id, 0.0, 100.0));

        let patches = vec![ExtensionPatchPreview {
            key: "test-ext".to_string(),
            nodes: vec![PreviewNode {
                temp_id: "new1".to_string(),
                node_type: "handler".to_string(),
                x: 300.0,
                y: 100.0,
            }],
            connections: vec![PreviewConnection {
                source: PreviewEndpoint::Existing(existing_id),
                target: PreviewEndpoint::Proposed("new1".to_string()),
                source_port: "out".to_string(),
                target_port: "in".to_string(),
            }],
        }];

        let result = compute_preview_edges(&patches, &existing);
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].0, "p0-e0");
        // Verify the SVG path starts with "M" (it's a valid path)
        assert!(result[0].1.starts_with('M'), "SVG path should start with 'M'");
    }

    #[test]
    fn edge_with_missing_existing_node_is_skipped() {
        let missing_id = NodeId::new();
        let existing = HashMap::new(); // empty, so missing_id won't be found

        let patches = vec![ExtensionPatchPreview {
            key: "test-ext".to_string(),
            nodes: vec![PreviewNode {
                temp_id: "new1".to_string(),
                node_type: "handler".to_string(),
                x: 300.0,
                y: 100.0,
            }],
            connections: vec![PreviewConnection {
                source: PreviewEndpoint::Existing(missing_id),
                target: PreviewEndpoint::Proposed("new1".to_string()),
                source_port: "out".to_string(),
                target_port: "in".to_string(),
            }],
        }];

        let result = compute_preview_edges(&patches, &existing);
        assert!(result.is_empty(), "Edge with missing node should be skipped");
    }

    #[test]
    fn edge_between_two_proposed_nodes() {
        let existing = HashMap::new();

        let patches = vec![ExtensionPatchPreview {
            key: "test-ext".to_string(),
            nodes: vec![
                PreviewNode {
                    temp_id: "src".to_string(),
                    node_type: "ingress".to_string(),
                    x: 0.0,
                    y: 0.0,
                },
                PreviewNode {
                    temp_id: "tgt".to_string(),
                    node_type: "handler".to_string(),
                    x: 200.0,
                    y: 100.0,
                },
            ],
            connections: vec![PreviewConnection {
                source: PreviewEndpoint::Proposed("src".to_string()),
                target: PreviewEndpoint::Proposed("tgt".to_string()),
                source_port: "out".to_string(),
                target_port: "in".to_string(),
            }],
        }];

        let result = compute_preview_edges(&patches, &existing);
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].0, "p0-e0");
        // Source position: (0 + 220, 0 + 34) = (220, 34)
        // Target position: (200, 100 + 34) = (200, 134)
        // mid_x = midpoint(220, 200) = 210
        assert!(result[0].1.contains("M 220 34"), "Source should be at (220, 34)");
    }

    #[test]
    fn multiple_edges_in_single_patch_use_correct_indices() {
        let existing_id = NodeId::new();
        let mut existing = HashMap::new();
        existing.insert(existing_id, sample_node(existing_id, 0.0, 0.0));

        let patches = vec![ExtensionPatchPreview {
            key: "test-ext".to_string(),
            nodes: vec![
                PreviewNode {
                    temp_id: "a".to_string(),
                    node_type: "handler".to_string(),
                    x: 300.0,
                    y: 0.0,
                },
                PreviewNode {
                    temp_id: "b".to_string(),
                    node_type: "handler".to_string(),
                    x: 600.0,
                    y: 0.0,
                },
            ],
            connections: vec![
                PreviewConnection {
                    source: PreviewEndpoint::Existing(existing_id),
                    target: PreviewEndpoint::Proposed("a".to_string()),
                    source_port: "out".to_string(),
                    target_port: "in".to_string(),
                },
                PreviewConnection {
                    source: PreviewEndpoint::Proposed("a".to_string()),
                    target: PreviewEndpoint::Proposed("b".to_string()),
                    source_port: "out".to_string(),
                    target_port: "in".to_string(),
                },
            ],
        }];

        let result = compute_preview_edges(&patches, &existing);
        assert_eq!(result.len(), 2);
        assert_eq!(result[0].0, "p0-e0");
        assert_eq!(result[1].0, "p0-e1");
    }
}
