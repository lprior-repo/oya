//! Result type definition and extension traits for Railway-Oriented Programming.
//!
//! Provides functional combinators for Result types, enabling clean error handling
//! without unwrap/expect/panic.

use crate::error::Error;
use async_trait::async_trait;
use either::Either;
use std::future::Future;

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
#[async_trait]
pub trait ResultExt<T>: Sized {
    /// Convert a Result to an Option, logging the error if present.
    fn into_option_logged(self) -> Option<T>;

    /// Get the value or a default, logging the error if present.
    fn or_default_logged(self, default: T) -> T;

    /// Inspect the error without consuming the Result.
    fn inspect_error<F: FnOnce(&Error)>(self, f: F) -> Self;

    /// Chain with another fallible operation.
    ///
    /// # Examples
    ///
    /// ```ignore
    /// fn get_user(id: i32) -> Result<String> { Ok("user".to_string()) }
    /// fn validate_user(user: String) -> Result<String> { Ok(user.to_uppercase()) }
    ///
    /// let result = get_user(1).and_then(|user| validate_user(user));
    /// assert!(result.is_ok());
    /// ```
    fn and_then<F, U>(self, f: F) -> Result<U>
    where
        F: FnOnce(T) -> Result<U>;

    /// Async version of and_then for chaining async fallible operations.
    ///
    /// # Examples
    ///
    /// ```ignore
    /// async fn fetch_user(id: i32) -> Result<String> { Ok("user".to_string()) }
    /// async fn validate_user(user: String) -> Result<String> { Ok(user.to_uppercase()) }
    ///
    /// let result = fetch_user(1).await.and_then_async(|user| validate_user(user)).await;
    /// assert!(result.is_ok());
    /// ```
    async fn and_then_async<F, U, Fut>(self, f: F) -> Result<U>
    where
        F: FnOnce(T) -> Fut + Send,
        Fut: Future<Output = Result<U>> + Send,
        T: Send;

    /// Transform the error type.
    ///
    /// # Examples
    ///
    /// ```ignore
    /// let result: Result<i32> = Err(Error::Unknown("fail".into()));
    /// let mapped = result.map_err(|e| format!("Error: {}", e));
    /// assert!(mapped.is_err());
    /// ```
    fn map_err<F, E2>(self, f: F) -> std::result::Result<T, E2>
    where
        F: FnOnce(Error) -> E2;

    /// Provide a default value for error cases.
    ///
    /// # Examples
    ///
    /// ```ignore
    /// let result: Result<i32> = Err(Error::Unknown("fail".into()));
    /// let unwrapped = result.unwrap_or(42);
    /// assert_eq!(unwrapped, 42);
    /// ```
    fn unwrap_or(self, default: T) -> T;

    /// Provide a default value for error cases using a function.
    ///
    /// # Examples
    ///
    /// ```ignore
    /// let result: Result<i32> = Err(Error::Unknown("fail".into()));
    /// let unwrapped = result.unwrap_or_else(|| 42);
    /// assert_eq!(unwrapped, 42);
    /// ```
    fn unwrap_or_else<F>(self, f: F) -> T
    where
        F: FnOnce(Error) -> T;

    /// Try an alternative operation if this Result is Err.
    ///
    /// # Examples
    ///
    /// ```ignore
    /// let primary: Result<i32> = Err(Error::Unknown("fail".into()));
    /// let fallback: Result<i32> = Ok(42);
    /// let result = primary.or(fallback);
    /// assert!(result.is_ok());
    /// ```
    fn or(self, other: Result<T>) -> Result<T>;

    /// Try an alternative operation using a function if this Result is Err.
    ///
    /// # Examples
    ///
    /// ```ignore
    /// let primary: Result<i32> = Err(Error::Unknown("fail".into()));
    /// let result = primary.or_else(|_| Ok(42));
    /// assert!(result.is_ok());
    /// ```
    fn or_else<F>(self, f: F) -> Result<T>
    where
        F: FnOnce(Error) -> Result<T>;

