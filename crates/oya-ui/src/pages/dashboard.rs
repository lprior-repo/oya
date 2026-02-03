//! Dashboard page component with graph visualization placeholder
//!
//! The dashboard provides an overview of the system state including:
//! - Status summary widgets showing bead counts by status
//! - Task cards for quick visibility
//! - Graph visualization placeholder for DAG rendering

use leptos::prelude::*;

use crate::models::bead::{Bead, BeadStatus};
use crate::models::mock::{StatusSummary, mock_beads, mock_graph};
use crate::models::{Graph, GraphNode};

/// Dashboard page component
#[component]
pub fn Dashboard() -> impl IntoView {
    // Load mock data into reactive signals
    let beads = RwSignal::new(mock_beads());
    let graph = RwSignal::new(mock_graph());

    // Derive status summary from beads
    let status_summary = Memo::new(move |_| StatusSummary::from_beads(&beads.get()));

    view! {
        <div class="dashboard-page">
            <header class="dashboard-header">
                <h1>"OYA Dashboard"</h1>
                <p class="dashboard-subtitle">"Real-time orchestration monitoring"</p>
            </header>

            <section class="status-summary">
                <h2>"Status Overview"</h2>
                <div class="status-widgets">
                    <StatusWidget
                        label="Pending"
                        count=move || status_summary.get().pending
                        color="#9ca3af"
                        icon="clock"
                    />
                    <StatusWidget
                        label="Ready"
                        count=move || status_summary.get().ready
                        color="#3b82f6"
                        icon="play"
                    />
                    <StatusWidget
                        label="Running"
                        count=move || status_summary.get().running
                        color="#f59e0b"
                        icon="spinner"
                    />
                    <StatusWidget
                        label="Completed"
                        count=move || status_summary.get().completed
                        color="#10b981"
                        icon="check"
                    />
                    <StatusWidget
                        label="Failed"
                        count=move || status_summary.get().failed
                        color="#ef4444"
                        icon="x"
                    />
                    <StatusWidget
                        label="Cancelled"
                        count=move || status_summary.get().cancelled
                        color="#6b7280"
                        icon="ban"
                    />
                </div>
                <div class="status-totals">
                    <span class="total-active">
                        {move || format!("{} active", status_summary.get().active())}
                    </span>
                    <span class="total-separator">" | "</span>
                    <span class="total-count">
                        {move || format!("{} total", status_summary.get().total())}
                    </span>
                </div>
            </section>

            <section class="dashboard-content">
                <div class="graph-panel">
                    <h2>"Dependency Graph"</h2>
                    <GraphVisualization graph=graph />
                </div>

                <div class="activity-panel">
                    <h2>"Recent Activity"</h2>
                    <ActivityFeed beads=beads />
                </div>
            </section>

            <section class="quick-actions">
                <h2>"Quick Actions"</h2>
                <div class="action-buttons">
                    <button class="action-btn primary" disabled=true>
                        "Create Bead"
                    </button>
                    <button class="action-btn secondary" disabled=true>
                        "View All Beads"
                    </button>
                    <button class="action-btn secondary" disabled=true>
                        "Refresh"
                    </button>
                </div>
            </section>
        </div>
    }
}

/// Status widget showing count with icon
#[component]
fn StatusWidget(
    label: &'static str,
    count: impl Fn() -> usize + Send + Sync + 'static,
    color: &'static str,
    icon: &'static str,
) -> impl IntoView {
    let icon_char = match icon {
        "clock" => '\u{23F0}',   // alarm clock
        "play" => '\u{25B6}',    // play button
        "spinner" => '\u{21BB}', // clockwise arrow
        "check" => '\u{2713}',   // check mark
        "x" => '\u{2717}',       // X mark
        "ban" => '\u{20E0}',     // circle with slash
        _ => '\u{2022}',         // bullet
    };

    view! {
        <div class="status-widget" style=format!("border-top: 3px solid {}", color)>
            <div class="widget-icon" style=format!("color: {}", color)>
                {icon_char}
            </div>
            <div class="widget-count">{count}</div>
            <div class="widget-label">{label}</div>
        </div>
    }
}

