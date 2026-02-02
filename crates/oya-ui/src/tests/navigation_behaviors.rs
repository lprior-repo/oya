//! Behavioral tests for navigation and routing

use crate::router::routes;

// ============================================================================
// ROUTE CONSTANT BEHAVIORS
// ============================================================================

#[test]
fn given_route_constants_when_checking_home_then_is_root() {
    assert_eq!(routes::HOME, "/");
}

#[test]
fn given_route_constants_when_checking_dashboard_then_starts_with_slash() {
    assert!(
        routes::DASHBOARD.starts_with('/'),
        "Dashboard route should start with /"
    );
}

#[test]
fn given_route_constants_when_checking_tasks_then_starts_with_slash() {
    assert!(
        routes::TASKS.starts_with('/'),
        "Tasks route should start with /"
    );
}

#[test]
fn given_route_constants_when_checking_beads_then_starts_with_slash() {
    assert!(
        routes::BEADS.starts_with('/'),
        "Beads route should start with /"
    );
}

#[test]
fn given_route_constants_when_checked_then_all_unique() {
    let routes = [routes::HOME, routes::DASHBOARD, routes::TASKS, routes::BEADS];

    let unique: std::collections::HashSet<_> = routes.iter().collect();
    assert_eq!(
        unique.len(),
        routes.len(),
        "All routes should be unique"
    );
}

#[test]
fn given_route_constants_except_home_when_checked_then_no_trailing_slash() {
    // Non-home routes should not have trailing slashes (clean URLs)
    assert!(
        !routes::DASHBOARD.ends_with('/'),
        "Dashboard route should not end with /"
    );
    assert!(
        !routes::TASKS.ends_with('/'),
        "Tasks route should not end with /"
    );
    assert!(
        !routes::BEADS.ends_with('/'),
        "Beads route should not end with /"
    );
}

#[test]
fn given_route_constants_when_checked_then_lowercase() {
    // Routes should be lowercase for consistency
    assert_eq!(
        routes::DASHBOARD,
        routes::DASHBOARD.to_lowercase(),
        "Dashboard route should be lowercase"
    );
    assert_eq!(
        routes::TASKS,
        routes::TASKS.to_lowercase(),
        "Tasks route should be lowercase"
    );
    assert_eq!(
        routes::BEADS,
        routes::BEADS.to_lowercase(),
        "Beads route should be lowercase"
    );
}

// ============================================================================
// NAVIGATION STRUCTURE BEHAVIORS
// ============================================================================

#[test]
fn given_main_navigation_routes_when_counted_then_matches_expected() {
    // Main navigation should have 4 routes: Home, Dashboard, Tasks, Beads
    let main_routes = [routes::HOME, routes::DASHBOARD, routes::TASKS, routes::BEADS];
    assert_eq!(main_routes.len(), 4, "Should have 4 main navigation routes");
}

#[test]
fn given_route_dashboard_when_checked_then_is_semantic() {
    assert_eq!(routes::DASHBOARD, "/dashboard", "Dashboard route should be semantic");
}

#[test]
fn given_route_tasks_when_checked_then_is_semantic() {
    assert_eq!(routes::TASKS, "/tasks", "Tasks route should be semantic");
}

#[test]
fn given_route_beads_when_checked_then_is_semantic() {
    assert_eq!(routes::BEADS, "/beads", "Beads route should be semantic");
}
