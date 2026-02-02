//! Mock data module for UI development and testing
//!
//! This module provides realistic sample data for all UI components,
//! enabling consistent development and testing without backend dependency.

use super::bead::{Bead, BeadPriority, BeadStatus};
use super::task::{Task, TaskPriority, TaskStatus, TaskType};
use super::{Graph, GraphEdge, GraphNode};

/// Provides mock task data for development
pub fn mock_tasks() -> Vec<Task> {
    vec![
        Task::new("src-3s0.1", "Set up project structure")
            .with_description(
                "Initialize the oya-ui crate with Leptos 0.7 and configure Trunk for WASM builds",
            )
            .with_status(TaskStatus::Done)
            .with_priority(TaskPriority::High)
            .with_type(TaskType::Feature),
        Task::new("src-3s0.2", "Implement WebSocket client")
            .with_description(
                "Create Railway-Oriented WebSocket client for real-time event streaming",
            )
            .with_status(TaskStatus::Done)
            .with_priority(TaskPriority::High)
            .with_type(TaskType::Feature),
        Task::new("src-3s0.3", "Create Dashboard component")
            .with_description("Build the main dashboard with status widgets and task cards")
            .with_status(TaskStatus::InProgress)
            .with_priority(TaskPriority::High)
            .with_type(TaskType::Feature),
        Task::new("src-3s0.4", "Implement graph visualization")
            .with_description("Canvas-based DAG rendering with pan/zoom controls")
            .with_status(TaskStatus::Open)
            .with_priority(TaskPriority::Medium)
            .with_type(TaskType::Feature),
        Task::new("src-3s0.5", "Add task filtering")
            .with_description("Implement filter controls for status, priority, and type")
            .with_status(TaskStatus::InProgress)
            .with_priority(TaskPriority::Medium)
            .with_type(TaskType::Feature),
        Task::new("bug-001", "Fix memory leak in canvas resize")
            .with_description("Canvas context not being released on window resize")
            .with_status(TaskStatus::Open)
            .with_priority(TaskPriority::High)
            .with_type(TaskType::Bug),
        Task::new("chore-001", "Update dependencies")
            .with_description("Update Leptos to latest 0.7.x release")
            .with_status(TaskStatus::Open)
            .with_priority(TaskPriority::Low)
            .with_type(TaskType::Chore),
        Task::new("src-3s0.6", "Implement Beads page")
            .with_description("Create bead list component with filtering and status display")
            .with_status(TaskStatus::Open)
            .with_priority(TaskPriority::Medium)
            .with_type(TaskType::Feature),
    ]
}

/// Provides mock bead data for development
pub fn mock_beads() -> Vec<Bead> {
    vec![
        Bead::new("src-abc12", "Implement event sourcing")
            .with_description("Set up event bus with durable storage and replay capabilities")
            .with_status(BeadStatus::Completed)
            .with_priority(BeadPriority::High)
            .with_tags(vec!["backend".into(), "events".into()]),
        Bead::new("src-def34", "Create orchestrator DAG")
            .with_description("Implement DAG-based execution ordering with dependency tracking")
            .with_status(BeadStatus::Running)
            .with_priority(BeadPriority::High)
            .with_dependency("src-abc12")
            .with_tags(vec!["backend".into(), "orchestrator".into()]),
        Bead::new("src-ghi56", "Build REST API endpoints")
            .with_description("Implement bead CRUD and workflow management endpoints")
            .with_status(BeadStatus::Running)
            .with_priority(BeadPriority::Medium)
            .with_dependency("src-abc12")
            .with_tags(vec!["backend".into(), "api".into()]),
        Bead::new("src-jkl78", "Implement WebSocket broadcast")
            .with_description("Add real-time event streaming over WebSocket")
            .with_status(BeadStatus::Ready)
            .with_priority(BeadPriority::High)
            .with_dependencies(vec!["src-abc12".into(), "src-ghi56".into()])
            .with_tags(vec!["backend".into(), "websocket".into()]),
        Bead::new("src-mno90", "Create UI dashboard")
            .with_description("Leptos-based dashboard with real-time updates")
            .with_status(BeadStatus::Pending)
            .with_priority(BeadPriority::Medium)
            .with_dependency("src-jkl78")
            .with_tags(vec!["frontend".into(), "ui".into()]),
        Bead::new("src-pqr12", "Add graph visualization")
            .with_description("Canvas-based DAG rendering with pan/zoom")
            .with_status(BeadStatus::Pending)
            .with_priority(BeadPriority::Medium)
            .with_dependency("src-mno90")
            .with_tags(vec!["frontend".into(), "canvas".into()]),
        Bead::new("bug-stu34", "Fix event replay ordering")
            .with_description("Events not replaying in correct timestamp order")
            .with_status(BeadStatus::Failed)
            .with_priority(BeadPriority::Critical)
            .with_tags(vec!["backend".into(), "bug".into()]),
        Bead::new("src-vwx56", "Implement bead scheduling")
            .with_description("Scheduler actor for bead execution management")
            .with_status(BeadStatus::Cancelled)
            .with_priority(BeadPriority::Low)
            .with_tags(vec!["backend".into(), "scheduler".into()]),
    ]
}

