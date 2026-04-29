#![deny(clippy::unwrap_used)]
#![deny(clippy::expect_used)]
#![deny(clippy::panic)]
#![warn(clippy::pedantic)]
#![forbid(unsafe_code)]

use crate::flow_extender::ExtensionPatchPreview;
use crate::graph::{validate_workflow, ValidationResult};
use crate::ui::constants::{
    DEFAULT_CANVAS_HEIGHT, DEFAULT_CANVAS_WIDTH, FIT_VIEW_PADDING, NODE_HANDLE_Y_OFFSET,
    NODE_WIDTH, ZOOM_CENTER_X, ZOOM_CENTER_Y, ZOOM_DELTA,
};
use crate::ui::{
    CanvasArea, CanvasContextMenu, EmptyCanvas, FlowPosition, FlowToolbar, InspectorPanel,
    NodeCommandPalette, NodeSidebar, NodeTemplateId, PayloadPreviewPanel, PrototypePalette,
    RightPanel, RunStatusBar, SelectedNodePanel, SettingsOverlay, ShortcutsOverlay, ToastContainer,
};
use dioxus::prelude::*;
use std::fmt::Write;

#[component]
pub fn AppShell() -> Element {
    // Hook-based state management
    let mut workflow = crate::hooks::use_workflow_state();
    let selection = crate::hooks::use_selection();
    let canvas = crate::hooks::use_canvas_interaction();
    let panels = crate::hooks::use_ui_panels();
    let sidebar = crate::hooks::use_sidebar();
    let restate = crate::hooks::use_restate_sync();
    let mut toast = crate::hooks::use_toast();

    // Persist workflow to localStorage
    use_effect(move || {
        let wf_signal = workflow.workflow();
        let wf = wf_signal.read();
        if let Ok(_json) = serde_json::to_string(&*wf) {
            #[cfg(target_arch = "wasm32")]
            {
                use web_sys::window;
                let storage = window().and_then(|w| match w.local_storage() {
                    Ok(s) => s,
                    Err(_) => None,
                });
                if let Some(s) = storage {
                    let _ = s.set_item("flow-wasm-v1-workflow", &_json);
                }
            }
        }
    });

    // Derived computations
    let _nodes = workflow.nodes();
    let nodes_by_id = workflow.nodes_by_id();
    let _connections = workflow.connections();
    let node_count = use_memo(move || workflow.nodes().read().len());
    let edge_count = use_memo(move || workflow.connections().read().len());
    let zoom_label = use_memo(move || {
        let mut s = String::with_capacity(16);
        let _ = write!(s, "{:.0}%", workflow.viewport().read().zoom * 100.0);
        s
    });
    let can_undo = use_memo(move || workflow.can_undo());
    let can_redo = use_memo(move || workflow.can_redo());
    let mut extension_previews = use_signal(Vec::<ExtensionPatchPreview>::new);
    let mut validation_collapsed = use_signal(|| false);
    let validation_result: Memo<ValidationResult> = use_memo(move || {
        let binding = workflow.workflow();
        let wf = binding.read();
        validate_workflow(&wf)
    });

    // RunStatusBar signals
    let current_step = use_memo(move || workflow.workflow().read().current_step);
    let total_steps = use_memo(move || workflow.workflow().read().execution_queue.len());
    let current_step_name = use_memo(move || {
        let binding = workflow.workflow();
        let wf = binding.read();
        let queue = &wf.execution_queue;
        let step = wf.current_step;
        queue
            .get(step)
            .and_then(|id| wf.nodes.iter().find(|n| n.id == *id))
            .map_or_else(String::new, |n| n.name.clone())
    });
    let overall_execution_status = use_memo(move || {
        use crate::graph::ExecutionState;
        let binding = workflow.workflow();
        let wf = binding.read();
        let nodes = &wf.nodes;
        if nodes.iter().any(|n| n.execution_state == ExecutionState::Running) {
            ExecutionState::Running
        } else if nodes.iter().any(|n| n.execution_state == ExecutionState::Failed) {
            ExecutionState::Failed
        } else if nodes
            .iter()
            .all(|n| n.execution_state.is_terminal() || n.execution_state == ExecutionState::Idle)
            && nodes.iter().any(|n| n.execution_state == ExecutionState::Completed)
        {
            ExecutionState::Completed
        } else {
            ExecutionState::Idle
        }
    });
    let mut frozen_run_id: Signal<Option<uuid::Uuid>> = use_signal(|| None);
    let is_frozen_mode = use_memo(move || frozen_run_id.read().is_some());
    let frozen_run_id_str =
        use_memo(move || (*frozen_run_id.read()).map(|id| id.to_string()[..8].to_string()));

    // InspectorPanel signals
    let selected_node = use_memo(move || {
        let selected_id = *selection.selected_id().read();
        selected_id.and_then(|id| workflow.nodes_by_id().read().get(&id).cloned())
    });
    let inspector_node = use_memo(move || selected_node.read().clone());
    let inspector_input = use_memo(move || {
        selected_node
            .read()
            .as_ref()
            .and_then(|n| n.config.as_object().cloned().map(serde_json::Value::Object))
    });
    let selected_frozen_run = use_memo(move || {
        let run_id = *frozen_run_id.read();
        run_id.and_then(|id| {
            workflow.workflow().read().history.iter().find(|run| run.id == id).cloned()
        })
    });
    let inspector_output = use_memo(move || {
        let selected = selected_node.read().clone();
        let frozen = selected_frozen_run.read().clone();

        if let (Some(node), Some(run)) = (selected.as_ref(), frozen.as_ref()) {
            return run.results.get(&node.id).cloned();
        }

        selected.and_then(|node| node.last_output)
    });
    let inspector_error = use_memo(move || {
        let selected = selected_node.read().clone();
        let frozen = selected_frozen_run.read().clone();

        if let (Some(node), Some(run)) = (selected.as_ref(), frozen.as_ref()) {
            return run
                .results
                .get(&node.id)
                .and_then(|result| result.get("error"))
                .and_then(serde_json::Value::as_str)
                .map(std::string::ToString::to_string);
        }

        selected.and_then(|node| node.error)
    });
    let mut show_inspector = use_signal(|| false);

    use_effect(move || {
        let has_node = inspector_node.read().is_some();
        if !has_node {
            show_inspector.set(false);
        }
    });

    // PrototypePalette signal
    let mut prototype_open = use_signal(|| false);

    let vp = workflow.viewport();
    let _vx = vp.read().x;
    let _vy = vp.read().y;
    let _vz = vp.read().zoom;

    // Temp edge computation for connecting mode
    let temp_edge = use_memo(move || {
        let mode = canvas.mode().read().clone();
        if let crate::hooks::InteractionMode::Connecting { from, handle } = mode {
            let to = canvas.temp_edge().read().as_positions();
            if let Some((to_pos, _)) = to {
                let node = nodes_by_id.read().get(&from).cloned();
                node.map(|n| {
                    let from_pos = if handle.as_str() == "source" {
                        FlowPosition { x: n.x + NODE_WIDTH, y: n.y + NODE_HANDLE_Y_OFFSET }
                    } else {
                        FlowPosition { x: n.x, y: n.y + NODE_HANDLE_Y_OFFSET }
                    };
                    (from_pos, to_pos)
                })
            } else {
                None
            }
        } else {
            None
        }
    });

    let preview_nodes = use_memo(move || {
        crate::flow_extender::preview_calc::compute_preview_nodes(&extension_previews.read())
    });

    let preview_edges = use_memo(move || {
        let existing_nodes = nodes_by_id.read().clone();
        crate::flow_extender::preview_calc::compute_preview_edges(
            &extension_previews.read(),
            &existing_nodes,
        )
    });

    rsx! {
        document::Stylesheet { href: "/assets/tailwind.css" }
        document::Stylesheet { href: asset!("/style.css") }
        style {
            "@keyframes dash {{ to {{ stroke-dashoffset: -16; }} }}"
            "@keyframes flow {{ to {{ stroke-dashoffset: -16; }} }}"
            "@keyframes slide-in-right {{ from {{ transform: translateX(24px); opacity: 0; }} to {{ transform: translateX(0); opacity: 1; }} }}"
            "@keyframes canvas-shimmer {{ 0% {{ background-position: 0px 0px; }} 100% {{ background-position: 56px 56px; }} }}"
            ".animate-slide-in-right {{ animation: slide-in-right 0.22s ease-out; }}"
            ".canvas-grid-animated {{ animation: canvas-shimmer 24s linear infinite; }}"
            "@media (prefers-reduced-motion: reduce) {{ .edge-animated {{ animation: none !important; }} }}"
            "@media (prefers-reduced-motion: reduce) {{ .canvas-grid-animated {{ animation: none !important; }} }}"
        }

        div { class: "relative flex h-screen w-screen flex-col overflow-hidden bg-[#f2f7fa] text-slate-900 [font-family:'Geist',_'Manrope',sans-serif] select-none",
            ToastContainer { store: toast }

            FlowToolbar {
                workflow_name: workflow.workflow_name(),
                on_workflow_name_change: move |value| workflow.workflow_name().set(value),
                node_count: node_count,
                edge_count: edge_count,
                zoom_label: zoom_label,
                can_undo: can_undo,
                can_redo: can_redo,
                on_zoom_in: move |_| workflow.zoom(ZOOM_DELTA, ZOOM_CENTER_X, ZOOM_CENTER_Y),
                on_zoom_out: move |_| workflow.zoom(-ZOOM_DELTA, ZOOM_CENTER_X, ZOOM_CENTER_Y),
                on_fit_view: move |_| workflow.fit_view(DEFAULT_CANVAS_WIDTH, DEFAULT_CANVAS_HEIGHT, FIT_VIEW_PADDING),
                on_layout: move |_| workflow.apply_layout(),
                on_execute: move |_| {
                    let result = validation_result.read();
                    if result.has_errors() {
                        validation_collapsed.set(false);
                    } else {
                        let ingress = restate.ingress_url.read().clone();
                        workflow.run(ingress);
                    }
                },
                on_undo: move |_| {
                    let _ = workflow.undo();
                    extension_previews.set(Vec::new());
                    selection.clear();
                },
                on_redo: move |_| {
                    let _ = workflow.redo();
                    extension_previews.set(Vec::new());
                    selection.clear();
                },
                on_save: move |_| {
                    #[cfg(target_arch = "wasm32")]
                    {
                        crate::ui::app_io::download_workflow_json(
                            &workflow.workflow_name().read(),
                            &workflow.workflow().read(),
                        );
                        toast.push("Workflow saved".to_string(), crate::ui::toast::ToastSeverity::Success);
                    }
                },
                on_import: move |_| {
                    #[cfg(target_arch = "wasm32")]
                    {
                        let mut toast_clone = toast;
                        crate::ui::app_io::trigger_import(move |result| {
                            match result {
                                crate::ui::app_io::ImportResult::Success(imported) => {
                                    workflow.load_workflow(imported);
                                    toast_clone.push("Workflow imported".to_string(), crate::ui::toast::ToastSeverity::Success);
                                }
                                crate::ui::app_io::ImportResult::Error(msg) => {
                                    toast_clone.push(format!("Import failed: {msg}"), crate::ui::toast::ToastSeverity::Error);
                                }
                            }
                        });
                    }
                },
                on_settings: move |_| panels.toggle_settings()
            }

            RunStatusBar {
                current_step: ReadSignal::from(current_step),
                total_steps: ReadSignal::from(total_steps),
                current_step_name: ReadSignal::from(current_step_name),
                overall_status: ReadSignal::from(overall_execution_status),
                is_frozen_mode: ReadSignal::from(is_frozen_mode),
                frozen_run_id: ReadSignal::from(frozen_run_id_str),
                on_exit_frozen: move |()| { frozen_run_id.set(None); }
            }

            SettingsOverlay { panels: panels }

            if panels.shortcuts_open() {
                ShortcutsOverlay {
                    on_close: move |_| panels.close_shortcuts(),
                }
            }

            NodeCommandPalette {
                open: panels.palette_open(),
                query: panels.palette_query(),
                on_query_change: move |value| panels.set_palette_query(value),
                on_close: move |()| panels.close_palette(),
                on_pick: move |node_type: NodeTemplateId| {
                    let (canvas_w, canvas_h) = crate::ui::app_io::canvas_rect_size()
                        .map_or((DEFAULT_CANVAS_WIDTH, DEFAULT_CANVAS_HEIGHT), std::convert::identity);
                    let _ = workflow.add_node_at_viewport_center_with_canvas(node_type.as_str(), canvas_w, canvas_h);
                    panels.close_palette();
                }
            }

            PrototypePalette {
                open: ReadSignal::from(use_memo(move || *prototype_open.read())),
                on_close: move |()| prototype_open.set(false),
                on_add_node: move |node_type: NodeTemplateId| {
                    let (canvas_w, canvas_h) = crate::ui::app_io::canvas_rect_size()
                        .map_or((DEFAULT_CANVAS_WIDTH, DEFAULT_CANVAS_HEIGHT), std::convert::identity);
                    let _ = workflow.add_node_at_viewport_center_with_canvas(node_type.as_str(), canvas_w, canvas_h);
                    prototype_open.set(false);
                }
            }

            CanvasContextMenu {
                open: ReadSignal::from(use_memo(move || panels.context_menu().read().is_visible())),
                x: ReadSignal::from(use_memo(move || panels.context_menu().read().position().map_or(0.0, |p| p.x))),
                y: ReadSignal::from(use_memo(move || panels.context_menu().read().position().map_or(0.0, |p| p.y))),
                on_close: move |_| panels.close_context_menu(),
                on_add_node: move |_| {
                    panels.close_context_menu();
                    panels.open_palette();
                },
                on_fit_view: move |_| {
                    panels.close_context_menu();
                    workflow.fit_view(DEFAULT_CANVAS_WIDTH, DEFAULT_CANVAS_HEIGHT, FIT_VIEW_PADDING);
                },
                on_layout: move |_| {
                    panels.close_context_menu();
                    workflow.apply_layout();
                }
            }

            div { class: "flex flex-1 overflow-hidden",
                NodeSidebar {
                    search: ReadSignal::from(use_memo(move || sidebar.search().read().as_str().to_string())),
                    on_search_change: move |value| sidebar.set_search(value),
                    on_pickup_node: move |node_type: &'static str| {
                        sidebar.pickup_node(node_type);
                    },
                    on_add_node: move |node_type: &'static str| {
                        sidebar.clear_pending_drop();
                        let (canvas_w, canvas_h) = crate::ui::app_io::canvas_rect_size()
                            .map_or((DEFAULT_CANVAS_WIDTH, DEFAULT_CANVAS_HEIGHT), std::convert::identity);
                        let _ = workflow.add_node_at_viewport_center_with_canvas(node_type, canvas_w, canvas_h);
                    }
                }

                main {
                    class: "relative flex-1 overflow-hidden bg-gradient-to-br from-slate-50 via-cyan-50/40 to-sky-100/40 {canvas.cursor_class()}",
                    tabindex: "0",
                    onmouseenter: move |evt| {
                        crate::hooks::use_canvas_mouse::handle_canvas_mouseenter_event(&evt, canvas);
                    },
                    oncontextmenu: move |evt| {
                        evt.prevent_default();
                        canvas.cancel_interaction();
                        selection.clear_pending_drag();
                        canvas.clear_drag_anchor();
                        let coordinates = evt.page_coordinates();
                        #[allow(clippy::cast_possible_truncation)]
                        let cx = coordinates.x as f32;
                        #[allow(clippy::cast_possible_truncation)]
                        let cy = coordinates.y as f32;
                        if cx.is_finite() && cy.is_finite() {
                            panels.show_context_menu(cx, cy);
                        }
                    },
                    onkeydown: move |evt| {
                        let key = evt.key().to_string().to_lowercase();
                        crate::hooks::use_canvas_events::handle_canvas_keydown(
                            &key,
                            &evt,
                            &panels,
                            canvas,
                            selection,
                            &workflow,
                            &mut extension_previews,
                        );
                    },
                    onkeyup: move |evt| {
                        let key = evt.key().to_string().to_lowercase();
                        if key == " " || key == "space" {
                            evt.prevent_default();
                            canvas.disable_space_hand();
                            canvas.end_interaction();
                            selection.clear_pending_drag();
                            canvas.clear_drag_anchor();
                        }
                    },
                    onwheel: move |evt| {
                        crate::hooks::use_canvas_mouse::handle_canvas_wheel_event(&evt, canvas, &workflow);
                    },
                    onmousemove: move |evt| {
                        crate::hooks::use_canvas_mouse::handle_canvas_mousemove_event(
                            &evt, canvas, selection, sidebar, &workflow,
                        );
                    },
                    onmouseup: move |evt| {
                        crate::hooks::use_canvas_mouse::handle_canvas_mouseup_event(
                            &evt, canvas, selection, sidebar, &workflow,
                        );
                    },
                    onmouseleave: move |_| {
                        crate::hooks::use_canvas_mouse::handle_canvas_mouseleave_event(
                            canvas, sidebar, selection,
                        );
                    },
                    onmousedown: move |evt| {
                        crate::hooks::use_canvas_mouse::handle_canvas_mousedown_event(
                            &evt, &panels, canvas, selection, sidebar,
                        );
                    },

                    CanvasArea {
                        workflow: workflow,
                        selection: selection,
                        canvas: canvas,
                        panels: panels,
                        temp_edge: temp_edge,
                        preview_nodes: preview_nodes,
                        preview_edges: preview_edges,
                        show_inspector: show_inspector,
                    }

                    if *node_count.read() == 0 {
                        EmptyCanvas {
                            on_add_node: move |_| panels.open_palette(),
                            on_import: move |_| {
                                #[cfg(target_arch = "wasm32")]
                                {
                                    let mut toast_clone = toast;
                                    crate::ui::app_io::trigger_import(move |result| {
                                        match result {
                                            crate::ui::app_io::ImportResult::Success(imported) => {
                                                workflow.load_workflow(imported);
                                                toast_clone.push("Workflow imported".to_string(), crate::ui::toast::ToastSeverity::Success);
                                            }
                                            crate::ui::app_io::ImportResult::Error(msg) => {
                                                toast_clone.push(format!("Import failed: {msg}"), crate::ui::toast::ToastSeverity::Error);
                                            }
                                        }
                                    });
                                }
                            },
                        }
                    }
                }

                RightPanel {
                    workflow: workflow,
                    validation_result: validation_result,
                    validation_collapsed: validation_collapsed,
                    frozen_run_id: frozen_run_id,
                    on_select_node: move |node_id| {
                        selection.select_single(node_id);
                    },
                    restate: restate,
                }

                SelectedNodePanel {
                    selection: selection,
                    nodes_by_id,
                    workflow_state: workflow,
                    preview_patches: extension_previews,
                }

                PayloadPreviewPanel {
                    on_close: move |_| selection.clear(),
                }
            }

            if *show_inspector.read() {
                InspectorPanel {
                    node: ReadSignal::from(inspector_node),
                    step_input: ReadSignal::from(inspector_input),
                    step_output: ReadSignal::from(inspector_output),
                    step_error: ReadSignal::from(inspector_error),
                    step_stack_trace: ReadSignal::from(use_memo(move || None::<String>)),
                    step_start_time: ReadSignal::from(use_memo(move || None::<String>)),
                    step_end_time: ReadSignal::from(use_memo(move || None::<String>)),
                    step_duration_ms: ReadSignal::from(use_memo(move || None::<i64>)),
                    step_attempt: ReadSignal::from(use_memo(move || 1u32)),
                    right_offset_px: if selection.selected_id().read().is_some() { 320 } else { 0 },
                    on_close: move |()| { show_inspector.set(false); }
                }
            }
        }
    }
}
