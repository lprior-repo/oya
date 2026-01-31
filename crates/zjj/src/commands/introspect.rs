//! Introspect command - discover jjz capabilities
//!
//! This command enables AI agents to understand available commands,
//! system state, and dependencies.

use anyhow::Result;
use im::HashMap;
use zjj_core::introspection::{
    ArgumentSpec, CommandExample, CommandIntrospection, DependencyInfo, ErrorCondition, FlagSpec,
    IntrospectOutput, Prerequisites, SystemState,
};

use crate::{
    cli::{is_command_available, is_jj_repo, run_command},
    commands::{get_session_db, zjj_data_dir},
};

/// Get version of a command by running `command --version`
fn get_command_version(command: &str) -> Option<String> {
    run_command(command, &["--version"])
        .ok()
        .and_then(|output| output.lines().next().map(|line| line.trim().to_string()))
}

/// Check dependencies and their status
fn check_dependencies() -> HashMap<String, DependencyInfo> {
    // JJ (required)
    let jj_installed = is_command_available("jj");
    let jj_info = DependencyInfo {
        required: true,
        installed: jj_installed,
        version: if jj_installed {
            get_command_version("jj")
        } else {
            None
        },
        command: "jj".to_string(),
    };

    // Zellij (required)
    let zellij_installed = is_command_available("zellij");
    let zellij_info = DependencyInfo {
        required: true,
        installed: zellij_installed,
        version: if zellij_installed {
            get_command_version("zellij")
        } else {
            None
        },
        command: "zellij".to_string(),
    };

    // Claude (optional)
    let claude_installed = is_command_available("claude");
    let claude_info = DependencyInfo {
        required: false,
        installed: claude_installed,
        version: if claude_installed {
            get_command_version("claude")
        } else {
            None
        },
        command: "claude".to_string(),
    };

    // Beads (optional)
    let beads_installed = is_command_available("bd");
    let beads_info = DependencyInfo {
        required: false,
        installed: beads_installed,
        version: if beads_installed {
            get_command_version("bd")
        } else {
            None
        },
        command: "bd".to_string(),
    };

    HashMap::new()
        .update("jj".to_string(), jj_info)
        .update("zellij".to_string(), zellij_info)
        .update("claude".to_string(), claude_info)
        .update("beads".to_string(), beads_info)
}

/// Get current system state
fn get_system_state() -> SystemState {
    let jj_repo = is_jj_repo().unwrap_or(false);
    let initialized = zjj_data_dir().is_ok();

    let (config_path, state_db, sessions_count, active_sessions) = if initialized {
        let data_dir = zjj_data_dir().ok();
        let config = data_dir
            .as_ref()
            .map(|d| d.join("config.toml").display().to_string());
        let db = data_dir
            .as_ref()
            .map(|d| d.join("sessions.db").display().to_string());

        let (count, active) = get_session_db()
            .ok()
            .and_then(|db| {
                db.list(None).ok().map(|sessions| {
                    let total = sessions.len();
                    let active = sessions
                        .iter()
                        .filter(|s| s.status.to_string() == "active")
                        .count();
                    (total, active)
                })
            })
            .unwrap_or((0, 0));

        (config, db, count, active)
    } else {
        (None, None, 0, 0)
    };

    SystemState {
        initialized,
        jj_repo,
        config_path,
        state_db,
        sessions_count,
        active_sessions,
    }
}

/// Run the introspect command - show all capabilities
pub fn run(json: bool) -> Result<()> {
    let version = env!("CARGO_PKG_VERSION");
    let mut output = IntrospectOutput::new(version);

    // Add dependencies
    output.dependencies = check_dependencies();

    // Add system state
    output.system_state = get_system_state();

    if json {
        println!("{}", serde_json::to_string_pretty(&output)?);
    } else {
        print_human_readable(&output);
    }

    Ok(())
}

