//! ZJJ CLI - JJ workspace + Zellij session manager
//!
//! Binary name: `jjz`

use std::process;

use anyhow::Result;
use clap::{Arg, Command as ClapCommand};

mod cli;
mod commands;
mod db;
mod json_output;
mod session;

use commands::{
    add, config, dashboard, diff, doctor, focus, init, introspect, list, query, remove, status,
    sync,
};

fn cmd_init() -> ClapCommand {
    ClapCommand::new("init")
        .about("Initialize jjz in a JJ repository (or create one)")
        .arg(
            Arg::new("json")
                .long("json")
                .action(clap::ArgAction::SetTrue)
                .help("Output as JSON"),
        )
}

fn cmd_add() -> ClapCommand {
    ClapCommand::new("add")
        .about("Create a new session with JJ workspace + Zellij tab")
        .arg(
            Arg::new("name")
                .required(true)
                .allow_hyphen_values(true) // Allow -name to be passed through for validation
                .help("Name for the new session (must start with a letter)"),
        )
        .arg(
            Arg::new("no-hooks")
                .long("no-hooks")
                .action(clap::ArgAction::SetTrue)
                .help("Skip executing post_create hooks"),
        )
        .arg(
            Arg::new("template")
                .short('t')
                .long("template")
                .value_name("TEMPLATE")
                .help("Zellij layout template to use (minimal, standard, full)"),
        )
        .arg(
            Arg::new("no-open")
                .long("no-open")
                .action(clap::ArgAction::SetTrue)
                .help("Create workspace without opening Zellij tab"),
        )
        .arg(
            Arg::new("json")
                .long("json")
                .action(clap::ArgAction::SetTrue)
                .help("Output as JSON"),
        )
}

fn cmd_list() -> ClapCommand {
    ClapCommand::new("list")
        .about("List all sessions")
        .arg(
            Arg::new("all")
                .long("all")
                .action(clap::ArgAction::SetTrue)
                .help("Include completed and failed sessions"),
        )
        .arg(
            Arg::new("json")
                .long("json")
                .action(clap::ArgAction::SetTrue)
                .help("Output as JSON"),
        )
}

fn cmd_remove() -> ClapCommand {
    ClapCommand::new("remove")
        .about("Remove a session and its workspace")
        .arg(
            Arg::new("name")
                .required(true)
                .allow_hyphen_values(true) // Allow -name to be passed through for validation
                .help("Name of the session to remove"),
        )
        .arg(
            Arg::new("force")
                .short('f')
                .long("force")
                .action(clap::ArgAction::SetTrue)
                .help("Skip confirmation prompt and hooks"),
        )
        .arg(
            Arg::new("merge")
                .short('m')
                .long("merge")
                .action(clap::ArgAction::SetTrue)
                .help("Squash-merge to main before removal"),
        )
        .arg(
            Arg::new("keep-branch")
                .short('k')
                .long("keep-branch")
                .action(clap::ArgAction::SetTrue)
                .help("Preserve branch after removal"),
        )
        .arg(
            Arg::new("json")
                .long("json")
                .action(clap::ArgAction::SetTrue)
                .help("Output as JSON"),
        )
}

fn cmd_focus() -> ClapCommand {
    ClapCommand::new("focus")
        .about("Switch to a session's Zellij tab")
        .arg(
            Arg::new("name")
                .required(true)
                .allow_hyphen_values(true) // Allow -name to be passed through for validation
                .help("Name of the session to focus"),
        )
        .arg(
            Arg::new("json")
                .long("json")
                .action(clap::ArgAction::SetTrue)
                .help("Output as JSON"),
        )
}

fn cmd_status() -> ClapCommand {
    ClapCommand::new("status")
        .about("Show detailed session status")
        .arg(
            Arg::new("name")
                .required(false)
                .help("Session name to show status for (shows all if omitted)"),
        )
        .arg(
            Arg::new("json")
                .long("json")
                .action(clap::ArgAction::SetTrue)
                .help("Output as JSON"),
        )
        .arg(
            Arg::new("watch")
                .long("watch")
                .action(clap::ArgAction::SetTrue)
                .help("Continuously update status (1s refresh)"),
        )
}

