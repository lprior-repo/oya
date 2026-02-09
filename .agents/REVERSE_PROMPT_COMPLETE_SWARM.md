# Reverse Prompt: 12-Agent Parallel Bead Processing Swarm

This document contains everything needed to recreate the 12-agent (or N-agent) parallel bead processing swarm system from scratch.

## System Overview

**Architecture**: N parallel agents processing P0 beads through a 4-stage functional Rust pipeline
**Coordinator**: PostgreSQL database (swarm_db) tracking all agent state
**Bead Storage**: SQLite at `.beads/beads.db` (issues table)
**Workspace Isolation**: zjj CLI for isolated JJ workspaces

```
┌─────────────────────────────────────────────────────────────┐
│                    PostgreSQL Database                       │
│                   (swarm_db - coordinator)                   │
│  ┌──────────────┐  ┌──────────────┐  ┌──────────────────┐  │
│  │ bead_claims  │  │ agent_state  │  │ stage_history    │  │
│  │              │  │ (N agents)   │  │ (audit log)      │  │
│  └──────────────┘  └──────────────┘  └──────────────────┘  │
└─────────────────────────────────────────────────────────────┘
          ▲                    ▲                   ▲
          │                    │                   │
          └────────────────────┴───────────────────┘
                               │
    ┌──────────┬──────────┬──────────┬─────────────┐
    │          │          │          │             │
┌───▼───┐ ┌───▼───┐ ┌───▼───┐ ┌────▼────┐   ┌────▼────┐
│Agent 1│ │Agent 2│ │Agent 3│ │Agent 4  │...│Agent N  │
└───┬───┘ └───┬───┘ └───┬───┘ └────┬────┘   └────┬────┘
    │         │         │            │             │
    └─────────┴─────────┴────────────┴─────────────┘
                         │
              ┌──────────▼──────────┐
              │   Each Agent Runs:  │
              │                     │
              │ 1. rust-contract    │
              │ 2. implement        │
              │ 3. qa-enforcer      │
              │ 4. red-queen        │
              │                     │
              │ Loop on failure:    │
              │ qa/red-queen →      │
              │ implement (retry)   │
              └─────────────────────┘
```

## File Structure

```
/home/lewis/src/oya/
├── .agents/
│   ├── agent_prompt.md              # Agent prompt template ({N} placeholder)
│   ├── init_postgres_swarm.sh       # Database initialization script
│   ├── launch_swarm.sh              # Generates agent prompts
│   ├── spawn_swarm.py               # Generates Task tool calls
│   ├── monitor.sh                   # Live monitoring script
│   ├── README.md                    # Full documentation
│   ├── SWARM_REVERSE_PROMPT.md      # This file
│   └── REVERSE_PROMPT_COMPLETE_SWARM.md
│
├── crates/
│   └── swarm-coordinator/
│       ├── Cargo.toml               # Coordinator crate (optional)
│       └── schema.sql               # PostgreSQL schema
│
└── .beads/
    └── beads.db                     # SQLite bead storage
```

## Quick Start (From Scratch)

### Step 1: Initialize PostgreSQL Database

```bash
# Start PostgreSQL (peer auth as 'postgres' user)
# System PostgreSQL or Docker:
psql -U postgres -d postgres -c "CREATE DATABASE swarm_db;"

# Load schema
psql -U postgres -d swarm_db < /home/lewis/src/oya/crates/swarm-coordinator/schema.sql

# Initialize N agents
for i in {1..100}; do
  psql -U postgres -d swarm_db -c "
    INSERT INTO agent_state (agent_id, status, last_update, implementation_attempt)
    VALUES ($i, 'idle', NOW(), 0)
    ON CONFLICT (agent_id) DO NOTHING;
  "
done

# Verify
psql -U postgres -d swarm_db -c "SELECT * FROM v_swarm_progress;"
```

### Step 2: Generate Agent Prompts

```bash
cd /home/lewis/src/oya

# Create prompts for agents 1-N
for i in {1..12}; do
  sed "s/{N}/$i/g" .agents/agent_prompt.md > .agents/agent_$i.md
done
```