/// Print introspection output in human-readable format
fn print_human_readable(output: &IntrospectOutput) {
    println!("JJZ Version: {}", output.jjz_version);
    println!();

    println!("Capabilities:");
    println!("  Session Management:");
    for cmd in &output.capabilities.session_management.commands {
        println!("    - {cmd}");
    }
    println!("  Version Control:");
    for cmd in &output.capabilities.version_control.commands {
        println!("    - {cmd}");
    }
    println!("  Introspection:");
    for cmd in &output.capabilities.introspection.commands {
        println!("    - {cmd}");
    }
    println!();

    println!("Dependencies:");
    for (name, info) in &output.dependencies {
        let status = if info.installed { "✓" } else { "✗" };
        let required = if info.required {
            " (required)"
        } else {
            " (optional)"
        };
        let version = info
            .version
            .as_ref()
            .map(|v| format!(" - {v}"))
            .unwrap_or_default();
        println!("  {status} {name}{required}{version}");
    }
    println!();

    println!("System State:");
    println!(
        "  Initialized: {}",
        if output.system_state.initialized {
            "yes"
        } else {
            "no"
        }
    );
    println!(
        "  JJ Repository: {}",
        if output.system_state.jj_repo {
            "yes"
        } else {
            "no"
        }
    );
    if let Some(ref path) = output.system_state.config_path {
        println!("  Config: {path}");
    }
    if let Some(ref path) = output.system_state.state_db {
        println!("  Database: {path}");
    }
    println!(
        "  Sessions: {} total, {} active",
        output.system_state.sessions_count, output.system_state.active_sessions
    );
}

/// Introspect a specific command
pub fn run_command_introspect(command: &str, json: bool) -> Result<()> {
    let introspection = match command {
        "add" => get_add_introspection(),
        "remove" => get_remove_introspection(),
        "list" => get_list_introspection(),
        "init" => get_init_introspection(),
        "focus" => get_focus_introspection(),
        "status" => get_status_introspection(),
        "sync" => get_sync_introspection(),
        "diff" => get_diff_introspection(),
        "introspect" => get_introspect_introspection(),
        "doctor" => get_doctor_introspection(),
        "query" => get_query_introspection(),
        _ => {
            anyhow::bail!("Unknown command: {command}");
        }
    };

    if json {
        println!("{}", serde_json::to_string_pretty(&introspection)?);
    } else {
        print_command_human_readable(&introspection);
    }

    Ok(())
}

/// Print command introspection in human-readable format
fn print_command_human_readable(cmd: &CommandIntrospection) {
    println!("Command: {}", cmd.command);
    println!("Description: {}", cmd.description);
    println!();

    if !cmd.arguments.is_empty() {
        println!("Arguments:");
        for arg in &cmd.arguments {
            let required = if arg.required {
                " (required)"
            } else {
                " (optional)"
            };
            println!("  {}{required}", arg.name);
            println!("    Type: {}", arg.arg_type);
            println!("    Description: {}", arg.description);
            if !arg.examples.is_empty() {
                println!("    Examples: {}", arg.examples.join(", "));
            }
        }
        println!();
    }

    if !cmd.flags.is_empty() {
        println!("Flags:");
        for flag in &cmd.flags {
            let short = flag
                .short
                .as_ref()
                .map(|s| format!("-{s}, "))
                .unwrap_or_default();
            println!("  {short}--{}", flag.long);
            println!("    Type: {}", flag.flag_type);
            println!("    Description: {}", flag.description);
            if let Some(ref default) = flag.default {
                println!("    Default: {default}");
            }
            if !flag.possible_values.is_empty() {
                println!("    Values: {}", flag.possible_values.join(", "));
            }
        }
        println!();
    }

    if !cmd.examples.is_empty() {
        println!("Examples:");
        for example in &cmd.examples {
            println!("  {}", example.command);
            println!("    {}", example.description);
        }
        println!();
    }

    println!("Prerequisites:");
    println!("  Initialized: {}", cmd.prerequisites.initialized);
    println!("  JJ Installed: {}", cmd.prerequisites.jj_installed);
    println!("  Zellij Running: {}", cmd.prerequisites.zellij_running);
}

// Command introspection definitions

