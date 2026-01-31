//! Contextual hints and smart suggestions for AI agents
//!
//! Provides context-aware hints based on system state:
//! - Suggested next actions
//! - State explanations
//! - Learning from errors
//! - Predictive hints

use serde::{Deserialize, Serialize};

use crate::{
    types::{BeadsSummary, Session, SessionStatus},
    Result,
};

// ═══════════════════════════════════════════════════════════════════════════
// HINT TYPES
// ═══════════════════════════════════════════════════════════════════════════

/// A contextual hint from jjz
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Hint {
    /// Hint type
    #[serde(rename = "type")]
    pub hint_type: HintType,

    /// Human-readable message
    pub message: String,

    /// Suggested command to run
    #[serde(skip_serializing_if = "Option::is_none")]
    pub suggested_command: Option<String>,

    /// Rationale for this hint
    #[serde(skip_serializing_if = "Option::is_none")]
    pub rationale: Option<String>,

    /// Additional context
    #[serde(skip_serializing_if = "Option::is_none")]
    pub context: Option<serde_json::Value>,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum HintType {
    /// Information about current state
    Info,
    /// Suggested next action
    Suggestion,
    /// Warning about potential issue
    Warning,
    /// Explanation of error
    Error,
    /// Learning tip
    Tip,
}

/// System state for hint generation
#[derive(Debug, Clone)]
pub struct SystemState {
    /// All sessions
    pub sessions: Vec<Session>,

    /// Whether system is initialized
    pub initialized: bool,

    /// Whether JJ repo exists
    pub jj_repo: bool,
}

/// Next action suggestion
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct NextAction {
    /// Action description
    pub action: String,

    /// Commands to execute
    pub commands: Vec<String>,
}

/// Complete hints response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HintsResponse {
    /// Current system context
    pub context: SystemContext,

    /// Generated hints
    pub hints: Vec<Hint>,

    /// Suggested next actions
    pub next_actions: Vec<NextAction>,
}

/// System context summary
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct SystemContext {
    /// Is jjz initialized?
    pub initialized: bool,

    /// Is this a JJ repository?
    pub jj_repo: bool,

    /// Total number of sessions
    pub sessions_count: usize,

    /// Number of active sessions
    pub active_sessions: usize,

    /// Are there uncommitted changes?
    pub has_changes: bool,
}

// ═══════════════════════════════════════════════════════════════════════════
// HINT GENERATION
// ═══════════════════════════════════════════════════════════════════════════

impl Hint {
    /// Create an info hint
    pub fn info(message: impl Into<String>) -> Self {
        Self {
            hint_type: HintType::Info,
            message: message.into(),
            suggested_command: None,
            rationale: None,
            context: None,
        }
    }

    /// Create a suggestion hint
    pub fn suggestion(message: impl Into<String>) -> Self {
        Self {
            hint_type: HintType::Suggestion,
            message: message.into(),
            suggested_command: None,
            rationale: None,
            context: None,
        }
    }

    /// Create a warning hint
    pub fn warning(message: impl Into<String>) -> Self {
        Self {
            hint_type: HintType::Warning,
            message: message.into(),
            suggested_command: None,
            rationale: None,
            context: None,
        }
    }

    /// Create a tip hint
    pub fn tip(message: impl Into<String>) -> Self {
        Self {
            hint_type: HintType::Tip,
            message: message.into(),
            suggested_command: None,
            rationale: None,
            context: None,
        }
    }

    /// Add a suggested command
    #[must_use]
    pub fn with_command(mut self, command: impl Into<String>) -> Self {
        self.suggested_command = Some(command.into());
        self
    }

    /// Add a rationale
    #[must_use]
    pub fn with_rationale(mut self, rationale: impl Into<String>) -> Self {
        self.rationale = Some(rationale.into());
        self
    }

    /// Add context
    #[must_use]
    pub fn with_context(mut self, context: serde_json::Value) -> Self {
        self.context = Some(context);
        self
    }
}

