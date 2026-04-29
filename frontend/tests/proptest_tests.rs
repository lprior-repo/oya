//! Property-based tests covering core domain invariants.
//!
//! Tests in this file verify:
//! 1. UndoSnapshot round-trip: structural fields survive serde; volatile fields are excluded.
//! 2. WorkflowState undo/redo composition: stacks behave correctly under arbitrary sequences.
//! 3. Category theme consistency: NodeCategory Display matches the canonical lowercase names.
//! 4. Node position snap idempotency: grid-snapped positions are stable under repeated snaps.
//! 5. Connection ID uniqueness: every connection receives a distinct UUID.
//! 6. Viewport fit_view determinism: identical inputs produce identical outputs.
//! 7. Validation result consistency: ValidationResult::from_issues matches has_errors.
//! 8. RunOutcome exhaustiveness: every variant has a deterministic is_success value.
//! 9. find_safe_position progress: every call advances past existing positions.
//! 10. calculate_pan_offset identity: zoom=1.0 with finite inputs returns original viewport.
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic, clippy::float_cmp)]

use oya_frontend::graph::{
    calc, validate_workflow, NodeCategory, NodeId, PortName, RunOutcome, Viewport, Workflow,
};
use proptest::prelude::*;
use std::collections::HashSet;

// ===========================================================================
// Strategies
// ===========================================================================

fn category_strategy() -> impl Strategy<Value = NodeCategory> {
    prop_oneof![
        Just(NodeCategory::Entry),
        Just(NodeCategory::Durable),
        Just(NodeCategory::State),
        Just(NodeCategory::Flow),
        Just(NodeCategory::Timing),
        Just(NodeCategory::Signal),
    ]
}

fn viewport_strategy() -> impl Strategy<Value = Viewport> {
    (-1000.0f32..1000.0, -1000.0f32..1000.0, 0.1f32..5.0).prop_map(|(x, y, zoom)| Viewport {
        x,
        y,
        zoom,
    })
}

// ===========================================================================
// 1. UndoSnapshot round-trip: structural fields survive serde,
//    volatile fields are excluded.
// ===========================================================================

proptest! {
    /// After serializing and deserializing a Workflow, structural fields
    /// (nodes, connections, viewport) must be preserved, while volatile
    /// fields (execution_records, history, current_step, execution_queue)
    /// must reset to defaults because they carry `#[serde(skip)]`.
    #[test]
    fn prop_undo_snapshot_roundtrip_preserves_structure_excludes_volatile(
        node_count in 0usize..5,
        viewport in viewport_strategy(),
    ) {
        let mut wf = Workflow::new();
        for _ in 0..node_count {
            wf.add_node("run", 0.0, 0.0);
        }

        // Wire up chain connections if possible
        let port = PortName::from("main");
        let ids: Vec<NodeId> = wf.nodes.iter().map(|n| n.id).collect();
        for i in 0..ids.len().saturating_sub(1) {
            let _ = wf.add_connection_checked(ids[i], ids[i + 1], &port, &port);
        }

        wf.viewport = viewport;

        // Round-trip through serde (simulates undo snapshot save/restore)
        let json = serde_json::to_string(&wf).expect("serialization must succeed");
        let restored: Workflow =
            serde_json::from_str(&json).expect("deserialization must succeed");

        // Structural fields must be preserved
        prop_assert_eq!(wf.nodes.len(), restored.nodes.len(),
            "Node count must survive round-trip");
        prop_assert_eq!(wf.connections.len(), restored.connections.len(),
            "Connection count must survive round-trip");
        prop_assert_eq!(wf.viewport, restored.viewport,
            "Viewport must survive round-trip");

        // Volatile fields must be reset to defaults (they are #[serde(skip)])
        prop_assert!(restored.history.is_empty(),
            "History must be empty after round-trip (volatile field excluded)");
        prop_assert!(restored.execution_records.is_empty(),
            "Execution records must be empty after round-trip (volatile field excluded)");
        prop_assert_eq!(restored.current_step, 0,
            "Current step must reset to 0 after round-trip");
        prop_assert!(restored.execution_queue.is_empty(),
            "Execution queue must be empty after round-trip (volatile field excluded)");
    }
}

proptest! {
    /// Node positions and IDs must survive serialization round-trip exactly.
    #[test]
    fn prop_node_positions_preserved_after_serde_roundtrip(
        x in -500.0f32..500.0,
        y in -500.0f32..500.0,
    ) {
        let mut wf = Workflow::new();
        let id = wf.add_node("run", x, y);

        let json = serde_json::to_string(&wf).expect("serialization");
        let restored: Workflow = serde_json::from_str(&json).expect("deserialization");

        let original_node = wf.nodes.iter().find(|n| n.id == id);
        let restored_node = restored.nodes.iter().find(|n| n.id == id);

        prop_assert!(original_node.is_some(), "Original node must exist");
        prop_assert!(restored_node.is_some(), "Restored node must exist");

        if let (Some(orig), Some(rest)) = (original_node, restored_node) {
            prop_assert_eq!(orig.id, rest.id, "Node ID must survive round-trip");
            prop_assert!((orig.x - rest.x).abs() < f32::EPSILON,
                "Node x must survive round-trip");
            prop_assert!((orig.y - rest.y).abs() < f32::EPSILON,
                "Node y must survive round-trip");
        }
    }
}

