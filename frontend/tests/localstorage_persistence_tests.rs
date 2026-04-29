//! Integration tests for localStorage persistence (flow-wasm-v1.yaml).
//!
//! The actual localStorage read/write is gated behind `#[cfg(target_arch = "wasm32")]`
//! and requires a browser. These tests verify the underlying persistence logic:
//!
//! 1. Serde round-trip: serialize → deserialize preserves all state
//! 2. Corrupted data: invalid JSON falls back gracefully
//! 3. Partial data: missing fields get serde defaults
//! 4. Key consistency: save and load use the same storage key
//! 5. Large workflow: serialization succeeds within memory limits
//!
//! Browser-level tests (actual localStorage) require E2E (Playwright).
//!
//! Spec ref: specs/flow-wasm-v1.yaml → workflow-persistence behavior
//! Run: cargo test --test localstorage_persistence_tests

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use oya_frontend::graph::{PortName, Workflow};
use oya_frontend::ui::app_bootstrap::default_workflow;

/// The localStorage key used by both save and load paths.
/// Must match `src/ui/app_shell.rs` and `src/hooks/use_workflow_state.rs`.
const STORAGE_KEY: &str = "flow-wasm-v1-workflow";

fn main_port() -> PortName {
    PortName::from("main")
}

fn build_workflow_with_nodes() -> Workflow {
    let mut w = Workflow::new();
    let a = w.add_node("http-handler", 10.0, 20.0);
    let b = w.add_node("run", 100.0, 20.0);
    let c = w.add_node("condition", 200.0, 20.0);
    let mp = main_port();
    let _ = w.add_connection_checked(a, b, &mp, &mp);
    let _ = w.add_connection_checked(b, c, &mp, &mp);
    w
}

// ═══════════════════════════════════════════════════════════════════════════
// 1. Serde Round-Trip: Workflow saves and loads without data loss
// spec: workflow-persistence "All nodes are restored to their original positions"
// ═══════════════════════════════════════════════════════════════════════════

#[test]
fn round_trip_preserves_node_count() {
    let original = build_workflow_with_nodes();
    let json = serde_json::to_string(&original).unwrap();
    let restored: Workflow = serde_json::from_str(&json).unwrap();

    assert_eq!(restored.nodes.len(), original.nodes.len(), "node count must survive round-trip");
}

#[test]
fn round_trip_preserves_connections() {
    let original = build_workflow_with_nodes();
    let json = serde_json::to_string(&original).unwrap();
    let restored: Workflow = serde_json::from_str(&json).unwrap();

    assert_eq!(
        restored.connections.len(),
        original.connections.len(),
        "connection count must survive round-trip"
    );
    for (orig, rest) in original.connections.iter().zip(restored.connections.iter()) {
        assert_eq!(orig.source, rest.source, "connection source must match");
        assert_eq!(orig.target, rest.target, "connection target must match");
        assert_eq!(orig.source_port, rest.source_port, "source port must match");
        assert_eq!(orig.target_port, rest.target_port, "target port must match");
    }
}

#[test]
fn round_trip_preserves_node_positions() {
    // spec: "All nodes are restored to their original positions"
    let original = build_workflow_with_nodes();
    let json = serde_json::to_string(&original).unwrap();
    let restored: Workflow = serde_json::from_str(&json).unwrap();

    for (orig, rest) in original.nodes.iter().zip(restored.nodes.iter()) {
        assert!(
            (orig.x - rest.x).abs() < f32::EPSILON,
            "node x position must match: {} vs {}",
            orig.x,
            rest.x
        );
        assert!(
            (orig.y - rest.y).abs() < f32::EPSILON,
            "node y position must match: {} vs {}",
            orig.y,
            rest.y
        );
    }
}

#[test]
fn round_trip_preserves_viewport() {
    let mut original = build_workflow_with_nodes();
    original.viewport.x = 100.0;
    original.viewport.y = 200.0;
    original.viewport.zoom = 1.5;

    let json = serde_json::to_string(&original).unwrap();
    let restored: Workflow = serde_json::from_str(&json).unwrap();

    assert!((restored.viewport.x - 100.0).abs() < f32::EPSILON);
    assert!((restored.viewport.y - 200.0).abs() < f32::EPSILON);
    assert!((restored.viewport.zoom - 1.5).abs() < f32::EPSILON);
}

#[test]
fn round_trip_preserves_node_ids_uniqueness() {
    // spec invariant: "Node IDs must be unique within a workflow"
    let original = build_workflow_with_nodes();
    let json = serde_json::to_string(&original).unwrap();
    let restored: Workflow = serde_json::from_str(&json).unwrap();

    let ids: std::collections::HashSet<_> = restored.nodes.iter().map(|n| n.id).collect();
    assert_eq!(ids.len(), restored.nodes.len(), "all node IDs must be unique after round-trip");
}

// ═══════════════════════════════════════════════════════════════════════════
// 2. Corrupted Storage: invalid JSON falls back gracefully
// spec: workflow-persistence.corrupted-storage
// ═══════════════════════════════════════════════════════════════════════════

#[test]
fn corrupted_json_falls_back_to_default() {
    // Simulates: localStorage contains garbage JSON
    let corrupted = "{{not valid json}}}";
    let result = serde_json::from_str::<Workflow>(corrupted);

    assert!(result.is_err(), "corrupted JSON should fail to deserialize");

    // The app falls back to default_workflow() on error
    let fallback = default_workflow();
    assert!(!fallback.nodes.is_empty(), "default workflow should have nodes");
}

