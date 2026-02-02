//! Behavioral tests for Bead model and related functionality

use crate::models::bead::{Bead, BeadPriority, BeadStatus};

// ============================================================================
// BEAD CREATION BEHAVIORS
// ============================================================================

#[test]
fn given_minimal_args_when_creating_bead_then_has_sensible_defaults() {
    // Given/When
    let bead = Bead::new("bead-1", "Test Bead");

    // Then
    assert_eq!(bead.id, "bead-1");
    assert_eq!(bead.title, "Test Bead");
    assert_eq!(bead.description, "");
    assert_eq!(bead.status, BeadStatus::Pending);
    assert_eq!(bead.priority, BeadPriority::Medium);
    assert!(bead.dependencies.is_empty());
    assert!(bead.tags.is_empty());
}

#[test]
fn given_builder_pattern_when_chaining_then_all_fields_set() {
    // Given/When
    let bead = Bead::new("bead-2", "Builder Bead")
        .with_description("Detailed description")
        .with_status(BeadStatus::Running)
        .with_priority(BeadPriority::High)
        .with_dependency("bead-1")
        .with_tag("backend");

    // Then
    assert_eq!(bead.description, "Detailed description");
    assert_eq!(bead.status, BeadStatus::Running);
    assert_eq!(bead.priority, BeadPriority::High);
    assert_eq!(bead.dependencies, vec!["bead-1".to_string()]);
    assert_eq!(bead.tags, vec!["backend".to_string()]);
}

#[test]
fn given_builder_when_adding_multiple_dependencies_then_all_added() {
    // Given
    let bead = Bead::new("bead-3", "Multi-dep Bead")
        .with_dependency("bead-1")
        .with_dependency("bead-2");

    // Then
    assert_eq!(bead.dependencies.len(), 2);
    assert!(bead.dependencies.contains(&"bead-1".to_string()));
    assert!(bead.dependencies.contains(&"bead-2".to_string()));
}

#[test]
fn given_builder_when_setting_dependencies_as_vec_then_replaces_all() {
    // Given
    let bead = Bead::new("bead-4", "Vec-dep Bead")
        .with_dependency("old-dep")
        .with_dependencies(vec!["new-1".into(), "new-2".into()]);

    // Then
    assert_eq!(bead.dependencies.len(), 2);
    assert!(!bead.dependencies.contains(&"old-dep".to_string()));
}

// ============================================================================
// BEAD STATUS BEHAVIORS
// ============================================================================

#[test]
fn given_bead_status_pending_when_default_then_is_pending() {
    assert_eq!(BeadStatus::default(), BeadStatus::Pending);
}

#[test]
fn given_all_statuses_when_getting_color_then_returns_valid_hex() {
    let statuses = [
        BeadStatus::Pending,
        BeadStatus::Ready,
        BeadStatus::Running,
        BeadStatus::Completed,
        BeadStatus::Failed,
        BeadStatus::Cancelled,
    ];

    for status in statuses {
        let color = status.color();
        assert!(
            color.starts_with('#'),
            "Status {:?} color should be hex format",
            status
        );
        assert_eq!(
            color.len(),
            7,
            "Status {:?} color should be #RRGGBB format",
            status
        );
    }
}

#[test]
fn given_all_statuses_when_getting_label_then_returns_non_empty() {
    let statuses = [
        BeadStatus::Pending,
        BeadStatus::Ready,
        BeadStatus::Running,
        BeadStatus::Completed,
        BeadStatus::Failed,
        BeadStatus::Cancelled,
    ];

    for status in statuses {
        let label = status.label();
        assert!(
            !label.is_empty(),
            "Status {:?} should have a non-empty label",
            status
        );
    }
}

// ============================================================================
// BEAD PRIORITY BEHAVIORS
// ============================================================================

#[test]
fn given_bead_priority_medium_when_default_then_is_medium() {
    assert_eq!(BeadPriority::default(), BeadPriority::Medium);
}

#[test]
fn given_all_priorities_when_getting_value_then_increases_with_importance() {
    // Given
    let low = BeadPriority::Low.value();
    let medium = BeadPriority::Medium.value();
    let high = BeadPriority::High.value();
    let critical = BeadPriority::Critical.value();

    // Then priority values should increase
    assert!(low < medium, "Low should be less than medium");
    assert!(medium < high, "Medium should be less than high");
    assert!(high < critical, "High should be less than critical");
}

#[test]
fn given_all_priorities_when_getting_color_then_returns_valid_hex() {
    let priorities = [
        BeadPriority::Low,
        BeadPriority::Medium,
        BeadPriority::High,
        BeadPriority::Critical,
    ];

    for priority in priorities {
        let color = priority.color();
        assert!(
            color.starts_with('#'),
            "Priority {:?} color should be hex format",
            priority
        );
    }
}

