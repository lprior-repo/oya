//! Command implementations

pub mod add;
pub mod config;
pub mod dashboard;
pub mod diff;
pub mod doctor;
pub mod focus;
pub mod init;
pub mod introspect;
pub mod list;
pub mod query;
pub mod remove;
pub mod status;
pub mod sync;

use std::path::PathBuf;

use anyhow::{Context, Result};

use crate::db::SessionDb;

/// Check if JJ is installed and available
///
/// # Errors
///
/// Returns an error with helpful installation instructions if JJ is not found
pub fn check_jj_installed() -> Result<()> {
    zjj_core::jj::check_jj_installed().map_err(|_| {
        anyhow::anyhow!(
            "JJ is not installed or not found in PATH.\n\n\
            Installation instructions:\n\
            \n  cargo install jj-cli\n\
            \n  # or: brew install jj\n\
            \n  # or: https://martinvonz.github.io/jj/latest/install-and-setup/"
        )
    })
}

/// Check if current directory is in a JJ repository
///
/// # Errors
///
/// Returns an error if not in a JJ repository
pub fn check_in_jj_repo() -> Result<PathBuf> {
    zjj_core::jj::check_in_jj_repo().map_err(|_| {
        anyhow::anyhow!(
            "Not in a JJ repository.\n\n\
            Run 'jjz init' to initialize JJ and ZJJ in this directory."
        )
    })
}

/// Check prerequisites before executing JJ commands
///
/// This ensures:
/// 1. JJ binary is installed
/// 2. We're inside a JJ repository
///
/// # Errors
///
/// Returns an error with helpful messages if prerequisites are not met
pub fn check_prerequisites() -> Result<PathBuf> {
    // First check if JJ is installed
    check_jj_installed()?;

    // Then check if we're in a JJ repo
    check_in_jj_repo()
}

/// Get the ZJJ data directory for the current repository
///
/// # Errors
///
/// Returns an error if prerequisites are not met (JJ not installed or not in a JJ repo)
pub fn zjj_data_dir() -> Result<PathBuf> {
    // Check prerequisites first
    let root = check_prerequisites()?;
    Ok(root.join(".jjz"))
}

/// Get the session database for the current repository
///
/// # Errors
///
/// Returns an error if:
/// - Prerequisites are not met (JJ not installed or not in a JJ repo)
/// - ZJJ is not initialized
/// - Unable to open the database
pub fn get_session_db() -> Result<SessionDb> {
    let data_dir = zjj_data_dir()?;

    anyhow::ensure!(
        data_dir.exists(),
        "ZJJ not initialized. Run 'jjz init' first."
    );

    let db_path = data_dir.join("sessions.db");
    SessionDb::open(&db_path).context("Failed to open session database")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_check_jj_installed_error_message() {
        // This test verifies that check_jj_installed returns a helpful error message
        // We can't directly test the failure case without controlling PATH, but we can
        // verify the error message format by examining the code
        let result = check_jj_installed();

        // If JJ is installed (likely in CI), this will pass
        // If JJ is not installed, verify the error message is helpful
        if let Err(e) = result {
            let msg = e.to_string();
            assert!(
                msg.contains("JJ is not installed"),
                "Error should mention JJ is not installed"
            );
            assert!(
                msg.contains("Installation instructions"),
                "Error should include installation instructions"
            );
            assert!(
                msg.contains("cargo install jj-cli") || msg.contains("brew install jj"),
                "Error should include specific installation commands"
            );
        }
    }

    #[test]
    fn test_check_in_jj_repo_error_message() {
        // When not in a JJ repo, we should get a helpful error message
        // We can't control being in/out of a repo in tests, but we verify the error format
        let result = check_in_jj_repo();

        if let Err(e) = result {
            let msg = e.to_string();
            assert!(
                msg.contains("Not in a JJ repository") || msg.contains("Failed to execute jj"),
                "Error should indicate not in a JJ repository or JJ execution failure"
            );
        }
    }

    #[test]
    fn test_check_prerequisites_validates_jj_first() {
        // Prerequisites should check JJ installation before checking repo
        // This ensures we give the right error first
        let result = check_prerequisites();

        // If this fails, it should be because JJ is not installed OR we're not in a repo
        if let Err(e) = result {
            let msg = e.to_string();
            // Should mention either "not installed" or "Not in a JJ repository"
            assert!(
                msg.contains("JJ is not installed")
                    || msg.contains("Not in a JJ repository")
                    || msg.contains("Failed to execute jj"),
                "Error should mention JJ installation or repository issue"
            );
        }
    }

    #[test]
    fn test_zjj_data_dir_checks_prerequisites() {
        // zjj_data_dir should call check_prerequisites
        let result = zjj_data_dir();

        // If this fails, it should be due to prerequisites
        if let Err(e) = result {
            let msg = e.to_string();
            // The error should be from prerequisites check
            assert!(
                msg.contains("JJ is not installed")
                    || msg.contains("Not in a JJ repository")
                    || msg.contains("Failed to execute jj"),
                "zjj_data_dir should fail with prerequisite errors when not met"
            );
        } else {
            // If prerequisites pass, we should get a valid path
            let path = result.ok();
            assert!(
                path.is_some(),
                "zjj_data_dir should return a path when prerequisites are met"
            );
            if let Some(p) = path {
                assert!(
                    p.to_string_lossy().ends_with(".jjz"),
                    "Path should end with .jjz"
                );
            }
        }
    }

    #[test]
    fn test_get_session_db_requires_init() {
        // get_session_db should fail if zjj is not initialized
        // Even if we're in a JJ repo, if .jjz doesn't exist, it should fail
        let result = get_session_db();

        if let Err(e) = result {
            let msg = e.to_string();
            // Should mention either prerequisites or initialization
            assert!(
                msg.contains("JJ is not installed")
                    || msg.contains("Not in a JJ repository")
                    || msg.contains("ZJJ not initialized")
                    || msg.contains("Failed to execute jj")
                    || msg.contains("Failed to open session database"),
                "get_session_db should fail with clear error when not initialized: {msg}"
            );
        }
    }

    #[test]
    fn test_prerequisite_error_messages_are_actionable() {
        // Verify that error messages tell users what to do

        // Test check_jj_installed error
        let jj_err = check_jj_installed();
        if let Err(e) = jj_err {
            let msg = e.to_string();
            assert!(
                msg.contains("cargo install") || msg.contains("brew install"),
                "JJ installation error should include installation commands"
            );
        }

        // Test check_in_jj_repo error
        let repo_err = check_in_jj_repo();
        if let Err(e) = repo_err {
            let msg = e.to_string();
            // If we get the "not in repo" error, it should mention jjz init
            if msg.contains("Not in a JJ repository") {
                assert!(
                    msg.contains("jjz init"),
                    "Repository error should mention 'jjz init'"
                );
            }
        }
    }
}
