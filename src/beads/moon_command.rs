//! Quality Gates: Moon Command Generation
//!
//! Generates moon command from gate configuration.
//! Pure function: no I/O, stable command generation.

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
        Gate::Compiles => ("check", "Type check the project", "moon run :check"),
        Gate::TestsPass => ("test", "Run all tests", "moon run :test"),
        Gate::MoonCi => ("ci", "Run full CI pipeline", "moon run :ci"),
        Gate::HoldoutScenarios => {
            ("holdout", "Run hidden holdout scenario suite", "moon run :holdout")
        }
        Gate::CueArtifactGenerated => {
            ("cue", "Verify CUE artifact generated", "moon run :cue-check")
        }
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
        assert_eq!(cmd.description, "Type check the project");
        assert_eq!(cmd.command, "moon run :check");
    }

    #[test]
    fn generate_tests_pass_command() {
        let cmd = generate_moon_command(&Gate::TestsPass);
        assert_eq!(cmd.task_name, "test");
        assert_eq!(cmd.description, "Run all tests");
        assert_eq!(cmd.command, "moon run :test");
    }

    #[test]
    fn generate_moon_ci_command() {
        let cmd = generate_moon_command(&Gate::MoonCi);
        assert_eq!(cmd.task_name, "ci");
        assert_eq!(cmd.description, "Run full CI pipeline");
        assert_eq!(cmd.command, "moon run :ci");
    }

    #[test]
    fn generate_holdout_command() {
        let cmd = generate_moon_command(&Gate::HoldoutScenarios);
        assert_eq!(cmd.task_name, "holdout");
        assert_eq!(cmd.description, "Run hidden holdout scenario suite");
        assert_eq!(cmd.command, "moon run :holdout");
    }

    #[test]
    fn generate_cue_artifact_command() {
        let cmd = generate_moon_command(&Gate::CueArtifactGenerated);
        assert_eq!(cmd.task_name, "cue");
        assert_eq!(cmd.description, "Verify CUE artifact generated");
        assert_eq!(cmd.command, "moon run :cue-check");
    }

    #[test]
    fn generate_command_stable() {
        let cmd1 = generate_moon_command(&Gate::Compiles);
        let cmd2 = generate_moon_command(&Gate::Compiles);
        assert_eq!(cmd1, cmd2);
    }

    #[test]
    fn generate_command_all_gates() {
        let gates = vec![
            Gate::Compiles,
            Gate::TestsPass,
            Gate::MoonCi,
            Gate::HoldoutScenarios,
            Gate::CueArtifactGenerated,
        ];

        for gate in gates {
            let cmd = generate_moon_command(&gate);
            assert!(!cmd.task_name.is_empty());
            assert!(!cmd.description.is_empty());
            assert!(!cmd.command.is_empty());
        }
    }

    #[test]
    fn generate_command_cloneable() {
        let cmd = generate_moon_command(&Gate::Compiles);
        let cloned = cmd.clone();
        assert_eq!(cmd, cloned);
    }
}
