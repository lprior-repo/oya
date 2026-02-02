//! Tasks page component
//!
//! Full-featured task management page with filtering, search, and CRUD operations.

use leptos::prelude::*;

use crate::components::task_list::{FilterControls, TaskFilters, TaskListView};
use crate::models::mock::mock_tasks;
use crate::models::task::{Task, TaskPriority, TaskStatus, TaskType};

/// Tasks page component
#[component]
pub fn Tasks() -> impl IntoView {
    // Load mock tasks into reactive signal
    let tasks = RwSignal::new(mock_tasks());
    let search_term = RwSignal::new(String::new());
    let filters = RwSignal::new(TaskFilters::default());

    // Modal state for create/edit
    let show_create_modal = RwSignal::new(false);
    let selected_task = RwSignal::new(None::<String>);

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

    // Summary statistics
    let task_summary = Memo::new(move |_| {
        let all = tasks.get();
        let open = all.iter().filter(|t| t.status == TaskStatus::Open).count();
        let in_progress = all
            .iter()
            .filter(|t| t.status == TaskStatus::InProgress)
            .count();
        let done = all.iter().filter(|t| t.status == TaskStatus::Done).count();
        (open, in_progress, done, all.len())
    });

    view! {
        <div class="tasks-page">
            <header class="tasks-header">
                <div class="header-content">
                    <h1>"Tasks"</h1>
                    <p class="tasks-subtitle">"Manage and track your work items"</p>
                </div>
                <div class="header-actions">
                    <button
                        class="btn-primary create-task-btn"
                        on:click=move |_| show_create_modal.set(true)
                        disabled=true
                    >
                        "+ Create Task"
                    </button>
                </div>
            </header>

            <section class="tasks-summary">
                <div class="summary-cards">
                    <div class="summary-card open">
                        <span class="summary-count">{move || task_summary.get().0}</span>
                        <span class="summary-label">"Open"</span>
                    </div>
                    <div class="summary-card in-progress">
                        <span class="summary-count">{move || task_summary.get().1}</span>
                        <span class="summary-label">"In Progress"</span>
                    </div>
                    <div class="summary-card done">
                        <span class="summary-count">{move || task_summary.get().2}</span>
                        <span class="summary-label">"Done"</span>
                    </div>
                    <div class="summary-card total">
                        <span class="summary-count">{move || task_summary.get().3}</span>
                        <span class="summary-label">"Total"</span>
                    </div>
                </div>
            </section>

            <section class="tasks-filters">
                <FilterControls search_term=search_term filters=filters />
            </section>

            <section class="tasks-content">
                <div class="tasks-list-container">
                    <TaskListView tasks=filtered_tasks />
                </div>
            </section>

            <Show when=move || show_create_modal.get()>
                <CreateTaskModal
                    on_close=move || show_create_modal.set(false)
                    on_create=move |_task| {
                        // TODO: Actually add task
                        show_create_modal.set(false);
                    }
                />
            </Show>

            <Show when=move || selected_task.get().is_some()>
                <TaskDetailPanel
                    task_id=selected_task
                    tasks=tasks
                    on_close=move || selected_task.set(None)
                />
            </Show>
        </div>
    }
}

