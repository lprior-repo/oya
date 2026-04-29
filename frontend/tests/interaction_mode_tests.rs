//! Tests for interaction mode state machine transitions.
//!
//! Verifies that mode transitions follow valid paths and that each mode
//! correctly identifies itself while rejecting other mode predicates.
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic, clippy::float_cmp)]

use oya_frontend::graph::NodeId;
use oya_frontend::interaction_mode::{
    cursor_class_for, drag_mode_from_selection, update_marquee_mode, CanvasPoint, CursorTool,
    DragAnchor, HandleName, HoveredHandle, InteractionMode, TempEdge,
};

// ===========================================================================
// State Transition Validity
// ===========================================================================

#[test]
fn given_idle_when_transitioning_to_panning_then_mode_is_panning() {
    let from = InteractionMode::Idle;
    let to = InteractionMode::Panning;
    assert!(from.is_idle());
    assert!(to.is_panning());
    assert_ne!(from, to);
}

#[test]
fn given_idle_when_transitioning_to_dragging_then_mode_is_dragging() {
    let id = NodeId::new();
    let to = drag_mode_from_selection(id, vec![]);
    assert!(matches!(to, InteractionMode::Dragging { .. }));
    assert!(to.is_dragging());
    assert!(!to.is_idle());
}

#[test]
fn given_idle_when_transitioning_to_connecting_then_mode_is_connecting() {
    let id = NodeId::new();
    let to = InteractionMode::Connecting { from: id, handle: HandleName::new("source") };
    assert!(to.is_connecting());
    assert!(!to.is_idle());
}

#[test]
fn given_idle_when_transitioning_to_marquee_then_mode_is_marquee() {
    let to = InteractionMode::Marquee {
        start: CanvasPoint::from((0.0, 0.0)),
        current: CanvasPoint::from((10.0, 10.0)),
    };
    assert!(to.is_marquee());
    assert!(!to.is_idle());
}

#[test]
fn given_panning_when_transitioning_to_idle_then_mode_is_idle() {
    let from = InteractionMode::Panning;
    let to = InteractionMode::Idle;
    assert!(from.is_panning());
    assert!(to.is_idle());
}

#[test]
fn given_dragging_when_transitioning_to_idle_then_mode_is_idle() {
    let from = InteractionMode::Dragging { node_ids: vec![NodeId::new()] };
    let to = InteractionMode::Idle;
    assert!(from.is_dragging());
    assert!(to.is_idle());
}

#[test]
fn given_connecting_when_transitioning_to_idle_then_mode_is_idle() {
    let from =
        InteractionMode::Connecting { from: NodeId::new(), handle: HandleName::new("source") };
    let to = InteractionMode::Idle;
    assert!(from.is_connecting());
    assert!(to.is_idle());
}

#[test]
fn given_marquee_when_transitioning_to_idle_then_mode_is_idle() {
    let from = InteractionMode::Marquee {
        start: CanvasPoint::from((0.0, 0.0)),
        current: CanvasPoint::from((50.0, 50.0)),
    };
    let to = InteractionMode::Idle;
    assert!(from.is_marquee());
    assert!(to.is_idle());
}

// ===========================================================================
// Single-State Invariant (no mode claims two identities)
// ===========================================================================

#[test]
fn given_idle_mode_then_only_idle_predicate_is_true() {
    let mode = InteractionMode::Idle;
    let true_count = mode.is_idle() as u8
        + mode.is_dragging() as u8
        + mode.is_connecting() as u8
        + mode.is_marquee() as u8
        + mode.is_panning() as u8;
    assert_eq!(true_count, 1, "Idle should match exactly one predicate");
}

#[test]
fn given_panning_mode_then_only_panning_predicate_is_true() {
    let mode = InteractionMode::Panning;
    let true_count = mode.is_idle() as u8
        + mode.is_dragging() as u8
        + mode.is_connecting() as u8
        + mode.is_marquee() as u8
        + mode.is_panning() as u8;
    assert_eq!(true_count, 1, "Panning should match exactly one predicate");
}

