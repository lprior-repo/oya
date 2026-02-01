//! Dashboard component for displaying task cards and status counts

use leptos::prelude::*;
use serde::{Deserialize, Serialize};

/// Task status enum
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum TaskStatus {
    Open,
    InProgress,
    Closed,
    Blocked,
}

impl TaskStatus {
    /// Returns the color associated with this status
    pub fn color(&self) -> &'static str {
        match self {
            TaskStatus::Open => "#6b7280",
            TaskStatus::InProgress => "#3b82f6",
            TaskStatus::Closed => "#10b981",
            TaskStatus::Blocked => "#ef4444",
        }
    }

    /// Returns the label for this status
    pub fn label(&self) -> &'static str {
        match self {
            TaskStatus::Open => "Open",
            TaskStatus::InProgress => "In Progress",
            TaskStatus::Closed => "Closed",
            TaskStatus::Blocked => "Blocked",
        }
    }
}

/// Task data structure
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Task {
    pub id: String,
    pub title: String,
    pub status: TaskStatus,
    pub priority: u8,
}

impl Task {
    /// Creates a new Task with validation
    pub fn new(
        id: impl Into<String>,
        title: impl Into<String>,
        status: TaskStatus,
        priority: u8,
    ) -> Result<Self, String> {
        let id = id.into();
        let title = title.into();

        if id.is_empty() {
            return Err("Task ID cannot be empty".to_string());
        }

        if title.is_empty() {
            return Err("Task title cannot be empty".to_string());
        }

        if priority > 4 {
            return Err(format!("Priority must be 0-4, got {}", priority));
        }

        Ok(Task {
            id,
            title,
            status,
            priority,
        })
    }
}

/// Status count data structure
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct StatusCounts {
    pub open: usize,
    pub in_progress: usize,
    pub closed: usize,
    pub blocked: usize,
}

impl StatusCounts {
    /// Creates a new StatusCounts from a list of tasks
    pub fn from_tasks(tasks: &[Task]) -> Self {
        let mut counts = StatusCounts {
            open: 0,
            in_progress: 0,
            closed: 0,
            blocked: 0,
        };

        for task in tasks {
            match task.status {
                TaskStatus::Open => counts.open += 1,
                TaskStatus::InProgress => counts.in_progress += 1,
                TaskStatus::Closed => counts.closed += 1,
                TaskStatus::Blocked => counts.blocked += 1,
            }
        }

        counts
    }

    /// Returns the total count of all tasks
    pub fn total(&self) -> usize {
        self.open + self.in_progress + self.closed + self.blocked
    }
}

/// TaskCard component for rendering individual task cards
#[component]
pub fn TaskCard(task: Task) -> impl IntoView {
    let status_color = task.status.color();
    let status_label = task.status.label();
    let task_id = task.id.clone();
    let task_title = task.title.clone();
    let priority = task.priority;

    view! {
        <div class="task-card" style=format!("border-left: 4px solid {}", status_color)>
            <div class="task-header">
                <span class="task-id">{task_id}</span>
                <span class="task-priority">{"P"}{priority}</span>
            </div>
            <h3 class="task-title">{task_title}</h3>
            <div class="task-status" style=format!("color: {}", status_color)>
                {status_label}
            </div>
        </div>
    }
}

/// StatusWidget component for rendering status count widgets
#[component]
pub fn StatusWidget<F>(label: &'static str, #[prop(into)] count: F, color: &'static str) -> impl IntoView
where
    F: Fn() -> usize + 'static,
{
    view! {
        <div class="status-widget" style=format!("border-top: 3px solid {}", color)>
            <div class="status-label">{label}</div>
            <div class="status-count">{move || count()}</div>
        </div>
    }
}

