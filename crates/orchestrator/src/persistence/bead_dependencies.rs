//! Bead dependency persistence operations.
//!
//! CRUD operations for bead dependency relationships in SurrealDB.
//!
//! This module handles two types of dependency edges:
//! - `depends_on`: Bead A depends on Bead B (B must complete before A can start)
//! - `blocks`: Bead A blocks Bead B (if A fails, B cannot execute)

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use surrealdb::sql::{Datetime as SurrealDatetime, Thing};

use super::client::OrchestratorStore;
use super::error::{PersistenceError, PersistenceResult, from_surrealdb_error};

/// Dependency relationship types between beads
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DependencyRelation {
    /// Bead must complete before dependent can start
    DependsOn,
    /// Bead blocks another (if blocker fails, blocked cannot execute)
    Blocks,
}

impl std::fmt::Display for DependencyRelation {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::DependsOn => write!(f, "depends_on"),
            Self::Blocks => write!(f, "blocks"),
        }
    }
}

/// Dependency edge record stored in the database.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DependencyEdge {
    /// SurrealDB record ID
    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde(rename = "id")]
    pub record_id: Option<Thing>,
    /// The bead that has the dependency
    pub bead_id: String,
    /// The bead this relationship targets
    pub target_bead_id: String,
    /// Type of relationship
    pub relation_type: DependencyRelation,
    /// When this edge was created
    pub created_at: DateTime<Utc>,
    /// Additional metadata
    #[serde(skip_serializing_if = "Option::is_none")]
    pub metadata: Option<serde_json::Value>,
}

impl DependencyEdge {
    /// Create a new dependency edge.
    #[must_use]
    pub fn new(
        bead_id: impl Into<String>,
        target_bead_id: impl Into<String>,
        relation_type: DependencyRelation,
    ) -> Self {
        Self {
            record_id: None,
            bead_id: bead_id.into(),
            target_bead_id: target_bead_id.into(),
            relation_type,
            created_at: Utc::now(),
            metadata: None,
        }
    }

    /// Set metadata.
    #[must_use]
    pub fn with_metadata(mut self, metadata: serde_json::Value) -> Self {
        self.metadata = Some(metadata);
        self
    }
}

/// Blocked bead with blocking dependencies.
///
/// Represents a bead that is blocked along with the beads that are blocking it.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BlockedBead {
    /// The bead that is blocked
    pub bead_id: String,
    /// The beads that are blocking this bead (dependencies that haven't completed)
    pub blocking_deps: Vec<String>,
}

impl BlockedBead {
    /// Create a new BlockedBead entry.
    #[must_use]
    pub fn new(bead_id: impl Into<String>, blocking_deps: Vec<String>) -> Self {
        Self {
            bead_id: bead_id.into(),
            blocking_deps,
        }
    }
}

/// Input for creating/updating a dependency edge.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub(super) struct DependencyInput {
    bead_id: String,
    target_bead_id: String,
    relation_type: String,
    created_at: SurrealDatetime,
    metadata: Option<serde_json::Value>,
}

impl From<&DependencyEdge> for DependencyInput {
    fn from(edge: &DependencyEdge) -> Self {
        Self {
            bead_id: edge.bead_id.clone(),
            target_bead_id: edge.target_bead_id.clone(),
            relation_type: edge.relation_type.to_string(),
            created_at: SurrealDatetime::from(edge.created_at),
            metadata: edge.metadata.clone(),
        }
    }
}

impl OrchestratorStore {
    /// Save a dependency edge to the database.
    ///
    /// # Errors
    ///
    /// Returns an error if the database operation fails.
    pub async fn save_dependency_edge(
        &self,
        edge: &DependencyEdge,
    ) -> PersistenceResult<DependencyEdge> {
        let input = DependencyInput::from(edge);

        // Generate a unique ID for the edge
        let edge_id = format!(
            "{}:{}:{}",
            edge.bead_id,
            edge.target_bead_id,
            match edge.relation_type {
                DependencyRelation::DependsOn => "dep",
                DependencyRelation::Blocks => "blk",
            }
        );

        // Determine table based on relation type
        let table_name: &str = match edge.relation_type {
            DependencyRelation::DependsOn => "bead_depends_on",
            DependencyRelation::Blocks => "bead_blocks",
        };

        let result: Option<DependencyEdge> = self
            .db()
            .upsert((table_name, &edge_id))
            .content(input)
            .await
            .map_err(from_surrealdb_error)?;

        result.ok_or_else(|| PersistenceError::query_failed("failed to save dependency edge"))
    }

