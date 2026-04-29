#![deny(clippy::unwrap_used)]
#![deny(clippy::expect_used)]
#![deny(clippy::panic)]
#![warn(clippy::pedantic)]
#![forbid(unsafe_code)]

use crate::hooks::use_canvas_interaction::CanvasInteraction;
use crate::hooks::use_selection::SelectionState;
use crate::hooks::use_ui_panels::UiPanels;
use crate::hooks::use_workflow_state::WorkflowState;
use crate::ui::constants::{
    DEFAULT_CANVAS_HEIGHT, DEFAULT_CANVAS_WIDTH, FIT_VIEW_PADDING, ZOOM_CENTER_X, ZOOM_CENTER_Y,
    ZOOM_DELTA,
};
use crate::ui::{FlowEdges, FlowMinimap, FlowNodeComponent, FlowPosition, ParallelGroupOverlay};
use dioxus::html::input_data::MouseButton;
use dioxus::prelude::*;
#[component]
pub fn CanvasArea(
    workflow: WorkflowState,
    selection: SelectionState,
    canvas: CanvasInteraction,
    panels: UiPanels,
    temp_edge: Memo<Option<(FlowPosition, FlowPosition)>>,
    preview_nodes: Memo<Vec<(String, String, f32, f32)>>,
    preview_edges: Memo<Vec<(String, String)>>,
    show_inspector: Signal<bool>,
) -> Element {
    let nodes = workflow.nodes();
    let connections = workflow.connections();
    let viewport_state = workflow.viewport();
    let vx = viewport_state.read().x;
    let vy = viewport_state.read().y;
    let vz = viewport_state.read().zoom;

    let running_node_ids = use_memo(move || {
        nodes
            .read()
            .iter()
            .filter(|n| matches!(n.execution_state, crate::graph::ExecutionState::Running))
            .map(|n| n.id)
            .collect::<Vec<_>>()
    });

    let zoom = use_memo(move || viewport_state.read().zoom);

    rsx! {
        // Dot grid background
        div {
            class: "absolute inset-0 pointer-events-none",
            style: "background-image: radial-gradient(circle, rgba(100, 116, 139, 0.33) 1px, transparent 1px); background-size: calc(22px * {vz}) calc(22px * {vz}); background-position: {vx}px {vy}px;"
        }

        // Animated gradient shimmer
        div {
            class: "canvas-grid-animated absolute inset-0 pointer-events-none opacity-35",
            style: "background-image: linear-gradient(120deg, rgba(14, 165, 233, 0.08), transparent 45%, rgba(20, 184, 166, 0.08)); background-size: 56px 56px;"
        }

        // Transformed canvas layer
        div {
            class: "absolute origin-top-left",
            style: "transform: translate({vx}px, {vy}px) scale({vz}); will-change: transform;",
            FlowEdges {
                edges: connections,
                nodes: nodes,
                temp_edge: temp_edge,
                running_node_ids: running_node_ids,
                zoom: zoom,
            }

            ParallelGroupOverlay {
                nodes: nodes,
                connections: connections,
            }

            if !preview_edges.read().is_empty() {
                svg {
                    class: "absolute inset-0 overflow-visible pointer-events-none w-full h-full z-0",
                    for (preview_edge_id, preview_path) in preview_edges.read().iter() {
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

            for (preview_node_id, preview_node_type, preview_x, preview_y) in preview_nodes.read().iter() {
                div {
                    key: "{preview_node_id}",
                    class: "pointer-events-none absolute w-[220px] z-0 rounded-xl border border-indigo-300/70 bg-indigo-500/10 px-3 py-2",
                    style: "left: {preview_x}px; top: {preview_y}px;",
                    div { class: "text-[11px] font-semibold text-indigo-700", "Preview" }
                    div { class: "text-[10px] font-mono text-indigo-600", "{preview_node_type}" }
                }
            }

            for node in nodes.read().iter().cloned() {
                 {
                     let node_id = node.id;
                     let is_selected = selection.is_selected(node_id);
                     let is_inline_open = panels.is_inline_panel_open(node_id);
                     let workflow_clone = workflow;
                     let selection_clone = selection;
                     let canvas_clone = canvas;
                     let panels_clone = panels;
                     let mut show_inspector_clone = show_inspector;

                     rsx! {
                         FlowNodeComponent {
                             key: "{node_id}",
                             node,
                             selected: is_selected,
                             inline_open: is_inline_open,
                             on_mouse_down: move |evt: MouseEvent| {
                                 if evt.trigger_button() != Some(MouseButton::Primary) {
                                     return;
                                 }
                                  if canvas_clone.is_space_hand_active() {
                                     return;
                                 }
                                 evt.stop_propagation();

                                 let page = evt.page_coordinates();
                                 let origin = if let Some(origin) = crate::ui::app_io::canvas_origin() {
                                     origin
                                 } else {
                                     let coordinates = evt.element_coordinates();
                                     #[allow(clippy::cast_possible_truncation)]
                                     let fallback_x = page.x as f32 - coordinates.x as f32;
                                     #[allow(clippy::cast_possible_truncation)]
                                     let fallback_y = page.y as f32 - coordinates.y as f32;
                                     (fallback_x, fallback_y)
                                 };
                                 canvas_clone.set_origin(origin);
                                 #[allow(clippy::cast_possible_truncation)]
                                 let page_point = (page.x as f32, page.y as f32);
                                 let Some(mouse_pos) = crate::ui::interaction_guards::safe_canvas_point(page_point, origin) else {
                                     return;
                                 };
                                 canvas_clone.update_mouse(mouse_pos);

                                 let currently_selected = selection_clone.selected_ids().read().clone();
                                 let drag_targets = if currently_selected.contains(&node_id) {
                                     if currently_selected.is_empty() {
                                         vec![node_id]
                                     } else {
                                         currently_selected
                                     }
                                 } else {
                                     vec![node_id]
                                 };
                                 selection_clone.set_multiple(drag_targets.clone());
                                 selection_clone.set_pending_drag(drag_targets);
                                 canvas_clone.start_drag_anchor(mouse_pos);
                             },
                             on_click: move |_| {
                                 selection_clone.select_single(node_id);
                                 show_inspector_clone.set(true);
                             },
                             on_double_click: move |_| {
                                 panels_clone.toggle_inline_panel(node_id);
                             },
                             on_handle_mouse_down: move |args: (MouseEvent, String)| {
                                 let (evt, handle_type) = args;
                                 selection_clone.clear_pending_drag();
                                 canvas_clone.clear_drag_anchor();
                                 let page = evt.page_coordinates();
                                 let origin = if let Some(origin) = crate::ui::app_io::canvas_origin() {
                                     origin
                                 } else {
                                     let coordinates = evt.element_coordinates();
                                     #[allow(clippy::cast_possible_truncation)]
                                     let fallback_x = page.x as f32 - coordinates.x as f32;
                                     #[allow(clippy::cast_possible_truncation)]
                                     let fallback_y = page.y as f32 - coordinates.y as f32;
                                     (fallback_x, fallback_y)
                                 };
                                 canvas_clone.set_origin(origin);
                                 #[allow(clippy::cast_possible_truncation)]
                                 let page_point = (page.x as f32, page.y as f32);
                                 let Some(mouse_pos) = crate::ui::interaction_guards::safe_canvas_point(page_point, origin) else {
                                     return;
                                 };
                                 canvas_clone.update_mouse(mouse_pos);
                                 canvas_clone.start_connect(node_id, handle_type.clone());
                                 selection_clone.select_single(node_id);
                                 let current_vp = workflow_clone.viewport().read().clone();
                                 #[allow(clippy::cast_possible_truncation)]
                                 let page_point = (page.x as f32, page.y as f32);
                                 if let Some((canvas_x, canvas_y)) =
                                     crate::ui::interaction_guards::safe_canvas_from_viewport(
                                         page_point,
                                         origin,
                                         &current_vp,
                                     )
                                 {
                                     canvas_clone.set_temp_edge(Some((
                                         FlowPosition { x: canvas_x, y: canvas_y },
                                         FlowPosition { x: canvas_x, y: canvas_y },
                                     )));
                                 }
                             },
                             on_handle_mouse_enter: move |handle_type| canvas_clone.set_hovered_handle(Some((node_id, handle_type))),
                             on_handle_mouse_leave: move |()| canvas_clone.set_hovered_handle(None),
                              on_inline_change: move |new_config| {
                                  let mut binding = workflow_clone.workflow();
                                  let mut wf = binding.write();
                                  if let Some(n) = wf.nodes.iter_mut().find(|n| n.id == node_id) {
                                      n.apply_config_update(&new_config);
                                  }
                              },
                             on_inline_close: move |()| {
                                 panels_clone.close_inline_panel();
                             }
                         }
                     }
                 }
             }
         }

        // Marquee selection rectangle
        if let Some((start, end)) = canvas.marquee_rect() {
            {
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
        }

        FlowMinimap {
            nodes: nodes,
            edges: connections,
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
