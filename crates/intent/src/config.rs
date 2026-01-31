//! Configuration management for Intent CLI
//!
//! Handles loading, validation, and merging of configuration from
//! multiple sources: intent.toml, environment variables, CLI flags.
//!
//! # Philosophy
//!
//! - **Layered configuration**: Files < Env vars < CLI flags
//! - **Validated construction**: All config validated on load
//! - **Zero panics**: Use `Result` for all fallible operations
//! - **Immutable by default**: Config is read-only after construction

#![deny(clippy::unwrap_used)]
#![deny(clippy::expect_used)]
#![deny(clippy::panic)]

use std::{fmt, path::PathBuf, str::FromStr};

use directories::ProjectDirs;
use serde::{Deserialize, Serialize};

use crate::error::{IntentError, IntentResult};

// =============================================================================
// Log Level
// =============================================================================

/// Logging level for Intent CLI
///
/// Controls verbosity of output. Supports case-insensitive parsing from strings.
/// Default level is `Info`.
///
/// # Examples
///
/// ```
/// use std::str::FromStr;
///
/// use intent_core::config::LogLevel;
///
/// let level = LogLevel::from_str("DEBUG").expect("Valid log level");
/// assert_eq!(level, LogLevel::Debug);
/// assert_eq!(level.to_string(), "DEBUG");
///
/// // Default is Info
/// assert_eq!(LogLevel::default(), LogLevel::Info);
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum LogLevel {
    Error,
    Warn,
    Info,
    Debug,
    Trace,
}

impl fmt::Display for LogLevel {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Error => write!(f, "ERROR"),
            Self::Warn => write!(f, "WARN"),
            Self::Info => write!(f, "INFO"),
            Self::Debug => write!(f, "DEBUG"),
            Self::Trace => write!(f, "TRACE"),
        }
    }
}

impl FromStr for LogLevel {
    type Err = IntentError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_uppercase().as_str() {
            "ERROR" => Ok(Self::Error),
            "WARN" => Ok(Self::Warn),
            "INFO" => Ok(Self::Info),
            "DEBUG" => Ok(Self::Debug),
            "TRACE" => Ok(Self::Trace),
            _ => Err(IntentError::validation(
                "log_level",
                format!(
                    "Invalid log level: '{s}'. Must be ERROR, WARN, INFO, DEBUG, or TRACE"
                ),
            )),
        }
    }
}

impl Default for LogLevel {
    fn default() -> Self {
        Self::Info
    }
}

// =============================================================================
// Output Format
// =============================================================================

/// Output format for CLI output
///
/// Determines how command results are formatted and displayed.
///
/// # Examples
///
/// ```
/// use intent_core::config::OutputFormat;
/// use std::str::FromStr;
///
/// // Parsing from string (case-insensitive)
/// assert_eq!(OutputFormat::from_str("text"), Ok(OutputFormat::Text));
/// assert_eq!(OutputFormat::from_str("JSON"), Ok(OutputFormat::Json));
/// assert_eq!(OutputFormat::from_str("Pretty"), Ok(OutputFormat::Pretty));
///
/// // Default value
/// assert_eq!(OutputFormat::default(), OutputFormat::Text);
/// ```
///
/// # Errors
///
/// Returns `ParseOutputFormatError` when parsing an invalid format string.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum OutputFormat {
    /// Plain text output (default)
    Text,
    /// Compact JSON output
    Json,
    /// Pretty-printed JSON output with indentation
    Pretty,
}

/// Error type for parsing OutputFormat from string
#[derive(Debug, Error, Clone, PartialEq, Eq)]
#[error("invalid output format: '{input}'. Valid formats: text, json, pretty")]
pub struct ParseOutputFormatError {
    /// The invalid input string
    pub input: String,
}

impl ParseOutputFormatError {
    /// Create a new parse error
    #[must_use]
    pub fn new(input: impl Into<String>) -> Self {
        Self {
            input: input.into(),
        }
    }
}

impl Default for OutputFormat {
    fn default() -> Self {
        Self::Text
    }
}

impl FromStr for OutputFormat {
    type Err = ParseOutputFormatError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "text" => Ok(Self::Text),
            "json" => Ok(Self::Json),
            "pretty" => Ok(Self::Pretty),
            _ => Err(ParseOutputFormatError::new(s)),
        }
    }
}

