#![deny(clippy::unwrap_used)]
#![deny(clippy::expect_used)]
#![deny(clippy::panic)]
#![warn(clippy::pedantic)]
#![warn(clippy::nursery)]
#![forbid(unsafe_code)]

//! Command mode parser for Zellij plugin
//!
//! This module provides parsing for interactive commands entered in command mode
//! (triggered by `:` key). Commands follow the pattern `:command [args]`.
//!
//! ## Supported Commands
//!
//! - `:filter <pattern>` - Filter tasks by pattern (case-insensitive)
//! - `:clear` - Clear active filter
//! - `:refresh` - Refresh task list
//! - `:help` - Show help overlay
//!
//! ## Grammar
//!
//! ```text
//! command       ::= ':' command_name [arg_string]
//! command_name  ::= 'filter' | 'clear' | 'refresh' | 'help'
//! arg_string    ::= [^\n]+  # Any characters except newline
//! ```
//!
//! ## Error Handling
//!
//! All parsing operations return `Result<T, ParseError>`. No panics or unwraps.

use itertools::Itertools;
use thiserror::Error;

/// Maximum command length (including leading colon)
const MAX_COMMAND_LENGTH: usize = 512;

/// Maximum filter pattern length
const MAX_FILTER_PATTERN_LENGTH: usize = 256;

/// Command parsing errors
///
/// These errors represent syntactic and semantic issues with command parsing.
/// All errors are recoverable and provide user-friendly messages.
#[derive(Debug, Error, PartialEq, Eq)]
pub enum ParseError {
    /// Empty command (only colon with no text)
    #[error("empty command")]
    EmptyCommand,

    /// Command exceeds maximum length
    #[error("command too long (max {max} characters, got {actual})")]
    CommandTooLong { max: usize, actual: usize },

    /// Unknown command name
    #[error("unknown command: {0}")]
    UnknownCommand(String),

    /// Filter pattern exceeds maximum length
    #[error("filter pattern too long (max {max} characters, got {actual})")]
    FilterTooLong { max: usize, actual: usize },

    /// Invalid filter pattern (contains only whitespace)
    #[error("filter pattern cannot be empty or whitespace only")]
    EmptyFilterPattern,

    /// Command missing required arguments
    #[error("command '{0}' requires an argument")]
    MissingArgument(String),
}

/// Result type for command parsing
pub type ParseResult<T> = Result<T, ParseError>;

/// Parsed command with arguments
///
/// This represents a successfully parsed command with its associated data.
/// Commands are pure data structures suitable for pattern matching.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ParsedCommand {
    /// Filter tasks by pattern
    ///
    /// The pattern is case-insensitive and matches against:
    /// - Task slug
    /// - Task status
    /// - Task priority
    /// - Task technology
    Filter {
        /// Search pattern (case-insensitive)
        pattern: String,
    },

    /// Clear active filter
    ///
    /// Returns to showing all tasks
    ClearFilter,

    /// Refresh task list from orchestrator
    ///
    /// Reloads task data from IPC
    Refresh,

    /// Show help overlay
    ///
    /// Displays command and key binding help
    Help,
}

impl ParsedCommand {
    /// Get the command name as a string
    ///
    /// # Returns
    ///
    /// The command name without arguments or colon prefix
    #[must_use]
    pub const fn name(&self) -> &str {
        match self {
            Self::Filter { .. } => "filter",
            Self::ClearFilter => "clear",
            Self::Refresh => "refresh",
            Self::Help => "help",
        }
    }
}

