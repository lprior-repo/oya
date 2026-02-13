//! BDD Integration Test: Persistence workflow save and retrieve.
//!
//! This test validates the complete persistence lifecycle:
//! - GIVEN a persistence backend (InMemoryStorage)
//! - WHEN a workflow is saved
//! - THEN it can be retrieved correctly
//!
//! # Quality Standards
//!
//! - **Zero unwraps**: All errors use `Result` types with `?` operator
//! - **Zero panics**: No `panic!`, `unwrap()`, or `expect()` calls
//! - **BDD Style**: GIVEN-WHEN-THEN structure
//! - **Railway-Oriented Programming**: Proper error propagation throughout

#![deny(clippy::unwrap_used)]
#![deny(clippy::expect_used)]
#![deny(clippy::panic)]

use oya_workflow::{storage::InMemoryStorage, storage::WorkflowStorage, types::Phase, Workflow};
use std::time::Duration;

/// Test helper: Create a sample workflow for testing.
///
/// # Returns
///
/// A new Workflow with multiple phases.
fn create_sample_workflow(name: &str) -> Workflow {
    Workflow::new(name)
        .add_phase(Phase::new("build").with_timeout(Duration::from_secs(120)))
        .add_phase(Phase::new("test").with_timeout(Duration::from_secs(180)))
        .add_phase(Phase::new("deploy").with_timeout(Duration::from_secs(300)))
}

/// BDD Test: GIVEN persistence WHEN workflow saved THEN retrievable.
///
/// # Test Structure
///
/// - **GIVEN**: InMemoryStorage persistence backend initialized
/// - **WHEN**: A workflow is saved to storage
/// - **THEN**: The workflow can be retrieved and matches original
#[tokio::test]
async fn given_persistence_when_workflow_saved_then_retrievable() -> Result<(), String> {
    // GIVEN: Persistence backend initialized
    let storage = InMemoryStorage::new();
    let original_workflow = create_sample_workflow("test-workflow");
    let workflow_id = original_workflow.id;

    // WHEN: Workflow is saved to storage
    storage
        .save_workflow(&original_workflow)
        .await
        .map_err(|e| format!("Failed to save workflow: {}", e))?;

    // THEN: Workflow can be retrieved and matches original
    let loaded_workflow_opt = storage
        .load_workflow(workflow_id)
        .await
        .map_err(|e| format!("Failed to load workflow: {}", e))?;

    let loaded_workflow = loaded_workflow_opt
        .ok_or_else(|| "Expected to find workflow in storage but got None".to_string())?;

    // Verify workflow properties match
    assert_eq!(
        loaded_workflow.id, original_workflow.id,
        "Workflow ID should match"
    );
    assert_eq!(
        loaded_workflow.name, original_workflow.name,
        "Workflow name should match"
    );
    assert_eq!(
        loaded_workflow.phases().len(),
        original_workflow.phases().len(),
        "Number of phases should match"
    );

    // Verify all phase names match
    for (original_phase, loaded_phase) in original_workflow
        .phases()
        .iter()
        .zip(loaded_workflow.phases().iter())
    {
        assert_eq!(
            original_phase.name, loaded_phase.name,
            "Phase name should match"
        );
        assert_eq!(
            original_phase.timeout(), loaded_phase.timeout(),
            "Phase timeout should match for phase: {}",
            original_phase.name
        );
        assert_eq!(
            original_phase.retries(), loaded_phase.retries(),
            "Phase retries should match for phase: {}",
            original_phase.name
        );
    }

    // Verify state matches
    assert_eq!(
        loaded_workflow.state, original_workflow.state,
        "Workflow state should match"
    );

    Ok(())
}

