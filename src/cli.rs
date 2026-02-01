//! CLI command definitions using clap.

#![deny(clippy::unwrap_used)]
#![deny(clippy::expect_used)]
#![deny(clippy::panic)]

use clap::{Parser, Subcommand};

/// OYA - SDLC System
#[derive(Parser, Debug)]
#[command(name = "oya")]
#[command(version)]
#[command(
    about = "Storm goddess of transformation - 100x developer throughput with AI agent swarms"
)]
#[command(
    long_about = "OYA manages isolated workspaces, runs pipeline stages, and tracks task progress across multiple programming languages."
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

    /// Run a stage with AI assistance (OpenCode)
    AiStage {
        /// Task slug
        #[arg(short, long)]
        slug: String,

        /// Stage name to run (implement, test, review, refactor, document)
        #[arg(long)]
        stage: String,

        /// Custom prompt/input for the AI
        #[arg(short, long)]
        prompt: Option<String>,

        /// Files to include in context
        #[arg(short, long)]
        files: Vec<String>,
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

    /// Say hello to the world
    Hello {
        /// Custom greeting message
        #[arg(short, long, default_value = "Hello, World!")]
        message: String,
    },
}
