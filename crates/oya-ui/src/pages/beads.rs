//! Beads page component for issue tracking and management
//!
//! The Beads page provides a comprehensive view of all beads (work items)
//! with filtering, status display, dependency visualization, and actions.

use leptos::prelude::*;

use crate::models::bead::{Bead, BeadFilters, BeadPriority, BeadStatus};
use crate::models::mock::{mock_beads, StatusSummary};

/// Beads page component
#[component]
pub fn Beads() -> impl IntoView {
    // Load mock beads into reactive signal
    let beads = RwSignal::new(mock_beads());
    let search_term = RwSignal::new(String::new());
    let filters = RwSignal::new(BeadFilters::default());

    // View mode: list or kanban
    let view_mode = RwSignal::new(ViewMode::List);

    // Selected bead for detail panel
    let selected_bead = RwSignal::new(None::<String>);

    // Derived signal for filtered beads
    let filtered_beads = Memo::new(move |_| {
        let current_beads = beads.get();
        let current_search = search_term.get();
        let current_filters = filters.get();

        current_beads
            .into_iter()
            .filter(|bead| bead.matches_search(&current_search))
            .filter(|bead| current_filters.matches(bead))
            .collect::<Vec<_>>()
    });

    // Status summary
    let status_summary = Memo::new(move |_| StatusSummary::from_beads(&beads.get()));

    // Collect unique tags from all beads
    let all_tags = Memo::new(move |_| {
        let mut tags: Vec<String> = beads
            .get()
            .iter()
            .flat_map(|b| b.tags.clone())
            .collect();
        tags.sort();
        tags.dedup();
        tags
    });

    view! {
        <div class="beads-page">
            <header class="beads-header">
                <div class="header-content">
                    <h1>"Beads"</h1>
                    <p class="beads-subtitle">"Track and manage work items"</p>
                </div>
                <div class="header-actions">
                    <div class="view-toggle">
                        <button
                            class={move || if view_mode.get() == ViewMode::List { "active" } else { "" }}
                            on:click=move |_| view_mode.set(ViewMode::List)
                        >
                            "List"
                        </button>
                        <button
                            class={move || if view_mode.get() == ViewMode::Kanban { "active" } else { "" }}
                            on:click=move |_| view_mode.set(ViewMode::Kanban)
                        >
                            "Kanban"
                        </button>
                    </div>
                    <button class="btn-primary" disabled=true>"+ Create Bead"</button>
                </div>
            </header>

            <section class="beads-summary">
                <StatusBar summary=status_summary />
            </section>

            <section class="beads-filters">
                <BeadFilterControls
                    search_term=search_term
                    filters=filters
                    tags=all_tags
                />
            </section>

            <section class="beads-content">
                <Show
                    when=move || view_mode.get() == ViewMode::List
                    fallback=move || view! {
                        <KanbanView beads=filtered_beads on_select=move |id| selected_bead.set(Some(id)) />
                    }
                >
                    <BeadListView beads=filtered_beads on_select=move |id| selected_bead.set(Some(id)) />
                </Show>
            </section>

            <Show when=move || selected_bead.get().is_some()>
                <BeadDetailPanel
                    bead_id=selected_bead
                    beads=beads
                    on_close=move || selected_bead.set(None)
                />
            </Show>
        </div>
    }
}

/// View mode for the beads page
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ViewMode {
    #[default]
    List,
    Kanban,
}

/// Status bar showing summary counts
#[component]
fn StatusBar(summary: Memo<StatusSummary>) -> impl IntoView {
    view! {
        <div class="status-bar">
            <div class="status-item pending">
                <span class="status-dot" style="background: #9ca3af"></span>
                <span class="status-label">"Pending"</span>
                <span class="status-count">{move || summary.get().pending}</span>
            </div>
            <div class="status-item ready">
                <span class="status-dot" style="background: #3b82f6"></span>
                <span class="status-label">"Ready"</span>
                <span class="status-count">{move || summary.get().ready}</span>
            </div>
            <div class="status-item running">
                <span class="status-dot" style="background: #f59e0b"></span>
                <span class="status-label">"Running"</span>
                <span class="status-count">{move || summary.get().running}</span>
            </div>
            <div class="status-item completed">
                <span class="status-dot" style="background: #10b981"></span>
                <span class="status-label">"Completed"</span>
                <span class="status-count">{move || summary.get().completed}</span>
            </div>
            <div class="status-item failed">
                <span class="status-dot" style="background: #ef4444"></span>
                <span class="status-label">"Failed"</span>
                <span class="status-count">{move || summary.get().failed}</span>
            </div>
            <div class="status-item cancelled">
                <span class="status-dot" style="background: #6b7280"></span>
                <span class="status-label">"Cancelled"</span>
                <span class="status-count">{move || summary.get().cancelled}</span>
            </div>
            <div class="status-total">
                <span class="total-label">"Total"</span>
                <span class="total-count">{move || summary.get().total()}</span>
            </div>
        </div>
    }
}

