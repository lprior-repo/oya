#![deny(clippy::unwrap_used)]
#![deny(clippy::expect_used)]
#![deny(clippy::panic)]
#![warn(clippy::pedantic)]
#![forbid(unsafe_code)]

use crate::graph::{ExecutionState, Node, NodeId};
use crate::hooks::use_canvas_interaction::CanvasInteraction;
use crate::hooks::use_selection::SelectionState;
use crate::hooks::use_ui_panels::UiPanels;
use crate::hooks::use_workflow_state::WorkflowState;
use crate::ui::constants::{
    DEFAULT_CANVAS_HEIGHT, DEFAULT_CANVAS_WIDTH, FIT_VIEW_PADDING, ZOOM_CENTER_X, ZOOM_CENTER_Y,
    ZOOM_DELTA,
};
use crate::ui::{
    FlowEdges, FlowMinimap, FlowNodeComponent, FlowNodeEvent, FlowPosition, ParallelGroupOverlay,
};
use dioxus::html::input_data::MouseButton;
use dioxus::prelude::*;

#[derive(Clone, Copy, PartialEq)]
pub struct CanvasPreviewLayer {
    pub temp_edge: Memo<Option<(FlowPosition, FlowPosition)>>,
    pub preview_nodes: Memo<Vec<(String, String, f32, f32)>>,
    pub preview_edges: Memo<Vec<(String, String)>>,
}

#[derive(Clone, Copy, PartialEq)]
pub struct CanvasPanelControls {
    pub panels: UiPanels,
    pub show_inspector: Signal<bool>,
}

#[component]
pub fn CanvasArea(
    workflow: WorkflowState,
    selection: SelectionState,
    canvas: CanvasInteraction,
    preview: CanvasPreviewLayer,
    controls: CanvasPanelControls,
) -> Element {
    let nodes = workflow.nodes();
    let viewport = workflow.viewport();
    let vx = viewport.read().x;
    let vy = viewport.read().y;
    let vz = viewport.read().zoom;
    let running_node_ids = running_nodes(nodes);
    let zoom = use_memo(move || viewport.read().zoom);

    rsx! {
        CanvasGrid { vx, vy, vz }
        CanvasContentLayer {
            workflow,
            selection,
            canvas,
            preview,
            controls,
            running_node_ids,
            zoom,
            vx,
            vy,
            vz,
        }
        MarqueeSelection { canvas }
        CanvasMiniMapControls { workflow, selection }
    }
}

#[component]
fn CanvasGrid(vx: f32, vy: f32, vz: f32) -> Element {
    rsx! {
        div {
            class: "absolute inset-0 pointer-events-none",
            style: "background-image: radial-gradient(circle, rgba(100, 116, 139, 0.33) 1px, transparent 1px); background-size: calc(22px * {vz}) calc(22px * {vz}); background-position: {vx}px {vy}px;"
        }
        div {
            class: "canvas-grid-animated absolute inset-0 pointer-events-none opacity-35",
            style: "background-image: linear-gradient(120deg, rgba(14, 165, 233, 0.08), transparent 45%, rgba(20, 184, 166, 0.08)); background-size: 56px 56px;"
        }
    }
}

#[component]
fn CanvasContentLayer(
    workflow: WorkflowState,
    selection: SelectionState,
    canvas: CanvasInteraction,
    preview: CanvasPreviewLayer,
    controls: CanvasPanelControls,
    running_node_ids: Memo<Vec<NodeId>>,
    zoom: Memo<f32>,
    vx: f32,
    vy: f32,
    vz: f32,
) -> Element {
    let nodes = workflow.nodes();
    let connections = workflow.connections();
    rsx! {
        div {
            class: "absolute origin-top-left",
            style: "transform: translate({vx}px, {vy}px) scale({vz}); will-change: transform;",
            FlowEdges { edges: connections, nodes, temp_edge: preview.temp_edge, running_node_ids, zoom }
            ParallelGroupOverlay { nodes, connections }
            PreviewEdges { edges: preview.preview_edges }
            PreviewNodes { nodes: preview.preview_nodes }
            NodeLayer { workflow, selection, canvas, controls }
        }
    }
}

#[component]
fn PreviewEdges(edges: Memo<Vec<(String, String)>>) -> Element {
    rsx! {
        if !edges.read().is_empty() {
            svg { class: "absolute inset-0 overflow-visible pointer-events-none w-full h-full z-0",
                for (preview_edge_id, preview_path) in edges.read().iter() {
                    path {
                        key: "{preview_edge_id}",
                        d: "{preview_path}",
                        fill: "none",
                        stroke: "rgba(99, 102, 241, 0.75)",
                        stroke_width: "2",
                        stroke_dasharray: "6 4"
                    }
                }
            }
        }
    }
}

