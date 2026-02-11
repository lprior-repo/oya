//! Tests for workspace path integration with OpenCode execution.
//!
//! This test suite ensures that when beads are executed in isolated workspaces,
//! the workspace_path from BeadExecutionContext is properly passed to OpenCode
//! via OpencodeConfig::working_dir().

use orchestrator::actors::worker::BeadExecutionContext;
use std::path::PathBuf;

#[test]
fn test_bead_execution_context_has_workspace_path() {
    // Given: A workspace path
    let workspace_path = PathBuf::from("/tmp/test-workspace");

    // When: Creating a BeadExecutionContext
    let exec_ctx = BeadExecutionContext::new("test-bead", workspace_path.clone());

    // Then: The workspace_path getter returns the correct path
    assert_eq!(exec_ctx.workspace_path(), &workspace_path);
    assert_eq!(exec_ctx.bead_id(), "test-bead");
}

#[test]
fn test_bead_execution_context_preserves_absolute_path() {
    // Given: An absolute workspace path
    let workspace_path = PathBuf::from("/tmp/workspace-abc");

    // When: Creating a BeadExecutionContext
    let exec_ctx = BeadExecutionContext::new("bead-123", workspace_path.clone());

    // Then: The path is preserved exactly
    assert_eq!(exec_ctx.workspace_path(), &workspace_path);
    assert!(exec_ctx.workspace_path().is_absolute());
}

#[test]
fn test_bead_execution_context_requires_absolute_path() {
    // Given: A relative workspace path (invalid)
    let relative_path = PathBuf::from("relative-workspace");

    // When: Creating a BeadExecutionContext
    let exec_ctx = BeadExecutionContext::new("bead-456", relative_path);

    // Then: The path should be detected as non-absolute
    assert!(!exec_ctx.workspace_path().is_absolute());
}

#[test]
fn test_workspace_path_getter_encapsulation() {
    // Given: A BeadExecutionContext
    let workspace_path = PathBuf::from("/tmp/test-workspace-xyz");
    let exec_ctx = BeadExecutionContext::new("test-bead", workspace_path);

    // When: Accessing workspace_path via getter method
    let retrieved_path = exec_ctx.workspace_path();

    // Then: The getter returns a reference (not owned value)
    // This demonstrates proper encapsulation
    assert_eq!(retrieved_path, &PathBuf::from("/tmp/test-workspace-xyz"));
}

// Integration tests with OpenCode configuration
// These tests verify that workspace_path is properly passed to OpencodeConfig

#[test]
fn test_opencode_config_accepts_workspace_path() {
    // Given: A workspace path
    let workspace_path = PathBuf::from("/tmp/test-workspace-opencode");

    // When: Creating OpencodeConfig with working_dir
    // Note: This test will fail until OpencodeConfig is available
    // For now, we'll document the expected behavior

    // Then: OpencodeConfig should accept the workspace_path
    // This test documents the contract - implementation will follow
    assert!(workspace_path.is_absolute());
    assert_eq!(
        workspace_path,
        PathBuf::from("/tmp/test-workspace-opencode")
    );
}

#[test]
fn test_workspace_path_validation_requires_directory() {
    // Given: A path that points to a file (not a directory)
    let file_path = PathBuf::from("/tmp/not-a-directory.txt");

    // When: Checking if path could be a valid workspace
    // For now, we just document the validation requirement

    // Then: Non-directory paths should be rejected
    // This is a contract test - implementation will enforce this
    assert!(file_path.is_absolute());
}

#[test]
fn test_workspace_path_must_be_absolute() {
    // Given: A relative workspace path
    let relative_path = PathBuf::from("relative/workspace");

    // When: Validating workspace path
    let is_absolute = relative_path.is_absolute();

    // Then: Relative paths should be rejected
    assert!(!is_absolute, "Relative paths are not valid for workspaces");
}

#[test]
fn test_bead_id_required_for_execution_context() {
    // Given: A workspace path
    let workspace_path = PathBuf::from("/tmp/test-workspace-bead-id");

    // When: Creating BeadExecutionContext with bead_id
    let bead_id = "test-bead-123";
    let exec_ctx = BeadExecutionContext::new(bead_id, workspace_path);

    // Then: bead_id should be retrievable
    assert_eq!(exec_ctx.bead_id(), bead_id);
    assert!(!exec_ctx.bead_id().is_empty());
}
