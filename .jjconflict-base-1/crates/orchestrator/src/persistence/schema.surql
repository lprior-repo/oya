-- Orchestrator Schema for SurrealDB
-- This schema defines the tables for workflows, beads, and checkpoints

-- ============================================================================
-- WORKFLOWS TABLE
-- ============================================================================
-- Stores workflow definitions and their DAG structure

DEFINE TABLE IF NOT EXISTS workflow SCHEMAFULL;

DEFINE FIELD IF NOT EXISTS workflow_id ON workflow TYPE string;
DEFINE FIELD IF NOT EXISTS name ON workflow TYPE string;
DEFINE FIELD IF NOT EXISTS description ON workflow TYPE option<string>;
DEFINE FIELD IF NOT EXISTS dag_json ON workflow TYPE string;
DEFINE FIELD IF NOT EXISTS status ON workflow TYPE string;
DEFINE FIELD IF NOT EXISTS created_at ON workflow TYPE datetime;
DEFINE FIELD IF NOT EXISTS updated_at ON workflow TYPE datetime;
DEFINE FIELD IF NOT EXISTS completed_at ON workflow TYPE option<datetime>;
DEFINE FIELD IF NOT EXISTS metadata ON workflow TYPE option<object>;

DEFINE INDEX IF NOT EXISTS workflow_status ON workflow FIELDS status;
DEFINE INDEX IF NOT EXISTS workflow_created ON workflow FIELDS created_at;

-- ============================================================================
-- BEADS TABLE
-- ============================================================================
-- Stores individual bead records and their execution state

DEFINE TABLE IF NOT EXISTS bead SCHEMAFULL;

DEFINE FIELD IF NOT EXISTS bead_id ON bead TYPE string;
DEFINE FIELD IF NOT EXISTS workflow_id ON bead TYPE string;
DEFINE FIELD IF NOT EXISTS state ON bead TYPE string;
DEFINE FIELD IF NOT EXISTS assigned_worker ON bead TYPE option<string>;
DEFINE FIELD IF NOT EXISTS assigned_queue ON bead TYPE option<string>;
DEFINE FIELD IF NOT EXISTS created_at ON bead TYPE datetime;
DEFINE FIELD IF NOT EXISTS updated_at ON bead TYPE datetime;
DEFINE FIELD IF NOT EXISTS started_at ON bead TYPE option<datetime>;
DEFINE FIELD IF NOT EXISTS completed_at ON bead TYPE option<datetime>;
DEFINE FIELD IF NOT EXISTS error_message ON bead TYPE option<string>;
DEFINE FIELD IF NOT EXISTS retry_count ON bead TYPE int DEFAULT 0;
DEFINE FIELD IF NOT EXISTS metadata ON bead TYPE option<object>;

DEFINE INDEX IF NOT EXISTS bead_workflow ON bead FIELDS workflow_id;
DEFINE INDEX IF NOT EXISTS bead_state ON bead FIELDS state;
DEFINE INDEX IF NOT EXISTS bead_worker ON bead FIELDS assigned_worker;

-- ============================================================================
-- CHECKPOINTS TABLE
-- ============================================================================
-- Stores scheduler state checkpoints for recovery

DEFINE TABLE IF NOT EXISTS checkpoint SCHEMAFULL;

DEFINE FIELD IF NOT EXISTS checkpoint_id ON checkpoint TYPE string;
DEFINE FIELD IF NOT EXISTS scheduler_state ON checkpoint TYPE string;
DEFINE FIELD IF NOT EXISTS event_sequence ON checkpoint TYPE int;
DEFINE FIELD IF NOT EXISTS created_at ON checkpoint TYPE datetime;
DEFINE FIELD IF NOT EXISTS workflow_snapshots ON checkpoint TYPE option<string>;
DEFINE FIELD IF NOT EXISTS metadata ON checkpoint TYPE option<object>;

DEFINE INDEX IF NOT EXISTS checkpoint_sequence ON checkpoint FIELDS event_sequence;
DEFINE INDEX IF NOT EXISTS checkpoint_created ON checkpoint FIELDS created_at;

-- ============================================================================
-- EVENTS TABLE
-- ============================================================================
-- Stores orchestrator events for replay

DEFINE TABLE IF NOT EXISTS orchestrator_event SCHEMAFULL;

DEFINE FIELD IF NOT EXISTS event_id ON orchestrator_event TYPE string;
DEFINE FIELD IF NOT EXISTS event_type ON orchestrator_event TYPE string;
DEFINE FIELD IF NOT EXISTS event_data ON orchestrator_event TYPE string;
DEFINE FIELD IF NOT EXISTS sequence ON orchestrator_event TYPE int;
DEFINE FIELD IF NOT EXISTS timestamp ON orchestrator_event TYPE datetime;
DEFINE FIELD IF NOT EXISTS workflow_id ON orchestrator_event TYPE option<string>;
DEFINE FIELD IF NOT EXISTS bead_id ON orchestrator_event TYPE option<string>;

DEFINE INDEX IF NOT EXISTS event_sequence ON orchestrator_event FIELDS sequence;
DEFINE INDEX IF NOT EXISTS event_type_idx ON orchestrator_event FIELDS event_type;
DEFINE INDEX IF NOT EXISTS event_workflow ON orchestrator_event FIELDS workflow_id;

-- ============================================================================
-- AGENTS TABLE
-- ============================================================================
-- Stores agent registration and health information

DEFINE TABLE IF NOT EXISTS agent SCHEMAFULL;

DEFINE FIELD IF NOT EXISTS id ON agent TYPE string;
DEFINE FIELD IF NOT EXISTS state ON agent TYPE string;
DEFINE FIELD IF NOT EXISTS current_bead ON agent TYPE option<string>;
DEFINE FIELD IF NOT EXISTS last_heartbeat ON agent TYPE datetime;
DEFINE FIELD IF NOT EXISTS registered_at ON agent TYPE datetime;
DEFINE FIELD IF NOT EXISTS capabilities ON agent TYPE option<array>;
DEFINE FIELD IF NOT EXISTS metadata ON agent TYPE option<object>;

DEFINE INDEX IF NOT EXISTS agent_state ON agent FIELDS state;
DEFINE INDEX IF NOT EXISTS agent_heartbeat ON agent FIELDS last_heartbeat;
