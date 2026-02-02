//! Behavioral tests for mock data
//!
//! These tests verify that mock data behaves correctly and consistently,
//! ensuring reliable development and testing scenarios.

use crate::models::bead::{BeadPriority, BeadStatus};
use crate::models::mock::{mock_beads, mock_graph, mock_tasks, StatusSummary, TaskSummary};
use crate::models::task::TaskStatus;

// ============================================================================
// GIVEN: Mock data exists
// ============================================================================

#[test]
fn given_mock_tasks_when_loaded_then_contains_variety_of_statuses() {
    // Given
    let tasks = mock_tasks();

    // When we check for status variety
    let open_count = tasks.iter().filter(|t| t.status == TaskStatus::Open).count();
    let in_progress_count = tasks
        .iter()
        .filter(|t| t.status == TaskStatus::InProgress)
        .count();
    let done_count = tasks.iter().filter(|t| t.status == TaskStatus::Done).count();

    // Then each status should be represented
    assert!(open_count > 0, "Should have open tasks");
    assert!(in_progress_count > 0, "Should have in-progress tasks");
    assert!(done_count > 0, "Should have done tasks");
}

#[test]
fn given_mock_beads_when_loaded_then_contains_variety_of_statuses() {
    // Given
    let beads = mock_beads();

    // When we check for status variety
    let pending = beads
        .iter()
        .filter(|b| b.status == BeadStatus::Pending)
        .count();
    let running = beads
        .iter()
        .filter(|b| b.status == BeadStatus::Running)
        .count();
    let completed = beads
        .iter()
        .filter(|b| b.status == BeadStatus::Completed)
        .count();
    let failed = beads
        .iter()
        .filter(|b| b.status == BeadStatus::Failed)
        .count();

    // Then each status should be represented
    assert!(pending > 0, "Should have pending beads");
    assert!(running > 0, "Should have running beads");
    assert!(completed > 0, "Should have completed beads");
    assert!(failed > 0, "Should have failed beads");
}

#[test]
fn given_mock_beads_when_loaded_then_contains_dependencies() {
    // Given
    let beads = mock_beads();

    // When we check for dependencies
    let beads_with_deps = beads.iter().filter(|b| !b.dependencies.is_empty()).count();

    // Then some beads should have dependencies
    assert!(beads_with_deps > 0, "Should have beads with dependencies");
}

#[test]
fn given_mock_beads_when_loaded_then_contains_tags() {
    // Given
    let beads = mock_beads();

    // When we check for tags
    let beads_with_tags = beads.iter().filter(|b| !b.tags.is_empty()).count();

    // Then most beads should have tags
    assert!(beads_with_tags > 0, "Should have beads with tags");
}

// ============================================================================
// GIVEN: Mock graph data exists
// ============================================================================

#[test]
fn given_mock_graph_when_loaded_then_has_nodes_and_edges() {
    // Given
    let graph = mock_graph();

    // Then
    assert!(!graph.nodes.is_empty(), "Graph should have nodes");
    assert!(!graph.edges.is_empty(), "Graph should have edges");
}

#[test]
fn given_mock_graph_when_edges_checked_then_all_reference_valid_nodes() {
    // Given
    let graph = mock_graph();
    let node_ids: Vec<&str> = graph.nodes.iter().map(|n| n.id.as_str()).collect();

    // When we check each edge
    for edge in &graph.edges {
        // Then both source and target should reference valid nodes
        assert!(
            node_ids.contains(&edge.source.as_str()),
            "Edge source '{}' should reference a valid node",
            edge.source
        );
        assert!(
            node_ids.contains(&edge.target.as_str()),
            "Edge target '{}' should reference a valid node",
            edge.target
        );
    }
}

#[test]
fn given_mock_graph_when_nodes_checked_then_all_have_positions() {
    // Given
    let graph = mock_graph();

    // When we check each node
    for node in &graph.nodes {
        // Then positions should be finite (not NaN or infinite)
        assert!(node.x.is_finite(), "Node x position should be finite");
        assert!(node.y.is_finite(), "Node y position should be finite");
    }
}

