#![deny(clippy::unwrap_used)]
#![deny(clippy::expect_used)]
#![deny(clippy::panic)]

use crate::graph::NodeId;

// ---------------------------------------------------------------------------
// InteractionMode — the core state-machine variant for canvas interaction
// ---------------------------------------------------------------------------

#[derive(Clone, Debug, Default, PartialEq)]
pub enum InteractionMode {
    #[default]
    Idle,
    Panning,
    Dragging {
        node_ids: Vec<NodeId>,
    },
    Connecting {
        from: NodeId,
        handle: HandleName,
    },
    Marquee {
        start: CanvasPoint,
        current: CanvasPoint,
    },
}

impl InteractionMode {
    /// Returns `true` when the mode is `Dragging`.
    #[must_use]
    pub fn is_dragging(&self) -> bool {
        matches!(self, Self::Dragging { .. })
    }

    /// Returns `true` when the mode is `Connecting`.
    #[must_use]
    pub fn is_connecting(&self) -> bool {
        matches!(self, Self::Connecting { .. })
    }

    /// Returns `true` when the mode is `Marquee`.
    #[must_use]
    pub fn is_marquee(&self) -> bool {
        matches!(self, Self::Marquee { .. })
    }

    /// Returns `true` when the mode is `Panning`.
    #[must_use]
    pub fn is_panning(&self) -> bool {
        matches!(self, Self::Panning)
    }

    /// Returns `true` when the mode is `Idle`.
    #[must_use]
    pub fn is_idle(&self) -> bool {
        matches!(self, Self::Idle)
    }
}

// ---------------------------------------------------------------------------
// CanvasPoint — 2-D point on the canvas coordinate space
// ---------------------------------------------------------------------------

#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct CanvasPoint {
    pub x: f32,
    pub y: f32,
}

impl From<(f32, f32)> for CanvasPoint {
    fn from((x, y): (f32, f32)) -> Self {
        Self { x, y }
    }
}

impl From<CanvasPoint> for (f32, f32) {
    fn from(point: CanvasPoint) -> Self {
        (point.x, point.y)
    }
}

// ---------------------------------------------------------------------------
// HandleName — newtype for node handle identifiers
// ---------------------------------------------------------------------------

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct HandleName(String);

impl HandleName {
    pub fn new(name: impl Into<String>) -> Self {
        Self(name.into())
    }

    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl From<String> for HandleName {
    fn from(s: String) -> Self {
        Self::new(s)
    }
}

impl From<&str> for HandleName {
    fn from(s: &str) -> Self {
        Self::new(s)
    }
}

// ---------------------------------------------------------------------------
// CursorTool — keyboard modifier tool state
// ---------------------------------------------------------------------------

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum CursorTool {
    #[default]
    Select,
    SpaceHand,
}

// ---------------------------------------------------------------------------
// DragAnchor — anchor position for drag calculations
// ---------------------------------------------------------------------------

#[derive(Clone, Debug, Default, PartialEq)]
pub enum DragAnchor {
    #[default]
    None,
    Active {
        x: f32,
        y: f32,
    },
}

impl DragAnchor {
    #[must_use]
    pub fn active(x: f32, y: f32) -> Self {
        Self::Active { x, y }
    }

    // Test-only: explicit constructor for None variant
    #[allow(dead_code)]
    #[must_use]
    pub fn none() -> Self {
        Self::None
    }

    #[must_use]
    pub fn as_point(&self) -> Option<(f32, f32)> {
        match self {
            DragAnchor::None => None,
            DragAnchor::Active { x, y } => Some((*x, *y)),
        }
    }
}

// ---------------------------------------------------------------------------
// HoveredHandle — tracks which handle the cursor is over
// ---------------------------------------------------------------------------

#[derive(Clone, Debug, Default, PartialEq)]
pub enum HoveredHandle {
    #[default]
    None,
    Active {
        node_id: NodeId,
        handle: HandleName,
    },
}

impl HoveredHandle {
    #[must_use]
    pub fn active(node_id: NodeId, handle: HandleName) -> Self {
        Self::Active { node_id, handle }
    }

    // Test-only: explicit constructor for None variant
    #[allow(dead_code)]
    #[must_use]
    pub fn none() -> Self {
        Self::None
    }

    #[must_use]
    pub fn as_tuple(&self) -> Option<(NodeId, String)> {
        match self {
            HoveredHandle::None => None,
            HoveredHandle::Active { node_id, handle } => Some((*node_id, handle.0.clone())),
        }
    }
}

// ---------------------------------------------------------------------------
// TempEdge — ephemeral visual edge during connecting mode
// ---------------------------------------------------------------------------

use crate::ui::edges::Position as FlowPosition;

#[derive(Clone, Debug, Default, PartialEq)]
pub enum TempEdge {
    #[default]
    None,
    Active {
        source: FlowPosition,
        target: FlowPosition,
    },
}

impl TempEdge {
    #[must_use]
    pub fn active(source: FlowPosition, target: FlowPosition) -> Self {
        Self::Active { source, target }
    }

