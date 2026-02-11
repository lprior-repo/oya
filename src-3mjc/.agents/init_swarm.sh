#!/usr/bin/env bash
# Initialize Swarm State Database and Agent Registry

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
DB_PATH="$SCRIPT_DIR/swarm_state.db"

echo "🐝 Initializing Swarm State Database..."

# Remove existing database if present
if [[ -f "$DB_PATH" ]]; then
    echo "  ⚠️  Removing existing database..."
    rm "$DB_PATH"
fi

# Create database and schema
echo "  📋 Creating schema..."
sqlite3 "$DB_PATH" < "$SCRIPT_DIR/swarm_state.sql"

# Initialize 12 agents in idle state
echo "  🤖 Registering 12 agents..."
for i in {1..12}; do
    sqlite3 "$DB_PATH" <<SQL
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
    datetime('now'),
    0,
    NULL
);
SQL
done

echo "  ✅ Database initialized: $DB_PATH"
echo ""
echo "📊 Swarm Status:"
sqlite3 "$DB_PATH" "SELECT * FROM v_swarm_progress;"

echo ""
echo "🔧 Next Steps:"
echo "  1. Claim beads: UPDATE bead_claims SET (...)"
echo "  2. Launch agents: Task tool with swarm prompts"
echo "  3. Monitor: sqlite3 $DB_PATH 'SELECT * FROM v_active_agents;'"
