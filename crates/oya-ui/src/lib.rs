//! # Oya UI
//!
//! UI components and layout calculations for OYA.

#![warn(missing_docs)]
#![deny(clippy::unwrap_used)]
#![deny(clippy::expect_used)]
#![deny(clippy::panic)]
#![warn(clippy::pedantic)]
#![warn(clippy::nursery)]
#![forbid(unsafe_code)]

// Layout modules
pub mod layout;

// Re-exports for convenience
pub use layout::spring_force::{Force, Position, SpringForce, SpringForceError};
