//! Doctor command - system health checks and auto-fix
//!
//! This command checks the health of the jjz system and can
//! automatically fix common issues.
//!
//! # Exit Codes
//!
//! The doctor command follows standard Unix conventions for exit codes:
//!
//! - **Exit 0**: System is healthy (all checks passed), or all critical issues were successfully
//!   fixed
//! - **Exit 1**: System has errors (one or more checks failed), or critical issues remain after
//!   auto-fix
//!
//! Warnings (CheckStatus::Warn) do not cause non-zero exit codes - only failures
//! (CheckStatus::Fail) do.

use std::process::Command;

use anyhow::Result;
use zjj_core::introspection::{
    CheckStatus, DoctorCheck, DoctorFixOutput, DoctorOutput, FixResult, UnfixableIssue,
};

use crate::{
    cli::{is_command_available, is_inside_zellij, is_jj_repo, jj_root},
    commands::get_session_db,
};

/// Run health checks
pub fn run(json: bool, fix: bool) -> Result<()> {
    let checks = run_all_checks();

    if fix {
        run_fixes(&checks, json)
    } else {
        show_health_report(&checks, json)
    }
}

/// Run all health checks
fn run_all_checks() -> Vec<DoctorCheck> {
    vec![
        check_jj_installed(),
        check_zellij_installed(),
        check_zellij_running(),
        check_jj_repo(),
        check_initialized(),
        check_state_db(),
        check_orphaned_workspaces(),
        check_beads(),
    ]
}

/// Check if JJ is installed
fn check_jj_installed() -> DoctorCheck {
    let installed = is_command_available("jj");

    DoctorCheck {
        name: "JJ Installation".to_string(),
        status: if installed {
            CheckStatus::Pass
        } else {
            CheckStatus::Fail
        },
        message: if installed {
            "JJ is installed".to_string()
        } else {
            "JJ is not installed".to_string()
        },
        suggestion: if installed {
            None
        } else {
            Some("Install JJ: https://github.com/martinvonz/jj#installation".to_string())
        },
        auto_fixable: false,
        details: None,
    }
}

/// Check if Zellij is installed
fn check_zellij_installed() -> DoctorCheck {
    let installed = is_command_available("zellij");

    DoctorCheck {
        name: "Zellij Installation".to_string(),
        status: if installed {
            CheckStatus::Pass
        } else {
            CheckStatus::Fail
        },
        message: if installed {
            "Zellij is installed".to_string()
        } else {
            "Zellij is not installed".to_string()
        },
        suggestion: if installed {
            None
        } else {
            Some("Install Zellij: https://zellij.dev/documentation/installation".to_string())
        },
        auto_fixable: false,
        details: None,
    }
}

/// Check if Zellij is running
fn check_zellij_running() -> DoctorCheck {
    let running = is_inside_zellij();

    DoctorCheck {
        name: "Zellij Running".to_string(),
        status: if running {
            CheckStatus::Pass
        } else {
            CheckStatus::Warn
        },
        message: if running {
            "Inside Zellij session".to_string()
        } else {
            "Not running inside Zellij".to_string()
        },
        suggestion: if running {
            None
        } else {
            Some("Start Zellij: zellij".to_string())
        },
        auto_fixable: false,
        details: None,
    }
}

/// Check if current directory is a JJ repository
fn check_jj_repo() -> DoctorCheck {
    let is_repo = is_jj_repo().unwrap_or(false);

    DoctorCheck {
        name: "JJ Repository".to_string(),
        status: if is_repo {
            CheckStatus::Pass
        } else {
            CheckStatus::Fail
        },
        message: if is_repo {
            "Current directory is a JJ repository".to_string()
        } else {
            "Current directory is not a JJ repository".to_string()
        },
        suggestion: if is_repo {
            None
        } else {
            Some("Initialize JJ: jjz init or jj git init".to_string())
        },
        auto_fixable: false,
        details: None,
    }
}

