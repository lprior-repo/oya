//! AI-first introspection capabilities
//!
//! This module provides structured metadata about jjz capabilities,
//! enabling AI agents to discover features and understand system state.

use im::HashMap;
use serde::{Deserialize, Serialize};

use crate::{Error, Result};

/// Complete introspection output
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IntrospectOutput {
    /// JJZ version
    pub jjz_version: String,
    /// Categorized capabilities
    pub capabilities: Capabilities,
    /// External dependency status
    pub dependencies: HashMap<String, DependencyInfo>,
    /// Current system state
    pub system_state: SystemState,
}

/// Categorized capabilities
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Capabilities {
    /// Session management capabilities
    pub session_management: CapabilityCategory,
    /// Configuration capabilities
    pub configuration: CapabilityCategory,
    /// Version control capabilities
    pub version_control: CapabilityCategory,
    /// Introspection and diagnostics
    pub introspection: CapabilityCategory,
}

/// A category of related capabilities
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CapabilityCategory {
    /// Available commands in this category
    pub commands: Vec<String>,
    /// Feature descriptions
    pub features: Vec<String>,
}

/// Information about an external dependency
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DependencyInfo {
    /// Whether this dependency is required for core functionality
    pub required: bool,
    /// Whether the dependency is currently installed
    pub installed: bool,
    /// Installed version if available
    #[serde(skip_serializing_if = "Option::is_none")]
    pub version: Option<String>,
    /// Command name
    pub command: String,
}

/// Current system state
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct SystemState {
    /// Whether jjz has been initialized in this repo
    pub initialized: bool,
    /// Whether current directory is a JJ repository
    pub jj_repo: bool,
    /// Path to config file
    #[serde(skip_serializing_if = "Option::is_none")]
    pub config_path: Option<String>,
    /// Path to state database
    #[serde(skip_serializing_if = "Option::is_none")]
    pub state_db: Option<String>,
    /// Total number of sessions
    pub sessions_count: usize,
    /// Number of active sessions
    pub active_sessions: usize,
}

/// Detailed command introspection
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CommandIntrospection {
    /// Command name
    pub command: String,
    /// Human-readable description
    pub description: String,
    /// Command aliases
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub aliases: Vec<String>,
    /// Positional arguments
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub arguments: Vec<ArgumentSpec>,
    /// Optional flags
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub flags: Vec<FlagSpec>,
    /// Usage examples
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub examples: Vec<CommandExample>,
    /// Prerequisites for running this command
    pub prerequisites: Prerequisites,
    /// Side effects this command will produce
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub side_effects: Vec<String>,
    /// Possible error conditions
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub error_conditions: Vec<ErrorCondition>,
}

/// Argument specification
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ArgumentSpec {
    /// Argument name
    pub name: String,
    /// Type of argument
    #[serde(rename = "type")]
    pub arg_type: String,
    /// Whether this argument is required
    pub required: bool,
    /// Human-readable description
    pub description: String,
    /// Validation pattern (regex)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub validation: Option<String>,
    /// Example values
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub examples: Vec<String>,
}

/// Flag specification
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FlagSpec {
    /// Long flag name (e.g., "no-hooks")
    pub long: String,
    /// Short flag name (e.g., "t")
    #[serde(skip_serializing_if = "Option::is_none")]
    pub short: Option<String>,
    /// Human-readable description
    pub description: String,
    /// Type of flag value
    #[serde(rename = "type")]
    pub flag_type: String,
    /// Default value
    #[serde(skip_serializing_if = "Option::is_none")]
    pub default: Option<serde_json::Value>,
    /// Possible values for enum-like flags
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub possible_values: Vec<String>,
}

/// Command usage example
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CommandExample {
    /// Example command line
    pub command: String,
    /// Description of what this example does
    pub description: String,
}

/// Prerequisites for a command
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Prerequisites {
    /// Must be initialized
    pub initialized: bool,
    /// JJ must be installed
    pub jj_installed: bool,
    /// Zellij must be running
    pub zellij_running: bool,
    /// Additional custom checks
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub custom: Vec<String>,
}

