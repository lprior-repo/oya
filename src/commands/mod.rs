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
pub use doctor::{DoctorArgs, DoctorError, DoctorOutput, doctor_command};
pub use init::{InitArgs, InitError, InitOutput, init_command};
pub use logs::{LogsArgs, LogsError, LogsOutput, logs_command};