/// Check if jjz is initialized
fn check_initialized() -> DoctorCheck {
    // Check for .jjz directory existence directly, without depending on JJ installation
    let jjz_dir = std::path::Path::new(".jjz");
    let config_file = jjz_dir.join("config.toml");
    let initialized = jjz_dir.exists() && config_file.exists();

    DoctorCheck {
        name: "jjz Initialized".to_string(),
        status: if initialized {
            CheckStatus::Pass
        } else {
            CheckStatus::Fail
        },
        message: if initialized {
            ".jjz directory exists with valid config".to_string()
        } else {
            "jjz not initialized".to_string()
        },
        suggestion: if initialized {
            None
        } else {
            Some("Initialize jjz: jjz init".to_string())
        },
        auto_fixable: false,
        details: None,
    }
}

/// Check state database health
fn check_state_db() -> DoctorCheck {
    get_session_db().map_or_else(
        |_| DoctorCheck {
            name: "State Database".to_string(),
            status: CheckStatus::Warn,
            message: "State database not accessible".to_string(),
            suggestion: Some("Initialize jjz: jjz init".to_string()),
            auto_fixable: false,
            details: None,
        },
        |db| match db.list(None) {
            Ok(sessions) => DoctorCheck {
                name: "State Database".to_string(),
                status: CheckStatus::Pass,
                message: format!("state.db is healthy ({} sessions)", sessions.len()),
                suggestion: None,
                auto_fixable: false,
                details: None,
            },
            Err(e) => DoctorCheck {
                name: "State Database".to_string(),
                status: CheckStatus::Warn,
                message: format!("Database exists but error reading: {e}"),
                suggestion: Some("Database may be corrupted".to_string()),
                auto_fixable: false,
                details: None,
            },
        },
    )
}

/// Check for orphaned workspaces
fn check_orphaned_workspaces() -> DoctorCheck {
    // Get list of JJ workspaces
    let jj_workspaces = jj_root().map_or_else(
        |_| vec![],
        |root| {
            let output = Command::new("jj")
                .args(["workspace", "list"])
                .current_dir(&root)
                .output();

            match output {
                Ok(out) if out.status.success() => {
                    String::from_utf8_lossy(&out.stdout)
                        .lines()
                        .filter_map(|line| {
                            // Parse workspace list output
                            line.split_whitespace().next().map(str::to_string)
                        })
                        .collect::<Vec<_>>()
                }
                _ => vec![],
            }
        },
    );

    // Get list of sessions from DB
    let session_names = get_session_db()
        .ok()
        .and_then(|db| db.list(None).ok())
        .map(|sessions| sessions.into_iter().map(|s| s.name).collect::<Vec<_>>())
        .unwrap_or_default();

    // Find workspaces without sessions (excluding 'default')
    let orphaned: Vec<_> = jj_workspaces
        .into_iter()
        .filter(|ws| ws != "default" && !session_names.contains(ws))
        .collect();

    if orphaned.is_empty() {
        DoctorCheck {
            name: "Orphaned Workspaces".to_string(),
            status: CheckStatus::Pass,
            message: "No orphaned workspaces found".to_string(),
            suggestion: None,
            auto_fixable: false,
            details: None,
        }
    } else {
        DoctorCheck {
            name: "Orphaned Workspaces".to_string(),
            status: CheckStatus::Warn,
            message: format!(
                "Found {} workspace(s) without session records",
                orphaned.len()
            ),
            suggestion: Some("Run 'jjz doctor --fix' to clean up".to_string()),
            auto_fixable: true,
            details: Some(serde_json::json!({
                "orphaned_workspaces": orphaned,
            })),
        }
    }
}

/// Check Beads integration
fn check_beads() -> DoctorCheck {
    let installed = is_command_available("bd");

    if !installed {
        return DoctorCheck {
            name: "Beads Integration".to_string(),
            status: CheckStatus::Pass,
            message: "Beads not installed (optional)".to_string(),
            suggestion: None,
            auto_fixable: false,
            details: None,
        };
    }

    // Count open issues
    let output = Command::new("bd").args(["list", "--status=open"]).output();

    match output {
        Ok(out) if out.status.success() => {
            let count = String::from_utf8_lossy(&out.stdout)
                .lines()
                .filter(|line| !line.is_empty())
                .count();

            DoctorCheck {
                name: "Beads Integration".to_string(),
                status: CheckStatus::Pass,
                message: format!("Beads installed, {count} open issues"),
                suggestion: None,
                auto_fixable: false,
                details: None,
            }
        }
        _ => DoctorCheck {
            name: "Beads Integration".to_string(),
            status: CheckStatus::Pass,
            message: "Beads installed".to_string(),
            suggestion: None,
            auto_fixable: false,
            details: None,
        },
    }
}

