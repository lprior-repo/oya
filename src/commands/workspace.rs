//! OYA CLI zjj workspace management commands
//!
//! This module implements direct zjj CLI integration commands that were
//! referenced in the issue src-2rb1: "zjj integration exists but oya CLI has no zjj commands"
//!
//! # Current State
//!
//! The oya CLI already uses zjj indirectly via:
//! - `new` command: creates workspace via zjj spawn (in create_zjj_workspace)
//! - `doctor` command: checks zjj availability
//! - `workflow` crate: cleanup logic for orphaned sessions
//!
//! # Missing Direct Commands
//!
//! Based on `zjj --help` output, the following zjj commands should be added:
//! - `zjj list` → `oya workspace list`
//! - `oya workspace sync <name>` → sync session workspace with main
//! - `oya workspace done <name>` → merge workspace to main and cleanup
//! - `oya workspace focus <name>` → switch to session's Zellij tab
//! - `oya workspace status <name>` → show session status
//! - `oya workspace remove <name>` → remove a session
//! - `oya workspace add <name>` → create session for manual work
//! - `oya workspace spawn <bead>` → create session for automated agent work

use anyhow::Result;
use clap::{Parser, Subcommand};
use std::path::PathBuf;

/// Workspace management commands via zjj integration
#[derive(Parser, Debug, Clone)]
pub struct WorkspaceArgs {
    #[command(subcommand)]
    pub command: WorkspaceCommand,
}

/// Available workspace commands
#[derive(Subcommand, Debug, Clone)]
pub enum WorkspaceCommand {
    /// List all workspace sessions
    List(WorkspaceListArgs),
    /// Show status of a workspace session
    Status(StatusArgs),
    /// Sync a session's workspace with main branch
    Sync(SyncArgs),
    /// Merge workspace to main and cleanup session
    Done(DoneArgs),
    /// Remove a workspace session
    Remove(RemoveArgs),
    /// Add a workspace for manual work (you control tab)
    Add(AddArgs),
    /// Spawn a workspace for automated agent work on a bead
    Spawn(SpawnArgs),
    /// Focus on a session's Zellij tab
    Focus(FocusArgs),
}

/// Arguments for workspace list command
#[derive(Parser, Debug, Clone)]
pub struct WorkspaceListArgs {
    /// Repository root path (default: current directory)
    #[arg(long)]
    pub root: Option<PathBuf>,

    /// Format output as JSON
    #[arg(long)]
    pub json: bool,
}

/// Arguments for workspace status command
#[derive(Parser, Debug, Clone)]
pub struct StatusArgs {
    /// Session name to show status for
    #[arg(short, long)]
    pub name: String,

    /// Repository root path (default: current directory)
    #[arg(long)]
    pub root: Option<PathBuf>,

    /// Format output as JSON
    #[arg(long)]
    pub json: bool,
}

/// Arguments for workspace sync command
#[derive(Parser, Debug, Clone)]
pub struct SyncArgs {
    /// Session name to sync
    #[arg(short, long)]
    pub name: String,

    /// Repository root path (default: current directory)
    #[arg(long)]
    pub root: Option<PathBuf>,

    /// Force sync (rebase even with conflicts)
    #[arg(long)]
    pub force: bool,
}

/// Arguments for workspace done command
#[derive(Parser, Debug, Clone)]
pub struct DoneArgs {
    /// Session name to complete
    #[arg(short, long)]
    pub name: String,

    /// Repository root path (default: current directory)
    #[arg(long)]
    pub root: Option<PathBuf>,

    /// Skip git push (for testing)
    #[arg(long)]
    pub no_push: bool,
}

/// Arguments for workspace remove command
#[derive(Parser, Debug, Clone)]
pub struct RemoveArgs {
    /// Session name to remove
    #[arg(short, long)]
    pub name: String,

    /// Repository root path (default: current directory)
    #[arg(long)]
    pub root: Option<PathBuf>,

    /// Force removal (don't ask for confirmation)
    #[arg(long)]
    pub force: bool,
}

/// Arguments for workspace add command
#[derive(Parser, Debug, Clone)]
pub struct AddArgs {
    /// Session name to create
    #[arg(short, long)]
    pub name: String,

