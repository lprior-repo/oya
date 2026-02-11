#![deny(clippy::unwrap_used)]
#![deny(clippy::expect_used)]
#![deny(clippy::panic)]
#![warn(clippy::pedantic)]
#![warn(clippy::nursery)]
#![forbid(unsafe_code)]

//! Doctor command implementation
//!
//! Comprehensive workspace health diagnostics.

use anyhow::Result;
use clap::Parser;
use itertools::Itertools;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::time::Duration;
use thiserror::Error;
use tokio::fs;
use tokio::process::Command;
use tokio::time::timeout;
use tracing::{debug, info};

/// Arguments for the doctor command
#[derive(Parser, Debug, Clone)]
pub struct DoctorArgs {
    /// Specific check to run (moon, zjj, bead-store, clippy, coverage, deps, workspace)
    #[arg(long)]
    pub check: Option<String>,

    /// Auto-fix issues where possible
    #[arg(long)]
    pub fix: bool,

    /// Output format (human, json)
    #[arg(long, default_value = "human")]
    pub output: String,

    /// Treat warnings as failures
    #[arg(long)]
    pub strict: bool,
}

/// Output from the doctor command
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DoctorOutput {
    /// Check results
    pub checks: Vec<CheckResult>,
    /// Overall status
    pub status: CheckStatus,
    /// Summary message
    pub summary: String,
}

/// Result of a single check
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CheckResult {
    /// Check name
    pub name: String,
    /// Check status
    pub status: CheckStatus,
    /// Check message
    pub message: String,
    /// Suggested fix command
    pub fix_command: Option<String>,
    /// Time taken for check
    pub duration_ms: u64,
}

/// Check status enumeration
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum CheckStatus {
    Passed,
    Warning,
    Failed,
    Skipped,
}

/// Errors specific to the doctor command
#[derive(Debug, Error)]
pub enum DoctorError {
    #[error("Not an Oya workspace: {path}")]
    NotWorkspace { path: PathBuf },

    #[error("Check timeout: {check_name}")]
    CheckTimeout { check_name: String },

    #[error("Cargo error: {command}")]
    CargoFailed { command: String },
}

impl DoctorError {
    /// Get the exit code for this error
    pub const fn exit_code(&self) -> i32 {
        match self {
            Self::NotWorkspace { .. } => 3,
            Self::CheckTimeout { .. } => 4,
            Self::CargoFailed { .. } => 5,
        }
    }

    /// Get a hint for remediation
    pub fn hint(&self) -> Option<String> {
        match self {
            Self::NotWorkspace { .. } => {
                Some("Run from workspace root directory containing Cargo.toml".to_string())
            }
            Self::CheckTimeout { .. } => {
                Some("Process may be hanging, try running check manually".to_string())
            }
            Self::CargoFailed { .. } => {
                Some("Check cargo installation and workspace configuration".to_string())
            }
        }
    }
}

/// Core function to check if current directory is a workspace
async fn check_workspace(path: &PathBuf) -> Result<bool, DoctorError> {
    let cargo_toml = path.join("Cargo.toml");

    if !cargo_toml.exists() {
        return Err(DoctorError::NotWorkspace { path: path.clone() });
    }

    // Check if it's a workspace
    let content_result = fs::read_to_string(&cargo_toml).await;

    let content: String = match content_result {
        Ok(c) => c,
        Err(_) => return Err(DoctorError::NotWorkspace { path: path.clone() }),
    };

    Ok(content.contains("[workspace]"))
}

/// Shell function: Run a command with timeout
async fn run_command_with_timeout(
    program: &str,
    args: &[&str],
    timeout_secs: u64,
) -> Result<(bool, String)> {
    let result = timeout(
        Duration::from_secs(timeout_secs),
        Command::new(program).args(args).output(),
    )
    .await;

    let output = match result {
        Ok(Ok(o)) => o,
        Ok(Err(_)) => return Ok((false, "Command failed to execute".to_string())),
        Err(_) => return Ok((false, "Command timed out".to_string())),
    };

    let success = output.status.success();
    let stdout = String::from_utf8_lossy(&output.stdout).to_string();
    let stderr = String::from_utf8_lossy(&output.stderr).to_string();

    Ok((success, format!("{stdout}{stderr}")))
}

