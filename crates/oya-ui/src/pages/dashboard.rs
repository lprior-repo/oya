//! Dashboard page component with graph visualization

use leptos::prelude::*;

/// Dashboard page component
#[component]
pub fn Dashboard() -> impl IntoView {
    view! {
        <div class="dashboard-page">
            <h1>"Dashboard"</h1>
            <p>"Graph visualization coming soon"</p>
            <div class="dashboard-content">
                <div class="graph-container">
                    <p>"Canvas-based graph rendering will go here"</p>
                </div>
            </div>
        </div>
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_dashboard_component_exists() {
        let _component = Dashboard;
    }
}
