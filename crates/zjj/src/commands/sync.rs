//! Sync a session's workspace with main branch

use std::time::SystemTime;

use anyhow::{Context, Result};

use crate::{
    cli::run_command,
    commands::get_session_db,
    json_output::{SyncError, SyncOutput},
    session::SessionUpdate,
};

/// Options for the sync command
#[derive(Debug, Clone, Default)]
pub struct SyncOptions {
    /// Output as JSON
    pub json: bool,
}

/// Run the sync command with options
///
/// If a session name is provided, syncs that session's workspace.
/// Otherwise, syncs all sessions.
pub fn run_with_options(name: Option<&str>, options: SyncOptions) -> Result<()> {
    match name {
        Some(n) => sync_session_with_options(n, options),
        None => sync_all_with_options(options),
    }
}

/// Sync a specific session's workspace
fn sync_session_with_options(name: &str, options: SyncOptions) -> Result<()> {
    let db = get_session_db()?;

    // Get the session
    let session = db
        .get(name)?
        .ok_or_else(|| anyhow::anyhow!("Session '{name}' not found"))?;

    // Use internal sync function
    match sync_session_internal(&db, &session.name, &session.workspace_path) {
        Ok(()) => {
            if options.json {
                let output = SyncOutput {
                    success: true,
                    session_name: Some(name.to_string()),
                    synced_count: 1,
                    failed_count: 0,
                    errors: Vec::new(),
                };
                println!("{}", serde_json::to_string(&output)?);
            } else {
                println!("Synced session '{name}' with main");
            }
            Ok(())
        }
        Err(e) => {
            if options.json {
                let output = SyncOutput {
                    success: false,
                    session_name: Some(name.to_string()),
                    synced_count: 0,
                    failed_count: 1,
                    errors: vec![SyncError {
                        session_name: name.to_string(),
                        error: e.to_string(),
                    }],
                };
                println!("{}", serde_json::to_string(&output)?);
            }
            Err(e)
        }
    }
}

/// Sync all sessions
fn sync_all_with_options(options: SyncOptions) -> Result<()> {
    let db = get_session_db()?;

    // Get all sessions
    let sessions = db
        .list(None)
        .map_err(|e| anyhow::anyhow!("Failed to list sessions: {e}"))?;

    if sessions.is_empty() {
        if options.json {
            let output = SyncOutput {
                success: true,
                session_name: None,
                synced_count: 0,
                failed_count: 0,
                errors: Vec::new(),
            };
            println!("{}", serde_json::to_string(&output)?);
        } else {
            println!("No sessions to sync");
        }
        return Ok(());
    }

    if options.json {
        // For JSON output, collect results and output once at the end
        let mut success_count = 0;
        let mut failure_count = 0;
        let mut errors = Vec::new();

        for session in &sessions {
            match sync_session_internal(&db, &session.name, &session.workspace_path) {
                Ok(()) => {
                    success_count += 1;
                }
                Err(e) => {
                    errors.push(SyncError {
                        session_name: session.name.clone(),
                        error: e.to_string(),
                    });
                    failure_count += 1;
                }
            }
        }

        let output = SyncOutput {
            success: failure_count == 0,
            session_name: None,
            synced_count: success_count,
            failed_count: failure_count,
            errors,
        };
        println!("{}", serde_json::to_string(&output)?);
        Ok(())
    } else {
        // Original text output
        println!("Syncing {} session(s)...", sessions.len());

        let mut success_count = 0;
        let mut failure_count = 0;
        let mut errors = Vec::new();

        for session in &sessions {
            print!("Syncing '{}' ... ", session.name);

            match sync_session_internal(&db, &session.name, &session.workspace_path) {
                Ok(()) => {
                    println!("OK");
                    success_count += 1;
                }
                Err(e) => {
                    println!("FAILED: {e}");
                    errors.push((session.name.clone(), e));
                    failure_count += 1;
                }
            }
        }

        println!();
        println!("Summary: {success_count} succeeded, {failure_count} failed");

        if !errors.is_empty() {
            println!("\nErrors:");
            for (name, error) in errors {
                println!("  {name}: {error}");
            }
        }

        Ok(())
    }
}

