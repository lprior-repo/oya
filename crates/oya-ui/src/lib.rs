//! Leptos 0.7 CSR frontend for OYA graph visualization
//!
//! This crate provides a client-side rendered web UI for visualizing
//! the OYA task dependency graph using Leptos 0.7 and WASM.
//!
//! ## Architecture
//! - Pure CSR (Client-Side Rendering) with Leptos 0.7
//! - WASM compilation target (wasm32-unknown-unknown)
//! - Canvas-based graph rendering
//! - WebSocket communication for real-time updates
//!
//! ## Module Structure
//! - `models`: Data structures for graph nodes and edges
//! - `components`: Leptos UI components
//! - `layout`: Graph layout algorithms
//! - `utils`: Helper functions and utilities

#![forbid(unsafe_code)]

pub mod components;
pub mod layout;
pub mod models;
pub mod utils;

use leptos::prelude::*;

/// Main application component
#[component]
pub fn App() -> impl IntoView {
    view! {
        <div class="app-container">
            <h1>"OYA Graph Visualization"</h1>
            <p>"Leptos 0.7 CSR - WASM powered"</p>
        </div>
    }
}

/// WASM entry point
#[cfg(target_arch = "wasm32")]
#[wasm_bindgen::prelude::wasm_bindgen]
pub fn hydrate() {
    console_error_panic_hook::set_once();
    leptos::mount::mount_to_body(App);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_module_structure() {
        // Verify that all modules are accessible
        let _models = models::GraphNode::default();
        let _layout_result = layout::force_directed::compute_positions(&[]);
        let _util_result = utils::canvas::clear_canvas();
        assert!(_layout_result.is_err()); // Empty graph should error
        assert!(_util_result.is_ok()); // Clear canvas should succeed
    }
}