fn get_add_introspection() -> CommandIntrospection {
    CommandIntrospection {
        command: "add".to_string(),
        description: "Create new parallel development session".to_string(),
        aliases: vec!["a".to_string(), "new".to_string()],
        arguments: vec![ArgumentSpec {
            name: "name".to_string(),
            arg_type: "string".to_string(),
            required: true,
            description: "Session name".to_string(),
            validation: Some("^[a-zA-Z0-9_-]+$".to_string()),
            examples: vec![
                "feature-auth".to_string(),
                "bugfix-123".to_string(),
                "experiment".to_string(),
            ],
        }],
        flags: vec![
            FlagSpec {
                long: "no-hooks".to_string(),
                short: None,
                description: "Skip post_create hooks".to_string(),
                flag_type: "bool".to_string(),
                default: Some(serde_json::json!(false)),
                possible_values: vec![],
            },
            FlagSpec {
                long: "template".to_string(),
                short: Some("t".to_string()),
                description: "Layout template name".to_string(),
                flag_type: "string".to_string(),
                default: Some(serde_json::json!("standard")),
                possible_values: vec![
                    "minimal".to_string(),
                    "standard".to_string(),
                    "full".to_string(),
                ],
            },
            FlagSpec {
                long: "no-open".to_string(),
                short: None,
                description: "Create workspace but don't open Zellij tab".to_string(),
                flag_type: "bool".to_string(),
                default: Some(serde_json::json!(false)),
                possible_values: vec![],
            },
        ],
        examples: vec![
            CommandExample {
                command: "jjz add feature-auth".to_string(),
                description: "Create session with default template".to_string(),
            },
            CommandExample {
                command: "jjz add bugfix-123 --no-hooks".to_string(),
                description: "Create without running hooks".to_string(),
            },
            CommandExample {
                command: "jjz add experiment -t minimal".to_string(),
                description: "Create with minimal layout".to_string(),
            },
        ],
        prerequisites: Prerequisites {
            initialized: true,
            jj_installed: true,
            zellij_running: true,
            custom: vec!["Session name must be unique".to_string()],
        },
        side_effects: vec![
            "Creates JJ workspace".to_string(),
            "Generates Zellij layout file".to_string(),
            "Opens Zellij tab".to_string(),
            "Executes post_create hooks".to_string(),
            "Records session in state.db".to_string(),
        ],
        error_conditions: vec![
            ErrorCondition {
                code: "SESSION_ALREADY_EXISTS".to_string(),
                description: "Session with this name exists".to_string(),
                resolution: "Use different name or remove existing session".to_string(),
            },
            ErrorCondition {
                code: "INVALID_SESSION_NAME".to_string(),
                description: "Session name contains invalid characters".to_string(),
                resolution: "Use only alphanumeric, hyphens, underscores".to_string(),
            },
            ErrorCondition {
                code: "ZELLIJ_NOT_RUNNING".to_string(),
                description: "Zellij is not running".to_string(),
                resolution: "Start Zellij first: zellij".to_string(),
            },
        ],
    }
}

fn get_remove_introspection() -> CommandIntrospection {
    CommandIntrospection {
        command: "remove".to_string(),
        description: "Remove a session and its workspace".to_string(),
        aliases: vec!["rm".to_string(), "delete".to_string()],
        arguments: vec![ArgumentSpec {
            name: "name".to_string(),
            arg_type: "string".to_string(),
            required: true,
            description: "Name of the session to remove".to_string(),
            validation: None,
            examples: vec!["my-session".to_string()],
        }],
        flags: vec![
            FlagSpec {
                long: "force".to_string(),
                short: Some("f".to_string()),
                description: "Skip confirmation prompt and hooks".to_string(),
                flag_type: "bool".to_string(),
                default: Some(serde_json::json!(false)),
                possible_values: vec![],
            },
            FlagSpec {
                long: "merge".to_string(),
                short: Some("m".to_string()),
                description: "Squash-merge to main before removal".to_string(),
                flag_type: "bool".to_string(),
                default: Some(serde_json::json!(false)),
                possible_values: vec![],
            },
            FlagSpec {
                long: "keep-branch".to_string(),
                short: Some("k".to_string()),
                description: "Preserve branch after removal".to_string(),
                flag_type: "bool".to_string(),
                default: Some(serde_json::json!(false)),
                possible_values: vec![],
            },
        ],
        examples: vec![
            CommandExample {
                command: "jjz remove my-session".to_string(),
                description: "Remove session with confirmation".to_string(),
            },
            CommandExample {
                command: "jjz remove my-session -f".to_string(),
                description: "Remove without confirmation".to_string(),
            },
            CommandExample {
                command: "jjz remove my-session -m".to_string(),
                description: "Merge changes before removing".to_string(),
            },
        ],
        prerequisites: Prerequisites {
            initialized: true,
            jj_installed: true,
            zellij_running: false,
            custom: vec!["Session must exist".to_string()],
        },
        side_effects: vec![
            "Closes Zellij tab".to_string(),
            "Removes JJ workspace".to_string(),
            "Deletes layout file".to_string(),
            "Removes session from state.db".to_string(),
        ],
        error_conditions: vec![ErrorCondition {
            code: "SESSION_NOT_FOUND".to_string(),
            description: "Session does not exist".to_string(),
            resolution: "Check session name with 'jjz list'".to_string(),
        }],
    }
}

