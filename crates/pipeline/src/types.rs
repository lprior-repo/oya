//! Validated wrapper types for making illegal states unrepresentable.
//!
//! These types enforce invariants at compile-time or construction-time,
//! eliminating entire classes of runtime errors.

use std::fmt;
use std::ops::Deref;

use serde::{Deserialize, Serialize};

use crate::error::{Error, Result};

// =============================================================================
// NonEmpty<T> - A collection that is guaranteed to have at least one element
// =============================================================================

/// A non-empty collection wrapper.
///
/// Guarantees at least one element exists, eliminating the need to check
/// for emptiness before accessing the first element.
///
/// # Examples
///
/// ```ignore
/// let items = NonEmpty::new(vec!["a", "b"])?;
/// let first = items.first(); // Always safe, no Option needed
/// ```
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(bound(
    serialize = "T: Serialize + Clone",
    deserialize = "T: Deserialize<'de> + Clone"
))]
#[serde(try_from = "Vec<T>", into = "Vec<T>")]
pub struct NonEmpty<T: Clone>(Vec<T>);

impl<T: Clone> NonEmpty<T> {
    /// Create a new NonEmpty from a vector.
    ///
    /// Returns Err if the vector is empty.
    pub fn new(items: Vec<T>) -> Result<Self> {
        if items.is_empty() {
            Err(Error::InvalidRecord {
                reason: "collection cannot be empty".into(),
            })
        } else {
            Ok(Self(items))
        }
    }

    /// Create a NonEmpty from a single item.
    #[must_use]
    pub fn singleton(item: T) -> Self {
        Self(vec![item])
    }

    /// Create a NonEmpty from a head and tail.
    #[must_use]
    pub fn cons(head: T, tail: Vec<T>) -> Self {
        let mut items = vec![head];
        items.extend(tail);
        Self(items)
    }

    /// Get the first element (always exists).
    #[must_use]
    pub fn first(&self) -> &T {
        // SAFETY: We guarantee at least one element exists
        &self.0[0]
    }

    /// Get the last element (always exists).
    #[must_use]
    pub fn last(&self) -> &T {
        // SAFETY: We guarantee at least one element exists
        &self.0[self.0.len() - 1]
    }

    /// Get the head (first) and tail (rest) of the collection.
    #[must_use]
    pub fn uncons(&self) -> (&T, &[T]) {
        // Safe direct indexing: NonEmpty invariant guarantees len >= 1
        // First element (index 0) always exists, rest slice [1..] is always valid
        (&self.0[0], &self.0[1..])
    }

    /// Get the length of the collection (always >= 1).
    #[must_use]
    pub fn len(&self) -> usize {
        self.0.len()
    }

    /// NonEmpty is never empty (this always returns false).
    #[must_use]
    pub const fn is_empty(&self) -> bool {
        false
    }

    /// Map a function over all elements.
    #[must_use]
    pub fn map<U: Clone, F: FnMut(T) -> U>(self, f: F) -> NonEmpty<U> {
        NonEmpty(self.0.into_iter().map(f).collect())
    }

    /// Push an element to the end.
    pub fn push(&mut self, item: T) {
        self.0.push(item);
    }

    /// Convert into the inner Vec.
    #[must_use]
    pub fn into_vec(self) -> Vec<T> {
        self.0
    }

    /// Get a reference to the inner slice.
    #[must_use]
    pub fn as_slice(&self) -> &[T] {
        &self.0
    }

    /// Iterate over references.
    pub fn iter(&self) -> impl Iterator<Item = &T> {
        self.0.iter()
    }
}

impl<T: Clone> TryFrom<Vec<T>> for NonEmpty<T> {
    type Error = Error;

    fn try_from(items: Vec<T>) -> Result<Self> {
        Self::new(items)
    }
}

impl<T: Clone> From<NonEmpty<T>> for Vec<T> {
    fn from(ne: NonEmpty<T>) -> Self {
        ne.0
    }
}

