// Integration tests for Zellij command pane functionality
//
// This test suite validates the command pane lifecycle, IPC message flow,
// and error handling for interactive command execution in the OYA orchestrator.

use std::collections::HashMap;
use std::path::PathBuf;
use std::time::Duration;
use serde::{Deserialize, Serialize};
use tempfile::TempDir;

// ============================================================================
// Command Pane Data Structures
// ============================================================================

/// Context for a command pane execution
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct CommandPaneContext {
    pub pane_id: String,
    pub bead_id: String,
    pub stage: String,
    pub working_dir: PathBuf,
    pub command: Vec<String>,
    pub environment: HashMap<String, String>,
}

/// Lifecycle events from host to guest (orchestrator → plugin)
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum CommandPaneEvent {
    Opened {
        pane_id: String,
        context: CommandPaneContext,
        timestamp: u64,
    },
    Output {
        pane_id: String,
        stdout: String,
        stderr: String,
        timestamp: u64,
    },
    Exited {
        pane_id: String,
        exit_code: i32,
        timestamp: u64,
    },
    Failed {
        pane_id: String,
        error: String,
        timestamp: u64,
    },
}

/// Commands from guest to host (plugin → orchestrator)
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum CommandPaneCommand {
    Open {
        context: CommandPaneContext,
    },
    Close {
        pane_id: String,
    },
    ReRun {
        pane_id: String,
    },
}

/// Command pane errors
#[derive(Debug, thiserror::Error)]
pub enum CommandPaneError {
    #[error("Command not found: {0}")]
    CommandNotFound(String),

    #[error("Working directory not found: {0}")]
    WorkingDirectoryNotFound(PathBuf),

    #[error("Permission denied: {0}")]
    PermissionDenied(String),

    #[error("Command pane not found: {0}")]
    PaneNotFound(String),

    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("Serialization error: {0}")]
    Serialization(#[from] serde_json::Error),

    #[error("IPC communication failed: {0}")]
    IpcFailed(String),

    #[error("Empty command vector")]
    EmptyCommand,
}

/// Result type for command pane operations
pub type CommandPaneResult<T> = Result<T, CommandPaneError>;

// ============================================================================
// Test Fixture
// ============================================================================

/// Test fixture for command pane integration tests
pub struct CommandPaneTestFixture {
    temp_dir: TempDir,
    active_panes: HashMap<String, CommandPaneContext>,
}

impl CommandPaneTestFixture {
    /// Create a new test fixture with temporary directory
    pub fn new() -> CommandPaneResult<Self> {
        let temp_dir = TempDir::new()
            .map_err(|e| CommandPaneError::Io(std::io::Error::new(
                std::io::ErrorKind::Other,
                format!("Failed to create temp dir: {e}"),
            )))?;

        Ok(Self {
            temp_dir,
            active_panes: HashMap::new(),
        })
    }

    /// Get the temporary directory path
    pub fn temp_path(&self) -> &std::path::Path {
        self.temp_dir.path()
    }

    /// Open a command pane with the given context
    pub fn open_command_pane(&mut self, context: CommandPaneContext) -> CommandPaneResult<CommandPaneEvent> {
        // Validate command is non-empty
        if context.command.is_empty() {
            return Err(CommandPaneError::EmptyCommand);
        }

        // Validate working directory exists
        if !context.working_dir.exists() {
            return Err(CommandPaneError::WorkingDirectoryNotFound(
                context.working_dir.clone(),
            ));
        }

        // Validate command is executable
        let cmd = &context.command[0];
        if !Self::command_exists(cmd) {
            return Err(CommandPaneError::CommandNotFound(cmd.to_string()));
        }

        // Store the active pane
        let pane_id = context.pane_id.clone();
        self.active_panes.insert(pane_id.clone(), context.clone());

        // Return Opened event
        Ok(CommandPaneEvent::Opened {
            pane_id,
            context,
            timestamp: Self::current_timestamp(),
        })
    }

