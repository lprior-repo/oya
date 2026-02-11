#![deny(clippy::unwrap_used)]
#![deny(clippy::expect_used)]
#![deny(clippy::panic)]
#![warn(clippy::pedantic)]
#![warn(clippy::nursery)]
#![forbid(unsafe_code)]

// OYA - Storm goddess of transformation
// 100x developer throughput with AI agent swarms

mod commands;

use anyhow::Result;
use clap::{CommandFactory, Parser, Subcommand};
use commands::{
    DoctorArgs, InitArgs, LogsArgs, StormArgs, doctor_command, init_command, install_command,
    logs_command, serve_command, storm_command,
};
use oya_telemetry::TelemetryConfig;
use tracing::info;

/// OYA SDLC System - Storm goddess of transformation
///
/// 100x developer throughput with AI agent swarms
///
/// # Examples
///
/// Create a new task:
///   oya new --slug my-feature
///
/// Run a pipeline stage:
///   oya stage --slug my-feature --stage implement
///
/// Approve a task for integration:
///   oya approve --slug my-feature
///
/// View workspace diagnostics:
///   oya doctor
///
/// Start the IPC server:
///   oya serve
///   oya serve --address 127.0.0.1:5555
#[derive(Parser, Debug)]
#[command(name = "oya")]
#[command(author = "Lewis Prior <lewis@lewisandquark.com>")]
#[command(version = "0.1.0")]
#[command(about = "100x developer throughput with AI agent swarms")]
#[command(long_about = "100x developer throughput with AI agent swarms

Examples:
  oya new --slug my-feature
  oya stage --slug my-feature --stage implement
  oya approve --slug my-feature
  oya doctor
  oya serve --address 127.0.0.1:5555")]
struct Oya {
    #[command(subcommand)]
    command: Option<Commands>,
}

#[derive(Subcommand, Debug)]
enum Commands {
    /// List all beads in the workspace
    List,
    /// Show details of a specific bead
    Show {
        /// Bead ID or slug
        slug: String,
    },
    /// Create a new task
    New {
        /// Slug for the task
        #[arg(short, long)]
        slug: String,
    },
    /// Run a pipeline stage
    Stage {
        /// Slug for the task
        #[arg(short, long)]
        slug: String,
        /// Stage name to run
        #[arg(short, long)]
        stage: String,
    },
    /// Approve a task for integration
    Approve {
        /// Slug for the task
        #[arg(short, long)]
        slug: String,
    },
    /// View and filter logs
    Logs(LogsArgs),
    /// Initialize a new project
    Init(InitArgs),
    /// Run workspace diagnostics
    Doctor(DoctorArgs),
    /// Orchestrate bead execution with workflow DAG
    Storm(StormArgs),
    /// Start the IPC server (background daemon)
    Serve {
        /// IPC server address (default: 127.0.0.1:5555)
        #[arg(short, long)]
        address: Option<String>,
    },
    /// Install Zellij WASM plugin
    Install {
        /// Force reinstall even if already installed
        #[arg(long)]
        force: bool,
    },
}

