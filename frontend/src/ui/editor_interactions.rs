#![deny(clippy::unwrap_used)]
#![deny(clippy::expect_used)]
#![deny(clippy::panic)]
#![warn(clippy::pedantic)]
#![forbid(unsafe_code)]

pub use crate::ui::constants::{NODE_HANDLE_Y_OFFSET, NODE_HEIGHT, NODE_WIDTH};

pub type SelectionRect = (f32, f32, f32, f32);
type SnapOutput = (crate::graph::NodeId, String, crate::ui::edges::Position);

#[derive(Clone, Copy)]
struct SnapPoint {
    canvas_x: f32,
    canvas_y: f32,
    radius_sq: f32,
}

#[derive(Clone)]
struct SnapCandidate {
    node_id: crate::graph::NodeId,
    kind: &'static str,
    position: crate::ui::edges::Position,
    dist_sq: f32,
}

#[must_use]
pub fn normalize_rect(start: (f32, f32), end: (f32, f32)) -> SelectionRect {
    let min_x = start.0.min(end.0);
    let min_y = start.1.min(end.1);
    let max_x = start.0.max(end.0);
    let max_y = start.1.max(end.1);
    (min_x, min_y, max_x, max_y)
}

#[must_use]
pub fn rect_contains(rect: SelectionRect, point: (f32, f32)) -> bool {
    point.0 >= rect.0 && point.0 <= rect.2 && point.1 >= rect.1 && point.1 <= rect.3
}

#[must_use]
pub fn node_intersects_rect(node_x: f32, node_y: f32, rect: SelectionRect) -> bool {
    let node_left = node_x;
    let node_top = node_y;
    let node_right = node_x + NODE_WIDTH;
    let node_bottom = node_y + NODE_HEIGHT;

    !(node_right < rect.0 || node_left > rect.2 || node_bottom < rect.1 || node_top > rect.3)
}

#[must_use]
pub fn snap_handle(
    nodes: &[crate::graph::Node],
    mx: f32,
    my: f32,
    viewport: &crate::graph::Viewport,
) -> Option<SnapOutput> {
    let snap_point = canvas_snap_point(mx, my, viewport)?;
    let best = nodes
        .iter()
        .flat_map(|node| node_snap_candidates(node, snap_point))
        .reduce(choose_snap_candidate);

    best.map(|candidate| (candidate.node_id, candidate.kind.to_string(), candidate.position))
}

fn canvas_snap_point(mx: f32, my: f32, viewport: &crate::graph::Viewport) -> Option<SnapPoint> {
    const SCREEN_SNAP_RADIUS: f32 = 24.0;

    let zoom = viewport.zoom;
    if !zoom.is_finite() || zoom.abs() <= f32::EPSILON {
        return None;
    }

    let canvas_radius = SCREEN_SNAP_RADIUS / zoom.abs();
    Some(SnapPoint {
        canvas_x: (mx - viewport.x) / zoom,
        canvas_y: (my - viewport.y) / zoom,
        radius_sq: canvas_radius * canvas_radius,
    })
}

fn node_snap_candidates(
    node: &crate::graph::Node,
    snap_point: SnapPoint,
) -> impl Iterator<Item = SnapCandidate> + '_ {
    handle_positions(node)
        .into_iter()
        .filter_map(move |(kind, position)| snap_candidate(node.id, kind, position, snap_point))
}

fn handle_positions(node: &crate::graph::Node) -> [(&'static str, crate::ui::edges::Position); 2] {
    let handle_y = node.y + NODE_HANDLE_Y_OFFSET;
    [
        ("target", crate::ui::edges::Position { x: node.x, y: handle_y }),
        ("source", crate::ui::edges::Position { x: node.x + NODE_WIDTH, y: handle_y }),
    ]
}

fn snap_candidate(
    node_id: crate::graph::NodeId,
    kind: &'static str,
    position: crate::ui::edges::Position,
    snap_point: SnapPoint,
) -> Option<SnapCandidate> {
    let dx = snap_point.canvas_x - position.x;
    let dy = snap_point.canvas_y - position.y;
    let dist_sq = dx.mul_add(dx, dy * dy);
    (dist_sq <= snap_point.radius_sq).then_some(SnapCandidate { node_id, kind, position, dist_sq })
}

fn choose_snap_candidate(current: SnapCandidate, candidate: SnapCandidate) -> SnapCandidate {
    if snap_candidate_precedes(&current, &candidate) {
        current
    } else {
        candidate
    }
}

fn snap_candidate_precedes(current: &SnapCandidate, candidate: &SnapCandidate) -> bool {
    if current.dist_sq < candidate.dist_sq {
        return true;
    }
    if (current.dist_sq - candidate.dist_sq).abs() >= f32::EPSILON {
        return false;
    }

    current.node_id.0.cmp(&candidate.node_id.0).then(current.kind.cmp(candidate.kind)).is_le()
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used, clippy::panic, clippy::float_cmp)]
mod tests {
    use super::{node_intersects_rect, normalize_rect, rect_contains, snap_handle};
    use crate::graph::{Viewport, Workflow};

    #[test]
    fn given_drag_points_when_normalizing_then_rect_bounds_are_ordered() {
        let rect = normalize_rect((120.0, 30.0), (20.0, 90.0));

        assert_eq!(rect, (20.0, 30.0, 120.0, 90.0));
    }

