//! Behavioral tests for filtering functionality
//!
//! These tests verify filter behavior across tasks and beads using BDD-style naming.

use crate::models::bead::{Bead, BeadFilters, BeadPriority, BeadStatus};
use crate::models::task::{Task, TaskPriority, TaskStatus, TaskType};
use crate::components::task_list::TaskFilters;

// ============================================================================
// TASK FILTER BEHAVIORS
// ============================================================================

#[test]
fn given_no_filters_when_matching_task_then_always_returns_true() {
    // Given
    let filters = TaskFilters::default();
    let task = Task::new("task-1", "Any Task")
        .with_status(TaskStatus::InProgress)
        .with_priority(TaskPriority::High)
        .with_type(TaskType::Bug);

    // When
    let result = filters.matches(&task);

    // Then
    assert!(result, "No filters should match all tasks");
}

#[test]
fn given_status_filter_when_task_matches_status_then_returns_true() {
    // Given
    let filters = TaskFilters {
        status: Some(TaskStatus::InProgress),
        priority: None,
        task_type: None,
    };
    let matching_task = Task::new("task-1", "Matching").with_status(TaskStatus::InProgress);

    // When
    let result = filters.matches(&matching_task);

    // Then
    assert!(result, "Task with matching status should match filter");
}

#[test]
fn given_status_filter_when_task_has_different_status_then_returns_false() {
    // Given
    let filters = TaskFilters {
        status: Some(TaskStatus::InProgress),
        priority: None,
        task_type: None,
    };
    let non_matching_task = Task::new("task-2", "Non-matching").with_status(TaskStatus::Open);

    // When
    let result = filters.matches(&non_matching_task);

    // Then
    assert!(
        !result,
        "Task with different status should not match filter"
    );
}

#[test]
fn given_priority_filter_when_task_matches_priority_then_returns_true() {
    // Given
    let filters = TaskFilters {
        status: None,
        priority: Some(TaskPriority::High),
        task_type: None,
    };
    let matching_task = Task::new("task-1", "High Priority").with_priority(TaskPriority::High);

    // When
    let result = filters.matches(&matching_task);

    // Then
    assert!(result, "Task with matching priority should match filter");
}

#[test]
fn given_type_filter_when_task_matches_type_then_returns_true() {
    // Given
    let filters = TaskFilters {
        status: None,
        priority: None,
        task_type: Some(TaskType::Bug),
    };
    let matching_task = Task::new("task-1", "Bug Fix").with_type(TaskType::Bug);

    // When
    let result = filters.matches(&matching_task);

    // Then
    assert!(result, "Task with matching type should match filter");
}

#[test]
fn given_multiple_filters_when_all_match_then_returns_true() {
    // Given
    let filters = TaskFilters {
        status: Some(TaskStatus::InProgress),
        priority: Some(TaskPriority::High),
        task_type: Some(TaskType::Bug),
    };
    let task = Task::new("task-1", "Complex Filter Test")
        .with_status(TaskStatus::InProgress)
        .with_priority(TaskPriority::High)
        .with_type(TaskType::Bug);

    // When
    let result = filters.matches(&task);

    // Then
    assert!(result, "Task matching all filters should match");
}

#[test]
fn given_multiple_filters_when_one_does_not_match_then_returns_false() {
    // Given
    let filters = TaskFilters {
        status: Some(TaskStatus::InProgress),
        priority: Some(TaskPriority::High),
        task_type: Some(TaskType::Bug),
    };
    let task = Task::new("task-1", "Partial Match")
        .with_status(TaskStatus::InProgress)
        .with_priority(TaskPriority::Low) // Different priority
        .with_type(TaskType::Bug);

    // When
    let result = filters.matches(&task);

    // Then
    assert!(
        !result,
        "Task not matching all filters should not match"
    );
}

// ============================================================================
// TASK SEARCH BEHAVIORS
// ============================================================================

#[test]
fn given_empty_search_when_matching_then_all_tasks_match() {
    // Given
    let task = Task::new("task-1", "Any Task");

    // When
    let result = task.matches_search("");

    // Then
    assert!(result, "Empty search should match all tasks");
}

#[test]
fn given_search_term_when_title_contains_term_then_matches() {
    // Given
    let task = Task::new("task-1", "Fix Authentication Bug");

    // When
    let result = task.matches_search("auth");

    // Then
    assert!(result, "Search should match title substring");
}

#[test]
fn given_search_term_when_description_contains_term_then_matches() {
    // Given
    let task =
        Task::new("task-1", "Update UI").with_description("Refactor the login component");

    // When
    let result = task.matches_search("login");

    // Then
    assert!(result, "Search should match description substring");
}

#[test]
fn given_uppercase_search_when_lowercase_in_content_then_matches() {
    // Given
    let task = Task::new("task-1", "authentication system");

    // When
    let result = task.matches_search("AUTHENTICATION");

    // Then
    assert!(result, "Search should be case-insensitive");
}

#[test]
fn given_search_term_when_not_in_task_then_does_not_match() {
    // Given
    let task = Task::new("task-1", "Add tests");

    // When
    let result = task.matches_search("authentication");

    // Then
    assert!(!result, "Search should not match unrelated content");
}

// ============================================================================
// BEAD FILTER BEHAVIORS
// ============================================================================

