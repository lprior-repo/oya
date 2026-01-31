//! Switch to a session's Zellij tab

use anyhow::Result;

use crate::{
    cli::{attach_to_zellij_session, is_inside_zellij, run_command},
    commands::get_session_db,
    json_output::FocusOutput,
};

/// Options for the focus command
#[derive(Debug, Clone, Default)]
pub struct FocusOptions {
    /// Output as JSON
    pub json: bool,
}

/// Run the focus command with options
pub fn run_with_options(name: &str, options: &FocusOptions) -> Result<()> {
    let db = get_session_db()?;

    // Get the session
    let session = db
        .get(name)?
        .ok_or_else(|| anyhow::anyhow!("Session '{name}' not found"))?;

    let zellij_tab = session.zellij_tab;

    if is_inside_zellij() {
        // Inside Zellij: Switch to the tab
        run_command("zellij", &["action", "go-to-tab-name", &zellij_tab])?;

        if options.json {
            let output = FocusOutput {
                success: true,
                session_name: name.to_string(),
                zellij_tab,
                message: format!("Switched to session '{name}'"),
            };
            println!("{}", serde_json::to_string(&output)?);
        } else {
            println!("Switched to session '{name}'");
        }
    } else {
        // Outside Zellij: Attach to the Zellij session
        // User will land in session and can navigate to desired tab
        if options.json {
            let output = FocusOutput {
                success: true,
                session_name: name.to_string(),
                zellij_tab: zellij_tab.clone(),
                message: format!(
                    "Session '{name}' is in tab '{zellij_tab}'. Attaching to Zellij session..."
                ),
            };
            println!("{}", serde_json::to_string(&output)?);
        } else {
            println!("Session '{name}' is in tab '{zellij_tab}'");
            println!("Attaching to Zellij session...");
        }
        attach_to_zellij_session(None)?;
        // Note: This never returns - we exec into Zellij
    }

    Ok(())
}

#[cfg(test)]
mod tests {
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
    fn test_focus_session_not_found() -> Result<()> {
        let (db, _dir) = setup_test_db()?;

        // Try to get a non-existent session
        let result = db.get("nonexistent")?;
        assert!(result.is_none());

        // Verify the error message format when session not found
        let session_name = "nonexistent";
        let result = db
            .get(session_name)?
            .ok_or_else(|| anyhow::anyhow!("Session '{session_name}' not found"));

        assert!(result.is_err());
        if let Err(e) = result {
            assert_eq!(e.to_string(), "Session 'nonexistent' not found");
        }

        Ok(())
    }

    #[test]
    fn test_focus_session_exists() -> Result<()> {
        let (db, _dir) = setup_test_db()?;

        // Create a session
        let session = db.create("test-session", "/tmp/test")?;

        // Verify we can retrieve it
        let retrieved = db.get("test-session")?;
        assert!(retrieved.is_some());

        let retrieved_session = retrieved.ok_or_else(|| anyhow::anyhow!("Session not found"))?;
        assert_eq!(retrieved_session.name, session.name);
        assert_eq!(retrieved_session.zellij_tab, "jjz:test-session");

        Ok(())
    }

    #[test]
    fn test_focus_session_with_hyphens() -> Result<()> {
        let (db, _dir) = setup_test_db()?;

        // Create a session with hyphens in the name
        let _session = db.create("my-test-session", "/tmp/my-test")?;

        // Verify we can retrieve it
        let retrieved = db.get("my-test-session")?;
        assert!(retrieved.is_some());

        let retrieved_session = retrieved.ok_or_else(|| anyhow::anyhow!("Session not found"))?;
        assert_eq!(retrieved_session.name, "my-test-session");
        assert_eq!(retrieved_session.zellij_tab, "jjz:my-test-session");

        Ok(())
    }

    #[test]
    fn test_focus_session_with_underscores() -> Result<()> {
        let (db, _dir) = setup_test_db()?;

        // Create a session with underscores in the name
        let _session = db.create("my_test_session", "/tmp/my_test")?;

        // Verify we can retrieve it
        let retrieved = db.get("my_test_session")?;
        assert!(retrieved.is_some());

        let retrieved_session = retrieved.ok_or_else(|| anyhow::anyhow!("Session not found"))?;
        assert_eq!(retrieved_session.name, "my_test_session");
        assert_eq!(retrieved_session.zellij_tab, "jjz:my_test_session");

        Ok(())
    }

    #[test]
    fn test_focus_session_with_mixed_special_chars() -> Result<()> {
        let (db, _dir) = setup_test_db()?;

        // Create a session with mixed special characters
        let _session = db.create("my-test_123", "/tmp/my-test_123")?;

        // Verify we can retrieve it
        let retrieved = db.get("my-test_123")?;
        assert!(retrieved.is_some());

        let retrieved_session = retrieved.ok_or_else(|| anyhow::anyhow!("Session not found"))?;
        assert_eq!(retrieved_session.name, "my-test_123");
        assert_eq!(retrieved_session.zellij_tab, "jjz:my-test_123");

        Ok(())
    }

    #[test]
    fn test_zellij_tab_format() -> Result<()> {
        let (db, _dir) = setup_test_db()?;

        // Create sessions and verify tab name format
        let session1 = db.create("session1", "/tmp/s1")?;
        assert_eq!(session1.zellij_tab, "jjz:session1");

        let session2 = db.create("my-session", "/tmp/s2")?;
        assert_eq!(session2.zellij_tab, "jjz:my-session");

        let session3 = db.create("test_session_123", "/tmp/s3")?;
        assert_eq!(session3.zellij_tab, "jjz:test_session_123");

        Ok(())
    }

    #[test]
    fn test_is_inside_zellij_detection() {
        // Save original value
        let original = std::env::var("ZELLIJ").ok();

        // Test when ZELLIJ env var is not set
        std::env::remove_var("ZELLIJ");
        assert!(!is_inside_zellij());

        // Test when ZELLIJ env var is set
        std::env::set_var("ZELLIJ", "1");
        assert!(is_inside_zellij());

        // Restore original value
        if let Some(val) = original {
            std::env::set_var("ZELLIJ", val);
        } else {
            std::env::remove_var("ZELLIJ");
        }
    }
}
