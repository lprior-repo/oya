#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic, clippy::float_cmp)]
use oya_frontend::graph::{PortName, Workflow};
use std::collections::HashMap;

fn snapshot_positions(workflow: &Workflow) -> HashMap<String, (f32, f32)> {
    workflow
        .nodes
        .iter()
        .map(|node| (node.id.to_string(), (node.x, node.y)))
        .collect::<HashMap<_, _>>()
}

fn same_positions(left: &HashMap<String, (f32, f32)>, right: &HashMap<String, (f32, f32)>) -> bool {
    left.len() == right.len()
        && left.iter().all(|(id, (lx, ly))| {
            right.get(id).is_some_and(|(rx, ry)| {
                (lx - rx).abs() < 0.000_1_f32 && (ly - ry).abs() < 0.000_1_f32
            })
        })
}

fn build_branching_workflow() -> Workflow {
    let mut workflow = Workflow::new();
    let start = workflow.add_node("http-handler", 20.0, 20.0);
    let branch = workflow.add_node("condition", 40.0, 120.0);
    let left = workflow.add_node("custom-left", 10.0, 260.0);
    let right = workflow.add_node("custom-right", 260.0, 260.0);
    let join = workflow.add_node("custom-join", 140.0, 420.0);

    let main = PortName("main".to_string());
    let _ = workflow.add_connection(start, branch, &main, &main);
    let _ = workflow.add_connection(branch, left, &main, &main);
    let _ = workflow.add_connection(branch, right, &main, &main);
    let _ = workflow.add_connection(left, join, &main, &main);
    let _ = workflow.add_connection(right, join, &main, &main);
    workflow
}

#[test]
fn given_branching_graph_when_layout_runs_twice_then_node_positions_are_stable() {
    let mut workflow = build_branching_workflow();

    workflow.apply_layout();
    let first = snapshot_positions(&workflow);
    workflow.apply_layout();
    let second = snapshot_positions(&workflow);

    assert!(same_positions(&first, &second));
}

#[test]
fn given_disconnected_graph_when_layout_runs_twice_then_positions_are_stable() {
    let mut workflow = Workflow::new();

    let a1 = workflow.add_node("a1", 0.0, 0.0);
    let a2 = workflow.add_node("a2", 0.0, 0.0);
    let b1 = workflow.add_node("b1", 0.0, 0.0);
    let b2 = workflow.add_node("b2", 0.0, 0.0);
    let isolated = workflow.add_node("isolated", 0.0, 0.0);

    let main = PortName("main".to_string());
    let _ = workflow.add_connection(a1, a2, &main, &main);
    let _ = workflow.add_connection(b1, b2, &main, &main);
    let _ = workflow.add_connection(a1, isolated, &main, &main);

    workflow.apply_layout();
    let first = snapshot_positions(&workflow);
    workflow.apply_layout();
    let second = snapshot_positions(&workflow);

    assert!(same_positions(&first, &second));
}

#[test]
fn given_layouted_graph_when_checking_canvas_bounds_then_nodes_stay_positive() {
    let mut workflow = build_branching_workflow();
    workflow.apply_layout();

    assert!(workflow.nodes.iter().all(|node| node.x >= 100.0_f32 && node.y >= 70.0_f32));
}

#[test]
fn given_same_bounds_when_fitting_view_twice_then_viewport_is_idempotent() {
    let mut workflow = build_branching_workflow();
    workflow.apply_layout();

    workflow.fit_view(1200.0, 720.0, 180.0);
    let first = workflow.viewport.clone();
    workflow.fit_view(1200.0, 720.0, 180.0);
    let second = workflow.viewport.clone();

    assert!((first.x - second.x).abs() < 0.000_1_f32);
    assert!((first.y - second.y).abs() < 0.000_1_f32);
    assert!((first.zoom - second.zoom).abs() < 0.000_1_f32);
}
