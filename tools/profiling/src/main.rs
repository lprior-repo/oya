#![deny(clippy::unwrap_used)]
#![deny(clippy::expect_used)]
#![deny(clippy::panic)]
#![warn(clippy::pedantic)]

//! CLI for memory profiling harness

use oya_profiling::{ProfilingConfig, ProfilingRunner};
use std::process;

fn main() {
    let result = run();

    match result {
        Ok(()) => process::exit(0),
        Err(e) => {
            eprintln!("Error: {e}");
            process::exit(1);
        }
    }
}

fn run() -> oya_profiling::Result<()> {
    // Parse command line arguments
    let args: Vec<String> = std::env::args().collect();

    if args.len() < 2 {
        print_usage();
        return Ok(());
    }

    let command = args[1].clone();
    let command_args = args[2..].to_vec();

    // Create configuration for 1-hour profiling
    let config = ProfilingConfig::one_hour_default(command, command_args)?;

    println!("=== Memory Profiling Harness ===");
    println!("Duration: 1 hour (3600s)");
    println!("Sampling interval: 10s");
    println!("Output: memory-profile.jsonl");
    println!("Command: {} {}", config.command(), config.args().join(" "));
    println!();
    println!("Starting profiling...");

    // Run profiling
    let runner = ProfilingRunner::new(config);
    let summary = runner.run()?;

    // Print summary
    println!();
    println!("=== Profiling Complete ===");
    println!("Samples collected: {}", summary.sample_count());
    println!(
        "Max RSS: {:.2} MB ({} KB)",
        summary.max_rss_mb(),
        summary.max_rss_kb()
    );
    println!(
        "Avg RSS: {:.2} MB ({} KB)",
        summary.avg_rss_mb(),
        summary.avg_rss_kb()
    );
    println!("Duration: {}s", summary.duration_secs());
    println!();
    println!("Metrics saved to: memory-profile.jsonl");

    Ok(())
}

fn print_usage() {
    eprintln!("Usage: oya-profiling <command> [args...]");
    eprintln!();
    eprintln!("Runs memory profiling with heaptrack for 1 hour, sampling RSS every 10s.");
    eprintln!();
    eprintln!("Examples:");
    eprintln!("  oya-profiling ./my-app --load-test");
    eprintln!("  oya-profiling cargo run --release");
    eprintln!();
    eprintln!("Requirements:");
    eprintln!("  - heaptrack must be installed and in PATH");
    eprintln!("  - Command must run for at least 1 hour");
    eprintln!();
    eprintln!("Output:");
    eprintln!("  Metrics are logged to memory-profile.jsonl (JSON lines format)");
}