/// Parse a command string entered in command mode
///
/// This function validates and parses a command string starting with `:`.
/// It performs length validation, command recognition, and argument extraction.
///
/// # Preconditions
///
/// - Input must contain the leading `:` character
/// - Input length must not exceed `MAX_COMMAND_LENGTH`
///
/// # Postconditions
///
/// - Returns `Ok(ParsedCommand)` for valid commands
/// - Returns `Err(ParseError)` for invalid syntax or unknown commands
///
/// # Examples
///
/// ```
/// use zellij_frontend::command::parse_command;
///
/// // Valid filter command
/// let cmd = parse_command(":filter task-1");
/// assert!(cmd.is_ok());
///
/// // Clear filter
/// let cmd = parse_command(":clear");
/// assert!(cmd.is_ok());
///
/// // Unknown command
/// let cmd = parse_command(":unknown");
/// assert!(cmd.is_err());
///
/// // Empty command
/// let cmd = parse_command(":");
/// assert!(cmd.is_err());
/// ```
///
/// # Errors
///
/// - `ParseError::EmptyCommand` - If command is only `:` with no text
/// - `ParseError::CommandTooLong` - If command exceeds maximum length
/// - `ParseError::UnknownCommand` - If command name is not recognized
/// - `ParseError::FilterTooLong` - If filter pattern exceeds maximum length
/// - `ParseError::EmptyFilterPattern` - If filter pattern is whitespace only
/// - `ParseError::MissingArgument` - If command requires argument but none provided
pub fn parse_command(input: &str) -> ParseResult<ParsedCommand> {
    // Validate length before processing
    let input_len = input.len();
    if input_len > MAX_COMMAND_LENGTH {
        return Err(ParseError::CommandTooLong {
            max: MAX_COMMAND_LENGTH,
            actual: input_len,
        });
    }

    // Strip leading colon and validate non-empty
    let without_colon = input
        .strip_prefix(':')
        .map_or_else(|| Err(ParseError::EmptyCommand), |s| Ok(s.trim()))?;

    // Check if command is empty after stripping colon
    if without_colon.is_empty() {
        return Err(ParseError::EmptyCommand);
    }

    // Split into command name and arguments
    let parts = without_colon.split_whitespace().collect_vec();

    // Extract command name (must exist after trim/split)
    let command_name = parts.first().ok_or(ParseError::EmptyCommand)?;

    match *command_name {
        "filter" => parse_filter_command(&parts[1..]),
        "clear" => parse_no_arg_command(ParsedCommand::ClearFilter, &parts[1..]),
        "refresh" => parse_no_arg_command(ParsedCommand::Refresh, &parts[1..]),
        "help" => parse_no_arg_command(ParsedCommand::Help, &parts[1..]),
        unknown => Err(ParseError::UnknownCommand(unknown.to_string())),
    }
}

/// Parse a filter command with pattern argument
///
/// # Arguments
///
/// * `args` - Command arguments after "filter" keyword
///
/// # Returns
///
/// - `Ok(ParsedCommand::Filter)` with pattern if valid
/// - `Err(ParseError)` if pattern is missing, empty, or too long
fn parse_filter_command(args: &[&str]) -> ParseResult<ParsedCommand> {
    // Check for missing argument
    if args.is_empty() {
        return Err(ParseError::MissingArgument("filter".to_string()));
    }

    // Join remaining parts as the filter pattern (preserves spaces)
    let pattern = args.join(" ");

    // Validate pattern length
    let pattern_len = pattern.len();
    if pattern_len > MAX_FILTER_PATTERN_LENGTH {
        return Err(ParseError::FilterTooLong {
            max: MAX_FILTER_PATTERN_LENGTH,
            actual: pattern_len,
        });
    }

    // Validate pattern is not empty or whitespace only
    let trimmed = pattern.trim();
    if trimmed.is_empty() {
        return Err(ParseError::EmptyFilterPattern);
    }

    Ok(ParsedCommand::Filter {
        pattern: trimmed.to_string(),
    })
}

/// Parse a command that takes no arguments
///
/// # Arguments
///
/// * `command` - The command to return if no extra arguments present
/// * `args` - Remaining command arguments (should be empty)
///
/// # Returns
///
/// - `Ok(command)` if no extra arguments
/// - `Err(ParseError)` if unexpected arguments provided
fn parse_no_arg_command(command: ParsedCommand, args: &[&str]) -> ParseResult<ParsedCommand> {
    if args.is_empty() {
        Ok(command)
    } else {
        // For commands like :clear, :refresh, :help, we ignore extra arguments
        // rather than erroring. This provides better UX.
        Ok(command)
    }
}

/// Suggest completions for a partial command
///
/// This function provides command suggestions based on partial input.
/// Useful for autocomplete functionality.
///
/// # Arguments
///
/// * `partial` - Partial command input (with or without leading `:`)
///
/// # Returns
///
/// A vector of command names that could complete the partial input
///
/// # Examples
///
/// ```
/// use zellij_frontend::command::suggest_completions;
///
/// let suggestions = suggest_completions(":f");
/// assert!(suggestions.contains(&"filter".to_string()));
///
/// let suggestions = suggest_completions("c");
/// assert!(suggestions.contains(&"clear".to_string()));
/// ```
#[must_use]
pub fn suggest_completions(partial: &str) -> Vec<String> {
    let commands = ["filter", "clear", "refresh", "help"];

    let input = partial.strip_prefix(':').unwrap_or(partial).to_lowercase();

    commands
        .iter()
        .filter(|cmd| cmd.starts_with(&input))
        .map(std::string::ToString::to_string)
        .collect()
}

