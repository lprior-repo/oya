//! Sticky assignment storage schema for SurrealDB.
//!
//! This module provides type-safe Rust mappings for the `worker_assignment` table,
//! which enables sticky distribution mode (prefer assigning beads to the same worker).
//!
//! # Table Schema
//!
//! The `worker_assignment` table in SurrealDB stores sticky assignments:
//! - `assignment_id`: Unique identifier (ULID)
//! - `bead_id`: The bead being assigned (unique constraint)
//! - `worker_id`: The worker assigned to this bead
//! - `assigned_at`: Timestamp of initial assignment
//! - `updated_at`: Timestamp of last update

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::fmt;
use thiserror::Error;

// ============================================================================
// Error Types
// ============================================================================

/// Errors that can occur during sticky assignment operations.
#[derive(Debug, Error, Clone, PartialEq, Eq)]
pub enum StickyAssignmentError {
    /// Invalid assignment ID (empty or invalid format).
    #[error("invalid assignment ID: {0}")]
    InvalidAssignmentId(String),

    /// Invalid bead ID (empty).
    #[error("invalid bead ID: {0}")]
    InvalidBeadId(String),

    /// Invalid worker ID (empty).
    #[error("invalid worker ID: {0}")]
    InvalidWorkerId(String),

    /// Assignment already exists for this bead.
    #[error("assignment already exists for bead: {0}")]
    AssignmentAlreadyExists(String),

    /// Assignment not found.
    #[error("assignment not found for bead: {0}")]
    AssignmentNotFound(String),

    /// Database operation failed.
    #[error("database error: {0}")]
    DatabaseError(String),
}

pub type Result<T> = std::result::Result<T, StickyAssignmentError>;

// ============================================================================
// Assignment ID (Newtype for type safety)
// ============================================================================

/// Assignment ID newtype for type safety and validation.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct AssignmentId(String);

impl AssignmentId {
    /// Create a new assignment ID.
    ///
    /// # Errors
    ///
    /// Returns an error if the ID is empty.
    pub fn new(id: String) -> Result<Self> {
        if id.trim().is_empty() {
            return Err(StickyAssignmentError::InvalidAssignmentId(id));
        }
        Ok(Self(id))
    }

    /// Generate a new unique assignment ID using ULID.
    #[must_use]
    pub fn generate() -> Self {
        Self(ulid::Ulid::new().to_string())
    }

    /// Get the inner ID string.
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }

    /// Consume and return the inner string.
    #[must_use]
    pub fn into_inner(self) -> String {
        self.0
    }
}

impl fmt::Display for AssignmentId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

// ============================================================================
// Bead ID (Newtype for type safety)
// ============================================================================

/// Bead ID newtype for sticky assignment context.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct BeadIdRef(String);

impl BeadIdRef {
    /// Create a new bead ID reference.
    ///
    /// # Errors
    ///
    /// Returns an error if the ID is empty.
    pub fn new(id: String) -> Result<Self> {
        if id.trim().is_empty() {
            return Err(StickyAssignmentError::InvalidBeadId(id));
        }
        Ok(Self(id))
    }

