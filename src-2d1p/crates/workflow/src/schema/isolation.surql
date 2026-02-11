-- Isolation Schema for SurrealDB
-- Provides workspace isolation (zjj integration) and scheduled execution (cron-like)

-- ============================================================================
-- Workspace Table (zjj Session Tracking)
-- ============================================================================
-- Tracks isolated workspaces created by zjj:
-- - Maps to zjj sessions with workspace_path, branch, and status
-- - Enforces unique constraints on workspace paths
-- - Supports state transitions for workspace lifecycle

DEFINE TABLE workspace SCHEMAFULL;

-- Primary key: ULID-generated unique identifier
-- DEFINE FIELD id ON TABLE workspace TYPE string
--     ASSERT $value != NONE AND string::len($value) > 0;

-- Workspace path (unique across all workspaces)
DEFINE FIELD workspace_path ON TABLE workspace TYPE string
    ASSERT $value != NONE AND string::len($value) > 0;

-- Branch name for the workspace
DEFINE FIELD branch ON TABLE workspace TYPE string
    ASSERT $value != NONE AND string::len($value) > 0;

-- Workspace status: creating, active, paused, completed, failed
DEFINE FIELD status ON TABLE workspace TYPE string
    ASSERT $value IN ['creating', 'active', 'paused', 'completed', 'failed'];

-- Timestamp when workspace was created
DEFINE FIELD created_at ON TABLE workspace TYPE datetime;

-- Timestamp of last update (status change, sync, etc.)
DEFINE FIELD updated_at ON TABLE workspace TYPE datetime;

-- Last time workspace was synced with main (optional)
DEFINE FIELD last_synced ON TABLE workspace TYPE option datetime;

-- Unique index on workspace_path (enforces one workspace per path)
DEFINE INDEX idx_workspace_path ON TABLE workspace COLUMNS workspace_path UNIQUE;

-- Index for querying by status (find all active workspaces)
DEFINE INDEX idx_workspace_status ON TABLE workspace COLUMNS status;

-- Index for querying by last_synced (find stale workspaces)
DEFINE INDEX idx_workspace_last_synced ON TABLE workspace COLUMNS last_synced;

-- ============================================================================
-- Schedule Table (Deferred Execution)
-- ============================================================================
-- Manages scheduled tasks with cron-like execution:
-- - Stores cron expressions for scheduling
-- - Tracks next_run and last_run timestamps
-- - Supports enabling/disabling schedules

DEFINE TABLE schedule SCHEMAFULL;

-- Primary key: ULID-generated unique identifier
-- DEFINE FIELD id ON TABLE schedule TYPE string
--     ASSERT $value != NONE AND string::len($value) > 0;

-- Human-readable schedule name
DEFINE FIELD name ON TABLE schedule TYPE string
    ASSERT $value != NONE AND string::len($value) > 0;

-- Cron expression (5 or 6 fields: minute hour day month weekday [year])
DEFINE FIELD cron_expr ON TABLE schedule TYPE string
    ASSERT $value != NONE AND string::len($value) > 0;

-- Next scheduled execution time
DEFINE FIELD next_run ON TABLE schedule TYPE datetime
    ASSERT $value >= time::now();

-- Last execution time (optional, null until first run)
DEFINE FIELD last_run ON TABLE schedule TYPE option datetime;

-- Timestamp when schedule was created
DEFINE FIELD created_at ON TABLE schedule TYPE datetime;

-- Timestamp of last update
DEFINE FIELD updated_at ON TABLE schedule TYPE datetime;

-- Whether the schedule is enabled for execution
DEFINE FIELD enabled ON TABLE schedule TYPE bool
    ASSERT $value IN [true, false];

-- Unique index on schedule name
DEFINE INDEX idx_schedule_name ON TABLE schedule COLUMNS name UNIQUE;

-- Index for finding due schedules (enabled and next_run <= now)
DEFINE INDEX idx_schedule_due ON TABLE schedule COLUMNS next_run;

-- Index for enabled status
DEFINE INDEX idx_schedule_enabled ON TABLE schedule COLUMNS enabled;

-- ============================================================================
-- Atomic Operations (Implemented via SurrealDB queries)
-- ============================================================================
--
-- Workspace Operations:
--
-- 1. Create workspace:
--    CREATE workspace CONTENT {
--        workspace_path: '/path/to/workspace',
--        branch: 'feature-branch',
--        status: 'creating',
--        created_at: time::now(),
--        updated_at: time::now()
--    };
--
-- 2. Transition status:
--    UPDATE workspace:<id> SET
--        status = 'active',
--        updated_at = time::now()
--    WHERE id = <id>
--    RETURN AFTER;
--
-- 3. Update sync timestamp:
--    UPDATE workspace:<id> SET
--        last_synced = time::now(),
--        updated_at = time::now()
--    WHERE id = <id>
--    RETURN AFTER;
--
-- 4. Find active workspaces:
--    SELECT * FROM workspace WHERE status = 'active';
--
-- 5. Find stale workspaces (not synced in > 1 hour):
--    SELECT * FROM workspace
--    WHERE status = 'active'
--      AND (last_synced == NONE OR last_synced < time::now() - 1h);
--
-- Schedule Operations:
--
-- 1. Create schedule:
--    CREATE schedule CONTENT {
--        name: 'daily-backup',
--        cron_expr: '0 2 * * *',
--        next_run: <calculate-next-run>,
--        created_at: time::now(),
--        updated_at: time::now(),
--        enabled: true
--    };
--
-- 2. Find due schedules:
--    SELECT * FROM schedule
--    WHERE enabled = true
--      AND next_run <= time::now();
--
-- 3. Update after execution:
--    UPDATE schedule:<id> SET
--        last_run = time::now(),
--        next_run = <calculate-next-run>,
--        updated_at = time::now()
--    WHERE id = <id>
--    RETURN AFTER;
--
-- 4. Disable schedule:
--    UPDATE schedule:<id> SET
--        enabled = false,
--        updated_at = time::now()
--    WHERE id = <id>;
--
-- 5. Enable schedule:
--    UPDATE schedule:<id> SET
--        enabled = true,
--        updated_at = time::now()
--    WHERE id = <id>;

-- ============================================================================
-- Example Usage
-- ============================================================================

-- Create a workspace for a zjj session
-- CREATE workspace SET
--   workspace_path = '/home/user/repo__workspaces/feature-auth',
--   branch = 'feature-auth',
--   status = 'creating',
--   created_at = time::now(),
--   updated_at = time::now();

-- Activate the workspace after creation completes
-- UPDATE workspace:ulid SET
--   status = 'active',
--   updated_at = time::now()
-- WHERE id = 'ulid';

-- Create a daily backup schedule (runs at 2 AM)
-- CREATE schedule SET
--   name = 'daily-backup',
--   cron_expr = '0 2 * * *',
--   next_run = <calculate-next-midnight-2am>,
--   created_at = time::now(),
--   updated_at = time::now(),
--   enabled = true;

-- Find all due schedules for execution
-- SELECT * FROM schedule WHERE enabled = true AND next_run <= time::now();

-- Find all active workspaces for monitoring
-- SELECT * FROM workspace WHERE status = 'active';