#[test]
fn given_mock_graph_when_nodes_checked_then_all_have_colors() {
    // Given
    let graph = mock_graph();

    // When we check each node
    for node in &graph.nodes {
        // Then color should be present and valid hex
        assert!(node.color.is_some(), "Node should have a color");
        let color = node.color.as_ref().map(|c| c.as_str()).unwrap_or_default();
        assert!(
            color.starts_with('#'),
            "Color '{}' should be hex format",
            color
        );
    }
}

// ============================================================================
// GIVEN: Status summary computed
// ============================================================================

#[test]
fn given_mock_beads_when_summary_computed_then_total_matches() {
    // Given
    let beads = mock_beads();

    // When
    let summary = StatusSummary::from_beads(&beads);

    // Then total should match bead count
    assert_eq!(
        summary.total(),
        beads.len(),
        "Summary total should match bead count"
    );
}

#[test]
fn given_mock_beads_when_summary_computed_then_active_plus_terminal_equals_total() {
    // Given
    let beads = mock_beads();

    // When
    let summary = StatusSummary::from_beads(&beads);

    // Then active + terminal = total
    assert_eq!(
        summary.active() + summary.terminal(),
        summary.total(),
        "Active + terminal should equal total"
    );
}

#[test]
fn given_mock_tasks_when_summary_computed_then_total_matches() {
    // Given
    let tasks = mock_tasks();

    // When
    let summary = TaskSummary::from_tasks(&tasks);

    // Then
    assert_eq!(
        summary.total(),
        tasks.len(),
        "Summary total should match task count"
    );
}

// ============================================================================
// GIVEN: IDs must be unique
// ============================================================================

#[test]
fn given_mock_tasks_when_ids_checked_then_all_unique() {
    // Given
    let tasks = mock_tasks();

    // When we collect all IDs
    let ids: std::collections::HashSet<_> = tasks.iter().map(|t| &t.id).collect();

    // Then ID count should match task count
    assert_eq!(ids.len(), tasks.len(), "All task IDs should be unique");
}

#[test]
fn given_mock_beads_when_ids_checked_then_all_unique() {
    // Given
    let beads = mock_beads();

    // When we collect all IDs
    let ids: std::collections::HashSet<_> = beads.iter().map(|b| &b.id).collect();

    // Then ID count should match bead count
    assert_eq!(ids.len(), beads.len(), "All bead IDs should be unique");
}

#[test]
fn given_mock_graph_when_node_ids_checked_then_all_unique() {
    // Given
    let graph = mock_graph();

    // When we collect all node IDs
    let ids: std::collections::HashSet<_> = graph.nodes.iter().map(|n| &n.id).collect();

    // Then ID count should match node count
    assert_eq!(
        ids.len(),
        graph.nodes.len(),
        "All node IDs should be unique"
    );
}

// ============================================================================
// GIVEN: Priority distribution
// ============================================================================

#[test]
fn given_mock_beads_when_priorities_checked_then_distribution_is_realistic() {
    // Given
    let beads = mock_beads();

    // When we count priorities
    let critical = beads
        .iter()
        .filter(|b| b.priority == BeadPriority::Critical)
        .count();
    let high = beads
        .iter()
        .filter(|b| b.priority == BeadPriority::High)
        .count();
    let medium = beads
        .iter()
        .filter(|b| b.priority == BeadPriority::Medium)
        .count();
    let low = beads
        .iter()
        .filter(|b| b.priority == BeadPriority::Low)
        .count();

    // Then we should have a variety
    let total_priorities = critical + high + medium + low;
    assert_eq!(
        total_priorities,
        beads.len(),
        "All beads should have a priority"
    );

    // Critical should be rare
    assert!(
        critical <= beads.len() / 2,
        "Critical priority should not dominate"
    );
}
