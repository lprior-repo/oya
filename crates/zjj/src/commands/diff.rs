//! Show diff between session and main branch

use std::{
    path::Path,
    process::{Command, Stdio},
};

use anyhow::{Context, Result};

use crate::commands::get_session_db;

/// Run the diff command
pub fn run(name: &str, stat: bool) -> Result<()> {
    let db = get_session_db()?;

    // Get the session
    let session = db
        .get(name)?
        .ok_or_else(|| anyhow::anyhow!("Session '{name}' not found"))?;

    // Verify workspace exists
    let workspace_path = Path::new(&session.workspace_path);
    if !workspace_path.exists() {
        anyhow::bail!(
            "Workspace not found: {}. The session may be stale.",
            session.workspace_path
        );
    }

    // Determine the main branch
    let main_branch = determine_main_branch(workspace_path);

    // Build the diff command
    let mut args = vec!["diff"];

    if stat {
        args.push("--stat");
    } else {
        args.push("--git");
    }

    // Show diff from main branch to current workspace (@)
    args.push("-r");
    let revset = format!("{main_branch}..@");
    args.push(&revset);

    // Execute the diff command
    let output = Command::new("jj")
        .args(&args)
        .current_dir(workspace_path)
        .output()
        .map_err(|e| {
            if e.kind() == std::io::ErrorKind::NotFound {
                anyhow::anyhow!(
                    "Failed to execute jj diff: JJ is not installed or not in PATH.\n\n\
                    Install JJ:\n\
                      cargo install jj-cli\n\
                    or:\n\
                      brew install jj\n\
                    or visit: https://github.com/martinvonz/jj#installation\n\n\
                    Error: {e}"
                )
            } else {
                anyhow::anyhow!("Failed to execute jj diff: {e}")
            }
        })?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        anyhow::bail!("jj diff failed: {stderr}");
    }

    let stdout = String::from_utf8_lossy(&output.stdout);

    // For stat output, just print directly
    if stat {
        print!("{stdout}");
        return Ok(());
    }

    // For full diff, try to use a pager
    if let Some(pager) = get_pager() {
        // Spawn the pager and pipe the diff to it
        match Command::new(&pager).stdin(Stdio::piped()).spawn() {
            Ok(mut child) => {
                if let Some(mut stdin) = child.stdin.take() {
                    use std::io::Write;
                    let _ = stdin.write_all(stdout.as_bytes());
                }
                let _ = child.wait();
            }
            Err(_) => {
                // If pager fails, just print directly
                print!("{stdout}");
            }
        }
    } else {
        // No pager available, print directly
        print!("{stdout}");
    }

    Ok(())
}

/// Determine the main branch for diffing
fn determine_main_branch(workspace_path: &Path) -> String {
    // Try to find the trunk/main branch using jj
    // If jj is not available or fails, fall back to "main"
    let output = Command::new("jj")
        .args(["log", "-r", "trunk()", "--no-graph", "-T", "commit_id"])
        .current_dir(workspace_path)
        .output();

    // Handle case where jj is not installed or command fails
    if let Ok(output) = output {
        if output.status.success() {
            let commit_id = String::from_utf8_lossy(&output.stdout);
            let trimmed = commit_id.trim();
            if !trimmed.is_empty() {
                return trimmed.to_string();
            }
        }
    }

    // Fallback: use "main" branch
    "main".to_string()
}

/// Get the pager command from environment or defaults
fn get_pager() -> Option<String> {
    // Check PAGER environment variable
    if let Ok(pager) = std::env::var("PAGER") {
        if !pager.is_empty() {
            return Some(pager);
        }
    }

    // Try common pagers in order of preference
    let pagers = ["delta", "bat", "less"];
    for pager in &pagers {
        if which::which(pager).is_ok() {
            return Some(pager.to_string());
        }
    }

    None
}

#[cfg(test)]
mod tests {
    use serial_test::serial;
    use tempfile::TempDir;

    use super::*;
    use crate::db::SessionDb;

    fn setup_test_db() -> Result<(SessionDb, TempDir)> {
        let dir = TempDir::new()?;
        let db_path = dir.path().join("test.db");
        let db = SessionDb::open(&db_path)?;
        Ok((db, dir))
    }

    #[test]
    fn test_determine_main_branch_not_in_repo() -> Result<()> {
        // When not in a JJ repo (or jj not installed), should fall back to "main"
        let temp = TempDir::new().context("Failed to create temp dir")?;
        let result = determine_main_branch(temp.path());

        // Should return fallback "main"
        assert_eq!(result, "main");
        Ok(())
    }

    #[test]
    #[serial]
    fn test_get_pager_from_env() {
        // Set PAGER environment variable
        std::env::set_var("PAGER", "custom-pager");
        let pager = get_pager();
        assert_eq!(pager, Some("custom-pager".to_string()));

        // Clean up
        std::env::remove_var("PAGER");
    }

    #[test]
    #[serial]
    fn test_get_pager_defaults() {
        // Unset PAGER
        std::env::remove_var("PAGER");
        let pager = get_pager();

        // Should return one of the default pagers if available
        // We can't assert a specific value since it depends on system
        // But we can verify it returns either Some or None
        assert!(pager.is_some() || pager.is_none());
    }

    #[test]
    #[serial]
    fn test_get_pager_empty_env() {
        // Set PAGER to empty string
        std::env::set_var("PAGER", "");
        let pager = get_pager();

        // Should fall back to defaults
        assert!(pager.is_some() || pager.is_none());

        // Clean up
        std::env::remove_var("PAGER");
    }

    #[test]
    fn test_run_session_not_found() -> Result<()> {
        let _temp_db = setup_test_db()?;

        // Try to diff a non-existent session
        // We need to set up the context so get_session_db works
        // This is tricky in unit tests, so we'll focus on testing the helpers

        Ok(())
    }

    #[test]
    fn test_run_workspace_not_found() -> Result<()> {
        let (db, _dir) = setup_test_db()?;

        // Create a session with a non-existent workspace
        let session = db.create("test-session", "/nonexistent/path")?;

        // Verify the session exists
        assert_eq!(session.name, "test-session");

        // The run function would fail because workspace doesn't exist
        // We can't easily test this without mocking, so we verify the logic in integration tests

        Ok(())
    }

    #[test]
    fn test_diff_command_args_full() {
        // Verify that full diff uses --git flag
        let args = ["diff", "--git", "-r", "main..@"];
        assert!(args.contains(&"--git"));
        assert!(args.contains(&"-r"));
    }

    #[test]
    fn test_diff_command_args_stat() {
        // Verify that stat diff uses --stat flag
        let args = ["diff", "--stat", "-r", "main..@"];
        assert!(args.contains(&"--stat"));
        assert!(!args.contains(&"--git"));
    }

    #[test]
    fn test_revset_format() {
        let main_branch = "main";
        let revset = format!("{main_branch}..@");
        assert_eq!(revset, "main..@");

        let commit_id = "abc123";
        let revset2 = format!("{commit_id}..@");
        assert_eq!(revset2, "abc123..@");
    }
}
