//! Rate limiting and concurrency control types for SurrealDB schema.
//!
//! This module provides type-safe Rust mappings for the `token_bucket` and
//! `concurrency_limit` tables, with atomic counter operations implemented
//! using SurrealDB's built-in functions.

#![deny(clippy::unwrap_used)]
#![deny(clippy::expect_used)]
#![deny(clippy::panic)]
#![warn(clippy::pedantic)]
#![warn(clippy::nursery)]

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use thiserror::Error;

// ============================================================================
// Error Types
// ============================================================================

/// Errors that can occur during rate limiting operations.
#[derive(Debug, Error, Clone, PartialEq)]
pub enum RateLimitError {
    /// Invalid capacity (must be positive).
    #[error("capacity must be positive, got {0}")]
    InvalidCapacity(i64),

    /// Invalid refill rate (must be positive).
    #[error("refill rate must be positive, got {0}")]
    InvalidRefillRate(f64),

    /// Invalid token count (cannot be negative or exceed capacity).
    #[error("token count {count} must be between 0 and {capacity}")]
    InvalidTokenCount { count: i64, capacity: i64 },

    /// Resource identifier is empty.
    #[error("resource identifier cannot be empty")]
    EmptyResourceId,

    /// Not enough tokens available for operation.
    #[error("insufficient tokens: requested {requested}, available {available}")]
    InsufficientTokens { requested: i64, available: i64 },

    /// Maximum concurrent operations reached.
    #[error("concurrency limit reached: {current}/{max}")]
    ConcurrencyLimitReached { current: i64, max: i64 },

    /// Invalid concurrent count.
    #[error("concurrent count {count} must be between 0 and {max}")]
    InvalidConcurrentCount { count: i64, max: i64 },

    /// Invalid maximum concurrent limit (must be positive).
    #[error("max concurrent must be positive, got {0}")]
    InvalidMaxConcurrent(i64),
}

pub type Result<T> = std::result::Result<T, RateLimitError>;

// ============================================================================
// Token Bucket Types
// ============================================================================

/// Configuration for a token bucket.
#[derive(Debug, Clone, PartialEq)]
pub struct TokenBucketConfig {
    capacity: i64,
    refill_rate: f64,
}

impl TokenBucketConfig {
    /// Create a new token bucket configuration.
    pub fn new(capacity: i64, refill_rate: f64) -> Result<Self> {
        if capacity <= 0 {
            return Err(RateLimitError::InvalidCapacity(capacity));
        }
        if refill_rate <= 0.0 {
            return Err(RateLimitError::InvalidRefillRate(refill_rate));
        }

        Ok(Self {
            capacity,
            refill_rate,
        })
    }

    /// Get the capacity.
    #[must_use]
    pub const fn capacity(&self) -> i64 {
        self.capacity
    }

    /// Get the refill rate.
    #[must_use]
    pub const fn refill_rate(&self) -> f64 {
        self.refill_rate
    }
}

/// Resource identifier newtype for type safety.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct ResourceId(String);

impl ResourceId {
    /// Create a new resource identifier.
    pub fn new(id: String) -> Result<Self> {
        if id.is_empty() {
            Err(RateLimitError::EmptyResourceId)
        } else {
            Ok(Self(id))
        }
    }

    /// Get the inner string.
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }

    /// Consume and return the inner string.
    #[must_use]
    pub fn into_inner(self) -> String {
        self.0
    }
}

/// Token bucket for rate limiting.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct TokenBucket {
    pub resource_id: String,
    pub capacity: i64,
    pub current_tokens: i64,
    pub refill_rate: f64,
    pub last_refill_at: DateTime<Utc>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl TokenBucket {
    /// Create a new token bucket with full capacity.
    pub fn create(resource_id: String, config: TokenBucketConfig) -> Result<Self> {
        let resource = ResourceId::new(resource_id)?;
        let now = Utc::now();

        Ok(Self {
            resource_id: resource.into_inner(),
            capacity: config.capacity(),
            current_tokens: config.capacity(),
            refill_rate: config.refill_rate(),
            last_refill_at: now,
            created_at: now,
            updated_at: now,
        })
    }

    /// Calculate tokens to add based on elapsed time.
    #[must_use]
    pub fn calculate_refill(&self, now: DateTime<Utc>) -> i64 {
        let elapsed_secs = (now - self.last_refill_at).num_milliseconds().max(0) as f64 / 1000.0;
        let tokens_to_add = (elapsed_secs * self.refill_rate).floor() as i64;
        tokens_to_add.max(0)
    }

    /// Check if tokens can be acquired without mutating state.
    #[must_use]
    pub fn can_acquire(&self, amount: i64, now: DateTime<Utc>) -> bool {
        let refill_amount = self.calculate_refill(now);
        let available = (self.current_tokens + refill_amount).min(self.capacity);
        available >= amount && amount > 0
    }
}

