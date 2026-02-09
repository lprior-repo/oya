#!/usr/bin/env bash
# Initialize PostgreSQL Swarm Database for 12-Agent Parallel Execution

set -euo pipefail

# Configuration
DB_NAME="${SWARM_DB:-swarm_db}"
DB_USER="${SWARM_USER:-oya}"
DB_HOST="${SWARM_HOST:-localhost}"
DB_PORT="${SWARM_PORT:-5432}"

SCHEMA_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"

echo "🐝 Initializing PostgreSQL Swarm Database..."
echo "   Database: $DB_NAME"
echo "   Host: $DB_HOST:$DB_PORT"
echo ""

# Check if PostgreSQL is running
if ! pg_isready -h "$DB_HOST" -p "$DB_PORT" -U "$DB_USER" &>/dev/null; then
    echo "❌ PostgreSQL is not running or not accessible"
    echo "   Start PostgreSQL: sudo systemctl start postgresql"
    echo "   Or: docker run -d -p 5432:5432 -e POSTGRES_PASSWORD=oya postgres:16"
    exit 1
fi

echo "✅ PostgreSQL is running"
echo ""

# Create database if it doesn't exist
echo "📋 Creating database (if not exists)..."
psql -h "$DB_HOST" -p "$DB_PORT" -U "$DB_USER" -d postgres <<SQL 2>/dev/null || true
SELECT 'CREATE DATABASE $DB_NAME' WHERE NOT EXISTS (
    SELECT FROM pg_database WHERE datname = '$DB_NAME'
)\\gexec
SQL

echo "✅ Database ready: $DB_NAME"
echo ""

# Load schema
echo "📋 Loading schema..."
psql -h "$DB_HOST" -p "$DB_PORT" -U "$DB_USER" -d "$DB_NAME" < "$SCHEMA_DIR/../swarm-coordinator/schema.sql"

echo "✅ Schema loaded"
echo ""

# Initialize 12 agents in idle state
echo "🤖 Registering 12 agents..."
for i in {1..12}; do
    psql -h "$DB_HOST" -p "$DB_PORT" -U "$DB_USER" -d "$DB_NAME" -c "
        INSERT INTO agent_state (
            agent_id,
            bead_id,
            current_stage,
            stage_started_at,
            status,
            last_update,
            implementation_attempt,
            feedback
        ) VALUES (
            $i,
            NULL,
            NULL,
            NULL,
            'idle',
            NOW(),
            0,
            NULL
        ) ON CONFLICT (agent_id) DO NOTHING;
    "
done

echo "✅ 12 agents registered"
echo ""

# Show swarm status
echo "📊 Swarm Status:"
psql -h "$DB_HOST" -p "$DB_PORT" -U "$DB_USER" -d "$DB_NAME" -c "SELECT * FROM v_swarm_progress;"

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
echo "   1. Ensure beads exist in .beads/beads.db"
echo "   2. Launch 12 agents using Task tool"
echo "   3. Monitor: psql -c 'SELECT * FROM v_active_agents;'"