    /// Repository root path (default: current directory)
    #[arg(long)]
    pub root: Option<PathBuf>,
}

/// Arguments for workspace spawn command
#[derive(Parser, Debug, Clone)]
pub struct SpawnArgs {
    /// Bead ID to work on (e.g., "bd-123")
    #[arg(short, long)]
    pub bead: String,

    /// Repository root path (default: current directory)
    #[arg(long)]
    pub root: Option<PathBuf>,
}

/// Arguments for workspace focus command
#[derive(Parser, Debug, Clone)]
pub struct FocusArgs {
    /// Session name to focus
    #[arg(short, long)]
    pub name: String,

    /// Repository root path (default: current directory)
    #[arg(long)]
    pub root: Option<PathBuf>,
}

/// Run zjj command with given arguments
async fn run_zjj_command(args: &[&str], root: &PathBuf) -> Result<String> {
    use tokio::process::Command;

    let zjj_path = std::env::var("ZJJ_PATH").unwrap_or_else(|_| "zjj".to_string());
    let mut cmd = Command::new(&zjj_path);
    cmd.args(args);

    // Set working directory to repo root
    cmd.current_dir(root);

    let output = cmd.output().await?;

    if output.status.success() {
        let stdout = String::from_utf8_lossy(&output.stdout);
        Ok(stdout.trim().to_string())
    } else {
        let stderr = String::from_utf8_lossy(&output.stderr);
        Err(anyhow::anyhow!("zjj command failed: {}", stderr))
    }
}

/// List all workspace sessions
pub async fn workspace_list_command(args: WorkspaceListArgs) -> Result<String> {
    let root = args.root.unwrap_or_else(|| PathBuf::from("."));
    let output = run_zjj_command(&["list", "--json"], &root).await?;
    Ok(output)
}

/// Show status of a workspace session
pub async fn workspace_status_command(args: StatusArgs) -> Result<String> {
    let root = args.root.unwrap_or_else(|| PathBuf::from("."));
    let output = run_zjj_command(&["status", &args.name], &root).await?;
    Ok(output)
}

/// Sync a session's workspace with main branch
pub async fn workspace_sync_command(args: SyncArgs) -> Result<String> {
    let root = args.root.unwrap_or_else(|| PathBuf::from("."));
    let mut zjj_args = vec!["sync", &args.name];
    if args.force {
        zjj_args.push("--force");
    }
    let output = run_zjj_command(&zjj_args, &root).await?;
    Ok(output)
}

/// Merge workspace to main and cleanup session
pub async fn workspace_done_command(args: DoneArgs) -> Result<String> {
    let root = args.root.unwrap_or_else(|| PathBuf::from("."));
    let mut zjj_args = vec!["done", &args.name];
    if args.no_push {
        zjj_args.push("--no-push");
    }
    let output = run_zjj_command(&zjj_args, &root).await?;
    Ok(output)
}

/// Remove a workspace session
pub async fn workspace_remove_command(args: RemoveArgs) -> Result<String> {
    let root = args.root.unwrap_or_else(|| PathBuf::from("."));
    let mut zjj_args = vec!["remove", &args.name];
    if args.force {
        zjj_args.push("--force");
    }
    let output = run_zjj_command(&zjj_args, &root).await?;
    Ok(output)
}

/// Add a workspace for manual work
pub async fn workspace_add_command(args: AddArgs) -> Result<String> {
    let root = args.root.unwrap_or_else(|| PathBuf::from("."));
    let output = run_zjj_command(&["add", &args.name], &root).await?;
    Ok(output)
}

/// Spawn a workspace for automated agent work
pub async fn workspace_spawn_command(args: SpawnArgs) -> Result<String> {
    let root = args.root.unwrap_or_else(|| PathBuf::from("."));
    let output = run_zjj_command(&["spawn", &args.bead], &root).await?;
    Ok(output)
}

/// Focus on a session's Zellij tab
pub async fn workspace_focus_command(args: FocusArgs) -> Result<String> {
    let root = args.root.unwrap_or_else(|| PathBuf::from("."));
    let output = run_zjj_command(&["focus", &args.name], &root).await?;
    Ok(output)
}