/// Error condition documentation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ErrorCondition {
    /// Error code
    pub code: String,
    /// Human-readable description
    pub description: String,
    /// How to resolve this error
    pub resolution: String,
}

/// System health check
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DoctorCheck {
    /// Check name
    pub name: String,
    /// Check status
    pub status: CheckStatus,
    /// Status message
    pub message: String,
    /// Suggestion for fixing issues
    #[serde(skip_serializing_if = "Option::is_none")]
    pub suggestion: Option<String>,
    /// Whether this issue can be auto-fixed
    pub auto_fixable: bool,
    /// Additional details about the check
    #[serde(skip_serializing_if = "Option::is_none")]
    pub details: Option<serde_json::Value>,
}

/// Status of a health check
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum CheckStatus {
    /// Check passed
    Pass,
    /// Warning - non-critical issue
    Warn,
    /// Failure - critical issue
    Fail,
}

/// Overall health check result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DoctorOutput {
    /// Whether the system is healthy overall
    pub healthy: bool,
    /// Individual check results
    pub checks: Vec<DoctorCheck>,
    /// Count of warnings
    pub warnings: usize,
    /// Count of errors
    pub errors: usize,
    /// Number of issues that can be auto-fixed
    pub auto_fixable_issues: usize,
}

/// Result of auto-fix operation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DoctorFixOutput {
    /// Issues that were fixed
    pub fixed: Vec<FixResult>,
    /// Issues that could not be fixed
    pub unable_to_fix: Vec<UnfixableIssue>,
}

/// Result of fixing a single issue
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FixResult {
    /// Issue that was fixed
    pub issue: String,
    /// Action taken
    pub action: String,
    /// Whether the fix succeeded
    pub success: bool,
}

/// Issue that could not be auto-fixed
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UnfixableIssue {
    /// Issue name
    pub issue: String,
    /// Reason why it couldn't be fixed
    pub reason: String,
    /// Manual fix suggestion
    pub suggestion: String,
}

/// Error information for failed queries
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QueryError {
    /// Error code
    pub code: String,
    /// Human-readable error message
    pub message: String,
}

/// Query result for session existence
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionExistsQuery {
    /// Whether the session exists (null if query failed)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub exists: Option<bool>,
    /// Session details if it exists
    #[serde(skip_serializing_if = "Option::is_none")]
    pub session: Option<SessionInfo>,
    /// Error information if query failed
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<QueryError>,
}

/// Basic session information for queries
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionInfo {
    /// Session name
    pub name: String,
    /// Session status
    pub status: String,
}

/// Query result for session count
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionCountQuery {
    /// Number of sessions matching filter (null if query failed)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub count: Option<usize>,
    /// Filter that was applied
    #[serde(skip_serializing_if = "Option::is_none")]
    pub filter: Option<serde_json::Value>,
    /// Error information if query failed
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<QueryError>,
}

/// Query result for "can run" check
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CanRunQuery {
    /// Whether the command can be run
    pub can_run: bool,
    /// Command being checked
    pub command: String,
    /// Prerequisites that are blocking execution
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub blockers: Vec<Blocker>,
    /// Number of prerequisites met
    pub prerequisites_met: usize,
    /// Total number of prerequisites
    pub prerequisites_total: usize,
}

/// A prerequisite that is blocking command execution
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Blocker {
    /// Check name
    pub check: String,
    /// Check status (should be false)
    pub status: bool,
    /// Human-readable message
    pub message: String,
}

/// Query result for name suggestion
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SuggestNameQuery {
    /// Pattern used
    pub pattern: String,
    /// Suggested name
    pub suggested: String,
    /// Next available number in sequence
    pub next_available_n: usize,
    /// Existing names matching pattern
    pub existing_matches: Vec<String>,
}

