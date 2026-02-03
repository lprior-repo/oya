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
-- Event Sourcing Tables
-- ============================================================================

DEFINE TABLE state_transition SCHEMAFULL;

DEFINE FIELD aggregate_id ON TABLE state_transition TYPE string
    ASSERT $value != NONE AND string::len($value) > 0;

DEFINE FIELD sequence_number ON TABLE state_transition TYPE int
    ASSERT $value >= 0;

DEFINE FIELD event_type ON TABLE state_transition TYPE string
    ASSERT $value != NONE AND string::len($value) > 0;

DEFINE FIELD event_data ON TABLE state_transition TYPE option<object>;

DEFINE FIELD occurred_at ON TABLE state_transition TYPE datetime;

DEFINE FIELD causation_id ON TABLE state_transition TYPE option<string>;

DEFINE FIELD correlation_id ON TABLE state_transition TYPE option<string>;

DEFINE FIELD created_at ON TABLE state_transition TYPE datetime VALUE time::now();

DEFINE INDEX idx_state_transition_aggregate_seq ON TABLE state_transition COLUMNS aggregate_id, sequence_number;
DEFINE INDEX idx_state_transition_correlation ON TABLE state_transition COLUMNS correlation_id;

DEFINE TABLE idempotency_key SCHEMAFULL;

DEFINE FIELD key_hash ON TABLE idempotency_key TYPE string
    ASSERT $value != NONE AND string::len($value) > 0;

DEFINE FIELD response_data ON TABLE idempotency_key TYPE option<object>;

DEFINE FIELD processed_at ON TABLE idempotency_key TYPE datetime VALUE time::now();

DEFINE FIELD expires_at ON TABLE idempotency_key TYPE option<datetime>;

DEFINE INDEX idx_idempotency_key_hash ON TABLE idempotency_key COLUMNS key_hash UNIQUE;
DEFINE INDEX idx_idempotency_key_expires ON TABLE idempotency_key COLUMNS expires_at;

DEFINE TABLE checkpoint SCHEMAFULL;

DEFINE FIELD aggregate_id ON TABLE checkpoint TYPE string
    ASSERT $value != NONE AND string::len($value) > 0;

DEFINE FIELD sequence_number ON TABLE checkpoint TYPE int
    ASSERT $value >= 0;

DEFINE FIELD state_snapshot ON TABLE checkpoint TYPE object;

DEFINE FIELD snapshot_version ON TABLE checkpoint TYPE int
    ASSERT $value >= 0;

DEFINE FIELD created_at ON TABLE checkpoint TYPE datetime VALUE time::now();

DEFINE INDEX idx_checkpoint_aggregate_seq ON TABLE checkpoint COLUMNS aggregate_id, sequence_number;

-- ============================================================================
-- Orchestration Tables
-- ============================================================================

DEFINE TABLE bead SCHEMAFULL;

DEFINE FIELD bead_id ON TABLE bead TYPE string
    ASSERT $value != NONE AND string::len($value) > 0;

DEFINE FIELD title ON TABLE bead TYPE string
    ASSERT $value != NONE AND string::len($value) > 0;

DEFINE FIELD description ON TABLE bead TYPE option<string>;

DEFINE FIELD status ON TABLE bead TYPE string
    ASSERT $value IN ['open', 'in_progress', 'blocked', 'deferred', 'closed'];

DEFINE FIELD priority ON TABLE bead TYPE string
    ASSERT $value IN ['P0', 'P1', 'P2', 'P3'];

DEFINE FIELD owner ON TABLE bead TYPE option<string>;

DEFINE FIELD created_at ON TABLE bead TYPE datetime VALUE time::now();

DEFINE FIELD updated_at ON TABLE bead TYPE datetime VALUE time::now();

DEFINE INDEX idx_bead_id ON TABLE bead COLUMNS bead_id UNIQUE;
DEFINE INDEX idx_bead_status ON TABLE bead COLUMNS status;
DEFINE INDEX idx_bead_priority ON TABLE bead COLUMNS priority;

DEFINE TABLE workflow_run SCHEMAFULL;

DEFINE FIELD workflow_id ON TABLE workflow_run TYPE string
    ASSERT $value != NONE AND string::len($value) > 0;

DEFINE FIELD workflow_name ON TABLE workflow_run TYPE string
    ASSERT $value != NONE AND string::len($value) > 0;

DEFINE FIELD status ON TABLE workflow_run TYPE string
    ASSERT $value IN ['pending', 'running', 'paused', 'completed', 'failed', 'cancelled'];

DEFINE FIELD input_data ON TABLE workflow_run TYPE option<object>;

DEFINE FIELD output_data ON TABLE workflow_run TYPE option<object>;

DEFINE FIELD started_at ON TABLE workflow_run TYPE option<datetime>;

DEFINE FIELD completed_at ON TABLE workflow_run TYPE option<datetime>;

DEFINE FIELD error_message ON TABLE workflow_run TYPE option<string>;

DEFINE FIELD created_at ON TABLE workflow_run TYPE datetime VALUE time::now();

DEFINE INDEX idx_workflow_run_id ON TABLE workflow_run COLUMNS workflow_id UNIQUE;
DEFINE INDEX idx_workflow_run_status ON TABLE workflow_run COLUMNS status;

DEFINE TABLE process SCHEMAFULL;

DEFINE FIELD process_id ON TABLE process TYPE string
    ASSERT $value != NONE AND string::len($value) > 0;

