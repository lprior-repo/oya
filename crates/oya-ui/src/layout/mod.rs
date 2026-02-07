//! # Layout calculations and utilities
//!
//! Provides spring force physics, edge path calculations, and graph layout algorithms.

pub mod dag_edge;
pub mod spring_force;

// Re-exports
pub use dag_edge::{EdgeError, PathSegment, calculate_line_path};
pub use spring_force::{Force, Position, SpringForce, SpringForceError};
