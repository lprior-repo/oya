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

    /// Enable verbose logging
    #[arg(short, long, global = true)]
    pub verbose: bool,

    /// Quiet mode (minimal output)
    #[arg(short, long, global = true)]
    pub quiet: bool,
}

impl Cli {
    /// Parse command-line arguments from environment.
    ///
    /// This is a convenience method that uses std::env::args_os().
    pub fn parse() -> Self {
        <Cli as clap::Parser>::parse()
    }

    /// Parse from iterator.
    pub fn parse_from<I, T>(itr: I) -> Self
    where
        I: IntoIterator<Item = T>,
        T: Into<std::ffi::OsString> + Clone,
    {
        <Cli as clap::Parser>::parse_from(itr)
    }
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

    /// Build the project with Moon
    Build {
        /// Number of parallel jobs
        #[arg(short, long, default_value_t = 4)]
        parallel: usize,

        /// Run release build instead of debug
        #[arg(long, default_value_t = false)]
        release: bool,

        /// Specific target to build (defaults to all)
        #[arg(short, long)]
        target: Option<String>,
    },

    /// Run tests with parallel execution
    Test {
        /// Enable swarm mode (massive parallelism)
        #[arg(long)]
        swarm: bool,
    },

    /// Refactor codebase with automated transformations
    Refactor {
        /// Force refactor without confirmation prompts
        #[arg(long)]
        force: bool,
    },

    /// Deploy validated changes
    Deploy {
        /// Deploy without safety checks (dangerous)
        #[arg(long)]
        no_mercy: bool,
    },

    /// Run quality gates (validation checks)
    Gate {
        /// Enable strict mode - fail on any warning
        #[arg(long)]
        strict: bool,
    },

    /// Manage agent pool
    Agents {
        /// API server URL (defaults to http://localhost:3000)
        #[arg(long)]
        server: Option<String>,

        #[command(subcommand)]
        command: AgentCommands,
    },

    /// Run swarm mode (13-agent continuous assembly line)
    Swarm {
        /// Target number of beads to complete [default: 25]
        #[arg(long, default_value_t = 25)]
        target: usize,

        /// Number of Test Writer agents [default: 4]
        #[arg(long, default_value_t = 4)]
        test_writers: usize,

        /// Number of Implementer agents [default: 4]
        #[arg(long, default_value_t = 4)]
        implementers: usize,

        /// Number of Reviewer agents [default: 4]
        #[arg(long, default_value_t = 4)]
        reviewers: usize,

        /// Enable Planner agent (contract-first development) [default: true]
        #[arg(long, default_value_t = true)]
        planner: bool,

        /// Enable continuous-deployment principles [default: true, CANNOT DISABLE]
        #[arg(long, default_value_t = true)]
        continuous_deployment: bool,

        /// Dry run mode (preview without execution)
        #[arg(long, default_value_t = false)]
        dry_run: bool,

        /// Resume from previous session
        #[arg(long)]
        resume: Option<String>,

        /// Output format [default: text]
        #[arg(long, default_value = "text")]
        format: String,
    },
}

#[derive(Subcommand, Debug)]
pub enum AgentCommands {
    /// Spawn new agents
    Spawn {
        /// Number of agents to spawn
        #[arg(long)]
        count: usize,
    },

    /// Scale agent pool to target size
    Scale {
        /// Target total agents
        #[arg(long)]
        target: usize,
    },

    /// List agents
    List,
}