    // Test-only: explicit constructor for None variant
    #[allow(dead_code)]
    #[must_use]
    pub fn none() -> Self {
        Self::None
    }

    #[must_use]
    pub fn as_positions(&self) -> Option<(FlowPosition, FlowPosition)> {
        match self {
            TempEdge::None => None,
            TempEdge::Active { source, target } => Some((*source, *target)),
        }
    }
}

// ---------------------------------------------------------------------------
// Pure transition functions
// ---------------------------------------------------------------------------

/// Determines the correct [`InteractionMode::Dragging`] variant based on the
/// clicked node and any already-selected nodes.
#[must_use]
pub fn drag_mode_from_selection(node_id: NodeId, selected_ids: Vec<NodeId>) -> InteractionMode {
    if selected_ids.is_empty() {
        InteractionMode::Dragging { node_ids: vec![node_id] }
    } else if !selected_ids.contains(&node_id) {
        let all_ids = selected_ids.into_iter().chain(std::iter::once(node_id)).collect();
        InteractionMode::Dragging { node_ids: all_ids }
    } else {
        InteractionMode::Dragging { node_ids: selected_ids }
    }
}

/// Updates `current` position in a [`InteractionMode::Marquee`] while keeping
/// `start` unchanged. Returns a clone of `mode` unchanged for other variants.
#[must_use]
pub fn update_marquee_mode(mode: &InteractionMode, pos: (f32, f32)) -> InteractionMode {
    match mode {
        InteractionMode::Marquee { start, .. } => {
            InteractionMode::Marquee { start: *start, current: pos.into() }
        }
        _ => mode.clone(),
    }
}

/// Returns the CSS cursor class string for the current mode + tool combo.
#[must_use]
pub fn cursor_class_for(mode: &InteractionMode, cursor_tool: CursorTool) -> &'static str {
    match mode {
        InteractionMode::Panning => "cursor-grabbing",
        InteractionMode::Idle if cursor_tool == CursorTool::SpaceHand => "cursor-grab",
        _ => "cursor-default",
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used, clippy::panic, clippy::float_cmp)]
mod tests {
    use super::*;

    // -- InteractionMode predicate tests -------------------------------------

    #[test]
    fn idle_mode_is_idle() {
        assert!(InteractionMode::Idle.is_idle());
        assert!(!InteractionMode::Idle.is_dragging());
        assert!(!InteractionMode::Idle.is_connecting());
        assert!(!InteractionMode::Idle.is_marquee());
        assert!(!InteractionMode::Idle.is_panning());
    }

    #[test]
    fn panning_mode_is_panning() {
        assert!(InteractionMode::Panning.is_panning());
        assert!(!InteractionMode::Panning.is_idle());
        assert!(!InteractionMode::Panning.is_dragging());
    }

    #[test]
    fn dragging_mode_is_dragging() {
        let mode = InteractionMode::Dragging { node_ids: vec![NodeId::new()] };
        assert!(mode.is_dragging());
        assert!(!mode.is_idle());
        assert!(!mode.is_connecting());
    }

    #[test]
    fn connecting_mode_is_connecting() {
        let mode =
            InteractionMode::Connecting { from: NodeId::new(), handle: HandleName::new("output") };
        assert!(mode.is_connecting());
        assert!(!mode.is_idle());
        assert!(!mode.is_dragging());
    }

    #[test]
    fn marquee_mode_is_marquee() {
        let mode = InteractionMode::Marquee {
            start: CanvasPoint::from((0.0, 0.0)),
            current: CanvasPoint::from((10.0, 10.0)),
        };
        assert!(mode.is_marquee());
        assert!(!mode.is_idle());
        assert!(!mode.is_panning());
    }

    // -- drag_mode_from_selection tests --------------------------------------

    #[test]
    fn given_empty_selected_ids_when_starting_drag_then_uses_node_id_as_fallback() {
        let id = NodeId::new();
        let next = drag_mode_from_selection(id, Vec::new());
        assert_eq!(next, InteractionMode::Dragging { node_ids: vec![id] });
    }

    #[test]
    fn given_non_empty_selected_ids_when_starting_drag_then_dragging_mode_includes_all() {
        let id = NodeId::new();
        let id2 = NodeId::new();
        let next = drag_mode_from_selection(id, vec![id2]);
        assert!(matches!(next, InteractionMode::Dragging { .. }));
        if let InteractionMode::Dragging { node_ids } = next {
            assert!(node_ids.contains(&id));
            assert!(node_ids.contains(&id2));
        }
    }

    #[test]
    fn given_node_id_not_in_selected_ids_when_starting_drag_then_includes_node_id() {
        let id = NodeId::new();
        let other_id = NodeId::new();
        let next = drag_mode_from_selection(id, vec![other_id]);
        if let InteractionMode::Dragging { node_ids } = next {
            assert!(node_ids.contains(&id));
            assert!(node_ids.contains(&other_id));
        }
    }

