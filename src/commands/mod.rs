//! Command implementations for OYA CLI
//!
//! Each command is implemented as a separate module following functional Rust principles:
//! - Pure core functions for business logic
//! - Imperative shell for I/O and async operations
//! - Railway-oriented error handling with thiserror/anyhow
//! - Zero unwrap/expect/panic throughout
//!
//! # Workspace Management
//!
//! Direct zjj CLI integration commands are in the `workspace` module:
//! - `workspace list`: List all sessions
//! - `workspace status <name>`: Show session status
//! - `workspace sync <name>`: Sync workspace with main
//! - `workspace done <name>`: Merge and cleanup
//! - `workspace add <name>`: Create manual session
//! - `workspace spawn <bead>`: Create agent session
//!
//! # BDD-Style Tests
//!
//! CLI validation tests are in `tests/cli_validation.rs` using Given-When-Then format.

pub mod approve;
pub mod doctor;
pub mod init;
pub mod install;
pub mod list;
pub mod logs;
pub mod new;
pub mod serve;
pub mod show;
pub mod stage;
pub mod storm;
pub mod workspace;

// Re-export command implementations and types for convenience
pub use approve::{approve_command, ApproveArgs};
pub use doctor::{doctor_command, CheckStatus, DoctorArgs};
pub use init::{init_command, InitArgs};
pub use install::install_command;
pub use list::{list_command, ListArgs};
pub use logs::{logs_command, LogsArgs};
pub use new::{new_command, NewArgs};
pub use serve::serve_command;
pub use show::{show_command, ShowArgs};
pub use stage::{stage_command, StageArgs};
pub use storm::{storm_command, StormArgs};
pub use workspace::{
    workspace_add_command, workspace_done_command, workspace_focus_command, workspace_list_command,
    workspace_remove_command, workspace_spawn_command, workspace_status_command,
    workspace_sync_command, AddArgs, DoneArgs, FocusArgs, RemoveArgs, SpawnArgs, StatusArgs,
    SyncArgs, WorkspaceArgs, WorkspaceCommand, WorkspaceListArgs,
};