impl OutputFormat {
    /// Check if this format outputs JSON
    #[must_use]
    pub const fn is_json(&self) -> bool {
        matches!(self, Self::Json | Self::Pretty)
    }

    /// Get the string representation of this format
    #[must_use]
    pub const fn as_str(&self) -> &'static str {
        match self {
            Self::Text => "text",
            Self::Json => "json",
            Self::Pretty => "pretty",
        }
    }
}
// =============================================================================
// Configuration
// =============================================================================

/// Main configuration struct
///
/// Configuration is loaded and merged from multiple sources in order:
/// 1. Default values
/// 2. intent.toml file
/// 3. Environment variables (INTENT_*)
/// 4. CLI flags
///
/// Later sources override earlier ones.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    // Will be filled in by subsequent beads
}

// =============================================================================
// XDG Path Resolution (Railway-Oriented Programming)
// =============================================================================

/// Resolves the configuration file path using XDG Base Directory specification.
///
/// Search order:
/// 1. `~/.config/intent/config.toml` (XDG config directory)
/// 2. `./intent.toml` (current directory fallback)
///
/// # Philosophy
///
/// - **Zero panics**: Returns `Result` for all path operations
/// - **Railway-oriented**: Uses combinators to chain fallible operations
/// - **Functional core**: Pure function with no side effects beyond path construction
///
/// # Examples
///
/// ```
/// # use intent_core::config::get_config_path;
/// # use intent_core::error::IntentResult;
/// fn load_config() -> IntentResult<String> {
///     let path = get_config_path()?;
///     std::fs::read_to_string(&path)
///         .map_err(|e| intent_core::error::IntentError::not_found(path, e))
/// }
/// ```
///
/// # Errors
///
/// Returns `IntentError::Config` if:
/// - XDG config directory cannot be determined (rare - only on unsupported platforms)
/// - Path construction fails
///
/// # Returns
///
/// - `Ok(PathBuf)` - Path to the config file (may or may not exist)
/// - `Err(IntentError)` - If path resolution fails
pub fn get_config_path() -> IntentResult<PathBuf> {
    get_xdg_config_path()
        .or_else(|_| get_local_config_path())
        .map_err(|e| {
            IntentError::config_for_key(
                "config_path",
                format!("Failed to resolve config path: {e}"),
            )
        })
}

/// Attempts to get the XDG config directory path for Intent.
///
/// Returns `~/.config/intent/config.toml` on Unix-like systems.
///
/// # Railway Pattern
///
/// This is the first track in our railway - if XDG fails, we switch to local path.
///
/// # Errors
///
/// Returns `IntentError::Config` if XDG directories cannot be determined.
fn get_xdg_config_path() -> IntentResult<PathBuf> {
    ProjectDirs::from("", "", "intent")
        .map(|dirs| dirs.config_dir().join("config.toml"))
        .ok_or_else(|| {
            IntentError::config("Unable to determine XDG config directory for platform")
        })
}

