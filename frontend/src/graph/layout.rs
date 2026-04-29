use crate::graph::{NodeId, Workflow};
use petgraph::algo::toposort;
use petgraph::graph::NodeIndex;
use petgraph::Graph;
use std::collections::HashMap;

/// Visual width of a flow node in pixels.
const NODE_WIDTH: f32 = 220.0;

/// Visual height of a flow node in pixels.
const NODE_HEIGHT: f32 = 68.0;

pub struct DagLayout {
    pub layer_spacing: f32,
    pub node_spacing: f32,
}

impl Default for DagLayout {
    fn default() -> Self {
        Self { layer_spacing: 140.0, node_spacing: 60.0 }
    }
}

const LEFT_PADDING: f32 = 120.0;
const TOP_PADDING: f32 = 80.0;

impl DagLayout {
    #[allow(clippy::cast_precision_loss, clippy::too_many_lines, clippy::items_after_statements)]
    pub fn apply(&self, workflow: &mut Workflow) {
        if workflow.nodes.is_empty() {
            return;
        }

        let mut graph = Graph::<NodeId, ()>::new();
        let mut index_map = HashMap::new();
        let mut reverse_map = HashMap::new();
        let mut node_order = HashMap::new();

        // Add nodes to petgraph
        for (order, node) in workflow.nodes.iter().enumerate() {
            let idx = graph.add_node(node.id);
            index_map.insert(node.id, idx);
            reverse_map.insert(idx, node.id);
            node_order.insert(idx, order);
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
        let Ok(sorted_indices) = toposort(&graph, None) else {
            // If cyclic, try to layout what we can or return
            return;
        };

        // 2. Layer Assignment (Longest Path Layering)
        let mut layers: HashMap<NodeIndex, usize> = HashMap::new();
        for &node_idx in &sorted_indices {
            let layer = graph
                .neighbors_directed(node_idx, petgraph::Direction::Incoming)
                .filter_map(|parent| layers.get(&parent).map(|&l| l + 1))
                .max()
                .map_or(0, |layer| layer);
            layers.insert(node_idx, layer);
        }

        // Group by layer in deterministic topological order
        let mut nodes_by_layer: Vec<Vec<NodeIndex>> = Vec::new();
        for &node_idx in &sorted_indices {
            let layer = layers.get(&node_idx).map_or(0, |value| *value);
            while nodes_by_layer.len() <= layer {
                nodes_by_layer.push(Vec::new());
            }
            nodes_by_layer[layer].push(node_idx);
        }

        // 3. Crossing minimization (barycenter sweep)
        // Optimized: use a position HashMap for O(1) index lookups instead of
        // .iter().position() which is O(n) per call, called per node per iteration.
        for _ in 0..4 {
            for layer_idx in 1..nodes_by_layer.len() {
                // Build position map for the previous layer: O(n) once per layer
                let prev_positions: HashMap<NodeIndex, usize> = nodes_by_layer[layer_idx - 1]
                    .iter()
                    .enumerate()
                    .map(|(pos, &node)| (node, pos))
                    .collect();

                let mut barycenters: Vec<(NodeIndex, f32)> = nodes_by_layer[layer_idx]
                    .iter()
                    .map(|&node| {
                        let (sum, count) = graph
                            .neighbors_directed(node, petgraph::Direction::Incoming)
                            .filter_map(|parent| prev_positions.get(&parent).map(|&pos| pos as f32))
                            .fold((0.0, 0.0), |(s, c), pos| (s + pos, c + 1.0));

                        let barycenter = if count > 0.0 { sum / count } else { 0.0 };
                        (node, barycenter)
                    })
                    .collect();

                barycenters.sort_by(|a, b| {
                    let order_a = node_order.get(&a.0).map_or(usize::MAX, |value| *value);
                    let order_b = node_order.get(&b.0).map_or(usize::MAX, |value| *value);
                    a.1.total_cmp(&b.1).then_with(|| order_a.cmp(&order_b))
                });
                nodes_by_layer[layer_idx] = barycenters.into_iter().map(|(n, _)| n).collect();
            }
        }

        // 4. Coordinate assignment (left-to-right layered layout)
        //
        // Pre-build a NodeId -> index map for O(1) node lookups instead of
        // repeated workflow.nodes.iter_mut().find() which is O(n) per call.
        let node_position_map: HashMap<NodeId, usize> =
            workflow.nodes.iter().enumerate().map(|(i, n)| (n.id, i)).collect();

        let mut y_by_index: HashMap<NodeIndex, f32> = HashMap::new();
        let mut max_layer_height = 0.0_f32;

        for (layer, nodes) in nodes_by_layer.iter().enumerate() {
            let mut placed_y: Vec<f32> = Vec::new();

            for node_idx in nodes {
                let parent_positions = graph
                    .neighbors_directed(*node_idx, petgraph::Direction::Incoming)
                    .filter_map(|parent| y_by_index.get(&parent).copied())
                    .collect::<Vec<_>>();

                let preferred_y = if parent_positions.is_empty() {
                    0.0
                } else {
                    parent_positions.iter().sum::<f32>() / (parent_positions.len() as f32)
                };

                let y = placed_y.last().map_or(preferred_y, |prev| {
                    preferred_y.max(*prev + NODE_HEIGHT + self.node_spacing)
                });
                placed_y.push(y);
                y_by_index.insert(*node_idx, y);
            }

            if let (Some(first), Some(last)) = (placed_y.first(), placed_y.last()) {
                let layer_height = (last - first + NODE_HEIGHT).max(0.0);
                max_layer_height = max_layer_height.max(layer_height);
            }

            let x = (layer as f32) * (NODE_WIDTH + self.layer_spacing);
            for node_idx in nodes {
                if let Some(&node_id) = reverse_map.get(node_idx) {
                    if let Some(&idx) = node_position_map.get(&node_id) {
                        workflow.nodes[idx].x = x;
                    }
                }
            }
        }

        for nodes in &nodes_by_layer {
            let layer_positions =
                nodes.iter().filter_map(|idx| y_by_index.get(idx).copied()).collect::<Vec<_>>();
            if layer_positions.is_empty() {
                continue;
            }

            let first = layer_positions.first().map_or(0.0, |v| *v);
            let last = layer_positions.last().map_or(0.0, |v| *v);
            let layer_height = (last - first + NODE_HEIGHT).max(0.0);
            let layer_offset = (max_layer_height - layer_height) / 2.0;

            for node_idx in nodes {
                if let Some(&node_id) = reverse_map.get(node_idx) {
                    if let Some(&idx) = node_position_map.get(&node_id) {
                        let y = y_by_index.get(node_idx).map_or(0.0, |value| *value);
                        workflow.nodes[idx].y = y + layer_offset;
                    }
                }
            }
        }

        // Single pass to find both min_x and min_y instead of two separate iterations
        let (min_x, min_y) =
            workflow.nodes.iter().fold((f32::INFINITY, f32::INFINITY), |(mx, my), node| {
                (mx.min(node.x), my.min(node.y))
            });
        let min_x = if min_x.is_finite() { min_x } else { 0.0 };
        let min_y = if min_y.is_finite() { min_y } else { 0.0 };

        for node in &mut workflow.nodes {
            node.x = node.x - min_x + LEFT_PADDING;
            node.y = node.y - min_y + TOP_PADDING;
        }
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used, clippy::panic, clippy::float_cmp)]
mod tests {
    use super::{DagLayout, LEFT_PADDING, NODE_WIDTH, TOP_PADDING};
    use crate::graph::{Connection, NodeId, PortName, Workflow};