/// BDD Test: GIVEN persistence WHEN multiple workflows saved THEN all retrievable.
///
/// # Test Structure
///
/// - **GIVEN**: InMemoryStorage persistence backend initialized
/// - **WHEN**: Multiple workflows are saved
/// - **THEN**: All workflows can be retrieved via list_workflows
#[tokio::test]
async fn given_persistence_when_multiple_workflows_saved_then_all_retrievable() -> Result<(), String>
{
    // GIVEN: Persistence backend initialized
    let storage = InMemoryStorage::new();

    // WHEN: Multiple workflows are saved
    let workflow_ids = vec![
        {
            let wf = create_sample_workflow("workflow-1");
            let id = wf.id;
            storage
                .save_workflow(&wf)
                .await
                .map_err(|e| format!("Failed to save workflow-1: {}", e))?;
            id
        },
        {
            let wf = create_sample_workflow("workflow-2");
            let id = wf.id;
            storage
                .save_workflow(&wf)
                .await
                .map_err(|e| format!("Failed to save workflow-2: {}", e))?;
            id
        },
        {
            let wf = create_sample_workflow("workflow-3");
            let id = wf.id;
            storage
                .save_workflow(&wf)
                .await
                .map_err(|e| format!("Failed to save workflow-3: {}", e))?;
            id
        },
    ];

    // THEN: All workflows can be retrieved
    let all_workflows = storage
        .list_workflows()
        .await
        .map_err(|e| format!("Failed to list workflows: {}", e))?;

    assert_eq!(
        all_workflows.len(),
        3,
        "Should have exactly 3 workflows in storage"
    );

    let stored_ids: Vec<_> = all_workflows.iter().map(|wf| wf.id).collect();

    // Verify all original workflow IDs are present
    for id in workflow_ids.iter() {
        assert!(
            stored_ids.contains(id),
            "Workflow ID {:?} should be in the list of stored workflows",
            id
        );
    }

    // Verify each workflow by ID
    for id in workflow_ids {
        let loaded = storage
            .load_workflow(id)
            .await
            .map_err(|e| format!("Failed to load workflow {:?}: {}", id, e))?
            .ok_or_else(|| format!("Workflow {:?} not found in storage", id))?;

        assert_eq!(loaded.id, id, "Loaded workflow ID should match");
    }

    Ok(())
}

/// BDD Test: GIVEN persistence WHEN workflow updated THEN latest_version_retrievable.
///
/// # Test Structure
///
/// - **GIVEN**: A workflow saved in storage
/// - **WHEN**: The workflow is updated (modified and re-saved)
/// - **THEN**: The latest version is retrievable
#[tokio::test]
async fn given_persistence_when_workflow_updated_then_latest_version_retrievable(
) -> Result<(), String> {
    // GIVEN: A workflow saved in storage
    let storage = InMemoryStorage::new();
    let mut workflow = create_sample_workflow("updatable-workflow");
    let workflow_id = workflow.id;

    storage
        .save_workflow(&workflow)
        .await
        .map_err(|e| format!("Failed to save initial workflow: {}", e))?;

    // WHEN: Workflow is updated (modified and re-saved)
    workflow
        .add_phase_mut(Phase::new("monitor").with_timeout(Duration::from_secs(60)));
    storage
        .save_workflow(&workflow)
        .await
        .map_err(|e| format!("Failed to save updated workflow: {}", e))?;

    // THEN: Latest version is retrievable
    let loaded = storage
        .load_workflow(workflow_id)
        .await
        .map_err(|e| format!("Failed to load updated workflow: {}", e))?
        .ok_or_else(|| "Workflow not found after update".to_string())?;

    assert_eq!(
        loaded.phases().len(),
        4,
        "Updated workflow should have 4 phases (original 3 + 1 added)"
    );

    assert_eq!(
        loaded.phases()[3].name, "monitor",
        "Fourth phase should be 'monitor'"
    );

    Ok(())
}

/// BDD Test: GIVEN persistence WHEN workflow_deleted THEN_not_retrievable.
///
/// # Test Structure
///
/// - **GIVEN**: A workflow saved in storage
/// - **WHEN**: The workflow is deleted
/// - **THEN**: It is no longer retrievable
#[tokio::test]
async fn given_persistence_when_workflow_deleted_then_not_retrievable() -> Result<(), String> {
    // GIVEN: A workflow saved in storage
    let storage = InMemoryStorage::new();
    let workflow = create_sample_workflow("deletable-workflow");
    let workflow_id = workflow.id;

    storage
        .save_workflow(&workflow)
        .await
        .map_err(|e| format!("Failed to save workflow: {}", e))?;

    // Verify it exists before deletion
    let before_delete = storage
        .load_workflow(workflow_id)
        .await
        .map_err(|e| format!("Failed to load workflow before delete: {}", e))?;

    assert!(
        before_delete.is_some(),
        "Workflow should exist before deletion"
    );

    // WHEN: Workflow is deleted
    storage
        .delete_workflow(workflow_id)
        .await
        .map_err(|e| format!("Failed to delete workflow: {}", e))?;

    // THEN: It is no longer retrievable
    let after_delete = storage
        .load_workflow(workflow_id)
        .await
        .map_err(|e| format!("Failed to load workflow after delete: {}", e))?;

    assert!(
        after_delete.is_none(),
        "Workflow should not exist after deletion"
    );

    Ok(())
}
