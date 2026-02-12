//! OYA CLI - Storm goddess of transformation
//!
//! 100x developer throughput with AI agent swarms

use anyhow::Result;
use clap::{Parser, Subcommand};
use tracing::info;

use oya::commands::{
    ApproveArgs, ListArgs, NewArgs, ShowArgs, StageArgs, WorkspaceArgs, WorkspaceCommand,
    approve_command, list_command, new_command, show_command, stage_command, workspace_add_command,
    workspace_done_command, workspace_focus_command, workspace_list_command,
    workspace_remove_command, workspace_spawn_command, workspace_status_command,
    workspace_sync_command,
};

#[tokio::main]
async fn main() -> Result<()> {
    let oya = Oya::parse();

    match oya.command {
        Some(Commands::List(args)) => {
            let result = list_command(args).await?;
            info!("Listed {} tasks", result.total);
            Ok(())
        }
        Some(Commands::Show(args)) => {
            let result = show_command(args).await?;
            info!("Showed task: {}", result.task.slug);
            Ok(())
        }
        Some(Commands::New(args)) => {
            let result = new_command(args).await?;
            info!("Created task: {}", result.task.slug);
            if let Some(workspace) = result.workspace_path {
                info!("Workspace: {}", workspace);
            }
            Ok(())
        }
        Some(Commands::Stage(args)) => {
            let result = stage_command(args).await?;
            info!("{}", result.report);
            Ok(())
        }
        Some(Commands::Approve(args)) => {
            let result = approve_command(args).await?;
            info!("Approved task: {}", result.task.slug);
            Ok(())
        }
        Some(Commands::Workspace(args)) => match args.command {
            WorkspaceCommand::List(a) => {
                let result = workspace_list_command(a).await?;
                info!("Workspace list retrieved");
                println!("{}", result);
                Ok(())
            }
            WorkspaceCommand::Status(a) => {
                let result = workspace_status_command(a).await?;
                info!("Status for workspace: {}", result);
                println!("{}", result);
                Ok(())
            }
            WorkspaceCommand::Sync(a) => {
                let result = workspace_sync_command(a).await?;
                info!("Synced workspace: {}", result);
                println!("{}", result);
                Ok(())
            }
            WorkspaceCommand::Done(a) => {
                let result = workspace_done_command(a).await?;
                info!("Completed workspace: {}", result);
                println!("{}", result);
                Ok(())
            }
            WorkspaceCommand::Remove(a) => {
                let result = workspace_remove_command(a).await?;
                info!("Removed workspace: {}", result);
                println!("{}", result);
                Ok(())
            }
            WorkspaceCommand::Add(a) => {
                let result = workspace_add_command(a).await?;
                info!("Added workspace: {}", result);
                println!("{}", result);
                Ok(())
            }
            WorkspaceCommand::Spawn(a) => {
                let result = workspace_spawn_command(a).await?;
                info!("Spawned workspace for bead: {}", result);
                println!("{}", result);
                Ok(())
            }
            WorkspaceCommand::Focus(a) => {
                let result = workspace_focus_command(a).await?;
                info!("Focused on workspace: {}", result);
                println!("{}", result);
                Ok(())
            }
        },
        None => {
            println!("OYA CLI - Storm goddess of transformation");
            println!("Use --help for more information");
            Ok(())
        }
    }
}

/// OYA SDLC System - Storm goddess of transformation
#[derive(Parser, Debug)]
#[command(name = "oya")]
#[command(version = "0.1.0")]
#[command(about = "100x developer throughput with AI agent swarms", long_about = None)]
#[command(long_about = "100x developer throughput with AI agent swarms

 Examples:
   oya new --slug my-feature
   oya stage --slug my-feature --stage implement
   oya approve --slug my-feature
   oya list
   oya show --slug my-feature

 Workspace Commands:
   oya workspace list
   oya workspace status <name>
   oya workspace sync <name>
   oya workspace done <name>
   oya workspace add <name>
   oya workspace spawn <bead>
   oya workspace focus <name>
   oya workspace remove <name>")]
struct Oya {
    #[command(subcommand)]
    command: Option<Commands>,
}

#[derive(Subcommand, Debug)]
enum Commands {
    /// List all tasks in the workspace
    List(ListArgs),
    /// Show details of a specific task
    Show(ShowArgs),
    /// Create a new task
    New(NewArgs),
    /// Run a pipeline stage
    Stage(StageArgs),
    /// Approve a task for integration
    Approve(ApproveArgs),
    /// Manage workspace sessions via zjj
    Workspace(WorkspaceArgs),
}
