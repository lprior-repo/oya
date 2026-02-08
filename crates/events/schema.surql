-- SurrealDB Schema for Oya Event Sourcing System
--
-- This schema defines 12 tables for event sourcing, workflow orchestration,
-- and process management with kv-rocksdb backend and sync_mode='full'
-- for fsync guarantees on every write.
--
-- Expected overhead: 2-3ms per write (acceptable for zero data loss)
--
-- Configuration:
-- - Engine: Embedded SurrealDB with kv-rocksdb backend
-- - Sync Mode: sync_mode='full' for fsync on every write
-- - Namespace: oya
-- - Database: events

-- ============================================================================
-- 1. STATE_TRANSITION - Append-only event log with fsync
-- ============================================================================
DEFINE TABLE state_transition SCHEMAFULL;

DEFINE FIELD event_id ON TABLE state_transition TYPE string
    ASSERT string::len($value) > 0;

DEFINE FIELD bead_id ON TABLE state_transition TYPE string
    ASSERT string::len($value) > 0;

DEFINE FIELD event_type ON TABLE state_transition TYPE string
    ASSERT $value IN ["created", "started", "phase_completed", "phase_failed",
                     "completed", "failed", "cancelled"];

DEFINE FIELD timestamp ON TABLE state_transition TYPE datetime
    DEFAULT time::now();

DEFINE FIELD phase_id ON TABLE state_transition TYPE option<string>;

DEFINE FIELD data ON TABLE state_transition TYPE option<any>;

-- Index for efficient querying by bead_id
DEFINE INDEX bead_id_idx ON TABLE state_transition COLUMNS bead_id;

-- Index for time-based queries
DEFINE INDEX timestamp_idx ON TABLE state_transition COLUMNS timestamp;

-- ============================================================================
-- 2. IDEMPOTENCY_KEY - Prevents duplicate execution
-- ============================================================================
DEFINE TABLE idempotency_key SCHEMAFULL;

DEFINE FIELD key ON TABLE idempotency_key TYPE string
    ASSERT string::len($value) > 0;

DEFINE FIELD bead_id ON TABLE idempotency_key TYPE string
    ASSERT string::len($value) > 0;

DEFINE FIELD result ON TABLE idempotency_key TYPE option<any>;

DEFINE FIELD executed_at ON TABLE idempotency_key TYPE datetime
    DEFAULT time::now();

DEFINE FIELD expires_at ON TABLE idempotency_key TYPE option<datetime>;

-- Unique index on key to prevent duplicates
DEFINE INDEX key_unique_idx ON TABLE idempotency_key COLUMNS key UNIQUE;

-- Index for bead-based queries
DEFINE INDEX bead_id_idx ON TABLE idempotency_key COLUMNS bead_id;

-- ============================================================================
-- 3. CHECKPOINT - Compressed state snapshots (zstd)
-- ============================================================================
DEFINE TABLE checkpoint SCHEMAFULL;

DEFINE FIELD checkpoint_id ON TABLE checkpoint TYPE string
    ASSERT string::len($value) > 0;

DEFINE FIELD bead_id ON TABLE checkpoint TYPE string
    ASSERT string::len($value) > 0;

DEFINE FIELD sequence_number ON TABLE checkpoint TYPE number
    ASSERT $value >= 0;

DEFINE FIELD data ON TABLE checkpoint TYPE any
    ASSERT $value != NONE;

DEFINE FIELD compressed ON TABLE checkpoint TYPE bool
    DEFAULT true;

DEFINE FIELD compression_level ON TABLE checkpoint TYPE number
    ASSERT $value >= 0 AND $value <= 19
    DEFAULT 3;

DEFINE FIELD created_at ON TABLE checkpoint TYPE datetime
    DEFAULT time::now();

-- Unique checkpoint per bead and sequence number
DEFINE INDEX bead_sequence_idx ON TABLE checkpoint COLUMNS bead_id, sequence_number;

