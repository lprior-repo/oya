//! Command pane tracking data structures
//!
//! Provides types for tracking command panes opened by the Zellij plugin,
//! including their lifecycle, state, and associated context.

use std::time::{Duration, Instant};

/// Unique identifier for a command pane
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct CommandPaneId(String);

impl CommandPaneId {
    #[allow(dead_code)]
    pub fn new(id: String) -> Self {
        Self(id)
    }

    #[allow(dead_code)]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

/// Tracking state for a command pane
#[derive(Clone, Debug, PartialEq, Eq)]
#[allow(dead_code)]
pub enum CommandPaneState {
    /// Pane has been opened but command not yet started
    Initializing,
    /// Command is currently running
    Running,
    /// Command completed successfully
    Completed,
    /// Command failed
    Failed,
    /// Pane was closed
    Closed,
}

impl CommandPaneState {
    #[allow(dead_code)]
    pub fn as_str(&self) -> &str {
        match self {
            Self::Initializing => "initializing",
            Self::Running => "running",
            Self::Completed => "completed",
            Self::Failed => "failed",
            Self::Closed => "closed",
        }
    }

    #[allow(dead_code)]
    pub fn color(&self) -> &str {
        match self {
            Self::Initializing => "\x1b[90m",
            Self::Running => "\x1b[33m",
            Self::Completed => "\x1b[32m",
            Self::Failed => "\x1b[31m",
            Self::Closed => "\x1b[90m",
        }
    }

    #[allow(dead_code)]
    pub fn symbol(&self) -> &str {
        match self {
            Self::Initializing => "○",
            Self::Running => "◐",
            Self::Completed => "●",
            Self::Failed => "✗",
            Self::Closed => "⊘",
        }
    }

    #[allow(dead_code)]
    pub fn is_terminal(&self) -> bool {
        matches!(self, Self::Completed | Self::Failed | Self::Closed)
    }
}

/// Tracks a command pane with its associated context
#[derive(Clone, Debug)]
#[allow(dead_code)]
pub struct CommandPane {
    /// Unique pane identifier from Zellij
    pub id: CommandPaneId,
    /// Current state of the command pane
    pub state: CommandPaneState,
    /// Bead ID this pane is operating on
    pub bead_id: String,
    /// Stage name being run (if applicable)
    pub stage_name: Option<String>,
    /// Action being performed (e.g., "run_stage", "spawn_workspace")
    pub action: String,
    /// When the pane was opened
    #[allow(dead_code)]
    pub opened_at: Instant,
    /// When the pane completed (if applicable)
    pub completed_at: Option<Instant>,
    /// Exit code from command (if completed)
    pub exit_code: Option<i32>,
    /// Human-readable description
    #[allow(dead_code)]
    pub description: String,
}

impl CommandPane {
    #[allow(dead_code)]
    pub fn new(
        id: CommandPaneId,
        bead_id: String,
        stage_name: Option<String>,
        action: String,
    ) -> Self {
        let description = if let Some(stage) = stage_name.as_ref() {
            format!("{}: {} - {}", action, bead_id, stage)
        } else {
            format!("{}: {}", action, bead_id)
        };

        Self {
            id,
            state: CommandPaneState::Initializing,
            bead_id,
            stage_name,
            action,
            opened_at: Instant::now(),
            completed_at: None,
            exit_code: None,
            description,
        }
    }

    #[allow(dead_code)]
    pub fn mark_running(&mut self) {
        self.state = CommandPaneState::Running;
    }

    #[allow(dead_code)]
    pub fn mark_completed(&mut self, exit_code: i32) {
        self.state = if exit_code == 0 {
            CommandPaneState::Completed
        } else {
            CommandPaneState::Failed
        };
        self.completed_at = Some(Instant::now());
        self.exit_code = Some(exit_code);
    }

    #[allow(dead_code)]
    pub fn mark_closed(&mut self) {
        self.state = CommandPaneState::Closed;
        self.completed_at = Some(Instant::now());
    }

    #[allow(dead_code)]
    pub fn duration(&self) -> Option<Duration> {
        let end = self.completed_at.unwrap_or_else(Instant::now);
        Some(end.duration_since(self.opened_at))
    }

    #[allow(dead_code)]
    pub fn is_active(&self) -> bool {
        matches!(
            self.state,
            CommandPaneState::Initializing | CommandPaneState::Running
        )
    }