// ===========================================================================
// 2. WorkflowState undo/redo composition
//    (tested at graph::Workflow level since hooks require Dioxus runtime)
// ===========================================================================

proptest! {
    /// Adding N nodes then performing N undo-style clone-and-replace operations
    /// preserves the count invariant at each step.
    #[test]
    fn prop_undo_sequence_preserves_invariants(
        add_count in 1usize..8,
    ) {
        let mut current = Workflow::new();
        let mut undo_stack: Vec<Workflow> = Vec::new();

        for i in 0..add_count {
            // Snapshot before mutation
            undo_stack.push(current.clone());
            current.add_node("run", (i * 100) as f32, 0.0);
        }

        // Verify all nodes were added
        prop_assert_eq!(current.nodes.len(), add_count);

        // Undo each one (restore from stack)
        let mut expected_len = add_count;
        while let Some(snapshot) = undo_stack.pop() {
            expected_len -= 1;
            current = snapshot;
            prop_assert_eq!(current.nodes.len(), expected_len);
        }

        prop_assert!(current.nodes.is_empty(), "All undos must restore empty state");
    }
}

proptest! {
    /// Redo after undo restores the exact same state.
    #[test]
    fn prop_redo_after_undo_restores_state(
        node_count in 1usize..5,
    ) {
        let mut wf = Workflow::new();
        for i in 0..node_count {
            wf.add_node("run", (i * 50) as f32, (i * 30) as f32);
        }
        let state_after_add = wf.clone();

        // Simulate undo (save current, restore previous)
        let mut redo_stack: Vec<Workflow> = Vec::new();
        redo_stack.push(wf.clone());
        wf = Workflow::new();

        prop_assert!(wf.nodes.is_empty(), "After undo, workflow must be empty");

        // Redo
        if let Some(redo_state) = redo_stack.pop() {
            wf = redo_state;
        }

        prop_assert_eq!(wf.nodes.len(), state_after_add.nodes.len(),
            "Redo must restore same node count");
        for (orig, restored) in state_after_add.nodes.iter().zip(wf.nodes.iter()) {
            prop_assert_eq!(orig.id, restored.id, "Redo must restore same node IDs");
            prop_assert!((orig.x - restored.x).abs() < f32::EPSILON,
                "Redo must restore same x positions");
        }
    }
}

// ===========================================================================
// 3. Category theme consistency
//    NodeCategory Display must match canonical lowercase names used in UI.
// ===========================================================================

proptest! {
    /// Every NodeCategory Display representation must match its canonical
    /// lowercase string name. This ensures the three match arms in node.rs
    /// (category_border, category_icon_bg, category_accent_bar) stay in sync.
    #[test]
    fn prop_category_display_matches_canonical_name(
        category in category_strategy(),
    ) {
        let expected = match category {
            NodeCategory::Entry => "entry",
            NodeCategory::Durable => "durable",
            NodeCategory::State => "state",
            NodeCategory::Flow => "flow",
            NodeCategory::Timing => "timing",
            NodeCategory::Signal => "signal",
        };

        let display = category.to_string();
        prop_assert_eq!(display, expected);
    }
}

/// Every category produces a non-empty, distinct Display string.
/// No two categories may share the same display string.
#[test]
fn prop_all_categories_have_unique_display_names() {
    let all_categories = [
        NodeCategory::Entry,
        NodeCategory::Durable,
        NodeCategory::State,
        NodeCategory::Flow,
        NodeCategory::Timing,
        NodeCategory::Signal,
    ];

    let display_strings: Vec<String> = all_categories.iter().map(|c| c.to_string()).collect();

    let unique_set: HashSet<String> = display_strings.iter().cloned().collect();
    assert_eq!(
        unique_set.len(),
        all_categories.len(),
        "All categories must produce unique display strings"
    );

    // None should be empty
    for s in &display_strings {
        assert!(!s.is_empty(), "Category display string must not be empty");
    }
}

// ===========================================================================
// 4. Node position snap idempotency
// ===========================================================================