    #[test]
    fn given_cycle_when_applying_layout_then_node_positions_remain_unchanged() {
        let mut workflow = Workflow::new();
        let a = workflow.add_node("run", 10.0, 20.0);
        let b = workflow.add_node("run", 40.0, 50.0);
        let before: Vec<(f32, f32)> = workflow.nodes.iter().map(|n| (n.x, n.y)).collect();

        workflow.connections.push(Connection {
            id: uuid::Uuid::new_v4(),
            source: a,
            target: b,
            source_port: PortName::from("main"),
            target_port: PortName::from("main"),
        });
        workflow.connections.push(Connection {
            id: uuid::Uuid::new_v4(),
            source: b,
            target: a,
            source_port: PortName::from("main"),
            target_port: PortName::from("main"),
        });

        DagLayout::default().apply(&mut workflow);

        let after: Vec<(f32, f32)> = workflow.nodes.iter().map(|n| (n.x, n.y)).collect();
        assert_eq!(before, after);
    }

    #[test]
    fn disconnected_graph_when_applying_layout_then_nodes_have_distinct_y_positions() {
        let mut workflow = Workflow::new();
        workflow.add_node("run", 0.0, 0.0);
        workflow.add_node("run", 0.0, 0.0);
        workflow.add_node("run", 0.0, 0.0);

        DagLayout::default().apply(&mut workflow);

        let mut ys: Vec<f32> = workflow.nodes.iter().map(|n| n.y).collect();
        ys.sort_by(f32::total_cmp);
        ys.dedup_by(|a, b| (*a - *b).abs() < 0.001);
        assert_eq!(ys.len(), 3);
    }

