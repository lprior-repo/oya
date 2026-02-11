#!/usr/bin/env bash
# Monitor the 12-agent swarm

watch -n 2 -c "
echo '=== 12-Agent Swarm Status ==='
echo ''
psql -U postgres -d swarm_db -c 'SELECT * FROM v_swarm_progress;'
echo ''
echo '=== Active Agents ==='
psql -U postgres -d swarm_db -c 'SELECT agent_id, bead_id, current_stage, status FROM agent_state WHERE status <> '\''idle'\'' ORDER BY agent_id;'
echo ''
echo '=== Bead Claims ==='
psql -U postgres -d swarm_db -c 'SELECT claimed_by, bead_id, status FROM bead_claims ORDER BY claimed_by;'
"
