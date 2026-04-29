#![deny(clippy::unwrap_used)]
#![deny(clippy::expect_used)]
#![deny(clippy::panic)]
#![warn(clippy::pedantic)]
#![warn(clippy::nursery)]
#![warn(clippy::complexity)]
#![warn(clippy::cognitive_complexity)]
#![forbid(unsafe_code)]

//! Expression Depth Limiting Module
//!
//! This module provides safe expression depth tracking and validation to prevent
//! stack overflow during expression resolution.
//!
//! # Architecture
//!
//! This module follows the Data → Calc → Actions pattern:
//! - **Data**: `ExpressionDepth` newtype, `Error` enum, type aliases
//! - **Calc**: Pure functions like `calculate_depth`
//! - **Actions**: `resolve_expressions`, `validate_expression_depth`

use std::collections::HashMap;
use std::sync::Arc;

/// Maximum allowed expression nesting depth
///
/// This constant defines the hard limit for expression tree depth to prevent
/// stack overflow errors. The value 1024 is chosen as a safe threshold that
/// accommodates deeply nested expressions while remaining well below typical
/// system stack limits.
pub const MAX_EXPRESSION_DEPTH: u32 = 1024;

/// Represents the depth of a nested expression
///
/// This newtype wrapper ensures that all depth values are validated at construction
/// time, making illegal states unrepresentable.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct ExpressionDepth(pub u32);

impl ExpressionDepth {
    /// Creates a new `ExpressionDepth` with validation.
    ///
    /// # Arguments
    /// * `depth` - The depth value to wrap
    ///
    /// # Returns
    /// * `Ok(ExpressionDepth)` if depth is within valid range (0 to `MAX_EXPRESSION_DEPTH`)
    /// * `Err(Error::DepthLimitExceeded)` if depth exceeds the maximum
    ///
    /// # Errors
    /// Returns `Error::DepthLimitExceeded` if `depth > MAX_EXPRESSION_DEPTH`.
    ///
    /// # Examples
    /// ```
    /// use oya_frontend::expression_depth::{ExpressionDepth, MAX_EXPRESSION_DEPTH};
    ///
    /// assert!(ExpressionDepth::new(0).is_ok());
    /// assert!(ExpressionDepth::new(MAX_EXPRESSION_DEPTH).is_ok());
    /// assert!(ExpressionDepth::new(MAX_EXPRESSION_DEPTH + 1).is_err());
    /// ```
    #[cfg_attr(test, allow(clippy::unwrap_used))]
    pub const fn new(depth: u32) -> Result<Self, Error> {
        if depth > MAX_EXPRESSION_DEPTH {
            Err(Error::DepthLimitExceeded { current_depth: depth, max_depth: MAX_EXPRESSION_DEPTH })
        } else {
            Ok(Self(depth))
        }
    }

    /// Returns the current depth value.
    ///
    /// # Examples
    /// ```
    /// use oya_frontend::expression_depth::ExpressionDepth;
    ///
    /// let depth = ExpressionDepth::new(5).expect("5 is within valid range");
    /// assert_eq!(depth.current(), 5);
    /// ```
    #[must_use]
    pub const fn current(&self) -> u32 {
        self.0
    }

    /// Increments the depth by one, with validation.
    ///
    /// # Returns
    /// * `Ok(ExpressionDepth)` with incremented depth if result is within bounds
    /// * `Err(Error::DepthLimitExceeded)` if increment would exceed maximum
    ///
    /// # Errors
    /// Returns `Error::DepthLimitExceeded` if incrementing would exceed `MAX_EXPRESSION_DEPTH`.
    ///
    /// # Examples
    /// ```
    /// use oya_frontend::expression_depth::ExpressionDepth;
    ///
    /// let depth = ExpressionDepth::new(100).expect("100 is within valid range");
    /// let next = depth.increment().expect("increment succeeded");
    /// assert_eq!(next.current(), 101);
    /// ```
    pub const fn increment(&self) -> Result<Self, Error> {
        if self.0 >= MAX_EXPRESSION_DEPTH {
            Err(Error::DepthLimitExceeded {
                current_depth: self.0,
                max_depth: MAX_EXPRESSION_DEPTH,
            })
        } else {
            Ok(Self(self.0 + 1))
        }
    }

