use crate::metrics::{MetricsStore, SessionStatus};
use clap::{Parser, Subcommand};
use std::path::{Path, PathBuf};

#[derive(Parser)]
#[command(name = "quality-dashboard")]
#[command(about = "View quality gate metrics")]
pub struct Args {
    #[command(subcommand)]
    pub command: Commands,
}

#[derive(Subcommand)]
pub enum Commands {
    #[command(about = "Show metrics summary")]
    Summary,

    #[command(about = "Show recent sessions")]
    Sessions {
        #[arg(short, long, default_value = "10")]
        count: usize,
    },

    #[command(about = "Export metrics report")]
    Export {
        #[arg(short, long, default_value = "text")]
        format: String,
        #[arg(short, long)]
        output: Option<PathBuf>,
    },
}

/// Run the dashboard application.
///
/// # Errors
/// Returns an error if metrics export fails.
pub fn run(args: Args) -> Result<(), Box<dyn std::error::Error>> {
    let metrics_store = MetricsStore::new(&PathBuf::from("."));

    match args.command {
        Commands::Summary => {
            let summary = metrics_store.get_summary();
            print_summary(&summary);
        }

        Commands::Sessions { count } => {
            print_sessions(&metrics_store, count);
        }

        Commands::Export { format, output } => {
            let output_path = output.clone().unwrap_or_else(|| PathBuf::from("metrics-report.txt"));
            export_metrics(&metrics_store, &format, &output_path)?;
        }
    }

    Ok(())
}

pub fn print_summary(summary: &crate::metrics::MetricsSummary) {
    println!("\n━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    println!("  QUALITY GATE SUMMARY");
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    println!();
    println!("  Sessions:");
    println!("    Total: {}", summary.total_sessions);

    let total = summary.total_sessions;
    let passed_pct = if total > 0 {
        #[allow(clippy::cast_precision_loss)]
        {
            summary.passed_sessions as f64 / total as f64 * 100.0
        }
    } else {
        0.0
    };
    let failed_pct = if total > 0 {
        #[allow(clippy::cast_precision_loss)]
        {
            summary.failed_sessions as f64 / total as f64 * 100.0
        }
    } else {
        0.0
    };
    let escalated_pct = if total > 0 {
        #[allow(clippy::cast_precision_loss)]
        {
            summary.escalated_sessions as f64 / total as f64 * 100.0
        }
    } else {
        0.0
    };

    println!("    Passed: {} ({passed_pct:.1}%)", summary.passed_sessions);
    println!("    Failed: {} ({failed_pct:.1}%)", summary.failed_sessions);
    println!("    Escalated: {} ({escalated_pct:.1}%)", summary.escalated_sessions);
    println!();
    println!("  Performance:");
    println!("    Avg iterations to pass: {:.2}", summary.avg_iterations_to_pass);
}

pub fn print_sessions(_store: &MetricsStore, _count: usize) {
    println!("\n━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    println!("  RECENT SESSIONS");
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    println!();
    println!("  (Session listing currently limited to summary view)");
}

#[must_use]
pub const fn format_status(status: &SessionStatus) -> &'static str {
    match status {
        SessionStatus::Passed => "✓",
        SessionStatus::Failed => "✗",
        SessionStatus::Escalated => "!",
        SessionStatus::InProgress => "→",
    }
}

/// Export metrics to a file.
///
/// # Errors
/// Returns an error if writing to the output file fails.
pub fn export_metrics(
    store: &MetricsStore,
    format: &str,
    output: &Path,
) -> Result<(), Box<dyn std::error::Error>> {
    let summary = store.get_summary();

    let report = match format {
        "json" => serde_json::to_string_pretty(&summary)?,
        "text" => format_text_summary(&summary),
        _ => return Err("Unsupported format".into()),
    };

    std::fs::write(output, report)?;

    println!("✅ Exported metrics to {}", output.display());
    Ok(())
}

fn format_text_summary(summary: &crate::metrics::MetricsSummary) -> String {
    let total = summary.total_sessions;
    let passed_pct = if total > 0 {
        #[allow(clippy::cast_precision_loss)]
        {
            summary.passed_sessions as f64 / total as f64 * 100.0
        }
    } else {
        0.0
    };
    let failed_pct = if total > 0 {
        #[allow(clippy::cast_precision_loss)]
        {
            summary.failed_sessions as f64 / total as f64 * 100.0
        }
    } else {
        0.0
    };
    let escalated_pct = if total > 0 {
        #[allow(clippy::cast_precision_loss)]
        {
            summary.escalated_sessions as f64 / total as f64 * 100.0
        }
    } else {
        0.0
    };

    format!(
        "Quality Gate Metrics
====================

Total Sessions: {}
Passed: {} ({passed_pct:.1}%)
Failed: {} ({failed_pct:.1}%)
Escalated: {} ({escalated_pct:.1}%)

Avg Iterations to Pass: {:.2}
",
        summary.total_sessions,
        summary.passed_sessions,
        summary.failed_sessions,
        summary.escalated_sessions,
        summary.avg_iterations_to_pass
    )
}