/// Fallback to local directory config path.
///
/// Returns `./intent.toml` in the current working directory.
///
/// # Railway Pattern
///
/// This is the fallback track - always succeeds as it uses a relative path.
///
/// # Errors
///
/// This function cannot fail - it always returns a valid path.
/// The path may not exist, but construction is infallible.
fn get_local_config_path() -> IntentResult<PathBuf> {
    Ok(PathBuf::from("./intent.toml"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn config_module_compiles() {
        // Smoke test - module exists and compiles
    }

    // =========================================================================
    // LogLevel Tests (TDD - BEAD: intent-cli-o9d9)
    // =========================================================================

    #[test]
    fn test_log_level_from_str() {
        // Valid cases - uppercase
        assert_eq!("ERROR".parse::<LogLevel>().ok(), Some(LogLevel::Error));
        assert_eq!("WARN".parse::<LogLevel>().ok(), Some(LogLevel::Warn));
        assert_eq!("INFO".parse::<LogLevel>().ok(), Some(LogLevel::Info));
        assert_eq!("DEBUG".parse::<LogLevel>().ok(), Some(LogLevel::Debug));
        assert_eq!("TRACE".parse::<LogLevel>().ok(), Some(LogLevel::Trace));
    }

    #[test]
    fn log_level_from_str_case_insensitive() {
        // Lowercase
        assert_eq!("error".parse::<LogLevel>().ok(), Some(LogLevel::Error));
        assert_eq!("warn".parse::<LogLevel>().ok(), Some(LogLevel::Warn));
        assert_eq!("info".parse::<LogLevel>().ok(), Some(LogLevel::Info));
        assert_eq!("debug".parse::<LogLevel>().ok(), Some(LogLevel::Debug));
        assert_eq!("trace".parse::<LogLevel>().ok(), Some(LogLevel::Trace));

        // Mixed case
        assert_eq!("Error".parse::<LogLevel>().ok(), Some(LogLevel::Error));
        assert_eq!("WaRn".parse::<LogLevel>().ok(), Some(LogLevel::Warn));
        assert_eq!("iNfO".parse::<LogLevel>().ok(), Some(LogLevel::Info));
    }

    #[test]
    fn log_level_from_str_invalid() {
        let result = "INVALID".parse::<LogLevel>();
        assert!(result.is_err());
        if let Err(err) = result {
            assert!(err.to_string().contains("Invalid log level"));
        }
    }

    #[test]
    fn log_level_from_str_empty() {
        let result = "".parse::<LogLevel>();
        assert!(result.is_err());
    }

    #[test]
    fn log_level_display() {
        assert_eq!(LogLevel::Error.to_string(), "ERROR");
        assert_eq!(LogLevel::Warn.to_string(), "WARN");
        assert_eq!(LogLevel::Info.to_string(), "INFO");
        assert_eq!(LogLevel::Debug.to_string(), "DEBUG");
        assert_eq!(LogLevel::Trace.to_string(), "TRACE");
    }

    #[test]
    fn log_level_default() {
        assert_eq!(LogLevel::default(), LogLevel::Info);
    }

    #[test]
    fn log_level_equality() {
        assert_eq!(LogLevel::Info, LogLevel::Info);
        assert_ne!(LogLevel::Info, LogLevel::Debug);
    }

    #[test]
    fn log_level_clone() {
        let level = LogLevel::Debug;
        let cloned = level;
        assert_eq!(level, cloned);
    }

    #[test]
    fn log_level_debug_format() {
        let level = LogLevel::Info;
        let debug_str = format!("{level:?}");
        assert!(debug_str.contains("Info"));
    }

    #[test]
    fn log_level_all_variants() {
        let levels = vec![
            LogLevel::Error,
            LogLevel::Warn,
            LogLevel::Info,
            LogLevel::Debug,
            LogLevel::Trace,
        ];

        // Ensure all variants can be created
        assert_eq!(levels.len(), 5);

        // Ensure all variants are unique
        for (i, level1) in levels.iter().enumerate() {
            for (j, level2) in levels.iter().enumerate() {
                if i == j {
                    assert_eq!(level1, level2);
                } else {
                    assert_ne!(level1, level2);
                }
            }
        }
    }

    #[test]
    fn log_level_round_trip() {
        // Test that parsing and displaying are inverses for uppercase
        let levels = vec![
            LogLevel::Error,
            LogLevel::Warn,
            LogLevel::Info,
            LogLevel::Debug,
            LogLevel::Trace,
        ];

        for level in levels {
            let string = level.to_string();
            let parsed = string.parse::<LogLevel>().ok();
            assert_eq!(parsed, Some(level));
        }
    }

    #[test]
    fn log_level_serde_lowercase() {
        // Test serde serialization uses lowercase (due to #[serde(rename_all = "lowercase")])
        let level = LogLevel::Debug;
        let json = serde_json::to_string(&level).ok();
        assert_eq!(json, Some("\"debug\"".to_string()));

        // Test deserialization from lowercase
        let deserialized: Result<LogLevel, _> = serde_json::from_str("\"info\"");
        assert_eq!(deserialized.ok(), Some(LogLevel::Info));
    }

    // =========================================================================
    // TDD: XDG Path Resolution Tests (BEAD: intent-cli-9bn8)
    // =========================================================================

    #[test]
    fn test_xdg_paths_returns_result() {
        // TDD: Verify that get_config_path returns a Result (no panics)
        let result = get_config_path();
        assert!(result.is_ok(), "get_config_path should return Ok(PathBuf)");
    }

    #[test]
    fn test_xdg_config_path_contains_intent() {
        // TDD: Verify XDG path contains "intent" directory
        let result = get_xdg_config_path();

        // XDG should succeed on most platforms
        if let Ok(path) = result {
            let path_str = path.to_string_lossy();
            assert!(
                path_str.contains("intent"),
                "XDG config path should contain 'intent': {path_str}"
            );
            assert!(
                path_str.ends_with("config.toml"),
                "XDG config path should end with 'config.toml': {path_str}"
            );
        }
    }

    #[test]
    fn test_local_config_path_is_relative() {
        // TDD: Verify local fallback path is ./intent.toml
        let result = get_local_config_path();

        assert!(result.is_ok(), "Local config path should always succeed");

        // Safe to use expect here in tests - we just verified is_ok()
        if let Ok(path) = result {
            assert_eq!(
                path,
                PathBuf::from("./intent.toml"),
                "Local config path should be ./intent.toml"
            );
        }
    }

    #[test]
    fn test_get_config_path_never_panics() {
        // TDD: Critical safety test - function must never panic
        // This is the core of our zero-panic philosophy
        let result = get_config_path();

        // We don't care if it's Ok or Err, just that it didn't panic
        match result {
            Ok(path) => {
                // Should be a valid PathBuf
                assert!(
                    !path.as_os_str().is_empty(),
                    "Returned path should not be empty"
                );
            }
            Err(e) => {
                // Error case is valid too - just verify it's the right type
                assert!(
                    matches!(e, IntentError::Config { .. }),
                    "Error should be Config variant: {e:?}"
                );
            }
        }
    }

    #[test]
    fn test_xdg_path_structure_on_unix() {
        // TDD: On Unix-like systems, verify path follows XDG spec
        #[cfg(unix)]
        {
            let result = get_xdg_config_path();

            if let Ok(path) = result {
                let path_str = path.to_string_lossy();

                // Should be in home directory config
                // Note: We can't assert exact path as it depends on user,
                // but we can verify structure
                assert!(
                    path_str.contains("config") || path_str.contains(".config"),
                    "Unix XDG path should contain config directory: {path_str}"
                );
            }
        }
    }

    #[test]
    fn test_config_path_is_deterministic() {
        // TDD: Calling get_config_path multiple times should return same path
        let path1 = get_config_path();
        let path2 = get_config_path();

        assert_eq!(
            path1.is_ok(),
            path2.is_ok(),
            "Function should be deterministic"
        );

        if let (Ok(p1), Ok(p2)) = (path1, path2) {
            assert_eq!(p1, p2, "Same call should return same path");
        }
    }

    #[test]
    fn test_local_config_path_never_fails() {
        // TDD: Local fallback is infallible
        let result = get_local_config_path();

        assert!(
            result.is_ok(),
            "Local config path construction should never fail"
        );
    }

    #[test]
    fn test_config_path_functional_pipeline() {
        // TDD: Verify railway-oriented programming pattern
        // The function should use .or_else() combinator for fallback

        let result = get_config_path();

        // Since we're testing the functional pipeline, we verify:
        // 1. Function returns Result (railway pattern)
        // 2. Result is usable in further railway chains
        let transformed = result.map(|path| path.join("extra.toml"));

        assert!(
            transformed.is_ok(),
            "Result should be composable in railway chains"
        );
    }

    #[test]
    fn test_error_contains_context() {
        // TDD: If get_config_path fails, error should have meaningful context

        // We can't force a failure easily, but we can verify error type
        // by checking that IntentError::config_for_key creates proper context
        let error = IntentError::config_for_key("test_key", "test message");

        let error_string = error.to_string();
        assert!(
            error_string.contains("test_key") || error_string.contains("test message"),
            "Error should contain context: {error_string}"
        );
    }

    // =========================================================================
    // Property-Based Testing (Stress Test)
    // =========================================================================

    #[test]
    fn test_path_construction_never_panics_stress_test() {
        // TDD: Stress test - call many times to verify no panics
        for _ in 0..1000 {
            let _ = get_config_path();
            let _ = get_xdg_config_path();
            let _ = get_local_config_path();
        }
        // If we get here, no panics occurred
    }

    #[test]
    fn test_railway_pattern_with_or_else() {
        // TDD: Verify the or_else combinator is working correctly
        // This tests the railway pattern implementation

        // get_config_path should try XDG first, then fall back to local
        let config_path = get_config_path();

        // Should always succeed because local fallback never fails
        assert!(
            config_path.is_ok(),
            "Config path should always resolve with fallback"
        );
    }
}
