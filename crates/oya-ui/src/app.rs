//! Main application component
//!
//! This module provides the root App component that sets up routing
//! and the overall application structure.

use leptos::prelude::*;

use crate::router::AppRouter;

/// Main application component with router integration
///
/// This component serves as the root of the Leptos application,
/// providing the router and overall layout structure.
#[component]
pub fn App() -> impl IntoView {
    view! {
        <div class="app-container">
            <header class="app-header">
                <h1>"OYA"</h1>
                <nav class="app-nav">
                    <a href="/">"Home"</a>
                    <a href="/dashboard">"Dashboard"</a>
                    <a href="/tasks">"Tasks"</a>
                    <a href="/beads">"Beads"</a>
                </nav>
            </header>
            <main class="app-main">
                <AppRouter />
            </main>
            <footer class="app-footer">
                <p>"OYA Orchestration Framework - Leptos 0.7 CSR"</p>
            </footer>
        </div>
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_app_component_exists() {
        // Compile-time test - if this compiles, the component is valid
        let _component = App;
    }
}
