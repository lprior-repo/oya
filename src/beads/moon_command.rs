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
        Gate::Compiles => ("check", "Type check the project", "moon run :check"),
        Gate::AcceptanceTestsAreRed => {
            ("test", "Verify acceptance tests are red", "moon run :test")
        }
        Gate::TestsPass => ("test", "Run all tests", "moon run :test"),
        Gate::EdgeCases => ("test", "Test edge cases", "moon run :test -- --test-threads=1"),
        Gate::NoVulnerabilities => ("security", "Check for vulnerabilities", "moon run :security"),
        Gate::ClippyClean => ("clippy", "Run clippy lints", "moon run :clippy"),
        Gate::Security => ("security", "Security audit", "moon run :security"),
        Gate::MoonCi => ("ci", "Run full CI pipeline", "moon run :ci"),
        Gate::ZjjMergeQueue => ("test", "Verify merge queue", "zjj sync --status"),
        Gate::CueArtifactGenerated => ("cue", "Verify CUE artifact generated", "moon run :cue-check"),
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
    fn generate_edge_cases_command() {
        let cmd = generate_moon_command(&Gate::EdgeCases);
        assert_eq!(cmd.task_name, "test");
        assert_eq!(cmd.command, "moon run :test -- --test-threads=1");
    }

    #[test]
    fn generate_no_vulnerabilities_command() {
        let cmd = generate_moon_command(&Gate::NoVulnerabilities);
        assert_eq!(cmd.task_name, "security");
        assert_eq!(cmd.command, "moon run :security");
    }

    #[test]
    fn generate_clippy_command() {
        let cmd = generate_moon_command(&Gate::ClippyClean);
        assert_eq!(cmd.task_name, "clippy");
        assert_eq!(cmd.command, "moon run :clippy");
    }

    #[test]
    fn generate_security_command() {
        let cmd = generate_moon_command(&Gate::Security);
        assert_eq!(cmd.task_name, "security");
        assert_eq!(cmd.command, "moon run :security");
    }

    #[test]
    fn generate_moon_ci_command() {
        let cmd = generate_moon_command(&Gate::MoonCi);
        assert_eq!(cmd.task_name, "ci");
        assert_eq!(cmd.description, "Run full CI pipeline");
        assert_eq!(cmd.command, "moon run :ci");
    }

    #[test]
    fn generate_zjj_merge_queue_command() {
        let cmd = generate_moon_command(&Gate::ZjjMergeQueue);
        assert_eq!(cmd.task_name, "test");
        assert_eq!(cmd.command, "zjj sync --status");
    }

    #[test]
    fn generate_cue_artifact_generated_command() {
        let cmd = generate_moon_command(&Gate::CueArtifactGenerated);
        assert_eq!(cmd.task_name, "cue");
        assert_eq!(cmd.description, "Verify CUE artifact generated");
        assert_eq!(cmd.command, "moon run :cue-check");
    }

    #[test]
    fn generate_command_deterministic() {
        let cmd1 = generate_moon_command(&Gate::Compiles);
        let cmd2 = generate_moon_command(&Gate::Compiles);
        assert_eq!(cmd1, cmd2);
    }

    #[test]
    fn generate_command_all_gates() {
        let gates = vec![
            Gate::Compiles,
            Gate::TestsPass,
            Gate::EdgeCases,
            Gate::NoVulnerabilities,
            Gate::ClippyClean,
            Gate::Security,
            Gate::MoonCi,
            Gate::ZjjMergeQueue,
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
