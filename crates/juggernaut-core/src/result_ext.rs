//! Railway-Oriented Programming extensions for Result.
//!
//! Provides `tap`, `tap_err`, and async combinators.

use std::future::Future;

/// Railway-Oriented extensions for Result.
pub trait ResultExt<T, E> {
    /// Tap into the success value without consuming it.
    ///
    /// Useful for logging or side effects in a pipeline.
    fn tap<F: FnOnce(&T)>(self, f: F) -> Self;

    /// Tap into the error value without consuming it.
    fn tap_err<F: FnOnce(&E)>(self, f: F) -> Self;

    /// Map the success value, returning a new Result.
    /// This is just `map` but explicit for Railway naming.
    fn railway_map<U, F: FnOnce(T) -> U>(self, f: F) -> Result<U, E>;

    /// Flat map the success value with a fallible operation.
    /// This is just `and_then` but explicit for Railway naming.
    fn railway_bind<U, F: FnOnce(T) -> Result<U, E>>(self, f: F) -> Result<U, E>;
}

impl<T, E> ResultExt<T, E> for Result<T, E> {
    fn tap<F: FnOnce(&T)>(self, f: F) -> Self {
        if let Ok(ref value) = self {
            f(value);
        }
        self
    }

    fn tap_err<F: FnOnce(&E)>(self, f: F) -> Self {
        if let Err(ref err) = self {
            f(err);
        }
        self
    }

    fn railway_map<U, F: FnOnce(T) -> U>(self, f: F) -> Result<U, E> {
        self.map(f)
    }

    fn railway_bind<U, F: FnOnce(T) -> Result<U, E>>(self, f: F) -> Result<U, E> {
        self.and_then(f)
    }
}

/// Async Railway-Oriented extensions.
pub trait AsyncResultExt<T, E> {
    /// Async flat map - chain async operations on success.
    fn and_then_async<U, F, Fut>(self, f: F) -> impl Future<Output = Result<U, E>>
    where
        F: FnOnce(T) -> Fut,
        Fut: Future<Output = Result<U, E>>;

    /// Async map - transform success value asynchronously.
    fn map_async<U, F, Fut>(self, f: F) -> impl Future<Output = Result<U, E>>
    where
        F: FnOnce(T) -> Fut,
        Fut: Future<Output = U>;
}

impl<T, E> AsyncResultExt<T, E> for Result<T, E> {
    async fn and_then_async<U, F, Fut>(self, f: F) -> Result<U, E>
    where
        F: FnOnce(T) -> Fut,
        Fut: Future<Output = Result<U, E>>,
    {
        match self {
            Ok(value) => f(value).await,
            Err(e) => Err(e),
        }
    }

    async fn map_async<U, F, Fut>(self, f: F) -> Result<U, E>
    where
        F: FnOnce(T) -> Fut,
        Fut: Future<Output = U>,
    {
        match self {
            Ok(value) => Ok(f(value).await),
            Err(e) => Err(e),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_tap_on_ok() {
        let mut tapped = false;
        let result: Result<i32, &str> = Ok(42);
        let _ = result.tap(|v| {
            tapped = *v == 42;
        });
        assert!(tapped);
    }

    #[test]
    fn test_tap_on_err() {
        let mut tapped = false;
        let result: Result<i32, &str> = Err("error");
        let _ = result.tap(|_| {
            tapped = true;
        });
        assert!(!tapped); // Should not tap on error
    }

    #[test]
    fn test_tap_err_on_err() {
        let mut tapped = false;
        let result: Result<i32, &str> = Err("error");
        let _ = result.tap_err(|e| {
            tapped = *e == "error";
        });
        assert!(tapped);
    }

    #[test]
    fn test_tap_err_on_ok() {
        let mut tapped = false;
        let result: Result<i32, &str> = Ok(42);
        let _ = result.tap_err(|_| {
            tapped = true;
        });
        assert!(!tapped); // Should not tap_err on success
    }

    #[test]
    fn test_railway_map() {
        let result: Result<i32, &str> = Ok(21);
        let mapped = result.railway_map(|v| v * 2);
        assert_eq!(mapped, Ok(42));
    }

    #[test]
    fn test_railway_bind() {
        let result: Result<i32, &str> = Ok(21);
        let bound = result.railway_bind(|v| Ok(v * 2));
        assert_eq!(bound, Ok(42));
    }

    #[test]
    fn test_railway_bind_propagates_error() {
        let result: Result<i32, &str> = Ok(21);
        let bound: Result<i32, &str> = result.railway_bind(|_| Err("failed"));
        assert_eq!(bound, Err("failed"));
    }

    #[test]
    fn test_pipeline() {
        let result: Result<i32, &str> = Ok(10)
            .railway_map(|v| v + 5)
            .tap(|v| assert_eq!(*v, 15))
            .railway_bind(|v| Ok(v * 2))
            .tap(|v| assert_eq!(*v, 30));

        assert_eq!(result, Ok(30));
    }

    #[tokio::test]
    async fn test_and_then_async() {
        let result: Result<i32, &str> = Ok(21);
        let async_result = result.and_then_async(|v| async move { Ok(v * 2) }).await;
        assert_eq!(async_result, Ok(42));
    }

    #[tokio::test]
    async fn test_map_async() {
        let result: Result<i32, &str> = Ok(21);
        let async_result = result.map_async(|v| async move { v * 2 }).await;
        assert_eq!(async_result, Ok(42));
    }

    #[tokio::test]
    async fn test_and_then_async_propagates_error() {
        let result: Result<i32, &str> = Err("initial error");
        let async_result = result.and_then_async(|v| async move { Ok(v * 2) }).await;
        assert_eq!(async_result, Err("initial error"));
    }
}
