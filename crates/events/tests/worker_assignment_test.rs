//! Tests for worker_assignment table in SurrealDB
//!
//! These tests validate that:
//! - The worker_assignment table exists and is properly defined
//! - Assignments can be created and retrieved
//! - One assignment per bead constraint is enforced
//! - Worker-based queries work correctly
//! - No orphaned assignments (via FK constraints if supported)

use oya_events::db::{SurrealDbClient, SurrealDbConfig};
use serde::{Deserialize, Serialize};
use tempfile::tempdir;

#[derive(Debug, Serialize, Deserialize, PartialEq)]
struct WorkerAssignment {
    assignment_id: String,
    bead_id: String,
    worker_id: String,
}

/// Helper to load and initialize the schema
async fn init_test_db() -> Result<SurrealDbClient, String> {
    let temp_dir = tempdir().map_err(|e| format!("Failed to create temp dir: {}", e))?;
    let db_path = temp_dir
        .path()
        .join("test_worker_assignment_db")
        .to_string_lossy()
        .to_string();

    let config = SurrealDbConfig::new(db_path);
    let client = SurrealDbClient::connect(config)
        .await
        .map_err(|e| format!("Failed to connect to SurrealDB: {}", e))?;

    let schema = std::fs::read_to_string("schema.surql")
        .map_err(|e| format!("Failed to read schema: {}", e))?;
    client
        .init_schema(&schema)
        .await
        .map_err(|e| format!("Schema initialization failed: {}", e))?;

    Ok(client)
}

#[tokio::test]
async fn test_worker_assignment_table_exists() -> Result<(), String> {
    let client = init_test_db().await?;

    // Query table information - if it executes, table exists
    let _result = client
        .client()
        .query("SELECT * FROM worker_assignment LIMIT 0")
        .await
        .map_err(|e| format!("Query should execute: {}", e))?;

    Ok(())
}

#[tokio::test]
async fn test_worker_assignment_create() -> Result<(), String> {
    let client = init_test_db().await?;

    // Create a worker assignment
    client
        .client()
        .query(
            "CREATE worker_assignment CONTENT {
                assignment_id: 'assign-1',
                bead_id: 'bead-123',
                worker_id: 'worker-a',
                assigned_at: time::now()
            }",
        )
        .await
        .map_err(|e| format!("Query should execute: {e}"))?;

    Ok(())
}

#[tokio::test]
async fn test_worker_assignment_query_by_bead_id() -> Result<(), String> {
    let client = init_test_db().await?;

    // Create an assignment
    client
        .client()
        .query(
            "CREATE worker_assignment CONTENT {
                assignment_id: 'assign-2',
                bead_id: 'bead-456',
                worker_id: 'worker-b'
            }",
        )
        .await
        .map_err(|e| format!("Create should succeed: {e}"))?;

    // Query by bead_id
    let mut result = client
        .client()
        .query("SELECT assignment_id, bead_id, worker_id FROM worker_assignment WHERE bead_id = 'bead-456'")
        .await
        .map_err(|e| format!("Query should execute: {e}"))?;

    // Check that we got results
    let assignments: Vec<WorkerAssignment> = result
        .take(0)
        .map_err(|e| format!("Should get result: {e}"))?;
    assert!(
        !assignments.is_empty(),
        "Should find assignment for bead-456"
    );
    assert_eq!(assignments[0].bead_id, "bead-456");
    Ok(())
}

#[tokio::test]
async fn test_worker_assignment_query_by_worker_id() -> Result<(), String> {
    let client = init_test_db().await?;

    // Create multiple assignments for same worker
    client
        .client()
        .query(
            "CREATE worker_assignment CONTENT {
                assignment_id: 'assign-3a',
                bead_id: 'bead-789',
                worker_id: 'worker-c'
            }",
        )
        .await
        .map_err(|e| format!("Create should succeed: {e}"))?;

    client
        .client()
        .query(
            "CREATE worker_assignment CONTENT {
                assignment_id: 'assign-3b',
                bead_id: 'bead-101',
                worker_id: 'worker-c'
            }",
        )
        .await
        .map_err(|e| format!("Create should succeed: {e}"))?;

    // Query by worker_id
    let mut result = client
        .client()
        .query("SELECT assignment_id, bead_id, worker_id FROM worker_assignment WHERE worker_id = 'worker-c'")
        .await
        .map_err(|e| format!("Query should execute: {e}"))?;

    // Check that we got results
    let assignments: Vec<WorkerAssignment> = result
        .take(0)
        .map_err(|e| format!("Should get result: {e}"))?;
    assert!(
        assignments.len() >= 2,
        "Should find at least 2 assignments for worker-c"
    );
    Ok(())
}