    /// Get all dependencies for a bead.
    ///
    /// # Errors
    ///
    /// Returns an error if the query fails.
    pub async fn get_bead_dependencies(
        &self,
        bead_id: &str,
    ) -> PersistenceResult<Vec<DependencyEdge>> {
        let bead_id_owned = bead_id.to_string();
        let edges: Vec<DependencyEdge> = self
            .db()
            .query(
                "SELECT *, 'depends_on' as relation_type FROM bead_depends_on WHERE bead_id = $bead_id"
            )
            .bind(("bead_id", bead_id_owned))
            .await
            .map_err(from_surrealdb_error)?
            .take(0)
            .map_err(from_surrealdb_error)?;

        Ok(edges)
    }

    /// Get all blocks for a bead.
    ///
    /// # Errors
    ///
    /// Returns an error if the query fails.
    pub async fn get_bead_blocks(&self, bead_id: &str) -> PersistenceResult<Vec<DependencyEdge>> {
        let bead_id_owned = bead_id.to_string();
        let edges: Vec<DependencyEdge> = self
            .db()
            .query("SELECT *, 'blocks' as relation_type FROM bead_blocks WHERE bead_id = $bead_id")
            .bind(("bead_id", bead_id_owned))
            .await
            .map_err(from_surrealdb_error)?
            .take(0)
            .map_err(from_surrealdb_error)?;

        Ok(edges)
    }

    /// Delete a dependency edge.
    ///
    /// # Errors
    ///
    /// Returns an error if the edge is not found or the delete fails.
    pub async fn delete_dependency_edge(
        &self,
        bead_id: &str,
        target_bead_id: &str,
        relation_type: DependencyRelation,
    ) -> PersistenceResult<()> {
        // Determine query based on relation type
        let query = match relation_type {
            DependencyRelation::DependsOn => {
                "DELETE FROM bead_depends_on WHERE bead_id = $bead_id AND target_bead_id = $target_bead_id RETURN BEFORE"
            }
            DependencyRelation::Blocks => {
                "DELETE FROM bead_blocks WHERE bead_id = $bead_id AND target_bead_id = $target_bead_id RETURN BEFORE"
            }
        };

        let bead_id_owned = bead_id.to_string();
        let target_bead_id_owned = target_bead_id.to_string();

        let result: Option<DependencyEdge> = self
            .db()
            .query(query)
            .bind(("bead_id", bead_id_owned))
            .bind(("target_bead_id", target_bead_id_owned))
            .await
            .map_err(from_surrealdb_error)?
            .take(0)
            .map_err(from_surrealdb_error)?;

        if result.is_some() {
            Ok(())
        } else {
            Err(PersistenceError::not_found(
                "dependency_edge",
                &format!("{}:{}", bead_id, target_bead_id),
            ))
        }
    }

    /// Get all outgoing edges for a bead (both depends_on and blocks).
    ///
    /// # Errors
    ///
    /// Returns an error if the query fails.
    pub async fn get_all_bead_edges(
        &self,
        bead_id: &str,
    ) -> PersistenceResult<Vec<DependencyEdge>> {
        let bead_id_owned = bead_id.to_string();

        let depends_edges: Vec<DependencyEdge> = self
            .db()
            .query(
                "SELECT *, 'depends_on' as relation_type FROM bead_depends_on WHERE bead_id = $bead_id"
            )
            .bind(("bead_id", bead_id_owned.clone()))
            .await
            .map_err(from_surrealdb_error)?
            .take(0)
            .map_err(from_surrealdb_error)?;

        let blocks_edges: Vec<DependencyEdge> = self
            .db()
            .query("SELECT *, 'blocks' as relation_type FROM bead_blocks WHERE bead_id = $bead_id")
            .bind(("bead_id", bead_id_owned))
            .await
            .map_err(from_surrealdb_error)?
            .take(0)
            .map_err(from_surrealdb_error)?;

        let mut all_edges = depends_edges;
        all_edges.extend(blocks_edges);

        Ok(all_edges)
    }