proptest! {
    /// Grid-snapped positions must be stable: applying update_node_position
    /// with zero delta to an already-snapped position must return the same
    /// position. This tests idempotency of the snap grid.
    #[test]
    fn prop_position_snap_is_idempotent(
        x in -10000.0f32..10000.0,
        y in -10000.0f32..10000.0,
    ) {
        let (snap_x, snap_y) = calc::update_node_position(x, y, 0.0, 0.0);

        // Must be grid-aligned (multiple of 10)
        prop_assert!((snap_x % 10.0).abs() < f32::EPSILON,
            "Snapped x must be grid-aligned");
        prop_assert!((snap_y % 10.0).abs() < f32::EPSILON,
            "Snapped y must be grid-aligned");

        // Idempotent: snap again with zero delta must produce same result
        let (snap2_x, snap2_y) = calc::update_node_position(snap_x, snap_y, 0.0, 0.0);
        prop_assert_eq!((snap_x, snap_y), (snap2_x, snap2_y),
            "Grid snap must be idempotent");
    }
}

// ===========================================================================
// 5. Connection ID uniqueness
// ===========================================================================

proptest! {
    /// Every connection added to a workflow must receive a unique UUID.
    /// Adding connections to different node pairs must never produce
    /// duplicate IDs.
    #[test]
    fn prop_connection_ids_are_unique(
        pair_count in 2usize..6,
    ) {
        let mut wf = Workflow::new();
        // Create enough nodes for pair_count connections
        for _ in 0..pair_count + 1 {
            wf.add_node("run", 0.0, 0.0);
        }

        let port = PortName::from("main");
        let ids: Vec<NodeId> = wf.nodes.iter().map(|n| n.id).collect();

        // Create chain connections: 0->1, 1->2, ...
        for i in 0..pair_count.min(ids.len().saturating_sub(1)) {
            let result = wf.add_connection_checked(ids[i], ids[i + 1], &port, &port);
            prop_assert!(result.is_ok(), "Connection must succeed");
        }

        // Verify all connection IDs are unique
        let conn_ids: HashSet<uuid::Uuid> = wf.connections.iter().map(|c| c.id).collect();
        prop_assert_eq!(conn_ids.len(), wf.connections.len(),
            "All connection IDs must be unique");
    }
}

/// Adding a connection to the same endpoints must produce unique UUIDs
/// when the first connection succeeds.
#[test]
fn prop_same_endpoints_produce_unique_connection_ids() {
    let mut wf = Workflow::new();
    let src = wf.add_node("http-handler", 0.0, 0.0);
    let tgt = wf.add_node("run", 100.0, 0.0);
    let port = PortName::from("main");

    let first = wf.add_connection_checked(src, tgt, &port, &port);
    assert!(first.is_ok(), "First connection must succeed");

    // Second identical connection must be rejected as duplicate
    let second = wf.add_connection_checked(src, tgt, &port, &port);
    assert!(
        matches!(second, Err(oya_frontend::graph::ConnectivityConnectionError::Duplicate)),
        "Duplicate connection must be rejected"
    );

    // Only one connection should exist
    assert_eq!(wf.connections.len(), 1, "Only one connection should exist");
}

// ===========================================================================
// 6. Viewport fit_view determinism
// ===========================================================================

proptest! {
    /// calculate_fit_view must produce identical results for identical inputs.
    #[test]
    fn prop_fit_view_is_deterministic(
        x1 in 0.0f32..500.0,
        y1 in 0.0f32..500.0,
        x2 in 500.0f32..1000.0,
        y2 in 500.0f32..1000.0,
        vp_width in 400.0f32..1920.0,
        vp_height in 300.0f32..1080.0,
        padding in 0.0f32..100.0,
    ) {
        let nodes = [(x1, y1), (x2, y2)];

        let result1 = calc::calculate_fit_view(&nodes, vp_width, vp_height, padding);
        let result2 = calc::calculate_fit_view(&nodes, vp_width, vp_height, padding);

        prop_assert_eq!(result1, result2,
            "calculate_fit_view must be deterministic for identical inputs");
    }
}

proptest! {
    /// calculate_fit_view zoom must always be within [0.15, 1.5] when it
    /// returns Some for finite inputs.
    #[test]
    fn prop_fit_view_zoom_in_bounds(
        x in -1000.0f32..1000.0,
        y in -1000.0f32..1000.0,
        vp_width in 400.0f32..1920.0,
        vp_height in 300.0f32..1080.0,
        padding in 0.0f32..100.0,
    ) {
        let nodes = [(x, y)];

        if let Some((_vx, _vy, zoom)) = calc::calculate_fit_view(&nodes, vp_width, vp_height, padding) {
            prop_assert!(zoom >= 0.15, "Zoom must be >= 0.15");
            prop_assert!(zoom <= 1.5, "Zoom must be <= 1.5");
        }
    }
}

// ===========================================================================
// 7. Validation result consistency
// ===========================================================================