/// Generate contextual hints based on system state
///
/// # Errors
///
/// Returns error if unable to analyze state
pub fn generate_hints(state: &SystemState) -> Result<Vec<Hint>> {
    let mut hints = Vec::new();

    // No sessions - encourage creation
    if state.sessions.is_empty() {
        hints.push(
            Hint::suggestion("No sessions yet. Create your first parallel workspace!")
                .with_command("jjz add <name>")
                .with_rationale("Sessions enable parallel work on multiple features"),
        );
        return Ok(hints);
    }

    // Sessions with changes
    for session in &state.sessions {
        if session.status == SessionStatus::Active {
            // Note: In real implementation, would query actual changes
            // For now, just demonstrate the hint structure
            hints.push(
                Hint::info(format!("Session '{}' is active", session.name))
                    .with_command(format!("jjz status {}", session.name))
                    .with_rationale("Review session status regularly"),
            );
        }
    }

    // Completed sessions not removed
    let completed: Vec<_> = state
        .sessions
        .iter()
        .filter(|s| s.status == SessionStatus::Completed)
        .collect();

    for session in completed {
        let age = (chrono::Utc::now() - session.updated_at).num_days();
        if age > 1 {
            hints.push(
                Hint::suggestion(format!(
                    "Session '{}' completed {} day(s) ago, consider removing",
                    session.name, age
                ))
                .with_command(format!("jjz remove {} --merge", session.name))
                .with_rationale("Clean up completed work")
                .with_context(serde_json::json!({
                    "session": session.name,
                    "age_days": age,
                })),
            );
        }
    }

    // Failed sessions
    let failed: Vec<_> = state
        .sessions
        .iter()
        .filter(|s| s.status == SessionStatus::Failed)
        .collect();

    if !failed.is_empty() {
        for session in failed {
            hints.push(
                Hint::warning(format!("Session '{}' failed during creation", session.name))
                    .with_command(format!("jjz remove {}", session.name))
                    .with_rationale("Clean up failed session and retry"),
            );
        }
    }

    // Multiple active sessions - suggest dashboard
    let active_count = state
        .sessions
        .iter()
        .filter(|s| s.status == SessionStatus::Active)
        .count();

    if active_count > 2 {
        hints.push(
            Hint::tip("You have multiple active sessions. Use the dashboard for an overview")
                .with_command("jjz dashboard")
                .with_rationale("Visual overview helps manage multiple sessions"),
        );
    }

    Ok(hints)
}

/// Generate hints for a specific error
pub fn hints_for_error(error_code: &str, error_msg: &str) -> Vec<Hint> {
    match error_code {
        "SESSION_ALREADY_EXISTS" => {
            let session_name = extract_session_name(error_msg).unwrap_or("session");
            vec![
                Hint::suggestion("Use a different name for the new session")
                    .with_command(format!("jjz add {session_name}-v2"))
                    .with_rationale("Append version or date to differentiate"),
                Hint::suggestion("Switch to the existing session")
                    .with_command(format!("jjz focus {session_name}"))
                    .with_rationale("Continue work in existing session"),
                Hint::suggestion("Remove the existing session first")
                    .with_command(format!("jjz remove {session_name}"))
                    .with_rationale("Clean up old session before creating new one"),
            ]
        }
        "ZELLIJ_NOT_RUNNING" => {
            vec![
                Hint::suggestion("Start Zellij first")
                    .with_command("zellij")
                    .with_rationale("jjz requires Zellij to be running"),
                Hint::tip("You can attach to existing Zellij session")
                    .with_command("zellij attach")
                    .with_rationale("Reuse existing session instead of creating new one"),
            ]
        }
        "NOT_INITIALIZED" => {
            vec![
                Hint::suggestion("Initialize jjz in this repository")
                    .with_command("jjz init")
                    .with_rationale("Creates .jjz directory with configuration"),
                Hint::tip("After init, you can configure jjz in .jjz/config.toml")
                    .with_rationale("Customize workspace paths, hooks, and layouts"),
            ]
        }
        "JJ_NOT_FOUND" => {
            vec![
                Hint::warning("JJ (Jujutsu) is not installed or not in PATH")
                    .with_rationale("jjz requires JJ for workspace management"),
                Hint::suggestion("Install JJ from https://github.com/martinvonz/jj")
                    .with_rationale("Follow installation instructions for your platform"),
            ]
        }
        "SESSION_NOT_FOUND" => {
            vec![
                Hint::suggestion("List all sessions to see available ones")
                    .with_command("jjz list")
                    .with_rationale("Check session names and status"),
                Hint::tip("Session names are case-sensitive")
                    .with_rationale("Ensure exact match when referencing sessions"),
            ]
        }
        _ => vec![],
    }
}