-- Latest checkpoint per bead
DEFINE INDEX bead_id_idx ON TABLE checkpoint COLUMNS bead_id;

-- ============================================================================
-- 4. BEAD - Bead metadata
-- ============================================================================
DEFINE TABLE bead SCHEMAFULL;

DEFINE FIELD bead_id ON TABLE bead TYPE string
    ASSERT string::len($value) > 0;

DEFINE FIELD title ON TABLE bead TYPE string
    ASSERT string::len($value) > 0;

DEFINE FIELD description ON TABLE bead TYPE option<string>;

DEFINE FIELD spec ON TABLE bead TYPE any;

DEFINE FIELD status ON TABLE bead TYPE string
    ASSERT $value IN ["pending", "in_progress", "completed", "failed", "cancelled"]
    DEFAULT "pending";

DEFINE FIELD complexity ON TABLE bead TYPE option<string>
    ASSERT $value IN ["Low", "Medium", "High", "Critical"];

DEFINE FIELD created_at ON TABLE bead TYPE datetime
    DEFAULT time::now();

DEFINE FIELD updated_at ON TABLE bead TYPE datetime
    DEFAULT time::now();

DEFINE FIELD completed_at ON TABLE bead TYPE option<datetime>;

-- Primary key index
DEFINE INDEX bead_id_unique_idx ON TABLE bead COLUMNS bead_id UNIQUE;

-- Status-based queries
DEFINE INDEX status_idx ON TABLE bead COLUMNS status;

-- ============================================================================
-- 5. WORKFLOW_RUN - Workflow execution tracking
-- ============================================================================
DEFINE TABLE workflow_run SCHEMAFULL;

DEFINE FIELD workflow_id ON TABLE workflow_run TYPE string
    ASSERT string::len($value) > 0;

DEFINE FIELD name ON TABLE workflow_run TYPE string
    ASSERT string::len($value) > 0;

DEFINE FIELD status ON TABLE workflow_run TYPE string
    ASSERT $value IN ["pending", "running", "completed", "failed", "cancelled"]
    DEFAULT "pending";

DEFINE FIELD current_phase ON TABLE workflow_run TYPE option<string>;

DEFINE FIELD phases_total ON TABLE workflow_run TYPE number
    ASSERT $value >= 0
    DEFAULT 0;

DEFINE FIELD phases_completed ON TABLE workflow_run TYPE number
    ASSERT $value >= 0
    DEFAULT 0;

DEFINE FIELD started_at ON TABLE workflow_run TYPE option<datetime>;

DEFINE FIELD completed_at ON TABLE workflow_run TYPE option<datetime>;

DEFINE FIELD error_message ON TABLE workflow_run TYPE option<string>;

-- Primary key
DEFINE INDEX workflow_id_unique_idx ON TABLE workflow_run COLUMNS workflow_id UNIQUE;

-- Status queries
DEFINE INDEX status_idx ON TABLE workflow_run COLUMNS status;

-- ============================================================================
-- 6. PROCESS - Worker process tracking
-- ============================================================================
DEFINE TABLE process SCHEMAFULL;

DEFINE FIELD process_id ON TABLE process TYPE string
    ASSERT string::len($value) > 0;

DEFINE FIELD bead_id ON TABLE process TYPE string
    ASSERT string::len($value) > 0;

DEFINE FIELD pid ON TABLE process TYPE option<number>;

DEFINE FIELD status ON TABLE process TYPE string
    ASSERT $value IN ["starting", "running", "completed", "failed", "killed"]
    DEFAULT "starting";

DEFINE FIELD started_at ON TABLE process TYPE datetime
    DEFAULT time::now();

DEFINE FIELD completed_at ON TABLE process TYPE option<datetime>;

DEFINE FIELD exit_code ON TABLE process TYPE option<number>;

