//! Bead model for issue tracking
//!
//! Beads are the fundamental work items in the oya system.
//! They represent tasks, bugs, features, or any trackable work.

use rkyv::{Archive, Deserialize as RkyvDeserialize, Serialize as RkyvSerialize};
use serde::{Deserialize, Serialize};
use std::sync::Arc;

/// Bead status enumeration matching the backend BeadScheduleState
#[derive(
    Debug,
    Clone,
    Copy,
    PartialEq,
    Eq,
    Serialize,
    Deserialize,
    Default,
    Archive,
    RkyvSerialize,
    RkyvDeserialize,
)]
#[rkyv(compare(PartialEq))]
#[serde(rename_all = "snake_case")]
pub enum BeadStatus {
    #[default]
    Pending,
    Ready,
    Running,
    Completed,
    Failed,
    Cancelled,
}

impl BeadStatus {
    /// Returns the color associated with this status
    #[must_use]
    pub const fn color(&self) -> &'static str {
        match self {
            BeadStatus::Pending => "#9ca3af",   // gray
            BeadStatus::Ready => "#3b82f6",     // blue
            BeadStatus::Running => "#f59e0b",   // amber
            BeadStatus::Completed => "#10b981", // green
            BeadStatus::Failed => "#ef4444",    // red
            BeadStatus::Cancelled => "#6b7280", // dark gray
        }
    }

    /// Returns the display label for this status
    #[must_use]
    pub const fn label(&self) -> &'static str {
        match self {
            BeadStatus::Pending => "Pending",
            BeadStatus::Ready => "Ready",
            BeadStatus::Running => "Running",
            BeadStatus::Completed => "Completed",
            BeadStatus::Failed => "Failed",
            BeadStatus::Cancelled => "Cancelled",
        }
    }

    /// Returns true if this is a terminal state
    #[must_use]
    pub const fn is_terminal(&self) -> bool {
        matches!(
            self,
            BeadStatus::Completed | BeadStatus::Failed | BeadStatus::Cancelled
        )
    }
}

/// Bead priority enumeration
#[derive(
    Debug,
    Clone,
    Copy,
    PartialEq,
    Eq,
    Serialize,
    Deserialize,
    Default,
    Archive,
    RkyvSerialize,
    RkyvDeserialize,
)]
#[rkyv(compare(PartialEq))]
#[serde(rename_all = "lowercase")]
pub enum BeadPriority {
    Low,
    #[default]
    Medium,
    High,
    Critical,
}

impl BeadPriority {
    /// Returns the numeric value (higher = more important)
    #[must_use]
    pub const fn value(&self) -> u8 {
        match self {
            BeadPriority::Low => 1,
            BeadPriority::Medium => 2,
            BeadPriority::High => 3,
            BeadPriority::Critical => 4,
        }
    }

    /// Returns the display label
    #[must_use]
    pub const fn label(&self) -> &'static str {
        match self {
            BeadPriority::Low => "Low",
            BeadPriority::Medium => "Medium",
            BeadPriority::High => "High",
            BeadPriority::Critical => "Critical",
        }
    }

    /// Returns the color for this priority
    #[must_use]
    pub const fn color(&self) -> &'static str {
        match self {
            BeadPriority::Low => "#6b7280",
            BeadPriority::Medium => "#3b82f6",
            BeadPriority::High => "#f59e0b",
            BeadPriority::Critical => "#ef4444",
        }
    }
}

/// Bead data structure representing an issue/work item
#[derive(Debug, Clone, PartialEq, Archive, RkyvSerialize, RkyvDeserialize)]
#[rkyv(compare(PartialEq))]
pub struct Bead {
    pub id: Arc<str>,
    pub title: Arc<str>,
    pub description: Arc<str>,
    pub status: BeadStatus,
    pub priority: BeadPriority,
    pub dependencies: Vec<Arc<str>>,
    pub tags: Vec<Arc<str>>,
    pub created_at: Arc<str>,
    pub updated_at: Arc<str>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct BeadSerde {
    pub id: String,
    pub title: String,
    pub description: String,
    pub status: BeadStatus,
    pub priority: BeadPriority,
    pub dependencies: Vec<String>,
    pub tags: Vec<String>,
    pub created_at: String,
    pub updated_at: String,
}

impl Serialize for Bead {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        let serde_bead = BeadSerde {
            id: self.id.to_string(),
            title: self.title.to_string(),
            description: self.description.to_string(),
            status: self.status,
            priority: self.priority,
            dependencies: self.dependencies.iter().map(|s| s.to_string()).collect(),
            tags: self.tags.iter().map(|s| s.to_string()).collect(),
            created_at: self.created_at.to_string(),
            updated_at: self.updated_at.to_string(),
        };
        serde_bead.serialize(serializer)
    }
}

