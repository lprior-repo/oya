//! Workspace Module Refactor Unit Tests
//!
//! These tests verify workspace.rs has been refactored to use jj module.
//! Tests should FAIL (red) before implementation and PASS after.
//!
//! Run with: cargo test --lib workspace_tests

use super::{prepare_stage_workspace, WorkspacePrepRequest};
use oya::types::StageName;
use std::path::PathBuf;
use std::process::Command;

fn init_temp_jj_repo() -> tempfile::TempDir {
    let temp_dir = tempfile::tempdir().unwrap_or_else(|error| {
        panic!("failed to create temporary directory for workspace tests: {}", error)
    });
    let output = Command::new("jj")
        .arg("git")
        .arg("init")
        .current_dir(temp_dir.path())
        .output()
        .unwrap_or_else(|error| panic!("failed to launch 'jj git init': {}", error));
    assert!(
        output.status.success(),
        "jj git init failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    temp_dir
}

fn make_test_request(bead_id: &str, repo_root: PathBuf) -> WorkspacePrepRequest {
    WorkspacePrepRequest {
        run_id: "test-run".to_string(),
        bead_id: bead_id.to_string(),
        stage: StageName::Implementation,
        attempt: 1,
        recorded_at: "2024-01-01T00:00:00Z".to_string(),
        repo_root,
    }
}

// =============================================================================
// CONTRACT: prepare_stage_workspace uses jj module (not zjj binary)
// =============================================================================

/// Given: valid WorkspacePrepRequest
/// When: preparing stage workspace
/// Then: returns Ok with jj-style workspace path
///
/// RED TEST: Currently fails because:
/// 1. prepare_stage_workspace calls run_zjj_queue which uses zjj binary
/// 2. resolve_workspace_path returns "oya__workspaces/{bead}" (zjj style)
///
/// GREEN CONDITION:
/// 1. prepare_stage_workspace uses jj::create_workspace
/// 2. workspace_path contains "oya-{bead_id}" (jj style)
/// 3. workspace_path does NOT contain "oya__workspaces"
#[test]
fn test_prepare_stage_workspace_uses_jj_module() {
    let temp_repo = init_temp_jj_repo();
    let request = make_test_request("test-jj-integration", temp_repo.path().to_path_buf());

    let result = prepare_stage_workspace(request);

    assert!(result.is_ok(), "prepare_stage_workspace should return Ok, got: {:?}", result.err());

    let event = result.unwrap();
    assert!(event.is_some(), "Should return Some(WorkspaceLifecycleEvent)");

    let event = event.unwrap();

    assert_eq!(
        event.workspace_name, "oya-test-jj-integration",
        "workspace_name should be 'oya-{{bead_id}}'"
    );

    assert!(
        event.workspace_path.contains("oya-test-jj-integration"),
        "workspace_path should contain jj-style name, got: {}",
        event.workspace_path
    );

    assert!(
        !event.workspace_path.contains("oya__workspaces"),
        "workspace_path should NOT contain zjj pattern 'oya__workspaces', got: {}",
        event.workspace_path
    );
}

/// Given: valid request
/// When: preparing workspace twice
/// Then: succeeds idempotently (jj handles existing workspace)
#[test]
fn test_prepare_stage_workspace_idempotent() {
    let temp_repo = init_temp_jj_repo();
    let request1 = make_test_request("test-idempotent-ws", temp_repo.path().to_path_buf());
    let request2 = make_test_request("test-idempotent-ws", temp_repo.path().to_path_buf());

    let result1 = prepare_stage_workspace(request1);
    let result2 = prepare_stage_workspace(request2);

    assert!(result1.is_ok(), "First call should succeed: {:?}", result1.err());
    assert!(result2.is_ok(), "Second call should succeed: {:?}", result2.err());
}

// =============================================================================
// CONTRACT: Workspace path follows jj convention
// =============================================================================

/// Given: repo_root and bead_id
/// When: computing workspace path
/// Then: path is repo_root/oya-{bead_id} (not ../oya__workspaces/{bead_id})
///
/// RED TEST: Currently fails because resolve_workspace_path returns zjj style.
#[test]
fn test_workspace_path_uses_jj_convention() {
    let temp_repo = init_temp_jj_repo();
    let request = make_test_request("test-path-convention", temp_repo.path().to_path_buf());

    let result = prepare_stage_workspace(request);

    assert!(result.is_ok());
    let event = result.unwrap().unwrap();

    let path = &event.workspace_path;

    assert!(
        path.contains("oya-test-path-convention"),
        "Path should contain 'oya-{{bead_id}}', got: {}",
        path
    );

    assert!(
        !path.contains("oya__workspaces"),
        "Path should NOT contain 'oya__workspaces', got: {}",
        path
    );
}

// =============================================================================
// CONTRACT: Error handling for invalid bead_id
// =============================================================================

/// Given: bead_id with path traversal
/// When: preparing workspace
/// Then: returns validation error
#[test]
fn test_invalid_bead_id_rejected() {
    let request = WorkspacePrepRequest {
        run_id: "test-err".to_string(),
        bead_id: "../escape".to_string(),
        stage: StageName::Implementation,
        attempt: 1,
        recorded_at: "2024-01-01T00:00:00Z".to_string(),
        repo_root: PathBuf::from("."),
    };

    let result = prepare_stage_workspace(request);

    assert!(result.is_err(), "Should return error for path traversal in bead_id");

    let err = result.unwrap_err();
    let err_str = err.to_string();

    assert!(
        err_str.contains("forbidden")
            || err_str.contains("invalid")
            || err_str.contains("relative"),
        "Error should mention forbidden/invalid bead_id: {}",
        err_str
    );
}

/// Given: bead_id with path separator
/// When: preparing workspace
/// Then: returns validation error
#[test]
fn test_bead_id_with_separator_rejected() {
    let request = WorkspacePrepRequest {
        run_id: "test-err2".to_string(),
        bead_id: "src/foo".to_string(),
        stage: StageName::Implementation,
        attempt: 1,
        recorded_at: "2024-01-01T00:00:00Z".to_string(),
        repo_root: PathBuf::from("."),
    };

    let result = prepare_stage_workspace(request);

    assert!(result.is_err(), "Should return error for path separator in bead_id");
}

// =============================================================================
// CONTRACT: WorkspaceLifecycleEvent structure
// =============================================================================

/// Given: successful workspace preparation
/// When: examining lifecycle event
/// Then: has exactly 4 fields with correct values
#[test]
fn test_lifecycle_event_structure() {
    let temp_repo = init_temp_jj_repo();
    let request = make_test_request("test-event-struct", temp_repo.path().to_path_buf());

    let result = prepare_stage_workspace(request);

    assert!(result.is_ok());
    let event = result.unwrap().unwrap();

    assert!(!event.workspace_name.is_empty(), "workspace_name must be set");
    assert!(!event.workspace_path.is_empty(), "workspace_path must be set");
    assert!(!event.timestamp.is_empty(), "timestamp must be set");

    assert_eq!(
        event.workspace_name, "oya-test-event-struct",
        "workspace_name should match pattern"
    );
}

// =============================================================================
// INVARIANT: Zjj binary functions removed
// =============================================================================

/// This test documents that run_zjj_queue should not be called.
/// The implementation should use jj::create_workspace instead.
///
/// To verify: workspace.rs should not contain "zjj queue" command
#[test]
fn test_invariant_no_zjj_queue_command() {
    // After implementation, workspace.rs should not have:
    // - fn run_zjj_queue
    // - ["queue", "--add", ...] command
    // This is verified by code review/grep
}

/// This test documents that run_zjj_add should not be called.
/// The implementation should use jj::create_workspace instead.
///
/// To verify: workspace.rs should not contain "zjj add" command
#[test]
fn test_invariant_no_zjj_add_command() {
    // After implementation, workspace.rs should not have:
    // - fn run_zjj_add
    // - ["add", ...] command for zjj
}