// ============================================================================
// BEAD SERIALIZATION BEHAVIORS
// ============================================================================

#[test]
fn given_bead_when_serialized_then_uses_snake_case_for_status() {
    // Given
    let bead = Bead::new("bead-1", "Test").with_status(BeadStatus::Running);

    // When
    let json = serde_json::to_string(&bead).expect("serialization should work");

    // Then
    assert!(json.contains("\"running\""), "Status should be snake_case in JSON");
}

#[test]
fn given_bead_when_serialized_and_deserialized_then_equals_original() {
    // Given
    let original = Bead::new("bead-1", "Round Trip Test")
        .with_description("Testing serialization")
        .with_status(BeadStatus::Completed)
        .with_priority(BeadPriority::Critical)
        .with_dependencies(vec!["dep-1".into(), "dep-2".into()])
        .with_tags(vec!["backend".into(), "api".into()]);

    // When
    let json = serde_json::to_string(&original).expect("serialization should work");
    let restored: Bead = serde_json::from_str(&json).expect("deserialization should work");

    // Then
    assert_eq!(restored.id, original.id);
    assert_eq!(restored.title, original.title);
    assert_eq!(restored.description, original.description);
    assert_eq!(restored.status, original.status);
    assert_eq!(restored.priority, original.priority);
    assert_eq!(restored.dependencies, original.dependencies);
    assert_eq!(restored.tags, original.tags);
}

#[test]
fn given_json_with_all_statuses_when_deserialized_then_parses_correctly() {
    // Given
    let statuses = [
        ("pending", BeadStatus::Pending),
        ("ready", BeadStatus::Ready),
        ("running", BeadStatus::Running),
        ("completed", BeadStatus::Completed),
        ("failed", BeadStatus::Failed),
        ("cancelled", BeadStatus::Cancelled),
    ];

    for (json_value, expected_status) in statuses {
        // When
        let json = format!(
            r#"{{
                "id":"b",
                "title":"T",
                "description":"",
                "status":"{}",
                "priority":"medium",
                "dependencies":[],
                "tags":[],
                "created_at":"2026-02-02T00:00:00Z",
                "updated_at":"2026-02-02T00:00:00Z"
            }}"#,
            json_value
        );
        let bead: Bead = serde_json::from_str(&json).expect("deserialization should work");

        // Then
        assert_eq!(
            bead.status, expected_status,
            "Status '{}' should deserialize correctly",
            json_value
        );
    }
}

#[test]
fn given_json_with_all_priorities_when_deserialized_then_parses_correctly() {
    // Given
    let priorities = [
        ("low", BeadPriority::Low),
        ("medium", BeadPriority::Medium),
        ("high", BeadPriority::High),
        ("critical", BeadPriority::Critical),
    ];

    for (json_value, expected_priority) in priorities {
        // When
        let json = format!(
            r#"{{
                "id":"b",
                "title":"T",
                "description":"",
                "status":"pending",
                "priority":"{}",
                "dependencies":[],
                "tags":[],
                "created_at":"2026-02-02T00:00:00Z",
                "updated_at":"2026-02-02T00:00:00Z"
            }}"#,
            json_value
        );
        let bead: Bead = serde_json::from_str(&json).expect("deserialization should work");

        // Then
        assert_eq!(
            bead.priority, expected_priority,
            "Priority '{}' should deserialize correctly",
            json_value
        );
    }
}

// ============================================================================
// BEAD DEPENDENCY BEHAVIORS
// ============================================================================

#[test]
fn given_bead_with_no_deps_when_pending_then_not_blocked() {
    // Given
    let bead = Bead::new("bead-1", "Independent").with_status(BeadStatus::Pending);

    // Then
    assert!(!bead.is_blocked());
}

#[test]
fn given_bead_with_deps_when_pending_then_is_blocked() {
    // Given
    let bead = Bead::new("bead-1", "Dependent")
        .with_dependency("bead-0")
        .with_status(BeadStatus::Pending);

    // Then
    assert!(bead.is_blocked());
}

#[test]
fn given_bead_with_deps_when_ready_then_not_blocked() {
    // Given
    let bead = Bead::new("bead-1", "Ready Bead")
        .with_dependency("bead-0")
        .with_status(BeadStatus::Ready);

    // Then
    assert!(!bead.is_blocked(), "Ready beads are not blocked");
}

#[test]
fn given_bead_with_deps_when_completed_then_not_blocked() {
    // Given
    let bead = Bead::new("bead-1", "Done Bead")
        .with_dependency("bead-0")
        .with_status(BeadStatus::Completed);

    // Then
    assert!(!bead.is_blocked(), "Completed beads are not blocked");
}