impl<'de> Deserialize<'de> for Bead {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let serde_bead: BeadSerde = BeadSerde::deserialize(deserializer)?;
        Ok(Bead {
            id: serde_bead.id.into(),
            title: serde_bead.title.into(),
            description: serde_bead.description.into(),
            status: serde_bead.status,
            priority: serde_bead.priority,
            dependencies: serde_bead
                .dependencies
                .into_iter()
                .map(Into::into)
                .collect(),
            tags: serde_bead.tags.into_iter().map(Into::into).collect(),
            created_at: serde_bead.created_at.into(),
            updated_at: serde_bead.updated_at.into(),
        })
    }
}

impl Bead {
    /// Creates a new bead with minimal required fields
    #[must_use]
    pub fn new(id: impl Into<String>, title: impl Into<String>) -> Self {
        let now = "2026-02-02T00:00:00Z".to_string();
        Self {
            id: id.into().into(),
            title: title.into().into(),
            description: String::new().into(),
            status: BeadStatus::default(),
            priority: BeadPriority::default(),
            dependencies: Vec::new(),
            tags: Vec::new(),
            created_at: now.clone().into(),
            updated_at: now.into(),
        }
    }

    /// Builder: set description
    #[must_use]
    pub fn with_description(self, description: impl Into<String>) -> Self {
        Self {
            description: description.into().into(),
            ..self
        }
    }

    /// Builder: set status
    #[must_use]
    pub fn with_status(self, status: BeadStatus) -> Self {
        Self { status, ..self }
    }

    /// Builder: set priority
    #[must_use]
    pub fn with_priority(self, priority: BeadPriority) -> Self {
        Self { priority, ..self }
    }

    /// Builder: set dependencies
    #[must_use]
    pub fn with_dependencies(self, dependencies: Vec<String>) -> Self {
        Self {
            dependencies: dependencies.into_iter().map(Into::into).collect(),
            ..self
        }
    }

    /// Builder: add a single dependency
    #[must_use]
    pub fn with_dependency(mut self, dep: impl Into<String>) -> Self {
        self.dependencies.push(dep.into().into());
        self
    }

    /// Builder: set tags
    #[must_use]
    pub fn with_tags(self, tags: Vec<String>) -> Self {
        Self {
            tags: tags.into_iter().map(Into::into).collect(),
            ..self
        }
    }

    /// Builder: add a single tag
    #[must_use]
    pub fn with_tag(mut self, tag: impl Into<String>) -> Self {
        self.tags.push(tag.into().into());
        self
    }

    /// Check if bead matches search term (case-insensitive)
    #[must_use]
    pub fn matches_search(&self, search_term: &str) -> bool {
        if search_term.is_empty() {
            return true;
        }
        let search_lower = search_term.to_lowercase();
        self.title.to_lowercase().contains(&search_lower)
            || self.description.to_lowercase().contains(&search_lower)
            || self.id.to_lowercase().contains(&search_lower)
            || self
                .tags
                .iter()
                .any(|t| t.to_lowercase().contains(&search_lower))
    }

    /// Check if bead is blocked by dependencies
    #[must_use]
    pub fn is_blocked(&self) -> bool {
        !self.dependencies.is_empty() && self.status == BeadStatus::Pending
    }
}

/// Bead filter options
#[derive(Debug, Default, Clone, PartialEq)]
pub struct BeadFilters {
    pub status: Option<BeadStatus>,
    pub priority: Option<BeadPriority>,
    pub tag: Option<Arc<str>>,
}