fn get_list_introspection() -> CommandIntrospection {
    CommandIntrospection {
        command: "list".to_string(),
        description: "List all sessions".to_string(),
        aliases: vec!["ls".to_string()],
        arguments: vec![],
        flags: vec![
            FlagSpec {
                long: "all".to_string(),
                short: None,
                description: "Include completed and failed sessions".to_string(),
                flag_type: "bool".to_string(),
                default: Some(serde_json::json!(false)),
                possible_values: vec![],
            },
            FlagSpec {
                long: "json".to_string(),
                short: None,
                description: "Output as JSON".to_string(),
                flag_type: "bool".to_string(),
                default: Some(serde_json::json!(false)),
                possible_values: vec![],
            },
        ],
        examples: vec![
            CommandExample {
                command: "jjz list".to_string(),
                description: "List active sessions".to_string(),
            },
            CommandExample {
                command: "jjz list --all".to_string(),
                description: "List all sessions including completed".to_string(),
            },
        ],
        prerequisites: Prerequisites {
            initialized: true,
            jj_installed: false,
            zellij_running: false,
            custom: vec![],
        },
        side_effects: vec![],
        error_conditions: vec![],
    }
}

fn get_init_introspection() -> CommandIntrospection {
    CommandIntrospection {
        command: "init".to_string(),
        description: "Initialize jjz in a JJ repository".to_string(),
        aliases: vec![],
        arguments: vec![],
        flags: vec![],
        examples: vec![CommandExample {
            command: "jjz init".to_string(),
            description: "Initialize jjz in current directory".to_string(),
        }],
        prerequisites: Prerequisites {
            initialized: false,
            jj_installed: true,
            zellij_running: false,
            custom: vec![],
        },
        side_effects: vec![
            "Creates .jjz directory".to_string(),
            "Creates config.toml".to_string(),
            "Creates sessions.db".to_string(),
        ],
        error_conditions: vec![ErrorCondition {
            code: "ALREADY_INITIALIZED".to_string(),
            description: "JJZ already initialized".to_string(),
            resolution: "Remove .jjz directory to reinitialize".to_string(),
        }],
    }
}

fn get_focus_introspection() -> CommandIntrospection {
    CommandIntrospection {
        command: "focus".to_string(),
        description: "Switch to a session's Zellij tab".to_string(),
        aliases: vec!["switch".to_string()],
        arguments: vec![ArgumentSpec {
            name: "name".to_string(),
            arg_type: "string".to_string(),
            required: true,
            description: "Name of the session to focus".to_string(),
            validation: None,
            examples: vec!["my-session".to_string()],
        }],
        flags: vec![],
        examples: vec![CommandExample {
            command: "jjz focus my-session".to_string(),
            description: "Switch to my-session tab".to_string(),
        }],
        prerequisites: Prerequisites {
            initialized: true,
            jj_installed: false,
            zellij_running: true,
            custom: vec!["Session must exist".to_string()],
        },
        side_effects: vec!["Switches Zellij tab".to_string()],
        error_conditions: vec![ErrorCondition {
            code: "SESSION_NOT_FOUND".to_string(),
            description: "Session does not exist".to_string(),
            resolution: "Check session name with 'jjz list'".to_string(),
        }],
    }
}