### Step 3: Launch Agents via Task Tool

For each agent (1-N), use the Task tool:

```python
Task(
    description=f"Agent {i} process bead through pipeline",
    prompt=open(f".agents/agent_{i}.md").read(),
    subagent_type="general-purpose",
    run_in_background=True,
    max_turns=50
)
```

Or use the spawn script:
```bash
python3 .agents/spawn_swarm.py
```

### Step 4: Monitor Swarm

```bash
# Live monitoring (auto-refreshes every 2 seconds)
.agents/monitor.sh

# Or manual queries:
psql -U postgres -d swarm_db -c "SELECT * FROM v_swarm_progress;"
psql -U postgres -d swarm_db -c "SELECT * FROM v_active_agents;"
```

## PostgreSQL Schema

### Key Tables

```sql
-- Bead claims: Tracks which beads are claimed by which agents
CREATE TABLE bead_claims (
    bead_id TEXT PRIMARY KEY,
    claimed_by SMALLINT CHECK (claimed_by BETWEEN 1 AND 100),
    claimed_at TIMESTAMPTZ DEFAULT NOW(),
    status TEXT CHECK (status IN ('in_progress', 'completed', 'blocked'))
);

-- Agent state: Current state of each agent
CREATE TABLE agent_state (
    agent_id SMALLINT PRIMARY KEY CHECK (agent_id BETWEEN 1 AND 100),
    bead_id TEXT,
    current_stage TEXT CHECK (current_stage IN ('rust-contract', 'implement', 'qa-enforcer', 'red-queen', 'done')),
    stage_started_at TIMESTAMPTZ,
    status TEXT CHECK (status IN ('idle', 'working', 'waiting', 'error', 'done')),
    last_update TIMESTAMPTZ DEFAULT NOW(),
    implementation_attempt INTEGER DEFAULT 0,
    feedback TEXT
);

-- Stage history: Audit log of all stage executions
CREATE TABLE stage_history (
    id BIGSERIAL PRIMARY KEY,
    agent_id SMALLINT CHECK (agent_id BETWEEN 1 AND 100),
    bead_id TEXT NOT NULL,
    stage TEXT CHECK (stage IN ('rust-contract', 'implement', 'qa-enforcer', 'red-queen')),
    attempt_number INTEGER NOT NULL,
    status TEXT CHECK (status IN ('started', 'passed', 'failed', 'error')),
    result TEXT,
    feedback TEXT,
    started_at TIMESTAMPTZ DEFAULT NOW(),
    completed_at TIMESTAMPTZ,
    duration_ms INTEGER
);
```

### Key Views

```sql
-- Active agents
CREATE VIEW v_active_agents AS
SELECT a.agent_id, a.bead_id, a.current_stage, a.status, a.implementation_attempt
FROM agent_state a
WHERE a.status IN ('working', 'waiting', 'error');

-- Progress summary
CREATE VIEW v_swarm_progress AS
SELECT
    COUNT(*) FILTER (WHERE status = 'done')::BIGINT as completed,
    COUNT(*) FILTER (WHERE status = 'working')::BIGINT as working,
    COUNT(*) FILTER (WHERE status = 'waiting')::BIGINT as waiting,
    COUNT(*) FILTER (WHERE status = 'error')::BIGINT as errors,
    COUNT(*) FILTER (WHERE status = 'idle')::BIGINT as idle,
    COUNT(*)::BIGINT as total_agents
FROM agent_state;

-- Failed stages requiring feedback
CREATE VIEW v_feedback_required AS
SELECT DISTINCT ON (agent_id, bead_id)
    agent_id, bead_id, stage, attempt_number, feedback, completed_at
FROM stage_history
WHERE status = 'failed'
ORDER BY agent_id, bead_id, completed_at DESC;
```

## Agent Prompt Template

File: `.agents/agent_prompt.md`