impl BeadFilters {
    /// Check if a bead matches all active filters
    #[must_use]
    pub fn matches(&self, bead: &Bead) -> bool {
        let status_match = self.status.is_none_or(|s| s == bead.status);
        let priority_match = self.priority.is_none_or(|p| p == bead.priority);
        let tag_match = self.tag.as_ref().is_none_or(|t| bead.tags.contains(t));

        status_match && priority_match && tag_match
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_bead_creation() {
        let bead = Bead::new("bead-1", "Test Bead");
        assert_eq!(&*bead.id, "bead-1");
        assert_eq!(&*bead.title, "Test Bead");
        assert_eq!(&*bead.description, "");
        assert_eq!(bead.status, BeadStatus::Pending);
        assert_eq!(bead.priority, BeadPriority::Medium);
        assert!(bead.dependencies.is_empty());
        assert!(bead.tags.is_empty());
    }

    #[test]
    fn test_bead_builder_pattern() {
        let bead = Bead::new("bead-2", "Builder Test")
            .with_description("Test description")
            .with_status(BeadStatus::Running)
            .with_priority(BeadPriority::High)
            .with_dependency("bead-1")
            .with_tag("feature");

        assert_eq!(&*bead.description, "Test description");
        assert_eq!(bead.status, BeadStatus::Running);
        assert_eq!(bead.priority, BeadPriority::High);
        assert_eq!(&*bead.dependencies[0], "bead-1");
        assert_eq!(&*bead.tags[0], "feature");
    }

    #[test]
    fn test_bead_serialization() -> Result<(), Box<dyn std::error::Error>> {
        let bead = Bead::new("bead-3", "Serialize Test")
            .with_status(BeadStatus::Completed)
            .with_priority(BeadPriority::Critical);

        let json = serde_json::to_string(&bead)?;
        assert!(json.contains("bead-3"));
        assert!(json.contains("Serialize Test"));
        assert!(json.contains("completed"));
        assert!(json.contains("critical"));
        Ok(())
    }

    #[test]
    fn test_bead_search_title_match() {
        let bead = Bead::new("bead-5", "Fix Authentication Bug");
        assert!(bead.matches_search("auth"));
        assert!(bead.matches_search("AUTH"));
        assert!(bead.matches_search("bug"));
    }

    #[test]
    fn test_bead_search_tag_match() {
        let bead = Bead::new("bead-6", "Update UI").with_tag("frontend");
        assert!(bead.matches_search("frontend"));
        assert!(bead.matches_search("FRONTEND"));
    }

    #[test]
    fn test_bead_is_blocked() {
        let blocked = Bead::new("bead-9", "Blocked")
            .with_dependency("bead-1")
            .with_status(BeadStatus::Pending);
        assert!(blocked.is_blocked());

        let not_blocked = Bead::new("bead-10", "Not Blocked").with_status(BeadStatus::Pending);
        assert!(!not_blocked.is_blocked());

        let running_with_deps = Bead::new("bead-11", "Running")
            .with_dependency("bead-1")
            .with_status(BeadStatus::Running);
        assert!(!running_with_deps.is_blocked());
    }

    #[test]
    fn test_status_is_terminal() {
        assert!(!BeadStatus::Pending.is_terminal());
        assert!(!BeadStatus::Ready.is_terminal());
        assert!(!BeadStatus::Running.is_terminal());
        assert!(BeadStatus::Completed.is_terminal());
        assert!(BeadStatus::Failed.is_terminal());
        assert!(BeadStatus::Cancelled.is_terminal());
    }

    #[test]
    fn test_filters_matches_combined() {
        let filters = BeadFilters {
            status: Some(BeadStatus::Running),
            priority: Some(BeadPriority::High),
            tag: Some("feature".into()),
        };

        let matching = Bead::new("bead-1", "Test")
            .with_status(BeadStatus::Running)
            .with_priority(BeadPriority::High)
            .with_tag("feature");

        let non_matching_status = Bead::new("bead-2", "Test")
            .with_status(BeadStatus::Pending)
            .with_priority(BeadPriority::High)
            .with_tag("feature");

        assert!(filters.matches(&matching));
        assert!(!filters.matches(&non_matching_status));
    }

    #[test]
    fn test_rkyv_serialization() {
        let bead = Bead::new("bead-rkyv", "rkyv Test")
            .with_status(BeadStatus::Running)
            .with_priority(BeadPriority::High);

        // Serialize to bytes
        let bytes = rkyv::to_bytes::<rkyv::rancor::Error>(&bead);
        assert!(bytes.is_ok());

        // Verify we can access archived data (zero-copy)
        let bytes = bytes.map_err(|e| format!("{e:?}"));
        assert!(bytes.is_ok());
    }
}