/// Filter controls for beads
#[component]
fn BeadFilterControls(
    search_term: RwSignal<String>,
    filters: RwSignal<BeadFilters>,
    tags: Memo<Vec<String>>,
) -> impl IntoView {
    view! {
        <div class="filter-controls">
            <div class="search-box">
                <label for="bead-search">"Search: "</label>
                <input
                    id="bead-search"
                    type="text"
                    placeholder="Search by ID, title, or tag..."
                    on:input=move |ev| search_term.set(event_target_value(&ev))
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
                                    "pending" => Some(BeadStatus::Pending),
                                    "ready" => Some(BeadStatus::Ready),
                                    "running" => Some(BeadStatus::Running),
                                    "completed" => Some(BeadStatus::Completed),
                                    "failed" => Some(BeadStatus::Failed),
                                    "cancelled" => Some(BeadStatus::Cancelled),
                                    _ => None,
                                };
                            });
                        }
                    >
                        <option value="all">"All"</option>
                        <option value="pending">"Pending"</option>
                        <option value="ready">"Ready"</option>
                        <option value="running">"Running"</option>
                        <option value="completed">"Completed"</option>
                        <option value="failed">"Failed"</option>
                        <option value="cancelled">"Cancelled"</option>
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
                                    "low" => Some(BeadPriority::Low),
                                    "medium" => Some(BeadPriority::Medium),
                                    "high" => Some(BeadPriority::High),
                                    "critical" => Some(BeadPriority::Critical),
                                    _ => None,
                                };
                            });
                        }
                    >
                        <option value="all">"All"</option>
                        <option value="low">"Low"</option>
                        <option value="medium">"Medium"</option>
                        <option value="high">"High"</option>
                        <option value="critical">"Critical"</option>
                    </select>
                </div>

                <div class="filter-group">
                    <label for="tag-filter">"Tag: "</label>
                    <select
                        id="tag-filter"
                        on:change=move |ev| {
                            let value = event_target_value(&ev);
                            filters.update(|f| {
                                f.tag = if value == "all" { None } else { Some(value) };
                            });
                        }
                    >
                        <option value="all">"All"</option>
                        {move || {
                            tags.get().into_iter().map(|tag| {
                                let tag_clone = tag.clone();
                                view! {
                                    <option value={tag}>{tag_clone}</option>
                                }
                            }).collect::<Vec<_>>()
                        }}
                    </select>
                </div>
            </div>
        </div>
    }
}

/// List view of beads
#[component]
fn BeadListView(
    beads: Memo<Vec<Bead>>,
    on_select: impl Fn(String) + Clone + Send + 'static,
) -> impl IntoView {
    view! {
        <div class="bead-list">
            <div class="bead-count">
                {move || format!("{} beads", beads.get().len())}
            </div>
            <ul class="bead-items">
                {move || {
                    let on_select = on_select.clone();
                    beads.get().into_iter().map(move |bead| {
                        let on_select = on_select.clone();
                        let bead_id = bead.id.clone();
                        view! {
                            <BeadItem
                                bead=bead
                                on_click=move || on_select(bead_id.clone())
                            />
                        }
                    }).collect::<Vec<_>>()
                }}
            </ul>
        </div>
    }
}