/// Dashboard component with task cards and status counts
#[component]
pub fn Dashboard() -> impl IntoView {
    // Create reactive signal for tasks
    let (tasks, set_tasks) = signal(Vec::<Task>::new());

    // Derive status counts from tasks
    let status_counts = Memo::new(move |_| StatusCounts::from_tasks(&tasks.get()));

    // Sample data for demonstration
    let sample_tasks = vec![
        Task::new(
            "src-3s0.4",
            "Create Dashboard component",
            TaskStatus::InProgress,
            1,
        )
        .unwrap_or_else(|_| Task {
            id: "default".to_string(),
            title: "Default".to_string(),
            status: TaskStatus::Open,
            priority: 1,
        }),
        Task::new("src-3s0.3", "Configure Trunk for WASM", TaskStatus::Open, 1).unwrap_or_else(
            |_| Task {
                id: "default".to_string(),
                title: "Default".to_string(),
                status: TaskStatus::Open,
                priority: 1,
            },
        ),
        Task::new(
            "src-3s0.2",
            "Initialize Leptos project",
            TaskStatus::Closed,
            1,
        )
        .unwrap_or_else(|_| Task {
            id: "default".to_string(),
            title: "Default".to_string(),
            status: TaskStatus::Open,
            priority: 1,
        }),
    ];

    set_tasks.set(sample_tasks);

    view! {
        <div class="dashboard">
            <h1 class="dashboard-title">"OYA Task Dashboard"</h1>

            <div class="status-widgets">
                <StatusWidget
                    label="Open"
                    count=move || status_counts.get().open
                    color="#6b7280"
                />
                <StatusWidget
                    label="In Progress"
                    count=move || status_counts.get().in_progress
                    color="#3b82f6"
                />
                <StatusWidget
                    label="Closed"
                    count=move || status_counts.get().closed
                    color="#10b981"
                />
                <StatusWidget
                    label="Blocked"
                    count=move || status_counts.get().blocked
                    color="#ef4444"
                />
            </div>

            <div class="task-cards">
                <For
                    each=move || tasks.get()
                    key=|task| task.id.clone()
                    children=move |task| {
                        view! {
                            <TaskCard task=task />
                        }
                    }
                />
            </div>

            <div class="graph-placeholder">
                <h2>"Graph Visualization"</h2>
                <p>"Graph rendering will be implemented here"</p>
            </div>
        </div>
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_task_creation_valid() {
        let task = Task::new("task-1", "Test Task", TaskStatus::Open, 1);
        assert!(task.is_ok());

        let task = task.expect("Task creation failed");
        assert_eq!(task.id, "task-1");
        assert_eq!(task.title, "Test Task");
        assert_eq!(task.status, TaskStatus::Open);
        assert_eq!(task.priority, 1);
    }

    #[test]
    fn test_task_creation_empty_id() {
        let task = Task::new("", "Test Task", TaskStatus::Open, 1);
        assert!(task.is_err());
        assert_eq!(task.expect_err("Expected error"), "Task ID cannot be empty");
    }

    #[test]
    fn test_task_creation_empty_title() {
        let task = Task::new("task-1", "", TaskStatus::Open, 1);
        assert!(task.is_err());
        assert_eq!(
            task.expect_err("Expected error"),
            "Task title cannot be empty"
        );
    }

    #[test]
    fn test_task_creation_invalid_priority() {
        let task = Task::new("task-1", "Test Task", TaskStatus::Open, 5);
        assert!(task.is_err());
        assert!(
            task.expect_err("Expected error")
                .contains("Priority must be 0-4")
        );
    }

    #[test]
    fn test_status_counts_empty() {
        let tasks: Vec<Task> = vec![];
        let counts = StatusCounts::from_tasks(&tasks);

        assert_eq!(counts.open, 0);
        assert_eq!(counts.in_progress, 0);
        assert_eq!(counts.closed, 0);
        assert_eq!(counts.blocked, 0);
        assert_eq!(counts.total(), 0);
    }

    #[test]
    fn test_status_counts_mixed() {
        let tasks = vec![
            Task::new("t1", "Task 1", TaskStatus::Open, 1).expect("Task creation failed"),
            Task::new("t2", "Task 2", TaskStatus::InProgress, 1).expect("Task creation failed"),
            Task::new("t3", "Task 3", TaskStatus::Closed, 1).expect("Task creation failed"),
            Task::new("t4", "Task 4", TaskStatus::Blocked, 2).expect("Task creation failed"),
            Task::new("t5", "Task 5", TaskStatus::Open, 1).expect("Task creation failed"),
        ];

        let counts = StatusCounts::from_tasks(&tasks);

        assert_eq!(counts.open, 2);
        assert_eq!(counts.in_progress, 1);
        assert_eq!(counts.closed, 1);
        assert_eq!(counts.blocked, 1);
        assert_eq!(counts.total(), 5);
    }

    #[test]
    fn test_task_status_colors() {
        assert_eq!(TaskStatus::Open.color(), "#6b7280");
        assert_eq!(TaskStatus::InProgress.color(), "#3b82f6");
        assert_eq!(TaskStatus::Closed.color(), "#10b981");
        assert_eq!(TaskStatus::Blocked.color(), "#ef4444");
    }

    #[test]
    fn test_task_status_labels() {
        assert_eq!(TaskStatus::Open.label(), "Open");
        assert_eq!(TaskStatus::InProgress.label(), "In Progress");
        assert_eq!(TaskStatus::Closed.label(), "Closed");
        assert_eq!(TaskStatus::Blocked.label(), "Blocked");
    }

    #[test]
    fn test_task_serialization() {
        let task =
            Task::new("task-1", "Test Task", TaskStatus::Open, 1).expect("Task creation failed");

        let json = serde_json::to_string(&task);
        assert!(json.is_ok());

        let json_str = json.expect("Serialization failed");
        let deserialized: Result<Task, _> = serde_json::from_str(&json_str);
        assert!(deserialized.is_ok());

        let deserialized_task = deserialized.expect("Deserialization failed");
        assert_eq!(deserialized_task.id, "task-1");
        assert_eq!(deserialized_task.title, "Test Task");
        assert_eq!(deserialized_task.status, TaskStatus::Open);
        assert_eq!(deserialized_task.priority, 1);
    }

    #[test]
    fn test_status_counts_serialization() {
        let counts = StatusCounts {
            open: 1,
            in_progress: 2,
            closed: 3,
            blocked: 4,
        };

        let json = serde_json::to_string(&counts);
        assert!(json.is_ok());

        let json_str = json.expect("Serialization failed");
        let deserialized: Result<StatusCounts, _> = serde_json::from_str(&json_str);
        assert!(deserialized.is_ok());

        let deserialized_counts = deserialized.expect("Deserialization failed");
        assert_eq!(deserialized_counts.open, 1);
        assert_eq!(deserialized_counts.in_progress, 2);
        assert_eq!(deserialized_counts.closed, 3);
        assert_eq!(deserialized_counts.blocked, 4);
    }

    #[test]
    fn test_components_compile() {
        // Verify that all components compile correctly
        let _ = Dashboard;
        let _ = TaskCard;
        let _ = StatusWidget;
    }
}