/// Show health report
///
/// # Exit Codes
/// - 0: All checks passed (healthy system)
/// - 1: One or more checks failed (unhealthy system)
fn show_health_report(checks: &[DoctorCheck], json: bool) -> Result<()> {
    let output = DoctorOutput::from_checks(checks.to_vec());

    if json {
        println!("{}", serde_json::to_string_pretty(&output)?);
    } else {
        println!("jjz System Health Check");
        println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
        println!();

        output.checks.iter().for_each(|check| {
            let symbol = match check.status {
                CheckStatus::Pass => "✓",
                CheckStatus::Warn => "⚠",
                CheckStatus::Fail => "✗",
            };

            println!("{symbol} {:<25} {}", check.name, check.message);

            if let Some(ref suggestion) = check.suggestion {
                println!("  → {suggestion}");
            }
        });

        println!();
        println!(
            "Health: {} passed, {} warning(s), {} error(s)",
            output.checks.len() - output.warnings - output.errors,
            output.warnings,
            output.errors
        );

        if output.auto_fixable_issues > 0 {
            println!("Some issues can be auto-fixed: jjz doctor --fix");
        }
    }

    // Return error if system is unhealthy (has failures)
    if !output.healthy {
        anyhow::bail!("Health check failed: {} error(s) detected", output.errors);
    }

    Ok(())
}

/// Run auto-fixes
///
/// # Exit Codes
/// - 0: All critical issues were fixed or none existed
/// - 1: Critical issues remain unfixed
fn run_fixes(checks: &[DoctorCheck], json: bool) -> Result<()> {
    let mut fixed = vec![];
    let mut unable_to_fix = vec![];

    for check in checks {
        if !check.auto_fixable {
            if check.status != CheckStatus::Pass {
                unable_to_fix.push(UnfixableIssue {
                    issue: check.name.clone(),
                    reason: "Requires manual intervention".to_string(),
                    suggestion: check.suggestion.clone().unwrap_or_default(),
                });
            }
            continue;
        }

        // Try to fix the issue
        match check.name.as_str() {
            "Orphaned Workspaces" => match fix_orphaned_workspaces(check) {
                Ok(action) => {
                    fixed.push(FixResult {
                        issue: check.name.clone(),
                        action,
                        success: true,
                    });
                }
                Err(e) => {
                    unable_to_fix.push(UnfixableIssue {
                        issue: check.name.clone(),
                        reason: format!("Fix failed: {e}"),
                        suggestion: check.suggestion.clone().unwrap_or_default(),
                    });
                }
            },
            _ => {
                unable_to_fix.push(UnfixableIssue {
                    issue: check.name.clone(),
                    reason: "No auto-fix available".to_string(),
                    suggestion: check.suggestion.clone().unwrap_or_default(),
                });
            }
        }
    }

    let output = DoctorFixOutput {
        fixed,
        unable_to_fix,
    };

    if json {
        println!("{}", serde_json::to_string_pretty(&output)?);
    } else {
        if !output.fixed.is_empty() {
            println!("Fixed Issues:");
            output.fixed.iter().for_each(|fix| {
                let symbol = if fix.success { "✓" } else { "✗" };
                println!("{symbol} {}: {}", fix.issue, fix.action);
            });
            println!();
        }

        if !output.unable_to_fix.is_empty() {
            println!("Unable to Fix:");
            output.unable_to_fix.iter().for_each(|issue| {
                println!("✗ {}: {}", issue.issue, issue.reason);
                println!("  → {}", issue.suggestion);
            });
        }
    }

    // Count critical (Fail status) issues that couldn't be fixed
    let critical_unfixed = checks
        .iter()
        .filter(|c| {
            c.status == CheckStatus::Fail && !output.fixed.iter().any(|f| f.issue == c.name)
        })
        .count();

    if critical_unfixed > 0 {
        anyhow::bail!(
            "Auto-fix completed but {} critical issue(s) remain unfixed",
            critical_unfixed
        );
    }

    Ok(())
}