/// Individual bead item in list view
#[component]
fn BeadItem(bead: Bead, on_click: impl Fn() + Clone + 'static) -> impl IntoView {
    let status_color = bead.status.color();
    let priority_color = bead.priority.color();
    let is_blocked = bead.is_blocked();

    view! {
        <li
            class={format!("bead-item {}", if is_blocked { "blocked" } else { "" })}
            on:click=move |_| on_click()
        >
            <div class="bead-status" style=format!("background-color: {}", status_color)>
                {bead.status.label()}
            </div>
            <div class="bead-content">
                <div class="bead-header">
                    <span class="bead-id">{bead.id.clone()}</span>
                    <span class="bead-priority" style=format!("color: {}", priority_color)>
                        {bead.priority.label()}
                    </span>
                </div>
                <h3 class="bead-title">{bead.title.clone()}</h3>
                <p class="bead-description">{bead.description.clone()}</p>
                <div class="bead-meta">
                    <div class="bead-tags">
                        {bead.tags.iter().map(|tag| {
                            view! {
                                <span class="bead-tag">{tag.clone()}</span>
                            }
                        }).collect::<Vec<_>>()}
                    </div>
                    {
                        let deps_count = bead.dependencies.len();
                        let has_deps = deps_count > 0;
                        view! {
                            <Show when=move || has_deps>
                                <div class="bead-deps">
                                    <span class="deps-icon">"Deps: "</span>
                                    <span class="deps-count">{deps_count}</span>
                                </div>
                            </Show>
                        }
                    }
                </div>
            </div>
        </li>
    }
}

/// Kanban view of beads
#[component]
fn KanbanView(
    beads: Memo<Vec<Bead>>,
    on_select: impl Fn(String) + Clone + Send + 'static,
) -> impl IntoView {
    // Group beads by status
    let grouped = Memo::new(move |_| {
        let all_beads = beads.get();
        let pending: Vec<_> = all_beads
            .iter()
            .filter(|b| b.status == BeadStatus::Pending)
            .cloned()
            .collect();
        let ready: Vec<_> = all_beads
            .iter()
            .filter(|b| b.status == BeadStatus::Ready)
            .cloned()
            .collect();
        let running: Vec<_> = all_beads
            .iter()
            .filter(|b| b.status == BeadStatus::Running)
            .cloned()
            .collect();
        let completed: Vec<_> = all_beads
            .iter()
            .filter(|b| b.status == BeadStatus::Completed)
            .cloned()
            .collect();
        let failed: Vec<_> = all_beads
            .iter()
            .filter(|b| b.status == BeadStatus::Failed)
            .cloned()
            .collect();
        (pending, ready, running, completed, failed)
    });

    view! {
        <div class="kanban-board">
            <KanbanColumn
                title="Pending"
                color="#9ca3af"
                beads=Memo::new(move |_| grouped.get().0.clone())
                on_select=on_select.clone()
            />
            <KanbanColumn
                title="Ready"
                color="#3b82f6"
                beads=Memo::new(move |_| grouped.get().1.clone())
                on_select=on_select.clone()
            />
            <KanbanColumn
                title="Running"
                color="#f59e0b"
                beads=Memo::new(move |_| grouped.get().2.clone())
                on_select=on_select.clone()
            />
            <KanbanColumn
                title="Completed"
                color="#10b981"
                beads=Memo::new(move |_| grouped.get().3.clone())
                on_select=on_select.clone()
            />
            <KanbanColumn
                title="Failed"
                color="#ef4444"
                beads=Memo::new(move |_| grouped.get().4.clone())
                on_select=on_select.clone()
            />
        </div>
    }
}

/// Kanban column component
#[component]
fn KanbanColumn(
    title: &'static str,
    color: &'static str,
    beads: Memo<Vec<Bead>>,
    on_select: impl Fn(String) + Clone + Send + 'static,
) -> impl IntoView {
    view! {
        <div class="kanban-column">
            <div class="column-header" style=format!("border-bottom-color: {}", color)>
                <span class="column-title">{title}</span>
                <span class="column-count">{move || beads.get().len()}</span>
            </div>
            <div class="column-content">
                {move || {
                    let on_select = on_select.clone();
                    beads.get().into_iter().map(move |bead| {
                        let on_select = on_select.clone();
                        let bead_id = bead.id.clone();
                        view! {
                            <KanbanCard
                                bead=bead
                                on_click=move || on_select(bead_id.clone())
                            />
                        }
                    }).collect::<Vec<_>>()
                }}
            </div>
        </div>
    }
}

