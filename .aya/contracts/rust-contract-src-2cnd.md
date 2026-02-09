# Rust Contract: Integration Test Command Panes

**Bead ID:** `src-2cnd`
**Priority:** P2
**Size:** small
**Generated:** 2026-02-09
**Type:** Feature

## Overview

This bead implements integration tests for Zellij command pane functionality in the oya-orchestrator system. Command panes provide interactive command execution for pipeline stages (implement, test, lint, etc.).

Integration scope:
- Test command pane lifecycle (open, execute, close)
- Test IPC message flow for command pane events
- Test error handling and edge cases
- Validate command pane <-> orchestrator integration

## Domain Terms

- **CommandPane**: Interactive Zellij pane for running shell commands during pipeline stages
- **StageRunner**: Orchestrator actor that executes pipeline stages (implement, unit-test, coverage, lint, static, integration, security, review, accept)
- **CommandPaneContext**: Metadata about command (bead_id, stage, working_dir, command string)
- **CommandPaneLifecycle**: Events: Opened → Running → Exited with exit_code
- **IpcWorker**: Actor that bridges EventBus events to Zellij plugin via IPC

## Functional Requirements

### Core Functionality

Implement integration test suite that validates:

1. **Command Pane Context Tracking**
   - Create command context with bead_id, stage, working_dir
   - Serialize/deserialize context via IPC
   - Track multiple active command panes

2. **Command Execution Lifecycle**
   - Open command pane with context
   - Execute command in subprocess
   - Capture stdout/stderr
   - Report exit code to orchestrator

3. **IPC Message Flow**
   - Guest → Host: OpenCommandPane, CloseCommandPane, CommandPaneReRun
   - Host → Guest: CommandPaneOpened, CommandPaneOutput, CommandPaneExited

4. **Error Handling**
   - Command not found
   - Permission denied
   - Timeout
   - Invalid working directory

## API Surface

```rust
// Command pane context (shared between orchestrator and plugin)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CommandPaneContext {
    pub pane_id: String,
    pub bead_id: String,
    pub stage: String,
    pub working_dir: std::path::PathBuf,
    pub command: Vec<String>,
    pub environment: HashMap<String, String>,
}

// Command pane lifecycle events (Host → Guest)
#[derive(Debug, Clone, Serialize, Deserialize)]
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

// Command pane commands (Guest → Host)
#[derive(Debug, Clone, Serialize, Deserialize)]
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

// Test fixture for integration tests
pub struct CommandPaneTestFixture {
    temp_dir: tempfile::TempDir,
    orchestrator_handle: Option<String>,
    zellij_process: Option<tokio::process::Child>,
}

impl CommandPaneTestFixture {
    pub fn new() -> Result<Self, TestFixtureError>;
    pub fn spawn_orchestrator(&mut self) -> Result<(), TestFixtureError>;
    pub fn spawn_zellij_plugin(&mut self) -> Result<(), TestFixtureError>;
    pub fn send_command(&self, cmd: CommandPaneCommand) -> Result<(), TestFixtureError>;
    pub fn recv_event(&mut self) -> Result<CommandPaneEvent, TestFixtureError>;
    pub fn cleanup(mut self) -> Result<(), TestFixtureError>;
}
```

## Input/Output Specifications

| Input | Type | Validation | Output |
|-------|------|------------|--------|
| `CommandPaneContext::command` | `Vec<String>` | Non-empty, executable exists | CommandPaneOpened |
| `CommandPaneContext::working_dir` | `PathBuf` | Directory exists, readable | CommandPaneOpened |
| `CommandPaneCommand::Open` | Command | All fields valid | CommandPaneOpened/Failed |
| `CommandPaneCommand::Close` | pane_id | Pane exists | CommandPaneExited |
| `CommandPaneCommand::ReRun` | pane_id | Pane exists, has context | CommandPaneOpened |

## Error Handling

### Error Types

```rust
use thiserror::Error;

#[derive(Error, Debug)]
pub enum CommandPaneError {
    #[error("Command not found: {0}")]
    CommandNotFound(String),

    #[error("Working directory not found: {0}")]
    WorkingDirectoryNotFound(std::path::PathBuf),

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
}
```

### Error Propagation Strategy

- **Zero panics**: All error paths use `Result<T, CommandPaneError>`
- **Zero unwraps**: Forbidden in production code
- **Railway-Oriented Programming**: Use `?` operator throughout
- **Test coverage**: All error variants must have tests

## Implementation Details

### Test Structure