impl<T: Clone> Deref for NonEmpty<T> {
    type Target = [T];

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl<T: Clone> IntoIterator for NonEmpty<T> {
    type Item = T;
    type IntoIter = std::vec::IntoIter<T>;

    fn into_iter(self) -> Self::IntoIter {
        self.0.into_iter()
    }
}

impl<'a, T: Clone> IntoIterator for &'a NonEmpty<T> {
    type Item = &'a T;
    type IntoIter = std::slice::Iter<'a, T>;

    fn into_iter(self) -> Self::IntoIter {
        self.0.iter()
    }
}

// =============================================================================
// NonEmptyString - A string that is guaranteed to be non-empty
// =============================================================================

/// A non-empty string wrapper.
///
/// Guarantees the string is not empty or whitespace-only.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(try_from = "String", into = "String")]
pub struct NonEmptyString(String);

impl NonEmptyString {
    /// Create a new NonEmptyString.
    ///
    /// Returns Err if the string is empty or whitespace-only.
    pub fn new(s: impl Into<String>) -> Result<Self> {
        let s = s.into();
        if s.trim().is_empty() {
            Err(Error::InvalidRecord {
                reason: "string cannot be empty or whitespace-only".into(),
            })
        } else {
            Ok(Self(s))
        }
    }

    /// Get the string as a str slice.
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }

    /// Get the length of the string.
    #[must_use]
    pub fn len(&self) -> usize {
        self.0.len()
    }

    /// NonEmptyString is never empty.
    #[must_use]
    pub const fn is_empty(&self) -> bool {
        false
    }

    /// Convert into the inner String.
    #[must_use]
    pub fn into_string(self) -> String {
        self.0
    }
}

impl TryFrom<String> for NonEmptyString {
    type Error = Error;

    fn try_from(s: String) -> Result<Self> {
        Self::new(s)
    }
}

impl TryFrom<&str> for NonEmptyString {
    type Error = Error;

    fn try_from(s: &str) -> Result<Self> {
        Self::new(s)
    }
}

impl From<NonEmptyString> for String {
    fn from(s: NonEmptyString) -> Self {
        s.0
    }
}

impl Deref for NonEmptyString {
    type Target = str;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl fmt::Display for NonEmptyString {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl AsRef<str> for NonEmptyString {
    fn as_ref(&self) -> &str {
        &self.0
    }
}

// =============================================================================
// Bounded<T, MIN, MAX> - A number within a fixed range
// =============================================================================

/// A bounded numeric value.
///
/// Guarantees the value is within [MIN, MAX] inclusive.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Bounded<const MIN: i64, const MAX: i64>(i64);

impl<const MIN: i64, const MAX: i64> Bounded<MIN, MAX> {
    /// Create a new Bounded value.
    ///
    /// Returns Err if value is outside [MIN, MAX].
    pub fn new(value: i64) -> Result<Self> {
        if value < MIN || value > MAX {
            Err(Error::InvalidRecord {
                reason: format!("value {value} must be between {MIN} and {MAX}"),
            })
        } else {
            Ok(Self(value))
        }
    }

    /// Get the inner value.
    #[must_use]
    pub const fn get(&self) -> i64 {
        self.0
    }

    /// Get the minimum allowed value.
    #[must_use]
    pub const fn min() -> i64 {
        MIN
    }

    /// Get the maximum allowed value.
    #[must_use]
    pub const fn max() -> i64 {
        MAX
    }

    /// Saturating add that clamps to bounds.
    #[must_use]
    pub fn saturating_add(&self, n: i64) -> Self {
        Self((self.0 + n).clamp(MIN, MAX))
    }

    /// Saturating sub that clamps to bounds.
    #[must_use]
    pub fn saturating_sub(&self, n: i64) -> Self {
        Self((self.0 - n).clamp(MIN, MAX))
    }
}

/// Type alias for percentage (0-100).
pub type Percentage = Bounded<0, 100>;

/// Type alias for retry count (0-10).
pub type RetryCount = Bounded<0, 10>;

// =============================================================================
// PositiveInt - A positive integer (> 0)
// =============================================================================

/// A positive integer wrapper (value > 0).
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(try_from = "i64", into = "i64")]
pub struct PositiveInt(i64);

impl PositiveInt {
    /// Create a new PositiveInt.
    ///
    /// Returns Err if value <= 0.
    pub fn new(value: i64) -> Result<Self> {
        if value <= 0 {
            Err(Error::InvalidRecord {
                reason: format!("value {value} must be positive (> 0)"),
            })
        } else {
            Ok(Self(value))
        }
    }

