#![deny(clippy::unwrap_used)]
#![deny(clippy::expect_used)]
#![deny(clippy::panic)]
#![allow(clippy::cast_possible_truncation)]
#![allow(clippy::too_many_lines)]
#![allow(clippy::cognitive_complexity)]
#![forbid(unsafe_code)]

use crate::graph::PortName;
use crate::hooks::use_canvas_interaction::CanvasInteraction;
use crate::hooks::use_selection::SelectionState;
use crate::hooks::use_sidebar::SidebarState;
use crate::hooks::use_ui_panels::UiPanels;
use crate::hooks::use_workflow_state::WorkflowState;
use crate::ui::constants::{
    EDGE_AUTO_PAN_MAX, EDGE_AUTO_PAN_ZONE, FALLBACK_CANVAS_HEIGHT, FALLBACK_CANVAS_WIDTH,
    NODE_CENTER_X_OFFSET, NODE_HANDLE_Y_OFFSET,
};
use crate::ui::edges::Position as FlowPosition;
use dioxus::html::input_data::MouseButton;
use dioxus::prelude::*;

/// Re-export for backward compatibility.
pub use crate::ui::constants::DRAG_THRESHOLD_PX;

// ---------------------------------------------------------------------------
// onmouseenter
// ---------------------------------------------------------------------------

/// Handle canvas `onmouseenter`.
///
/// Resolves and stores the canvas origin so subsequent handlers can
/// compute mouse-relative positions. Falls back to computing the origin
/// from page and element coordinates if `app_io::canvas_origin()` returns
/// `None` (non-WASM targets).
pub fn handle_canvas_mouseenter_event(evt: &MouseEvent, canvas: CanvasInteraction) {
    let page = evt.page_coordinates();
    let origin = if let Some(origin) = crate::ui::app_io::canvas_origin() {
        origin
    } else {
        let element = evt.element_coordinates();
        let fallback_x = page.x as f32 - element.x as f32;
        let fallback_y = page.y as f32 - element.y as f32;
        (fallback_x, fallback_y)
    };
    canvas.set_origin(origin);
}

// ---------------------------------------------------------------------------
// onwheel
// ---------------------------------------------------------------------------

/// Handle canvas `onwheel` for zoom-to-cursor behavior.
///
/// Computes the zoom delta from wheel scroll and applies it at the
/// cursor position so the point under the cursor stays fixed.
pub fn handle_canvas_wheel_event(
    evt: &WheelEvent,
    canvas: CanvasInteraction,
    workflow: &WorkflowState,
) {
    evt.prevent_default();
    let page = evt.page_coordinates();
    let origin = *canvas.canvas_origin().read();
    let origin_x = origin.x;
    let origin_y = origin.y;
    let delta = -evt.delta().strip_units().y as f32 * 0.001;
    let zoom_x = page.x as f32 - origin_x;
    let zoom_y = page.y as f32 - origin_y;
    if delta.is_finite() && zoom_x.is_finite() && zoom_y.is_finite() {
        (*workflow).zoom(delta, zoom_x, zoom_y);
    }
}

// ---------------------------------------------------------------------------
// onmouseleave
// ---------------------------------------------------------------------------

/// Handle canvas `onmouseleave`.
///
/// Cancels any ongoing interaction unless actively dragging, panning,
/// marquee-selecting, or connecting -- in those cases we want the
/// interaction to continue even if the cursor leaves the canvas element.
pub fn handle_canvas_mouseleave_event(
    canvas: CanvasInteraction,
    sidebar: SidebarState,
    selection: SelectionState,
) {
    if canvas.is_dragging() || canvas.is_panning() || canvas.is_marquee() || canvas.is_connecting()
    {
        return;
    }
    canvas.cancel_interaction();
    sidebar.clear_pending_drop();
    selection.clear_pending_drag();
}

// ---------------------------------------------------------------------------
// onmousedown
// ---------------------------------------------------------------------------

