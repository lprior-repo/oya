//! Placeholder for the `oya logs` command.
//!
//! This command is registered even though the implementation is still a stub so the
//! CLI can compile while we grow the feature incrementally.

use anyhow::Result;
use clap::Parser;

/// Arguments for the logs command.
#[derive(Parser, Debug, Clone)]
pub struct LogsArgs {
    /// Number of log lines to emit (tail).
    #[arg(long, default_value_t = 100)]
    pub tail: usize,

    /// Follow new log lines as they arrive.
    #[arg(long)]
    pub follow: bool,
}

/// Execute the logs command.
pub async fn logs_command(args: LogsArgs) -> Result<()> {
    println!("oya logs is not implemented yet (tail={}).", args.tail);

    if args.follow {
        println!("Following logs is not implemented yet.");
    }

    Ok(())
}