    #[test]
    fn same_graph_when_applying_twice_then_positions_are_deterministic() {
        let mut workflow = Workflow::new();
        let n1 = workflow.add_node("http-handler", 0.0, 0.0);
        let n2 = workflow.add_node("run", 0.0, 0.0);
        let n3 = workflow.add_node("condition", 0.0, 0.0);
        let main = PortName::from("main");
        let _ = workflow.add_connection_checked(n1, n2, &main, &main);
        let _ = workflow.add_connection_checked(n2, n3, &main, &main);

        let layout = DagLayout::default();
        layout.apply(&mut workflow);
        let once: Vec<(String, f32, f32)> =
            workflow.nodes.iter().map(|n| (n.name.clone(), n.x, n.y)).collect();

        layout.apply(&mut workflow);
        let twice: Vec<(String, f32, f32)> =
            workflow.nodes.iter().map(|n| (n.name.clone(), n.x, n.y)).collect();

        assert_eq!(once, twice);
    }

    #[test]
    fn layout_result_when_normalized_then_minimum_coordinates_match_padding() {
        let mut workflow = Workflow::new();
        workflow.add_node("run", -500.0, -400.0);
        workflow.add_node("run", -350.0, -200.0);

        DagLayout::default().apply(&mut workflow);

        let min_x = workflow.nodes.iter().map(|n| n.x).reduce(f32::min);
        let min_y = workflow.nodes.iter().map(|n| n.y).reduce(f32::min);
        assert!(min_x.is_some_and(|value| (value - LEFT_PADDING).abs() < 0.001));
        assert!(min_y.is_some_and(|value| (value - TOP_PADDING).abs() < 0.001));
    }

    // ---------------------------------------------------------------------------
    // Coordinate normalization / normalization with padding
    // ---------------------------------------------------------------------------

    #[test]
    fn given_positive_coordinates_when_applying_layout_then_minimum_x_equals_left_padding() {
        let mut workflow = Workflow::new();
        workflow.add_node("run", 0.0, 0.0);
        workflow.add_node("run", 100.0, 100.0);

        DagLayout::default().apply(&mut workflow);

        let min_x = workflow.nodes.iter().map(|n| n.x).fold(f32::INFINITY, f32::min);
        assert!(
            (min_x - LEFT_PADDING).abs() < 0.001,
            "min_x {min_x} should equal LEFT_PADDING {LEFT_PADDING}"
        );
    }

    #[test]
    fn given_positive_coordinates_when_applying_layout_then_minimum_y_equals_top_padding() {
        let mut workflow = Workflow::new();
        workflow.add_node("run", 0.0, 0.0);
        workflow.add_node("run", 100.0, 100.0);

        DagLayout::default().apply(&mut workflow);

        let min_y = workflow.nodes.iter().map(|n| n.y).fold(f32::INFINITY, f32::min);
        assert!(
            (min_y - TOP_PADDING).abs() < 0.001,
            "min_y {min_y} should equal TOP_PADDING {TOP_PADDING}"
        );
    }

    #[test]
    fn given_zero_coordinates_when_applying_layout_then_positions_are_at_padding_offset() {
        let mut workflow = Workflow::new();
        workflow.add_node("run", 0.0, 0.0);

        DagLayout::default().apply(&mut workflow);

        let node = workflow.nodes.first().expect("one node");
        assert!((node.x - LEFT_PADDING).abs() < 0.001, "x should be LEFT_PADDING");
        assert!((node.y - TOP_PADDING).abs() < 0.001, "y should be TOP_PADDING");
    }