/// Provides mock graph data for visualization
pub fn mock_graph() -> Graph {
    let mut graph = Graph::new();

    // Add nodes representing beads
    let nodes = vec![
        ("src-abc12", "Event Sourcing", 100.0, 50.0, "#10b981"),
        ("src-def34", "DAG Orchestrator", 250.0, 100.0, "#f59e0b"),
        ("src-ghi56", "REST API", 250.0, 200.0, "#f59e0b"),
        ("src-jkl78", "WebSocket", 400.0, 150.0, "#3b82f6"),
        ("src-mno90", "UI Dashboard", 550.0, 150.0, "#9ca3af"),
        ("src-pqr12", "Graph Viz", 700.0, 150.0, "#9ca3af"),
        ("bug-stu34", "Bug Fix", 100.0, 250.0, "#ef4444"),
    ];

    for (id, label, x, y, color) in nodes {
        graph.add_node(GraphNode {
            id: id.to_string(),
            label: label.to_string(),
            x,
            y,
            color: Some(color.to_string()),
        });
    }

    // Add edges representing dependencies
    let edges = vec![
        ("src-abc12", "src-def34"),
        ("src-abc12", "src-ghi56"),
        ("src-abc12", "src-jkl78"),
        ("src-ghi56", "src-jkl78"),
        ("src-jkl78", "src-mno90"),
        ("src-mno90", "src-pqr12"),
    ];

    for (source, target) in edges {
        graph.add_edge(GraphEdge {
            source: source.to_string(),
            target: target.to_string(),
            weight: Some(1.0),
        });
    }

    graph
}

/// Status counts for dashboard summary widgets
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct StatusSummary {
    pub pending: usize,
    pub ready: usize,
    pub running: usize,
    pub completed: usize,
    pub failed: usize,
    pub cancelled: usize,
}

impl StatusSummary {
    /// Compute status counts from beads
    pub fn from_beads(beads: &[Bead]) -> Self {
        let mut summary = Self::default();
        for bead in beads {
            match bead.status {
                BeadStatus::Pending => summary.pending += 1,
                BeadStatus::Ready => summary.ready += 1,
                BeadStatus::Running => summary.running += 1,
                BeadStatus::Completed => summary.completed += 1,
                BeadStatus::Failed => summary.failed += 1,
                BeadStatus::Cancelled => summary.cancelled += 1,
            }
        }
        summary
    }

    /// Total count of all beads
    pub fn total(&self) -> usize {
        self.pending + self.ready + self.running + self.completed + self.failed + self.cancelled
    }

    /// Count of active (non-terminal) beads
    pub fn active(&self) -> usize {
        self.pending + self.ready + self.running
    }

    /// Count of terminal beads
    pub fn terminal(&self) -> usize {
        self.completed + self.failed + self.cancelled
    }
}

/// Task status summary for dashboard
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct TaskSummary {
    pub open: usize,
    pub in_progress: usize,
    pub done: usize,
}