/// Graph visualization placeholder component
#[component]
fn GraphVisualization(graph: RwSignal<Graph>) -> impl IntoView {
    let node_count = move || graph.get().nodes.len();
    let edge_count = move || graph.get().edges.len();

    view! {
        <div class="graph-container">
            <div class="graph-canvas-placeholder">
                <div class="graph-info">
                    <span class="node-count">{node_count}" nodes"</span>
                    <span class="edge-count">{edge_count}" edges"</span>
                </div>
                <div class="graph-mock">
                    <GraphMockVisualization graph=graph />
                </div>
                <p class="graph-note">
                    "Canvas-based interactive graph rendering coming soon"
                </p>
            </div>
            <div class="graph-controls">
                <button class="graph-btn" disabled=true title="Zoom In">"+"</button>
                <button class="graph-btn" disabled=true title="Zoom Out">"-"</button>
                <button class="graph-btn" disabled=true title="Fit to View">"Fit"</button>
                <button class="graph-btn" disabled=true title="Reset View">"Reset"</button>
            </div>
        </div>
    }
}

/// Mock graph visualization using CSS positioning
#[component]
fn GraphMockVisualization(graph: RwSignal<Graph>) -> impl IntoView {
    view! {
        <div class="graph-mock-container">
            <svg class="graph-edges" viewBox="0 0 800 300">
                {move || {
                    let g = graph.get();
                    g.edges.iter().filter_map(|edge| {
                        let source = g.nodes.iter().find(|n| n.id == edge.source);
                        let target = g.nodes.iter().find(|n| n.id == edge.target);
                        match (source, target) {
                            (Some(s), Some(t)) => Some(view! {
                                <line
                                    x1={s.x}
                                    y1={s.y}
                                    x2={t.x}
                                    y2={t.y}
                                    stroke="#4b5563"
                                    stroke-width="2"
                                    marker-end="url(#arrowhead)"
                                />
                            }),
                            _ => None,
                        }
                    }).collect::<Vec<_>>()
                }}
                <defs>
                    <marker
                        id="arrowhead"
                        markerWidth="10"
                        markerHeight="7"
                        refX="9"
                        refY="3.5"
                        orient="auto"
                    >
                        <polygon points="0 0, 10 3.5, 0 7" fill="#4b5563" />
                    </marker>
                </defs>
            </svg>
            <div class="graph-nodes">
                {move || {
                    graph.get().nodes.iter().map(|node| {
                        let node = node.clone();
                        view! {
                            <GraphNodeView node=node />
                        }
                    }).collect::<Vec<_>>()
                }}
            </div>
        </div>
    }
}

/// Individual graph node view
#[component]
fn GraphNodeView(node: GraphNode) -> impl IntoView {
    let color = node.color.clone().unwrap_or_else(|| "#6b7280".to_string());
    let style = format!(
        "left: {}px; top: {}px; background-color: {}; border-color: {}",
        node.x - 40.0,
        node.y - 15.0,
        color,
        color
    );

    view! {
        <div class="graph-node" style=style title={node.id.clone()}>
            <span class="node-label">{node.label.clone()}</span>
        </div>
    }
}

/// Activity feed showing recent bead changes
#[component]
fn ActivityFeed(beads: RwSignal<Vec<Bead>>) -> impl IntoView {
    // Get beads sorted by most relevant (running, then failed, then others)
    let sorted_beads = Memo::new(move |_| {
        let mut beads = beads.get();
        beads.sort_by_key(|b| match b.status {
            BeadStatus::Running => 0,
            BeadStatus::Failed => 1,
            BeadStatus::Ready => 2,
            BeadStatus::Pending => 3,
            BeadStatus::Completed => 4,
            BeadStatus::Cancelled => 5,
        });
        beads.into_iter().take(5).collect::<Vec<_>>()
    });

    view! {
        <div class="activity-feed">
            <ul class="activity-list">
                {move || {
                    sorted_beads.get().into_iter().map(|bead| {
                        view! {
                            <ActivityItem bead=bead />
                        }
                    }).collect::<Vec<_>>()
                }}
            </ul>
            <div class="activity-footer">
                <a href="/beads" class="view-all-link">"View all beads"</a>
            </div>
        </div>
    }
}

/// Individual activity item
#[component]
fn ActivityItem(bead: Bead) -> impl IntoView {
    let status_color = bead.status.color();
    let status_label = bead.status.label();

    view! {
        <li class="activity-item">
            <div class="activity-status" style=format!("background-color: {}", status_color)>
                {status_label}
            </div>
            <div class="activity-content">
                <span class="activity-id">{bead.id.clone()}</span>
                <span class="activity-title">{bead.title.clone()}</span>
            </div>
            <div class="activity-meta">
                {bead.tags.first().map(|t| view! {
                    <span class="activity-tag">{t.clone()}</span>
                })}
            </div>
        </li>
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn test_dashboard_module_compiles() {
        // Dashboard module compiles successfully
        // Component tests require runtime environment
    }
}