    /// Get the inner value.
    #[must_use]
    pub const fn get(&self) -> i64 {
        self.0
    }
}

impl TryFrom<i64> for PositiveInt {
    type Error = Error;

    fn try_from(value: i64) -> Result<Self> {
        Self::new(value)
    }
}

impl From<PositiveInt> for i64 {
    fn from(p: PositiveInt) -> Self {
        p.0
    }
}

// =============================================================================
// NonNegativeInt - A non-negative integer (>= 0)
// =============================================================================

/// A non-negative integer wrapper (value >= 0).
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(try_from = "i64", into = "i64")]
pub struct NonNegativeInt(i64);

impl NonNegativeInt {
    /// Create a new NonNegativeInt.
    ///
    /// Returns Err if value < 0.
    pub fn new(value: i64) -> Result<Self> {
        if value < 0 {
            Err(Error::InvalidRecord {
                reason: format!("value {value} must be non-negative (>= 0)"),
            })
        } else {
            Ok(Self(value))
        }
    }

    /// Get the inner value.
    #[must_use]
    pub const fn get(&self) -> i64 {
        self.0
    }

    /// Zero value.
    #[must_use]
    pub const fn zero() -> Self {
        Self(0)
    }

    /// Increment the value.
    #[must_use]
    pub const fn increment(&self) -> Self {
        Self(self.0 + 1)
    }

    /// Decrement the value (saturating at 0).
    #[must_use]
    pub fn decrement(&self) -> Self {
        Self((self.0 - 1).max(0))
    }
}

impl TryFrom<i64> for NonNegativeInt {
    type Error = Error;

    fn try_from(value: i64) -> Result<Self> {
        Self::new(value)
    }
}

impl From<NonNegativeInt> for i64 {
    fn from(n: NonNegativeInt) -> Self {
        n.0
    }
}

impl Default for NonNegativeInt {
    fn default() -> Self {
        Self::zero()
    }
}

// =============================================================================
// Validated<T> - A value that has passed validation
// =============================================================================

/// Marker trait for validated values.
pub trait Validated: Sized {
    /// The raw type before validation.
    type Raw;

    /// Validate and create from raw value.
    fn validate(raw: Self::Raw) -> Result<Self>;
}

impl<T: Clone> Validated for NonEmpty<T> {
    type Raw = Vec<T>;

    fn validate(raw: Self::Raw) -> Result<Self> {
        Self::new(raw)
    }
}

impl Validated for NonEmptyString {
    type Raw = String;

    fn validate(raw: Self::Raw) -> Result<Self> {
        Self::new(raw)
    }
}

// =============================================================================
// Either<L, R> - A value that can be one of two types
// =============================================================================

/// A value that is either Left or Right.
///
/// Useful for representing branching logic in a functional way.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum Either<L, R> {
    /// The left variant.
    Left(L),
    /// The right variant.
    Right(R),
}

impl<L, R> Either<L, R> {
    /// Check if this is a Left value.
    #[must_use]
    pub const fn is_left(&self) -> bool {
        matches!(self, Self::Left(_))
    }

    /// Check if this is a Right value.
    #[must_use]
    pub const fn is_right(&self) -> bool {
        matches!(self, Self::Right(_))
    }