fn get_status_introspection() -> CommandIntrospection {
    CommandIntrospection {
        command: "status".to_string(),
        description: "Show detailed session status".to_string(),
        aliases: vec![],
        arguments: vec![ArgumentSpec {
            name: "name".to_string(),
            arg_type: "string".to_string(),
            required: false,
            description: "Session name (shows all if omitted)".to_string(),
            validation: None,
            examples: vec!["my-session".to_string()],
        }],
        flags: vec![
            FlagSpec {
                long: "json".to_string(),
                short: None,
                description: "Output as JSON".to_string(),
                flag_type: "bool".to_string(),
                default: Some(serde_json::json!(false)),
                possible_values: vec![],
            },
            FlagSpec {
                long: "watch".to_string(),
                short: None,
                description: "Continuously update status".to_string(),
                flag_type: "bool".to_string(),
                default: Some(serde_json::json!(false)),
                possible_values: vec![],
            },
        ],
        examples: vec![
            CommandExample {
                command: "jjz status".to_string(),
                description: "Show status of all sessions".to_string(),
            },
            CommandExample {
                command: "jjz status my-session".to_string(),
                description: "Show status of specific session".to_string(),
            },
        ],
        prerequisites: Prerequisites {
            initialized: true,
            jj_installed: true,
            zellij_running: false,
            custom: vec![],
        },
        side_effects: vec![],
        error_conditions: vec![],
    }
}

fn get_sync_introspection() -> CommandIntrospection {
    CommandIntrospection {
        command: "sync".to_string(),
        description: "Sync session workspace with main (rebase)".to_string(),
        aliases: vec![],
        arguments: vec![ArgumentSpec {
            name: "name".to_string(),
            arg_type: "string".to_string(),
            required: false,
            description: "Session name (syncs current if omitted)".to_string(),
            validation: None,
            examples: vec!["my-session".to_string()],
        }],
        flags: vec![],
        examples: vec![CommandExample {
            command: "jjz sync my-session".to_string(),
            description: "Sync session with main branch".to_string(),
        }],
        prerequisites: Prerequisites {
            initialized: true,
            jj_installed: true,
            zellij_running: false,
            custom: vec![],
        },
        side_effects: vec![
            "Rebases workspace onto main".to_string(),
            "Updates last_synced timestamp".to_string(),
        ],
        error_conditions: vec![ErrorCondition {
            code: "CONFLICTS".to_string(),
            description: "Rebase resulted in conflicts".to_string(),
            resolution: "Resolve conflicts manually".to_string(),
        }],
    }
}

fn get_diff_introspection() -> CommandIntrospection {
    CommandIntrospection {
        command: "diff".to_string(),
        description: "Show diff between session and main".to_string(),
        aliases: vec![],
        arguments: vec![ArgumentSpec {
            name: "name".to_string(),
            arg_type: "string".to_string(),
            required: true,
            description: "Session name".to_string(),
            validation: None,
            examples: vec!["my-session".to_string()],
        }],
        flags: vec![FlagSpec {
            long: "stat".to_string(),
            short: None,
            description: "Show diffstat only".to_string(),
            flag_type: "bool".to_string(),
            default: Some(serde_json::json!(false)),
            possible_values: vec![],
        }],
        examples: vec![
            CommandExample {
                command: "jjz diff my-session".to_string(),
                description: "Show full diff".to_string(),
            },
            CommandExample {
                command: "jjz diff my-session --stat".to_string(),
                description: "Show diffstat summary".to_string(),
            },
        ],
        prerequisites: Prerequisites {
            initialized: true,
            jj_installed: true,
            zellij_running: false,
            custom: vec!["Session must exist".to_string()],
        },
        side_effects: vec![],
        error_conditions: vec![],
    }
}