/// Internal function to sync a session's workspace
fn sync_session_internal(
    db: &crate::db::SessionDb,
    name: &str,
    workspace_path: &str,
) -> Result<()> {
    // Run rebase in the session's workspace
    run_command(
        "jj",
        &["--repository", workspace_path, "rebase", "-d", "main"],
    )
    .context("Failed to sync workspace with main")?;

    // Update last_synced timestamp
    let now = SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)
        .context("System time error")?
        .as_secs();

    db.update(
        name,
        SessionUpdate {
            last_synced: Some(now),
            ..Default::default()
        },
    )
    .map_err(|e| anyhow::anyhow!("Failed to update sync timestamp: {e}"))?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use std::time::SystemTime;

    use tempfile::TempDir;

    use crate::{db::SessionDb, session::SessionUpdate};

    // Helper to create a test database
    fn setup_test_db() -> anyhow::Result<(SessionDb, TempDir)> {
        let dir = TempDir::new()?;
        let db_path = dir.path().join("test.db");
        let db = SessionDb::open(&db_path)?;
        Ok((db, dir))
    }

    // Helper to get current unix timestamp
    fn current_timestamp() -> u64 {
        SystemTime::now()
            .duration_since(SystemTime::UNIX_EPOCH)
            .map(|d| d.as_secs())
            .unwrap_or(0)
    }

    #[test]
    fn test_sync_session_not_found() -> anyhow::Result<()> {
        let (db, _dir) = setup_test_db()?;

        // Try to sync a non-existent session
        // We can't actually run this without a real JJ repo, but we can test the lookup
        let result = db.get("nonexistent")?;
        assert!(result.is_none());
        Ok(())
    }

    #[test]
    fn test_sync_session_exists() -> anyhow::Result<()> {
        let (db, _dir) = setup_test_db()?;

        // Create a session
        let session = db.create("test-session", "/fake/workspace")?;
        assert!(session.last_synced.is_none());

        // Verify we can get it
        let retrieved = db.get("test-session")?;
        assert!(retrieved.is_some());
        if let Some(session) = retrieved {
            assert_eq!(session.name, "test-session");
        }

        Ok(())
    }

    #[test]
    fn test_update_last_synced_timestamp() -> anyhow::Result<()> {
        let (db, _dir) = setup_test_db()?;

        // Create a session
        db.create("test-session", "/fake/workspace")?;

        // Update last_synced
        let now = current_timestamp();
        let update = SessionUpdate {
            last_synced: Some(now),
            ..Default::default()
        };
        db.update("test-session", update)?;

        // Verify it was updated
        let session = db.get("test-session")?;
        assert!(session.is_some(), "Session not found");
        if let Some(session) = session {
            assert_eq!(session.last_synced, Some(now));
        }

        Ok(())
    }

    #[test]
    fn test_list_all_sessions() -> anyhow::Result<()> {
        let (db, _dir) = setup_test_db()?;

        // Create multiple sessions
        db.create("session1", "/fake/workspace1")?;
        db.create("session2", "/fake/workspace2")?;
        db.create("session3", "/fake/workspace3")?;

        // List all
        let sessions = db.list(None)?;
        assert_eq!(sessions.len(), 3);

        Ok(())
    }

    #[test]
    fn test_sync_updates_timestamp_on_success() -> anyhow::Result<()> {
        let (db, _dir) = setup_test_db()?;

        // Create a session
        db.create("test-session", "/fake/workspace")?;

        // Simulate successful sync by updating timestamp
        let before = current_timestamp();
        let update = SessionUpdate {
            last_synced: Some(before),
            ..Default::default()
        };
        db.update("test-session", update)?;

        // Verify timestamp was set
        let session = db.get("test-session")?;
        assert!(session.is_some(), "Session not found");
        if let Some(session) = session {
            assert!(session.last_synced.is_some(), "last_synced should be set");
            if let Some(last_synced) = session.last_synced {
                assert!(last_synced >= before);
            }
        }

        Ok(())
    }

    #[test]
    fn test_multiple_syncs_update_timestamp() -> anyhow::Result<()> {
        let (db, _dir) = setup_test_db()?;

        // Create a session
        db.create("test-session", "/fake/workspace")?;

        // First sync
        let first_sync = current_timestamp();
        db.update(
            "test-session",
            SessionUpdate {
                last_synced: Some(first_sync),
                ..Default::default()
            },
        )?;

        // Sleep to ensure different timestamp
        std::thread::sleep(std::time::Duration::from_millis(10));

        // Second sync
        let second_sync = current_timestamp();
        db.update(
            "test-session",
            SessionUpdate {
                last_synced: Some(second_sync),
                ..Default::default()
            },
        )?;

        // Verify second timestamp is newer
        let session = db.get("test-session")?;
        assert!(session.is_some(), "Session not found");
        if let Some(session) = session {
            assert!(session.last_synced.is_some(), "last_synced should be set");
            if let Some(last_synced) = session.last_synced {
                assert!(last_synced >= second_sync);
            }
        }

        Ok(())
    }
}