    /// Get the Left value if present.
    #[must_use]
    pub fn left(self) -> Option<L> {
        match self {
            Self::Left(l) => Some(l),
            Self::Right(_) => None,
        }
    }

    /// Get the Right value if present.
    #[must_use]
    pub fn right(self) -> Option<R> {
        match self {
            Self::Left(_) => None,
            Self::Right(r) => Some(r),
        }
    }

    /// Map over the Left value.
    #[must_use]
    pub fn map_left<U, F: FnOnce(L) -> U>(self, f: F) -> Either<U, R> {
        match self {
            Self::Left(l) => Either::Left(f(l)),
            Self::Right(r) => Either::Right(r),
        }
    }

    /// Map over the Right value.
    #[must_use]
    pub fn map_right<U, F: FnOnce(R) -> U>(self, f: F) -> Either<L, U> {
        match self {
            Self::Left(l) => Either::Left(l),
            Self::Right(r) => Either::Right(f(r)),
        }
    }

    /// Fold both variants into a single value.
    #[must_use]
    pub fn fold<T, FL: FnOnce(L) -> T, FR: FnOnce(R) -> T>(self, left_fn: FL, right_fn: FR) -> T {
        match self {
            Self::Left(l) => left_fn(l),
            Self::Right(r) => right_fn(r),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_non_empty_construction() {
        assert!(NonEmpty::new(vec![1, 2, 3]).is_ok());
        assert!(NonEmpty::<i32>::new(vec![]).is_err());
    }

    #[test]
    fn test_non_empty_first_last() {
        let ne = NonEmpty::new(vec![1, 2, 3]).ok();
        assert_eq!(ne.as_ref().map(|n| *n.first()), Some(1));
        assert_eq!(ne.map(|n| *n.last()), Some(3));
    }

    #[test]
    fn test_non_empty_singleton() {
        let ne = NonEmpty::singleton(42);
        assert_eq!(*ne.first(), 42);
        assert_eq!(*ne.last(), 42);
        assert_eq!(ne.len(), 1);
    }

    #[test]
    fn test_non_empty_string() {
        assert!(NonEmptyString::new("hello").is_ok());
        assert!(NonEmptyString::new("").is_err());
        assert!(NonEmptyString::new("   ").is_err());
    }

    #[test]
    fn test_bounded() {
        let pct = Percentage::new(50);
        assert!(pct.is_ok());
        if let Ok(p) = pct {
            assert_eq!(p.get(), 50);
        }

        assert!(Percentage::new(-1).is_err());
        assert!(Percentage::new(101).is_err());
    }

    #[test]
    fn test_bounded_saturating() {
        let pct = Percentage::new(90);
        assert!(pct.is_ok());
        if let Ok(p) = pct {
            assert_eq!(p.saturating_add(20).get(), 100);
            assert_eq!(p.saturating_sub(100).get(), 0);
        }
    }

    #[test]
    fn test_positive_int() {
        assert!(PositiveInt::new(1).is_ok());
        assert!(PositiveInt::new(0).is_err());
        assert!(PositiveInt::new(-1).is_err());
    }

    #[test]
    fn test_non_negative_int() {
        assert!(NonNegativeInt::new(0).is_ok());
        assert!(NonNegativeInt::new(1).is_ok());
        assert!(NonNegativeInt::new(-1).is_err());

        let n = NonNegativeInt::zero();
        assert_eq!(n.increment().get(), 1);
        assert_eq!(n.decrement().get(), 0);
    }

    #[test]
    fn test_either() {
        let left: Either<i32, &str> = Either::Left(42);
        let right: Either<i32, &str> = Either::Right("hello");

        assert!(left.is_left());
        assert!(right.is_right());

        let mapped_left: Either<i32, &str> = Either::Left(21).map_left(|x| x * 2);
        assert_eq!(mapped_left.left(), Some(42));

        let folded = Either::<i32, i32>::Left(10).fold(|l| l + 1, |r| r - 1);
        assert_eq!(folded, 11);
    }
}
