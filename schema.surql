-- Rate Limiting Schema for SurrealDB
-- Provides token bucket rate limiting and concurrency control
-- with atomic counter operations

-- ============================================================================
-- Token Bucket Table (Rate Limiting)
-- ============================================================================
-- Implements token bucket algorithm for rate limiting:
-- - Tokens refill at a constant rate
-- - Operations consume tokens
-- - Operations fail when bucket is empty
-- - Prevents burst traffic while allowing sustained load

DEFINE TABLE token_bucket SCHEMAFULL;

-- Primary key: resource identifier (e.g., "user:123", "api:endpoint:/users")
DEFINE FIELD resource_id ON TABLE token_bucket TYPE string
    ASSERT $value != NONE AND string::len($value) > 0;

-- Maximum tokens the bucket can hold (capacity)
DEFINE FIELD capacity ON TABLE token_bucket TYPE int
    ASSERT $value > 0;

-- Current number of tokens available
DEFINE FIELD current_tokens ON TABLE token_bucket TYPE int
    ASSERT $value >= 0 AND $value <= $parent.capacity;

-- Tokens added per second (refill rate)
DEFINE FIELD refill_rate ON TABLE token_bucket TYPE float
    ASSERT $value > 0.0;

-- Last time tokens were refilled (unix timestamp in seconds)
DEFINE FIELD last_refill_at ON TABLE token_bucket TYPE datetime;

-- Timestamp when bucket was created
DEFINE FIELD created_at ON TABLE token_bucket TYPE datetime VALUE time::now();

-- Timestamp of last update
DEFINE FIELD updated_at ON TABLE token_bucket TYPE datetime VALUE time::now();

-- Index for efficient lookups by resource
DEFINE INDEX idx_token_bucket_resource ON TABLE token_bucket COLUMNS resource_id UNIQUE;

-- ============================================================================
-- Concurrency Limit Table (Resource Management)
-- ============================================================================
-- Tracks concurrent operations per resource:
-- - Enforces maximum concurrent operations
-- - Atomic increment/decrement for safe concurrency
-- - Prevents resource exhaustion

DEFINE TABLE concurrency_limit SCHEMAFULL;

-- Primary key: resource identifier (e.g., "worker:pool", "db:connection")
DEFINE FIELD resource_id ON TABLE concurrency_limit TYPE string
    ASSERT $value != NONE AND string::len($value) > 0;

-- Maximum concurrent operations allowed
DEFINE FIELD max_concurrent ON TABLE concurrency_limit TYPE int
    ASSERT $value > 0;

-- Current number of active operations
DEFINE FIELD current_count ON TABLE concurrency_limit TYPE int
    ASSERT $value >= 0 AND $value <= $parent.max_concurrent;

-- Timestamp when limit was created
DEFINE FIELD created_at ON TABLE concurrency_limit TYPE datetime VALUE time::now();

-- Timestamp of last update
DEFINE FIELD updated_at ON TABLE concurrency_limit TYPE datetime VALUE time::now();

-- Index for efficient lookups by resource
DEFINE INDEX idx_concurrency_limit_resource ON TABLE concurrency_limit COLUMNS resource_id UNIQUE;

-- ============================================================================
-- Atomic Operations (Implemented via SurrealDB queries)
-- ============================================================================

-- Token Bucket Operations:
-- 1. Acquire tokens (with refill):
--    UPDATE token_bucket:<id> SET
--      current_tokens = math::max(0, current_tokens - <amount>),
--      last_refill_at = time::now()
--    WHERE current_tokens >= <amount>
--    RETURN AFTER;
--
-- 2. Refill tokens:
--    UPDATE token_bucket:<id> SET
--      current_tokens = math::min(capacity, current_tokens + <refill_amount>),
--      last_refill_at = time::now()
--    RETURN AFTER;

-- Concurrency Limit Operations:
-- 1. Acquire slot (increment):
--    UPDATE concurrency_limit:<id> SET
--      current_count = current_count + 1,
--      updated_at = time::now()
--    WHERE current_count < max_concurrent
--    RETURN AFTER;
--
-- 2. Release slot (decrement):
--    UPDATE concurrency_limit:<id> SET
--      current_count = math::max(0, current_count - 1),
--      updated_at = time::now()
--    RETURN AFTER;

-- ============================================================================
-- Example Usage
-- ============================================================================

-- Create a token bucket for API rate limiting (100 requests/second)
-- CREATE token_bucket:api_endpoint SET
--   resource_id = "api:/v1/users",
--   capacity = 100,
--   current_tokens = 100,
--   refill_rate = 100.0,
--   last_refill_at = time::now();

-- Create a concurrency limit for database connections (max 50)
-- CREATE concurrency_limit:db_pool SET
--   resource_id = "db:postgres:pool",
--   max_concurrent = 50,
--   current_count = 0;