DEFINE FIELD workflow_id ON TABLE process TYPE string
    ASSERT $value != NONE AND string::len($value) > 0;

DEFINE FIELD process_name ON TABLE process TYPE string
    ASSERT $value != NONE AND string::len($value) > 0;

DEFINE FIELD status ON TABLE process TYPE string
    ASSERT $value IN ['pending', 'running', 'completed', 'failed'];

DEFINE FIELD pid ON TABLE process TYPE option<int>;

DEFINE FIELD host ON TABLE process TYPE option<string>;

DEFINE FIELD started_at ON TABLE process TYPE option<datetime>;

DEFINE FIELD completed_at ON TABLE process TYPE option<datetime>;

DEFINE FIELD exit_code ON TABLE process TYPE option<int>;

DEFINE FIELD created_at ON TABLE process TYPE datetime VALUE time::now();

DEFINE INDEX idx_process_id ON TABLE process COLUMNS process_id UNIQUE;
DEFINE INDEX idx_process_workflow ON TABLE process COLUMNS workflow_id;
DEFINE INDEX idx_process_status ON TABLE process COLUMNS status;

DEFINE TABLE workspace SCHEMAFULL;

DEFINE FIELD workspace_id ON TABLE workspace TYPE string
    ASSERT $value != NONE AND string::len($value) > 0;

DEFINE FIELD workspace_name ON TABLE workspace TYPE string
    ASSERT $value != NONE AND string::len($value) > 0;

DEFINE FIELD workspace_path ON TABLE workspace TYPE string
    ASSERT $value != NONE AND string::len($value) > 0;

DEFINE FIELD status ON TABLE workspace TYPE string
    ASSERT $value IN ['creating', 'active', 'paused', 'completed', 'failed'];

DEFINE FIELD jj_branch ON TABLE workspace TYPE option<string>;

DEFINE FIELD zellij_session ON TABLE workspace TYPE option<string>;

DEFINE FIELD bead_id ON TABLE workspace TYPE option<string>;

DEFINE FIELD created_at ON TABLE workspace TYPE datetime VALUE time::now();

DEFINE FIELD updated_at ON TABLE workspace TYPE datetime VALUE time::now();

DEFINE INDEX idx_workspace_id ON TABLE workspace COLUMNS workspace_id UNIQUE;
DEFINE INDEX idx_workspace_status ON TABLE workspace COLUMNS status;
DEFINE INDEX idx_workspace_bead ON TABLE workspace COLUMNS bead_id;

DEFINE TABLE schedule SCHEMAFULL;

DEFINE FIELD schedule_id ON TABLE schedule TYPE string
    ASSERT $value != NONE AND string::len($value) > 0;

DEFINE FIELD target_id ON TABLE schedule TYPE string
    ASSERT $value != NONE AND string::len($value) > 0;

DEFINE FIELD target_type ON TABLE schedule TYPE string
    ASSERT $value IN ['workflow', 'process', 'bead', 'workspace'];

DEFINE FIELD cron_expression ON TABLE schedule TYPE string
    ASSERT $value != NONE AND string::len($value) > 0;

DEFINE FIELD next_run_at ON TABLE schedule TYPE datetime;

DEFINE FIELD last_run_at ON TABLE schedule TYPE option<datetime>;

DEFINE FIELD enabled ON TABLE schedule TYPE bool DEFAULT true;

DEFINE FIELD created_at ON TABLE schedule TYPE datetime VALUE time::now();

DEFINE INDEX idx_schedule_id ON TABLE schedule COLUMNS schedule_id UNIQUE;
DEFINE INDEX idx_schedule_target ON TABLE schedule COLUMNS target_id, target_type;
DEFINE INDEX idx_schedule_next_run ON TABLE schedule COLUMNS next_run_at;

-- ============================================================================
-- Webhooks Table
-- ============================================================================

DEFINE TABLE webhook SCHEMAFULL;

DEFINE FIELD webhook_id ON TABLE webhook TYPE string
    ASSERT $value != NONE AND string::len($value) > 0;

DEFINE FIELD webhook_name ON TABLE webhook TYPE string
    ASSERT $value != NONE AND string::len($value) > 0;

DEFINE FIELD url ON TABLE webhook TYPE string
    ASSERT $value != NONE AND string::len($value) > 0;

DEFINE FIELD secret ON TABLE webhook TYPE option<string>;

DEFINE FIELD events ON TABLE webhook TYPE array<string>
    ASSERT $value != NONE AND array::len($value) > 0;

DEFINE FIELD enabled ON TABLE webhook TYPE bool DEFAULT true;

DEFINE FIELD created_at ON TABLE webhook TYPE datetime VALUE time::now();

DEFINE INDEX idx_webhook_id ON TABLE webhook COLUMNS webhook_id UNIQUE;

-- ============================================================================
-- Graph Relations
-- ============================================================================

RELATE depends_on FROM bead AS source TO bead AS target;

DEFINE FIELD depends_reason ON TABLE depends_on TYPE option<string>;

DEFINE FIELD created_at ON TABLE depends_on TYPE datetime VALUE time::now();

RELATE blocks FROM bead AS blocker TO bead AS blocked;

DEFINE FIELD blocks_reason ON TABLE blocks TYPE option<string>;

DEFINE FIELD created_at ON TABLE blocks TYPE datetime VALUE time::now();

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
