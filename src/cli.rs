//! CLI command definitions using clap.

#![deny(clippy::unwrap_used)]
#![deny(clippy::expect_used)]
#![deny(clippy::panic)]

use clap::{Parser, Subcommand};

/// Factory - Contract-driven CI/CD Pipeline
#[derive(Parser, Debug)]
#[command(name = "factory")]
#[command(version)]
#[command(about = "Contract-driven CI/CD pipeline for software projects")]
#[command(
    long_about = "Factory manages isolated workspaces, runs pipeline stages, and tracks task progress across multiple programming languages."
)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Commands,
}

#[derive(Subcommand, Debug)]
pub enum Commands {
    /// Create a new task with isolated worktree
    New {
        /// Task slug (identifier)
        #[arg(short, long)]
        slug: String,

        /// Contract file path
        #[arg(short, long)]
        contract: Option<String>,

        /// Enable interactive mode
        #[arg(short, long, default_value_t = false)]
        interactive: bool,
    },

    /// Run a pipeline stage
    Stage {
        /// Task slug
        #[arg(short, long)]
        slug: String,

        /// Stage name to run
        #[arg(long)]
        stage: String,

        /// Dry run mode (preview only)
        #[arg(short, long, default_value_t = false)]
        dry_run: bool,

        /// Start stage for range
        #[arg(long)]
        from: Option<String>,

        /// End stage for range
        #[arg(long)]
        to: Option<String>,
    },

    /// Approve task for deployment
    Approve {
        /// Task slug
        #[arg(short, long)]
        slug: String,

        /// Deployment strategy (immediate, gradual, canary)
        #[arg(long)]
        strategy: Option<String>,

        /// Force approval without checks
        #[arg(short, long, default_value_t = false)]
        force: bool,
    },

    /// Show task details
    Show {
        /// Task slug
        #[arg(short, long)]
        slug: String,

        /// Show detailed information
        #[arg(short, long, default_value_t = false)]
        detailed: bool,
    },

    /// List all tasks
    List {
        /// Filter by priority (P1, P2, P3)
        #[arg(short, long)]
        priority: Option<String>,

        /// Filter by status (open, in-progress, done)
        #[arg(long)]
        status: Option<String>,
    },

    /// Show help information
    Help {
        /// Topic to get help on
        topic: Option<String>,
    },
}

/// Help text for the CLI.
pub const HELP_TEXT: &str = r"Factory - Contract-driven CI/CD Pipeline

USAGE:
  factory <COMMAND> [FLAGS]

COMMANDS:
  new      Create new task
           factory new -s bd-52.1

  stage    Run pipeline stage
           factory stage -s bd-52.1 --stage implement [-d] [--from X] [--to Y]

  approve  Approve for deployment
           factory approve -s bd-52.1 [--strategy immediate|gradual|canary] [-f]

  show     Show task details
           factory show -s bd-52.1 [--detailed]

  list     List all tasks
           factory list [--priority P1|P2|P3] [--status open|in_progress|done]

  help     Show this help [--topic COMMAND]

SHORT FLAGS:
  -s       --slug
  -d       --dry-run
  -f       --force

EXAMPLES:
  factory new -s bd-52.1
  factory stage -s bd-52.1 --stage implement -d
  factory approve -s bd-52.1 --strategy gradual
  factory list --priority P1

Documentation: ./ARCHITECTURE.md";