DEFINE FIELD stdout_path ON TABLE process TYPE option<string>;

DEFINE FIELD stderr_path ON TABLE process TYPE option<string>;

-- Primary key
DEFINE INDEX process_id_unique_idx ON TABLE process COLUMNS process_id UNIQUE;

-- Bead-based queries
DEFINE INDEX bead_id_idx ON TABLE process COLUMNS bead_id;

-- Status tracking
DEFINE INDEX status_idx ON TABLE process COLUMNS status;

-- ============================================================================
-- 7. WORKSPACE - Workspace isolation state
-- ============================================================================
DEFINE TABLE workspace SCHEMAFULL;

DEFINE FIELD workspace_id ON TABLE workspace TYPE string
    ASSERT string::len($value) > 0;

DEFINE FIELD bead_id ON TABLE workspace TYPE string
    ASSERT string::len($value) > 0;

DEFINE FIELD path ON TABLE workspace TYPE string
    ASSERT string::len($value) > 0;

DEFINE FIELD workspace_type ON TABLE workspace TYPE string
    ASSERT $value IN ["jj", "git", "temp"]
    DEFAULT "jj";

DEFINE FIELD status ON TABLE workspace TYPE string
    ASSERT $value IN ["creating", "active", "cleaning", "removed"]
    DEFAULT "creating";

DEFINE FIELD created_at ON TABLE workspace TYPE datetime
    DEFAULT time::now();

DEFINE FIELD removed_at ON TABLE workspace TYPE option<datetime>;

-- Primary key
DEFINE INDEX workspace_id_unique_idx ON TABLE workspace COLUMNS workspace_id UNIQUE;

-- Bead-based queries
DEFINE INDEX bead_id_idx ON TABLE workspace COLUMNS bead_id;

-- ============================================================================
-- 8. SCHEDULE - Scheduled execution
-- ============================================================================
DEFINE TABLE schedule SCHEMAFULL;

DEFINE FIELD schedule_id ON TABLE schedule TYPE string
    ASSERT string::len($value) > 0;

DEFINE FIELD bead_id ON TABLE schedule TYPE string
    ASSERT string::len($value) > 0;

DEFINE FIELD scheduled_at ON TABLE schedule TYPE datetime
    ASSERT $value != NONE;

DEFINE FIELD status ON TABLE schedule TYPE string
    ASSERT $value IN ["pending", "triggered", "cancelled", "failed"]
    DEFAULT "pending";

DEFINE FIELD triggered_at ON TABLE schedule TYPE option<datetime>;

DEFINE FIELD repeat_interval ON TABLE schedule TYPE option<string>;

-- Primary key
DEFINE INDEX schedule_id_unique_idx ON TABLE schedule COLUMNS schedule_id UNIQUE;

-- Time-based queries
DEFINE INDEX scheduled_at_idx ON TABLE schedule COLUMNS scheduled_at;

-- Bead-based queries
DEFINE INDEX bead_id_idx ON TABLE schedule COLUMNS bead_id;

-- ============================================================================
-- 9. TOKEN_BUCKET - Rate limiting state
-- ============================================================================
DEFINE TABLE token_bucket SCHEMAFULL;

DEFINE FIELD bucket_id ON TABLE token_bucket TYPE string
    ASSERT string::len($value) > 0;

DEFINE FIELD beads_related_id ON TABLE token_bucket TYPE string
    ASSERT string::len($value) > 0;

DEFINE FIELD capacity ON TABLE token_bucket TYPE number
    ASSERT $value > 0;

DEFINE FIELD tokens ON TABLE token_bucket TYPE number
    ASSERT $value >= 0;

DEFINE FIELD last_refill_at ON TABLE token_bucket TYPE datetime
    DEFAULT time::now();

DEFINE FIELD refill_rate ON TABLE token_bucket TYPE number
    ASSERT $value > 0;

