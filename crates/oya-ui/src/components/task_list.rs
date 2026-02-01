//! TaskList component with filtering and search functionality

use crate::models::task::{Task, TaskPriority, TaskStatus, TaskType};
use leptos::prelude::*;

/// Filter options for the task list
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct TaskFilters {
    pub status: Option<TaskStatus>,
    pub priority: Option<TaskPriority>,
    pub task_type: Option<TaskType>,
}

impl Default for TaskFilters {
    fn default() -> Self {
        Self {
            status: None,
            priority: None,
            task_type: None,
        }
    }
}

impl TaskFilters {
    /// Check if a task matches all active filters
    pub fn matches(&self, task: &Task) -> bool {
        let status_match = self.status.map(|s| s == task.status).unwrap_or(true);
        let priority_match = self.priority.map(|p| p == task.priority).unwrap_or(true);
        let type_match = self.task_type.map(|t| t == task.task_type).unwrap_or(true);

        status_match && priority_match && type_match
    }
}

/// Main TaskList component with filtering and search
#[component]
pub fn TaskList() -> impl IntoView {
    // Sample data - in real app this would come from backend
    let initial_tasks = vec![
        Task::new("task-1", "Implement authentication")
            .with_description("Add JWT-based auth system")
            .with_status(TaskStatus::InProgress)
            .with_priority(TaskPriority::High)
            .with_type(TaskType::Feature),
        Task::new("task-2", "Fix memory leak in parser")
            .with_description("Parser holds references too long")
            .with_status(TaskStatus::Open)
            .with_priority(TaskPriority::High)
            .with_type(TaskType::Bug),
        Task::new("task-3", "Update documentation")
            .with_description("Add API examples to docs")
            .with_status(TaskStatus::Open)
            .with_priority(TaskPriority::Low)
            .with_type(TaskType::Chore),
        Task::new("task-4", "Add dark mode")
            .with_description("Implement dark theme toggle")
            .with_status(TaskStatus::Done)
            .with_priority(TaskPriority::Medium)
            .with_type(TaskType::Feature),
        Task::new("task-5", "Refactor database queries")
            .with_description("Optimize N+1 query patterns")
            .with_status(TaskStatus::InProgress)
            .with_priority(TaskPriority::Medium)
            .with_type(TaskType::Chore),
    ];

    // Reactive state
    let tasks = RwSignal::new(initial_tasks);
    let search_term = RwSignal::new(String::new());
    let filters = RwSignal::new(TaskFilters::default());

    // Derived signal for filtered tasks
    let filtered_tasks = Memo::new(move |_| {
        let current_tasks = tasks.get();
        let current_search = search_term.get();
        let current_filters = filters.get();

        current_tasks
            .into_iter()
            .filter(|task| task.matches_search(&current_search))
            .filter(|task| current_filters.matches(task))
            .collect::<Vec<_>>()
    });

    view! {
        <div class="task-list-container">
            <h1>"Task List"</h1>

            <FilterControls
                search_term=search_term
                filters=filters
            />

            <TaskListView tasks=filtered_tasks />
        </div>
    }
}

/// Filter controls component
#[component]
pub fn FilterControls(
    search_term: RwSignal<String>,
    filters: RwSignal<TaskFilters>,
) -> impl IntoView {
    view! {
        <div class="filter-controls">
            <div class="search-box">
                <label for="search">"Search: "</label>
                <input
                    id="search"
                    type="text"
                    placeholder="Search tasks..."
                    on:input=move |ev| {
                        search_term.set(event_target_value(&ev));
                    }
                    prop:value=move || search_term.get()
                />
            </div>

            <div class="filter-selects">
                <div class="filter-group">
                    <label for="status-filter">"Status: "</label>
                    <select
                        id="status-filter"
                        on:change=move |ev| {
                            let value = event_target_value(&ev);
                            filters.update(|f| {
                                f.status = match value.as_str() {
                                    "open" => Some(TaskStatus::Open),
                                    "in_progress" => Some(TaskStatus::InProgress),
                                    "done" => Some(TaskStatus::Done),
                                    _ => None,
                                };
                            });
                        }
                    >
                        <option value="all">"All"</option>
                        <option value="open">"Open"</option>
                        <option value="in_progress">"In Progress"</option>
                        <option value="done">"Done"</option>
                    </select>
                </div>

                <div class="filter-group">
                    <label for="priority-filter">"Priority: "</label>
                    <select
                        id="priority-filter"
                        on:change=move |ev| {
                            let value = event_target_value(&ev);
                            filters.update(|f| {
                                f.priority = match value.as_str() {
                                    "low" => Some(TaskPriority::Low),
                                    "medium" => Some(TaskPriority::Medium),
                                    "high" => Some(TaskPriority::High),
                                    _ => None,
                                };
                            });
                        }
                    >
                        <option value="all">"All"</option>
                        <option value="low">"Low"</option>
                        <option value="medium">"Medium"</option>
                        <option value="high">"High"</option>
                    </select>
                </div>

                <div class="filter-group">
                    <label for="type-filter">"Type: "</label>
                    <select
                        id="type-filter"
                        on:change=move |ev| {
                            let value = event_target_value(&ev);
                            filters.update(|f| {
                                f.task_type = match value.as_str() {
                                    "feature" => Some(TaskType::Feature),
                                    "bug" => Some(TaskType::Bug),
                                    "chore" => Some(TaskType::Chore),
                                    _ => None,
                                };
                            });
                        }
                    >
                        <option value="all">"All"</option>
                        <option value="feature">"Feature"</option>
                        <option value="bug">"Bug"</option>
                        <option value="chore">"Chore"</option>
                    </select>
                </div>
            </div>
        </div>
    }
}

