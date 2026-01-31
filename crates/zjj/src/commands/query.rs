//! Query command - state queries for AI agents
//!
//! This command provides programmatic access to system state
//! for AI agents to make informed decisions.

use anyhow::Result;
use zjj_core::introspection::{
    Blocker, CanRunQuery, QueryError, SessionCountQuery, SessionExistsQuery, SessionInfo,
};

use crate::{
    cli::{is_command_available, is_inside_zellij, is_jj_repo},
    commands::{get_session_db, zjj_data_dir},
};

/// Query type metadata for help generation
struct QueryTypeInfo {
    name: &'static str,
    description: &'static str,
    requires_arg: bool,
    arg_name: &'static str,
    usage_example: &'static str,
    returns_description: &'static str,
}

impl QueryTypeInfo {
    const fn all() -> &'static [Self] {
        &[
            Self {
                name: "session-exists",
                description: "Check if a session exists by name",
                requires_arg: true,
                arg_name: "session_name",
                usage_example: "jjz query session-exists my-session",
                returns_description: r#"{"exists": true, "session": {"name": "my-session", "status": "active"}}"#,
            },
            Self {
                name: "session-count",
                description: "Count total sessions or filter by status",
                requires_arg: false,
                arg_name: "--status=active",
                usage_example: "jjz query session-count --status=active",
                returns_description: r#"{"count": 5, "filter": {"raw": "--status=active"}}"#,
            },
            Self {
                name: "can-run",
                description: "Check if a command can run and show blockers",
                requires_arg: true,
                arg_name: "command_name",
                usage_example: "jjz query can-run add",
                returns_description: r#"{"can_run": true, "command": "add", "blockers": [], "prerequisites_met": 4, "prerequisites_total": 4}"#,
            },
            Self {
                name: "suggest-name",
                description: "Suggest next available name based on pattern",
                requires_arg: true,
                arg_name: "pattern",
                usage_example: r#"jjz query suggest-name "feature-{n}""#,
                returns_description: r#"{"pattern": "feature-{n}", "suggested": "feature-3", "next_available_n": 3, "existing_matches": ["feature-1", "feature-2"]}"#,
            },
        ]
    }

    fn find(name: &str) -> Option<&'static Self> {
        Self::all().iter().find(|q| q.name == name)
    }

    fn format_error_message(&self) -> String {
        format!(
            "Error: '{}' query requires {} argument\n\n\
             Description:\n  {}\n\n\
             Usage:\n  {} <{}>\n\n\
             Example:\n  {}\n\n\
             Returns:\n  {}",
            self.name,
            if self.requires_arg {
                "a"
            } else {
                "an optional"
            },
            self.description,
            self.name,
            self.arg_name,
            self.usage_example,
            self.returns_description
        )
    }

    fn list_all_queries() -> String {
        let mut output = String::from("Available query types:\n\n");
        for query in Self::all() {
            output.push_str(&format!(
                "  {} - {}\n    Example: {}\n\n",
                query.name, query.description, query.usage_example
            ));
        }
        output.push_str(
            "For detailed help on a specific query type, try running it without arguments.\n",
        );
        output
    }
}

/// Run a query
pub fn run(query_type: &str, args: Option<&str>) -> Result<()> {
    // Handle special help queries
    if query_type == "--help" || query_type == "help" || query_type == "--list" {
        println!("{}", QueryTypeInfo::list_all_queries());
        return Ok(());
    }

    match query_type {
        "session-exists" => {
            let name = args.ok_or_else(|| {
                QueryTypeInfo::find("session-exists")
                    .map(|info| anyhow::anyhow!(info.format_error_message()))
                    .unwrap_or_else(|| anyhow::anyhow!("Query type metadata not found"))
            })?;
            query_session_exists(name)
        }
        "session-count" => query_session_count(args),
        "can-run" => {
            let command = args.ok_or_else(|| {
                QueryTypeInfo::find("can-run")
                    .map(|info| anyhow::anyhow!(info.format_error_message()))
                    .unwrap_or_else(|| anyhow::anyhow!("Query type metadata not found"))
            })?;
            query_can_run(command)
        }
        "suggest-name" => {
            let pattern = args.ok_or_else(|| {
                QueryTypeInfo::find("suggest-name")
                    .map(|info| anyhow::anyhow!(info.format_error_message()))
                    .unwrap_or_else(|| anyhow::anyhow!("Query type metadata not found"))
            })?;
            query_suggest_name(pattern)
        }
        _ => {
            let error_msg = format!(
                "Error: Unknown query type '{}'\n\n{}",
                query_type,
                QueryTypeInfo::list_all_queries()
            );
            Err(anyhow::anyhow!(error_msg))
        }
    }
}

