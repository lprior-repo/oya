//! # ZJJ Core
//!
//! Core functionality for ZJJ - strictly functional Rust with zero unwraps.
//!
//! ## Laws (Compiler Enforced)
//!
//! - No `unwrap()` - returns `Result` instead
//! - No `expect()` - returns `Result` instead
//! - No `panic!()` - returns `Result` instead
//! - No `unsafe` - safe Rust only
//! - No `todo!()` / `unimplemented!()` - complete implementations only
//!
//! ## Error Handling
//!
//! All fallible operations return `Result<T, Error>`. Use:
//! - `?` operator for propagation
//! - `map`, `and_then` combinators for transformation
//! - `unwrap_or_default()`, `unwrap_or_else()` for defaults

pub mod config;
pub mod contracts;
mod error;
pub mod functional;
pub mod hints;
pub mod hooks;
pub mod introspection;
pub mod jj;
pub mod json;
mod result;
pub mod types;
pub mod watcher;
pub mod zellij;

pub use error::Error;
pub use result::{Result, ResultExt};

/// Marker trait for types guaranteed safe (no panics possible).
pub trait Infallible: Sized {}

/// Configuration builder with fallible construction.
#[derive(Debug, Clone, Default)]
pub struct ConfigBuilder {
    name: Option<String>,
}

impl ConfigBuilder {
    /// Create a new config builder.
    #[must_use]
    pub const fn new() -> Self {
        Self { name: None }
    }

    /// Set the configuration name.
    #[must_use]
    pub fn with_name(mut self, name: impl Into<String>) -> Self {
        self.name = Some(name.into());
        self
    }

    /// Build the configuration, returning an error if validation fails.
    pub fn build(self) -> Result<Config> {
        self.name
            .ok_or_else(|| Error::InvalidConfig("name is required".into()))
            .and_then(|name| {
                if name.is_empty() {
                    Err(Error::InvalidConfig("name cannot be empty".into()))
                } else {
                    Ok(Config { name })
                }
            })
    }
}

/// A validated configuration.
#[derive(Debug, Clone, Default)]
pub struct Config {
    /// The configuration name.
    pub name: String,
}

impl Config {
    /// Create a new config builder.
    #[must_use]
    pub const fn builder() -> ConfigBuilder {
        ConfigBuilder::new()
    }

    /// Validate a name, returning an error if invalid.
    pub fn validate_name(name: &str) -> Result<()> {
        if name.is_empty() {
            return Err(Error::InvalidConfig("name cannot be empty".into()));
        }

        if name.len() > 255 {
            return Err(Error::InvalidConfig(
                "name cannot exceed 255 characters".into(),
            ));
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_config_builder_success() {
        let result = ConfigBuilder::new().with_name("test").build();
        assert!(result.is_ok());

        // Use map to extract and check - no unwrap needed
        let name_matches = result.map(|c| c.name == "test").unwrap_or(false);
        assert!(name_matches);
    }

    #[test]
    fn test_config_builder_missing_name() {
        let result = ConfigBuilder::new().build();
        assert!(result.is_err());
    }

    #[test]
    fn test_config_builder_empty_name() {
        let result = ConfigBuilder::new().with_name("").build();
        assert!(result.is_err());
    }

    #[test]
    fn test_validate_name_empty() {
        assert!(Config::validate_name("").is_err());
    }

    #[test]
    fn test_validate_name_too_long() {
        let long_name = "x".repeat(256);
        assert!(Config::validate_name(&long_name).is_err());
    }

    #[test]
    fn test_validate_name_valid() {
        assert!(Config::validate_name("valid_name").is_ok());
    }
}
