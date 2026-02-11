// Swarm Coordinator: Sled-based state management for 12-agent parallel execution
// Zero panic, zero unwrap, purely functional Rust

pub mod db;
pub mod models;
pub mod coordinator;

pub use db::SwarmDatabase;
pub use models::*;
pub use coordinator::SwarmCoordinator;
