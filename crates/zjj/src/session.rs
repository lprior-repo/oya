//! Session data structures and utilities

#[cfg(test)]
use std::time::SystemTime;
use std::{fmt, str::FromStr};

use serde::{Deserialize, Serialize};
use zjj_core::{Error, Result};

/// Session status representing the lifecycle state
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum SessionStatus {
    /// Session is being created
    #[default]
    Creating,
    /// Session is active and ready for use
    Active,
    /// Session is temporarily paused
    Paused,
    /// Session work is completed
    Completed,
    /// Session creation or operation failed
    Failed,
}

impl fmt::Display for SessionStatus {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Creating => write!(f, "creating"),
            Self::Active => write!(f, "active"),
            Self::Paused => write!(f, "paused"),
            Self::Completed => write!(f, "completed"),
            Self::Failed => write!(f, "failed"),
        }
    }
}

impl FromStr for SessionStatus {
    type Err = Error;

    fn from_str(s: &str) -> Result<Self> {
        match s {
            "creating" => Ok(Self::Creating),
            "active" => Ok(Self::Active),
            "paused" => Ok(Self::Paused),
            "completed" => Ok(Self::Completed),
            "failed" => Ok(Self::Failed),
            _ => Err(Error::ValidationError(format!("Invalid status: {s}"))),
        }
    }
}

/// A ZJJ session representing a JJ workspace + Zellij tab pair
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct Session {
    /// Auto-generated database ID (None for new sessions not yet persisted)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub id: Option<i64>,
    /// Unique session name
    pub name: String,
    /// Current status of the session
    pub status: SessionStatus,
    /// Path to the JJ workspace directory
    pub workspace_path: String,
    /// Zellij tab name (format: `jjz:NAME`)
    pub zellij_tab: String,
    /// Git branch associated with this session
    #[serde(skip_serializing_if = "Option::is_none")]
    pub branch: Option<String>,
    /// Unix timestamp when session was created
    pub created_at: u64,
    /// Unix timestamp when session was last updated
    pub updated_at: u64,
    /// Unix timestamp of last sync operation
    #[serde(skip_serializing_if = "Option::is_none")]
    pub last_synced: Option<u64>,
    /// Extensible metadata as JSON
    #[serde(skip_serializing_if = "Option::is_none")]
    pub metadata: Option<serde_json::Value>,
}

impl Session {
    /// Create a new session with the given name and workspace path
    ///
    /// NOTE: This is primarily for testing. Production code should use
    /// `SessionDb::create` which handles persistence.
    #[cfg(test)]
    pub fn new(name: &str, workspace_path: &str) -> Result<Self> {
        validate_session_name(name)?;

        let now = SystemTime::now()
            .duration_since(SystemTime::UNIX_EPOCH)
            .map_err(|e| Error::Unknown(format!("System time error: {e}")))?
            .as_secs();

        Ok(Self {
            id: None,
            name: name.to_string(),
            status: SessionStatus::Creating,
            workspace_path: workspace_path.to_string(),
            zellij_tab: format!("jjz:{name}"),
            branch: None,
            created_at: now,
            updated_at: now,
            last_synced: None,
            metadata: None,
        })
    }
}

/// Fields that can be updated on an existing session
#[derive(Debug, Clone, Default)]
pub struct SessionUpdate {
    /// Update the session status
    pub status: Option<SessionStatus>,
    /// Update the branch
    pub branch: Option<String>,
    /// Update the last synced timestamp
    pub last_synced: Option<u64>,
    /// Update the metadata
    pub metadata: Option<serde_json::Value>,
}

/// Validate a session name
///
/// Session names must:
/// - Not be empty
/// - Not exceed 64 characters
/// - Only contain ASCII alphanumeric characters, dashes, and underscores
/// - Start with a letter (a-z, A-Z)
pub fn validate_session_name(name: &str) -> Result<()> {
    if name.is_empty() {
        return Err(Error::ValidationError(
            "Session name cannot be empty".into(),
        ));
    }

    // Check for non-ASCII characters first (prevents unicode bypasses)
    if !name.is_ascii() {
        return Err(Error::ValidationError(
            "Session name must contain only ASCII characters (a-z, A-Z, 0-9, -, _)".into(),
        ));
    }

    if name.len() > 64 {
        return Err(Error::ValidationError(
            "Session name cannot exceed 64 characters".into(),
        ));
    }

    // Only allow ASCII alphanumeric, dash, and underscore
    // Using is_ascii_alphanumeric() instead of is_alphanumeric() to reject unicode
    if !name
        .chars()
        .all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_')
    {
        return Err(Error::ValidationError(
            "Session name can only contain ASCII alphanumeric characters, dashes, and underscores"
                .into(),
        ));
    }

    // Must start with a letter (not dash, underscore, or digit)
    if let Some(first) = name.chars().next() {
        if !first.is_ascii_alphabetic() {
            return Err(Error::ValidationError(
                "Session name must start with a letter (a-z, A-Z)".into(),
            ));
        }
    }

    Ok(())
}

