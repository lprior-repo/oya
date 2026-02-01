//! Router configuration for OYA UI
//!
//! This module defines the routes and navigation structure for the application.

use leptos::prelude::*;
use leptos_router::{
    StaticSegment,
    components::{Route, Router, Routes},
};

use crate::pages::{Beads, Dashboard, Home, NotFound, Tasks};

/// Route definitions as constants for type safety
pub mod routes {
    pub const HOME: &str = "/";
    pub const DASHBOARD: &str = "/dashboard";
    pub const TASKS: &str = "/tasks";
    pub const BEADS: &str = "/beads";
}

/// Main router component that wraps the application
#[component]
pub fn AppRouter() -> impl IntoView {
    view! {
        <Router>
            <Routes fallback=|| view! { <NotFound /> }>
                <Route path=StaticSegment("") view=Home />
                <Route path=StaticSegment("dashboard") view=Dashboard />
                <Route path=StaticSegment("tasks") view=Tasks />
                <Route path=StaticSegment("beads") view=Beads />
            </Routes>
        </Router>
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_route_constants() {
        assert_eq!(routes::HOME, "/");
        assert_eq!(routes::DASHBOARD, "/dashboard");
        assert_eq!(routes::TASKS, "/tasks");
        assert_eq!(routes::BEADS, "/beads");
    }

    #[test]
    fn test_route_constants_are_unique() {
        let routes_list = vec![
            routes::HOME,
            routes::DASHBOARD,
            routes::TASKS,
            routes::BEADS,
        ];

        // Check for duplicates
        for i in 0..routes_list.len() {
            for j in (i + 1)..routes_list.len() {
                assert_ne!(
                    routes_list[i], routes_list[j],
                    "Routes should be unique"
                );
            }
        }
    }

    #[test]
    fn test_route_paths_format() {
        // All routes except home should start with /
        assert!(routes::DASHBOARD.starts_with('/'));
        assert!(routes::TASKS.starts_with('/'));
        assert!(routes::BEADS.starts_with('/'));

        // Routes should not end with / (except home)
        assert!(!routes::DASHBOARD.ends_with('/'));
        assert!(!routes::TASKS.ends_with('/'));
        assert!(!routes::BEADS.ends_with('/'));
    }

    #[test]
    fn test_router_component_exists() {
        let _component = AppRouter;
    }

    #[test]
    fn test_all_page_components_exist() {
        // Verify all page components compile
        let _home = Home;
        let _dashboard = Dashboard;
        let _tasks = Tasks;
        let _beads = Beads;
        let _not_found = NotFound;
    }
}