    /// Get the inner ID string.
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for BeadIdRef {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

// ============================================================================
// Worker ID (Newtype for type safety)
// ============================================================================

/// Worker ID newtype for sticky assignment context.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct WorkerIdRef(String);

impl WorkerIdRef {
    /// Create a new worker ID reference.
    ///
    /// # Errors
    ///
    /// Returns an error if the ID is empty.
    pub fn new(id: String) -> Result<Self> {
        if id.trim().is_empty() {
            return Err(StickyAssignmentError::InvalidWorkerId(id));
        }
        Ok(Self(id))
    }

    /// Get the inner ID string.
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for WorkerIdRef {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

// ============================================================================
// Sticky Assignment
// ============================================================================

/// Worker assignment table row for sticky distribution.
///
/// Maps to the `worker_assignment` table in SurrealDB.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct StickyAssignment {
    /// Unique assignment identifier.
    pub assignment_id: String,
    /// The bead being assigned (unique constraint in DB).
    pub bead_id: String,
    /// The worker assigned to this bead.
    pub worker_id: String,
    /// Timestamp of initial assignment.
    pub assigned_at: DateTime<Utc>,
    /// Timestamp of last update.
    pub updated_at: DateTime<Utc>,
}

impl StickyAssignment {
    /// Create a new sticky assignment.
    ///
    /// # Arguments
    ///
    /// * `bead_id` - The bead to assign
    /// * `worker_id` - The worker to assign the bead to
    ///
    /// # Errors
    ///
    /// Returns an error if bead_id or worker_id is empty.
    pub fn create(bead_id: &str, worker_id: &str) -> Result<Self> {
        let bead = BeadIdRef::new(bead_id.to_string())?;
        let worker = WorkerIdRef::new(worker_id.to_string())?;
        let now = Utc::now();

        Ok(Self {
            assignment_id: AssignmentId::generate().into_inner(),
            bead_id: bead.into_inner(),
            worker_id: worker.into_inner(),
            assigned_at: now,
            updated_at: now,
        })
    }

    /// Create an assignment with a specific ID (for reconstitution from DB).
    ///
    /// # Arguments
    ///
    /// * `assignment_id` - The existing assignment ID
    /// * `bead_id` - The bead being assigned
    /// * `worker_id` - The worker assigned
    /// * `assigned_at` - Original assignment timestamp
    ///
    /// # Errors
    ///
    /// Returns an error if any required field is invalid.
    pub fn reconstitute(
        assignment_id: &str,
        bead_id: &str,
        worker_id: &str,
        assigned_at: DateTime<Utc>,
    ) -> Result<Self> {
        let id = AssignmentId::new(assignment_id.to_string())?;
        let bead = BeadIdRef::new(bead_id.to_string())?;
        let worker = WorkerIdRef::new(worker_id.to_string())?;

        Ok(Self {
            assignment_id: id.into_inner(),
            bead_id: bead.into_inner(),
            worker_id: worker.into_inner(),
            assigned_at,
            updated_at: Utc::now(),
        })
    }

    /// Update the worker for this assignment.
    ///
    /// # Arguments
    ///
    /// * `new_worker_id` - The new worker to assign
    ///
    /// # Errors
    ///
    /// Returns an error if the new worker ID is empty.
    pub fn reassign(&mut self, new_worker_id: &str) -> Result<()> {
        let worker = WorkerIdRef::new(new_worker_id.to_string())?;
        self.worker_id = worker.into_inner();
        self.updated_at = Utc::now();
        Ok(())
    }

    /// Check if this assignment is for a specific worker.
    #[must_use]
    pub fn is_assigned_to(&self, worker_id: &str) -> bool {
        self.worker_id == worker_id
    }

    /// Get the age of this assignment in seconds.
    #[must_use]
    pub fn age_seconds(&self) -> i64 {
        (Utc::now() - self.assigned_at).num_seconds()
    }
}

impl BeadIdRef {
    fn into_inner(self) -> String {
        self.0
    }
}

impl WorkerIdRef {
    fn into_inner(self) -> String {
        self.0
    }
}

// ============================================================================
// Query Builders (Pure Functions)
// ============================================================================

/// Build a SurrealDB query to create a sticky assignment.
#[must_use]
pub fn build_create_assignment_query(bead_id: &str, worker_id: &str) -> String {
    format!(
        "CREATE worker_assignment CONTENT {{
            assignment_id: '{}',
            bead_id: '{}',
            worker_id: '{}',
            assigned_at: time::now(),
            updated_at: time::now()
        }}",
        ulid::Ulid::new(),
        bead_id,
        worker_id
    )
}

/// Build a SurrealDB query to find assignment by bead ID.
#[must_use]
pub fn build_find_by_bead_query(bead_id: &str) -> String {
    format!(
        "SELECT * FROM worker_assignment WHERE bead_id = '{}' LIMIT 1",
        bead_id
    )
}

/// Build a SurrealDB query to find all assignments for a worker.
#[must_use]
pub fn build_find_by_worker_query(worker_id: &str) -> String {
    format!(
        "SELECT * FROM worker_assignment WHERE worker_id = '{}'",
        worker_id
    )
}

/// Build a SurrealDB query to update assignment worker.
#[must_use]
pub fn build_update_worker_query(bead_id: &str, new_worker_id: &str) -> String {
    format!(
        "UPDATE worker_assignment SET worker_id = '{}', updated_at = time::now() WHERE bead_id = '{}'",
        new_worker_id, bead_id
    )
}

/// Build a SurrealDB query to delete assignment by bead ID.
#[must_use]
pub fn build_delete_by_bead_query(bead_id: &str) -> String {
    format!("DELETE worker_assignment WHERE bead_id = '{}'", bead_id)
}

/// Build a SurrealDB query to get all assignments.
#[must_use]
pub const fn build_find_all_query() -> &'static str {
    "SELECT * FROM worker_assignment"
}