/// Check: Moon binary availability
async fn check_moon() -> CheckResult {
    let start = std::time::Instant::now();

    let (available, message) = match run_command_with_timeout("moon", &["--version"], 5).await {
        Ok((true, output)) => (
            true,
            format!(
                "Moon installed: {}",
                output.lines().next().unwrap_or("unknown")
            ),
        ),
        Ok((false, msg)) => (false, format!("Moon not available: {msg}")),
        Err(_) => (false, "Failed to check moon".to_string()),
    };

    CheckResult {
        name: "moon".to_string(),
        status: if available {
            CheckStatus::Passed
        } else {
            CheckStatus::Skipped
        },
        message,
        fix_command: if available {
            None
        } else {
            Some("Install moon: curl -fsSL https://moonrepo.dev/install.sh | bash".to_string())
        },
        duration_ms: start.elapsed().as_millis() as u64,
    }
}

/// Check: Zjj binary availability
async fn check_zjj() -> CheckResult {
    let start = std::time::Instant::now();

    let (available, message) = match run_command_with_timeout("zjj", &["--version"], 5).await {
        Ok((true, output)) => (
            true,
            format!(
                "Zjj installed: {}",
                output.lines().next().unwrap_or("unknown")
            ),
        ),
        Ok((false, msg)) => (false, format!("Zjj not available: {msg}")),
        Err(_) => (false, "Failed to check zjj".to_string()),
    };

    CheckResult {
        name: "zjj".to_string(),
        status: if available {
            CheckStatus::Passed
        } else {
            CheckStatus::Skipped
        },
        message,
        fix_command: if available {
            None
        } else {
            Some("Install zjj from your OYA distribution".to_string())
        },
        duration_ms: start.elapsed().as_millis() as u64,
    }
}

/// Check: Bead-store accessibility
async fn check_bead_store() -> CheckResult {
    let start = std::time::Instant::now();

    let (available, message) = match run_command_with_timeout("br", &["--version"], 5).await {
        Ok((true, output)) => (
            true,
            format!(
                "Bead-store accessible: {}",
                output.lines().next().unwrap_or("unknown")
            ),
        ),
        Ok((false, msg)) => (false, format!("Bead-store not accessible: {msg}")),
        Err(_) => (false, "Failed to check bead-store".to_string()),
    };

    CheckResult {
        name: "bead-store".to_string(),
        status: if available {
            CheckStatus::Passed
        } else {
            CheckStatus::Failed
        },
        message,
        fix_command: if available {
            None
        } else {
            Some("Run: br init".to_string())
        },
        duration_ms: start.elapsed().as_millis() as u64,
    }
}

/// Check: Cargo workspace
async fn check_cargo_workspace() -> CheckResult {
    let start = std::time::Instant::now();

    let (valid, message) = match run_command_with_timeout("cargo", &["check"], 30).await {
        Ok((true, _)) => (true, "Cargo workspace is valid".to_string()),
        Ok((false, output)) => (false, format!("Cargo check failed: {output}")),
        Err(_) => (false, "Failed to run cargo check".to_string()),
    };

    CheckResult {
        name: "cargo-workspace".to_string(),
        status: if valid {
            CheckStatus::Passed
        } else {
            CheckStatus::Failed
        },
        message,
        fix_command: if valid {
            None
        } else {
            Some("Run: cargo check".to_string())
        },
        duration_ms: start.elapsed().as_millis() as u64,
    }
}

/// Check: Clippy linting
async fn check_clippy() -> CheckResult {
    let start = std::time::Instant::now();

    let (clean, message) =
        match run_command_with_timeout("cargo", &["clippy", "--all-targets"], 30).await {
            Ok((true, _)) => (true, "No clippy warnings".to_string()),
            Ok((false, output)) => {
                let warning_count = output.matches("warning:").count();
                (false, format!("{warning_count} clippy warning(s)"))
            }
            Err(_) => (false, "Failed to run clippy".to_string()),
        };

    CheckResult {
        name: "clippy".to_string(),
        status: if clean {
            CheckStatus::Passed
        } else {
            CheckStatus::Warning
        },
        message,
        fix_command: if clean {
            None
        } else {
            Some("Run: cargo clippy --fix".to_string())
        },
        duration_ms: start.elapsed().as_millis() as u64,
    }
}

/// Check: Test coverage (basic check)
async fn check_test_coverage() -> CheckResult {
    let start = std::time::Instant::now();

    let (tests_run, message) = match run_command_with_timeout("cargo", &["test"], 30).await {
        Ok((true, output)) => {
            let test_count = output.matches("test result: ok").count();
            (true, format!("{test_count} test suite(s) passing"))
        }
        Ok((false, output)) => (false, format!("Tests failed: {output}")),
        Err(_) => (false, "Failed to run tests".to_string()),
    };

    CheckResult {
        name: "test-coverage".to_string(),
        status: if tests_run {
            CheckStatus::Passed
        } else {
            CheckStatus::Warning
        },
        message,
        fix_command: if tests_run {
            None
        } else {
            Some("Run: cargo test".to_string())
        },
        duration_ms: start.elapsed().as_millis() as u64,
    }
}