```markdown
# Agent #{N} - Parallel Bead Processing Swarm

You are Agent #{N} of N in a parallel bead processing swarm.

## Your Mission

Execute a 4-stage pipeline on a single assigned P0 bead using functional Rust patterns.

## Your Pipeline (Execute in Order)

1. **rust-contract** (Skill: `rust-contract`)
   - Design-by-contract, exhaustive break analysis
   - Output: Contract document with invariants and test plan

2. **implement** (Skill: `functional-rust-generator`)
   - Functional Rust: zero panics, zero unwraps, Railway-Oriented Programming
   - Output: Complete Rust implementation

3. **qa-enforcer** (Skill: `qa-enforcer`)
   - Execute actual tests, deep inspection, auto-fix
   - Output: Test results

4. **red-queen** (Skill: `red-queen`)
   - Adversarial evolutionary QA, regression hunting
   - Output: Passed or detailed failure feedback

## Failure Handling

If **qa-enforcer** or **red-queen** fails:
1. Collect the error feedback
2. Loop back to **implement** stage
3. Re-implement with fixes addressing the feedback
4. Retry qa-enforcer → red-queen again
5. Max 3 implementation attempts before marking bead as `blocked`

## Workflow

### Step 1: Claim Your Bead

```bash
export AGENT_ID={N}