/// Build a SurrealDB query to count assignments for a worker.
#[must_use]
pub fn build_count_by_worker_query(worker_id: &str) -> String {
    format!(
        "SELECT count() FROM worker_assignment WHERE worker_id = '{}' GROUP ALL",
        worker_id
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    // ========================================================================
    // AssignmentId Tests
    // ========================================================================

    #[test]
    fn test_assignment_id_new_valid() {
        let id = AssignmentId::new("assign-123".to_string());
        assert!(id.is_ok());
        assert_eq!(id.as_ref().map(AssignmentId::as_str), Ok("assign-123"));
    }

    #[test]
    fn test_assignment_id_new_empty() {
        let id = AssignmentId::new("".to_string());
        assert!(id.is_err());
    }

    #[test]
    fn test_assignment_id_new_whitespace() {
        let id = AssignmentId::new("   ".to_string());
        assert!(id.is_err());
    }

    #[test]
    fn test_assignment_id_generate() {
        let id1 = AssignmentId::generate();
        let id2 = AssignmentId::generate();
        assert_ne!(id1.as_str(), id2.as_str(), "ULIDs should be unique");
    }

    #[test]
    fn test_assignment_id_display() -> Result<()> {
        let id = AssignmentId::new("test-id".to_string())?;
        assert_eq!(format!("{}", id), "test-id");
        Ok(())
    }

    // ========================================================================
    // BeadIdRef Tests
    // ========================================================================

    #[test]
    fn test_bead_id_ref_new_valid() {
        let id = BeadIdRef::new("bead-123".to_string());
        assert!(id.is_ok());
        assert_eq!(id.as_ref().map(BeadIdRef::as_str), Ok("bead-123"));
    }

    #[test]
    fn test_bead_id_ref_new_empty() {
        let id = BeadIdRef::new("".to_string());
        assert!(id.is_err());
    }

    // ========================================================================
    // WorkerIdRef Tests
    // ========================================================================

    #[test]
    fn test_worker_id_ref_new_valid() {
        let id = WorkerIdRef::new("worker-123".to_string());
        assert!(id.is_ok());
        assert_eq!(id.as_ref().map(WorkerIdRef::as_str), Ok("worker-123"));
    }

    #[test]
    fn test_worker_id_ref_new_empty() {
        let id = WorkerIdRef::new("".to_string());
        assert!(id.is_err());
    }

    // ========================================================================
    // StickyAssignment Tests
    // ========================================================================

    #[test]
    fn test_sticky_assignment_create() -> Result<()> {
        let assignment = StickyAssignment::create("bead-1", "worker-a")?;
        assert_eq!(assignment.bead_id, "bead-1");
        assert_eq!(assignment.worker_id, "worker-a");
        assert!(!assignment.assignment_id.is_empty());
        Ok(())
    }

    #[test]
    fn test_sticky_assignment_create_empty_bead() {
        let assignment = StickyAssignment::create("", "worker-a");
        assert!(assignment.is_err());
    }

    #[test]
    fn test_sticky_assignment_create_empty_worker() {
        let assignment = StickyAssignment::create("bead-1", "");
        assert!(assignment.is_err());
    }

    #[test]
    fn test_sticky_assignment_reconstitute() -> Result<()> {
        let assigned_at = Utc::now() - chrono::Duration::hours(1);
        let assignment = StickyAssignment::reconstitute(
            "assign-existing",
            "bead-old",
            "worker-old",
            assigned_at,
        )?;

        assert_eq!(assignment.assignment_id, "assign-existing");
        assert_eq!(assignment.bead_id, "bead-old");
        assert_eq!(assignment.worker_id, "worker-old");
        assert_eq!(assignment.assigned_at, assigned_at);
        Ok(())
    }

    #[test]
    fn test_sticky_assignment_reassign() -> Result<()> {
        let mut assignment =
            StickyAssignment::create("bead-1", "worker-a")?;

        let old_updated = assignment.updated_at;
        std::thread::sleep(std::time::Duration::from_millis(10));

        let result = assignment.reassign("worker-b");
        assert!(result.is_ok());
        assert_eq!(assignment.worker_id, "worker-b");
        assert!(assignment.updated_at > old_updated);
        Ok(())
    }

    #[test]
    fn test_sticky_assignment_reassign_empty() -> Result<()> {
        let mut assignment =
            StickyAssignment::create("bead-1", "worker-a")?;

        let result = assignment.reassign("");
        assert!(result.is_err());
        Ok(())
    }

    #[test]
    fn test_sticky_assignment_is_assigned_to() -> Result<()> {
        let assignment = StickyAssignment::create("bead-1", "worker-a")?;

        assert!(assignment.is_assigned_to("worker-a"));
        assert!(!assignment.is_assigned_to("worker-b"));
        Ok(())
    }

    #[test]
    fn test_sticky_assignment_age_seconds() -> Result<()> {
        let assignment = StickyAssignment::create("bead-1", "worker-a")?;

        let age = assignment.age_seconds();
        assert!(age >= 0);
        assert!(
            age < 5,
            "Age should be less than 5 seconds for new assignment"
        );
        Ok(())
    }

    // ========================================================================
    // Query Builder Tests
    // ========================================================================

    #[test]
    fn test_build_create_assignment_query() {
        let query = build_create_assignment_query("bead-123", "worker-xyz");
        assert!(query.contains("CREATE worker_assignment"));
        assert!(query.contains("bead-123"));
        assert!(query.contains("worker-xyz"));
    }

    #[test]
    fn test_build_find_by_bead_query() {
        let query = build_find_by_bead_query("bead-456");
        assert!(query.contains("SELECT * FROM worker_assignment"));
        assert!(query.contains("bead_id = 'bead-456'"));
        assert!(query.contains("LIMIT 1"));
    }

    #[test]
    fn test_build_find_by_worker_query() {
        let query = build_find_by_worker_query("worker-abc");
        assert!(query.contains("SELECT * FROM worker_assignment"));
        assert!(query.contains("worker_id = 'worker-abc'"));
    }

    #[test]
    fn test_build_update_worker_query() {
        let query = build_update_worker_query("bead-789", "worker-new");
        assert!(query.contains("UPDATE worker_assignment"));
        assert!(query.contains("worker_id = 'worker-new'"));
        assert!(query.contains("bead_id = 'bead-789'"));
    }

    #[test]
    fn test_build_delete_by_bead_query() {
        let query = build_delete_by_bead_query("bead-delete");
        assert!(query.contains("DELETE worker_assignment"));
        assert!(query.contains("bead_id = 'bead-delete'"));
    }

    #[test]
    fn test_build_find_all_query() {
        let query = build_find_all_query();
        assert_eq!(query, "SELECT * FROM worker_assignment");
    }

    #[test]
    fn test_build_count_by_worker_query() {
        let query = build_count_by_worker_query("worker-count");
        assert!(query.contains("SELECT count()"));
        assert!(query.contains("worker_id = 'worker-count'"));
        assert!(query.contains("GROUP ALL"));
    }

    // ========================================================================
    // Error Type Tests
    // ========================================================================

    #[test]
    fn test_sticky_assignment_error_display() {
        let err = StickyAssignmentError::InvalidBeadId("".to_string());
        assert!(err.to_string().contains("invalid bead ID"));

        let err = StickyAssignmentError::AssignmentAlreadyExists("bead-x".to_string());
        assert!(err.to_string().contains("already exists"));
    }

    #[test]
    fn test_sticky_assignment_error_equality() {
        let err1 = StickyAssignmentError::InvalidBeadId("abc".to_string());
        let err2 = StickyAssignmentError::InvalidBeadId("abc".to_string());
        let err3 = StickyAssignmentError::InvalidBeadId("def".to_string());

        assert_eq!(err1, err2);
        assert_ne!(err1, err3);
    }
}