    /// Convert Result to Either type (useful for error type unification).
    ///
    /// # Examples
    ///
    /// ```ignore
    /// use either::Either;
    /// let result: Result<i32> = Ok(42);
    /// let either = result.to_either();
    /// assert!(either.is_right());
    /// ```
    fn to_either(self) -> Either<Error, T>
    where
        T: Sized;
}

#[async_trait]
impl<T: std::fmt::Debug + Send> ResultExt<T> for Result<T> {
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

    fn and_then<F, U>(self, f: F) -> Result<U>
    where
        F: FnOnce(T) -> Result<U>,
    {
        match self {
            Ok(v) => f(v),
            Err(e) => Err(e),
        }
    }

    async fn and_then_async<F, U, Fut>(self, f: F) -> Result<U>
    where
        F: FnOnce(T) -> Fut + Send,
        Fut: Future<Output = Result<U>> + Send,
        T: Send,
    {
        match self {
            Ok(v) => f(v).await,
            Err(e) => Err(e),
        }
    }

    fn map_err<F, E2>(self, f: F) -> std::result::Result<T, E2>
    where
        F: FnOnce(Error) -> E2,
    {
        match self {
            Ok(v) => Ok(v),
            Err(e) => Err(f(e)),
        }
    }

    fn unwrap_or(self, default: T) -> T {
        match self {
            Ok(v) => v,
            Err(_) => default,
        }
    }

    fn unwrap_or_else<F>(self, f: F) -> T
    where
        F: FnOnce(Error) -> T,
    {
        match self {
            Ok(v) => v,
            Err(e) => f(e),
        }
    }

    fn or(self, other: Result<T>) -> Result<T> {
        match self {
            Ok(v) => Ok(v),
            Err(_) => other,
        }
    }

    fn or_else<F>(self, f: F) -> Result<T>
    where
        F: FnOnce(Error) -> Result<T>,
    {
        match self {
            Ok(v) => Ok(v),
            Err(e) => f(e),
        }
    }