```
crates/zellij-frontend/tests/
├── command_pane_integration_test.rs
│   ├── mod command_pane_lifecycle
│   │   ├── test_open_command_pane_with_valid_context
│   │   ├── test_open_command_pane_with_invalid_directory_fails
│   │   ├── test_open_command_pane_with_nonexistent_command_fails
│   │   ├── test_close_command_pane_sends_exited_event
│   │   └── test_rerun_command_reopens_pane_with_same_context
│   ├── mod ipc_message_flow
│   │   ├── test_open_command_sends_opened_event
│   │   ├── test_command_output_streams_to_plugin
│   │   ├── test_command_failure_sends_failed_event
│   │   └── test_multiple_command_panes_can_be_active
│   └── mod error_handling
│       ├── test_command_timeout_is_handled
│       ├── test_permission_denied_returns_error
│       └── test_malformed_ipc_message_is_rejected
```

### Test Utilities

```rust
// Helper to create valid command context
pub fn create_test_context(bead_id: &str) -> CommandPaneContext {
    CommandPaneContext {
        pane_id: format!("pane-{}", uuid::Uuid::new_v4()),
        bead_id: bead_id.to_string(),
        stage: "implement".to_string(),
        working_dir: std::env::current_dir().unwrap(),
        command: vec!["echo".to_string(), "test".to_string()],
        environment: HashMap::new(),
    }
}

// Helper to wait for event with timeout
pub async fn wait_for_event(
    rx: &mut tokio::sync::mpsc::Receiver<CommandPaneEvent>,
    timeout: Duration,
) -> Result<CommandPaneEvent, TestFixtureError> {
    tokio::time::timeout(timeout, rx.recv())
        .await
        .map_err(|_| TestFixtureError::Timeout)?
        .ok_or(TestFixtureError::ChannelClosed)
}
```

## Testing Requirements

### Martin Fowler Test Categories

#### 1. Lifecycle Tests (Command → Result)
- ✅ Open command pane with valid context → Opened event
- ✅ Execute successful command → Output + Exited(0)
- ✅ Execute failing command → Output + Exited(non-zero)
- ✅ Close command pane → Exited event
- ✅ Re-run command → Opened event with same context

#### 2. Error Path Tests (Failure → Error)
- ✅ Open with invalid directory → Failed event
- ✅ Open with non-existent command → Failed event
- ✅ Open without permissions → Failed event
- ✅ Close non-existent pane → Error response
- ✅ Re-run without context → Error response

#### 3. Integration Tests (End-to-End)
- ✅ Open → Execute → Close lifecycle
- ✅ Multiple concurrent command panes
- ✅ IPC message round-trip (Guest → Host → Guest)
- ✅ Event streaming (multiple Output events)

#### 4. Edge Cases
- ✅ Empty command vector
- ✅ Command with special characters
- ✅ Very long output (>1MB)
- ✅ Environment variable expansion
- ✅ Working directory with spaces

### Test Execution

```bash
# Run all command pane integration tests
moon run :test -- crates/zellij-frontend/tests/command_pane_integration_test.rs

# Run specific test suite
moon run :test -- --test command_pane_integration_test lifecycle

# Run with logging
RUST_LOG=debug moon run :test -- --test command_pane_integration_test
```

## Integration Points

### Upstream Dependencies

- **oya-orchestrator**: StageRunner executes commands
- **oya-ipc**: IPC message types (extend HostMessage/GuestMessage)
- **zellij-frontend**: Command pane event handling

### Downstream Consumers

- **Zellij plugin**: Displays command output in UI
- **StageRunner**: Receives command pane events to update stage state

## Documentation Requirements

- [x] Public API documentation for CommandPaneContext
- [x] Integration test guide (how to run, what's tested)
- [x] Error handling documentation
- [x] IPC protocol documentation (message formats)

## Non-Functional Requirements

### Reliability

- Commands must execute in isolated subprocesses (no parent pollution)
- Command panes must be tracked and cleaned up on exit
- IPC failures must not crash the orchestrator

### Performance

- Command output streaming must be low-latency (<100ms per chunk)
- Multiple command panes must run concurrently without blocking
- Test suite must complete in <30 seconds

### Security

- Commands execute in sandboxed working directory
- Environment variables are explicitly controlled
- No shell injection (command vector, not string)

## Acceptance Criteria

1. [ ] All lifecycle tests pass (5 tests)
2. [ ] All error path tests pass (5 tests)
3. [ ] All integration tests pass (4 tests)
4. [ ] All edge case tests pass (5 tests)
5. [ ] Zero panics, zero unwraps in test code
6. [ ] Test coverage >90% for command pane code
7. [ ] Documentation complete
8. [ ] `moon run :ci` passes

---

*Generated by Autonomous Agent #3*
*Contract status: COMPLETE - Ready for implementation*
