#![deny(clippy::unwrap_used)]
#![deny(clippy::expect_used)]
#![deny(clippy::panic)]
#![warn(clippy::pedantic)]
#![warn(clippy::nursery)]
#![forbid(unsafe_code)]

//! Core data types for BeadStore.
//!
//! Defines bead records, statuses, and identifiers with serialization support.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::fmt;

/// Unique identifier for a bead.
///
/// Wraps a String to provide type safety and prevent ID confusion.
#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct BeadId(String);

impl BeadId {
    /// Create a new BeadId from a string.
    #[must_use]
    pub fn new(id: impl Into<String>) -> Self {
        Self(id.into())
    }

    /// Get the underlying string value.
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }

    /// Convert into the underlying string.
    #[must_use]
    pub fn into_inner(self) -> String {
        self.0
    }
}

impl fmt::Display for BeadId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl From<String> for BeadId {
    fn from(s: String) -> Self {
        Self(s)
    }
}

impl From<&str> for BeadId {
    fn from(s: &str) -> Self {
        Self(s.to_string())
    }
}

/// Status of a bead in the workflow.
#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum BeadStatus {
    /// Bead is open and not yet started.
    Open,
    /// Bead is currently being worked on.
    InProgress,
    /// Bead has been completed/closed.
    Closed,
}

impl fmt::Display for BeadStatus {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Open => write!(f, "open"),
            Self::InProgress => write!(f, "in_progress"),
            Self::Closed => write!(f, "closed"),
        }
    }
}

/// A record representing a bead with all its metadata.
///
/// BeadRecords are immutable by convention - use `BeadStore::update_bead`
/// to create a new version.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct BeadRecord {
    /// Unique identifier.
    pub id: BeadId,
    /// Human-readable title.
    pub title: String,
    /// Detailed description.
    pub description: String,
    /// Current status.
    pub status: BeadStatus,
    /// Labels for categorization.
    pub labels: Vec<String>,
    /// Priority (0 = highest).
    pub priority: u8,
    /// Creation timestamp.
    pub created_at: DateTime<Utc>,
    /// Last update timestamp.
    pub updated_at: DateTime<Utc>,
}

impl BeadRecord {
    /// Create a new BeadRecord.
    #[must_use]
    pub fn new(
        id: impl Into<BeadId>,
        title: impl Into<String>,
        description: impl Into<String>,
        status: BeadStatus,
        priority: u8,
    ) -> Self {
        let now = Utc::now();
        Self {
            id: id.into(),
            title: title.into(),
            description: description.into(),
            status,
            labels: Vec::new(),
            priority,
            created_at: now,
            updated_at: now,
        }
    }

    /// Create a new BeadRecord with labels.
    #[must_use]
    pub fn with_labels(
        id: impl Into<BeadId>,
        title: impl Into<String>,
        description: impl Into<String>,
        status: BeadStatus,
        priority: u8,
        labels: Vec<String>,
    ) -> Self {
        let now = Utc::now();
        Self {
            id: id.into(),
            title: title.into(),
            description: description.into(),
            status,
            labels,
            priority,
            created_at: now,
            updated_at: now,
        }
    }

    /// Update the status and refresh the updated_at timestamp.
    #[must_use]
    pub fn with_status(mut self, status: BeadStatus) -> Self {
        self.status = status;
        self.updated_at = Utc::now();
        self
    }

    /// Add a label.
    #[must_use]
    pub fn with_label(mut self, label: impl Into<String>) -> Self {
        let label = label.into();
        if !self.labels.contains(&label) {
            self.labels.push(label);
        }
        self.updated_at = Utc::now();
        self
    }

    /// Check if bead has a specific label.
    #[must_use]
    pub fn has_label(&self, label: &str) -> bool {
        self.labels.iter().any(|l| l == label)
    }

    /// Create a test fixture for unit tests.
    #[cfg(test)]
    #[must_use]
    pub fn test_fixture() -> Self {
        Self::with_labels(
            "test-bead-123",
            "Test Bead",
            "A test bead for unit testing",
            BeadStatus::Open,
            1,
            vec!["test".to_string(), "fixture".to_string()],
        )
    }
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used)]
    #![allow(clippy::expect_used)]
    #![allow(clippy::panic)]

    use super::*;

    #[test]
    fn test_bead_id_from_string() {
        let id = BeadId::new("test-123");
        assert_eq!(id.as_str(), "test-123");
        assert_eq!(id.into_inner(), "test-123");
    }

    #[test]
    fn test_bead_id_display() {
        let id = BeadId::new("test-123");
        assert_eq!(format!("{id}"), "test-123");
    }

    #[test]
    fn test_bead_status_display() {
        assert_eq!(format!("{}", BeadStatus::Open), "open");
        assert_eq!(format!("{}", BeadStatus::InProgress), "in_progress");
        assert_eq!(format!("{}", BeadStatus::Closed), "closed");
    }

    #[test]
    fn test_bead_record_new() {
        let bead = BeadRecord::new(
            "bead-1",
            "Title",
            "Description",
            BeadStatus::Open,
            0,
        );
        assert_eq!(bead.id.as_str(), "bead-1");
        assert_eq!(bead.title, "Title");
        assert_eq!(bead.description, "Description");
        assert_eq!(bead.status, BeadStatus::Open);
        assert!(bead.labels.is_empty());
        assert_eq!(bead.priority, 0);
    }

    #[test]
    fn test_bead_record_with_status() {
        let bead = BeadRecord::test_fixture();
        let updated = bead.with_status(BeadStatus::InProgress);
        assert_eq!(updated.status, BeadStatus::InProgress);
        assert!(updated.updated_at > bead.created_at);
    }

    #[test]
    fn test_bead_record_with_label() {
        let bead = BeadRecord::test_fixture();
        let with_label = bead.with_label("new-label");
        assert!(with_label.has_label("new-label"));
        assert!(with_label.updated_at > bead.created_at);
    }

    #[test]
    fn test_bead_record_duplicate_label() {
        let bead = BeadRecord::test_fixture();
        let count_before = bead.labels.len();
        let with_dup = bead.with_label("test"); // Already has this label
        assert_eq!(with_dup.labels.len(), count_before);
    }

    #[test]
    fn test_bead_record_serialization() {
        let bead = BeadRecord::test_fixture();
        let json = serde_json::to_string(&bead).expect("serialization failed");
        let deserialized: BeadRecord = serde_json::from_str(&json).expect("deserialization failed");
        assert_eq!(bead, deserialized);
    }
}