fn get_introspect_introspection() -> CommandIntrospection {
    CommandIntrospection {
        command: "introspect".to_string(),
        description: "Discover jjz capabilities".to_string(),
        aliases: vec![],
        arguments: vec![ArgumentSpec {
            name: "command".to_string(),
            arg_type: "string".to_string(),
            required: false,
            description: "Command to introspect (shows all if omitted)".to_string(),
            validation: None,
            examples: vec!["add".to_string(), "remove".to_string()],
        }],
        flags: vec![FlagSpec {
            long: "json".to_string(),
            short: None,
            description: "Output as JSON".to_string(),
            flag_type: "bool".to_string(),
            default: Some(serde_json::json!(false)),
            possible_values: vec![],
        }],
        examples: vec![
            CommandExample {
                command: "jjz introspect".to_string(),
                description: "Show all capabilities".to_string(),
            },
            CommandExample {
                command: "jjz introspect add --json".to_string(),
                description: "Get add command schema as JSON".to_string(),
            },
        ],
        prerequisites: Prerequisites {
            initialized: false,
            jj_installed: false,
            zellij_running: false,
            custom: vec![],
        },
        side_effects: vec![],
        error_conditions: vec![],
    }
}

fn get_doctor_introspection() -> CommandIntrospection {
    CommandIntrospection {
        command: "doctor".to_string(),
        description: "Run system health checks".to_string(),
        aliases: vec!["check".to_string()],
        arguments: vec![],
        flags: vec![
            FlagSpec {
                long: "json".to_string(),
                short: None,
                description: "Output as JSON".to_string(),
                flag_type: "bool".to_string(),
                default: Some(serde_json::json!(false)),
                possible_values: vec![],
            },
            FlagSpec {
                long: "fix".to_string(),
                short: None,
                description: "Auto-fix issues where possible".to_string(),
                flag_type: "bool".to_string(),
                default: Some(serde_json::json!(false)),
                possible_values: vec![],
            },
        ],
        examples: vec![
            CommandExample {
                command: "jjz doctor".to_string(),
                description: "Check system health".to_string(),
            },
            CommandExample {
                command: "jjz doctor --fix".to_string(),
                description: "Auto-fix issues".to_string(),
            },
        ],
        prerequisites: Prerequisites {
            initialized: false,
            jj_installed: false,
            zellij_running: false,
            custom: vec![],
        },
        side_effects: vec!["May fix issues with --fix flag".to_string()],
        error_conditions: vec![],
    }
}

fn get_query_introspection() -> CommandIntrospection {
    CommandIntrospection {
        command: "query".to_string(),
        description: "Query system state".to_string(),
        aliases: vec![],
        arguments: vec![
            ArgumentSpec {
                name: "query_type".to_string(),
                arg_type: "string".to_string(),
                required: true,
                description: "Type of query".to_string(),
                validation: None,
                examples: vec![
                    "session-exists".to_string(),
                    "session-count".to_string(),
                    "can-run".to_string(),
                    "suggest-name".to_string(),
                ],
            },
            ArgumentSpec {
                name: "args".to_string(),
                arg_type: "string".to_string(),
                required: false,
                description: "Query-specific arguments".to_string(),
                validation: None,
                examples: vec!["my-session".to_string(), "feature-{n}".to_string()],
            },
        ],
        flags: vec![FlagSpec {
            long: "json".to_string(),
            short: None,
            description: "Output as JSON".to_string(),
            flag_type: "bool".to_string(),
            default: Some(serde_json::json!(true)),
            possible_values: vec![],
        }],
        examples: vec![
            CommandExample {
                command: "jjz query session-exists my-session".to_string(),
                description: "Check if session exists".to_string(),
            },
            CommandExample {
                command: "jjz query can-run add".to_string(),
                description: "Check if add command can run".to_string(),
            },
            CommandExample {
                command: "jjz query suggest-name feature-{n}".to_string(),
                description: "Suggest next available name".to_string(),
            },
        ],
        prerequisites: Prerequisites {
            initialized: false,
            jj_installed: false,
            zellij_running: false,
            custom: vec![],
        },
        side_effects: vec![],
        error_conditions: vec![],
    }
}