/// Categorize database errors for better error reporting
fn categorize_db_error(err: &anyhow::Error) -> (String, String) {
    let err_str = err.to_string();
    if err_str.contains("no such table") || err_str.contains("database schema") {
        (
            "DATABASE_NOT_INITIALIZED".to_string(),
            "Database not initialized. Run 'jjz init' first.".to_string(),
        )
    } else if err_str.contains("locked") {
        (
            "DATABASE_LOCKED".to_string(),
            "Database is locked by another process".to_string(),
        )
    } else {
        (
            "DATABASE_INIT_ERROR".to_string(),
            format!("Failed to access database: {}", err),
        )
    }
}

/// Query if a session exists
fn query_session_exists(name: &str) -> Result<()> {
    let result = match get_session_db() {
        Ok(db) => match db.get(name) {
            Ok(session) => SessionExistsQuery {
                exists: Some(session.is_some()),
                session: session.map(|s| SessionInfo {
                    name: s.name,
                    status: s.status.to_string(),
                }),
                error: None,
            },
            Err(e) => SessionExistsQuery {
                exists: None,
                session: None,
                error: Some(QueryError {
                    code: "DATABASE_ERROR".to_string(),
                    message: format!("Failed to query session: {}", e),
                }),
            },
        },
        Err(e) => {
            let (code, message) = categorize_db_error(&e);
            SessionExistsQuery {
                exists: None,
                session: None,
                error: Some(QueryError { code, message }),
            }
        }
    };

    println!("{}", serde_json::to_string_pretty(&result)?);
    Ok(())
}

/// Query session count
fn query_session_count(filter: Option<&str>) -> Result<()> {
    let result = match get_session_db() {
        Ok(db) => match db.list(None) {
            Ok(sessions) => {
                let count = filter
                    .and_then(|f| f.strip_prefix("--status="))
                    .map(|status| {
                        sessions
                            .iter()
                            .filter(|s| s.status.to_string() == status)
                            .count()
                    })
                    .unwrap_or_else(|| sessions.len());

                SessionCountQuery {
                    count: Some(count),
                    filter: filter.map(|f| serde_json::json!({"raw": f})),
                    error: None,
                }
            }
            Err(e) => SessionCountQuery {
                count: None,
                filter: filter.map(|f| serde_json::json!({"raw": f})),
                error: Some(QueryError {
                    code: "DATABASE_ERROR".to_string(),
                    message: format!("Failed to list sessions: {}", e),
                }),
            },
        },
        Err(e) => {
            let (code, message) = categorize_db_error(&e);
            SessionCountQuery {
                count: None,
                filter: filter.map(|f| serde_json::json!({"raw": f})),
                error: Some(QueryError { code, message }),
            }
        }
    };

    println!("{}", serde_json::to_string_pretty(&result)?);
    Ok(())
}