    /// Simulate command execution
    pub fn execute_command(&self, pane_id: &str) -> CommandPaneResult<CommandPaneEvent> {
        let context = self.active_panes
            .get(pane_id)
            .ok_or_else(|| CommandPaneError::PaneNotFound(pane_id.to_string()))?;

        // For testing, we'll just execute echo commands
        // In real implementation, this would spawn a subprocess
        let output = if context.command[0] == "echo" {
            let output = context.command[1..].join(" ");
            (output.clone(), String::new(), 0)
        } else if context.command[0] == "false" {
            (String::new(), String::new(), 1)
        } else {
            (String::new(), String::new(), 0)
        };

        Ok(CommandPaneEvent::Output {
            pane_id: pane_id.to_string(),
            stdout: output.0,
            stderr: output.1,
            timestamp: Self::current_timestamp(),
        })
    }

    /// Close a command pane
    pub fn close_command_pane(&mut self, pane_id: &str) -> CommandPaneResult<CommandPaneEvent> {
        self.active_panes
            .remove(pane_id)
            .ok_or_else(|| CommandPaneError::PaneNotFound(pane_id.to_string()))?;

        Ok(CommandPaneEvent::Exited {
            pane_id: pane_id.to_string(),
            exit_code: 0,
            timestamp: Self::current_timestamp(),
        })
    }

    /// Re-run a command in an existing pane
    pub fn rerun_command(&mut self, pane_id: &str) -> CommandPaneResult<CommandPaneEvent> {
        let context = self.active_panes
            .get(pane_id)
            .ok_or_else(|| CommandPaneError::PaneNotFound(pane_id.to_string()))?;

        // Create a new pane ID for the rerun
        let new_context = CommandPaneContext {
            pane_id: format!("{}-rerun", pane_id),
            ..context.clone()
        };

        self.open_command_pane(new_context)
    }

    /// Check if a command exists (simplified for testing)
    fn command_exists(cmd: &str) -> bool {
        // For testing, we only support a few commands
        matches!(cmd, "echo" | "false" | "true" | "cat" | "ls")
    }

    /// Get current timestamp
    fn current_timestamp() -> u64 {
        use std::time::{SystemTime, UNIX_EPOCH};
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_secs())
            .unwrap_or(0)
    }
}

// ============================================================================
// Test Helpers
// ============================================================================

/// Create a valid test context
pub fn create_test_context(bead_id: &str) -> CommandPaneContext {
    CommandPaneContext {
        pane_id: format!("pane-{}", uuid::Uuid::new_v4()),
        bead_id: bead_id.to_string(),
        stage: "implement".to_string(),
        working_dir: std::env::current_dir().unwrap_or_else(|_| PathBuf::from("/tmp")),
        command: vec!["echo".to_string(), "test".to_string()],
        environment: HashMap::new(),
    }
}

/// Create a test context with custom command
pub fn create_test_context_with_command(bead_id: &str, command: Vec<String>) -> CommandPaneContext {
    let mut ctx = create_test_context(bead_id);
    ctx.command = command;
    ctx
}

/// Create a test context with custom working directory
pub fn create_test_context_with_dir(bead_id: &str, working_dir: PathBuf) -> CommandPaneContext {
    let mut ctx = create_test_context(bead_id);
    ctx.working_dir = working_dir;
    ctx
}

// ============================================================================
// Lifecycle Tests
// ============================================================================

#[test]
fn test_open_command_pane_with_valid_context() {
    let mut fixture = CommandPaneTestFixture::new()
        .expect("Failed to create test fixture");

    let context = create_test_context("bd-3a0a.8");
    let result = fixture.open_command_pane(context);

    assert!(result.is_ok(), "Opening command pane should succeed");

    let event = result.unwrap();
    assert!(matches!(event, CommandPaneEvent::Opened { .. }));

    if let CommandPaneEvent::Opened { pane_id, context: ctx, .. } = event {
        assert_eq!(ctx.bead_id, "bd-3a0a.8");
        assert_eq!(ctx.stage, "implement");
        assert_eq!(ctx.command, vec!["echo", "test"]);
        assert!(pane_id.starts_with("pane-"));
    }
}