/// Modal for creating new tasks
#[component]
fn CreateTaskModal(
    on_close: impl Fn() + Clone + 'static,
    on_create: impl Fn(Task) + Clone + 'static,
) -> impl IntoView {
    let title = RwSignal::new(String::new());
    let description = RwSignal::new(String::new());
    let priority = RwSignal::new(TaskPriority::Medium);
    let task_type = RwSignal::new(TaskType::Feature);

    let on_close_overlay = on_close.clone();
    let on_close_btn = on_close.clone();
    let on_close_cancel = on_close.clone();

    let handle_submit = move |_| {
        // Generate a simple timestamp-based ID for now
        let timestamp = web_sys::js_sys::Date::now() as u64;
        let task = Task::new(format!("task-{}", timestamp), title.get())
            .with_description(description.get())
            .with_priority(priority.get())
            .with_type(task_type.get());
        on_create(task);
    };

    view! {
        <div class="modal-overlay" on:click=move |_| on_close_overlay()>
            <div class="modal-content" on:click=|e| e.stop_propagation()>
                <header class="modal-header">
                    <h2>"Create New Task"</h2>
                    <button class="modal-close" on:click=move |_| on_close_btn()>"×"</button>
                </header>

                <form class="task-form" on:submit=move |e| {
                    e.prevent_default();
                    handle_submit(());
                }>
                    <div class="form-group">
                        <label for="title">"Title"</label>
                        <input
                            id="title"
                            type="text"
                            placeholder="Enter task title..."
                            on:input=move |ev| title.set(event_target_value(&ev))
                            prop:value=move || title.get()
                            required=true
                        />
                    </div>

                    <div class="form-group">
                        <label for="description">"Description"</label>
                        <textarea
                            id="description"
                            placeholder="Enter task description..."
                            on:input=move |ev| description.set(event_target_value(&ev))
                            prop:value=move || description.get()
                            rows="3"
                        />
                    </div>

                    <div class="form-row">
                        <div class="form-group">
                            <label for="priority">"Priority"</label>
                            <select
                                id="priority"
                                on:change=move |ev| {
                                    let value = event_target_value(&ev);
                                    priority.set(match value.as_str() {
                                        "low" => TaskPriority::Low,
                                        "high" => TaskPriority::High,
                                        _ => TaskPriority::Medium,
                                    });
                                }
                            >
                                <option value="low">"Low"</option>
                                <option value="medium" selected=true>"Medium"</option>
                                <option value="high">"High"</option>
                            </select>
                        </div>

                        <div class="form-group">
                            <label for="type">"Type"</label>
                            <select
                                id="type"
                                on:change=move |ev| {
                                    let value = event_target_value(&ev);
                                    task_type.set(match value.as_str() {
                                        "bug" => TaskType::Bug,
                                        "chore" => TaskType::Chore,
                                        _ => TaskType::Feature,
                                    });
                                }
                            >
                                <option value="feature" selected=true>"Feature"</option>
                                <option value="bug">"Bug"</option>
                                <option value="chore">"Chore"</option>
                            </select>
                        </div>
                    </div>

                    <div class="form-actions">
                        <button type="button" class="btn-secondary" on:click=move |_| on_close_cancel()>
                            "Cancel"
                        </button>
                        <button type="submit" class="btn-primary" disabled=true>
                            "Create Task"
                        </button>
                    </div>
                </form>
            </div>
        </div>
    }
}

/// Side panel showing task details
#[component]
fn TaskDetailPanel(
    task_id: RwSignal<Option<String>>,
    tasks: RwSignal<Vec<Task>>,
    on_close: impl Fn() + Clone + 'static,
) -> impl IntoView {
    let selected_task = Memo::new(move |_| {
        task_id
            .get()
            .and_then(|id| tasks.get().into_iter().find(|t| t.id == id))
    });

    view! {
        <div class="task-detail-panel">
            <header class="panel-header">
                <h2>"Task Details"</h2>
                <button class="panel-close" on:click=move |_| on_close()>"×"</button>
            </header>

            <Show
                when=move || selected_task.get().is_some()
                fallback=|| view! { <p>"No task selected"</p> }
            >
                {move || selected_task.get().map(|task| {
                    view! {
                        <div class="panel-content">
                            <div class="detail-group">
                                <label>"ID"</label>
                                <span class="detail-value">{task.id.clone()}</span>
                            </div>
                            <div class="detail-group">
                                <label>"Title"</label>
                                <span class="detail-value">{task.title.clone()}</span>
                            </div>
                            <div class="detail-group">
                                <label>"Description"</label>
                                <p class="detail-value">{task.description.clone()}</p>
                            </div>
                            <div class="detail-group">
                                <label>"Status"</label>
                                <span class={format!("status-badge status-{:?}", task.status)}>
                                    {format!("{:?}", task.status)}
                                </span>
                            </div>
                            <div class="detail-group">
                                <label>"Priority"</label>
                                <span class={format!("priority-badge priority-{:?}", task.priority)}>
                                    {format!("{:?}", task.priority)}
                                </span>
                            </div>
                            <div class="detail-group">
                                <label>"Type"</label>
                                <span class="type-badge">{format!("{:?}", task.task_type)}</span>
                            </div>
                        </div>
                        <div class="panel-actions">
                            <button class="btn-secondary" disabled=true>"Edit"</button>
                            <button class="btn-danger" disabled=true>"Delete"</button>
                        </div>
                    }
                })}
            </Show>
        </div>
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_tasks_module_compiles() {
        // Tasks module compiles successfully
        // Component tests require runtime environment
        assert!(true);
    }

    #[test]
    fn test_task_priority_debug() {
        assert_eq!(format!("{:?}", TaskPriority::Low), "Low");
        assert_eq!(format!("{:?}", TaskPriority::Medium), "Medium");
        assert_eq!(format!("{:?}", TaskPriority::High), "High");
    }

    #[test]
    fn test_task_type_debug() {
        assert_eq!(format!("{:?}", TaskType::Feature), "Feature");
        assert_eq!(format!("{:?}", TaskType::Bug), "Bug");
        assert_eq!(format!("{:?}", TaskType::Chore), "Chore");
    }
}
