#!/usr/bin/env bash
# Launch 12 parallel agents for bead processing swarm

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROMPT_FILE="$SCRIPT_DIR/agent_prompt.md"

# Ensure database is initialized
if ! psql -h "${SWARM_HOST:-localhost}" -p "${SWARM_PORT:-5432}" -U "${SWARM_USER:-oya}" -d "${SWARM_DB:-swarm_db}" -c "SELECT 1" &>/dev/null; then
    echo "❌ Database not initialized. Running init script..."
    bash "$SCRIPT_DIR/init_postgres_swarm.sh"
fi

echo "🚀 Launching 12-Agent Swarm..."
echo ""

# Read the prompt template
PROMPT_TEMPLATE=$(cat "$PROMPT_FILE")

# Spawn 12 agents in parallel using background tasks
for i in {1..12}; do
    AGENT_PROMPT=$(echo "$PROMPT_TEMPLATE" | sed "s/{N}/$i/g")

    echo "🤖 Spawning Agent #$i..."

    # Use Task tool to launch each agent in background
    # Note: This will be called from Claude Code, not bash
    # The actual spawning will happen via Task tool calls

    # For now, save the prompt to a file for each agent
    echo "$AGENT_PROMPT" > "$SCRIPT_DIR/agent_$i.md"
done

echo ""
echo "✅ Agent prompts generated: .agents/agent_1.md through agent_12.md"
echo ""
echo "📋 To launch agents from Claude Code, run:"
echo ""
echo "   for i in {1..12}; do"
echo "     # Spawn each agent using Task tool with prompt from .agents/agent_\$i.md"
echo "     # Use run_in_background=true"
echo "   done"
echo ""
echo "🔍 Monitor swarm:"
echo "   psql -h \${SWARM_HOST:-localhost} -p \${SWARM_PORT:-5432} -U \${SWARM_USER:-oya} -d \${SWARM_DB:-swarm_db} -c 'SELECT * FROM v_active_agents;'"
echo ""
echo "📊 View progress:"
echo "   psql -h \${SWARM_HOST:-localhost} -p \${SWARM_PORT:-5432} -U \${SWARM_USER:-oya} -d \${SWARM_DB:-swarm_db} -c 'SELECT * FROM v_swarm_progress;'"