#[component]
fn PreviewNodes(nodes: Memo<Vec<(String, String, f32, f32)>>) -> Element {
    rsx! {
        for (preview_node_id, preview_node_type, preview_x, preview_y) in nodes.read().iter() {
            div {
                key: "{preview_node_id}",
                class: "pointer-events-none absolute w-[220px] z-0 rounded-xl border border-indigo-300/70 bg-indigo-500/10 px-3 py-2",
                style: "left: {preview_x}px; top: {preview_y}px;",
                div { class: "text-[11px] font-semibold text-indigo-700", "Preview" }
                div { class: "text-[10px] font-mono text-indigo-600", "{preview_node_type}" }
            }
        }
    }
}

#[component]
fn NodeLayer(
    workflow: WorkflowState,
    selection: SelectionState,
    canvas: CanvasInteraction,
    controls: CanvasPanelControls,
) -> Element {
    let nodes = workflow.nodes();
    rsx! {
        for node in nodes.read().iter().cloned() {
            CanvasNode { key: "{node.id}", node, workflow, selection, canvas, controls }
        }
    }
}

#[component]
fn CanvasNode(
    node: Node,
    workflow: WorkflowState,
    selection: SelectionState,
    canvas: CanvasInteraction,
    controls: CanvasPanelControls,
) -> Element {
    let node_id = node.id;
    let selected = selection.is_selected(node_id);
    let inline_open = controls.panels.is_inline_panel_open(node_id);
    let on_event = EventHandler::new(move |event| {
        handle_node_event(event, node_id, workflow, selection, canvas, controls);
    });

    rsx! { FlowNodeComponent { node, selected, inline_open, on_event } }
}

#[component]
fn MarqueeSelection(canvas: CanvasInteraction) -> Element {
    let Some((start, end)) = canvas.marquee_rect() else {
        return rsx! {};
    };
    let rect = crate::ui::editor_interactions::normalize_rect(start, end);
    let left = rect.0;
    let top = rect.1;
    let width = (rect.2 - rect.0).max(1.0);
    let height = (rect.3 - rect.1).max(1.0);
    rsx! {
        div {
            class: "pointer-events-none absolute border border-indigo-400/70 bg-indigo-500/10",
            style: "left: {left}px; top: {top}px; width: {width}px; height: {height}px;",
        }
    }
}

#[component]
fn CanvasMiniMapControls(workflow: WorkflowState, selection: SelectionState) -> Element {
    rsx! {
        FlowMinimap {
            nodes: workflow.nodes(),
            edges: workflow.connections(),
            selected_node_id: selection.selected_id(),
            viewport: workflow.viewport(),
            canvas_width: DEFAULT_CANVAS_WIDTH,
            canvas_height: DEFAULT_CANVAS_HEIGHT,
            on_zoom_in: move |evt: MouseEvent| {
                evt.stop_propagation();
                workflow.zoom(ZOOM_DELTA, ZOOM_CENTER_X, ZOOM_CENTER_Y);
            },
            on_zoom_out: move |evt: MouseEvent| {
                evt.stop_propagation();
                workflow.zoom(-ZOOM_DELTA, ZOOM_CENTER_X, ZOOM_CENTER_Y);
            },
            on_fit_view: move |evt: MouseEvent| {
                evt.stop_propagation();
                workflow.fit_view(DEFAULT_CANVAS_WIDTH, DEFAULT_CANVAS_HEIGHT, FIT_VIEW_PADDING);
            }
        }
    }
}

fn running_nodes(nodes: ReadSignal<Vec<Node>>) -> Memo<Vec<NodeId>> {
    use_memo(move || {
        nodes
            .read()
            .iter()
            .filter(|node| matches!(node.execution_state, ExecutionState::Running))
            .map(|node| node.id)
            .collect::<Vec<_>>()
    })
}