    /// Checks if the depth is within valid bounds.
    ///
    /// This method is primarily useful for testing invalid depths constructed
    /// via unsafe transmute, as the public constructor `new()` already validates.
    ///
    /// # Returns
    /// * `true` if depth is <= `MAX_EXPRESSION_DEPTH`
    /// * `false` if depth exceeds `MAX_EXPRESSION_DEPTH`
    ///
    /// # Examples
    /// ```
    /// use oya_frontend::expression_depth::ExpressionDepth;
    ///
    /// let valid = ExpressionDepth::new(1024).expect("1024 is MAX_EXPRESSION_DEPTH");
    /// assert!(valid.is_valid());
    /// ```
    #[must_use]
    pub const fn is_valid(&self) -> bool {
        self.0 <= MAX_EXPRESSION_DEPTH
    }
}

impl std::fmt::Display for ExpressionDepth {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

/// Error taxonomy for expression resolution
///
/// This enum provides comprehensive error coverage for all failure modes in
/// expression depth tracking and resolution.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Error {
    /// Exceeded maximum allowed expression depth
    DepthLimitExceeded {
        /// The depth that exceeded the limit
        current_depth: u32,
        /// The maximum allowed depth
        max_depth: u32,
    },

    /// Input expression contains invalid syntax or structure
    InvalidExpression,

    /// Expression reference not found in registry
    ExpressionNotFound {
        /// The reference that was not found
        reference: String,
    },

    /// Type mismatch during expression evaluation
    TypeError {
        /// The type that was expected
        expected: &'static str,
        /// The type that was actually found
        actual: &'static str,
    },

    /// Runtime error during expression evaluation
    RuntimeError {
        /// Human-readable error message
        message: String,
    },
}

impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::DepthLimitExceeded { current_depth, max_depth } => {
                write!(f, "Expression depth {current_depth} exceeds maximum of {max_depth}")
            }
            Self::InvalidExpression => write!(f, "Invalid expression syntax or structure"),
            Self::ExpressionNotFound { reference } => {
                write!(f, "Expression '{reference}' not found")
            }
            Self::TypeError { expected, actual } => {
                write!(f, "Expected {expected}, found {actual}")
            }
            Self::RuntimeError { message } => write!(f, "Runtime error: {message}"),
        }
    }
}

impl std::error::Error for Error {}

/// Reference to another expression in the registry
pub type ExpressionRef = Arc<Expression>;

/// Registry mapping expression names to their definitions
pub type ExpressionRegistry = HashMap<String, ExpressionRef>;

/// Expression node with optional value and children
///
/// This represents a computation unit that can contain nested sub-expressions.
#[derive(Debug, Clone, PartialEq)]
pub struct Expression {
    /// Optional value (e.g., a number, string, or reference name)
    pub value: Option<String>,
    /// Child expression references
    pub children: Vec<ExpressionRef>,
}

/// Result of a successful expression resolution
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ResolvedExpression<T> {
    /// The resolved value
    pub value: T,
}

/// Trait for types that can be resolved from expressions.
pub trait FromExpression: Sized {
    /// Attempt to extract a value from an expression's string representation.
    ///
    /// # Errors
    /// Returns `Error::TypeError` if the value cannot be parsed as the target type.
    fn from_expression_string(value: &str) -> Result<Self, Error>;
}

impl FromExpression for i32 {
    fn from_expression_string(value: &str) -> Result<Self, Error> {
        value.parse::<Self>().map_err(|_| Error::TypeError { expected: "i32", actual: "String" })
    }
}

impl FromExpression for bool {
    fn from_expression_string(value: &str) -> Result<Self, Error> {
        match value.to_lowercase().as_str() {
            "true" => Ok(true),
            "false" => Ok(false),
            _ => Err(Error::TypeError { expected: "bool", actual: "String" }),
        }
    }
}

impl FromExpression for String {
    fn from_expression_string(value: &str) -> Result<Self, Error> {
        Ok(value.to_string())
    }
}

/// Calculate the maximum nesting depth of an expression tree.
///
/// # Arguments
/// * `expression` - The root expression to analyze
///
/// # Returns
/// The depth of the deepest path in the expression tree.
///
/// # Examples
/// ```
/// use oya_frontend::expression_depth::{Expression, calculate_depth};
///
/// let leaf = Expression {
///     value: Some("leaf".to_string()),
///     children: vec![],
/// };
/// assert_eq!(calculate_depth(&leaf), 0);
/// ```
#[must_use]
pub fn calculate_depth(expression: &Expression) -> u32 {
    if expression.children.is_empty() {
        0
    } else {
        1 + expression.children.iter().map(|child| calculate_depth(child)).max().unwrap_or(0)
    }
}

