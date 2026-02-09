-- PostgreSQL Schema for Swarm Coordinator
-- 12 concurrent agents, zero lock contention

-- ============================================================
-- Table: bead_claims
-- Tracks which beads are claimed by which agents
-- ============================================================
CREATE TABLE IF NOT EXISTS bead_claims (
    bead_id TEXT PRIMARY KEY,
    claimed_by SMALLINT NOT NULL CHECK (claimed_by BETWEEN 1 AND 12),
    claimed_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    status TEXT NOT NULL DEFAULT 'in_progress' CHECK (status IN ('in_progress', 'completed', 'blocked'))
);

CREATE INDEX IF NOT EXISTS idx_bead_claims_agent ON bead_claims(claimed_by);
CREATE INDEX IF NOT EXISTS idx_bead_claims_status ON bead_claims(status);
CREATE INDEX IF NOT EXISTS idx_bead_claims_claimed_at ON bead_claims(claimed_at);

-- ============================================================
-- Table: agent_state
-- Current state of each agent (12 rows, one per agent)
-- ============================================================
CREATE TABLE IF NOT EXISTS agent_state (
    agent_id SMALLINT PRIMARY KEY CHECK (agent_id BETWEEN 1 AND 12),
    bead_id TEXT REFERENCES bead_claims(bead_id),
    current_stage TEXT CHECK (current_stage IN ('rust-contract', 'implement', 'qa-enforcer', 'red-queen', 'done')),
    stage_started_at TIMESTAMPTZ,
    status TEXT NOT NULL DEFAULT 'idle' CHECK (status IN ('idle', 'working', 'waiting', 'error', 'done')),
    last_update TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    implementation_attempt INTEGER NOT NULL DEFAULT 0,
    feedback TEXT
);

CREATE INDEX IF NOT EXISTS idx_agent_state_stage ON agent_state(current_stage);
CREATE INDEX IF NOT EXISTS idx_agent_state_status ON agent_state(status);
CREATE INDEX IF NOT EXISTS idx_agent_state_last_update ON agent_state(last_update DESC);

-- ============================================================
-- Table: stage_history
-- Complete audit log of all stage executions
-- ============================================================
CREATE TABLE IF NOT EXISTS stage_history (
    id BIGSERIAL PRIMARY KEY,
    agent_id SMALLINT NOT NULL CHECK (agent_id BETWEEN 1 AND 12),
    bead_id TEXT NOT NULL,
    stage TEXT NOT NULL CHECK (stage IN ('rust-contract', 'implement', 'qa-enforcer', 'red-queen')),
    attempt_number INTEGER NOT NULL,
    status TEXT NOT NULL CHECK (status IN ('started', 'passed', 'failed', 'error')),
    result TEXT,
    feedback TEXT,
    started_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    completed_at TIMESTAMPTZ,
    duration_ms INTEGER,
    FOREIGN KEY (agent_id) REFERENCES agent_state(agent_id),
    FOREIGN KEY (bead_id) REFERENCES bead_claims(bead_id)
);

CREATE INDEX IF NOT EXISTS idx_stage_history_agent ON stage_history(agent_id);
CREATE INDEX IF NOT EXISTS idx_stage_history_bead ON stage_history(bead_id);
CREATE INDEX IF NOT EXISTS idx_stage_history_stage ON stage_history(stage);
CREATE INDEX IF NOT EXISTS idx_stage_history_time ON stage_history(started_at DESC);

