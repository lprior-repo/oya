//! Bead model for issue tracking component

use serde::{Deserialize, Serialize};

/// Bead status enumeration matching the backend BeadScheduleState
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
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
    pub fn color(&self) -> &'static str {
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
    pub fn label(&self) -> &'static str {
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
    pub fn is_terminal(&self) -> bool {
        matches!(
            self,
            BeadStatus::Completed | BeadStatus::Failed | BeadStatus::Cancelled
        )
    }
}

/// Bead priority enumeration
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
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
    pub fn value(&self) -> u8 {
        match self {
            BeadPriority::Low => 1,
            BeadPriority::Medium => 2,
            BeadPriority::High => 3,
            BeadPriority::Critical => 4,
        }
    }

    /// Returns the display label
    pub fn label(&self) -> &'static str {
        match self {
            BeadPriority::Low => "Low",
            BeadPriority::Medium => "Medium",
            BeadPriority::High => "High",
            BeadPriority::Critical => "Critical",
        }
    }

    /// Returns the color for this priority
    pub fn color(&self) -> &'static str {
        match self {
            BeadPriority::Low => "#6b7280",
            BeadPriority::Medium => "#3b82f6",
            BeadPriority::High => "#f59e0b",
            BeadPriority::Critical => "#ef4444",
        }
    }
}

/// Bead data structure representing an issue/work item
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
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
    pub fn with_description(mut self, description: impl Into<String>) -> Self {
        self.description = description.into();
        self
    }

    /// Builder: set status
    pub fn with_status(mut self, status: BeadStatus) -> Self {
        self.status = status;
        self
    }

    /// Builder: set priority
    pub fn with_priority(mut self, priority: BeadPriority) -> Self {
        self.priority = priority;
        self
    }

    /// Builder: set dependencies
    pub fn with_dependencies(mut self, dependencies: Vec<String>) -> Self {
        self.dependencies = dependencies;
        self
    }

    /// Builder: add a single dependency
    pub fn with_dependency(mut self, dep: impl Into<String>) -> Self {
        self.dependencies.push(dep.into());
        self
    }

    /// Builder: set tags
    pub fn with_tags(mut self, tags: Vec<String>) -> Self {
        self.tags = tags;
        self
    }

    /// Builder: add a single tag
    pub fn with_tag(mut self, tag: impl Into<String>) -> Self {
        self.tags.push(tag.into());
        self
    }

    /// Check if bead matches search term (case-insensitive)
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
    pub fn matches(&self, bead: &Bead) -> bool {
        let status_match = self.status.map(|s| s == bead.status).unwrap_or(true);
        let priority_match = self.priority.map(|p| p == bead.priority).unwrap_or(true);
        let tag_match = self
            .tag
            .as_ref()
            .map(|t| bead.tags.contains(t))
            .unwrap_or(true);

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
    fn test_bead_deserialization() -> Result<(), Box<dyn std::error::Error>> {
        let json = r#"{
            "id": "bead-4",
            "title": "Deserialize Test",
            "description": "Test desc",
            "status": "running",
            "priority": "high",
            "dependencies": ["bead-1", "bead-2"],
            "tags": ["feature", "ui"],
            "created_at": "2026-02-02T00:00:00Z",
            "updated_at": "2026-02-02T00:00:00Z"
        }"#;

        let bead: Bead = serde_json::from_str(json)?;
        assert_eq!(bead.id, "bead-4");
        assert_eq!(bead.title, "Deserialize Test");
        assert_eq!(bead.description, "Test desc");
        assert_eq!(bead.status, BeadStatus::Running);
        assert_eq!(bead.priority, BeadPriority::High);
        assert_eq!(bead.dependencies.len(), 2);
        assert_eq!(bead.tags.len(), 2);
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
    fn test_bead_search_id_match() {
        let bead = Bead::new("src-abc123", "Some Task");
        assert!(bead.matches_search("abc123"));
        assert!(bead.matches_search("src-"));
    }

    #[test]
    fn test_bead_search_no_match() {
        let bead = Bead::new("bead-7", "Add tests");
        assert!(!bead.matches_search("xyz"));
        assert!(!bead.matches_search("random"));
    }

    #[test]
    fn test_bead_search_empty_term() {
        let bead = Bead::new("bead-8", "Any bead");
        assert!(bead.matches_search(""));
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
    fn test_status_colors() {
        assert_eq!(BeadStatus::Pending.color(), "#9ca3af");
        assert_eq!(BeadStatus::Ready.color(), "#3b82f6");
        assert_eq!(BeadStatus::Running.color(), "#f59e0b");
        assert_eq!(BeadStatus::Completed.color(), "#10b981");
        assert_eq!(BeadStatus::Failed.color(), "#ef4444");
        assert_eq!(BeadStatus::Cancelled.color(), "#6b7280");
    }

    #[test]
    fn test_priority_values() {
        assert_eq!(BeadPriority::Low.value(), 1);
        assert_eq!(BeadPriority::Medium.value(), 2);
        assert_eq!(BeadPriority::High.value(), 3);
        assert_eq!(BeadPriority::Critical.value(), 4);
    }

    #[test]
    fn test_filters_default() {
        let filters = BeadFilters::default();
        assert_eq!(filters.status, None);
        assert_eq!(filters.priority, None);
        assert_eq!(filters.tag, None);
    }

    #[test]
    fn test_filters_matches_all_when_no_filters() {
        let filters = BeadFilters::default();
        let bead = Bead::new("bead-1", "Test")
            .with_status(BeadStatus::Running)
            .with_priority(BeadPriority::High)
            .with_tag("feature");

        assert!(filters.matches(&bead));
    }

    #[test]
    fn test_filters_matches_status() {
        let filters = BeadFilters {
            status: Some(BeadStatus::Running),
            priority: None,
            tag: None,
        };

        let matching = Bead::new("bead-1", "Test").with_status(BeadStatus::Running);
        let non_matching = Bead::new("bead-2", "Test").with_status(BeadStatus::Pending);

        assert!(filters.matches(&matching));
        assert!(!filters.matches(&non_matching));
    }

    #[test]
    fn test_filters_matches_priority() {
        let filters = BeadFilters {
            status: None,
            priority: Some(BeadPriority::High),
            tag: None,
        };

        let matching = Bead::new("bead-1", "Test").with_priority(BeadPriority::High);
        let non_matching = Bead::new("bead-2", "Test").with_priority(BeadPriority::Low);

        assert!(filters.matches(&matching));
        assert!(!filters.matches(&non_matching));
    }

    #[test]
    fn test_filters_matches_tag() {
        let filters = BeadFilters {
            status: None,
            priority: None,
            tag: Some("feature".to_string()),
        };

        let matching = Bead::new("bead-1", "Test").with_tag("feature");
        let non_matching = Bead::new("bead-2", "Test").with_tag("bug");

        assert!(filters.matches(&matching));
        assert!(!filters.matches(&non_matching));
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
}