    #[allow(dead_code)]
    pub fn is_successful(&self) -> bool {
        matches!(self.state, CommandPaneState::Completed)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn create_test_pane() -> CommandPane {
        CommandPane::new(
            CommandPaneId::new("pane-123".to_string()),
            "bead-abc".to_string(),
            Some("implement".to_string()),
            "run_stage".to_string(),
        )
    }

    #[test]
    fn test_command_pane_id_new() {
        let id = CommandPaneId::new("test-id".to_string());
        assert_eq!(id.as_str(), "test-id");
    }

    #[test]
    fn test_command_pane_initial_state() {
        let pane = create_test_pane();
        assert!(pane.is_active());
        assert!(!pane.is_successful());
        assert_eq!(pane.bead_id, "bead-abc");
        assert_eq!(pane.stage_name, Some("implement".to_string()));
        assert_eq!(pane.action, "run_stage");
        assert!(pane.description.contains("bead-abc"));
        assert!(pane.description.contains("implement"));
    }

    #[test]
    fn test_command_pane_state_transitions() {
        let mut pane = create_test_pane();

        // Initial state
        assert_eq!(pane.state, CommandPaneState::Initializing);

        // Transition to running
        pane.mark_running();
        assert_eq!(pane.state, CommandPaneState::Running);
        assert!(pane.is_active());

        // Transition to completed successfully
        pane.mark_completed(0);
        assert_eq!(pane.state, CommandPaneState::Completed);
        assert!(!pane.is_active());
        assert!(pane.is_successful());
        assert_eq!(pane.exit_code, Some(0));
        assert!(pane.completed_at.is_some());
    }

    #[test]
    fn test_command_pane_failure() {
        let mut pane = create_test_pane();
        pane.mark_running();
        pane.mark_completed(1);

        assert_eq!(pane.state, CommandPaneState::Failed);
        assert!(!pane.is_active());
        assert!(!pane.is_successful());
        assert_eq!(pane.exit_code, Some(1));
    }

    #[test]
    fn test_command_pane_closed() {
        let mut pane = create_test_pane();
        pane.mark_closed();

        assert_eq!(pane.state, CommandPaneState::Closed);
        assert!(!pane.is_active());
        assert!(pane.completed_at.is_some());
    }

    #[test]
    fn test_command_pane_duration() {
        let mut pane = create_test_pane();
        std::thread::sleep(Duration::from_millis(10));
        pane.mark_completed(0);

        let duration = pane.duration();
        assert!(duration.is_some());
        assert!(duration.is_some_and(|d| d >= Duration::from_millis(10)));
    }

    #[test]
    fn test_command_pane_duration_while_active() {
        let pane = create_test_pane();
        std::thread::sleep(Duration::from_millis(10));

        let duration = pane.duration();
        assert!(duration.is_some());
        assert!(duration.is_some_and(|d| d >= Duration::from_millis(10)));
    }

    #[test]
    fn test_command_pane_without_stage() {
        let pane = CommandPane::new(
            CommandPaneId::new("pane-456".to_string()),
            "bead-xyz".to_string(),
            None,
            "spawn_workspace".to_string(),
        );

        assert_eq!(pane.stage_name, None);
        assert!(pane.description.contains("spawn_workspace"));
        assert!(pane.description.contains("bead-xyz"));
        assert!(!pane.description.contains("implement"));
    }

    #[test]
    fn test_command_pane_state_symbols() {
        assert_eq!(CommandPaneState::Initializing.symbol(), "○");
        assert_eq!(CommandPaneState::Running.symbol(), "◐");
        assert_eq!(CommandPaneState::Completed.symbol(), "●");
        assert_eq!(CommandPaneState::Failed.symbol(), "✗");
        assert_eq!(CommandPaneState::Closed.symbol(), "⊘");
    }

    #[test]
    fn test_command_pane_state_colors() {
        assert_eq!(CommandPaneState::Initializing.color(), "\x1b[90m");
        assert_eq!(CommandPaneState::Running.color(), "\x1b[33m");
        assert_eq!(CommandPaneState::Completed.color(), "\x1b[32m");
        assert_eq!(CommandPaneState::Failed.color(), "\x1b[31m");
        assert_eq!(CommandPaneState::Closed.color(), "\x1b[90m");
    }

    #[test]
    fn test_command_pane_state_strings() {
        assert_eq!(CommandPaneState::Initializing.as_str(), "initializing");
        assert_eq!(CommandPaneState::Running.as_str(), "running");
        assert_eq!(CommandPaneState::Completed.as_str(), "completed");
        assert_eq!(CommandPaneState::Failed.as_str(), "failed");
        assert_eq!(CommandPaneState::Closed.as_str(), "closed");
    }

    #[test]
    fn test_command_pane_state_is_terminal() {
        assert!(!CommandPaneState::Initializing.is_terminal());
        assert!(!CommandPaneState::Running.is_terminal());
        assert!(CommandPaneState::Completed.is_terminal());
        assert!(CommandPaneState::Failed.is_terminal());
        assert!(CommandPaneState::Closed.is_terminal());
    }

    #[test]
    fn test_command_pane_id_hash() {
        use std::collections::HashMap;
        let id1 = CommandPaneId::new("same".to_string());
        let id2 = CommandPaneId::new("same".to_string());
        let id3 = CommandPaneId::new("different".to_string());

        assert_eq!(id1, id2);
        assert_ne!(id1, id3);

        let mut map = HashMap::new();
        map.insert(id1.clone(), "first");
        map.insert(id3.clone(), "second");

        // Same ID should overwrite
        map.insert(id2, "overwritten");

        assert_eq!(map.len(), 2);
        assert_eq!(map.get(&id1), Some(&"overwritten"));
        assert_eq!(map.get(&id3), Some(&"second"));
    }
}