while true; do
  BEAD_ID=$(sqlite3 /home/lewis/src/oya/.beads/beads.db "
    SELECT id FROM issues
    WHERE status = 'open' AND priority = 0
    ORDER BY created_at ASC
    LIMIT 1 OFFSET $((RANDOM % 10));
  ")

  if [ -z "$BEAD_ID" ]; then
    echo "No beads available. Exiting."
    exit 0
  fi

  CLAIM_RESULT=$(psql -U postgres -d swarm_db -t -c "
    INSERT INTO bead_claims (bead_id, claimed_by, status)
    VALUES ('$BEAD_ID', {N}, 'in_progress')
    ON CONFLICT (bead_id) DO NOTHING
    RETURNING bead_id;
  " | xargs)

  if [ -n "$CLAIM_RESULT" ]; then
    echo "✓ Claimed bead: $BEAD_ID"
    break
  else
    echo "  Already claimed, trying another..."
    sleep 0.1
  fi
done

# Update agent state
psql -U postgres -d swarm_db -c "
UPDATE agent_state
SET bead_id = '$BEAD_ID',
    current_stage = 'rust-contract',
    stage_started_at = NOW(),
    status = 'working',
    last_update = NOW()
WHERE agent_id = {N};
"
```

### Step 2: Spawn Workspace

```bash
zjj add agent-{N}-$BEAD_ID
```

### Step 3-6: Execute Pipeline Stages

For each stage:
1. Insert `stage_history` record with status='started'
2. Execute the appropriate Skill tool
3. Update `stage_history` with result (passed/failed)
4. If failed: update `agent_state.feedback`, increment `implementation_attempt`, loop back to Step 4

### Step 7: Success

```bash
br update $BEAD_ID --status completed

psql -U postgres -d swarm_db -c "
UPDATE agent_state SET current_stage = 'done', status = 'done', last_update = NOW() WHERE agent_id = {N};
UPDATE bead_claims SET status = 'completed' WHERE bead_id = '$BEAD_ID';
"

jj commit -m "Completed bead $BEAD_ID"
br sync --flush-only
git add .beads/
git commit -m "sync beads"
jj git fetch
jj git push

zjj done
```

## Database Connection

**PostgreSQL (swarm coordinator):**
```bash
psql -U postgres -d swarm_db
# No password (peer auth)
```

**SQLite (beads database):**
```bash
sqlite3 /home/lewis/src/oya/.beads/beads.db
# Query: SELECT id, title FROM issues WHERE status = 'open' AND priority = 0;
```

## Rules

1. **Always work in isolated zjj workspace**
2. **Update database after each stage**
3. **Loop back to implement on QA/Red Queen failure**
4. **Max 3 implementation attempts**
5. **Functional Rust only** (zero panics, zero unwraps)
6. **Work is not done until jj git push succeeds**

Your agent ID: **{N}**
```

## Monitoring Scripts

### Live Monitor

File: `.agents/monitor.sh`

```bash
#!/usr/bin/env bash
watch -n 2 -c "
echo '=== Swarm Progress ==='
psql -U postgres -d swarm_db -c 'SELECT * FROM v_swarm_progress;'
echo ''
echo '=== Active Agents ==='
psql -U postgres -d swarm_db -c 'SELECT agent_id, bead_id, current_stage FROM agent_state WHERE status <> '\''idle'\'' ORDER BY agent_id;'
"
```

### Progress Query

```bash
psql -U postgres -d swarm_db -c "
SELECT
    sh.agent_id,
    COUNT(*) FILTER (WHERE sh.status = 'passed') as stages_passed,
    a.current_stage,
    a.status
FROM agent_state a
LEFT JOIN stage_history sh ON a.agent_id = sh.agent_id
GROUP BY a.agent_id, a.current_stage, a.status
ORDER BY a.agent_id;
"
```

## Configuration

### Environment Variables

```bash
export SWARM_DB=swarm_db
export SWARM_USER=postgres
export SWARM_HOST=localhost
export SWARM_PORT=5432
```

### Pipeline Configuration

```sql
INSERT INTO pipeline_config (key, value) VALUES
    ('max_agents', '100'),
    ('max_implementation_attempts', '3'),
    ('claim_label', 'p0'),
    ('swarm_started_at', NOW()),
    ('swarm_status', 'running');
```

## Bead Storage (SQLite)

### Issues Table Schema

```sql
CREATE TABLE issues (
    id TEXT PRIMARY KEY,
    title TEXT NOT NULL,
    description TEXT DEFAULT '',
    status TEXT DEFAULT 'open',
    priority INTEGER DEFAULT 2,  -- 0 = P0
    -- ... other fields
);

-- Query available P0 beads
SELECT id, title FROM issues
WHERE status = 'open' AND priority = 0
ORDER BY created_at ASC;
```

## Troubleshooting

### Database Connection Failed

```bash
# Check PostgreSQL is running
pg_isready -h localhost -p 5432

# Check database exists
psql -U postgres -d postgres -c "\l" | grep swarm_db
```

### No Beads Available

```bash
# Check beads exist
sqlite3 .beads/beads.db "SELECT COUNT(*) FROM issues WHERE status = 'open' AND priority = 0;"

# Add test beads if needed
br new --slug test-bead-{1..12} --priority p0
```

### Agent Stuck in Error State

```sql
-- Reset agent to idle
UPDATE agent_state
SET status = 'idle',
    bead_id = NULL,
    current_stage = NULL,
    feedback = NULL,
    implementation_attempt = 0,
    last_update = NOW()
WHERE agent_id = <agent_id>;
```

### Scale to More Agents

```sql
-- Update constraints to support N agents
ALTER TABLE agent_state DROP CONSTRAINT agent_state_agent_id_check;
ALTER TABLE agent_state ADD CONSTRAINT agent_state_agent_id_check
  CHECK (agent_id BETWEEN 1 AND 200);

-- Repeat for stage_history and bead_claims

-- Insert new agents
INSERT INTO agent_state (agent_id, status, last_update, implementation_attempt)
SELECT i, 'idle', NOW(), 0
FROM generate_series(13, 200) AS i
ON CONFLICT (agent_id) DO NOTHING;
```

## Summary

This system provides:
- **Parallel bead processing**: N agents working simultaneously
- **Centralized coordination**: PostgreSQL tracking all state
- **Isolated workspaces**: zjj prevents conflicts
- **Feedback loops**: QA failures trigger re-implementation
- **Full audit trail**: Every stage execution logged
- **Scalability**: Easy to add more agents

**Key Files:**
- `.agents/agent_prompt.md` - Agent template
- `crates/swarm-coordinator/schema.sql` - Database schema
- `.agents/monitor.sh` - Live monitoring

**Key Commands:**
- `.agents/init_postgres_swarm.sh` - Initialize database
- `.agents/monitor.sh` - Monitor swarm
- Spawn via Task tool with `run_in_background=True`

**Database:**
- Host: localhost
- Port: 5432
- Database: swarm_db
- User: postgres (peer auth)
