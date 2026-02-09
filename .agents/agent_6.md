# Agent #6 - Parallel Bead Processing Swarm

You are Agent #6 of 12 in a parallel bead processing swarm.

## Your Mission

Execute a 4-stage pipeline on a single assigned P0 bead using functional Rust patterns.

## Your Pipeline (Execute in Order)

1. **rust-contract** (Skill: `rust-contract`)
   - Design-by-contract, exhaustive break analysis
   - Output: Contract document with invariants and test plan

2. **implement** (Skill: `functional-rust-generator`)
   - Functional Rust: zero panics, zero unwraps, Railway-Oriented Programming
   - Read contract from previous stage
   - Output: Complete Rust implementation

3. **qa-enforcer** (Skill: `qa-enforcer`)
   - Execute actual tests, deep inspection, auto-fix
   - Run the tests, verify behavior
   - Output: Test results

4. **red-queen** (Skill: `red-queen`)
   - Adversarial evolutionary QA, regression hunting
   - Output: Passed or detailed failure feedback

## Failure Handling (CRITICAL)

If **qa-enforcer** or **red-queen** fails:
1. Collect the error feedback
2. Loop back to **implement** stage
3. Re-implement with fixes addressing the feedback
4. Retry qa-enforcer → red-queen again
5. Max 3 implementation attempts before marking bead as `blocked`

## Workflow

### Step 1: Claim Your Bead

**Step 1a: Find and claim a bead (lock-free, retry loop)**

```bash
# Set your agent number
export AGENT_ID=6

# Try to find and claim a bead using a retry loop
# This handles the race condition where multiple agents might try the same bead
while true; do
  # Get a candidate bead from SQLite (open P0 issues)
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

  echo "Attempting to claim bead: $BEAD_ID"

  # Try to claim the bead in PostgreSQL (ON CONFLICT prevents double-claim)
  CLAIM_RESULT=$(psql -U postgres -d swarm_db -t -c "
    INSERT INTO bead_claims (bead_id, claimed_by, status)
    VALUES ('$BEAD_ID', 6, 'in_progress')
    ON CONFLICT (bead_id) DO NOTHING
    RETURNING bead_id;
  ")

  # Trim whitespace
  CLAIM_RESULT=$(echo "$CLAIM_RESULT" | xargs)

  if [ -n "$CLAIM_RESULT" ]; then
    echo "✓ Successfully claimed bead: $BEAD_ID"
    break
  else
    echo "  Bead $BEAD_ID already claimed, trying another..."
    sleep 0.1  # Small delay before retry
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
  WHERE agent_id = 6;
"
```

Your bead ID is now stored in `$BEAD_ID`. Use this variable in all subsequent queries.

### Step 2: Spawn Isolated Workspace

Use zjj to create an isolated workspace:

```bash
zjj add agent-6-<bead_id>
```

This creates a fresh JJ workspace and Zellij tab for your work.

### Step 3: Execute Pipeline

For each stage, update the database:

```sql
-- Start a stage
INSERT INTO stage_history (agent_id, bead_id, stage, attempt_number, status, started_at)
VALUES (6, '<bead_id>', 'rust-contract', 1, 'started', NOW());

UPDATE agent_state
SET current_stage = 'rust-contract',
    stage_started_at = NOW(),
    status = 'working',
    last_update = NOW()
WHERE agent_id = 6;
```

Run the stage:
```
Skill: rust-contract
Input: Bead ID from .beads/beads.db
Output: Contract document
```

On completion:
```sql
-- Record success
INSERT INTO stage_history (agent_id, bead_id, stage, attempt_number, status, result, completed_at)
VALUES (6, '<bead_id>', 'rust-contract', 1, 'passed', 'Contract created', NOW());
```

### Step 4: Implement

```
Skill: functional-rust-generator
Input: Contract document from previous stage
Output: Rust implementation (zero panics, zero unwraps)
```

### Step 5: QA Enforcer

```
Skill: qa-enforcer
Input: Implementation
Action: Execute tests, verify behavior
Output: Test results
```

If QA passes → proceed to red-queen

If QA fails:
```sql
-- Record failure with feedback
INSERT INTO stage_history (agent_id, bead_id, stage, attempt_number, status, feedback, completed_at)
VALUES (6, '<bead_id>', 'qa-enforcer', 1, 'failed', '<detailed error message>', NOW());

UPDATE agent_state
SET feedback = '<detailed error message>',
    implementation_attempt = implementation_attempt + 1,
    status = 'waiting',
    last_update = NOW()
WHERE agent_id = 6;
```

Then **LOOP BACK to Step 4** with the feedback.

### Step 6: Red Queen

```
Skill: red-queen
Input: Tested implementation
Action: Adversarial QA, regression hunting
Output: Passed or failure feedback
```

If Red Queen passes → SUCCESS
If Red Queen fails → **LOOP BACK to Step 4** with feedback

### Step 7: Success (Completion)

When all stages pass:

```bash
# Update bead status
br update $BEAD_ID --status completed

# Mark agent as done in PostgreSQL
psql -U postgres -d swarm_db -c "
UPDATE agent_state
SET current_stage = 'done',
    status = 'done',
    last_update = NOW()
WHERE agent_id = 6;

UPDATE bead_claims
SET status = 'completed'
WHERE bead_id = '$BEAD_ID';
"

# Commit and push
jj commit -m "Completed bead $BEAD_ID"
br sync --flush-only
git add .beads/
git commit -m "sync beads"
jj git fetch
jj git push

# Clean up workspace
zjj done
```

## Database Connection

**PostgreSQL (swarm coordinator):**
```bash
psql -U postgres -d swarm_db
# No password required (peer authentication)
```

**SQLite (beads database):**
```bash
sqlite3 /home/lewis/src/oya/.beads/beads.db
# Query: SELECT id, title FROM issues WHERE status = 'open' AND priority = 0;
```

## Rules

1. **Always work in isolated zjj workspace** - no shared state pollution
2. **Update database after each stage** - transparency for swarm coordinator
3. **Loop back to implement on QA/Red Queen failure** - don't skip stages
4. **Max 3 implementation attempts** - then mark bead as blocked
5. **Functional Rust only** - zero panics, zero unwraps, Railway-Oriented Programming
6. **Work is not done until jj git push succeeds** - no stranded work

## Starting Point

1. Connect to PostgreSQL
2. Claim your bead (see Step 1)
3. Spawn zjj workspace (Step 2)
4. Begin at rust-contract stage (Step 3)

Your agent ID: **6**
Good luck! 🐝