/// Validate a status transition
///
/// Enforces valid state transitions in the session lifecycle:
/// - Creating -> Active, Failed
/// - Active -> Paused, Completed, Failed
/// - Paused -> Active, Failed
/// - Failed -> Creating (retry)
/// - Completed -> Active (reopen)
#[allow(dead_code)]
pub fn validate_status_transition(from: SessionStatus, to: SessionStatus) -> Result<()> {
    use SessionStatus::{Active, Completed, Creating, Failed, Paused};

    let valid = matches!(
        (from, to),
        (Creating | Paused | Completed, Active)
            | (Creating | Active | Paused, Failed)
            | (Active, Paused | Completed)
            | (Failed, Creating) // Can retry failed session
    );

    if valid {
        Ok(())
    } else {
        Err(Error::ValidationError(format!(
            "Invalid status transition from {from} to {to}"
        )))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_session_new_valid() -> Result<()> {
        let session = Session::new("my-session", "/path/to/workspace")?;
        assert_eq!(session.name, "my-session");
        assert_eq!(session.zellij_tab, "jjz:my-session");
        assert_eq!(session.status, SessionStatus::Creating);
        assert!(session.id.is_none());
        assert!(session.created_at > 0);
        assert_eq!(session.created_at, session.updated_at);
        Ok(())
    }

    #[test]
    fn test_session_name_empty() {
        let result = validate_session_name("");
        assert!(result.is_err());
    }

    #[test]
    fn test_session_name_too_long() {
        let long_name = "a".repeat(65);
        let result = validate_session_name(&long_name);
        assert!(result.is_err());
    }

    #[test]
    fn test_session_name_invalid_chars() {
        let result = validate_session_name("my session");
        assert!(result.is_err());
    }

    #[test]
    fn test_session_name_starts_with_dash() {
        let result = validate_session_name("-session");
        assert!(result.is_err());
    }

    #[test]
    fn test_session_name_valid_with_underscore() {
        let result = validate_session_name("my_session");
        assert!(result.is_ok());
    }

    #[test]
    fn test_session_name_starts_with_underscore_rejected() {
        let result = validate_session_name("_session");
        assert!(result.is_err());
    }

    #[test]
    fn test_session_name_starts_with_digit_rejected() {
        let result = validate_session_name("123session");
        assert!(result.is_err());
    }

    #[test]
    fn test_session_name_rejects_unicode() {
        // Unicode characters should be rejected
        let unicode_cases = vec![
            "中文名字", // Chinese
            "日本語",   // Japanese
            "café",     // Accented Latin
            "Ñoño",     // Spanish
            "🚀rocket", // Emoji
            "naïve",    // Diaeresis
            "résumé",   // Accents
        ];

        for name in unicode_cases {
            let result = validate_session_name(name);
            assert!(result.is_err(), "Should reject unicode name: {name}");
        }
    }

    #[test]
    fn test_session_name_accepts_valid_names() {
        let valid_cases = vec![
            "name",
            "my-name",
            "myName",
            "MyName123",
            "name123",
            "n-a-m-e",
            "feature-branch-123",
            "UPPERCASE",
            "a", // Single letter
        ];

        for name in valid_cases {
            let result = validate_session_name(name);
            assert!(result.is_ok(), "Should accept valid name: {name}");
        }
    }

    #[test]
    fn test_status_display() {
        assert_eq!(SessionStatus::Creating.to_string(), "creating");
        assert_eq!(SessionStatus::Active.to_string(), "active");
        assert_eq!(SessionStatus::Paused.to_string(), "paused");
        assert_eq!(SessionStatus::Completed.to_string(), "completed");
        assert_eq!(SessionStatus::Failed.to_string(), "failed");
    }

    #[test]
    fn test_status_from_str() -> Result<()> {
        assert_eq!(
            SessionStatus::from_str("creating")?,
            SessionStatus::Creating
        );
        assert_eq!(SessionStatus::from_str("active")?, SessionStatus::Active);
        assert_eq!(SessionStatus::from_str("paused")?, SessionStatus::Paused);
        assert_eq!(
            SessionStatus::from_str("completed")?,
            SessionStatus::Completed
        );
        assert_eq!(SessionStatus::from_str("failed")?, SessionStatus::Failed);
        Ok(())
    }

    #[test]
    fn test_status_from_str_invalid() {
        let result = SessionStatus::from_str("invalid");
        assert!(result.is_err());
    }

    #[test]
    fn test_validate_transition_creating_to_active() {
        let result = validate_status_transition(SessionStatus::Creating, SessionStatus::Active);
        assert!(result.is_ok());
    }

    #[test]
    fn test_validate_transition_creating_to_failed() {
        let result = validate_status_transition(SessionStatus::Creating, SessionStatus::Failed);
        assert!(result.is_ok());
    }

    #[test]
    fn test_validate_transition_active_to_paused() {
        let result = validate_status_transition(SessionStatus::Active, SessionStatus::Paused);
        assert!(result.is_ok());
    }

    #[test]
    fn test_validate_transition_invalid() {
        let result = validate_status_transition(SessionStatus::Completed, SessionStatus::Paused);
        assert!(result.is_err());
    }
}
