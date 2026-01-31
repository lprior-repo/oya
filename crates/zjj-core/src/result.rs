//! Result type definition and extension traits.

use crate::error::Error;

/// The standard Result type for ZJJ operations.
///
/// All fallible operations in ZJJ return this type.
/// Use the `?` operator, `match`, or combinator methods to handle results.
///
/// # Examples
///
/// ```ignore
/// // Using the ? operator
/// fn operation() -> Result<String> {
///     let config = Config::builder().build()?;
///     Ok(config.name)
/// }
///
/// // Using match
/// match operation() {
///     Ok(name) => println!("Got: {}", name),
///     Err(e) => eprintln!("Error: {}", e),
/// }
///
/// // Using combinators
/// operation()
///     .map(|name| name.to_uppercase())
///     .unwrap_or_else(|e| {
///         eprintln!("Failed: {}", e);
///         String::default()
///     })
/// ```
pub type Result<T> = std::result::Result<T, Error>;

/// Extension trait providing safe combinators for Results.
///
/// This trait provides ergonomic methods that avoid the need for unwrap/expect.
pub trait ResultExt<T> {
    /// Convert a Result to an Option, logging the error if present.
    fn into_option_logged(self) -> Option<T>;

    /// Get the value or a default, logging the error if present.
    fn or_default_logged(self, default: T) -> T;

    /// Inspect the error without consuming the Result.
    fn inspect_error<F: FnOnce(&Error)>(self, f: F) -> Self;
}

impl<T: std::fmt::Debug> ResultExt<T> for Result<T> {
    fn into_option_logged(self) -> Option<T> {
        match self {
            Ok(value) => Some(value),
            Err(e) => {
                tracing::error!("Operation failed: {}", e);
                None
            }
        }
    }

    fn or_default_logged(self, default: T) -> T {
        match self {
            Ok(value) => value,
            Err(e) => {
                tracing::error!("Operation failed, using default: {}", e);
                default
            }
        }
    }

    fn inspect_error<F: FnOnce(&Error)>(self, f: F) -> Self {
        if let Err(ref e) = self {
            f(e);
        }
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_result_into_option_ok() {
        let result: Result<i32> = Ok(42);
        assert_eq!(result.into_option_logged(), Some(42));
    }

    #[test]
    fn test_result_into_option_err() {
        let result: Result<i32> = Err(Error::Unknown("test".into()));
        assert_eq!(result.into_option_logged(), None);
    }

    #[test]
    fn test_result_or_default_logged_ok() {
        let result: Result<i32> = Ok(42);
        assert_eq!(result.or_default_logged(0), 42);
    }

    #[test]
    fn test_result_or_default_logged_err() {
        let result: Result<i32> = Err(Error::Unknown("test".into()));
        assert_eq!(result.or_default_logged(99), 99);
    }

    #[test]
    fn test_result_inspect_error() {
        let result: Result<i32> = Err(Error::Unknown("test".into()));
        let mut called = false;
        let _ = result.inspect_error(|_| {
            called = true;
        });
        assert!(called);
    }
}