fn cmd_sync() -> ClapCommand {
    ClapCommand::new("sync")
        .about("Sync a session's workspace with main (rebase)")
        .arg(
            Arg::new("name")
                .required(false)
                .help("Session name to sync (syncs current workspace if omitted)"),
        )
        .arg(
            Arg::new("json")
                .long("json")
                .action(clap::ArgAction::SetTrue)
                .help("Output as JSON"),
        )
}

fn cmd_diff() -> ClapCommand {
    ClapCommand::new("diff")
        .about("Show diff between session and main branch")
        .arg(
            Arg::new("name")
                .required(true)
                .allow_hyphen_values(true) // Allow -name to be passed through for validation
                .help("Session name to show diff for"),
        )
        .arg(
            Arg::new("stat")
                .long("stat")
                .action(clap::ArgAction::SetTrue)
                .help("Show diffstat only (summary of changes)"),
        )
        .arg(
            Arg::new("json")
                .long("json")
                .action(clap::ArgAction::SetTrue)
                .help("Output as JSON"),
        )
}

fn cmd_config() -> ClapCommand {
    ClapCommand::new("config")
        .alias("cfg")
        .about("View or modify configuration")
        .arg(Arg::new("key").help("Config key to view/set (dot notation: 'zellij.use_tabs')"))
        .arg(Arg::new("value").help("Value to set (omit to view)"))
        .arg(
            Arg::new("global")
                .long("global")
                .short('g')
                .action(clap::ArgAction::SetTrue)
                .help("Operate on global config instead of project"),
        )
}

fn cmd_dashboard() -> ClapCommand {
    ClapCommand::new("dashboard")
        .about("Launch interactive TUI dashboard with kanban view")
        .alias("dash")
}

fn cmd_introspect() -> ClapCommand {
    ClapCommand::new("introspect")
        .about("Discover jjz capabilities and command details")
        .arg(
            Arg::new("command")
                .required(false)
                .help("Command to introspect (shows all if omitted)"),
        )
        .arg(
            Arg::new("json")
                .long("json")
                .action(clap::ArgAction::SetTrue)
                .help("Output as JSON"),
        )
}

fn cmd_doctor() -> ClapCommand {
    ClapCommand::new("doctor")
        .about("Run system health checks")
        .alias("check")
        .arg(
            Arg::new("json")
                .long("json")
                .action(clap::ArgAction::SetTrue)
                .help("Output as JSON"),
        )
        .arg(
            Arg::new("fix")
                .long("fix")
                .action(clap::ArgAction::SetTrue)
                .help("Auto-fix issues where possible"),
        )
}

fn cmd_query() -> ClapCommand {
    ClapCommand::new("query")
        .about("Query system state programmatically")
        .arg(
            Arg::new("query_type")
                .required(true)
                .help("Type of query (session-exists, session-count, can-run, suggest-name)"),
        )
        .arg(
            Arg::new("args")
                .required(false)
                .help("Query-specific arguments"),
        )
}

fn build_cli() -> ClapCommand {
    ClapCommand::new("jjz")
        .version(env!("CARGO_PKG_VERSION"))
        .author("ZJJ Contributors")
        .about("ZJJ - Manage JJ workspaces with Zellij sessions")
        .subcommand_required(true)
        .subcommand(cmd_init())
        .subcommand(cmd_add())
        .subcommand(cmd_list())
        .subcommand(cmd_remove())
        .subcommand(cmd_focus())
        .subcommand(cmd_status())
        .subcommand(cmd_sync())
        .subcommand(cmd_diff())
        .subcommand(cmd_config())
        .subcommand(cmd_dashboard())
        .subcommand(cmd_introspect())
        .subcommand(cmd_doctor())
        .subcommand(cmd_query())
}

/// Format an error for user display (no stack traces)
fn format_error(err: &anyhow::Error) -> String {
    // Get the root cause message
    let mut msg = err.to_string();

    // If the error chain has more context, include it
    if let Some(source) = err.source() {
        let source_msg = source.to_string();
        // Only add source if it's different and adds value
        if !msg.contains(&source_msg) && !source_msg.is_empty() {
            msg = format!("{msg}\nCause: {source_msg}");
        }
    }

    msg
}

