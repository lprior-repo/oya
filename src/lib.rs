#![deny(clippy::unwrap_used)]
#![deny(clippy::expect_used)]
#![deny(clippy::panic)]
#![forbid(unsafe_code)]

//! Design Contract: hello-world
//!
//! ## Purpose and Goals
//! Create a simple function that returns the string "hello world" for demonstration purposes.
//!
//! ## Key Functions to Implement
//! - `hello_world() -> String` - Returns the greeting "hello world"
//!
//! ## Acceptance Criteria
//! - Function returns exact string "hello world"
//! - Function signature matches `fn hello_world() -> String`
//! - Function has no side effects
//! - Function is publicly accessible

pub mod application;
pub mod domain;
pub mod infrastructure;

/// Returns the string "hello world".
///
/// # Examples
/// ```
/// use oya::hello_world;
/// let result = hello_world();
/// assert_eq!(result, "hello world");
/// ```
pub fn hello_world() -> String {
    "hello world".to_string()
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OpencodeParseError {
    message: String,
}

impl OpencodeParseError {
    pub fn new(message: impl Into<String>) -> Self {
        Self { message: message.into() }
    }
}

impl std::fmt::Display for OpencodeParseError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.message)
    }
}

impl std::error::Error for OpencodeParseError {}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OpencodeRunOutput {
    pub stdout: String,
}

pub fn parse_opencode_output(raw: &str) -> Result<OpencodeRunOutput, OpencodeParseError> {
    let trimmed = raw.trim();
    if trimmed.is_empty() {
        return Err(OpencodeParseError::new("opencode output empty"));
    }

    let value: serde_json::Value = serde_json::from_str(trimmed)
        .map_err(|e| OpencodeParseError::new(format!("invalid opencode json: {}", e)))?;

    match value.get("stdout") {
        Some(serde_json::Value::String(stdout)) => {
            Ok(OpencodeRunOutput { stdout: stdout.to_string() })
        }
        Some(_) => Err(OpencodeParseError::new("opencode json stdout is not a string")),
        None => Err(OpencodeParseError::new("opencode json missing stdout field")),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_opencode_output_rejects_empty() {
        let result = parse_opencode_output("  \n\t ");
        assert!(result.is_err());
    }

    #[test]
    fn parse_opencode_output_rejects_invalid_json() {
        let result = parse_opencode_output("not json");
        assert!(result.is_err());
    }

    #[test]
    fn parse_opencode_output_requires_stdout_field() {
        let result = parse_opencode_output("{\"status\":\"ok\"}");
        assert!(result.is_err());
    }

    #[test]
    fn parse_opencode_output_requires_stdout_string() {
        let result = parse_opencode_output("{\"stdout\":123}");
        assert!(result.is_err());
    }

    #[test]
    fn parse_opencode_output_accepts_stdout_string() {
        let result = parse_opencode_output("{\"stdout\":\"ok\"}");
        assert_eq!(result, Ok(OpencodeRunOutput { stdout: "ok".to_string() }));
    }
}
