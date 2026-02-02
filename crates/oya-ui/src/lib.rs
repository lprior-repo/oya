//! Leptos 0.7 CSR frontend for OYA graph visualization
//!
//! This crate provides a client-side rendered web UI for visualizing
//! the OYA task dependency graph using Leptos 0.7 and WASM.
//!
//! ## Architecture
//! - Pure CSR (Client-Side Rendering) with Leptos 0.7
//! - WASM compilation target (wasm32-unknown-unknown)
//! - Type-safe routing with leptos_router
//! - Canvas-based graph rendering
//! - WebSocket communication for real-time updates
//!
//! ## Module Structure
//! - `app`: Main application component
//! - `router`: Route definitions and navigation
//! - `pages`: Top-level page components
//! - `models`: Data structures for graph nodes and edges
//! - `components`: Reusable UI components
//! - `layout`: Graph layout algorithms
//! - `utils`: Helper functions and utilities
//! - `error`: Error types and handling

#![forbid(unsafe_code)]

pub mod app;
pub mod components;
pub mod error;
pub mod interaction;
pub mod layout;
pub mod models;
pub mod pages;
pub mod router;
pub mod state;
pub mod utils;

// Re-export main App component for convenience - Trunk will auto-mount it
pub use app::App;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_module_structure() {
        // Verify that all modules are accessible
        let _app = App;
        let _models = models::GraphNode::default();
        let _layout_result = layout::force_directed::compute_positions(&[]);
        let _util_result = utils::canvas::clear_canvas();
        assert!(_layout_result.is_err()); // Empty graph should error
        assert!(_util_result.is_ok()); // Clear canvas should succeed
    }

    #[test]
    fn test_error_types() {
        use error::LeptosError;
        let err = LeptosError::RouteNotFound("/test".to_string());
        assert!(err.to_string().contains("Route not found"));
    }

    #[test]
    fn test_page_modules() {
        // Verify page components are accessible
        let _home = pages::Home;
        let _dashboard = pages::Dashboard;
        let _tasks = pages::Tasks;
        let _beads = pages::Beads;
        let _not_found = pages::NotFound;
    }

    #[test]
    fn test_router_module() {
        // Verify router constants
        assert_eq!(router::routes::HOME, "/");
        assert_eq!(router::routes::DASHBOARD, "/dashboard");
        assert_eq!(router::routes::TASKS, "/tasks");
        assert_eq!(router::routes::BEADS, "/beads");
    }

    #[test]
    fn test_state_module() {
        // Verify state module is accessible
        use state::{ConnectionState, WebSocketError};
        let state = ConnectionState::Disconnected;
        assert_eq!(state, ConnectionState::Disconnected);

        let err = WebSocketError::InvalidUrl("test".to_string());
        assert!(err.to_string().contains("Invalid URL"));
    }
}
