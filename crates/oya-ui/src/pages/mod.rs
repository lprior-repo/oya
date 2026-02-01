//! Page components for the OYA UI
//!
//! This module contains the top-level page components for each route.

pub mod beads;
pub mod dashboard;
pub mod home;
pub mod not_found;
pub mod tasks;

pub use beads::Beads;
pub use dashboard::Dashboard;
pub use home::Home;
pub use not_found::NotFound;
pub use tasks::Tasks;