impl IntrospectOutput {
    /// Create default introspection output
    pub fn new(version: &str) -> Self {
        Self {
            jjz_version: version.to_string(),
            capabilities: Capabilities::default(),
            dependencies: HashMap::new(),
            system_state: SystemState::default(),
        }
    }
}

impl Default for Capabilities {
    fn default() -> Self {
        Self {
            session_management: CapabilityCategory {
                commands: vec![
                    "init".to_string(),
                    "add".to_string(),
                    "remove".to_string(),
                    "list".to_string(),
                    "status".to_string(),
                    "focus".to_string(),
                    "sync".to_string(),
                ],
                features: vec![
                    "parallel_workspaces".to_string(),
                    "zellij_integration".to_string(),
                    "hook_lifecycle".to_string(),
                ],
            },
            configuration: CapabilityCategory {
                commands: vec![],
                features: vec![
                    "hierarchy".to_string(),
                    "placeholder_substitution".to_string(),
                ],
            },
            version_control: CapabilityCategory {
                commands: vec!["diff".to_string()],
                features: vec![
                    "jj_integration".to_string(),
                    "workspace_isolation".to_string(),
                ],
            },
            introspection: CapabilityCategory {
                commands: vec![
                    "introspect".to_string(),
                    "doctor".to_string(),
                    "query".to_string(),
                ],
                features: vec![
                    "capability_discovery".to_string(),
                    "health_checks".to_string(),
                    "auto_fix".to_string(),
                    "state_queries".to_string(),
                ],
            },
        }
    }
}

impl Prerequisites {
    /// Check if all prerequisites are met
    pub const fn all_met(&self) -> bool {
        self.initialized && self.jj_installed && (!self.zellij_running || self.custom.is_empty())
    }

    /// Count how many prerequisites are met
    pub const fn count_met(&self) -> usize {
        let mut count = 0;
        if self.initialized {
            count += 1;
        }
        if self.jj_installed {
            count += 1;
        }
        if self.zellij_running {
            count += 1;
        }
        count
    }

    /// Total number of prerequisites
    pub const fn total(&self) -> usize {
        3 + self.custom.len()
    }
}

impl DoctorOutput {
    /// Calculate summary statistics from checks
    pub fn from_checks(checks: Vec<DoctorCheck>) -> Self {
        let warnings = checks
            .iter()
            .filter(|c| c.status == CheckStatus::Warn)
            .count();
        let errors = checks
            .iter()
            .filter(|c| c.status == CheckStatus::Fail)
            .count();
        let auto_fixable_issues = checks.iter().filter(|c| c.auto_fixable).count();
        let healthy = errors == 0;

        Self {
            healthy,
            checks,
            warnings,
            errors,
            auto_fixable_issues,
        }
    }
}