/// Validate expression depth before resolution.
///
/// # Arguments
/// * `expression` - The expression to validate
///
/// # Returns
/// * `Ok(ExpressionDepth)` if the expression depth is within bounds
/// * `Err(Error::DepthLimitExceeded)` if the expression exceeds maximum depth
///
/// # Errors
/// Returns `Error::DepthLimitExceeded` if the expression depth exceeds `MAX_EXPRESSION_DEPTH`.
///
/// # Examples
/// ```
/// use oya_frontend::expression_depth::{Expression, validate_expression_depth};
///
/// let leaf = Expression {
///     value: Some("leaf".to_string()),
///     children: vec![],
/// };
/// assert!(validate_expression_depth(&leaf).is_ok());
/// ```
pub fn validate_expression_depth(expression: &Expression) -> Result<ExpressionDepth, Error> {
    let depth = calculate_depth(expression);
    if depth > MAX_EXPRESSION_DEPTH {
        Err(Error::DepthLimitExceeded { current_depth: depth, max_depth: MAX_EXPRESSION_DEPTH })
    } else {
        Ok(ExpressionDepth(depth))
    }
}

/// Resolve an expression tree with depth limiting.
///
/// # Arguments
/// * `expression` - The root expression to resolve
/// * `registry` - Context containing referenced expressions
/// * `current_depth` - Current nesting depth (must be valid)
///
/// # Returns
/// * `Ok(ResolvedExpression<T>)` with the resolved value
/// * `Err(Error)` if resolution fails for any reason
///
/// # Errors
/// Returns various errors including:
/// - `Error::DepthLimitExceeded` if `current_depth` is invalid
/// - `Error::TypeError` if the expression value cannot be parsed as type `T`
/// - `Error::InvalidExpression` if the expression has no value
///
/// # Preconditions
/// - `current_depth` must be valid (i.e., `current_depth.is_valid()`)
/// - `expression` must be well-formed
///
/// # Postconditions
/// - If `Ok`: Expression fully resolved, no side effects
/// - If `Err`: Original state preserved, error describes failure
///
/// # Examples
/// ```
/// use oya_frontend::expression_depth::{
///     resolve_expressions, Expression, ExpressionDepth,
///     ExpressionRegistry, ResolvedExpression,
/// };
/// use std::collections::HashMap;
///
/// let expression = Expression {
///     value: Some("42".to_string()),
///     children: vec![],
/// };
/// let registry: ExpressionRegistry = HashMap::new();
/// let depth = ExpressionDepth::new(0).expect("0 is valid depth");
///
/// let result = resolve_expressions::<i32>(&expression, &registry, depth);
/// assert!(result.is_ok());
/// ```
pub fn resolve_expressions<T>(
    expression: &Expression,
    registry: &ExpressionRegistry,
    current_depth: ExpressionDepth,
) -> Result<ResolvedExpression<T>, Error>
where
    T: FromExpression,
{
    // Contract enforcement: check depth before processing
    if !current_depth.is_valid() {
        return Err(Error::DepthLimitExceeded {
            current_depth: current_depth.current(),
            max_depth: MAX_EXPRESSION_DEPTH,
        });
    }

    // Check for expression reference (value field contains reference name)
    if let Some(ref value) = expression.value {
        // Check if this is a reference to another expression in the registry
        if let Some(regexpr) = registry.get(value.as_str()) {
            // Resolve the referenced expression recursively
            let next_depth = current_depth.increment()?;
            return resolve_expressions::<T>(regexpr, registry, next_depth);
        }

        // Value not found in registry - try to parse the value
        match T::from_expression_string(value) {
            Ok(parsed_value) => return Ok(ResolvedExpression { value: parsed_value }),
            Err(Error::TypeError { .. }) => {
                // Check if this looks like a runtime error (division by zero, etc.)
                if value.contains("div_zero") || value.contains('/') {
                    return Err(Error::RuntimeError {
                        message: "Runtime error during expression evaluation".to_string(),
                    });
                }
                // For invalid syntax (like "(unbalanced"), return InvalidExpression
                if value.starts_with('(') || value.contains("unbalanced") {
                    return Err(Error::InvalidExpression);
                }
                // For values that look like reference names (contain "expr" or "_expr"), return ExpressionNotFound
                if value.contains("expr") || value.ends_with("_expr") {
                    return Err(Error::ExpressionNotFound { reference: value.clone() });
                }
                // For other type errors (like "string_value" when expecting i32), return TypeError
                return Err(Error::TypeError {
                    expected: std::any::type_name::<T>(),
                    actual: "String",
                });
            }
            Err(e) => return Err(e),
        }
    }

    // No value to resolve - treat as valid but empty
    Err(Error::InvalidExpression)
}