// ============================================================================
// Concurrency Limit Types
// ============================================================================

/// Configuration for a concurrency limit.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ConcurrencyLimitConfig {
    max_concurrent: i64,
}

impl ConcurrencyLimitConfig {
    /// Create a new concurrency limit configuration.
    pub fn new(max_concurrent: i64) -> Result<Self> {
        if max_concurrent <= 0 {
            return Err(RateLimitError::InvalidMaxConcurrent(max_concurrent));
        }

        Ok(Self { max_concurrent })
    }

    /// Get the maximum concurrent operations.
    #[must_use]
    pub const fn max_concurrent(&self) -> i64 {
        self.max_concurrent
    }
}

/// Concurrency limit for resource management.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ConcurrencyLimit {
    pub resource_id: String,
    pub max_concurrent: i64,
    pub current_count: i64,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl ConcurrencyLimit {
    /// Create a new concurrency limit with zero active operations.
    pub fn create(resource_id: String, config: ConcurrencyLimitConfig) -> Result<Self> {
        let resource = ResourceId::new(resource_id)?;
        let now = Utc::now();

        Ok(Self {
            resource_id: resource.into_inner(),
            max_concurrent: config.max_concurrent(),
            current_count: 0,
            created_at: now,
            updated_at: now,
        })
    }

    /// Check if a slot can be acquired without mutating state.
    #[must_use]
    pub const fn can_acquire(&self) -> bool {
        self.current_count < self.max_concurrent
    }

    /// Check if current count can be decremented safely.
    #[must_use]
    pub const fn can_release(&self) -> bool {
        self.current_count > 0
    }
}

// ============================================================================
// Query Builders (Pure Functions)
// ============================================================================

/// Build a SurrealDB query to acquire tokens atomically.
#[must_use]
pub fn build_acquire_tokens_query(
    table_id: &str,
    amount: i64,
    refill_amount: i64,
) -> (String, Vec<(&'static str, String)>) {
    let query = format!(
        "UPDATE token_bucket:{} SET \
         current_tokens = math::max(0, current_tokens + $refill - $amount), \
         last_refill_at = time::now(), \
         updated_at = time::now() \
         WHERE (current_tokens + $refill) >= $amount \
         RETURN AFTER",
        table_id
    );

    let params = vec![
        ("refill", refill_amount.to_string()),
        ("amount", amount.to_string()),
    ];

    (query, params)
}

/// Build a SurrealDB query to acquire a concurrency slot atomically.
#[must_use]
pub fn build_acquire_slot_query(table_id: &str) -> String {
    format!(
        "UPDATE concurrency_limit:{} SET \
         current_count = current_count + 1, \
         updated_at = time::now() \
         WHERE current_count < max_concurrent \
         RETURN AFTER",
        table_id
    )
}

/// Build a SurrealDB query to release a concurrency slot atomically.
#[must_use]
pub fn build_release_slot_query(table_id: &str) -> String {
    format!(
        "UPDATE concurrency_limit:{} SET \
         current_count = math::max(0, current_count - 1), \
         updated_at = time::now() \
         RETURN AFTER",
        table_id
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_token_bucket_config_validation() {
        let valid = TokenBucketConfig::new(100, 10.0);
        assert!(valid.is_ok());

        let invalid_cap = TokenBucketConfig::new(0, 10.0);
        assert!(matches!(
            invalid_cap,
            Err(RateLimitError::InvalidCapacity(_))
        ));
    }

    #[test]
    fn test_token_bucket_creation() {
        let config = TokenBucketConfig::new(100, 10.0).ok();
        assert!(config.is_some());

        if let Some(cfg) = config {
            let bucket = TokenBucket::create("test-resource".to_string(), cfg);
            assert!(bucket.is_ok());
        }
    }
}