    // ---------------------------------------------------------------------------
    // Empty / single-node graph
    // ---------------------------------------------------------------------------

    #[test]
    fn given_empty_workflow_when_applying_layout_then_no_panic_occurs() {
        let mut workflow = Workflow::new();
        DagLayout::default().apply(&mut workflow);
        assert!(workflow.nodes.is_empty());
    }

    #[test]
    fn given_single_node_when_applying_layout_then_position_is_at_padding() {
        let mut workflow = Workflow::new();
        workflow.add_node("run", 999.0, 888.0);

        DagLayout::default().apply(&mut workflow);

        let node = workflow.nodes.first().expect("one node");
        assert!((node.x - LEFT_PADDING).abs() < 0.001, "single node x should be LEFT_PADDING");
        assert!((node.y - TOP_PADDING).abs() < 0.001, "single node y should be TOP_PADDING");
    }

    // ---------------------------------------------------------------------------
    // Layer assignment — nodes spread across layers based on depth
    // ---------------------------------------------------------------------------

    #[test]
    fn given_linear_chain_when_applying_layout_then_nodes_are_in_ascending_x_order() {
        let mut workflow = Workflow::new();
        let a = workflow.add_node("http-handler", 0.0, 0.0);
        let b = workflow.add_node("run", 100.0, 0.0);
        let c = workflow.add_node("run", 200.0, 0.0);
        let main = PortName::from("main");

        let _ = workflow.add_connection_checked(a, b, &main, &main);
        let _ = workflow.add_connection_checked(b, c, &main, &main);

        DagLayout::default().apply(&mut workflow);

        let positions: Vec<(NodeId, f32)> = workflow.nodes.iter().map(|n| (n.id, n.x)).collect();

        let x_a = positions.iter().find(|(id, _)| *id == a).map(|(_, x)| *x);
        let x_b = positions.iter().find(|(id, _)| *id == b).map(|(_, x)| *x);
        let x_c = positions.iter().find(|(id, _)| *id == c).map(|(_, x)| *x);

        let (x_a, x_b, x_c) = (x_a.expect("a"), x_b.expect("b"), x_c.expect("c"));

        assert!(x_a < x_b, "a.x ({x_a}) should be < b.x ({x_b}) in linear chain");
        assert!(x_b < x_c, "b.x ({x_b}) should be < c.x ({x_c}) in linear chain");
    }

    // ---------------------------------------------------------------------------
    // Crossing minimization — deterministic output
    // ---------------------------------------------------------------------------

    #[test]
    fn given_diamond_dag_when_applying_layout_twice_then_results_are_identical() {
        let mut workflow = Workflow::new();
        let a = workflow.add_node("http-handler", 0.0, 0.0);
        let b = workflow.add_node("run", 100.0, 0.0);
        let c = workflow.add_node("run", 100.0, 100.0);
        let d = workflow.add_node("run", 200.0, 50.0);
        let main = PortName::from("main");

        let _ = workflow.add_connection_checked(a, b, &main, &main);
        let _ = workflow.add_connection_checked(a, c, &main, &main);
        let _ = workflow.add_connection_checked(b, d, &main, &main);
        let _ = workflow.add_connection_checked(c, d, &main, &main);

        let layout = DagLayout::default();
        layout.apply(&mut workflow);
        let first: Vec<(f32, f32)> = workflow.nodes.iter().map(|n| (n.x, n.y)).collect();

        layout.apply(&mut workflow);
        let second: Vec<(f32, f32)> = workflow.nodes.iter().map(|n| (n.x, n.y)).collect();

        assert_eq!(first, second, "layout should be deterministic");
    }

    // ---------------------------------------------------------------------------
    // Custom spacing
    // ---------------------------------------------------------------------------

