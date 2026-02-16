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