-- ============================================================
-- Table: pipeline_config
-- Configuration for swarm execution
-- ============================================================
CREATE TABLE IF NOT EXISTS pipeline_config (
    key TEXT PRIMARY KEY,
    value TEXT NOT NULL,
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

-- Insert default configuration
INSERT INTO pipeline_config (key, value) VALUES
    ('max_agents', '12'),
    ('max_implementation_attempts', '3'),
    ('claim_label', 'p0'),
    ('swarm_started_at', to_json(NOW())),
    ('swarm_status', 'initializing')
ON CONFLICT (key) DO NOTHING;

-- ============================================================
-- Functions: Common Operations
-- ============================================================

-- NOTE: Beads are stored in SQLite at .beads/beads.db
-- Agents query SQLite directly to find beads, then claim them here
-- This function returns beads that have been claimed in PostgreSQL
CREATE OR REPLACE FUNCTION get_claimed_beads()
RETURNS TABLE (bead_id TEXT, agent_id SMALLINT) AS $$
BEGIN
    RETURN QUERY
    SELECT bead_id, claimed_by
    FROM bead_claims
    WHERE status = 'in_progress';
END;
$$ LANGUAGE plpgsql;

-- Claim a bead (transactional)
CREATE OR REPLACE FUNCTION claim_bead(p_agent_id SMALLINT, p_bead_id TEXT)
RETURNS BOOLEAN AS $$
BEGIN
    -- Check if bead is already claimed
    IF EXISTS (SELECT 1 FROM bead_claims WHERE bead_id = p_bead_id AND status = 'in_progress') THEN
        RETURN FALSE;
    END IF;

    -- Claim the bead
    INSERT INTO bead_claims (bead_id, claimed_by, status)
    VALUES (p_bead_id, p_agent_id, 'in_progress');

    -- Update agent state
    UPDATE agent_state
    SET bead_id = p_bead_id,
        current_stage = 'rust-contract',
        stage_started_at = NOW(),
        status = 'working',
        last_update = NOW()
    WHERE agent_id = p_agent_id;

    RETURN TRUE;
END;
$$ LANGUAGE plpgsql;

-- ============================================================
-- Views: Common Queries
-- ============================================================

-- Active agents with their current beads
CREATE OR REPLACE VIEW v_active_agents AS
SELECT
    a.agent_id,
    a.bead_id,
    a.current_stage,
    a.status,
    a.implementation_attempt,
    b.claimed_at,
    EXTRACT(EPOCH FROM (NOW() - b.claimed_at)) * 1000 as time_elapsed_ms
FROM agent_state a
JOIN bead_claims b ON a.bead_id = b.bead_id
WHERE a.status IN ('working', 'waiting', 'error');

-- Swarm progress summary
CREATE OR REPLACE VIEW v_swarm_progress AS
SELECT
    COUNT(*) FILTER (WHERE status = 'done')::BIGINT as completed,
    COUNT(*) FILTER (WHERE status = 'working')::BIGINT as working,
    COUNT(*) FILTER (WHERE status = 'waiting')::BIGINT as waiting,
    COUNT(*) FILTER (WHERE status = 'error')::BIGINT as errors,
    COUNT(*) FILTER (WHERE status = 'idle')::BIGINT as idle,
    COUNT(*)::BIGINT as total_agents
FROM agent_state;

-- Latest failed stages requiring feedback
CREATE OR REPLACE VIEW v_feedback_required AS
SELECT DISTINCT ON (agent_id, bead_id)
    agent_id,
    bead_id,
    stage,
    attempt_number,
    feedback,
    completed_at
FROM stage_history
WHERE status = 'failed'
ORDER BY agent_id, bead_id, completed_at DESC;

-- NOTE: Beads are in SQLite at .beads/beads.db
-- Query available beads there:
-- sqlite3 .beads/beads.db "SELECT id, title FROM issues WHERE status = 'open' AND priority = 0 AND id NOT IN (SELECT bead_id FROM bead_claims WHERE status = 'in_progress') ORDER BY created_at ASC LIMIT 1;"

-- ============================================================
-- Triggers: Auto-update timestamps
-- ============================================================

-- Auto-update last_update on agent_state changes
CREATE OR REPLACE FUNCTION update_agent_last_update()
RETURNS TRIGGER AS $$
BEGIN
    NEW.last_update = NOW();
    RETURN NEW;
END;
$$ LANGUAGE plpgsql;

CREATE TRIGGER trg_agent_state_update
    BEFORE UPDATE ON agent_state
    FOR EACH ROW
    EXECUTE FUNCTION update_agent_last_update();

-- Auto-update config timestamp
CREATE OR REPLACE FUNCTION update_config_timestamp()
RETURNS TRIGGER AS $$
BEGIN
    NEW.updated_at = NOW();
    RETURN NEW;
END;
$$ LANGUAGE plpgsql;

CREATE TRIGGER trg_config_update
    BEFORE UPDATE ON pipeline_config
    FOR EACH ROW
    EXECUTE FUNCTION update_config_timestamp();