#[tokio::test]
async fn test_worker_assignment_uniqueness_per_bead() -> Result<(), String> {
    let client = init_test_db().await?;

    // Create first assignment
    client
        .client()
        .query(
            "CREATE worker_assignment CONTENT {
                assignment_id: 'assign-4',
                bead_id: 'bead-unique',
                worker_id: 'worker-a'
            }",
        )
        .await
        .map_err(|e| format!("First create should succeed: {e}"))?;

    // Try to create second assignment with same bead_id (should fail)
    let response = client
        .client()
        .query(
            "INSERT INTO worker_assignment {
                assignment_id: 'assign-5',
                bead_id: 'bead-unique',
                worker_id: 'worker-b'
            }",
        )
        .await
        .map_err(|e| e.to_string())?;

    // The unique constraint on bead_id should prevent duplicate
    assert!(
        response.check().is_err(),
        "Second assignment with same bead_id should fail due to unique constraint"
    );
    Ok(())
}

#[tokio::test]
async fn test_worker_assignment_update() -> Result<(), String> {
    let client = init_test_db().await?;

    // Create assignment
    client
        .client()
        .query(
            "CREATE worker_assignment CONTENT {
                assignment_id: 'assign-6',
                bead_id: 'bead-timestamp',
                worker_id: 'worker-d'
            }",
        )
        .await
        .map_err(|e| format!("Create should succeed: {e}"))?;

    // Update worker_id using MERGE to avoid overwriting bead_id
    client
        .client()
        .query(
            "UPDATE worker_assignment MERGE {
                worker_id: 'worker-e',
                updated_at: time::now()
            } WHERE assignment_id = 'assign-6'",
        )
        .await
        .map_err(|e| format!("Update should execute: {e}"))?;

    // Verify update
    let mut result = client
        .client()
        .query("SELECT assignment_id, bead_id, worker_id FROM worker_assignment WHERE assignment_id = 'assign-6'")
        .await
        .map_err(|e| format!("Query should execute: {e}"))?;

    let assignments: Vec<WorkerAssignment> = result
        .take(0)
        .map_err(|e| format!("Should get result: {e}"))?;
    assert_eq!(assignments.len(), 1, "Should find exactly one assignment");
    assert_eq!(assignments[0].worker_id, "worker-e", "Worker ID should be updated");
    
    Ok(())
}

#[tokio::test]
async fn test_worker_assignment_delete() -> Result<(), String> {
    let client = init_test_db().await?;

    // Create assignment
    client
        .client()
        .query(
            "CREATE worker_assignment CONTENT {
                assignment_id: 'assign-7',
                bead_id: 'bead-delete',
                worker_id: 'worker-f'
            }",
        )
        .await
        .map_err(|e| format!("Create should succeed: {e}"))?;

    // Delete assignment
    client
        .client()
        .query("DELETE worker_assignment WHERE bead_id = 'bead-delete'")
        .await
        .map_err(|e| format!("Delete should execute: {e}"))?;

    // Verify deletion
    let mut result = client
        .client()
        .query("SELECT * FROM worker_assignment WHERE bead_id = 'bead-delete'")
        .await
        .map_err(|e| format!("Query should execute: {e}"))?;

    let assignments: Vec<WorkerAssignment> = result
        .take(0)
        .map_err(|e| format!("Should get result: {e}"))?;
    assert!(assignments.is_empty(), "Assignment should be deleted");
    Ok(())
}

#[tokio::test]
async fn test_worker_assignment_multiple_workers_distribution() -> Result<(), String> {
    let client = init_test_db().await?;

    // Create multiple assignments across different workers
    let workers = vec!["worker-g", "worker-h", "worker-i"];
    let bead_ids = vec!["bead-g1", "bead-h1", "bead-i1"];

    for (i, (worker, bead)) in workers.iter().zip(bead_ids.iter()).enumerate() {
        client
            .client()
            .query(format!(
                "CREATE worker_assignment CONTENT {{
                    assignment_id: 'assign-dist-{}',
                    bead_id: '{}',
                    worker_id: '{}'
                }}",
                i, bead, worker
            ))
            .await
            .map_err(|e| format!("Create should succeed: {e}"))?;
    }

    // Query all assignments
    let mut result = client
        .client()
        .query("SELECT assignment_id, bead_id, worker_id FROM worker_assignment")
        .await
        .map_err(|e| format!("Query should execute: {e}"))?;

    let assignments: Vec<WorkerAssignment> = result
        .take(0)
        .map_err(|e| format!("Should get result: {e}"))?;
    assert!(
        assignments.len() >= 3,
        "Should retrieve at least 3 worker assignments, got {}",
        assignments.len()
    );
    Ok(())
}

#[tokio::test]
async fn test_worker_assignment_bead_id_unique_index() -> Result<(), String> {
    let client = init_test_db().await?;

    // Create first assignment
    client
        .client()
        .query(
            "CREATE worker_assignment CONTENT {
                assignment_id: 'assign-index-1',
                bead_id: 'bead-index-test',
                worker_id: 'worker-j'
            }",
        )
        .await
        .map_err(|e| format!("First create should succeed: {e}"))?;

    // Try to create duplicate bead_id
    let response = client
        .client()
        .query(
            "INSERT INTO worker_assignment {
                assignment_id: 'assign-index-2',
                bead_id: 'bead-index-test',
                worker_id: 'worker-k'
            }",
        )
        .await
        .map_err(|e| e.to_string())?;

    // Should fail due to unique index on bead_id
    assert!(
        response.check().is_err(),
        "Duplicate bead_id should violate unique constraint"
    );
    Ok(())
}
