#![deny(clippy::unwrap_used)]
#![deny(clippy::expect_used)]
#![deny(clippy::panic)]
#![forbid(unsafe_code)]

pub mod args;
pub mod commands;
pub mod doctor;
pub mod init;
pub mod repo;
pub mod restate;

pub use args::{Cli, Command};
pub use commands::dispatch_command;