    #[test]
    fn given_custom_layer_spacing_when_applying_layout_then_nodes_honor_spacing() {
        let mut workflow = Workflow::new();
        let a = workflow.add_node("http-handler", 0.0, 0.0);
        let b = workflow.add_node("run", 100.0, 0.0);
        let main = PortName::from("main");

        let _ = workflow.add_connection_checked(a, b, &main, &main);

        let custom_layout = DagLayout { layer_spacing: 300.0, node_spacing: 80.0 };
        custom_layout.apply(&mut workflow);

        let x_a = workflow.nodes.iter().find(|n| n.id == a).map(|n| n.x);
        let x_b = workflow.nodes.iter().find(|n| n.id == b).map(|n| n.x);

        let (x_a, x_b) = (x_a.expect("a"), x_b.expect("b"));
        let distance = x_b - x_a;

        // With custom layer_spacing of 300.0, the gap between layers should
        // be NODE_WIDTH (220) + layer_spacing (300) = 520
        let expected_gap = NODE_WIDTH + 300.0_f32;
        assert!(
            (distance - expected_gap).abs() < 0.001,
            "gap {distance} should be ~{expected_gap}"
        );
    }

    // ---------------------------------------------------------------------------
    // Disconnected graph — all nodes should get distinct positions
    // ---------------------------------------------------------------------------

    #[test]
    fn given_many_disconnected_nodes_when_applying_layout_then_all_have_unique_positions() {
        let mut workflow = Workflow::new();
        for i in 0..10 {
            workflow.add_node("run", i as f32 * 10.0, 0.0);
        }

        DagLayout::default().apply(&mut workflow);

        let positions: Vec<(f32, f32)> = workflow.nodes.iter().map(|n| (n.x, n.y)).collect();
        let mut unique = positions.clone();
        unique.sort_by(|a, b| {
            a.0.partial_cmp(&b.0)
                .unwrap_or(std::cmp::Ordering::Equal)
                .then_with(|| a.1.partial_cmp(&b.1).unwrap_or(std::cmp::Ordering::Equal))
        });
        unique.dedup_by(|a, b| (a.0 - b.0).abs() < 0.001 && (a.1 - b.1).abs() < 0.001);

        assert_eq!(unique.len(), 10, "all 10 disconnected nodes should have unique positions");
    }

    // ---------------------------------------------------------------------------
    // All coordinates are finite
    // ---------------------------------------------------------------------------

    #[test]
    fn given_varied_node_positions_when_applying_layout_then_all_coordinates_are_finite() {
        let mut workflow = Workflow::new();
        let a = workflow.add_node("http-handler", -1000.0, 1000.0);
        let b = workflow.add_node("run", 0.0, -500.0);
        let c = workflow.add_node("run", 500.0, 0.0);
        let main = PortName::from("main");

        let _ = workflow.add_connection_checked(a, b, &main, &main);
        let _ = workflow.add_connection_checked(b, c, &main, &main);

        DagLayout::default().apply(&mut workflow);

        for node in &workflow.nodes {
            assert!(node.x.is_finite(), "node {} x={}\u{00a0}is not finite", node.id, node.x);
            assert!(node.y.is_finite(), "node {} y={}\u{00a0}is not finite", node.id, node.y);
        }
    }

    // ---------------------------------------------------------------------------
    // Property-based tests (proptest)
    // ---------------------------------------------------------------------------

    use proptest::prelude::*;

    /// Node dimensions -- must match the values used throughout the codebase
    /// (editor_interactions, layout constants, etc.).
    const PROP_NODE_W: f32 = 220.0;
    const PROP_NODE_H: f32 = 68.0;

    /// Inlined mirror of `editor_interactions::normalize_rect` for use in
    /// property tests (the `ui` module is gated to the binary crate).
    fn normalize_rect(start: (f32, f32), end: (f32, f32)) -> (f32, f32, f32, f32) {
        (start.0.min(end.0), start.1.min(end.1), start.0.max(end.0), start.1.max(end.1))
    }

    /// Inlined mirror of `editor_interactions::rect_contains`.
    fn rect_contains(rect: (f32, f32, f32, f32), point: (f32, f32)) -> bool {
        let (rx1, ry1, rx2, ry2) = rect;
        let (px, py) = point;
        px >= rx1 && px <= rx2 && py >= ry1 && py <= ry2
    }