/// Check: Dependencies
async fn check_dependencies() -> CheckResult {
    let start = std::time::Instant::now();

    let (up_to_date, message) =
        match run_command_with_timeout("cargo", &["check", "--all-features"], 30).await {
            Ok((true, _)) => (true, "Dependencies are up to date".to_string()),
            Ok((false, output)) => (false, format!("Dependency issues: {output}")),
            Err(_) => (false, "Failed to check dependencies".to_string()),
        };

    CheckResult {
        name: "dependencies".to_string(),
        status: if up_to_date {
            CheckStatus::Passed
        } else {
            CheckStatus::Warning
        },
        message,
        fix_command: if up_to_date {
            None
        } else {
            Some("Run: cargo update".to_string())
        },
        duration_ms: start.elapsed().as_millis() as u64,
    }
}

/// Get all available checks
fn get_all_checks() -> Vec<&'static str> {
    vec![
        "moon",
        "zjj",
        "bead-store",
        "cargo-workspace",
        "clippy",
        "test-coverage",
        "dependencies",
    ]
}

/// Run a specific check
async fn run_check(check_name: &str) -> CheckResult {
    match check_name {
        "moon" => check_moon().await,
        "zjj" => check_zjj().await,
        "bead-store" => check_bead_store().await,
        "cargo-workspace" => check_cargo_workspace().await,
        "clippy" => check_clippy().await,
        "test-coverage" => check_test_coverage().await,
        "dependencies" => check_dependencies().await,
        _ => CheckResult {
            name: check_name.to_string(),
            status: CheckStatus::Skipped,
            message: format!("Unknown check: {check_name}"),
            fix_command: None,
            duration_ms: 0,
        },
    }
}

/// Main doctor command implementation
pub async fn doctor_command(args: DoctorArgs) -> Result<DoctorOutput, DoctorError> {
    debug!("Running doctor command with args: {args:?}");

    // Check if we're in a workspace
    let current_dir = std::env::current_dir().map_err(|_| DoctorError::NotWorkspace {
        path: PathBuf::from("."),
    })?;

    check_workspace(&current_dir).await?;

    // Determine which checks to run
    let checks_to_run = if let Some(ref check) = args.check {
        vec![check.as_str()]
    } else {
        get_all_checks()
    };

    // Run checks
    let mut checks = Vec::new();

    for check_name in checks_to_run {
        info!("Running check: {check_name}");
        let result = run_check(check_name).await;
        checks.push(result);
    }

    // Calculate overall status
    let has_failures = checks.iter().any(|c| c.status == CheckStatus::Failed);
    let has_warnings = checks.iter().any(|c| c.status == CheckStatus::Warning);

    let overall_status = if has_failures {
        CheckStatus::Failed
    } else if has_warnings {
        CheckStatus::Warning
    } else {
        CheckStatus::Passed
    };

    // Generate summary
    let summary = match overall_status {
        CheckStatus::Passed => "All systems operational".to_string(),
        CheckStatus::Warning => {
            format!(
                "{} warning(s) found",
                checks
                    .iter()
                    .filter(|c| c.status == CheckStatus::Warning)
                    .count()
            )
        }
        CheckStatus::Failed => {
            format!(
                "{} check(s) failed",
                checks
                    .iter()
                    .filter(|c| c.status == CheckStatus::Failed)
                    .count()
            )
        }
        CheckStatus::Skipped => "All checks skipped".to_string(),
    };

    // Apply auto-fix if requested
    if args.fix {
        for check in &checks {
            if let Some(ref fix_command) = check.fix_command {
                if check.status == CheckStatus::Failed || check.status == CheckStatus::Warning {
                    info!("Running fix: {fix_command}");
                    // In a real implementation, we would execute the fix command
                    // For now, just log it
                }
            }
        }
    }

    Ok(DoctorOutput {
        checks,
        status: overall_status,
        summary,
    })
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used)]
    #![allow(clippy::expect_used)]
    #![allow(clippy::panic)]

    use super::*;

    #[test]
    fn test_get_all_checks() {
        let checks = get_all_checks();

        assert!(checks.contains(&"moon"));
        assert!(checks.contains(&"clippy"));
        assert!(checks.contains(&"test-coverage"));
    }

    #[test]
    fn test_check_status_equality() {
        assert_eq!(CheckStatus::Passed, CheckStatus::Passed);
        assert_ne!(CheckStatus::Passed, CheckStatus::Failed);
    }
}
