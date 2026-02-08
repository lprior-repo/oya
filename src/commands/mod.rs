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

pub mod doctor;
pub mod init;
pub mod logs;

// Re-export command types for convenience
pub use doctor::{doctor_command, DoctorArgs, DoctorError, DoctorOutput};
pub use init::{init_command, InitArgs, InitError, InitOutput};
pub use logs::{logs_command, LogsArgs, LogsError, LogsOutput};