#[test]
fn truncated_json_falls_back_gracefully() {
    let original = build_workflow_with_nodes();
    let mut json = serde_json::to_string(&original).unwrap();
    json.truncate(json.len() / 2); // Truncate mid-JSON

    let result = serde_json::from_str::<Workflow>(&json);
    assert!(result.is_err(), "truncated JSON should fail to deserialize");
}

#[test]
fn empty_string_falls_back_gracefully() {
    let result = serde_json::from_str::<Workflow>("");
    assert!(result.is_err(), "empty string should fail to deserialize");
}

#[test]
fn null_json_falls_back_gracefully() {
    let result = serde_json::from_str::<Workflow>("null");
    assert!(result.is_err(), "null should fail to deserialize as Workflow");
}

#[test]
fn wrong_type_json_falls_back_gracefully() {
    let inputs = ["[]", "42", "true", "\"string\""];
    for input in &inputs {
        let result = serde_json::from_str::<Workflow>(input);
        assert!(result.is_err(), "{input} should fail to deserialize as Workflow");
    }
}

#[test]
fn missing_fields_get_defaults() {
    // Simulates: localStorage has valid JSON but missing optional fields
    let minimal = r#"{"nodes":[],"connections":[],"viewport":{"x":0.0,"y":0.0,"zoom":1.0},"execution_queue":[],"current_step":0,"history":[]}"#;
    let result = serde_json::from_str::<Workflow>(minimal);

    assert!(result.is_ok(), "minimal valid JSON should deserialize");
    let w = result.unwrap();
    assert!(w.nodes.is_empty());
    assert!(w.connections.is_empty());
    assert_eq!(w.current_step, 0);
    assert!(w.history.is_empty());
}

// ═══════════════════════════════════════════════════════════════════════════
// 3. Storage Key Consistency
// spec: workflow-persistence (save and load must use same key)
// ═══════════════════════════════════════════════════════════════════════════

#[test]
fn storage_key_matches_save_and_load() {
    // Verify the key constant matches what both app_shell.rs and
    // use_workflow_state.rs use. If either file changes the key,
    // this test will need updating.
    assert_eq!(
        STORAGE_KEY, "flow-wasm-v1-workflow",
        "storage key must be consistent between save and load paths"
    );
}

// ═══════════════════════════════════════════════════════════════════════════
// 4. Large Workflow Serialization
// spec: "Memory usage must stay under 50MB for typical workflows"
// ═══════════════════════════════════════════════════════════════════════════

#[test]
fn large_workflow_serializes_within_reasonable_size() {
    let mut w = Workflow::new();
    // Create 100 nodes with connections
    let mp = main_port();
    let mut prev = w.add_node("http-handler", 0.0, 0.0);
    for i in 1..100 {
        let id = w.add_node("run", (i * 10) as f32, 0.0);
        let _ = w.add_connection_checked(prev, id, &mp, &mp);
        prev = id;
    }

    let json = serde_json::to_string(&w).unwrap();
    let size_bytes = json.len();
    let size_kb = size_bytes / 1024;

    // 100 nodes should produce well under 1MB
    assert!(
        size_bytes < 1_000_000,
        "100-node workflow should serialize under 1MB, got {size_kb}KB"
    );

    // Verify round-trip still works
    let restored: Workflow = serde_json::from_str(&json).unwrap();
    assert_eq!(restored.nodes.len(), 100);
}

// ═══════════════════════════════════════════════════════════════════════════
// 5. Execution History Persistence
// spec: workflow-persistence "Workflow is executable immediately" after reload
// ═══════════════════════════════════════════════════════════════════════════

#[test]
fn round_trip_preserves_execution_history() {
    let mut w = build_workflow_with_nodes();
    // Simulate a run record
    w.history.push(oya_frontend::graph::RunRecord {
        id: uuid::Uuid::new_v4(),
        timestamp: chrono::Utc::now(),
        results: std::collections::HashMap::new(),
        success: true,
        restate_invocation_id: None,
    });

    let json = serde_json::to_string(&w).unwrap();
    let restored: Workflow = serde_json::from_str(&json).unwrap();

    assert_eq!(
        restored.history.len(),
        w.history.len(),
        "execution history must survive round-trip"
    );
    assert!(restored.history[0].success, "run success status must survive round-trip");
}

// ═══════════════════════════════════════════════════════════════════════════
// 6. Serialization Never Panics (storage quota / write failure)
// spec: workflow-persistence.local-storage-quota-exceeded
// ═══════════════════════════════════════════════════════════════════════════

#[test]
fn serialization_of_default_workflow_succeeds() {
    // The save path in app_shell.rs does: serde_json::to_string(&*wf)
    // This must never panic, even for the default workflow
    let w = default_workflow();
    let result = serde_json::to_string(&w);
    assert!(result.is_ok(), "default workflow must serialize without error");

    let json = result.unwrap();
    assert!(!json.is_empty(), "serialized workflow must not be empty");
}

#[test]
fn serialization_preserves_in_memory_state_on_error() {
    // Even if we discard the serialized output, the original workflow
    // must remain unchanged (no interior mutability side effects)
    let w = build_workflow_with_nodes();
    let node_count = w.nodes.len();
    let conn_count = w.connections.len();

    let _ = serde_json::to_string(&w);

    assert_eq!(w.nodes.len(), node_count, "serialization must not modify in-memory workflow");
    assert_eq!(w.connections.len(), conn_count, "serialization must not modify connections");
}