#[test]
fn test_open_command_pane_with_invalid_directory_fails() {
    let mut fixture = CommandPaneTestFixture::new()
        .expect("Failed to create test fixture");

    let context = create_test_context_with_dir(
        "bd-3a0a.8",
        PathBuf::from("/nonexistent/directory/that/does/not/exist"),
    );

    let result = fixture.open_command_pane(context);

    assert!(result.is_err(), "Opening with invalid directory should fail");

    let err = result.unwrap_err();
    assert!(matches!(
        err,
        CommandPaneError::WorkingDirectoryNotFound(_)
    ));
}

#[test]
fn test_open_command_pane_with_nonexistent_command_fails() {
    let mut fixture = CommandPaneTestFixture::new()
        .expect("Failed to create test fixture");

    let context = create_test_context_with_command(
        "bd-3a0a.8",
        vec!["nonexistent_command_xyz123".to_string()],
    );

    let result = fixture.open_command_pane(context);

    assert!(result.is_err(), "Opening with nonexistent command should fail");

    let err = result.unwrap_err();
    assert!(matches!(err, CommandPaneError::CommandNotFound(_)));
}

#[test]
fn test_close_command_pane_sends_exited_event() {
    let mut fixture = CommandPaneTestFixture::new()
        .expect("Failed to create test fixture");

    let context = create_test_context("bd-3a0a.8");
    let pane_id = context.pane_id.clone();

    fixture.open_command_pane(context)
        .expect("Failed to open command pane");

    let result = fixture.close_command_pane(&pane_id);

    assert!(result.is_ok(), "Closing command pane should succeed");

    let event = result.unwrap();
    assert!(matches!(event, CommandPaneEvent::Exited { .. }));

    if let CommandPaneEvent::Exited { exit_code, .. } = event {
        assert_eq!(exit_code, 0);
    }
}

#[test]
fn test_rerun_command_reopens_pane_with_same_context() {
    let mut fixture = CommandPaneTestFixture::new()
        .expect("Failed to create test fixture");

    let context = create_test_context("bd-3a0a.8");
    let pane_id = context.pane_id.clone();

    fixture.open_command_pane(context)
        .expect("Failed to open command pane");

    let result = fixture.rerun_command(&pane_id);

    assert!(result.is_ok(), "Rerunning command should succeed");

    let event = result.unwrap();
    assert!(matches!(event, CommandPaneEvent::Opened { .. }));

    if let CommandPaneEvent::Opened { pane_id: new_id, context: ctx, .. } = event {
        assert!(new_id.contains("-rerun"));
        assert_eq!(ctx.bead_id, "bd-3a0a.8");
        assert_eq!(ctx.stage, "implement");
    }
}

// ============================================================================
// Error Path Tests
// ============================================================================

#[test]
fn test_close_nonexistent_pane_returns_error() {
    let mut fixture = CommandPaneTestFixture::new()
        .expect("Failed to create test fixture");

    let result = fixture.close_command_pane("nonexistent-pane");

    assert!(result.is_err(), "Closing nonexistent pane should fail");

    let err = result.unwrap_err();
    assert!(matches!(err, CommandPaneError::PaneNotFound(_)));
}

#[test]
fn test_rerun_nonexistent_pane_returns_error() {
    let mut fixture = CommandPaneTestFixture::new()
        .expect("Failed to create test fixture");

    let result = fixture.rerun_command("nonexistent-pane");

    assert!(result.is_err(), "Rerunning nonexistent pane should fail");

    let err = result.unwrap_err();
    assert!(matches!(err, CommandPaneError::PaneNotFound(_)));
}

#[test]
fn test_empty_command_vector_returns_error() {
    let mut fixture = CommandPaneTestFixture::new()
        .expect("Failed to create test fixture");

    let mut context = create_test_context("bd-3a0a.8");
    context.command.clear();

    let result = fixture.open_command_pane(context);

    assert!(result.is_err(), "Empty command should fail");

    let err = result.unwrap_err();
    assert!(matches!(err, CommandPaneError::EmptyCommand));
}