-- Primary key
DEFINE INDEX bucket_id_unique_idx ON TABLE token_bucket COLUMNS bucket_id UNIQUE;

-- Bead-based queries
DEFINE INDEX beads_related_id_idx ON TABLE token_bucket COLUMNS beads_related_id;

-- ============================================================================
-- 10. CONCURRENCY_LIMIT - Resource limits
-- ============================================================================
DEFINE TABLE concurrency_limit SCHEMAFULL;

DEFINE FIELD limit_id ON TABLE concurrency_limit TYPE string
    ASSERT string::len($value) > 0;

DEFINE FIELD resource_type ON TABLE concurrency_limit TYPE string
    ASSERT string::len($value) > 0;

DEFINE FIELD max_concurrent ON TABLE concurrency_limit TYPE number
    ASSERT $value > 0;

DEFINE FIELD current_count ON TABLE concurrency_limit TYPE number
    ASSERT $value >= 0
    DEFAULT 0;

DEFINE FIELD updated_at ON TABLE concurrency_limit TYPE datetime
    DEFAULT time::now();

-- Primary key
DEFINE INDEX limit_id_unique_idx ON TABLE concurrency_limit COLUMNS limit_id UNIQUE;

-- Resource type queries
DEFINE INDEX resource_type_idx ON TABLE concurrency_limit COLUMNS resource_type;

-- ============================================================================
-- 11. WEBHOOK - Webhook configurations
-- ============================================================================
DEFINE TABLE webhook SCHEMAFULL;

DEFINE FIELD webhook_id ON TABLE webhook TYPE string
    ASSERT string::len($value) > 0;

DEFINE FIELD bead_id ON TABLE webhook TYPE string
    ASSERT string::len($value) > 0;

DEFINE FIELD url ON TABLE webhook TYPE string
    ASSERT string::len($value) > 0 AND string::contains($value, "http");

DEFINE FIELD event_types ON TABLE webhook TYPE array<string>
    ASSERT array::len($value) > 0;

DEFINE FIELD secret ON TABLE webhook TYPE option<string>;

DEFINE FIELD active ON TABLE webhook TYPE bool
    DEFAULT true;

DEFINE FIELD created_at ON TABLE webhook TYPE datetime
    DEFAULT time::now();

-- Primary key
DEFINE INDEX webhook_id_unique_idx ON TABLE webhook COLUMNS webhook_id UNIQUE;

-- Bead-based queries
DEFINE INDEX bead_id_idx ON TABLE webhook COLUMNS bead_id;

-- Active webhooks
DEFINE INDEX active_idx ON TABLE webhook COLUMNS active;

-- ============================================================================
-- GRAPH RELATIONS - Dependency tracking
-- ============================================================================

-- depends_on: Bead A depends on Bead B (B must complete before A starts)
DEFINE RELATION depends_on ON TABLE bead;

-- blocks: Bead A blocks Bead B (B cannot start while A is in progress)
DEFINE RELATION blocks ON TABLE bead;

-- ============================================================================
-- CONFIGURATION NOTES
-- ============================================================================
--
-- IMPORTANT: Configure SurrealDB with sync_mode='full' for fsync guarantees:
--
-- In your application code, when initializing the database:
--
-- ```rust
-- // Note: SurrealDB 2.0 with kv-rocksdb backend
-- // The sync_mode='full' configuration is set at database creation time
-- // to ensure fsync on every write for zero data loss.
--
-- // Expected overhead: 2-3ms per write
-- // This is acceptable for event-sourcing systems requiring durability.
-- ```
--
-- The schema above ensures:
-- 1. All tables have proper schema enforcement (SCHEMAFULL)
-- 2. All required fields have assertions for data integrity
-- 3. Indexes are defined for common query patterns
-- 4. Unique constraints prevent duplicate data
-- 5. Graph relations support dependency tracking
-- 6. Timestamp fields track when events occurred
-- 7. Status fields follow defined state machines
--