/// Parse a name pattern and suggest next available name
///
/// Pattern format: `prefix-{n}` or `{n}-suffix` where {n} is a number placeholder
#[allow(clippy::literal_string_with_formatting_args)]
pub fn suggest_name(pattern: &str, existing_names: &[String]) -> Result<SuggestNameQuery> {
    // Find {n} placeholder
    if !pattern.contains("{n}") {
        return Err(Error::ValidationError(
            "Pattern must contain {n} placeholder".into(),
        ));
    }

    // Extract prefix and suffix
    let parts: Vec<&str> = pattern.split("{n}").collect();
    if parts.len() != 2 {
        return Err(Error::ValidationError(
            "Pattern must contain exactly one {n} placeholder".into(),
        ));
    }

    let prefix = parts[0];
    let suffix = parts[1];

    // Find all numbers used in matching names
    let mut used_numbers = Vec::new();
    let mut matching = Vec::new();

    for name in existing_names {
        if name.starts_with(prefix) && name.ends_with(suffix) {
            let num_part = &name[prefix.len()..name.len() - suffix.len()];
            if let Ok(n) = num_part.parse::<usize>() {
                used_numbers.push(n);
                matching.push(name.clone());
            }
        }
    }

    // Find next available number
    let next_n = (1..=used_numbers.len() + 2)
        .find(|n| !used_numbers.contains(n))
        .unwrap_or(1);

    let suggested = pattern.replace("{n}", &next_n.to_string());

    Ok(SuggestNameQuery {
        pattern: pattern.to_string(),
        suggested,
        next_available_n: next_n,
        existing_matches: matching,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_introspect_output_new() {
        let output = IntrospectOutput::new("0.1.0");
        assert_eq!(output.jjz_version, "0.1.0");
        assert!(!output.capabilities.session_management.commands.is_empty());
    }

    #[test]
    fn test_capabilities_default() {
        let caps = Capabilities::default();
        assert!(caps
            .session_management
            .commands
            .contains(&"add".to_string()));
        assert!(caps.introspection.commands.contains(&"doctor".to_string()));
    }

    #[test]
    fn test_prerequisites_all_met() {
        let prereqs = Prerequisites {
            initialized: true,
            jj_installed: true,
            zellij_running: true,
            custom: vec![],
        };
        assert!(prereqs.all_met());
    }

    #[test]
    fn test_prerequisites_not_met() {
        let prereqs = Prerequisites {
            initialized: false,
            jj_installed: true,
            zellij_running: true,
            custom: vec![],
        };
        assert!(!prereqs.all_met());
    }

    #[test]
    fn test_prerequisites_count() {
        let prereqs = Prerequisites {
            initialized: true,
            jj_installed: true,
            zellij_running: false,
            custom: vec![],
        };
        assert_eq!(prereqs.count_met(), 2);
        assert_eq!(prereqs.total(), 3);
    }

    #[test]
    fn test_doctor_output_from_checks() {
        let checks = vec![
            DoctorCheck {
                name: "Check 1".to_string(),
                status: CheckStatus::Pass,
                message: "OK".to_string(),
                suggestion: None,
                auto_fixable: false,
                details: None,
            },
            DoctorCheck {
                name: "Check 2".to_string(),
                status: CheckStatus::Warn,
                message: "Warning".to_string(),
                suggestion: Some("Fix it".to_string()),
                auto_fixable: true,
                details: None,
            },
            DoctorCheck {
                name: "Check 3".to_string(),
                status: CheckStatus::Fail,
                message: "Error".to_string(),
                suggestion: None,
                auto_fixable: false,
                details: None,
            },
        ];

        let output = DoctorOutput::from_checks(checks);
        assert!(!output.healthy);
        assert_eq!(output.warnings, 1);
        assert_eq!(output.errors, 1);
        assert_eq!(output.auto_fixable_issues, 1);
    }

    #[test]
    fn test_suggest_name_basic() -> Result<()> {
        let existing = vec!["feature-1".to_string(), "feature-2".to_string()];
        let result = suggest_name("feature-{n}", &existing)?;
        assert_eq!(result.suggested, "feature-3");
        assert_eq!(result.next_available_n, 3);
        assert_eq!(result.existing_matches.len(), 2);
        Ok(())
    }

    #[test]
    fn test_suggest_name_gap() -> Result<()> {
        let existing = vec!["test-1".to_string(), "test-3".to_string()];
        let result = suggest_name("test-{n}", &existing)?;
        assert_eq!(result.suggested, "test-2");
        assert_eq!(result.next_available_n, 2);
        Ok(())
    }

    #[test]
    fn test_suggest_name_no_existing() -> Result<()> {
        let existing = vec![];
        let result = suggest_name("bug-{n}", &existing)?;
        assert_eq!(result.suggested, "bug-1");
        assert_eq!(result.next_available_n, 1);
        assert_eq!(result.existing_matches.len(), 0);
        Ok(())
    }

    #[test]
    fn test_suggest_name_invalid_pattern() {
        let result = suggest_name("no-placeholder", &[]);
        assert!(result.is_err());
    }

    #[test]
    fn test_suggest_name_multiple_placeholders() {
        let result = suggest_name("test-{n}-{n}", &[]);
        assert!(result.is_err());
    }
}