/// Fix orphaned workspaces
fn fix_orphaned_workspaces(check: &DoctorCheck) -> Result<String> {
    let orphaned = check
        .details
        .as_ref()
        .and_then(|d| d.get("orphaned_workspaces"))
        .and_then(|w| w.as_array())
        .ok_or_else(|| anyhow::anyhow!("No orphaned workspaces data"))?;

    let root = jj_root()?;

    let removed_count = orphaned
        .iter()
        .filter_map(|workspace| {
            let name = workspace.as_str()?;

            let result = Command::new("jj")
                .args(["workspace", "forget", name])
                .current_dir(&root)
                .output()
                .ok()?;

            if result.status.success() {
                Some(name)
            } else {
                None
            }
        })
        .count();

    Ok(format!("Removed {removed_count} orphaned workspace(s)"))
}

#[cfg(test)]
mod tests {
    use std::fs;

    use tempfile::TempDir;

    use super::*;

    #[test]
    fn test_check_initialized_detects_jjz_directory() {
        // Create a temporary directory
        let temp_dir = TempDir::new().ok().filter(|_| true);
        let Some(temp_dir) = temp_dir else {
            return;
        };

        // Change to temp directory
        let original_dir = std::env::current_dir().ok().filter(|_| true);
        let Some(original_dir) = original_dir else {
            return;
        };
        if std::env::set_current_dir(temp_dir.path()).is_err() {
            return;
        }

        // Test 1: No .jjz directory - should fail
        let result = check_initialized();
        assert_eq!(result.status, CheckStatus::Fail);
        assert_eq!(result.name, "jjz Initialized");
        assert!(result.message.contains("not initialized"));

        // Test 2: .jjz directory exists but no config.toml - should fail
        if fs::create_dir(".jjz").is_err() {
            let _ = std::env::set_current_dir(original_dir);
            return;
        }
        let result = check_initialized();
        assert_eq!(result.status, CheckStatus::Fail);

        // Test 3: .jjz directory with config.toml - should pass
        if fs::write(".jjz/config.toml", "workspace_dir = \"test\"").is_err() {
            let _ = std::env::set_current_dir(original_dir);
            return;
        }
        let result = check_initialized();
        assert_eq!(result.status, CheckStatus::Pass);
        assert!(result.message.contains(".jjz directory exists"));

        // Cleanup: restore original directory
        let _ = std::env::set_current_dir(original_dir);
    }

    #[test]
    fn test_check_initialized_independent_of_jj() {
        // This test verifies that check_initialized doesn't call jj commands
        // We test this by checking it works even without a JJ repo

        let temp_dir = TempDir::new().ok().filter(|_| true);
        let Some(temp_dir) = temp_dir else {
            return;
        };

        let original_dir = std::env::current_dir().ok().filter(|_| true);
        let Some(original_dir) = original_dir else {
            return;
        };
        if std::env::set_current_dir(temp_dir.path()).is_err() {
            return;
        }

        // Create .jjz structure WITHOUT initializing a JJ repo
        if fs::create_dir(".jjz").is_err() {
            let _ = std::env::set_current_dir(original_dir);
            return;
        }
        if fs::write(".jjz/config.toml", "workspace_dir = \"test\"").is_err() {
            let _ = std::env::set_current_dir(original_dir);
            return;
        }

        // Even without JJ installed/initialized, should detect .jjz
        let result = check_initialized();
        assert_eq!(result.status, CheckStatus::Pass);

        // Cleanup
        let _ = std::env::set_current_dir(original_dir);
    }

    #[test]
    fn test_check_jj_installed_vs_check_initialized() {
        // Verify that JJ installation check and initialization check are separate concerns
        let jj_check = check_jj_installed();
        let init_check = check_initialized();

        // These should be independent checks
        assert_eq!(jj_check.name, "JJ Installation");
        assert_eq!(init_check.name, "jjz Initialized");

        // They should have different purposes
        assert!(jj_check.message.contains("JJ") || jj_check.message.contains("installed"));
        assert!(init_check.message.contains("jjz") || init_check.message.contains("initialized"));
    }
}