fn handle_node_event(
    event: FlowNodeEvent,
    node_id: NodeId,
    workflow: WorkflowState,
    selection: SelectionState,
    canvas: CanvasInteraction,
    controls: CanvasPanelControls,
) {
    match event {
        FlowNodeEvent::NodeMouseDown(evt) => {
            handle_node_mouse_down(&evt, node_id, selection, canvas)
        }
        FlowNodeEvent::NodeClick(_) => select_node(node_id, selection, controls),
        FlowNodeEvent::NodeDoubleClick(_) => controls.panels.toggle_inline_panel(node_id),
        FlowNodeEvent::HandleMouseDown { event, side } => {
            handle_port_mouse_down(&event, side, node_id, workflow, selection, canvas);
        }
        FlowNodeEvent::HandleMouseEnter(side) => {
            canvas.set_hovered_handle(Some((node_id, side.to_string())));
        }
        FlowNodeEvent::HandleMouseLeave => canvas.set_hovered_handle(None),
        FlowNodeEvent::InlineChange(new_config) => {
            update_node_config(workflow, node_id, &new_config)
        }
        FlowNodeEvent::InlineClose => controls.panels.close_inline_panel(),
    }
}

fn select_node(node_id: NodeId, selection: SelectionState, controls: CanvasPanelControls) {
    selection.select_single(node_id);
    let mut show_inspector = controls.show_inspector;
    show_inspector.set(true);
}

fn handle_node_mouse_down(
    evt: &MouseEvent,
    node_id: NodeId,
    selection: SelectionState,
    canvas: CanvasInteraction,
) {
    if evt.trigger_button() != Some(MouseButton::Primary) || canvas.is_space_hand_active() {
        return;
    }
    evt.stop_propagation();
    let Some((origin, mouse_pos)) = mouse_context(evt) else {
        return;
    };
    canvas.set_origin(origin);
    canvas.update_mouse(mouse_pos);
    let drag_targets = drag_targets_for_node(selection, node_id);
    selection.set_multiple(drag_targets.clone());
    selection.set_pending_drag(drag_targets);
    canvas.start_drag_anchor(mouse_pos);
}

fn handle_port_mouse_down(
    evt: &MouseEvent,
    side: &'static str,
    node_id: NodeId,
    workflow: WorkflowState,
    selection: SelectionState,
    canvas: CanvasInteraction,
) {
    selection.clear_pending_drag();
    canvas.clear_drag_anchor();
    let Some((origin, mouse_pos)) = mouse_context(evt) else {
        return;
    };
    canvas.set_origin(origin);
    canvas.update_mouse(mouse_pos);
    canvas.start_connect(node_id, side.to_string());
    selection.select_single(node_id);
    set_temp_edge_from_event(evt, origin, workflow, canvas);
}

fn mouse_context(evt: &MouseEvent) -> Option<((f32, f32), (f32, f32))> {
    let origin = canvas_origin_from_event(evt);
    let page = evt.page_coordinates();
    #[allow(clippy::cast_possible_truncation)]
    let page_point = (page.x as f32, page.y as f32);
    let mouse_pos = crate::ui::interaction_guards::safe_canvas_point(page_point, origin)?;
    Some((origin, mouse_pos))
}

fn canvas_origin_from_event(evt: &MouseEvent) -> (f32, f32) {
    crate::ui::app_io::canvas_origin().map_or_else(
        || {
            let page = evt.page_coordinates();
            let coordinates = evt.element_coordinates();
            #[allow(clippy::cast_possible_truncation)]
            let fallback_x = page.x as f32 - coordinates.x as f32;
            #[allow(clippy::cast_possible_truncation)]
            let fallback_y = page.y as f32 - coordinates.y as f32;
            (fallback_x, fallback_y)
        },
        |origin| origin,
    )
}

fn drag_targets_for_node(selection: SelectionState, node_id: NodeId) -> Vec<NodeId> {
    let currently_selected = selection.selected_ids().read().clone();
    if currently_selected.contains(&node_id) && !currently_selected.is_empty() {
        currently_selected
    } else {
        vec![node_id]
    }
}

fn set_temp_edge_from_event(
    evt: &MouseEvent,
    origin: (f32, f32),
    workflow: WorkflowState,
    canvas: CanvasInteraction,
) {
    let page = evt.page_coordinates();
    #[allow(clippy::cast_possible_truncation)]
    let page_point = (page.x as f32, page.y as f32);
    let current_vp = workflow.viewport().read().clone();
    let Some((canvas_x, canvas_y)) =
        crate::ui::interaction_guards::safe_canvas_from_viewport(page_point, origin, &current_vp)
    else {
        return;
    };
    canvas.set_temp_edge(Some((
        FlowPosition { x: canvas_x, y: canvas_y },
        FlowPosition { x: canvas_x, y: canvas_y },
    )));
}

fn update_node_config(workflow: WorkflowState, node_id: NodeId, new_config: &serde_json::Value) {
    let mut binding = workflow.workflow();
    let mut wf = binding.write();
    if let Some(node) = wf.nodes.iter_mut().find(|node| node.id == node_id) {
        node.apply_config_update(new_config);
    }
}