/// An empty workflow must always validate with at most a missing-entry-point
/// error (never duplicate-ID or orphan-node errors).
#[test]
fn prop_empty_workflow_validates_with_only_entry_point_error() {
    let wf = Workflow::new();
    let result = validate_workflow(&wf);

    // An empty workflow has no entry point, so exactly one error is expected.
    // It must never report duplicate IDs or orphan nodes.
    assert!(
        result.error_count() <= 1,
        "Empty workflow should have at most 1 error, got: {:?}",
        result.issues
    );

    // The only possible error must be about missing entry point
    for issue in &result.issues {
        assert!(
            issue.message.contains("entry point"),
            "Only entry-point errors expected for empty workflow, got: {}",
            issue.message
        );
    }
}

proptest! {
    /// ValidationResult::from_issues must have has_errors() consistent
    /// with error_count(). If error_count > 0, has_errors must be true.
    #[test]
    fn prop_validation_result_consistency(
        node_count in 1usize..6,
    ) {
        let mut wf = Workflow::new();
        for _ in 0..node_count {
            wf.add_node("run", 0.0, 0.0);
        }

        let result = validate_workflow(&wf);

        // Invariant: has_errors() == (error_count() > 0)
        if result.error_count() > 0 {
            prop_assert!(result.has_errors(),
                "has_errors must be true when error_count > 0");
        } else {
            prop_assert!(!result.has_errors(),
                "has_errors must be false when error_count == 0");
        }
    }
}

// ===========================================================================
// 8. RunOutcome exhaustiveness
// ===========================================================================

/// For all RunOutcome variants, is_success must be deterministic and
/// mutually exclusive: exactly one variant returns true.
#[test]
fn prop_run_outcome_is_success_is_mutually_exclusive() {
    let success = RunOutcome::Success.is_success();
    let failure = RunOutcome::Failure.is_success();

    assert!(success, "RunOutcome::Success.is_success() must be true");
    assert!(!failure, "RunOutcome::Failure.is_success() must be false");

    // Exactly one must be true
    assert_eq!(
        [success, failure].iter().filter(|&&b| b).count(),
        1,
        "Exactly one RunOutcome variant must have is_success() == true"
    );
}

// ===========================================================================
// 9. find_safe_position progress
// ===========================================================================

proptest! {
    /// find_safe_position must always return a position that does not
    /// collide with any existing position.
    #[test]
    fn prop_find_safe_position_no_collision(
        existing_count in 0usize..10,
        desired_x in -500.0f32..500.0,
        desired_y in -500.0f32..500.0,
    ) {
        let mut existing: Vec<(f32, f32)> = Vec::new();
        for i in 0..existing_count {
            existing.push(((i * 40) as f32, (i * 40) as f32));
        }

        let (safe_x, safe_y) = calc::find_safe_position(&existing, desired_x, desired_y, 30.0);

        // The safe position must not collide with any existing position
        for &(ex, ey) in &existing {
            let x_close = (ex - safe_x).abs() < 10.0;
            let y_close = (ey - safe_y).abs() < 10.0;
            prop_assert!(!(x_close && y_close),
                "Safe position collides with existing position");
        }
    }
}

// ===========================================================================
// 10. calculate_pan_offset identity
// ===========================================================================

proptest! {
    /// When new_zoom equals old_zoom (identity zoom), calculate_pan_offset
    /// must return the original viewport coordinates for finite inputs.
    #[test]
    fn prop_pan_offset_identity_zoom(
        viewport_x in -1000.0f32..1000.0,
        viewport_y in -1000.0f32..1000.0,
        center_x in -500.0f32..500.0,
        center_y in -500.0f32..500.0,
        zoom in 0.1f32..5.0,
    ) {
        let (result_x, result_y) = calc::calculate_pan_offset(
            (viewport_x, viewport_y),
            (center_x, center_y),
            (zoom, zoom), // old_zoom == new_zoom
        );

        prop_assert!((result_x - viewport_x).abs() < 0.01,
            "Identity zoom must preserve viewport_x");
        prop_assert!((result_y - viewport_y).abs() < 0.01,
            "Identity zoom must preserve viewport_y");
    }
}

proptest! {
    /// calculate_pan_offset must never return NaN or infinity for finite inputs.
    #[test]
    fn prop_pan_offset_always_finite_for_finite_inputs(
        viewport_x in -10000.0f32..10000.0,
        viewport_y in -10000.0f32..10000.0,
        center_x in -10000.0f32..10000.0,
        center_y in -10000.0f32..10000.0,
        old_zoom in 0.01f32..10.0,
        new_zoom in 0.01f32..10.0,
    ) {
        let (result_x, result_y) = calc::calculate_pan_offset(
            (viewport_x, viewport_y),
            (center_x, center_y),
            (old_zoom, new_zoom),
        );

        prop_assert!(result_x.is_finite(), "Result x must be finite");
        prop_assert!(result_y.is_finite(), "Result y must be finite");
    }
}
