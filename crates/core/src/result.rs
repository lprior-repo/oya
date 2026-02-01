//! Result type definition and extension traits for Railway-Oriented Programming.
//!
//! Provides functional combinators for Result types, enabling clean error handling
//! without unwrap/expect/panic.

use crate::error::Error;

/// The standard Result type for OYA operations.
///
/// All fallible operations in OYA return this type.
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
/// Implements Railway-Oriented Programming patterns for Rust.
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

/// Generic extension trait for any Result type (not just oya_core::Result).
///
/// Provides tap-style combinators for side effects and transformations.
pub trait GenericResultExt<T, E> {
    /// Perform a side effect on Ok value without consuming the Result.
    fn tap_ok<F: FnOnce(&T)>(self, f: F) -> Self;

    /// Perform a side effect on Err value without consuming the Result.
    fn tap_err<F: FnOnce(&E)>(self, f: F) -> Self;

    /// Execute a fallible side effect on Ok, returning the original Result if effect succeeds.
    fn and_then_do<F: FnOnce(&T) -> std::result::Result<(), E>>(
        self,
        f: F,
    ) -> std::result::Result<T, E>;

    /// Map both Ok and Err in a single operation.
    fn bimap<U, F, EF, E2>(self, ok_fn: F, err_fn: EF) -> std::result::Result<U, E2>
    where
        F: FnOnce(T) -> U,
        EF: FnOnce(E) -> E2;

    /// Unwrap with a context message (still returns Result, no panic).
    fn with_context<C: std::fmt::Display, F: FnOnce() -> C>(
        self,
        context: F,
    ) -> std::result::Result<T, String>
    where
        E: std::fmt::Display;
}

impl<T, E> GenericResultExt<T, E> for std::result::Result<T, E> {
    fn tap_ok<F: FnOnce(&T)>(self, f: F) -> Self {
        if let Ok(ref v) = self {
            f(v);
        }
        self
    }

    fn tap_err<F: FnOnce(&E)>(self, f: F) -> Self {
        if let Err(ref e) = self {
            f(e);
        }
        self
    }

    fn and_then_do<F: FnOnce(&T) -> std::result::Result<(), E>>(
        self,
        f: F,
    ) -> std::result::Result<T, E> {
        self.and_then(|v| f(&v).map(|()| v))
    }

    fn bimap<U, F, EF, E2>(self, ok_fn: F, err_fn: EF) -> std::result::Result<U, E2>
    where
        F: FnOnce(T) -> U,
        EF: FnOnce(E) -> E2,
    {
        match self {
            Ok(v) => Ok(ok_fn(v)),
            Err(e) => Err(err_fn(e)),
        }
    }

    fn with_context<C: std::fmt::Display, F: FnOnce() -> C>(
        self,
        context: F,
    ) -> std::result::Result<T, String>
    where
        E: std::fmt::Display,
    {
        self.map_err(|e| format!("{}: {}", context(), e))
    }
}

/// Extension trait for Option types providing Railway-style operations.
pub trait OptionExt<T> {
    /// Convert Option to Result with a lazy error message.
    fn ok_or_else_lazy<E, F: FnOnce() -> E>(self, err: F) -> std::result::Result<T, E>;

    /// Tap into Some value without consuming the Option.
    fn tap_some<F: FnOnce(&T)>(self, f: F) -> Self;

    /// Tap into None without consuming the Option.
    fn tap_none<F: FnOnce()>(self, f: F) -> Self;
}

impl<T> OptionExt<T> for Option<T> {
    fn ok_or_else_lazy<E, F: FnOnce() -> E>(self, err: F) -> std::result::Result<T, E> {
        self.ok_or_else(err)
    }

    fn tap_some<F: FnOnce(&T)>(self, f: F) -> Self {
        if let Some(ref v) = self {
            f(v);
        }
        self
    }

    fn tap_none<F: FnOnce()>(self, f: F) -> Self {
        if self.is_none() {
            f();
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

    // Tests for GenericResultExt
    #[test]
    fn test_tap_ok() {
        let mut observed = 0;
        let result: std::result::Result<i32, &str> = Ok(42);
        let _ = result.tap_ok(|v| observed = *v);
        assert_eq!(observed, 42);
    }

    #[test]
    fn test_tap_err() {
        let mut observed = String::new();
        let result: std::result::Result<i32, &str> = Err("error");
        let _ = result.tap_err(|e| observed = (*e).to_string());
        assert_eq!(observed, "error");
    }

    #[test]
    fn test_bimap_ok() {
        let result: std::result::Result<i32, &str> = Ok(21);
        let mapped = result.bimap(|v| v * 2, |e| e.len());
        assert_eq!(mapped, Ok(42));
    }

    #[test]
    fn test_bimap_err() {
        let result: std::result::Result<i32, &str> = Err("hello");
        let mapped: std::result::Result<i32, usize> = result.bimap(|v| v * 2, |e| e.len());
        assert_eq!(mapped, Err(5));
    }

    #[test]
    fn test_with_context() {
        let result: std::result::Result<i32, &str> = Err("failed");
        let contextualized = result.with_context(|| "operation X");
        assert_eq!(contextualized, Err("operation X: failed".to_string()));
    }

    #[test]
    fn test_and_then_do_success() {
        let mut side_effect = false;
        let result: std::result::Result<i32, &str> = Ok(42);
        let final_result = result.and_then_do(|_| {
            side_effect = true;
            Ok(())
        });
        assert!(side_effect);
        assert_eq!(final_result, Ok(42));
    }

    #[test]
    fn test_and_then_do_effect_fails() {
        let result: std::result::Result<i32, &str> = Ok(42);
        let final_result = result.and_then_do(|_| Err("effect failed"));
        assert_eq!(final_result, Err("effect failed"));
    }

    // Tests for OptionExt
    #[test]
    fn test_tap_some() {
        let mut observed = 0;
        let opt = Some(42);
        let _ = opt.tap_some(|v| observed = *v);
        assert_eq!(observed, 42);
    }

    #[test]
    fn test_tap_none() {
        let mut called = false;
        let opt: Option<i32> = None;
        let _ = opt.tap_none(|| called = true);
        assert!(called);
    }

    #[test]
    fn test_ok_or_else_lazy() {
        let opt: Option<i32> = None;
        let result = opt.ok_or_else_lazy(|| "missing value");
        assert_eq!(result, Err("missing value"));
    }
}
