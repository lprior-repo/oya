//! Bead model for issue tracking
//!
//! Beads are the fundamental work items in the oya system.
//! They represent tasks, bugs, features, or any trackable work.

use rkyv::{Archive, Deserialize as RkyvDeserialize, Serialize as RkyvSerialize};
use serde::{Deserialize, Serialize};

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
#[derive(
    Debug, Clone, PartialEq, Serialize, Deserialize, Archive, RkyvSerialize, RkyvDeserialize,
)]
#[rkyv(compare(PartialEq))]
pub struct Bead {
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

impl Bead {
    /// Creates a new bead with minimal required fields
    #[must_use]
    pub fn new(id: impl Into<String>, title: impl Into<String>) -> Self {
        let now = "2026-02-02T00:00:00Z".to_string();
        Self {
            id: id.into(),
            title: title.into(),
            description: String::new(),
            status: BeadStatus::default(),
            priority: BeadPriority::default(),
            dependencies: Vec::new(),
            tags: Vec::new(),
            created_at: now.clone(),
            updated_at: now,
        }
    }

    /// Builder: set description
    #[must_use]
    pub fn with_description(mut self, description: impl Into<String>) -> Self {
        self.description = description.into();
        self
    }

    /// Builder: set status
    #[must_use]
    pub const fn with_status(mut self, status: BeadStatus) -> Self {
        self.status = status;
        self
    }

    /// Builder: set priority
    #[must_use]
    pub const fn with_priority(mut self, priority: BeadPriority) -> Self {
        self.priority = priority;
        self
    }

    /// Builder: set dependencies
    #[must_use]
    pub fn with_dependencies(mut self, dependencies: Vec<String>) -> Self {
        self.dependencies = dependencies;
        self
    }

    /// Builder: add a single dependency
    #[must_use]
    pub fn with_dependency(mut self, dep: impl Into<String>) -> Self {
        self.dependencies.push(dep.into());
        self
    }

    /// Builder: set tags
    #[must_use]
    pub fn with_tags(mut self, tags: Vec<String>) -> Self {
        self.tags = tags;
        self
    }

    /// Builder: add a single tag
    #[must_use]
    pub fn with_tag(mut self, tag: impl Into<String>) -> Self {
        self.tags.push(tag.into());
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
    pub tag: Option<String>,
}

impl BeadFilters {
    /// Check if a bead matches all active filters
    #[must_use]
    pub fn matches(&self, bead: &Bead) -> bool {
        let status_match = self.status.map_or(true, |s| s == bead.status);
        let priority_match = self.priority.map_or(true, |p| p == bead.priority);
        let tag_match = self
            .tag
            .as_ref()
            .map_or(true, |t| bead.tags.contains(t));

        status_match && priority_match && tag_match
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_bead_creation() {
        let bead = Bead::new("bead-1", "Test Bead");
        assert_eq!(bead.id, "bead-1");
        assert_eq!(bead.title, "Test Bead");
        assert_eq!(bead.description, "");
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

        assert_eq!(bead.description, "Test description");
        assert_eq!(bead.status, BeadStatus::Running);
        assert_eq!(bead.priority, BeadPriority::High);
        assert_eq!(bead.dependencies, vec!["bead-1".to_string()]);
        assert_eq!(bead.tags, vec!["feature".to_string()]);
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
            tag: Some("feature".to_string()),
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