    #[test]
    fn given_node_id_in_selected_ids_when_starting_drag_then_uses_selected_ids() {
        let id = NodeId::new();
        let id2 = NodeId::new();
        let next = drag_mode_from_selection(id, vec![id, id2]);
        if let InteractionMode::Dragging { node_ids } = next {
            assert!(node_ids.contains(&id));
            assert!(node_ids.contains(&id2));
            assert_eq!(node_ids.len(), 2);
        }
    }

    // -- update_marquee_mode tests -------------------------------------------

    #[test]
    fn given_marquee_mode_when_updating_then_start_is_preserved_and_current_updates() {
        let mode = InteractionMode::Marquee {
            start: CanvasPoint::from((1.0, 2.0)),
            current: CanvasPoint::from((3.0, 4.0)),
        };

        let next = update_marquee_mode(&mode, (9.0, 10.0));
        assert_eq!(
            next,
            InteractionMode::Marquee {
                start: CanvasPoint::from((1.0, 2.0)),
                current: CanvasPoint::from((9.0, 10.0))
            }
        );
    }

    #[test]
    fn given_idle_mode_when_updating_marquee_then_returns_clone_unchanged() {
        let mode = InteractionMode::Idle;
        let next = update_marquee_mode(&mode, (5.0, 5.0));
        assert_eq!(next, InteractionMode::Idle);
    }

    // -- cursor_class_for tests ----------------------------------------------

    #[test]
    fn given_space_hand_enabled_and_idle_when_getting_cursor_class_then_cursor_grab_is_returned() {
        let class = cursor_class_for(&InteractionMode::Idle, CursorTool::SpaceHand);
        assert_eq!(class, "cursor-grab");
    }

    #[test]
    fn given_panning_mode_when_getting_cursor_class_then_cursor_grabbing() {
        let class = cursor_class_for(&InteractionMode::Panning, CursorTool::Select);
        assert_eq!(class, "cursor-grabbing");
    }

    #[test]
    fn given_select_tool_and_idle_when_getting_cursor_class_then_cursor_default() {
        let class = cursor_class_for(&InteractionMode::Idle, CursorTool::Select);
        assert_eq!(class, "cursor-default");
    }

    #[test]
    fn given_dragging_mode_when_getting_cursor_class_then_cursor_default() {
        let mode = InteractionMode::Dragging { node_ids: vec![NodeId::new()] };
        let class = cursor_class_for(&mode, CursorTool::Select);
        assert_eq!(class, "cursor-default");
    }

    // -- CanvasPoint conversion tests ----------------------------------------

    #[test]
    fn canvas_point_from_tuple_roundtrips() {
        let point = CanvasPoint::from((3.5, 7.25));
        assert_eq!(point.x, 3.5);
        assert_eq!(point.y, 7.25);
        let back: (f32, f32) = point.into();
        assert_eq!(back, (3.5, 7.25));
    }

    // -- HandleName tests ----------------------------------------------------

    #[test]
    fn handle_name_from_str_and_as_str_roundtrip() {
        let name = HandleName::new("source");
        assert_eq!(name.as_str(), "source");
    }

    #[test]
    fn handle_name_from_string() {
        let name = HandleName::from(String::from("target"));
        assert_eq!(name.as_str(), "target");
    }

    #[test]
    fn handle_name_from_ref_str() {
        let name = HandleName::from("output");
        assert_eq!(name.as_str(), "output");
    }

    // -- DragAnchor tests ----------------------------------------------------

    #[test]
    fn given_drag_anchor_active_when_as_point_then_returns_some() {
        let anchor = DragAnchor::active(10.0, 20.0);
        assert_eq!(anchor.as_point(), Some((10.0, 20.0)));
    }

    #[test]
    fn given_drag_anchor_none_when_as_point_then_returns_none() {
        let anchor = DragAnchor::none();
        assert_eq!(anchor.as_point(), None);
    }

    #[test]
    fn drag_anchor_default_is_none() {
        assert_eq!(DragAnchor::default(), DragAnchor::None);
    }

    // -- HoveredHandle tests -------------------------------------------------

    #[test]
    fn given_hovered_handle_active_when_as_tuple_then_returns_some() {
        let id = NodeId::new();
        let handle = HoveredHandle::active(id, HandleName::new("output"));
        let result = handle.as_tuple();
        assert_eq!(result, Some((id, "output".to_string())));
    }

    #[test]
    fn given_hovered_handle_none_when_as_tuple_then_returns_none() {
        assert_eq!(HoveredHandle::None.as_tuple(), None);
    }

    #[test]
    fn hovered_handle_default_is_none() {
        assert_eq!(HoveredHandle::default(), HoveredHandle::None);
    }

    // -- TempEdge tests ------------------------------------------------------

    #[test]
    fn temp_edge_default_is_none() {
        assert_eq!(TempEdge::default(), TempEdge::None);
    }

    #[test]
    fn temp_edge_none_as_positions_returns_none() {
        assert_eq!(TempEdge::None.as_positions(), None);
    }
}