/// Task list view component
#[component]
pub fn TaskListView(tasks: Memo<Vec<Task>>) -> impl IntoView {
    view! {
        <div class="task-list">
            <div class="task-count">
                {move || format!("{} tasks", tasks.get().len())}
            </div>
            <ul class="task-items">
                {move || {
                    tasks.get()
                        .into_iter()
                        .map(|task| {
                            view! {
                                <TaskItem task=task />
                            }
                        })
                        .collect::<Vec<_>>()
                }}
            </ul>
        </div>
    }
}

/// Individual task item component
#[component]
pub fn TaskItem(task: Task) -> impl IntoView {
    let status_class = match task.status {
        TaskStatus::Open => "status-open",
        TaskStatus::InProgress => "status-in-progress",
        TaskStatus::Done => "status-done",
    };

    let priority_class = match task.priority {
        TaskPriority::Low => "priority-low",
        TaskPriority::Medium => "priority-medium",
        TaskPriority::High => "priority-high",
    };

    let type_badge = match task.task_type {
        TaskType::Feature => "Feature",
        TaskType::Bug => "Bug",
        TaskType::Chore => "Chore",
    };

    view! {
        <li class={format!("task-item {} {}", status_class, priority_class)}>
            <div class="task-header">
                <h3 class="task-title">{task.title.clone()}</h3>
                <span class="task-type-badge">{type_badge}</span>
            </div>
            <p class="task-description">{task.description.clone()}</p>
            <div class="task-metadata">
                <span class="task-id">{format!("ID: {}", task.id)}</span>
                <span class="task-status">{format!("{:?}", task.status)}</span>
                <span class="task-priority">{format!("{:?}", task.priority)}</span>
            </div>
        </li>
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_task_filters_default() {
        let filters = TaskFilters::default();
        assert_eq!(filters.status, None);
        assert_eq!(filters.priority, None);
        assert_eq!(filters.task_type, None);
    }

    #[test]
    fn test_task_filters_matches_all_when_no_filters() {
        let filters = TaskFilters::default();
        let task = Task::new("task-1", "Test")
            .with_status(TaskStatus::InProgress)
            .with_priority(TaskPriority::High)
            .with_type(TaskType::Bug);

        assert!(filters.matches(&task));
    }

    #[test]
    fn test_task_filters_matches_status() {
        let filters = TaskFilters {
            status: Some(TaskStatus::InProgress),
            priority: None,
            task_type: None,
        };

        let matching_task = Task::new("task-1", "Test").with_status(TaskStatus::InProgress);
        let non_matching_task = Task::new("task-2", "Test").with_status(TaskStatus::Open);

        assert!(filters.matches(&matching_task));
        assert!(!filters.matches(&non_matching_task));
    }

    #[test]
    fn test_task_filters_matches_priority() {
        let filters = TaskFilters {
            status: None,
            priority: Some(TaskPriority::High),
            task_type: None,
        };

        let matching_task = Task::new("task-1", "Test").with_priority(TaskPriority::High);
        let non_matching_task = Task::new("task-2", "Test").with_priority(TaskPriority::Low);

        assert!(filters.matches(&matching_task));
        assert!(!filters.matches(&non_matching_task));
    }

    #[test]
    fn test_task_filters_matches_type() {
        let filters = TaskFilters {
            status: None,
            priority: None,
            task_type: Some(TaskType::Bug),
        };

        let matching_task = Task::new("task-1", "Test").with_type(TaskType::Bug);
        let non_matching_task = Task::new("task-2", "Test").with_type(TaskType::Feature);

        assert!(filters.matches(&matching_task));
        assert!(!filters.matches(&non_matching_task));
    }

    #[test]
    fn test_task_filters_matches_combined() {
        let filters = TaskFilters {
            status: Some(TaskStatus::InProgress),
            priority: Some(TaskPriority::High),
            task_type: Some(TaskType::Bug),
        };

        let matching_task = Task::new("task-1", "Test")
            .with_status(TaskStatus::InProgress)
            .with_priority(TaskPriority::High)
            .with_type(TaskType::Bug);

        let non_matching_status = Task::new("task-2", "Test")
            .with_status(TaskStatus::Open)
            .with_priority(TaskPriority::High)
            .with_type(TaskType::Bug);

        let non_matching_priority = Task::new("task-3", "Test")
            .with_status(TaskStatus::InProgress)
            .with_priority(TaskPriority::Low)
            .with_type(TaskType::Bug);

        assert!(filters.matches(&matching_task));
        assert!(!filters.matches(&non_matching_status));
        assert!(!filters.matches(&non_matching_priority));
    }

    #[test]
    fn test_components_compile() {
        // This test verifies that all components compile correctly
        let _ = TaskList;
        let _ = FilterControls;
        let _ = TaskListView;
        let _ = TaskItem;
    }
}
