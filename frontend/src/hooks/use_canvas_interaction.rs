#![deny(clippy::unwrap_used)]
#![deny(clippy::expect_used)]
#![deny(clippy::panic)]

use crate::graph::NodeId;
use crate::interaction_mode::{cursor_class_for, drag_mode_from_selection, update_marquee_mode};
use crate::ui::edges::Position as FlowPosition;
use dioxus::prelude::*;

// Re-export all interaction-mode types so the public API is unchanged.
pub use crate::interaction_mode::{
    CanvasPoint, CursorTool, DragAnchor, HandleName, HoveredHandle, InteractionMode, TempEdge,
};

#[derive(Clone, Copy, PartialEq)]
pub struct CanvasInteraction {
    mode: Signal<InteractionMode>,
    cursor_tool: Signal<CursorTool>,
    mouse_pos: Signal<CanvasPoint>,
    canvas_origin: Signal<CanvasPoint>,
    temp_edge: Signal<TempEdge>,
    hovered_handle: Signal<HoveredHandle>,
    drag_anchor: Signal<DragAnchor>,
}

#[allow(dead_code)]
impl CanvasInteraction {
    #[must_use]
    pub fn mode(&self) -> ReadSignal<InteractionMode> {
        self.mode.into()
    }

    #[must_use]
    pub fn cursor_tool(&self) -> ReadSignal<CursorTool> {
        self.cursor_tool.into()
    }

    #[must_use]
    pub fn mouse_pos(&self) -> ReadSignal<CanvasPoint> {
        self.mouse_pos.into()
    }

    #[must_use]
    pub fn canvas_origin(&self) -> ReadSignal<CanvasPoint> {
        self.canvas_origin.into()
    }

    #[must_use]
    pub fn temp_edge(&self) -> ReadSignal<TempEdge> {
        self.temp_edge.into()
    }

    #[must_use]
    pub fn hovered_handle(&self) -> ReadSignal<HoveredHandle> {
        self.hovered_handle.into()
    }

    pub fn start_pan(mut self) {
        self.mode.set(InteractionMode::Panning);
    }

    pub fn start_drag(mut self, node_id: NodeId, selected_ids: Vec<NodeId>) {
        let next_mode = drag_mode_from_selection(node_id, selected_ids);
        self.mode.set(next_mode);
    }

    pub fn start_connect(mut self, node_id: NodeId, handle: String) {
        let handle_name = HandleName::new(handle.clone());
        self.hovered_handle.set(HoveredHandle::active(node_id, handle_name));
        self.mode
            .set(InteractionMode::Connecting { from: node_id, handle: HandleName::new(handle) });
    }

    pub fn start_marquee(mut self, pos: (f32, f32)) {
        self.mode.set(InteractionMode::Marquee {
            start: CanvasPoint::from(pos),
            current: CanvasPoint::from(pos),
        });
    }

    pub fn update_marquee(mut self, pos: (f32, f32)) {
        let mode = self.mode.read().clone();
        self.mode.set(update_marquee_mode(&mode, pos));
    }

    pub fn update_mouse(mut self, pos: (f32, f32)) {
        self.mouse_pos.set(CanvasPoint::from(pos));
    }

    pub fn set_origin(mut self, origin: (f32, f32)) {
        self.canvas_origin.set(CanvasPoint::from(origin));
    }

    pub fn set_temp_edge(mut self, positions: Option<(FlowPosition, FlowPosition)>) {
        match positions {
            Some((source, target)) => self.temp_edge.set(TempEdge::active(source, target)),
            None => self.temp_edge.set(TempEdge::None),
        }
    }

    pub fn set_hovered_handle(mut self, handle: Option<(NodeId, String)>) {
        match handle {
            Some((node_id, handle)) => {
                self.hovered_handle.set(HoveredHandle::active(node_id, HandleName::new(handle)));
            }
            None => self.hovered_handle.set(HoveredHandle::None),
        }
    }

