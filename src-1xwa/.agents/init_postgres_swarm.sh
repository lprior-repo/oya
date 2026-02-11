#!/usr/bin/env bash
# Initialize PostgreSQL Swarm Database for N-Agent Parallel Execution

set -euo pipefail

# Configuration
DB_NAME="${SWARM_DB:-swarm_db}"
DB_USER="${SWARM_USER:-postgres}"
DB_HOST="${SWARM_HOST:-localhost}"
DB_PORT="${SWARM_PORT:-5432}"
NUM_AGENTS="${1:-12}"

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"

echo "🐝 Initializing PostgreSQL Swarm Database..."
echo "   Database: $DB_NAME"
echo "   Host: $DB_HOST:$DB_PORT"
echo "   Agents: $NUM_AGENTS"
echo ""

# Check if PostgreSQL is running
if ! pg_isready -h "$DB_HOST" -p "$DB_PORT" &>/dev/null; then
	echo "❌ PostgreSQL is not running or not accessible"
	echo "   Start PostgreSQL: sudo systemctl start postgresql"
	echo "   Or: docker run -d -p 5432:5432 -e POSTGRES_PASSWORD=postgres postgres:16"
	exit 1
fi

echo "✅ PostgreSQL is running"
echo ""

# Create database if it doesn't exist
echo "📋 Creating database (if not exists)..."
psql -h "$DB_HOST" -p "$DB_PORT" -U "$DB_USER" -d postgres <<SQL 2>/dev/null || true
SELECT 'CREATE DATABASE $DB_NAME' WHERE NOT EXISTS (
    SELECT FROM pg_database WHERE datname = '$DB_NAME'
)\gexec
SQL

echo "✅ Database ready: $DB_NAME"
echo ""

# Load schema
echo "📋 Loading schema..."
SCHEMA_FILE="$SCRIPT_DIR/../crates/swarm-coordinator/schema.sql"
if [[ -f "$SCHEMA_FILE" ]]; then
	psql -h "$DB_HOST" -p "$DB_PORT" -U "$DB_USER" -d "$DB_NAME" <"$SCHEMA_FILE"
else
	echo "⚠️  Schema file not found at $SCHEMA_FILE"
	echo "   Using fallback schema..."

	# Create minimal schema inline
	psql -h "$DB_HOST" -p "$DB_PORT" -U "$DB_USER" -d "$DB_NAME" <<'SCHEMA'
-- Minimal swarm schema
CREATE TABLE IF NOT EXISTS agent_state (
    agent_id INTEGER PRIMARY KEY,
    bead_id TEXT,
    current_stage TEXT,
    stage_started_at TIMESTAMPTZ,
    status TEXT DEFAULT 'idle',
    last_update TIMESTAMPTZ DEFAULT NOW(),
    implementation_attempt INTEGER DEFAULT 0,
    feedback TEXT
);

CREATE TABLE IF NOT EXISTS bead_claims (
    bead_id TEXT PRIMARY KEY,
    claimed_by INTEGER,
    claimed_at TIMESTAMPTZ DEFAULT NOW(),
    status TEXT DEFAULT 'in_progress'
);

CREATE TABLE IF NOT EXISTS stage_history (
    id BIGSERIAL PRIMARY KEY,
    agent_id INTEGER,
    bead_id TEXT,
    stage TEXT,
    attempt_number INTEGER,
    status TEXT,
    feedback TEXT,
    started_at TIMESTAMPTZ DEFAULT NOW(),
    completed_at TIMESTAMPTZ
);

CREATE TABLE IF NOT EXISTS pipeline_config (
    key TEXT PRIMARY KEY,
    value TEXT,
    updated_at TIMESTAMPTZ DEFAULT NOW()
);

-- Default config
INSERT INTO pipeline_config (key, value) VALUES
    ('max_agents', '12'),
    ('max_implementation_attempts', '3'),
    ('claim_label', 'p0')
ON CONFLICT (key) DO NOTHING;
SCHEMA
fi

echo "✅ Schema loaded"
echo ""

# Initialize N agents in idle state
echo "🤖 Registering $NUM_AGENTS agents..."
for i in $(seq 1 $NUM_AGENTS); do
	psql -h "$DB_HOST" -p "$DB_PORT" -U "$DB_USER" -d "$DB_NAME" -c "
        INSERT INTO agent_state (agent_id, status, last_update, implementation_attempt)
        VALUES ($i, 'idle', NOW(), 0)
        ON CONFLICT (agent_id) DO NOTHING;
    " 2>/dev/null
done

echo "✅ $NUM_AGENTS agents registered"
echo ""

# Show swarm status
echo "📊 Swarm Status:"
psql -h "$DB_HOST" -p "$DB_PORT" -U "$DB_USER" -d "$DB_NAME" -c "
    SELECT 
        COUNT(*) FILTER (WHERE status = 'idle') as idle,
        COUNT(*) FILTER (WHERE status = 'working') as working,
        COUNT(*) FILTER (WHERE status = 'done') as done,
        COUNT(*) as total
    FROM agent_state;
"

echo ""
echo "🔧 Connection Details:"
echo "   export SWARM_DB=$DB_NAME"
echo "   export SWARM_USER=$DB_USER"
echo "   export SWARM_HOST=$DB_HOST"
echo "   export SWARM_PORT=$DB_PORT"
echo ""
echo "   psql -h $DB_HOST -p $DB_PORT -U $DB_USER -d $DB_NAME"
echo ""
echo "📋 Next Steps:"
echo "   1. Ensure beads exist: bv --robot-triage"
echo "   2. Launch agents: ./.agents/launch_swarm.sh"
echo "   3. Monitor: watch -n 2 'psql -c \"SELECT * FROM v_active_agents;\"'"