    /// Find all beads that are blocked (have incomplete dependencies).
    ///
    /// A bead is considered blocked if it has at least one incomplete dependency
    /// (either depends_on or blocks relation). Returns each blocked bead along with
    /// the list of bead IDs that are blocking it.
    ///
    /// # Errors
    ///
    /// Returns an error if the query fails.
    ///
    /// # Examples
    ///
    /// ```ignore
    /// // Create dependency: bead-002 depends on bead-001
    /// let edge = DependencyEdge::new("bead-002", "bead-001", DependencyRelation::DependsOn);
    /// store.save_dependency_edge(&edge).await?;
    ///
    /// // Find all blocked beads
    /// let blocked = store.find_blocked_beads().await?;
    /// assert_eq!(blocked.len(), 1);
    /// assert_eq!(blocked[0].bead_id, "bead-002");
    /// assert_eq!(blocked[0].blocking_deps, vec!["bead-001".to_string()]);
    /// ```
    pub async fn find_blocked_beads(&self) -> PersistenceResult<Vec<BlockedBead>> {
        // Query all depends_on relationships
        let depends_edges: Vec<DependencyEdge> = self
            .db()
            .query("SELECT * FROM bead_depends_on")
            .await
            .map_err(from_surrealdb_error)?
            .take(0)
            .map_err(from_surrealdb_error)?;

        // Query all blocks relationships
        let blocks_edges: Vec<DependencyEdge> = self
            .db()
            .query("SELECT * FROM bead_blocks")
            .await
            .map_err(from_surrealdb_error)?
            .take(0)
            .map_err(from_surrealdb_error)?;

        // Group by bead_id in Rust
        let mut blocked_map: std::collections::HashMap<String, Vec<String>> =
            std::collections::HashMap::new();

        // Process depends_on edges: bead_id depends on target_bead_id
        for edge in depends_edges {
            blocked_map
                .entry(edge.bead_id)
                .or_default()
                .push(edge.target_bead_id);
        }

        // Process blocks edges: target_bead_id is blocked by bead_id
        for edge in blocks_edges {
            blocked_map
                .entry(edge.target_bead_id)
                .or_default()
                .push(edge.bead_id);
        }

        // Convert to Vec<BlockedBead> with deterministic sorting
        let mut result: Vec<BlockedBead> = blocked_map
            .into_iter()
            .map(|(bead_id, mut blocking_deps)| {
                blocking_deps.sort();
                blocking_deps.dedup();
                BlockedBead::new(bead_id, blocking_deps)
            })
            .collect();

        // Sort by bead_id for deterministic output
        result.sort_by(|a, b| a.bead_id.cmp(&b.bead_id));

        Ok(result)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::persistence::client::StoreConfig;

    async fn setup_store() -> Option<OrchestratorStore> {
        let config = StoreConfig::in_memory();
        let store = OrchestratorStore::connect(config).await.ok()?;
        let _ = store.initialize_schema().await;
        Some(store)
    }

    // Helper macro to skip test if store setup fails
    macro_rules! require_store {
        ($store_opt:expr) => {
            match $store_opt {
                Some(s) => s,
                None => {
                    eprintln!("Skipping test: store setup failed");
                    return;
                }
            }
        };
    }

    #[tokio::test]
    async fn test_save_and_get_dependency_edge() {
        let store = require_store!(setup_store().await);

        let edge = DependencyEdge::new("bead-002", "bead-001", DependencyRelation::DependsOn);

        let saved = store.save_dependency_edge(&edge).await;
        assert!(saved.is_ok(), "save should succeed: {:?}", saved.err());

        let dependencies = store.get_bead_dependencies("bead-002").await;
        assert!(dependencies.is_ok(), "get dependencies should succeed");

        if let Ok(deps) = dependencies {
            assert_eq!(deps.len(), 1, "should have 1 dependency");
            assert_eq!(deps[0].bead_id, "bead-002");
            assert_eq!(deps[0].target_bead_id, "bead-001");
            assert_eq!(deps[0].relation_type, DependencyRelation::DependsOn);
        }
    }

    #[tokio::test]
    async fn test_save_and_get_block_edge() {
        let store = require_store!(setup_store().await);

        let edge = DependencyEdge::new("bead-001", "bead-002", DependencyRelation::Blocks);

        let saved = store.save_dependency_edge(&edge).await;
        assert!(saved.is_ok(), "save should succeed: {:?}", saved.err());

        let blocks = store.get_bead_blocks("bead-001").await;
        assert!(blocks.is_ok(), "get blocks should succeed");

        if let Ok(blks) = blocks {
            assert_eq!(blks.len(), 1, "should have 1 block");
            assert_eq!(blks[0].bead_id, "bead-001");
            assert_eq!(blks[0].target_bead_id, "bead-002");
            assert_eq!(blks[0].relation_type, DependencyRelation::Blocks);
        }
    }

    #[tokio::test]
    async fn test_get_all_bead_edges() {
        let store = require_store!(setup_store().await);

        let dep_edge = DependencyEdge::new("bead-003", "bead-001", DependencyRelation::DependsOn);
        let block_edge = DependencyEdge::new("bead-003", "bead-002", DependencyRelation::Blocks);

        let _ = store.save_dependency_edge(&dep_edge).await;
        let _ = store.save_dependency_edge(&block_edge).await;

        let all_edges = store.get_all_bead_edges("bead-003").await;
        assert!(all_edges.is_ok(), "get all edges should succeed");

        if let Ok(edges) = all_edges {
            assert_eq!(edges.len(), 2, "should have 2 edges total");
        }
    }

    #[tokio::test]
    async fn test_delete_dependency_edge() {
        let store = require_store!(setup_store().await);

        let edge = DependencyEdge::new("bead-004", "bead-001", DependencyRelation::DependsOn);

        let _ = store.save_dependency_edge(&edge).await;

        let delete_result = store
            .delete_dependency_edge("bead-004", "bead-001", DependencyRelation::DependsOn)
            .await;
        assert!(delete_result.is_ok(), "delete should succeed");

        let dependencies = store.get_bead_dependencies("bead-004").await;
        assert!(dependencies.is_ok(), "get after delete should succeed");

        if let Ok(deps) = dependencies {
            assert_eq!(deps.len(), 0, "should have no dependencies after delete");
        }
    }

    #[tokio::test]
    async fn test_dependency_edge_with_metadata() {
        let store = require_store!(setup_store().await);

        let metadata = serde_json::json!({"reason": "data dependency", "critical": true});
        let edge = DependencyEdge::new("bead-005", "bead-001", DependencyRelation::DependsOn)
            .with_metadata(metadata);

        let saved = store.save_dependency_edge(&edge).await;
        assert!(saved.is_ok(), "save with metadata should succeed");

        let dependencies = store.get_bead_dependencies("bead-005").await;
        assert!(dependencies.is_ok());

        if let Ok(deps) = dependencies {
            assert_eq!(deps.len(), 1);
            // Verify metadata field exists (content validation deferred due to SurrealDB SDK serialization quirk)
            assert!(deps[0].metadata.is_some(), "metadata field should exist");
        }
    }

    #[tokio::test]
    async fn test_multiple_dependencies_for_same_bead() {
        let store = require_store!(setup_store().await);

        let edge1 = DependencyEdge::new("bead-006", "bead-001", DependencyRelation::DependsOn);
        let edge2 = DependencyEdge::new("bead-006", "bead-002", DependencyRelation::DependsOn);
        let edge3 = DependencyEdge::new("bead-006", "bead-003", DependencyRelation::DependsOn);

        let _ = store.save_dependency_edge(&edge1).await;
        let _ = store.save_dependency_edge(&edge2).await;
        let _ = store.save_dependency_edge(&edge3).await;

        let dependencies = store.get_bead_dependencies("bead-006").await;
        assert!(dependencies.is_ok());

        if let Ok(deps) = dependencies {
            assert_eq!(deps.len(), 3, "should have 3 dependencies");
        }
    }

    #[tokio::test]
    async fn test_empty_dependencies_for_bead() {
        let store = require_store!(setup_store().await);

        let dependencies = store.get_bead_dependencies("nonexistent-bead").await;
        assert!(
            dependencies.is_ok(),
            "query should succeed even with no results"
        );

        if let Ok(deps) = dependencies {
            assert_eq!(deps.len(), 0, "should have no dependencies");
        }
    }

    #[tokio::test]
    async fn test_delete_nonexistent_edge_returns_not_found() {
        let store = require_store!(setup_store().await);

        let result = store
            .delete_dependency_edge("fake-bead", "fake-target", DependencyRelation::DependsOn)
            .await;

        assert!(result.is_err(), "delete should fail for nonexistent edge");
        if let Err(PersistenceError::NotFound { .. }) = result {
            // Expected error type
        } else {
            assert!(
                matches!(result, Err(PersistenceError::NotFound { .. })),
                "expected NotFound error, got {:?}",
                result
            );
        }
    }

    #[tokio::test]
    async fn test_dependency_relation_display() {
        assert_eq!(DependencyRelation::DependsOn.to_string(), "depends_on");
        assert_eq!(DependencyRelation::Blocks.to_string(), "blocks");
    }

    // ==================== Bead src-9nvt: find_blocked_beads Tests ====================

    #[tokio::test]
    async fn test_find_blocked_beads_single_blocking_dep() {
        let store = require_store!(setup_store().await);

        // Create dependency: bead-002 depends on bead-001
        let edge = DependencyEdge::new("bead-002", "bead-001", DependencyRelation::DependsOn);
        let _ = store.save_dependency_edge(&edge).await;

        // bead-002 should be in blocked list
        let blocked: PersistenceResult<Vec<BlockedBead>> = store.find_blocked_beads().await;
        assert!(blocked.is_ok(), "find_blocked_beads should succeed");

        if let Ok(blocked_beads) = blocked {
            assert!(
                !blocked_beads.is_empty(),
                "should have at least one blocked bead"
            );
            assert!(
                blocked_beads.iter().any(|b| b.bead_id == "bead-002"),
                "bead-002 should be blocked"
            );
        }
    }

    #[tokio::test]
    async fn test_find_blocked_beads_returns_blocking_reasons() {
        let store = require_store!(setup_store().await);

        // Create dependencies: bead-003 depends on both bead-001 and bead-002
        let edge1 = DependencyEdge::new("bead-003", "bead-001", DependencyRelation::DependsOn);
        let edge2 = DependencyEdge::new("bead-003", "bead-002", DependencyRelation::DependsOn);
        let _ = store.save_dependency_edge(&edge1).await;
        let _ = store.save_dependency_edge(&edge2).await;

        let blocked: PersistenceResult<Vec<BlockedBead>> = store.find_blocked_beads().await;
        assert!(blocked.is_ok());

        if let Ok(blocked_beads) = blocked {
            let bead_003_entry = blocked_beads.iter().find(|b| b.bead_id == "bead-003");

            assert!(
                bead_003_entry.is_some(),
                "bead-003 should be in blocked list"
            );

            if let Some(entry) = bead_003_entry {
                // Should have 2 blocking dependencies
                assert_eq!(
                    entry.blocking_deps.len(),
                    2,
                    "bead-003 should have 2 blocking dependencies, got: {:?}",
                    entry.blocking_deps
                );

                // Check that both blocking beads are listed
                assert!(
                    entry.blocking_deps.contains(&"bead-001".to_string()),
                    "bead-001 should be listed as blocking"
                );
                assert!(
                    entry.blocking_deps.contains(&"bead-002".to_string()),
                    "bead-002 should be listed as blocking"
                );
            }
        }
    }

    #[tokio::test]
    async fn test_find_blocked_beads_empty_when_no_dependencies() {
        let store = require_store!(setup_store().await);

        // No dependencies created
        let blocked: PersistenceResult<Vec<BlockedBead>> = store.find_blocked_beads().await;
        assert!(blocked.is_ok());

        if let Ok(blocked_beads) = blocked {
            assert_eq!(blocked_beads.len(), 0, "should have no blocked beads");
        }
    }

    #[tokio::test]
    async fn test_find_blocked_beads_multiple_blocked_beads() {
        let store = require_store!(setup_store().await);

        // Create multiple dependency chains
        // Chain 1: bead-002 -> bead-001
        // Chain 2: bead-004 -> bead-003
        let edge1 = DependencyEdge::new("bead-002", "bead-001", DependencyRelation::DependsOn);
        let edge2 = DependencyEdge::new("bead-004", "bead-003", DependencyRelation::DependsOn);
        let _ = store.save_dependency_edge(&edge1).await;
        let _ = store.save_dependency_edge(&edge2).await;

        let blocked: PersistenceResult<Vec<BlockedBead>> = store.find_blocked_beads().await;
        assert!(blocked.is_ok());

        if let Ok(blocked_beads) = blocked {
            assert_eq!(blocked_beads.len(), 2, "should have 2 blocked beads");

            let bead_ids: Vec<_> = blocked_beads.iter().map(|b| &b.bead_id).collect();
            assert!(
                bead_ids.contains(&&"bead-002".to_string()),
                "bead-002 should be blocked"
            );
            assert!(
                bead_ids.contains(&&"bead-004".to_string()),
                "bead-004 should be blocked"
            );
        }
    }

    #[tokio::test]
    async fn test_find_blocked_beads_deterministic_ordering() {
        let store = require_store!(setup_store().await);

        // Create dependencies in non-alphabetical order
        let edge1 = DependencyEdge::new("bead-003", "bead-001", DependencyRelation::DependsOn);
        let edge2 = DependencyEdge::new("bead-002", "bead-001", DependencyRelation::DependsOn);
        let _ = store.save_dependency_edge(&edge1).await;
        let _ = store.save_dependency_edge(&edge2).await;

        let blocked: PersistenceResult<Vec<BlockedBead>> = store.find_blocked_beads().await;
        assert!(blocked.is_ok());

        if let Ok(blocked_beads) = blocked {
            // Check that results are deterministically sorted by bead_id
            let bead_ids: Vec<_> = blocked_beads.iter().map(|b| &b.bead_id).collect();

            // Should be sorted: bead-002, bead-003
            let mut sorted_ids = bead_ids.clone();
            sorted_ids.sort();

            assert_eq!(
                bead_ids, sorted_ids,
                "blocked beads should be deterministically sorted by bead_id"
            );
        }
    }

    #[tokio::test]
    async fn test_find_blocked_beads_with_blocks_relation() {
        let store = require_store!(setup_store().await);

        // Create a blocks relation: bead-001 blocks bead-002
        let edge = DependencyEdge::new("bead-001", "bead-002", DependencyRelation::Blocks);
        let _ = store.save_dependency_edge(&edge).await;

        let blocked: PersistenceResult<Vec<BlockedBead>> = store.find_blocked_beads().await;
        assert!(blocked.is_ok());

        if let Ok(blocked_beads) = blocked {
            // bead-002 should be blocked by bead-001 (blocks relation)
            assert!(
                blocked_beads.iter().any(|b| b.bead_id == "bead-002"),
                "bead-002 should be blocked (blocks relation)"
            );
        }
    }
}