#[test]
fn given_dragging_mode_then_only_dragging_predicate_is_true() {
    let mode = InteractionMode::Dragging { node_ids: vec![NodeId::new()] };
    let true_count = mode.is_idle() as u8
        + mode.is_dragging() as u8
        + mode.is_connecting() as u8
        + mode.is_marquee() as u8
        + mode.is_panning() as u8;
    assert_eq!(true_count, 1, "Dragging should match exactly one predicate");
}

#[test]
fn given_connecting_mode_then_only_connecting_predicate_is_true() {
    let mode =
        InteractionMode::Connecting { from: NodeId::new(), handle: HandleName::new("source") };
    let true_count = mode.is_idle() as u8
        + mode.is_dragging() as u8
        + mode.is_connecting() as u8
        + mode.is_marquee() as u8
        + mode.is_panning() as u8;
    assert_eq!(true_count, 1, "Connecting should match exactly one predicate");
}

#[test]
fn given_marquee_mode_then_only_marquee_predicate_is_true() {
    let mode = InteractionMode::Marquee {
        start: CanvasPoint::from((0.0, 0.0)),
        current: CanvasPoint::from((10.0, 10.0)),
    };
    let true_count = mode.is_idle() as u8
        + mode.is_dragging() as u8
        + mode.is_connecting() as u8
        + mode.is_marquee() as u8
        + mode.is_panning() as u8;
    assert_eq!(true_count, 1, "Marquee should match exactly one predicate");
}

// ===========================================================================
// Mode Affects Cursor Behavior
// ===========================================================================

#[test]
fn given_each_mode_when_select_tool_then_correct_cursor() {
    assert_eq!(cursor_class_for(&InteractionMode::Idle, CursorTool::Select), "cursor-default");
    assert_eq!(cursor_class_for(&InteractionMode::Panning, CursorTool::Select), "cursor-grabbing");
    assert_eq!(
        cursor_class_for(
            &InteractionMode::Dragging { node_ids: vec![NodeId::new()] },
            CursorTool::Select
        ),
        "cursor-default"
    );
    assert_eq!(
        cursor_class_for(
            &InteractionMode::Connecting { from: NodeId::new(), handle: HandleName::new("source") },
            CursorTool::Select
        ),
        "cursor-default"
    );
    assert_eq!(
        cursor_class_for(
            &InteractionMode::Marquee {
                start: CanvasPoint::default(),
                current: CanvasPoint::default()
            },
            CursorTool::Select
        ),
        "cursor-default"
    );
}

#[test]
fn given_panning_mode_when_space_hand_tool_then_cursor_grabbing_overrides() {
    // Panning always produces cursor-grabbing regardless of tool
    assert_eq!(
        cursor_class_for(&InteractionMode::Panning, CursorTool::SpaceHand),
        "cursor-grabbing"
    );
    assert_eq!(cursor_class_for(&InteractionMode::Panning, CursorTool::Select), "cursor-grabbing");
}

// ===========================================================================
// Drag Mode Transitions with Selection State
// ===========================================================================

#[test]
fn given_single_node_selected_when_drag_starts_then_dragging_contains_node() {
    let id = NodeId::new();
    let mode = drag_mode_from_selection(id, vec![]);
    match mode {
        InteractionMode::Dragging { node_ids } => {
            assert_eq!(node_ids.len(), 1);
            assert_eq!(node_ids[0], id);
        }
        _ => panic!("Expected Dragging mode, got {:?}", mode),
    }
}

#[test]
fn given_multiple_selected_when_drag_starts_then_all_included() {
    let drag_id = NodeId::new();
    let sel1 = NodeId::new();
    let sel2 = NodeId::new();
    let mode = drag_mode_from_selection(drag_id, vec![sel1, sel2]);
    match mode {
        InteractionMode::Dragging { node_ids } => {
            assert_eq!(node_ids.len(), 3);
            assert!(node_ids.contains(&drag_id));
            assert!(node_ids.contains(&sel1));
            assert!(node_ids.contains(&sel2));
        }
        _ => panic!("Expected Dragging mode"),
    }
}

