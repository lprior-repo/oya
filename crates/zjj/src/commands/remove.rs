//! Remove a session and its workspace

use std::{
    fs,
    io::{self, Write},
};

use anyhow::{Context, Result};

use crate::{
    cli::{is_inside_zellij, run_command},
    commands::get_session_db,
    json_output::RemoveOutput,
};

/// Options for the remove command
#[derive(Debug, Clone, Default)]
#[allow(clippy::struct_excessive_bools)]
pub struct RemoveOptions {
    /// Skip confirmation prompt and hooks
    pub force: bool,
    /// Squash-merge to main before removal
    pub merge: bool,
    /// Preserve branch after removal
    #[allow(dead_code)]
    pub keep_branch: bool,
    /// Output as JSON
    pub json: bool,
}

/// Run the remove command
#[allow(dead_code)]
pub fn run(name: &str) -> Result<()> {
    run_with_options(name, &RemoveOptions::default())
}

/// Run the remove command with options
pub fn run_with_options(name: &str, options: &RemoveOptions) -> Result<()> {
    let db = get_session_db()?;

    // Get the session
    let session = db
        .get(name)?
        .ok_or_else(|| anyhow::anyhow!("Session '{name}' not found"))?;

    // Confirm removal unless --force
    if !options.force && !confirm_removal(name)? {
        if options.json {
            let output = RemoveOutput {
                success: false,
                session_name: name.to_string(),
                message: "Removal cancelled".to_string(),
            };
            println!("{}", serde_json::to_string(&output)?);
        } else {
            println!("Removal cancelled");
        }
        return Ok(());
    }

    // Run pre_remove hooks unless --force
    if !options.force {
        run_pre_remove_hooks(name, &session.workspace_path);
    }

    // If --merge: squash-merge to main
    if options.merge {
        merge_to_main(name, &session.workspace_path)?;
    }

    // Close Zellij tab if inside Zellij
    if is_inside_zellij() {
        // Try to close the tab - ignore errors if tab doesn't exist
        let _ = close_zellij_tab(&session.zellij_tab);
    }

    // Remove JJ workspace (this removes the workspace from JJ's tracking)
    let workspace_result = run_command("jj", &["workspace", "forget", name]);
    if let Err(e) = workspace_result {
        tracing::warn!("Failed to forget JJ workspace: {e}");
    }

    // Remove the workspace directory
    if fs::metadata(&session.workspace_path).is_ok() {
        fs::remove_dir_all(&session.workspace_path)
            .context("Failed to remove workspace directory")?;
    }

    // Remove from database
    db.delete(name)?;

    if options.json {
        let output = RemoveOutput {
            success: true,
            session_name: name.to_string(),
            message: format!("Removed session '{name}'"),
        };
        println!("{}", serde_json::to_string(&output)?);
    } else {
        println!("Removed session '{name}'");
    }

    Ok(())
}

/// Prompt user for confirmation
fn confirm_removal(name: &str) -> Result<bool> {
    print!("Remove session '{name}' and its workspace? [y/N] ");
    io::stdout().flush()?;

    let mut response = String::new();
    io::stdin().read_line(&mut response)?;

    let response = response.trim().to_lowercase();
    Ok(response == "y" || response == "yes")
}

/// Run `pre_remove` hooks
const fn run_pre_remove_hooks(_name: &str, _workspace_path: &str) {
    // TODO: Implement hook execution when config system is ready
    // For now, this is a placeholder that always succeeds
}

/// Merge session to main branch
fn merge_to_main(_name: &str, _workspace_path: &str) -> Result<()> {
    // TODO: Implement merge functionality
    // This should:
    // 1. Switch to the session workspace
    // 2. Squash commits
    // 3. Merge to main
    anyhow::bail!("--merge is not yet implemented")
}

/// Close a Zellij tab by name
fn close_zellij_tab(tab_name: &str) -> Result<()> {
    // First, go to the tab
    run_command("zellij", &["action", "go-to-tab-name", tab_name])?;
    // Then close it
    run_command("zellij", &["action", "close-tab"])?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use tempfile::TempDir;

    use super::*;
    use crate::db::SessionDb;

    // Helper to create a test database with a session
    #[allow(dead_code)]
    fn setup_test_session(name: &str) -> Result<(SessionDb, TempDir, String)> {
        let dir = TempDir::new()?;
        let db_path = dir.path().join("test.db");
        let db = SessionDb::open(&db_path)?;

        let workspace_dir = dir.path().join("workspaces").join(name);
        fs::create_dir_all(&workspace_dir)?;
        let workspace_path = workspace_dir
            .to_str()
            .ok_or_else(|| anyhow::anyhow!("Invalid workspace path"))?
            .to_string();

        db.create(name, &workspace_path)?;

        Ok((db, dir, workspace_path))
    }

    #[test]
    fn test_remove_options_default() {
        let opts = RemoveOptions::default();
        assert!(!opts.force);
        assert!(!opts.merge);
        assert!(!opts.keep_branch);
    }

    #[test]
    fn test_session_not_found() -> Result<()> {
        let dir = TempDir::new().context("Failed to create temp dir")?;
        let db_path = dir.path().join("test.db");
        let _db = SessionDb::open(&db_path)?;

        // Mock get_session_db to return our test db
        // Note: This test will fail until we refactor to use dependency injection
        // For now, it demonstrates the test case we need
        Ok(())
    }

    #[test]
    fn test_confirm_removal_format() {
        // Test that confirmation prompt is correct
        // This is a unit test for the confirmation logic
        // Actual I/O testing would require mocking stdin/stdout
    }

    #[test]
    fn test_merge_to_main_not_implemented() {
        let result = merge_to_main("test", "/path");
        let is_not_impl = result
            .as_ref()
            .map(|()| false)
            .unwrap_or_else(|e| e.to_string().contains("not yet implemented"));
        assert!(is_not_impl);
    }
}