    /// Inlined mirror of `editor_interactions::node_intersects_rect`.
    fn node_intersects_rect(node_x: f32, node_y: f32, rect: (f32, f32, f32, f32)) -> bool {
        let node_right = node_x + PROP_NODE_W;
        let node_bottom = node_y + PROP_NODE_H;
        !(node_right < rect.0 || node_x > rect.2 || node_bottom < rect.1 || node_y > rect.3)
    }

    proptest! {
        #![proptest_config(ProptestConfig::with_cases(64))]

        // Property: normalize_rect always produces a valid rectangle where
        // min <= max for both dimensions, regardless of input ordering.
        #[test]
        fn given_random_rect_when_normalizing_then_min_lte_max(
            x1 in any::<f32>(),
            y1 in any::<f32>(),
            x2 in any::<f32>(),
            y2 in any::<f32>(),
        ) {
            prop_assume!(x1.is_finite() && y1.is_finite() && x2.is_finite() && y2.is_finite());

            let (min_x, min_y, max_x, max_y) = normalize_rect((x1, y1), (x2, y2));

            prop_assert!(min_x <= max_x);
            prop_assert!(min_y <= max_y);
        }

        // Property: node_intersects_rect is commutative -- if node A's
        // bounding box intersects node B's bounding box, then B also
        // intersects A.
        #[test]
        fn given_two_nodes_when_checking_intersection_then_result_is_commutative(
            ax in any::<f32>(),
            ay in any::<f32>(),
            bx in any::<f32>(),
            by in any::<f32>(),
        ) {
            prop_assume!(ax.is_finite() && ay.is_finite() && bx.is_finite() && by.is_finite());

            let rect_a = (ax, ay, ax + PROP_NODE_W, ay + PROP_NODE_H);
            let rect_b = (bx, by, bx + PROP_NODE_W, by + PROP_NODE_H);

            let a_hits_b = node_intersects_rect(ax, ay, rect_b);
            let b_hits_a = node_intersects_rect(bx, by, rect_a);

            prop_assert_eq!(a_hits_b, b_hits_a);
        }

        // Property: rect_contains returns true for a point inside the rect
        // and false for points outside the rect.
        #[test]
        fn given_valid_rect_when_checking_contains_then_inside_true_outside_false(
            min_x in 0.0_f32..1000.0,
            min_y in 0.0_f32..1000.0,
            width in 1.0_f32..500.0,
            height in 1.0_f32..500.0,
        ) {
            let max_x = min_x + width;
            let max_y = min_y + height;
            let rect = (min_x, min_y, max_x, max_y);

            // Midpoint must be contained
            let mid_x = f32::midpoint(min_x, max_x);
            let mid_y = f32::midpoint(min_y, max_y);
            prop_assert!(rect_contains(rect, (mid_x, mid_y)));

            // Point beyond max corner must not be contained
            prop_assert!(!rect_contains(rect, (max_x + 1.0, max_y + 1.0)));

            // Point before min corner must not be contained
            let before_x = min_x - 1.0;
            let before_y = min_y - 1.0;
            prop_assume!(before_x.is_finite() && before_y.is_finite());
            prop_assert!(!rect_contains(rect, (before_x, before_y)));
        }

        // Property: DagLayout::apply produces non-overlapping node bounding
        // boxes when all nodes start at distinct positions.
        #[test]
        fn given_distinct_nodes_when_applying_layout_then_no_bounding_boxes_overlap(
            xs in prop::collection::vec(0.0_f32..2000.0, 2..8),
            ys in prop::collection::vec(0.0_f32..2000.0, 2..8),
        ) {
            let mut workflow = Workflow::new();
            for (&x, &y) in xs.iter().zip(ys.iter()) {
                workflow.add_node("run", x, y);
            }

            DagLayout::default().apply(&mut workflow);

            let boxes: Vec<(f32, f32, f32, f32)> = workflow
                .nodes
                .iter()
                .map(|n| (n.x, n.y, n.x + PROP_NODE_W, n.y + PROP_NODE_H))
                .collect();

            for i in 0..boxes.len() {
                for j in (i + 1)..boxes.len() {
                    let (l1, t1, r1, b1) = boxes[i];
                    let (l2, t2, r2, b2) = boxes[j];

                    let overlaps = !(r1 <= l2 || r2 <= l1 || b1 <= t2 || b2 <= t1);
                    prop_assert!(!overlaps);
                }
            }
        }
    }
}