/// Query if a command can run
fn query_can_run(command: &str) -> Result<()> {
    let mut blockers = vec![];
    let mut prereqs_met = 0;
    let prereqs_total = 4; // Adjust based on command

    // Check if initialized
    let initialized = zjj_data_dir().is_ok();
    if !initialized && requires_init(command) {
        blockers.push(Blocker {
            check: "initialized".to_string(),
            status: false,
            message: "jjz not initialized".to_string(),
        });
    } else if requires_init(command) {
        prereqs_met += 1;
    }

    // Check JJ installed
    let jj_installed = is_command_available("jj");
    if !jj_installed && requires_jj(command) {
        blockers.push(Blocker {
            check: "jj_installed".to_string(),
            status: false,
            message: "JJ not installed".to_string(),
        });
    } else if requires_jj(command) {
        prereqs_met += 1;
    }

    // Check JJ repo
    let jj_repo = is_jj_repo().unwrap_or(false);
    if !jj_repo && requires_jj_repo(command) {
        blockers.push(Blocker {
            check: "jj_repo".to_string(),
            status: false,
            message: "Not in a JJ repository".to_string(),
        });
    } else if requires_jj_repo(command) {
        prereqs_met += 1;
    }

    // Check Zellij running
    let zellij_running = is_inside_zellij();
    if !zellij_running && requires_zellij(command) {
        blockers.push(Blocker {
            check: "zellij_running".to_string(),
            status: false,
            message: "Zellij is not running".to_string(),
        });
    } else if requires_zellij(command) {
        prereqs_met += 1;
    }

    let result = CanRunQuery {
        can_run: blockers.is_empty(),
        command: command.to_string(),
        blockers,
        prerequisites_met: prereqs_met,
        prerequisites_total: prereqs_total,
    };

    println!("{}", serde_json::to_string_pretty(&result)?);
    Ok(())
}

/// Query for suggested name based on pattern
fn query_suggest_name(pattern: &str) -> Result<()> {
    // suggest_name can work without database access if we can't get sessions
    let existing_names = match get_session_db() {
        Ok(db) => match db.list(None) {
            Ok(sessions) => sessions.into_iter().map(|s| s.name).collect(),
            Err(_) => Vec::new(), // Fallback to empty list
        },
        Err(_) => Vec::new(), // Fallback to empty list if prerequisites not met
    };

    let result = zjj_core::introspection::suggest_name(pattern, &existing_names)?;

    println!("{}", serde_json::to_string_pretty(&result)?);
    Ok(())
}

/// Check if command requires initialization
fn requires_init(command: &str) -> bool {
    matches!(
        command,
        "add" | "remove" | "list" | "focus" | "status" | "sync" | "diff"
    )
}

/// Check if command requires JJ to be installed
fn requires_jj(command: &str) -> bool {
    matches!(
        command,
        "init" | "add" | "remove" | "status" | "sync" | "diff"
    )
}

/// Check if command requires being in a JJ repo
fn requires_jj_repo(command: &str) -> bool {
    matches!(command, "add" | "remove" | "status" | "sync" | "diff")
}

/// Check if command requires Zellij to be running
fn requires_zellij(command: &str) -> bool {
    matches!(command, "add" | "focus")
}

/// Categorize database errors into error codes and messages
fn categorize_db_error(error: &anyhow::Error) -> (String, String) {
    let error_msg = error.to_string();

    // Check for JJ not installed
    if error_msg.contains("JJ not installed") || error_msg.contains("jj: not found") {
        return (
            "JJ_NOT_INSTALLED".to_string(),
            "Cannot check session - JJ not installed".to_string(),
        );
    }

    // Check for not in JJ repo
    if error_msg.contains("Not in a JJ repository") || error_msg.contains("not a jj repo") {
        return (
            "NOT_JJ_REPO".to_string(),
            "Cannot check session - not in a JJ repository".to_string(),
        );
    }

    // Check for not initialized
    if error_msg.contains("not initialized") || error_msg.contains("Run 'jjz init'") {
        return (
            "NOT_INITIALIZED".to_string(),
            "Cannot check session - jjz not initialized".to_string(),
        );
    }

    // Generic database error
    (
        "DATABASE_ERROR".to_string(),
        format!("Cannot check session - {}", error_msg),
    )
}