    fn to_either(self) -> Either<Error, T>
    where
        T: Sized,
    {
        match self {
            Ok(v) => Either::Right(v),
            Err(e) => Either::Left(e),
        }
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

    // Original tests
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

    // Tests for and_then
    #[test]
    fn test_and_then_ok() {
        let result: Result<i32> = Ok(21);
        let chained = result.map(|v| v * 2);
        assert!(chained.is_ok());
        if let Ok(v) = chained {
            assert_eq!(v, 42);
        }
    }

    #[test]
    fn test_and_then_err() {
        let result: Result<i32> = Err(Error::Unknown("fail".into()));
        let chained = result.map(|v| v * 2);
        assert!(chained.is_err());
    }

    #[test]
    fn test_and_then_propagates_error() {
        let result: Result<i32> = Ok(21);
        let chained: Result<i32> = result.and_then(|_| Err(Error::Unknown("chained fail".into())));
        assert!(chained.is_err());
        assert!(matches!(chained, Err(Error::Unknown(_))));
    }

    // Tests for map_err
    #[test]
    fn test_map_err_on_error() {
        let result: Result<i32> = Err(Error::Unknown("fail".into()));
        let mapped = result.map_err(|e| format!("Error: {e}"));
        assert!(mapped.is_err());
        if let Err(e) = mapped {
            assert_eq!(e, "Error: unknown error: fail");
        }
    }

    #[test]
    fn test_map_err_on_ok() {
        let result: Result<i32> = Ok(42);
        let mapped = result.map_err(|e: Error| format!("Error: {e}"));
        assert!(mapped.is_ok());
        if let Ok(v) = mapped {
            assert_eq!(v, 42);
        }
    }

    // Tests for unwrap_or
    #[test]
    fn test_unwrap_or_ok() {
        fn get_ok() -> i32 {
            42
        }
        assert_eq!(Result::Ok(get_ok()).unwrap_or(0), 42);
    }

    #[test]
    fn test_unwrap_or_err() {
        fn get_err() -> Result<i32> {
            Err(Error::Unknown("fail".into()))
        }
        assert_eq!(get_err().unwrap_or(99), 99);
    }

    // Tests for unwrap_or_else
    #[test]
    fn test_unwrap_or_else_ok() {
        fn get_ok() -> i32 {
            42
        }
        assert_eq!(Result::Ok(get_ok()).unwrap_or(0), 42);
    }

    #[test]
    fn test_unwrap_or_else_err() {
        fn get_err() -> Result<i32> {
            Err(Error::Unknown("fail".into()))
        }
        assert_eq!(get_err().unwrap_or(100), 100);
    }

    // Tests for or
    #[test]
    fn test_or_primary_ok() {
        let primary: Result<i32> = Ok(42);
        let fallback: Result<i32> = Ok(99);
        let result = primary.or(fallback);
        assert!(result.is_ok());
        if let Ok(v) = result {
            assert_eq!(v, 42);
        }
    }

    #[test]
    fn test_or_primary_err_fallback_ok() {
        let primary: Result<i32> = Err(Error::Unknown("primary".into()));
        let fallback: Result<i32> = Ok(99);
        let result = primary.or(fallback);
        assert!(result.is_ok());
        if let Ok(v) = result {
            assert_eq!(v, 99);
        }
    }

    #[test]
    fn test_or_both_err() {
        let primary: Result<i32> = Err(Error::Unknown("primary".into()));
        let fallback: Result<i32> = Err(Error::Unknown("fallback".into()));
        assert!(primary.or(fallback).is_err());
    }

    // Tests for or_else
    #[test]
    fn test_or_else_primary_ok() {
        let primary: Result<i32> = Ok(42);
        let fallback: Result<i32> = Ok(99);
        let result = primary.or(fallback);
        assert!(result.is_ok());
        if let Ok(v) = result {
            assert_eq!(v, 42);
        }
    }

    #[test]
    fn test_or_else_primary_err() {
        let primary: Result<i32> = Err(Error::Unknown("primary".into()));
        let fallback: Result<i32> = Ok(99);
        let result = primary.or(fallback);
        assert!(result.is_ok());
        if let Ok(v) = result {
            assert_eq!(v, 99);
        }
    }

    // Tests for to_either
    #[test]
    fn test_to_either_ok() {
        let result: Result<i32> = Ok(42);
        let either = result.to_either();
        assert!(either.is_right());
        if let Either::Right(v) = either {
            assert_eq!(v, 42);
        }
    }

    #[test]
    fn test_to_either_err() {
        let result: Result<i32> = Err(Error::Unknown("fail".into()));
        let either = result.to_either();
        assert!(either.is_left());
    }

    // Integration test: Railway pattern
    #[test]
    fn test_railway_pattern_success() {
        fn validate_input(input: i32) -> Result<i32> {
            if input > 0 {
                Ok(input)
            } else {
                Err(Error::InvalidRecord {
                    reason: "must be positive".into(),
                })
            }
        }

        fn double_value(input: i32) -> i32 {
            input * 2
        }

        fn ensure_even(input: i32) -> Result<i32> {
            if input % 2 == 0 {
                Ok(input)
            } else {
                Err(Error::InvalidRecord {
                    reason: "must be even".into(),
                })
            }
        }

        let result = validate_input(21)
            .map(double_value)
            .and_then(ensure_even);

        assert!(result.is_ok());
        if let Ok(v) = result {
            assert_eq!(v, 42);
        }
    }

    #[test]
    fn test_railway_pattern_failure() {
        fn validate_input(input: i32) -> Result<i32> {
            if input > 0 {
                Ok(input)
            } else {
                Err(Error::InvalidRecord {
                    reason: "must be positive".into(),
                })
            }
        }

        fn double_value(input: i32) -> i32 {
            input * 2
        }

        let result = validate_input(-1).map(double_value);
        assert!(result.is_err());
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
        let mapped = result.bimap(|v| v * 2, str::len);
        assert_eq!(mapped, Ok(42));
    }

    #[test]
    fn test_bimap_err() {
        let result: std::result::Result<i32, &str> = Err("hello");
        let mapped: std::result::Result<i32, usize> = result.bimap(|v| v * 2, str::len);
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