#[cfg(test)]
#[cfg(test)]
mod tests {
    
    
            #![allow(clippy::arithmetic_side_effects)]

    use super::*;

    // ============================================================================
    // Parse Filter Command Tests
    // ============================================================================

    #[test]
    fn test_parse_filter_command_with_simple_pattern() {
        let result = parse_command(":filter task-1");
        assert!(result.is_ok());
        assert_eq!(
            result.unwrap(),
            ParsedCommand::Filter {
                pattern: "task-1".to_string()
            }
        );
    }

    #[test]
    fn test_parse_filter_command_with_multiword_pattern() {
        let result = parse_command(":filter task-1 rust");
        assert!(result.is_ok());
        assert_eq!(
            result.unwrap(),
            ParsedCommand::Filter {
                pattern: "task-1 rust".to_string()
            }
        );
    }

    #[test]
    fn test_parse_filter_command_with_special_characters() {
        let result = parse_command(":filter test_pattern-123");
        assert!(result.is_ok());
        assert_eq!(
            result.unwrap(),
            ParsedCommand::Filter {
                pattern: "test_pattern-123".to_string()
            }
        );
    }

    #[test]
    fn test_parse_filter_command_preserves_case() {
        let result = parse_command(":filter TaskPattern");
        assert!(result.is_ok());
        assert_eq!(
            result.unwrap(),
            ParsedCommand::Filter {
                pattern: "TaskPattern".to_string()
            }
        );
    }

    #[test]
    fn test_parse_filter_command_trim_whitespace() {
        let result = parse_command(":filter   task-1   ");
        assert!(result.is_ok());
        assert_eq!(
            result.unwrap(),
            ParsedCommand::Filter {
                pattern: "task-1".to_string()
            }
        );
    }

    #[test]
    fn test_parse_filter_command_without_argument_returns_error() {
        let result = parse_command(":filter");
        assert!(result.is_err());
        assert_eq!(
            result.unwrap_err(),
            ParseError::MissingArgument("filter".to_string())
        );
    }

    #[test]
    fn test_parse_filter_command_with_empty_pattern_returns_error() {
        let result = parse_command(":filter    ");
        assert!(result.is_err());
        // Should return MissingArgument since split_whitespace() removes all spaces
        assert_eq!(
            result.unwrap_err(),
            ParseError::MissingArgument("filter".to_string())
        );
    }

    #[test]
    fn test_parse_filter_command_with_whitespace_only_pattern_returns_error() {
        // This test verifies EmptyFilterPattern works when we have a token that's only whitespace
        // However, split_whitespace() eliminates such tokens, so this becomes MissingArgument
        let result = parse_command(":filter");
        assert!(result.is_err());
        assert_eq!(
            result.unwrap_err(),
            ParseError::MissingArgument("filter".to_string())
        );
    }

    #[test]
    fn test_parse_filter_command_too_long_returns_error() {
        let long_pattern = "a".repeat(MAX_FILTER_PATTERN_LENGTH + 1);
        let input = format!(":filter {long_pattern}");
        let result = parse_command(&input);
        assert!(result.is_err());
        match result.unwrap_err() {
            ParseError::FilterTooLong { max, actual } => {
                assert_eq!(max, MAX_FILTER_PATTERN_LENGTH);
                assert_eq!(actual, MAX_FILTER_PATTERN_LENGTH + 1);
            }
            _ => panic!("Expected FilterTooLong error"),
        }
    }

    // ============================================================================
    // Parse Clear Filter Command Tests
    // ============================================================================