    pub fn start_drag_anchor(mut self, pos: (f32, f32)) {
        self.drag_anchor.set(DragAnchor::active(pos.0, pos.1));
    }

    pub fn clear_drag_anchor(mut self) {
        self.drag_anchor.set(DragAnchor::None);
    }

    pub fn enable_space_hand(mut self) {
        self.cursor_tool.set(CursorTool::SpaceHand);
    }

    pub fn disable_space_hand(mut self) {
        self.cursor_tool.set(CursorTool::Select);
    }

    pub fn end_interaction(mut self) {
        self.mode.set(InteractionMode::Idle);
        self.temp_edge.set(TempEdge::None);
        self.hovered_handle.set(HoveredHandle::None);
        self.drag_anchor.set(DragAnchor::None);
    }

    pub fn cancel_interaction(mut self) {
        self.mode.set(InteractionMode::Idle);
        self.temp_edge.set(TempEdge::None);
        self.hovered_handle.set(HoveredHandle::None);
        self.cursor_tool.set(CursorTool::Select);
        self.drag_anchor.set(DragAnchor::None);
    }

    #[must_use]
    pub fn is_dragging(&self) -> bool {
        matches!(*self.mode.read(), InteractionMode::Dragging { .. })
    }

    #[must_use]
    pub fn is_connecting(&self) -> bool {
        matches!(*self.mode.read(), InteractionMode::Connecting { .. })
    }

    #[must_use]
    pub fn is_marquee(&self) -> bool {
        matches!(*self.mode.read(), InteractionMode::Marquee { .. })
    }

    #[must_use]
    pub fn is_panning(&self) -> bool {
        matches!(*self.mode.read(), InteractionMode::Panning)
    }

    #[must_use]
    pub fn is_idle(&self) -> bool {
        matches!(*self.mode.read(), InteractionMode::Idle)
    }

    #[must_use]
    pub fn is_space_hand_active(&self) -> bool {
        *self.cursor_tool.read() == CursorTool::SpaceHand
    }

    #[must_use]
    pub fn cursor_class(&self) -> &'static str {
        let mode = self.mode.read().clone();
        let cursor_tool = *self.cursor_tool.read();
        cursor_class_for(&mode, cursor_tool)
    }

    #[must_use]
    pub fn dragging_node_ids(&self) -> Option<Vec<NodeId>> {
        match &*self.mode.read() {
            InteractionMode::Dragging { node_ids } => Some(node_ids.clone()),
            _ => None,
        }
    }

    #[must_use]
    pub fn drag_anchor(&self) -> Option<(f32, f32)> {
        self.drag_anchor.read().as_point()
    }

    #[must_use]
    pub fn connecting_from(&self) -> Option<(NodeId, String)> {
        match &*self.mode.read() {
            InteractionMode::Connecting { from, handle } => {
                Some((*from, handle.as_str().to_string()))
            }
            _ => None,
        }
    }

    #[must_use]
    pub fn marquee_rect(&self) -> Option<((f32, f32), (f32, f32))> {
        match &*self.mode.read() {
            InteractionMode::Marquee { start, current } => {
                Some(((start.x, start.y), (current.x, current.y)))
            }
            _ => None,
        }
    }
}

pub fn provide_canvas_interaction_context() -> CanvasInteraction {
    let mode = use_signal(InteractionMode::default);
    let cursor_tool = use_signal(CursorTool::default);
    let mouse_pos = use_signal(CanvasPoint::default);
    let canvas_origin = use_signal(CanvasPoint::default);
    let temp_edge = use_signal(TempEdge::default);
    let hovered_handle = use_signal(HoveredHandle::default);
    let drag_anchor = use_signal(DragAnchor::default);

    let state = CanvasInteraction {
        mode,
        cursor_tool,
        mouse_pos,
        canvas_origin,
        temp_edge,
        hovered_handle,
        drag_anchor,
    };
    provide_context(state)
}

#[must_use]
pub fn use_canvas_interaction() -> CanvasInteraction {
    use_context::<CanvasInteraction>()
}