/// Execute the CLI and return a Result
fn run_cli() -> Result<()> {
    let matches = build_cli().get_matches();

    match matches.subcommand() {
        Some(("init", _)) => init::run(),
        Some(("add", sub_m)) => {
            let name = sub_m
                .get_one::<String>("name")
                .ok_or_else(|| anyhow::anyhow!("Name is required"))?;

            let no_hooks = sub_m.get_flag("no-hooks");
            let template = sub_m.get_one::<String>("template").cloned();
            let no_open = sub_m.get_flag("no-open");

            let options = add::AddOptions {
                name: name.clone(),
                no_hooks,
                template,
                no_open,
            };

            add::run_with_options(&options)
        }
        Some(("list", sub_m)) => {
            let all = sub_m.get_flag("all");
            let json = sub_m.get_flag("json");
            list::run(all, json)
        }
        Some(("remove", sub_m)) => {
            let name = sub_m
                .get_one::<String>("name")
                .ok_or_else(|| anyhow::anyhow!("Name is required"))?;
            let options = remove::RemoveOptions {
                force: sub_m.get_flag("force"),
                merge: sub_m.get_flag("merge"),
                keep_branch: sub_m.get_flag("keep-branch"),
                json: sub_m.get_flag("json"),
            };
            remove::run_with_options(name, options)
        }
        Some(("focus", sub_m)) => {
            let name = sub_m
                .get_one::<String>("name")
                .ok_or_else(|| anyhow::anyhow!("Name is required"))?;
            let options = focus::FocusOptions {
                json: sub_m.get_flag("json"),
            };
            focus::run_with_options(name, options)
        }
        Some(("status", sub_m)) => {
            let name = sub_m.get_one::<String>("name").map(String::as_str);
            let json = sub_m.get_flag("json");
            let watch = sub_m.get_flag("watch");
            status::run(name, json, watch)
        }
        Some(("sync", sub_m)) => {
            let name = sub_m.get_one::<String>("name").map(String::as_str);
            let options = sync::SyncOptions {
                json: sub_m.get_flag("json"),
            };
            sync::run_with_options(name, options)
        }
        Some(("diff", sub_m)) => {
            let name = sub_m
                .get_one::<String>("name")
                .ok_or_else(|| anyhow::anyhow!("Name is required"))?;
            let stat = sub_m.get_flag("stat");
            diff::run(name, stat)
        }
        Some(("config", sub_m)) => {
            let key = sub_m.get_one::<String>("key").cloned();
            let value = sub_m.get_one::<String>("value").cloned();
            let global = sub_m.get_flag("global");
            let options = config::ConfigOptions { key, value, global };
            config::run(options)
        }
        Some(("dashboard" | "dash", _)) => dashboard::run(),
        Some(("introspect", sub_m)) => {
            let command = sub_m.get_one::<String>("command").map(String::as_str);
            let json = sub_m.get_flag("json");
            command.map_or_else(
                || introspect::run(json),
                |cmd| introspect::run_command_introspect(cmd, json),
            )
        }
        Some(("doctor" | "check", sub_m)) => {
            let json = sub_m.get_flag("json");
            let fix = sub_m.get_flag("fix");
            doctor::run(json, fix)
        }
        Some(("query", sub_m)) => {
            let query_type = sub_m
                .get_one::<String>("query_type")
                .ok_or_else(|| anyhow::anyhow!("Query type is required"))?;
            let args = sub_m.get_one::<String>("args").map(String::as_str);
            query::run(query_type, args)
        }
        _ => {
            build_cli().print_help()?;
            Ok(())
        }
    }
}

fn main() {
    // Initialize tracing subscriber for logging
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::from_default_env()
                .add_directive(tracing::Level::INFO.into()),
        )
        .with_writer(std::io::stderr)
        .init();

    // Run the CLI and handle errors gracefully
    if let Err(err) = run_cli() {
        eprintln!("Error: {}", format_error(&err));
        process::exit(1);
    }
}