#[test]
fn given_no_bead_filters_when_matching_bead_then_always_returns_true() {
    // Given
    let filters = BeadFilters::default();
    let bead = Bead::new("bead-1", "Any Bead")
        .with_status(BeadStatus::Running)
        .with_priority(BeadPriority::High)
        .with_tag("feature");

    // When
    let result = filters.matches(&bead);

    // Then
    assert!(result, "No filters should match all beads");
}

#[test]
fn given_bead_status_filter_when_bead_matches_then_returns_true() {
    // Given
    let filters = BeadFilters {
        status: Some(BeadStatus::Running),
        priority: None,
        tag: None,
    };
    let bead = Bead::new("bead-1", "Running Bead").with_status(BeadStatus::Running);

    // When
    let result = filters.matches(&bead);

    // Then
    assert!(result, "Bead with matching status should match filter");
}

#[test]
fn given_bead_priority_filter_when_bead_matches_then_returns_true() {
    // Given
    let filters = BeadFilters {
        status: None,
        priority: Some(BeadPriority::Critical),
        tag: None,
    };
    let bead = Bead::new("bead-1", "Critical Bead").with_priority(BeadPriority::Critical);

    // When
    let result = filters.matches(&bead);

    // Then
    assert!(result, "Bead with matching priority should match filter");
}

#[test]
fn given_bead_tag_filter_when_bead_has_tag_then_returns_true() {
    // Given
    let filters = BeadFilters {
        status: None,
        priority: None,
        tag: Some("backend".to_string()),
    };
    let bead = Bead::new("bead-1", "Backend Work").with_tag("backend");

    // When
    let result = filters.matches(&bead);

    // Then
    assert!(result, "Bead with matching tag should match filter");
}

#[test]
fn given_bead_tag_filter_when_bead_missing_tag_then_returns_false() {
    // Given
    let filters = BeadFilters {
        status: None,
        priority: None,
        tag: Some("backend".to_string()),
    };
    let bead = Bead::new("bead-1", "Frontend Work").with_tag("frontend");

    // When
    let result = filters.matches(&bead);

    // Then
    assert!(!result, "Bead without matching tag should not match filter");
}

// ============================================================================
// BEAD SEARCH BEHAVIORS
// ============================================================================

#[test]
fn given_bead_search_when_id_contains_term_then_matches() {
    // Given
    let bead = Bead::new("src-abc123", "Some Work");

    // When
    let result = bead.matches_search("abc123");

    // Then
    assert!(result, "Search should match bead ID");
}

#[test]
fn given_bead_search_when_tag_contains_term_then_matches() {
    // Given
    let bead = Bead::new("bead-1", "Tagged Work").with_tag("orchestrator");

    // When
    let result = bead.matches_search("orchestrator");

    // Then
    assert!(result, "Search should match bead tags");
}

#[test]
fn given_bead_search_when_title_contains_term_then_matches() {
    // Given
    let bead = Bead::new("bead-1", "Implement WebSocket Server");

    // When
    let result = bead.matches_search("websocket");

    // Then
    assert!(result, "Search should match bead title (case-insensitive)");
}

#[test]
fn given_bead_search_when_description_contains_term_then_matches() {
    // Given
    let bead = Bead::new("bead-1", "API Work")
        .with_description("Implement REST endpoints with authentication");

    // When
    let result = bead.matches_search("authentication");

    // Then
    assert!(result, "Search should match bead description");
}

// ============================================================================
// BEAD STATE BEHAVIORS
// ============================================================================

#[test]
fn given_bead_with_dependencies_when_pending_then_is_blocked() {
    // Given
    let bead = Bead::new("bead-1", "Blocked Bead")
        .with_dependency("bead-0")
        .with_status(BeadStatus::Pending);

    // When
    let result = bead.is_blocked();

    // Then
    assert!(result, "Pending bead with dependencies should be blocked");
}

#[test]
fn given_bead_with_dependencies_when_running_then_not_blocked() {
    // Given
    let bead = Bead::new("bead-1", "Running Bead")
        .with_dependency("bead-0")
        .with_status(BeadStatus::Running);

    // When
    let result = bead.is_blocked();

    // Then
    assert!(!result, "Running bead should not be blocked");
}

#[test]
fn given_bead_without_dependencies_when_pending_then_not_blocked() {
    // Given
    let bead = Bead::new("bead-1", "Independent Bead").with_status(BeadStatus::Pending);

    // When
    let result = bead.is_blocked();

    // Then
    assert!(!result, "Bead without dependencies should not be blocked");
}

#[test]
fn given_completed_status_when_checked_then_is_terminal() {
    assert!(
        BeadStatus::Completed.is_terminal(),
        "Completed should be terminal"
    );
}

#[test]
fn given_failed_status_when_checked_then_is_terminal() {
    assert!(BeadStatus::Failed.is_terminal(), "Failed should be terminal");
}

#[test]
fn given_cancelled_status_when_checked_then_is_terminal() {
    assert!(
        BeadStatus::Cancelled.is_terminal(),
        "Cancelled should be terminal"
    );
}

#[test]
fn given_running_status_when_checked_then_is_not_terminal() {
    assert!(
        !BeadStatus::Running.is_terminal(),
        "Running should not be terminal"
    );
}

#[test]
fn given_pending_status_when_checked_then_is_not_terminal() {
    assert!(
        !BeadStatus::Pending.is_terminal(),
        "Pending should not be terminal"
    );
}

#[test]
fn given_ready_status_when_checked_then_is_not_terminal() {
    assert!(
        !BeadStatus::Ready.is_terminal(),
        "Ready should not be terminal"
    );
}