/// Kanban card component
#[component]
fn KanbanCard(bead: Bead, on_click: impl Fn() + Clone + 'static) -> impl IntoView {
    let priority_color = bead.priority.color();

    view! {
        <div class="kanban-card" on:click=move |_| on_click()>
            <div class="card-header">
                <span class="card-id">{bead.id.clone()}</span>
                <span class="card-priority" style=format!("background-color: {}", priority_color)>
                    {bead.priority.label()}
                </span>
            </div>
            <h4 class="card-title">{bead.title.clone()}</h4>
            <div class="card-tags">
                {bead.tags.iter().take(2).map(|tag| {
                    view! {
                        <span class="card-tag">{tag.clone()}</span>
                    }
                }).collect::<Vec<_>>()}
            </div>
        </div>
    }
}

/// Detail panel for selected bead
#[component]
fn BeadDetailPanel(
    bead_id: RwSignal<Option<String>>,
    beads: RwSignal<Vec<Bead>>,
    on_close: impl Fn() + Clone + 'static,
) -> impl IntoView {
    let selected_bead = Memo::new(move |_| {
        bead_id
            .get()
            .and_then(|id| beads.get().into_iter().find(|b| b.id == id))
    });

    view! {
        <div class="bead-detail-panel">
            <header class="panel-header">
                <h2>"Bead Details"</h2>
                <button class="panel-close" on:click=move |_| on_close()>"×"</button>
            </header>

            <Show
                when=move || selected_bead.get().is_some()
                fallback=|| view! { <p>"No bead selected"</p> }
            >
                {move || selected_bead.get().map(|bead| {
                    let status_color = bead.status.color();
                    let priority_color = bead.priority.color();

                    view! {
                        <div class="panel-content">
                            <div class="detail-header">
                                <span class="detail-id">{bead.id.clone()}</span>
                                <span
                                    class="detail-status"
                                    style=format!("background-color: {}", status_color)
                                >
                                    {bead.status.label()}
                                </span>
                            </div>

                            <h3 class="detail-title">{bead.title.clone()}</h3>

                            <div class="detail-group">
                                <label>"Description"</label>
                                <p class="detail-value">{bead.description.clone()}</p>
                            </div>

                            <div class="detail-row">
                                <div class="detail-group">
                                    <label>"Priority"</label>
                                    <span
                                        class="priority-badge"
                                        style=format!("color: {}", priority_color)
                                    >
                                        {bead.priority.label()}
                                    </span>
                                </div>
                                <div class="detail-group">
                                    <label>"Created"</label>
                                    <span class="detail-value">{bead.created_at.clone()}</span>
                                </div>
                            </div>

                            {
                                let deps = bead.dependencies.clone();
                                let has_deps = !deps.is_empty();
                                view! {
                                    <Show when=move || has_deps>
                                        <div class="detail-group">
                                            <label>"Dependencies"</label>
                                            <ul class="detail-deps">
                                                {deps.iter().map(|dep| {
                                                    view! {
                                                        <li class="dep-item">{dep.clone()}</li>
                                                    }
                                                }).collect::<Vec<_>>()}
                                            </ul>
                                        </div>
                                    </Show>
                                }
                            }

                            {
                                let tags = bead.tags.clone();
                                let has_tags = !tags.is_empty();
                                view! {
                                    <Show when=move || has_tags>
                                        <div class="detail-group">
                                            <label>"Tags"</label>
                                            <div class="detail-tags">
                                                {tags.iter().map(|tag| {
                                                    view! {
                                                        <span class="tag-badge">{tag.clone()}</span>
                                                    }
                                                }).collect::<Vec<_>>()}
                                            </div>
                                        </div>
                                    </Show>
                                }
                            }
                        </div>

                        <div class="panel-actions">
                            <button class="btn-secondary" disabled=true>"Edit"</button>
                            {
                                let is_terminal = bead.status.is_terminal();
                                let is_failed = bead.status == BeadStatus::Failed;
                                view! {
                                    <Show when=move || !is_terminal>
                                        <button class="btn-warning" disabled=true>"Cancel"</button>
                                    </Show>
                                    <Show when=move || is_failed>
                                        <button class="btn-primary" disabled=true>"Retry"</button>
                                    </Show>
                                }
                            }
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
    fn test_view_mode_default() {
        assert_eq!(ViewMode::default(), ViewMode::List);
    }

    #[test]
    fn test_view_mode_variants() {
        let list = ViewMode::List;
        let kanban = ViewMode::Kanban;
        assert_ne!(list, kanban);
    }

    #[test]
    fn test_beads_page_module_compiles() {
        // The Beads module compiles and exports its components
        assert!(true);
    }
}
