//! Quality Gates: Moon Command Generation
//!
//! Generates moon command from gate configuration.
//! Pure function: no I/O, deterministic command generation.

#![deny(clippy::unwrap_used)]
#![deny(clippy::expect_used)]
#![deny(clippy::panic)]
#![warn(clippy::pedantic)]
#![forbid(unsafe_code)]

use crate::types::Gate;
use thiserror::Error;

/// Error types for command generation
#[derive(Debug, Error)]
pub enum MoonCommandError {
    #[error("Unknown gate: {0}")]
    UnknownGate(String),
}

/// Moon command structure
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MoonCommand {
    pub task_name: String,
    pub description: String,
    pub command: String,
}

/// Generate moon command from gate
/// Pure function: maps gate to moon task
#[must_use]
pub fn generate_moon_command(gate: &Gate) -> MoonCommand {
    let (task_name, description, command) = match gate {
        Gate::Compiles => ("check", "Type check the project", "cargo check"),
        Gate::TestsPass => ("test", "Run all tests", "cargo test"),
        Gate::EdgeCases => ("test", "Test edge cases", "cargo test -- --test-threads=1"),
        Gate::NoVulnerabilities => ("test", "Check for vulnerabilities", "cargo audit"),
        Gate::ClippyClean => {
            ("clippy", "Run clippy lints", "cargo clippy --workspace --all-features -- -D warnings")
        }
        Gate::Security => ("test", "Security audit", "cargo audit"),
        Gate::MoonCi => ("ci", "Run full CI pipeline", "moon ci"),
        Gate::ZjjMergeQueue => ("test", "Verify merge queue", "zjj sync --status"),
    };

    MoonCommand {
        task_name: task_name.to_string(),
        description: description.to_string(),
        command: command.to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn generate_compiles_command() {
        let cmd = generate_moon_command(&Gate::Compiles);
        assert_eq!(cmd.task_name, "check");
        assert_eq!(cmd.command, "cargo check");
    }

    #[test]
    fn generate_clippy_command() {
        let cmd = generate_moon_command(&Gate::ClippyClean);
        assert_eq!(cmd.task_name, "clippy");
        assert!(cmd.command.contains("cargo clippy"));
    }

    #[test]
    fn generate_moon_ci_command() {
        let cmd = generate_moon_command(&Gate::MoonCi);
        assert_eq!(cmd.task_name, "ci");
        assert_eq!(cmd.command, "moon ci");
    }
}