/// Generate suggested next actions based on state
pub fn suggest_next_actions(state: &SystemState) -> Vec<NextAction> {
    let mut actions = Vec::new();

    // Not initialized
    if !state.initialized {
        actions.push(NextAction {
            action: "Initialize jjz".to_string(),
            commands: vec!["jjz init".to_string()],
        });
        return actions;
    }

    // No sessions
    if state.sessions.is_empty() {
        actions.push(NextAction {
            action: "Create first session".to_string(),
            commands: vec!["jjz add <name>".to_string()],
        });
        return actions;
    }

    // Has sessions - suggest common operations
    let has_active = state
        .sessions
        .iter()
        .any(|s| s.status == SessionStatus::Active);

    if has_active {
        actions.push(NextAction {
            action: "Review session status".to_string(),
            commands: vec!["jjz status".to_string(), "jjz dashboard".to_string()],
        });
    }

    let has_completed = state
        .sessions
        .iter()
        .any(|s| s.status == SessionStatus::Completed);

    if has_completed {
        let completed_name = state
            .sessions
            .iter()
            .find(|s| s.status == SessionStatus::Completed)
            .map(|s| &s.name);

        if let Some(name) = completed_name {
            actions.push(NextAction {
                action: "Clean up completed sessions".to_string(),
                commands: vec![format!("jjz remove {} --merge", name)],
            });
        }
    }

    actions.push(NextAction {
        action: "Create new session".to_string(),
        commands: vec!["jjz add <name>".to_string()],
    });

    actions
}

/// Generate complete hints response
///
/// # Errors
///
/// Returns error if unable to generate hints
pub fn generate_hints_response(state: &SystemState) -> Result<HintsResponse> {
    let hints = generate_hints(state)?;
    let next_actions = suggest_next_actions(state);

    let active_count = state
        .sessions
        .iter()
        .filter(|s| s.status == SessionStatus::Active)
        .count();

    let context = SystemContext {
        initialized: state.initialized,
        jj_repo: state.jj_repo,
        sessions_count: state.sessions.len(),
        active_sessions: active_count,
        has_changes: false, // TODO: Implement actual change detection
    };

    Ok(HintsResponse {
        context,
        hints,
        next_actions,
    })
}

// ═══════════════════════════════════════════════════════════════════════════
// HELPER FUNCTIONS
// ═══════════════════════════════════════════════════════════════════════════

/// Extract session name from error message
fn extract_session_name(error_msg: &str) -> Option<&str> {
    // Try to extract text between single quotes
    error_msg.split('\'').nth(1)
}

/// Generate hints for beads status
pub fn hints_for_beads(session_name: &str, beads: &BeadsSummary) -> Vec<Hint> {
    let mut hints = Vec::new();

    if beads.has_blockers() {
        hints.push(
            Hint::warning(format!(
                "Session '{}' has {} blocked issue(s)",
                session_name, beads.blocked
            ))
            .with_command("bv")
            .with_rationale("Resolve blockers to make progress")
            .with_context(serde_json::json!({
                "session": session_name,
                "blocked_count": beads.blocked,
            })),
        );
    }

    if beads.active() > 5 {
        hints.push(
            Hint::tip(format!(
                "Session '{}' has {} active issues - consider focusing on fewer tasks",
                session_name,
                beads.active()
            ))
            .with_rationale("Limiting work in progress improves focus"),
        );
    }

    if beads.total() == 0 {
        hints.push(
            Hint::info(format!("Session '{session_name}' has no beads issues"))
                .with_command("bd new")
                .with_rationale("Track your work with beads for better organization"),
        );
    }

    hints
}

