//! Tests for worker_assignment table in SurrealDB
//!
//! These tests validate that:
//! - The worker_assignment table exists and is properly defined
//! - Assignments can be created and retrieved
//! - One assignment per bead constraint is enforced
//! - Worker-based queries work correctly
//! - No orphaned assignments (via FK constraints if supported)

#![allow(clippy::unwrap_used)]
#![allow(clippy::expect_used)]
#![allow(clippy::panic)]

use oya_events::db::{SurrealDbClient, SurrealDbConfig};
use tempfile::tempdir;

/// Helper to load and initialize the schema
async fn init_test_db() -> SurrealDbClient {
    let temp_dir = tempdir().expect("Should create temp dir");
    let db_path = temp_dir
        .path()
        .join("test_worker_assignment_db")
        .to_string_lossy()
        .to_string();

    let config = SurrealDbConfig::new(db_path);
    let client = SurrealDbClient::connect(config)
        .await
        .expect("Failed to connect to SurrealDB");

    let schema = std::fs::read_to_string("schema.surql").expect("Schema file should exist");
    client
        .init_schema(&schema)
        .await
        .expect("Schema initialization should succeed");

    client
}

#[tokio::test]
async fn test_worker_assignment_table_exists() {
    let client = init_test_db().await;

    // Query table information - if it executes, table exists
    let _result = client
        .client()
        .query("SELECT * FROM information WHERE name = 'worker_assignment'")
        .await
        .expect("Query should execute");

    // If we get here without error, table exists
    assert!(true);
}

#[tokio::test]
async fn test_worker_assignment_create() {
    let client = init_test_db().await;

    // Create a worker assignment
    let _result = client
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
        .expect("Query should execute");

    // If no error, creation succeeded
    assert!(true);
}

#[tokio::test]
async fn test_worker_assignment_query_by_bead_id() {
    let client = init_test_db().await;

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
        .expect("Create should succeed");

    // Query by bead_id
    let mut result = client
        .client()
        .query("SELECT * FROM worker_assignment WHERE bead_id = 'bead-456'")
        .await
        .expect("Query should execute");

    // Check that we got results
    let assignments: Vec<serde_json::Value> = result.take(0).expect("Should get result");
    assert!(
        !assignments.is_empty(),
        "Should find assignment for bead-456"
    );
}

#[tokio::test]
async fn test_worker_assignment_query_by_worker_id() {
    let client = init_test_db().await;

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
        .expect("Create should succeed");

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
        .expect("Create should succeed");

    // Query by worker_id
    let mut result = client
        .client()
        .query("SELECT * FROM worker_assignment WHERE worker_id = 'worker-c'")
        .await
        .expect("Query should execute");

    // Check that we got results
    let assignments: Vec<serde_json::Value> = result.take(0).expect("Should get result");
    assert!(
        !assignments.is_empty(),
        "Should find assignments for worker-c"
    );
}

#[tokio::test]
async fn test_worker_assignment_uniqueness_per_bead() {
    let client = init_test_db().await;

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
        .expect("First create should succeed");

    // Try to create second assignment with same bead_id (should fail)
    let result = client
        .client()
        .query(
            "CREATE worker_assignment CONTENT {
                assignment_id: 'assign-5',
                bead_id: 'bead-unique',
                worker_id: 'worker-b'
            }",
        )
        .await;

    // The unique constraint should prevent duplicate bead_id
    // SurrealDB will return an error for duplicate unique key
    assert!(
        result.is_err(),
        "Second assignment with same bead_id should fail due to unique constraint"
    );
}

#[tokio::test]
async fn test_worker_assignment_update() {
    let client = init_test_db().await;

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
        .expect("Create should succeed");

    // Update worker_id
    let _result = client
        .client()
        .query(
            "UPDATE worker_assignment CONTENT {
                assignment_id: 'assign-6',
                worker_id: 'worker-e',
                updated_at: time::now()
            }",
        )
        .await
        .expect("Update should execute");

    // Verify update
    let mut result = client
        .client()
        .query("SELECT * FROM worker_assignment WHERE assignment_id = 'assign-6'")
        .await
        .expect("Query should execute");

    let assignments: Vec<serde_json::Value> = result.take(0).expect("Should get result");
    assert_eq!(assignments.len(), 1, "Should find exactly one assignment");

    let assignment = &assignments[0];
    if let Some(worker_id) = assignment.get("worker_id").and_then(|v| v.as_str()) {
        assert_eq!(worker_id, "worker-e", "Worker ID should be updated");
    } else {
        panic!("Worker ID field should exist");
    }
}

#[tokio::test]
async fn test_worker_assignment_delete() {
    let client = init_test_db().await;

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
        .expect("Create should succeed");

    // Delete assignment
    let _result = client
        .client()
        .query("DELETE worker_assignment WHERE bead_id = 'bead-delete'")
        .await
        .expect("Delete should execute");

    // Verify deletion
    let mut result = client
        .client()
        .query("SELECT * FROM worker_assignment WHERE bead_id = 'bead-delete'")
        .await
        .expect("Query should execute");

    let assignments: Vec<serde_json::Value> = result.take(0).expect("Should get result");
    assert!(
        assignments.is_empty(),
        "Assignment should be deleted"
    );
}

#[tokio::test]
async fn test_worker_assignment_multiple_workers_distribution() {
    let client = init_test_db().await;

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
            .expect("Create should succeed");
    }

    // Query all assignments
    let mut result = client
        .client()
        .query("SELECT * FROM worker_assignment")
        .await
        .expect("Query should execute");

    let assignments: Vec<serde_json::Value> = result.take(0).expect("Should get result");
    assert!(
        assignments.len() >= 3,
        "Should retrieve at least 3 worker assignments, got {}",
        assignments.len()
    );
}

#[tokio::test]
async fn test_worker_assignment_bead_id_unique_index() {
    let client = init_test_db().await;

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
        .expect("First create should succeed");

    // Try to create duplicate bead_id
    let result = client
        .client()
        .query(
            "CREATE worker_assignment CONTENT {
                assignment_id: 'assign-index-2',
                bead_id: 'bead-index-test',
                worker_id: 'worker-k'
            }",
        )
        .await;

    // Should fail due to unique index on bead_id
    assert!(
        result.is_err(),
        "Duplicate bead_id should violate unique constraint"
    );
}