/// Handle canvas `onmousedown`.
///
/// Routes the gesture based on the trigger button:
/// - Middle button or primary + space-hand: start panning
/// - Primary (no pending sidebar drop): start marquee selection
/// - Other buttons: clear selection
///
/// Also resolves and stores the canvas origin and current mouse position.
pub fn handle_canvas_mousedown_event(
    evt: &MouseEvent,
    panels: &UiPanels,
    canvas: CanvasInteraction,
    selection: SelectionState,
    sidebar: SidebarState,
) {
    panels.close_context_menu();
    panels.close_inline_panel();
    let trigger_button = evt.trigger_button();
    if matches!(trigger_button, Some(MouseButton::Primary | MouseButton::Auxiliary)) {
        evt.prevent_default();
        selection.clear_pending_drag();
        canvas.clear_drag_anchor();
        let page = evt.page_coordinates();
        let origin = if let Some(origin) = crate::ui::app_io::canvas_origin() {
            origin
        } else {
            let coordinates = evt.element_coordinates();
            let fallback_x = page.x as f32 - coordinates.x as f32;
            let fallback_y = page.y as f32 - coordinates.y as f32;
            (fallback_x, fallback_y)
        };
        canvas.set_origin(origin);
        let page_point = (page.x as f32, page.y as f32);
        let Some(mouse_pos) = crate::ui::interaction_guards::safe_canvas_point(page_point, origin)
        else {
            return;
        };
        canvas.update_mouse(mouse_pos);

        let has_pending_drop = sidebar.has_pending_drop();
        if matches!(trigger_button, Some(MouseButton::Auxiliary))
            || (matches!(trigger_button, Some(MouseButton::Primary))
                && canvas.is_space_hand_active())
        {
            canvas.start_pan();
        } else if matches!(trigger_button, Some(MouseButton::Primary)) && !has_pending_drop {
            canvas.start_marquee(mouse_pos);
        }
    } else {
        selection.clear();
    }
}

// ---------------------------------------------------------------------------
// onmousemove
// ---------------------------------------------------------------------------

/// Handle canvas `onmousemove`.
///
/// Routes mouse movement based on the current interaction mode:
/// - **Idle with drag anchor**: Checks drag threshold, then starts dragging
/// - **Dragging**: Moves selected nodes with edge-auto-panning
/// - **Connecting**: Snaps to handles or shows temp edge
/// - **Marquee**: Updates selection rectangle
/// - **Panning**: Pans the viewport
pub fn handle_canvas_mousemove_event(
    evt: &MouseEvent,
    canvas: CanvasInteraction,
    selection: SelectionState,
    _sidebar: SidebarState,
    workflow: &WorkflowState,
) {
    let page = evt.page_coordinates();
    let origin = *canvas.canvas_origin().read();
    let origin_x = origin.x;
    let origin_y = origin.y;
    let (mx, my) = (page.x as f32 - origin_x, page.y as f32 - origin_y);
    if !mx.is_finite() || !my.is_finite() {
        return;
    }
    let mouse_pos = *canvas.mouse_pos().read();
    let lx = mouse_pos.x;
    let ly = mouse_pos.y;
    let dx = mx - lx;
    let dy = my - ly;
    canvas.update_mouse((mx, my));

    if canvas.drag_anchor().is_some()
        && !canvas.is_dragging()
        && (canvas.is_panning() || canvas.is_marquee() || canvas.is_connecting())
    {
        canvas.clear_drag_anchor();
        selection.clear_pending_drag();
    }

    let current_vp = workflow.viewport().read().clone();
    let zoom = current_vp.zoom;
    if !crate::ui::interaction_guards::is_valid_zoom(zoom) {
        return;
    }

    if canvas.drag_anchor().is_some() && !canvas.is_dragging() {
        if let Some((ax, ay)) = canvas.drag_anchor() {
            let moved = (mx - ax).hypot(my - ay);
            if moved >= DRAG_THRESHOLD_PX {
                if let Some(node_ids) = selection.take_pending_drag() {
                    if let Some(primary_id) = node_ids.first().copied() {
                        canvas.start_drag(primary_id, node_ids);
                    }
                }
                canvas.clear_drag_anchor();
            }
        }
    }

    if canvas.is_dragging() {
        let (canvas_w, canvas_h) = crate::ui::app_io::canvas_rect_size()
            .map_or((FALLBACK_CANVAS_WIDTH, FALLBACK_CANVAS_HEIGHT), std::convert::identity);
        let edge = EDGE_AUTO_PAN_ZONE;
        let max_pan = EDGE_AUTO_PAN_MAX;

        let pan_x = if mx < edge {
            ((edge - mx) / edge).clamp(0.0, 1.0) * max_pan
        } else if mx > canvas_w - edge {
            -(((mx - (canvas_w - edge)) / edge).clamp(0.0, 1.0) * max_pan)
        } else {
            0.0
        };

        let pan_y = if my < edge {
            ((edge - my) / edge).clamp(0.0, 1.0) * max_pan
        } else if my > canvas_h - edge {
            -(((my - (canvas_h - edge)) / edge).clamp(0.0, 1.0) * max_pan)
        } else {
            0.0
        };

        if pan_x != 0.0 || pan_y != 0.0 {
            workflow.pan(pan_x, pan_y);
        }

        let offset_x = (dx - pan_x) / zoom;
        let offset_y = (dy - pan_y) / zoom;
        if let Some(node_ids) = canvas.dragging_node_ids() {
            for node_id in node_ids {
                workflow.update_node_position(node_id, offset_x, offset_y);
            }
        }
    } else if canvas.is_connecting() {
        let canvas_x = (mx - current_vp.x) / zoom;
        let canvas_y = (my - current_vp.y) / zoom;

        if let Some((source_id, source_kind)) = canvas.connecting_from() {
            let node_list = workflow.nodes().read().clone();
            let snapped =
                crate::ui::editor_interactions::snap_handle(&node_list, mx, my, &current_vp)
                    .filter(|(node_id, handle_kind, _)| {
                        *node_id != source_id && *handle_kind != source_kind
                    });

            if let Some((node_id, handle_kind, snapped_pos)) = snapped {
                canvas.set_hovered_handle(Some((node_id, handle_kind)));
                canvas.set_temp_edge(Some((snapped_pos, snapped_pos)));
            } else {
                canvas.set_hovered_handle(None);
                canvas.set_temp_edge(Some((
                    FlowPosition { x: canvas_x, y: canvas_y },
                    FlowPosition { x: canvas_x, y: canvas_y },
                )));
            }
        }
    } else if canvas.is_marquee() {
        if let Some((start, _)) = canvas.marquee_rect() {
            canvas.update_marquee((mx, my));
            let start_canvas = ((start.0 - current_vp.x) / zoom, (start.1 - current_vp.y) / zoom);
            let end_canvas = ((mx - current_vp.x) / zoom, (my - current_vp.y) / zoom);
            let rect = crate::ui::editor_interactions::normalize_rect(start_canvas, end_canvas);
            let selected = workflow
                .nodes()
                .read()
                .iter()
                .filter(|node| {
                    crate::ui::editor_interactions::node_intersects_rect(node.x, node.y, rect)
                })
                .map(|node| node.id)
                .collect::<Vec<_>>();
            selection.set_multiple(selected);
        }
    } else if canvas.is_panning() {
        workflow.pan(dx, dy);
    }
}

