//! Task model for task list component

use serde::{Deserialize, Serialize};

/// Task status enumeration
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
#[derive(Default)]
pub enum TaskStatus {
    #[default]
    Open,
    InProgress,
    Done,
}

/// Task priority enumeration
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
#[derive(Default)]
pub enum TaskPriority {
    Low,
    #[default]
    Medium,
    High,
}

/// Task type enumeration
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
#[derive(Default)]
pub enum TaskType {
    #[default]
    Feature,
    Bug,
    Chore,
}

/// Task data structure
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Task {
    pub id: String,
    pub title: String,
    pub description: String,
    pub status: TaskStatus,
    pub priority: TaskPriority,
    pub task_type: TaskType,
}

impl Task {
    /// Creates a new task with minimal required fields
    pub fn new(id: impl Into<String>, title: impl Into<String>) -> Self {
        Self {
            id: id.into(),
            title: title.into(),
            description: String::new(),
            status: TaskStatus::default(),
            priority: TaskPriority::default(),
            task_type: TaskType::default(),
        }
    }

    /// Builder pattern: set description
    pub fn with_description(mut self, description: impl Into<String>) -> Self {
        self.description = description.into();
        self
    }

    /// Builder pattern: set status
    pub fn with_status(mut self, status: TaskStatus) -> Self {
        self.status = status;
        self
    }

    /// Builder pattern: set priority
    pub fn with_priority(mut self, priority: TaskPriority) -> Self {
        self.priority = priority;
        self
    }

    /// Builder pattern: set task type
    pub fn with_type(mut self, task_type: TaskType) -> Self {
        self.task_type = task_type;
        self
    }

    /// Check if task matches search term (case-insensitive)
    pub fn matches_search(&self, search_term: &str) -> bool {
        if search_term.is_empty() {
            return true;
        }
        let search_lower = search_term.to_lowercase();
        self.title.to_lowercase().contains(&search_lower)
            || self.description.to_lowercase().contains(&search_lower)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_task_creation() {
        let task = Task::new("task-1", "Test Task");
        assert_eq!(task.id, "task-1");
        assert_eq!(task.title, "Test Task");
        assert_eq!(task.description, "");
        assert_eq!(task.status, TaskStatus::Open);
        assert_eq!(task.priority, TaskPriority::Medium);
        assert_eq!(task.task_type, TaskType::Feature);
    }

    #[test]
    fn test_task_builder_pattern() {
        let task = Task::new("task-2", "Builder Test")
            .with_description("Test description")
            .with_status(TaskStatus::InProgress)
            .with_priority(TaskPriority::High)
            .with_type(TaskType::Bug);

        assert_eq!(task.description, "Test description");
        assert_eq!(task.status, TaskStatus::InProgress);
        assert_eq!(task.priority, TaskPriority::High);
        assert_eq!(task.task_type, TaskType::Bug);
    }

    #[test]
    fn test_task_serialization() -> Result<(), Box<dyn std::error::Error>> {
        let task = Task::new("task-3", "Serialize Test")
            .with_status(TaskStatus::Done)
            .with_priority(TaskPriority::Low);

        let json = serde_json::to_string(&task)?;
        assert!(json.contains("task-3"));
        assert!(json.contains("Serialize Test"));
        assert!(json.contains("done"));
        assert!(json.contains("low"));
        Ok(())
    }

    #[test]
    fn test_task_deserialization() -> Result<(), Box<dyn std::error::Error>> {
        let json = r#"{
            "id": "task-4",
            "title": "Deserialize Test",
            "description": "Test desc",
            "status": "in_progress",
            "priority": "high",
            "task_type": "chore"
        }"#;

        let task: Task = serde_json::from_str(json)?;
        assert_eq!(task.id, "task-4");
        assert_eq!(task.title, "Deserialize Test");
        assert_eq!(task.description, "Test desc");
        assert_eq!(task.status, TaskStatus::InProgress);
        assert_eq!(task.priority, TaskPriority::High);
        assert_eq!(task.task_type, TaskType::Chore);
        Ok(())
    }

    #[test]
    fn test_task_search_title_match() {
        let task = Task::new("task-5", "Fix Authentication Bug");
        assert!(task.matches_search("auth"));
        assert!(task.matches_search("AUTH"));
        assert!(task.matches_search("authentication"));
        assert!(task.matches_search("bug"));
    }

    #[test]
    fn test_task_search_description_match() {
        let task =
            Task::new("task-6", "Update UI").with_description("Refactor the login component");
        assert!(task.matches_search("login"));
        assert!(task.matches_search("component"));
        assert!(task.matches_search("refactor"));
    }

    #[test]
    fn test_task_search_no_match() {
        let task = Task::new("task-7", "Add tests");
        assert!(!task.matches_search("xyz"));
        assert!(!task.matches_search("random"));
    }

    #[test]
    fn test_task_search_empty_term() {
        let task = Task::new("task-8", "Any task");
        assert!(task.matches_search(""));
    }

    #[test]
    fn test_status_default() {
        assert_eq!(TaskStatus::default(), TaskStatus::Open);
    }

    #[test]
    fn test_priority_default() {
        assert_eq!(TaskPriority::default(), TaskPriority::Medium);
    }

    #[test]
    fn test_type_default() {
        assert_eq!(TaskType::default(), TaskType::Feature);
    }
}
