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
use commands::{DoctorArgs, InitArgs, LogsArgs, doctor_command, init_command, logs_command};
use tracing::info;

/// OYA SDLC System - Storm goddess of transformation
#[derive(Parser, Debug)]
#[command(name = "oya")]
#[command(author = "Lewis Prior <lewis@lewisandquark.com>")]
#[command(version = "0.1.0")]
#[command(about = "100x developer throughput with AI agent swarms", long_about = None)]
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
}

impl Oya {
    #[allow(clippy::too_many_lines)]
    fn run(self) -> Result<()> {
        match self.command {
            Some(Commands::List) => {
                info!("Listing all beads");
                println!("List command not yet implemented");
                Ok(())
            }
            Some(Commands::Show { slug }) => {
                info!("Showing bead: {slug}");
                println!("Show command not yet implemented for bead: {slug}");
                Ok(())
            }
            Some(Commands::New { slug }) => {
                info!("Creating new task: {slug}");
                println!("New command not yet implemented for task: {slug}");
                Ok(())
            }
            Some(Commands::Stage { slug, stage }) => {
                info!("Running stage {stage} for task: {slug}");
                println!("Stage command not yet implemented for task: {slug}, stage: {stage}");
                Ok(())
            }
            Some(Commands::Approve { slug }) => {
                info!("Approving task: {slug}");
                println!("Approve command not yet implemented for task: {slug}");
                Ok(())
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
            None => {
                // No subcommand provided, show help
                Self::command().print_long_help()?;
                Ok(())
            }
        }
    }
}

fn main() {
    // Initialize tracing
    #[allow(clippy::option_if_let_else)] // match is clearer than unwrap_or_else here
    let env_filter = match tracing_subscriber::EnvFilter::try_from_default_env() {
        Ok(filter) => filter,
        Err(_) => tracing_subscriber::EnvFilter::new("info"),
    };

    tracing_subscriber::fmt().with_env_filter(env_filter).init();

    let oya = Oya::parse();

    // Run the CLI and handle errors with proper exit codes
    if let Err(error) = oya.run() {
        eprintln!("Error: {error}");
        std::process::exit(1);
    }
}