impl Oya {
    #[allow(clippy::too_many_lines)]
    #[tracing::instrument(skip(self))]
    fn run(self) -> Result<()> {
        match self.command {
            Some(Commands::List) => {
                info!("Listing all beads");
                println!("List command not yet implemented");
                eprintln!("Error: This command is not yet implemented");
                std::process::exit(1);
            }
            Some(Commands::Show { slug }) => {
                // Validate slug doesn't contain path traversal characters
                if slug.contains('/') || slug.contains("..") {
                    eprintln!("Error: Slug cannot contain path separators or traversal sequences");
                    eprintln!("Hint: Use a simple identifier like 'my-feature' or 'task-123'");
                    std::process::exit(2);
                }
                info!("Showing bead: {slug}");
                println!("Show command not yet implemented for bead: {slug}");
                eprintln!("Error: Show command is not yet implemented");
                std::process::exit(1);
            }
            Some(Commands::New { slug }) => {
                // Validate slug is not empty
                if slug.trim().is_empty() {
                    eprintln!("Error: Slug cannot be empty");
                    eprintln!("Hint: Provide a valid task slug, e.g., oya new --slug my-task");
                    std::process::exit(2);
                }
                // Validate slug doesn't contain path traversal characters
                if slug.contains('/') || slug.contains("..") {
                    eprintln!("Error: Slug cannot contain path separators or traversal sequences");
                    eprintln!("Hint: Use a simple identifier like 'my-feature' or 'fix-bug-123'");
                    std::process::exit(2);
                }
                info!("Creating new task: {slug}");
                println!("New command not yet implemented for task: {slug}");
                eprintln!("Error: New command is not yet implemented");
                std::process::exit(1);
            }
            Some(Commands::Stage { slug, stage }) => {
                // Validate slug doesn't contain path traversal characters
                if slug.contains('/') || slug.contains("..") {
                    eprintln!("Error: Slug cannot contain path separators or traversal sequences");
                    eprintln!("Hint: Use a simple identifier like 'my-feature' or 'task-123'");
                    std::process::exit(2);
                }
                info!("Running stage {stage} for task: {slug}");
                println!("Stage command not yet implemented for task: {slug}, stage: {stage}");
                eprintln!("Error: Stage command is not yet implemented");
                std::process::exit(1);
            }
            Some(Commands::Approve { slug }) => {
                // Validate slug doesn't contain path traversal characters
                if slug.contains('/') || slug.contains("..") {
                    eprintln!("Error: Slug cannot contain path separators or traversal sequences");
                    eprintln!("Hint: Use a simple identifier like 'my-feature' or 'task-123'");
                    std::process::exit(2);
                }
                info!("Approving task: {slug}");
                println!("Approve command not yet implemented for task: {slug}");
                eprintln!("Error: Approve command is not yet implemented");
                std::process::exit(1);
            }
            Some(Commands::Logs(args)) => {
                info!("Running logs command");
                let rt = tokio::runtime::Runtime::new()?;
                rt.block_on(async {
                    match logs_command(args).await {
                        Ok(output) => {
                            info!("Logs command completed: {} entries", output.entries.len());
                            Ok(())
                        }
                        Err(e) => {
                            eprintln!("Error: {e}");
                            if let Some(hint) = e.hint() {
                                eprintln!("Hint: {hint}");
                            }
                            std::process::exit(e.exit_code());
                        }
                    }
                })
            }
            Some(Commands::Init(args)) => {
                info!("Running init command");
                let rt = tokio::runtime::Runtime::new()?;
                rt.block_on(async {
                    match init_command(args).await {
                        Ok(output) => {
                            println!("Project created successfully at: {:?}", output.project_path);
                            println!("Files created: {}", output.files_created.len());
                            if output.git_initialized {
                                println!("Git repository initialized");
                            }
                            Ok(())
                        }
                        Err(e) => {
                            eprintln!("Error: {e}");
                            if let Some(hint) = e.hint() {
                                eprintln!("Hint: {hint}");
                            }
                            std::process::exit(e.exit_code());
                        }
                    }
                })
            }
            Some(Commands::Doctor(args)) => {
                info!("Running doctor command");
                let rt = tokio::runtime::Runtime::new()?;
                rt.block_on(async {
                    match doctor_command(args).await {
                        Ok(output) => {
                            println!("Workspace diagnostics:");
                            println!("Status: {:?}", output.status);
                            println!("{}", output.summary);
                            for check in output.checks {
                                println!(
                                    "  {}: {:?} - {}",
                                    check.name, check.status, check.message
                                );
                            }
                            // Exit with code 1 if status is not Passed
                            if output.status != crate::commands::CheckStatus::Passed {
                                eprintln!("Error: Workspace diagnostics failed");
                                std::process::exit(1);
                            }
                            Ok(())
                        }
                        Err(e) => {
                            eprintln!("Error: {e}");
                            if let Some(hint) = e.hint() {
                                eprintln!("Hint: {hint}");
                            }
                            std::process::exit(e.exit_code());
                        }
                    }
                })
            }
            Some(Commands::Storm(args)) => {
                info!("Running storm command");
                let rt = tokio::runtime::Runtime::new()?;
                let output_format = args.output.clone();
                rt.block_on(async {
                    match storm_command(args).await {
                        Ok(output) => {
                            if output_format == "json" {
                                match serde_json::to_string_pretty(&output) {
                                    Ok(json) => println!("{json}"),
                                    Err(_) => println!("{{}}"),
                                }
                            } else {
                                println!("Storm completed:");
                                println!("  Beads completed: {}", output.beads_completed);
                                println!("  Beads failed: {}", output.beads_failed);
                                println!("  Duration: {}ms", output.duration_ms);
                                if let Some(order) = output.planned_order {
                                    println!("  Planned order: {} beads", order.len());
                                }
                            }
                            Ok(())
                        }
                        Err(e) => {
                            eprintln!("Error: {e}");
                            if let Some(hint) = e.hint() {
                                eprintln!("Hint: {hint}");
                            }
                            std::process::exit(e.exit_code());
                        }
                    }
                })
            }
            Some(Commands::Serve { address }) => {
                info!("Starting IPC server");
                match serve_command(address) {
                    Ok(()) => Ok(()),
                    Err(e) => {
                        eprintln!("Error: {e}");
                        std::process::exit(1);
                    }
                }
            }
            Some(Commands::Install { force }) => {
                info!("Installing Zellij plugin");
                match install_command(force) {
                    Ok(()) => Ok(()),
                    Err(e) => {
                        eprintln!("Error: {e}");
                        std::process::exit(1);
                    }
                }
            }
            None => {
                // No subcommand provided, show help
                Self::command().print_long_help()?;
                Ok(())
            }
        }
    }
}

fn main() {
    let config = TelemetryConfig::new("oya-cli")
        .with_json_logging(false)
        .with_otel_enabled(false);

    let _guard = oya_telemetry::init_telemetry(&config);

    let oya = Oya::parse();

    // Run the CLI and handle errors with proper exit codes
    if let Err(error) = oya.run() {
        eprintln!("Error: {error}");
        std::process::exit(1);
    }
}
