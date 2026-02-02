//! Behavioral tests for Task model and related functionality

use crate::models::task::{Task, TaskPriority, TaskStatus, TaskType};

// ============================================================================
// TASK CREATION BEHAVIORS
// ============================================================================

#[test]
fn given_minimal_args_when_creating_task_then_has_sensible_defaults() {
    // Given/When
    let task = Task::new("task-1", "Test Task");

    // Then
    assert_eq!(task.id, "task-1");
    assert_eq!(task.title, "Test Task");
    assert_eq!(task.description, "");
    assert_eq!(task.status, TaskStatus::Open);
    assert_eq!(task.priority, TaskPriority::Medium);
    assert_eq!(task.task_type, TaskType::Feature);
}

#[test]
fn given_builder_pattern_when_chaining_then_all_fields_set() {
    // Given/When
    let task = Task::new("task-2", "Builder Task")
        .with_description("Detailed description")
        .with_status(TaskStatus::InProgress)
        .with_priority(TaskPriority::High)
        .with_type(TaskType::Bug);

    // Then
    assert_eq!(task.description, "Detailed description");
    assert_eq!(task.status, TaskStatus::InProgress);
    assert_eq!(task.priority, TaskPriority::High);
    assert_eq!(task.task_type, TaskType::Bug);
}

// ============================================================================
// TASK STATUS BEHAVIORS
// ============================================================================

#[test]
fn given_task_status_open_when_default_then_is_open() {
    assert_eq!(TaskStatus::default(), TaskStatus::Open);
}

#[test]
fn given_task_status_when_serialized_then_uses_snake_case() {
    // Given
    let task = Task::new("task-1", "Test").with_status(TaskStatus::InProgress);

    // When
    let json = serde_json::to_string(&task).expect("serialization should work");

    // Then
    assert!(json.contains("in_progress"), "Status should be snake_case in JSON");
}

// ============================================================================
// TASK PRIORITY BEHAVIORS
// ============================================================================

#[test]
fn given_task_priority_medium_when_default_then_is_medium() {
    assert_eq!(TaskPriority::default(), TaskPriority::Medium);
}

#[test]
fn given_task_priority_when_serialized_then_uses_lowercase() {
    // Given
    let task = Task::new("task-1", "Test").with_priority(TaskPriority::High);

    // When
    let json = serde_json::to_string(&task).expect("serialization should work");

    // Then
    assert!(json.contains("\"high\""), "Priority should be lowercase in JSON");
}

// ============================================================================
// TASK TYPE BEHAVIORS
// ============================================================================

#[test]
fn given_task_type_feature_when_default_then_is_feature() {
    assert_eq!(TaskType::default(), TaskType::Feature);
}

#[test]
fn given_task_type_when_serialized_then_uses_lowercase() {
    // Given
    let task = Task::new("task-1", "Test").with_type(TaskType::Bug);

    // When
    let json = serde_json::to_string(&task).expect("serialization should work");

    // Then
    assert!(json.contains("\"bug\""), "Type should be lowercase in JSON");
}

// ============================================================================
// TASK SERIALIZATION ROUND-TRIP BEHAVIORS
// ============================================================================

#[test]
fn given_task_when_serialized_and_deserialized_then_equals_original() {
    // Given
    let original = Task::new("task-1", "Round Trip Test")
        .with_description("Testing serialization")
        .with_status(TaskStatus::Done)
        .with_priority(TaskPriority::Low)
        .with_type(TaskType::Chore);

    // When
    let json = serde_json::to_string(&original).expect("serialization should work");
    let restored: Task = serde_json::from_str(&json).expect("deserialization should work");

    // Then
    assert_eq!(restored.id, original.id);
    assert_eq!(restored.title, original.title);
    assert_eq!(restored.description, original.description);
    assert_eq!(restored.status, original.status);
    assert_eq!(restored.priority, original.priority);
    assert_eq!(restored.task_type, original.task_type);
}

#[test]
fn given_json_with_all_statuses_when_deserialized_then_parses_correctly() {
    // Given
    let statuses = [
        ("open", TaskStatus::Open),
        ("in_progress", TaskStatus::InProgress),
        ("done", TaskStatus::Done),
    ];

    for (json_value, expected_status) in statuses {
        // When
        let json = format!(
            r#"{{"id":"t","title":"T","description":"","status":"{}","priority":"medium","task_type":"feature"}}"#,
            json_value
        );
        let task: Task = serde_json::from_str(&json).expect("deserialization should work");

        // Then
        assert_eq!(
            task.status, expected_status,
            "Status '{}' should deserialize correctly",
            json_value
        );
    }
}

#[test]
fn given_json_with_all_priorities_when_deserialized_then_parses_correctly() {
    // Given
    let priorities = [
        ("low", TaskPriority::Low),
        ("medium", TaskPriority::Medium),
        ("high", TaskPriority::High),
    ];

    for (json_value, expected_priority) in priorities {
        // When
        let json = format!(
            r#"{{"id":"t","title":"T","description":"","status":"open","priority":"{}","task_type":"feature"}}"#,
            json_value
        );
        let task: Task = serde_json::from_str(&json).expect("deserialization should work");

        // Then
        assert_eq!(
            task.priority, expected_priority,
            "Priority '{}' should deserialize correctly",
            json_value
        );
    }
}

#[test]
fn given_json_with_all_types_when_deserialized_then_parses_correctly() {
    // Given
    let types = [
        ("feature", TaskType::Feature),
        ("bug", TaskType::Bug),
        ("chore", TaskType::Chore),
    ];

    for (json_value, expected_type) in types {
        // When
        let json = format!(
            r#"{{"id":"t","title":"T","description":"","status":"open","priority":"medium","task_type":"{}"}}"#,
            json_value
        );
        let task: Task = serde_json::from_str(&json).expect("deserialization should work");

        // Then
        assert_eq!(
            task.task_type, expected_type,
            "Type '{}' should deserialize correctly",
            json_value
        );
    }
}