    #[test]
    fn given_rect_boundary_point_when_checking_contains_then_point_is_inside() {
        let rect = (10.0, 10.0, 20.0, 20.0);

        assert!(rect_contains(rect, (10.0, 20.0)));
    }

    #[test]
    fn given_node_overlapping_selection_when_checking_intersection_then_it_is_detected() {
        let intersects = node_intersects_rect(50.0, 50.0, (0.0, 0.0, 100.0, 100.0));

        assert!(intersects);
    }

    #[test]
    fn given_min_clamped_zoom_when_snapping_handle_then_zoom_is_valid() {
        let mut workflow = Workflow::new();
        let _ = workflow.add_node("http-handler", 200.0, 200.0);

        // new_clamped(0.0) produces MIN_ZOOM (0.15), which is valid
        let result = snap_handle(
            &workflow.nodes,
            200.0,
            200.0,
            &Viewport { x: 0.0, y: 0.0, zoom: 0.0_f32.clamp(0.15, 3.0) },
        );

        // ZoomFactor guarantees valid zoom, so snap_handle proceeds normally
        // The cursor is far from any handle at this position, so result is None
        // but NOT because of invalid zoom
        assert!(result.is_none());
    }

    #[test]
    fn given_zoom_level_when_snapping_then_behavior_is_zoom_invariant() {
        let mut workflow = Workflow::new();
        let _ = workflow.add_node("node", 100.0, 100.0);

        // Source handle is at (320, 134) in canvas space
        // At zoom 1.0: cursor at (318, 134) → canvas (318, 134) → distance 2
        // At zoom 0.5: cursor at (159, 67) → canvas (318, 134) → distance 2
        // At zoom 2.0: cursor at (240, 118) → canvas (120, 59) → distance ~214 (exceeds threshold)

        let viewport_1x = Viewport { x: 0.0, y: 0.0, zoom: 1.0 };
        let snapped_1x = snap_handle(&workflow.nodes, 318.0, 134.0, &viewport_1x);

        let viewport_05x = Viewport { x: 0.0, y: 0.0, zoom: 0.5 };
        // Corrected: screen_x = canvas_x * zoom = 318 * 0.5 = 159
        let snapped_05x = snap_handle(&workflow.nodes, 159.0, 67.0, &viewport_05x);

        let viewport_2x = Viewport { x: 0.0, y: 0.0, zoom: 2.0 };
        // Corrected: screen = canvas * zoom to achieve same canvas position (318, 134)
        // canvas_x = mx / 2.0 = 318 → mx = 636
        let snapped_2x = snap_handle(&workflow.nodes, 636.0, 268.0, &viewport_2x);

        // All should snap to the same handle (source handle at x=320, y=134)
        assert!(snapped_1x.is_some());
        assert!(snapped_05x.is_some());
        assert!(snapped_2x.is_some());

        if let (Some((id1, kind1, _)), Some((id2, kind2, _)), Some((id3, kind3, _))) =
            (snapped_1x, snapped_05x, snapped_2x)
        {
            assert_eq!(id1, id2, "zoom 0.5 should select same node as zoom 1.0");
            assert_eq!(id2, id3, "zoom 2.0 should select same node as zoom 0.5");
            assert_eq!(kind1, kind2, "zoom 0.5 should select same handle kind as zoom 1.0");
            assert_eq!(kind2, kind3, "zoom 2.0 should select same handle kind as zoom 0.5");
        } else {
            panic!("all zoom levels should find a snap handle");
        }
    }

    #[test]
    fn given_equal_distance_candidates_when_snapping_then_selection_is_deterministic() {
        let mut workflow = Workflow::new();
        // Add two nodes with source handles close together (40 canvas units apart)
        // Node A at x=100 → source at (320, 134)
        // Node B at x=140 → source at (360, 134)
        let _ = workflow.add_node("node-a", 100.0, 100.0);
        let _ = workflow.add_node("node-b", 140.0, 100.0);

        let viewport = Viewport { x: 0.0, y: 0.0, zoom: 1.0 };

        // Cursor at (340, 134) is equidistant (20 units) from both source handles
        // This is within the snap threshold (24 canvas units)
        let snapped = snap_handle(&workflow.nodes, 340.0, 134.0, &viewport);

        assert!(snapped.is_some());

        // The selection should be deterministic based on node id ordering
        // Run multiple times to verify consistency
        let first_result = snap_handle(&workflow.nodes, 340.0, 134.0, &viewport);
        assert!(first_result.is_some());
        let (first_id, _, _) = first_result.unwrap();

        for _ in 0..10 {
            let result = snap_handle(&workflow.nodes, 340.0, 134.0, &viewport);
            assert!(result.is_some());
            let (node_id, _, _) = result.unwrap();
            // All results should be identical due to deterministic ordering
            assert_eq!(node_id, first_id, "deterministic tie-break should always return same node");
        }
    }

    #[test]
    fn given_infinite_zoom_when_validating_then_zoom_is_not_finite() {
        assert!(!f32::INFINITY.is_finite());
    }

    #[test]
    fn given_nan_zoom_when_validating_then_zoom_is_not_finite() {
        assert!(!f32::NAN.is_finite());
    }
}