#[test]
fn given_drag_node_already_selected_when_drag_starts_then_no_duplicate() {
    let id = NodeId::new();
    let mode = drag_mode_from_selection(id, vec![id]);
    match mode {
        InteractionMode::Dragging { node_ids } => {
            assert_eq!(node_ids.len(), 1, "Should not duplicate the node ID");
            assert_eq!(node_ids[0], id);
        }
        _ => panic!("Expected Dragging mode"),
    }
}

// ===========================================================================
// Marquee Update Preserves Start
// ===========================================================================

#[test]
fn given_marquee_when_updated_multiple_times_then_start_never_changes() {
    let original_start = CanvasPoint::from((5.0, 5.0));
    let mode = InteractionMode::Marquee {
        start: original_start,
        current: CanvasPoint::from((10.0, 10.0)),
    };

    let updated1 = update_marquee_mode(&mode, (20.0, 30.0));
    let updated2 = update_marquee_mode(&updated1, (40.0, 50.0));
    let updated3 = update_marquee_mode(&updated2, (100.0, 200.0));

    for updated in [&updated1, &updated2, &updated3] {
        match updated {
            InteractionMode::Marquee { start, current: _ } => {
                assert_eq!(*start, original_start, "Start should never change");
            }
            _ => panic!("Expected Marquee mode"),
        }
    }
}

#[test]
fn given_non_marquee_modes_when_updating_marquee_then_noop() {
    let modes = vec![
        InteractionMode::Idle,
        InteractionMode::Panning,
        InteractionMode::Dragging { node_ids: vec![NodeId::new()] },
        InteractionMode::Connecting { from: NodeId::new(), handle: HandleName::new("source") },
    ];

    for mode in modes {
        let result = update_marquee_mode(&mode, (99.0, 99.0));
        assert_eq!(result, mode, "update_marquee_mode should be a no-op for non-Marquee modes");
    }
}

// ===========================================================================
// Default Values
// ===========================================================================

#[test]
fn given_default_interaction_mode_then_idle() {
    assert_eq!(InteractionMode::default(), InteractionMode::Idle);
}

#[test]
fn given_default_cursor_tool_then_select() {
    assert_eq!(CursorTool::default(), CursorTool::Select);
}

#[test]
fn given_default_drag_anchor_then_none() {
    assert_eq!(DragAnchor::default(), DragAnchor::None);
}

#[test]
fn given_default_hovered_handle_then_none() {
    assert_eq!(HoveredHandle::default(), HoveredHandle::None);
}

#[test]
fn given_default_temp_edge_then_none() {
    assert_eq!(TempEdge::default(), TempEdge::None);
}

#[test]
fn given_default_canvas_point_then_zero_zero() {
    let point = CanvasPoint::default();
    assert_eq!(point.x, 0.0);
    assert_eq!(point.y, 0.0);
}

// ===========================================================================
// Cross-Mode Transition: Full Select → Pan → Connect Cycle
// ===========================================================================

#[test]
fn given_full_interaction_cycle_then_modes_transition_correctly() {
    // Simulate: Idle → Dragging → Idle → Connecting → Idle → Marquee → Idle
    let node_id = NodeId::new();

    // Start Idle
    let mode = InteractionMode::Idle;
    assert!(mode.is_idle());

    // User drags a node
    let mode = drag_mode_from_selection(node_id, vec![]);
    assert!(mode.is_dragging());

    // Drop node, back to Idle
    let mode = InteractionMode::Idle;
    assert!(mode.is_idle());

    // User starts connecting from handle
    let mode = InteractionMode::Connecting { from: node_id, handle: HandleName::new("source") };
    assert!(mode.is_connecting());

    // Connection complete, back to Idle
    let mode = InteractionMode::Idle;
    assert!(mode.is_idle());

    // User starts marquee selection
    let mode = InteractionMode::Marquee {
        start: CanvasPoint::from((0.0, 0.0)),
        current: CanvasPoint::from((100.0, 100.0)),
    };
    assert!(mode.is_marquee());

    // Update marquee during drag
    let mode = update_marquee_mode(&mode, (200.0, 200.0));
    assert!(mode.is_marquee());

    // Release, back to Idle
    let mode = InteractionMode::Idle;
    assert!(mode.is_idle());
}
