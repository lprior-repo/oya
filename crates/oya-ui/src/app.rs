//! Main application component
//!
//! This module provides the root App component that sets up routing
//! and the overall application structure.

use leptos::prelude::*;

use crate::models::BeadEvent;
use crate::router::AppRouter;
use crate::state::init_backend;

/// Main application component with router integration
///
/// This component serves as the root of the Leptos application,
/// providing the router and overall layout structure.
/// Tauri backend connection is initialized on mount.
#[component]
pub fn App() -> impl IntoView {
    // Initialize Tauri backend connection
    let (backend_state, backend_event) = init_backend();

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
                <div class="backend-status">
                    {move || format!("Backend: {} | Events: {}",
                        backend_state.get(),
                        backend_event.get().as_ref().map(|e: &BeadEvent| e.event_type()).unwrap_or("none")
                    )}
                </div>
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