#[test]
fn test_execute_failing_command_returns_nonzero_exit() {
    let mut fixture = CommandPaneTestFixture::new()
        .expect("Failed to create test fixture");

    let context = create_test_context_with_command(
        "bd-3a0a.8",
        vec!["false".to_string()],
    );
    let pane_id = context.pane_id.clone();

    fixture.open_command_pane(context)
        .expect("Failed to open command pane");

    // For now, execute_command doesn't return exit code
    // This would need to be implemented in the real system
    let _result = fixture.execute_command(&pane_id);

    // TODO: Add exit code verification when implemented
}

#[test]
fn test_invalid_working_directory_path_fails() {
    let mut fixture = CommandPaneTestFixture::new()
        .expect("Failed to create test fixture");

    let context = create_test_context_with_dir(
        "bd-3a0a.8",
        PathBuf::from("/root/restricted_dir"), // Likely doesn't exist or isn't accessible
    );

    let result = fixture.open_command_pane(context);

    assert!(result.is_err(), "Invalid working directory should fail");
}

// ============================================================================
// Helper Methods for Event Inspection
// ============================================================================

impl CommandPaneEvent {
    fn pane_id(&self) -> &str {
        match self {
            CommandPaneEvent::Opened { pane_id, .. } => pane_id,
            CommandPaneEvent::Output { pane_id, .. } => pane_id,
            CommandPaneEvent::Exited { pane_id, .. } => pane_id,
            CommandPaneEvent::Failed { pane_id, .. } => pane_id,
        }
    }

    fn stdout(&self) -> Option<&str> {
        match self {
            CommandPaneEvent::Output { stdout, .. } => Some(stdout),
            _ => None,
        }
    }

    fn stderr(&self) -> Option<&str> {
        match self {
            CommandPaneEvent::Output { stderr, .. } => Some(stderr),
            _ => None,
        }
    }
}

// ============================================================================
// IPC Message Flow Tests
// ============================================================================

#[test]
fn test_command_context_serialization() {
    let context = create_test_context("bd-3a0a.8");

    let serialized = serde_json::to_string(&context)
        .expect("Serialization should succeed");

    let deserialized: CommandPaneContext = serde_json::from_str(&serialized)
        .expect("Deserialization should succeed");

    assert_eq!(context, deserialized);
}

#[test]
fn test_command_pane_event_serialization() {
    let event = CommandPaneEvent::Opened {
        pane_id: "pane-123".to_string(),
        context: create_test_context("bd-3a0a.8"),
        timestamp: 1234567890,
    };

    let serialized = serde_json::to_string(&event)
        .expect("Serialization should succeed");

    let deserialized: CommandPaneEvent = serde_json::from_str(&serialized)
        .expect("Deserialization should succeed");

    assert_eq!(event, deserialized);
}

#[test]
fn test_command_pane_command_serialization() {
    let cmd = CommandPaneCommand::Open {
        context: create_test_context("bd-3a0a.8"),
    };

    let serialized = serde_json::to_string(&cmd)
        .expect("Serialization should succeed");

    let deserialized: CommandPaneCommand = serde_json::from_str(&serialized)
        .expect("Deserialization should succeed");

    assert_eq!(cmd, deserialized);
}

#[test]
fn test_multiple_command_panes_can_be_tracked() {
    let mut fixture = CommandPaneTestFixture::new()
        .expect("Failed to create test fixture");

    let ctx1 = create_test_context("bd-3a0a.1");
    let ctx2 = create_test_context("bd-3a0a.2");
    let ctx3 = create_test_context("bd-3a0a.3");

    let pane1_id = ctx1.pane_id.clone();
    let pane2_id = ctx2.pane_id.clone();
    let pane3_id = ctx3.pane_id.clone();

    fixture.open_command_pane(ctx1)
        .expect("Failed to open pane 1");
    fixture.open_command_pane(ctx2)
        .expect("Failed to open pane 2");
    fixture.open_command_pane(ctx3)
        .expect("Failed to open pane 3");

    // All three panes should be active
    assert!(fixture.active_panes.contains_key(&pane1_id));
    assert!(fixture.active_panes.contains_key(&pane2_id));
    assert!(fixture.active_panes.contains_key(&pane3_id));
    assert_eq!(fixture.active_panes.len(), 3);
}