// ═══════════════════════════════════════════════════════════════════════════
// TESTS
// ═══════════════════════════════════════════════════════════════════════════

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use chrono::Utc;

    use super::*;

    fn create_test_session(name: &str, status: SessionStatus) -> Session {
        Session {
            id: format!("id-{name}"),
            name: name.to_string(),
            status,
            workspace_path: PathBuf::from("/tmp/test"),
            branch: None,
            created_at: Utc::now(),
            updated_at: Utc::now(),
            last_synced: None,
            metadata: serde_json::Value::Null,
        }
    }

    #[test]
    fn test_hint_builders() {
        let hint = Hint::info("Test message")
            .with_command("jjz test")
            .with_rationale("Testing");

        assert_eq!(hint.hint_type, HintType::Info);
        assert_eq!(hint.message, "Test message");
        assert_eq!(hint.suggested_command, Some("jjz test".to_string()));
        assert_eq!(hint.rationale, Some("Testing".to_string()));
    }

    #[test]
    fn test_generate_hints_no_sessions() {
        let state = SystemState {
            sessions: Vec::new(),
            initialized: true,
            jj_repo: true,
        };

        let hints = generate_hints(&state).unwrap_or_default();
        assert!(!hints.is_empty());
        assert!(hints[0].message.contains("first parallel workspace"));
    }

    #[test]
    fn test_generate_hints_completed_session() {
        let mut session = create_test_session("old-session", SessionStatus::Completed);
        session.updated_at = Utc::now() - chrono::Duration::days(3);

        let state = SystemState {
            sessions: vec![session],
            initialized: true,
            jj_repo: true,
        };

        let hints = generate_hints(&state).unwrap_or_default();
        assert!(hints
            .iter()
            .any(|h| h.message.contains("consider removing")));
    }

    #[test]
    fn test_generate_hints_failed_session() {
        let state = SystemState {
            sessions: vec![create_test_session("failed-session", SessionStatus::Failed)],
            initialized: true,
            jj_repo: true,
        };

        let hints = generate_hints(&state).unwrap_or_default();
        assert!(hints.iter().any(|h| h.hint_type == HintType::Warning));
    }

    #[test]
    fn test_generate_hints_multiple_active() {
        let state = SystemState {
            sessions: vec![
                create_test_session("session1", SessionStatus::Active),
                create_test_session("session2", SessionStatus::Active),
                create_test_session("session3", SessionStatus::Active),
            ],
            initialized: true,
            jj_repo: true,
        };

        let hints = generate_hints(&state).unwrap_or_default();
        assert!(hints.iter().any(|h| h.message.contains("dashboard")));
    }

    #[test]
    fn test_hints_for_error_session_exists() {
        let hints = hints_for_error("SESSION_ALREADY_EXISTS", "Session 'test' already exists");
        assert_eq!(hints.len(), 3);
        assert!(hints[0].message.contains("different name"));
        assert!(hints[1].message.contains("Switch"));
        assert!(hints[2].message.contains("Remove"));
    }

    #[test]
    fn test_hints_for_error_zellij_not_running() {
        let hints = hints_for_error("ZELLIJ_NOT_RUNNING", "Zellij is not running");
        assert!(!hints.is_empty());
        assert!(hints[0].message.contains("Start Zellij"));
    }

    #[test]
    fn test_hints_for_error_not_initialized() {
        let hints = hints_for_error("NOT_INITIALIZED", "jjz not initialized");
        assert!(!hints.is_empty());
        assert!(hints[0].message.contains("Initialize"));
    }

    #[test]
    fn test_suggest_next_actions_not_initialized() {
        let state = SystemState {
            sessions: Vec::new(),
            initialized: false,
            jj_repo: true,
        };

        let actions = suggest_next_actions(&state);
        assert_eq!(actions.len(), 1);
        assert_eq!(actions[0].action, "Initialize jjz");
    }

    #[test]
    fn test_suggest_next_actions_no_sessions() {
        let state = SystemState {
            sessions: Vec::new(),
            initialized: true,
            jj_repo: true,
        };

        let actions = suggest_next_actions(&state);
        assert!(actions.iter().any(|a| a.action.contains("first session")));
    }

    #[test]
    fn test_suggest_next_actions_has_completed() {
        let state = SystemState {
            sessions: vec![create_test_session("done", SessionStatus::Completed)],
            initialized: true,
            jj_repo: true,
        };

        let actions = suggest_next_actions(&state);
        assert!(actions.iter().any(|a| a.action.contains("Clean up")));
    }

    #[test]
    fn test_hints_for_beads_blockers() {
        let beads = BeadsSummary {
            open: 2,
            in_progress: 1,
            blocked: 3,
            closed: 5,
        };

        let hints = hints_for_beads("test-session", &beads);
        assert!(hints.iter().any(|h| h.hint_type == HintType::Warning));
        assert!(hints.iter().any(|h| h.message.contains("blocked")));
    }

    #[test]
    fn test_hints_for_beads_too_many_active() {
        let beads = BeadsSummary {
            open: 4,
            in_progress: 3,
            blocked: 0,
            closed: 5,
        };

        let hints = hints_for_beads("test-session", &beads);
        assert!(hints.iter().any(|h| h.message.contains("fewer tasks")));
    }

    #[test]
    fn test_hints_for_beads_none() {
        let beads = BeadsSummary::default();

        let hints = hints_for_beads("test-session", &beads);
        assert!(hints.iter().any(|h| h.message.contains("no beads")));
    }

    #[test]
    fn test_extract_session_name() {
        assert_eq!(
            extract_session_name("Session 'test-name' already exists"),
            Some("test-name")
        );
        assert_eq!(
            extract_session_name("Session 'my-session' not found"),
            Some("my-session")
        );
    }

    #[test]
    fn test_generate_hints_response() {
        let state = SystemState {
            sessions: vec![create_test_session("active", SessionStatus::Active)],
            initialized: true,
            jj_repo: true,
        };

        let response = generate_hints_response(&state).unwrap_or_else(|_| HintsResponse {
            context: SystemContext {
                initialized: true,
                jj_repo: true,
                sessions_count: 0,
                active_sessions: 0,
                has_changes: false,
            },
            hints: Vec::new(),
            next_actions: Vec::new(),
        });

        assert_eq!(response.context.sessions_count, 1);
        assert_eq!(response.context.active_sessions, 1);
        assert!(!response.hints.is_empty());
        assert!(!response.next_actions.is_empty());
    }
}
