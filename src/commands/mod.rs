#![deny(clippy::unwrap_used)]
#![deny(clippy::expect_used)]
#![deny(clippy::panic)]
#![warn(clippy::pedantic)]
#![warn(clippy::nursery)]
#![forbid(unsafe_code)]

//! Command implementations for OYA CLI
//!
//! Each command is implemented as a separate module following functional Rust principles:
//! - Pure core functions for business logic
//! - Imperative shell for I/O and async operations
//! - Railway-oriented error handling with thiserror/anyhow
//! - Zero unwrap/expect/panic throughout
//!
//! # BDD-Style Tests
//!
//! CLI validation tests are in `tests/cli_validation.rs` using Given-When-Then format.

pub mod doctor;
pub mod init;
pub mod install;
pub mod logs;
pub mod serve;
pub mod storm;

// Re-export command types for convenience
pub use doctor::{CheckStatus, DoctorArgs, doctor_command};
pub use init::{InitArgs, init_command};
pub use install::install_command;
pub use logs::{LogsArgs, logs_command};
pub use serve::serve_command;
pub use storm::{StormArgs, storm_command};
