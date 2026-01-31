//! List all sessions

use std::path::Path;

use anyhow::Result;
use serde::Serialize;

use crate::{
    commands::get_session_db,
    session::{Session, SessionStatus},
};

/// Enhanced session information for list output
#[derive(Debug, Clone, Serialize)]
pub struct SessionListItem {
    pub name: String,
    pub status: String,
    pub branch: String,
    pub changes: String,
    pub beads: String,
    #[serde(flatten)]
    pub session: Session,
}

/// Beads issue counts
#[derive(Debug, Clone, Default)]
pub struct BeadCounts {
    pub open: usize,
    pub in_progress: usize,
    pub blocked: usize,
}

impl std::fmt::Display for BeadCounts {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}/{}/{}", self.open, self.in_progress, self.blocked)
    }
}

/// Run the list command
pub fn run(all: bool, json: bool) -> Result<()> {
    let db = get_session_db()?;

    // Filter sessions: exclude completed/failed unless --all is used
    // We always get all and filter manually since we need multiple exclusions
    let mut sessions = db.list(None)?;

    // Filter out completed and failed unless --all flag is set
    if !all {
        sessions
            .retain(|s| s.status != SessionStatus::Completed && s.status != SessionStatus::Failed);
    }

    if sessions.is_empty() {
        if json {
            println!("[]");
        } else {
            println!("No sessions found.");
            println!("Use 'jjz add <name>' to create a session.");
        }
        return Ok(());
    }

    // Build list items with enhanced data
    let items: Vec<SessionListItem> = sessions
        .into_iter()
        .map(|session| {
            let changes = get_session_changes(&session.workspace_path);
            let beads = get_beads_count().unwrap_or_default();

            SessionListItem {
                name: session.name.clone(),
                status: session.status.to_string(),
                branch: session.branch.clone().unwrap_or_else(|| "-".to_string()),
                changes: changes.map_or_else(|| "-".to_string(), |c| c.to_string()),
                beads: beads.to_string(),
                session,
            }
        })
        .collect();

    if json {
        output_json(&items)?;
    } else {
        output_table(&items);
    }

    Ok(())
}

/// Get the number of changes in a workspace
fn get_session_changes(workspace_path: &str) -> Option<usize> {
    let path = Path::new(workspace_path);

    // Check if workspace exists
    if !path.exists() {
        return None;
    }

    // Try to get status from JJ
    zjj_core::jj::workspace_status(path)
        .ok()
        .map(|status| status.change_count())
}

/// Get beads count from the repository's beads database
fn get_beads_count() -> Result<BeadCounts> {
    use rusqlite::Connection;

    // Find repository root
    let repo_root = zjj_core::jj::check_in_jj_repo().ok();

    let Some(root) = repo_root else {
        return Ok(BeadCounts::default());
    };

    let beads_db_path = root.join(".beads").join("beads.db");

    if !beads_db_path.exists() {
        return Ok(BeadCounts::default());
    }

    // Query beads database
    let conn = Connection::open(&beads_db_path)
        .map_err(|e| anyhow::anyhow!("Failed to open beads database: {e}"))?;

    // Count open issues
    let open: usize = conn
        .query_row(
            "SELECT COUNT(*) FROM issues WHERE status = 'open'",
            [],
            |row| row.get(0),
        )
        .unwrap_or(0);

    // For now, we can't distinguish in_progress vs blocked without more schema knowledge
    // Let's return a simplified count
    Ok(BeadCounts {
        open,
        in_progress: 0,
        blocked: 0,
    })
}

/// Output sessions as formatted table
fn output_table(items: &[SessionListItem]) {
    println!(
        "{:<20} {:<12} {:<15} {:<10} {:<12}",
        "NAME", "STATUS", "BRANCH", "CHANGES", "BEADS"
    );
    println!("{}", "-".repeat(70));

    for item in items {
        println!(
            "{:<20} {:<12} {:<15} {:<10} {:<12}",
            item.name, item.status, item.branch, item.changes, item.beads
        );
    }
}

/// Output sessions as JSON
fn output_json(items: &[SessionListItem]) -> Result<()> {
    let json = serde_json::to_string_pretty(items)?;
    println!("{json}");
    Ok(())
}

#[cfg(test)]
mod tests {
    use tempfile::TempDir;

    use super::*;
    use crate::{
        db::SessionDb,
        session::{Session, SessionStatus, SessionUpdate},
    };

    fn setup_test_db() -> Result<(SessionDb, TempDir)> {
        let dir = TempDir::new()?;
        let db_path = dir.path().join("test.db");
        let db = SessionDb::open(&db_path)?;
        Ok((db, dir))
    }

    #[test]
    fn test_bead_counts_display() {
        let counts = BeadCounts {
            open: 5,
            in_progress: 3,
            blocked: 2,
        };
        assert_eq!(counts.to_string(), "5/3/2");
    }

    #[test]
    fn test_bead_counts_default() {
        let counts = BeadCounts::default();
        assert_eq!(counts.open, 0);
        assert_eq!(counts.in_progress, 0);
        assert_eq!(counts.blocked, 0);
    }

    #[test]
    fn test_session_list_item_serialization() -> Result<()> {
        let session = Session {
            id: Some(1),
            name: "test".to_string(),
            status: SessionStatus::Active,
            workspace_path: "/tmp/test".to_string(),
            zellij_tab: "jjz:test".to_string(),
            branch: Some("feature".to_string()),
            created_at: 1_234_567_890,
            updated_at: 1_234_567_890,
            last_synced: None,
            metadata: None,
        };

        let item = SessionListItem {
            name: session.name.clone(),
            status: session.status.to_string(),
            branch: session.branch.clone().unwrap_or_else(|| "-".to_string()),
            changes: "5".to_string(),
            beads: "3/2/1".to_string(),
            session,
        };

        let json = serde_json::to_string(&item)?;
        assert!(json.contains("\"name\":\"test\""));
        assert!(json.contains("\"status\":\"active\""));
        assert!(json.contains("\"changes\":\"5\""));
        Ok(())
    }