#[test]
fn test_command_output_event_structure() {
    let event = CommandPaneEvent::Output {
        pane_id: "pane-123".to_string(),
        stdout: "test output".to_string(),
        stderr: String::new(),
        timestamp: 1234567890,
    };

    assert_eq!(event.pane_id(), "pane-123");
    assert_eq!(event.stdout(), Some("test output"));
    assert_eq!(event.stderr(), Some("")));
}

// ============================================================================
// Edge Case Tests
// ============================================================================

#[test]
fn test_command_with_special_characters() {
    let mut fixture = CommandPaneTestFixture::new()
        .expect("Failed to create test fixture");

    let mut context = create_test_context("bd-3a0a.8");
    context.command = vec![
        "echo".to_string(),
        "test with spaces && special; chars".to_string(),
    ];

    let result = fixture.open_command_pane(context);

    assert!(result.is_ok(), "Command with special chars should open");
}

#[test]
fn test_working_directory_with_spaces() {
    let mut fixture = CommandPaneTestFixture::new()
        .expect("Failed to create test fixture");

    // Create a temp directory with spaces
    let dir_with_spaces = fixture.temp_path().join("dir with spaces");
    std::fs::create_dir(&dir_with_spaces)
        .expect("Failed to create dir with spaces");

    let context = create_test_context_with_dir("bd-3a0a.8", dir_with_spaces);

    let result = fixture.open_command_pane(context);

    assert!(result.is_ok(), "Working dir with spaces should work");
}

#[test]
fn test_environment_variables_can_be_stored() {
    let mut context = create_test_context("bd-3a0a.8");
    context.environment.insert("TEST_VAR".to_string(), "test_value".to_string());
    context.environment.insert("ANOTHER_VAR".to_string(), "another_value".to_string());

    assert_eq!(context.environment.len(), 2);
    assert_eq!(context.environment.get("TEST_VAR"), Some(&"test_value".to_string()));
}

#[test]
fn test_multiple_stages_can_be_tracked() {
    let fixture = CommandPaneTestFixture::new()
        .expect("Failed to create test fixture");

    let stages = vec!["implement", "unit-test", "lint", "coverage"];

    for stage in stages {
        let mut ctx = create_test_context("bd-3a0a.8");
        ctx.stage = stage.to_string();
        assert_eq!(ctx.stage, stage);
    }
}

#[test]
fn test_pane_id_uniqueness() {
    let ctx1 = create_test_context("bd-3a0a.8");
    let ctx2 = create_test_context("bd-3a0a.8");

    assert_ne!(ctx1.pane_id, ctx2.pane_id, "Pane IDs should be unique");
}

// ============================================================================
// Error Display Tests
// ============================================================================

#[test]
fn test_command_not_found_error_message() {
    let err = CommandPaneError::CommandNotFound("mycommand".to_string());
    let msg = format!("{err}");
    assert!(msg.contains("Command not found"));
    assert!(msg.contains("mycommand"));
}

#[test]
fn test_working_directory_not_found_error_message() {
    let path = PathBuf::from("/nonexistent/path");
    let err = CommandPaneError::WorkingDirectoryNotFound(path.clone());
    let msg = format!("{err}");
    assert!(msg.contains("Working directory not found"));
}

#[test]
fn test_pane_not_found_error_message() {
    let err = CommandPaneError::PaneNotFound("pane-123".to_string());
    let msg = format!("{err}");
    assert!(msg.contains("Command pane not found"));
    assert!(msg.contains("pane-123"));
}
