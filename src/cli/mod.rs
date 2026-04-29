#![deny(clippy::unwrap_used)]
#![deny(clippy::expect_used)]
#![deny(clippy::panic)]
#![forbid(unsafe_code)]

pub mod agent;
pub mod args;
pub mod commands;
pub mod doctor;
pub mod evidence;
pub mod explain;
pub mod init;
pub mod repo;
pub mod report;
pub mod restate;
pub mod run;
pub mod verify;
pub mod workspace;

pub use args::{Cli, Command};
pub use commands::dispatch_command;