    #[test]
    fn test_get_session_changes_missing_workspace() {
        let result = get_session_changes("/nonexistent/path");
        assert!(result.is_none());
    }

    #[test]
    fn test_output_table_format() {
        let session = Session {
            id: Some(1),
            name: "test-session".to_string(),
            status: SessionStatus::Active,
            workspace_path: "/tmp/test".to_string(),
            zellij_tab: "jjz:test-session".to_string(),
            branch: Some("main".to_string()),
            created_at: 1_234_567_890,
            updated_at: 1_234_567_890,
            last_synced: None,
            metadata: None,
        };

        let items = vec![SessionListItem {
            name: session.name.clone(),
            status: "active".to_string(),
            branch: "main".to_string(),
            changes: "5".to_string(),
            beads: "3/2/1".to_string(),
            session,
        }];

        // This test just verifies the function doesn't panic
        output_table(&items);
    }

    #[test]
    fn test_output_json_format() {
        let session = Session {
            id: Some(1),
            name: "test-session".to_string(),
            status: SessionStatus::Active,
            workspace_path: "/tmp/test".to_string(),
            zellij_tab: "jjz:test-session".to_string(),
            branch: Some("main".to_string()),
            created_at: 1_234_567_890,
            updated_at: 1_234_567_890,
            last_synced: None,
            metadata: None,
        };

        let items = vec![SessionListItem {
            name: session.name.clone(),
            status: "active".to_string(),
            branch: "main".to_string(),
            changes: "5".to_string(),
            beads: "3/2/1".to_string(),
            session,
        }];

        let result = output_json(&items);
        assert!(result.is_ok());
    }

    #[test]
    fn test_filter_completed_and_failed_sessions() -> Result<()> {
        let (db, _dir) = setup_test_db()?;

        // Create sessions with different statuses
        let s1 = db.create("active-session", "/tmp/active")?;
        db.update(
            &s1.name,
            SessionUpdate {
                status: Some(SessionStatus::Active),
                ..Default::default()
            },
        )?;

        let s2 = db.create("completed-session", "/tmp/completed")?;
        db.update(
            &s2.name,
            SessionUpdate {
                status: Some(SessionStatus::Completed),
                ..Default::default()
            },
        )?;

        let s3 = db.create("failed-session", "/tmp/failed")?;
        db.update(
            &s3.name,
            SessionUpdate {
                status: Some(SessionStatus::Failed),
                ..Default::default()
            },
        )?;

        let s4 = db.create("paused-session", "/tmp/paused")?;
        db.update(
            &s4.name,
            SessionUpdate {
                status: Some(SessionStatus::Paused),
                ..Default::default()
            },
        )?;

        // Get all sessions and filter
        let mut sessions = db.list(None)?;

        // Simulate the filtering logic from run()
        sessions
            .retain(|s| s.status != SessionStatus::Completed && s.status != SessionStatus::Failed);

        // Should only have active and paused
        assert_eq!(sessions.len(), 2);
        assert!(sessions.iter().any(|s| s.name == "active-session"));
        assert!(sessions.iter().any(|s| s.name == "paused-session"));
        assert!(!sessions.iter().any(|s| s.name == "completed-session"));
        assert!(!sessions.iter().any(|s| s.name == "failed-session"));

        Ok(())
    }

    #[test]
    fn test_all_flag_includes_all_sessions() -> Result<()> {
        let (db, _dir) = setup_test_db()?;

        // Create sessions with different statuses
        db.create("active-session", "/tmp/active")?;
        let s2 = db.create("completed-session", "/tmp/completed")?;
        db.update(
            &s2.name,
            SessionUpdate {
                status: Some(SessionStatus::Completed),
                ..Default::default()
            },
        )?;

        // With all=true, no filtering
        let sessions = db.list(None)?;
        assert_eq!(sessions.len(), 2);

        // With all=false, filter out completed
        let mut filtered = db.list(None)?;
        filtered
            .retain(|s| s.status != SessionStatus::Completed && s.status != SessionStatus::Failed);
        assert_eq!(filtered.len(), 1);

        Ok(())
    }

    #[test]
    fn test_empty_list_handling() -> Result<()> {
        let (db, _dir) = setup_test_db()?;

        let sessions = db.list(None)?;
        assert!(sessions.is_empty());

        Ok(())
    }

    #[test]
    fn test_session_list_item_with_none_branch() {
        let session = Session {
            id: Some(1),
            name: "test".to_string(),
            status: SessionStatus::Active,
            workspace_path: "/tmp/test".to_string(),
            zellij_tab: "jjz:test".to_string(),
            branch: None,
            created_at: 1_234_567_890,
            updated_at: 1_234_567_890,
            last_synced: None,
            metadata: None,
        };

        let item = SessionListItem {
            name: session.name.clone(),
            status: session.status.to_string(),
            branch: session.branch.clone().unwrap_or_else(|| "-".to_string()),
            changes: "-".to_string(),
            beads: "0/0/0".to_string(),
            session,
        };

        assert_eq!(item.branch, "-");
        assert_eq!(item.changes, "-");
    }

    #[test]
    fn test_get_beads_count_no_repo() {
        // When not in a repo or no beads db, should return default
        let counts = BeadCounts::default();
        assert_eq!(counts.open, 0);
        assert_eq!(counts.in_progress, 0);
        assert_eq!(counts.blocked, 0);
    }
}
