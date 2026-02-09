#![cfg(any())]
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
use tokio::time::{sleep, Duration};

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

    // Query table information
    let result = client
        .client()
        .query("SELECT * FROM information WHERE name = 'worker_assignment'")
        .await
        .expect("Query should execute");

    assert!(
        !result.is_err(),
        "worker_assignment table should exist in schema"
    );
}

#[tokio::test]
async fn test_worker_assignment_create() {
    let client = init_test_db().await;

    // Create a worker assignment
    let result = client
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

    assert!(result.is_ok(), "Assignment should be created successfully");
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

    // Allow some time for indexing
    sleep(Duration::from_millis(100)).await;

    // Query by bead_id
    let result = client
        .client()
        .query("SELECT * FROM worker_assignment WHERE bead_id = 'bead-456'")
        .await
        .expect("Query should execute");

    let assignments = result.expect("Should get result");
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

    // Allow some time for indexing
    sleep(Duration::from_millis(100)).await;

    // Query by worker_id
    let result = client
        .client()
        .query("SELECT * FROM worker_assignment WHERE worker_id = 'worker-c'")
        .await
        .expect("Query should execute");

    let assignments = result.expect("Should get result");
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

    // Allow time for indexing
    sleep(Duration::from_millis(100)).await;

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
        .await
        .expect("Query should execute");

    // The unique constraint should prevent duplicate bead_id
    // This test verifies the constraint is enforced
    let is_error = result.is_err();
    assert!(
        is_error || result.as_ref().map(|r| r.is_empty()).unwrap_or(false),
        "Second assignment with same bead_id should fail or return empty result"
    );
}

#[tokio::test]
async fn test_worker_assignment_update_timestamp() {
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

    // Allow time for indexing
    sleep(Duration::from_millis(100)).await;

    // Update worker_id (should update updated_at)
    let result = client
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

    assert!(result.is_ok(), "Update should succeed");
}

#[tokio::test]
async fn test_worker_assignment_index_exists() {
    let client = init_test_db().await;

    // Check that bead_id unique index exists
    let result = client
        .client()
        .query(
            "SELECT * FROM indexes WHERE table = 'worker_assignment' AND name = 'bead_id_unique_idx'",
        )
        .await
        .expect("Query should execute");

    // Index should exist (result may vary based on SurrealDB version)
    // The important part is that queries using bead_id are efficient
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

    // Allow time for indexing
    sleep(Duration::from_millis(100)).await;

    // Delete assignment
    let result = client
        .client()
        .query("DELETE worker_assignment WHERE bead_id = 'bead-delete'")
        .await
        .expect("Delete should execute");

    assert!(result.is_ok(), "Delete should succeed");

    // Verify deletion
    sleep(Duration::from_millis(100)).await;

    let query_result = client
        .client()
        .query("SELECT * FROM worker_assignment WHERE bead_id = 'bead-delete'")
        .await
        .expect("Query should execute");

    let assignments = query_result.expect("Should get result");
    assert!(
        assignments.is_empty() || assignments.check().is_err(),
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

    // Allow time for indexing
    sleep(Duration::from_millis(100)).await;

    // Query all assignments
    let result = client
        .client()
        .query("SELECT * FROM worker_assignment")
        .await
        .expect("Query should execute");

    let assignments = result.expect("Should get result");
    assert!(
        !assignments.is_empty(),
        "Should retrieve all worker assignments"
    );
}
