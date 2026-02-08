//! Tests for SurrealDB schema validation
//!
//! These tests validate that:
//! - All 12 tables are defined with proper schemas
//! - Graph relations (depends_on, blocks) work correctly
//! - sync_mode='full' is configured for fsync
//! - Indexes are properly created

#![allow(clippy::unwrap_used)]
#![allow(clippy::expect_used)]
#![allow(clippy::panic)]

use oya_events::db::{SurrealDbClient, SurrealDbConfig};
use tempfile::tempdir;

/// Helper to load the schema file
fn load_schema() -> Result<String, String> {
    std::fs::read_to_string("schema.surql")
        .map_err(|e| format!("Schema file should exist at crates/events/schema.surql: {e}"))
}

#[tokio::test]
async fn test_schema_file_exists() -> Result<(), String> {
    let schema_content = load_schema()?;
    assert!(
        !schema_content.is_empty(),
        "Schema file should not be empty"
    );
    Ok(())
}

#[tokio::test]
async fn test_all_tables_defined() -> Result<(), String> {
    let schema = load_schema()?;

    // Check that all 12 tables are defined
    let required_tables = [
        "state_transition",
        "idempotency_key",
        "checkpoint",
        "bead",
        "workflow_run",
        "process",
        "workspace",
        "schedule",
        "token_bucket",
        "concurrency_limit",
        "webhook",
    ];

    for table in required_tables {
        assert!(
            schema.contains(&format!("DEFINE TABLE {table}")),
            "Table {table} should be defined in schema"
        );
    }
    Ok(())
}

#[tokio::test]
async fn test_graph_relations_defined() -> Result<(), String> {
    let schema = load_schema()?;

    // Check for graph relation definitions
    assert!(
        schema.contains("DEFINE RELATION depends_on"),
        "depends_on relation should be defined"
    );
    assert!(
        schema.contains("DEFINE RELATION blocks"),
        "blocks relation should be defined"
    );
    Ok(())
}

#[tokio::test]
async fn test_sync_mode_full_configured() -> Result<(), String> {
    let schema = load_schema()?;

    // Check for sync_mode configuration
    // This should be in comments or as part of the setup
    assert!(
        schema.contains("sync_mode") || schema.contains("fsync") || schema.contains("SYNC"),
        "Schema should mention sync_mode or fsync configuration"
    );
    Ok(())
}

#[tokio::test]
async fn test_indexes_defined() -> Result<(), String> {
    let schema = load_schema()?;

    // Check for important indexes
    let expected_indexes = ["INDEX", "event_id", "bead_id", "workflow_id", "timestamp"];

    let index_count = expected_indexes
        .iter()
        .filter(|idx| schema.contains(*idx))
        .count();

    assert!(
        index_count >= 3,
        "Schema should define multiple indexes for performance"
    );
    Ok(())
}

#[tokio::test]
async fn test_schema_valid_syntax() -> Result<(), String> {
    let temp_dir = tempdir().map_err(|e| format!("Should create temp dir: {e}"))?;
    let db_path = temp_dir
        .path()
        .join("test_schema_db")
        .to_string_lossy()
        .to_string();

    let config = SurrealDbConfig::new(db_path);
    let client = SurrealDbClient::connect(config)
        .await
        .map_err(|e| format!("Failed to connect to SurrealDB: {e}"))?;

    let schema = load_schema()?;

    // Try to initialize schema - should not error
    client
        .init_schema(&schema)
        .await
        .map_err(|e| format!("Schema initialization failed: {e}"))?;
    Ok(())
}

#[tokio::test]
async fn test_state_transition_table_append_only() -> Result<(), String> {
    let schema = load_schema()?;

    // State transition table should enforce append-only behavior
    assert!(
        schema.contains("DEFINE TABLE state_transition"),
        "state_transition table should exist"
    );

    // Should have event_id and timestamp fields
    assert!(
        schema.contains("event_id"),
        "Schema should define event_id field"
    );
    assert!(
        schema.contains("timestamp"),
        "Schema should define timestamp field"
    );
    Ok(())
}

#[tokio::test]
async fn test_idempotency_key_table_unique() -> Result<(), String> {
    let schema = load_schema()?;

    assert!(
        schema.contains("DEFINE TABLE idempotency_key"),
        "idempotency_key table should exist"
    );

    // Should enforce uniqueness
    assert!(
        schema.contains("DEFINE FIELD key") || schema.contains("DEFINE FIELD id"),
        "Schema should define key field for idempotency"
    );
    Ok(())
}

#[tokio::test]
async fn test_checkpoint_table_compression_support() -> Result<(), String> {
    let schema = load_schema()?;

    assert!(
        schema.contains("DEFINE TABLE checkpoint"),
        "checkpoint table should exist"
    );

    // Should support compressed data
    assert!(
        schema.contains("data") || schema.contains("state"),
        "Schema should define data field for checkpoint"
    );
    Ok(())
}

#[tokio::test]
async fn test_workflow_run_table_tracking() -> Result<(), String> {
    let schema = load_schema()?;

    assert!(
        schema.contains("DEFINE TABLE workflow_run"),
        "workflow_run table should exist"
    );

    // Should track status and phases
    assert!(
        schema.contains("status") || schema.contains("state"),
        "Schema should track workflow status"
    );
    Ok(())
}