impl TaskSummary {
    /// Compute status counts from tasks
    pub fn from_tasks(tasks: &[Task]) -> Self {
        let mut summary = Self::default();
        for task in tasks {
            match task.status {
                TaskStatus::Open => summary.open += 1,
                TaskStatus::InProgress => summary.in_progress += 1,
                TaskStatus::Done => summary.done += 1,
            }
        }
        summary
    }

    /// Total count of all tasks
    pub fn total(&self) -> usize {
        self.open + self.in_progress + self.done
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_mock_tasks_not_empty() {
        let tasks = mock_tasks();
        assert!(!tasks.is_empty());
    }

    #[test]
    fn test_mock_tasks_have_variety() {
        let tasks = mock_tasks();

        // Check we have different statuses
        let has_open = tasks.iter().any(|t| t.status == TaskStatus::Open);
        let has_in_progress = tasks.iter().any(|t| t.status == TaskStatus::InProgress);
        let has_done = tasks.iter().any(|t| t.status == TaskStatus::Done);

        assert!(has_open);
        assert!(has_in_progress);
        assert!(has_done);

        // Check we have different types
        let has_feature = tasks.iter().any(|t| t.task_type == TaskType::Feature);
        let has_bug = tasks.iter().any(|t| t.task_type == TaskType::Bug);
        let has_chore = tasks.iter().any(|t| t.task_type == TaskType::Chore);

        assert!(has_feature);
        assert!(has_bug);
        assert!(has_chore);
    }

    #[test]
    fn test_mock_beads_not_empty() {
        let beads = mock_beads();
        assert!(!beads.is_empty());
    }

    #[test]
    fn test_mock_beads_have_variety() {
        let beads = mock_beads();

        // Check we have different statuses
        let has_pending = beads.iter().any(|b| b.status == BeadStatus::Pending);
        let has_running = beads.iter().any(|b| b.status == BeadStatus::Running);
        let has_completed = beads.iter().any(|b| b.status == BeadStatus::Completed);
        let has_failed = beads.iter().any(|b| b.status == BeadStatus::Failed);

        assert!(has_pending);
        assert!(has_running);
        assert!(has_completed);
        assert!(has_failed);

        // Check we have dependencies
        let has_deps = beads.iter().any(|b| !b.dependencies.is_empty());
        assert!(has_deps);

        // Check we have tags
        let has_tags = beads.iter().any(|b| !b.tags.is_empty());
        assert!(has_tags);
    }

    #[test]
    fn test_mock_graph_structure() {
        let graph = mock_graph();

        assert!(!graph.nodes.is_empty());
        assert!(!graph.edges.is_empty());

        // All edges should reference valid nodes
        let node_ids: Vec<_> = graph.nodes.iter().map(|n| n.id.as_str()).collect();
        for edge in &graph.edges {
            assert!(
                node_ids.contains(&edge.source.as_str()),
                "Edge source {} not found in nodes",
                edge.source
            );
            assert!(
                node_ids.contains(&edge.target.as_str()),
                "Edge target {} not found in nodes",
                edge.target
            );
        }
    }

    #[test]
    fn test_status_summary_from_beads() {
        let beads = mock_beads();
        let summary = StatusSummary::from_beads(&beads);

        // Total should match bead count
        assert_eq!(summary.total(), beads.len());

        // Active + terminal should equal total
        assert_eq!(summary.active() + summary.terminal(), summary.total());
    }

    #[test]
    fn test_task_summary_from_tasks() {
        let tasks = mock_tasks();
        let summary = TaskSummary::from_tasks(&tasks);

        // Total should match task count
        assert_eq!(summary.total(), tasks.len());
    }

    #[test]
    fn test_mock_data_ids_unique() {
        let tasks = mock_tasks();
        let task_ids: std::collections::HashSet<_> = tasks.iter().map(|t| &t.id).collect();
        assert_eq!(task_ids.len(), tasks.len(), "Task IDs must be unique");

        let beads = mock_beads();
        let bead_ids: std::collections::HashSet<_> = beads.iter().map(|b| &b.id).collect();
        assert_eq!(bead_ids.len(), beads.len(), "Bead IDs must be unique");
    }
}
