#![deny(clippy::unwrap_used)]
#![deny(clippy::expect_used)]
#![deny(clippy::panic)]

//! # OYA
//!
//! Storm goddess of transformation. 100x developer throughput with AI agent swarms.
//!
//! This library re-exports all OYA workspace crates for convenience.

// Re-export all crates
pub use oya_core;

// Swarm module for 13-agent continuous assembly line
pub mod swarm;