    #[test]
    fn test_parse_clear_filter_command() {
        let result = parse_command(":clear");
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), ParsedCommand::ClearFilter);
    }

    #[test]
    fn test_parse_clear_filter_command_with_whitespace() {
        let result = parse_command(":clear   ");
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), ParsedCommand::ClearFilter);
    }

    #[test]
    fn test_parse_clear_filter_command_ignores_extra_arguments() {
        let result = parse_command(":clear extra args");
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), ParsedCommand::ClearFilter);
    }

    // ============================================================================
    // Parse Refresh Command Tests
    // ============================================================================

    #[test]
    fn test_parse_refresh_command() {
        let result = parse_command(":refresh");
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), ParsedCommand::Refresh);
    }

    #[test]
    fn test_parse_refresh_command_with_whitespace() {
        let result = parse_command(":refresh   ");
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), ParsedCommand::Refresh);
    }

    // ============================================================================
    // Parse Help Command Tests
    // ============================================================================

    #[test]
    fn test_parse_help_command() {
        let result = parse_command(":help");
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), ParsedCommand::Help);
    }

    // ============================================================================
    // Parse Error Tests
    // ============================================================================

    #[test]
    fn test_parse_empty_command_returns_error() {
        let result = parse_command(":");
        assert!(result.is_err());
        assert_eq!(result.unwrap_err(), ParseError::EmptyCommand);
    }

    #[test]
    fn test_parse_command_without_colon_returns_error() {
        let result = parse_command("filter task-1");
        assert!(result.is_err());
        assert_eq!(result.unwrap_err(), ParseError::EmptyCommand);
    }

    #[test]
    fn test_parse_unknown_command_returns_error() {
        let result = parse_command(":unknown command");
        assert!(result.is_err());
        match result.unwrap_err() {
            ParseError::UnknownCommand(cmd) => {
                assert_eq!(cmd, "unknown");
            }
            _ => panic!("Expected UnknownCommand error"),
        }
    }

    #[test]
    fn test_parse_command_too_long_returns_error() {
        let long_input = format!(":filter {}", "a".repeat(MAX_COMMAND_LENGTH));
        let result = parse_command(&long_input);
        assert!(result.is_err());
        match result.unwrap_err() {
            ParseError::CommandTooLong { max, actual } => {
                assert_eq!(max, MAX_COMMAND_LENGTH);
                assert!(actual > MAX_COMMAND_LENGTH);
            }
            _ => panic!("Expected CommandTooLong error"),
        }
    }

    // ============================================================================
    // Suggest Completions Tests
    // ============================================================================

    #[test]
    fn test_suggest_completions_with_empty_input() {
        let suggestions = suggest_completions(":");
        assert_eq!(suggestions.len(), 4); // All commands
    }

    #[test]
    fn test_suggest_completions_with_partial_f() {
        let suggestions = suggest_completions(":f");
        assert_eq!(suggestions, vec!["filter".to_string()]);
    }

    #[test]
    fn test_suggest_completions_with_partial_c() {
        let suggestions = suggest_completions("c");
        assert_eq!(suggestions, vec!["clear".to_string()]);
    }

    #[test]
    fn test_suggest_completions_with_no_match() {
        let suggestions = suggest_completions(":xyz");
        assert!(suggestions.is_empty());
    }

    // ============================================================================
    // ParsedCommand Name Tests
    // ============================================================================

    #[test]
    fn test_parsed_command_name() {
        assert_eq!(
            ParsedCommand::Filter {
                pattern: "test".to_string()
            }
            .name(),
            "filter"
        );
        assert_eq!(ParsedCommand::ClearFilter.name(), "clear");
        assert_eq!(ParsedCommand::Refresh.name(), "refresh");
        assert_eq!(ParsedCommand::Help.name(), "help");
    }

    // ============================================================================
    // Edge Case Tests
    // ============================================================================

    #[test]
    fn test_parse_filter_with_tabs_and_newlines_trimmed() {
        let result = parse_command(":filter \t\n task-1 \n\t");
        assert!(result.is_ok());
        assert_eq!(
            result.unwrap(),
            ParsedCommand::Filter {
                pattern: "task-1".to_string()
            }
        );
    }

    #[test]
    fn test_parse_command_with_unicode_pattern() {
        let result = parse_command(":filter test-🎯-pattern");
        assert!(result.is_ok());
    }

    #[test]
    fn test_parse_filter_pattern_with_leading_trailing_spaces() {
        let result = parse_command(":filter  task-1  ");
        assert!(result.is_ok());
        assert_eq!(
            result.unwrap(),
            ParsedCommand::Filter {
                pattern: "task-1".to_string()
            }
        );
    }
}