// ---------------------------------------------------------------------------
// onmouseup
// ---------------------------------------------------------------------------

/// Handle canvas `onmouseup`.
///
/// Finalizes the current interaction:
/// - **Connecting**: Creates a connection if snapped to a valid handle
/// - **Marquee click**: Clears selection if it was a tiny click
/// - **Sidebar drop**: Places a new node at cursor position
/// - Always ends interaction and clears pending states
pub fn handle_canvas_mouseup_event(
    evt: &MouseEvent,
    canvas: CanvasInteraction,
    selection: SelectionState,
    sidebar: SidebarState,
    workflow: &WorkflowState,
) {
    let from = canvas.connecting_from();
    let over = canvas.hovered_handle().read().as_tuple();
    let is_dragging = canvas.is_dragging();
    let pending_drop = sidebar.pending_drop();
    let mut should_clear_selection = false;

    if let Some((start, end)) = canvas.marquee_rect() {
        let rect = crate::ui::editor_interactions::normalize_rect(start, end);
        let tiny_click = (rect.2 - rect.0).abs() < 2.0 && (rect.3 - rect.1).abs() < 2.0;
        if tiny_click && crate::ui::editor_interactions::rect_contains(rect, start) && !is_dragging
        {
            should_clear_selection = true;
        }
    }

    if let (Some((src_id, src_handle)), Some((tgt_id, _))) = (from, over) {
        if src_id != tgt_id {
            let (source, target) =
                if src_handle == "source" { (src_id, tgt_id) } else { (tgt_id, src_id) };

            let _ = workflow.add_connection(
                source,
                target,
                &PortName("main".to_string()),
                &PortName("main".to_string()),
            );
        }
    } else if !is_dragging && !canvas.is_marquee() {
        if let Some(node_type) = pending_drop {
            let coords = evt.page_coordinates();
            let origin = *canvas.canvas_origin().read();
            let origin_x = origin.x;
            let origin_y = origin.y;
            let mx = coords.x as f32 - origin_x;
            let my = coords.y as f32 - origin_y;
            if mx.is_finite() && my.is_finite() {
                let current_vp = workflow.viewport().read().clone();
                if crate::ui::interaction_guards::is_valid_zoom(current_vp.zoom) {
                    let canvas_x = (mx - current_vp.x) / current_vp.zoom - NODE_CENTER_X_OFFSET;
                    let canvas_y = (my - current_vp.y) / current_vp.zoom - NODE_HANDLE_Y_OFFSET;
                    let _ = workflow.add_node(node_type.as_str(), canvas_x, canvas_y);
                }
            }
        }
    }

    canvas.end_interaction();
    sidebar.clear_pending_drop();
    selection.clear_pending_drag();

    if should_clear_selection {
        selection.clear();
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used, clippy::panic, clippy::float_cmp)]
mod tests {
    #[test]
    fn given_source_file_when_checking_for_unwrap_then_none_found() {
        let source = include_str!("use_canvas_mouse.rs");
        let needle_unwrap = concat!(".", "unwrap", "()");
        assert!(!source.contains(needle_unwrap), "Module must not contain unwrap calls");
        let needle_expect = concat!(".", "expect", "(");
        assert!(!source.contains(needle_expect), "Module must not contain expect calls");
    }
}
