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
pub use oya_factory;
pub use oya_intent;
