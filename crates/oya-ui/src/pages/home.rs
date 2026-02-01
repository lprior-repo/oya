//! Home page component

use leptos::prelude::*;

/// Home page component
#[component]
pub fn Home() -> impl IntoView {
    view! {
        <div class="home-page">
            <h1>"OYA Orchestration Framework"</h1>
            <p>"Welcome to the OYA graph visualization interface"</p>
            <div class="feature-grid">
                <div class="feature-card">
                    <h2>"Task Management"</h2>
                    <p>"View and manage your tasks"</p>
                    <a href="/tasks">"Go to Tasks"</a>
                </div>
                <div class="feature-card">
                    <h2>"Beads Tracking"</h2>
                    <p>"Track issues with the Beads system"</p>
                    <a href="/beads">"Go to Beads"</a>
                </div>
                <div class="feature-card">
                    <h2>"Dashboard"</h2>
                    <p>"Visualize your project graph"</p>
                    <a href="/dashboard">"Go to Dashboard"</a>
                </div>
            </div>
        </div>
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_home_component_exists() {
        // This is a compile-time test - if the component exists and compiles, it passes
        let _component = Home;
    }
}
