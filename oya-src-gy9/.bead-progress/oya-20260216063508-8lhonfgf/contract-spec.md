# Contract Specification

## Context

**Feature:** Prototype Sled persistence for run-attempt-artifact replayability

**Domain Terms:**
- **Run**: Single execution of a Bead through the pipeline (aggregate root)
- **StageAttempt**: One try at passing a Stage (max 3 per Stage)
- **Artifact**: Value object - output from Stages (contracts, code, tests, reports)
- **StageResult**: Outcome of a StageAttempt (passed/failed, gate results, failure categories)
- **Replayability**: Ability to restore Run state from durable storage after restart

**Assumptions:**
1. Sled database is already initialized (see existing `OyaDb` in `src/persistence.rs`)
2. RunId, BeadId, StageName are existing domain types from `src/orchestration.rs`
3. Existing async `OyaDb` API will be supplemented with sync event sourcing primitives
4. Moon gates (`:ci`, `:test`) are available for verification
5. ULID-based IDs guarantee uniqueness without coordination

**Open Questions:**
None - domain types and persistence patterns are well-established in the codebase.

---

## Preconditions

### For Run Persistence
1. Sled database must be open and writable
2. RunId must be a valid ULID string (non-empty, well-formed)
3. BeadId must reference an existing bead
4. Run state must be valid (no invariants violated)

### For StageAttempt Persistence
1. RunId must already exist in `bead_runs` tree
2. StageName must be one of the canonical stages (Contract..ShipGate)
3. Attempt number must be <= StageName::max_attempts() (3)
4. Timestamps must be valid ISO 8601 strings

### For Artifact Persistence
1. RunId must reference an existing run
2. Stage must be a valid stage name
3. Artifact type must be from the ArtifactType enum
4. Location must be a valid file path or URL

### For Replayability (Restore)
1. Sled database must contain records for the requested RunId
2. Serialized data must be deserializable to domain types
3. No partial writes (Sled ACID properties guarantee atomicity)

---

## Postconditions

### For Run Persistence
1. Run is stored in `bead_runs` tree with key = run_id
2. Run state, created_at, updated_at are serialized to JSON
3. Write is flushed to disk (Sled durability)
4. Returns `Ok(())` on success

### For StageAttempt Persistence
1. Attempt is stored in `stage_attempts` tree with key = "{run_id}:{stage}:{attempt:03}"
2. All attempt fields (session_id, timestamps, state) are persisted
3. Returns `Ok(())` on success

### For Artifact Persistence
1. Artifact is stored in `artifacts` tree with key = "{run_id}:{artifact_id}"
2. Checksum (if present) is validated before storage
3. Returns `Ok(())` on success

### For Replayability (Restore)
1. Returns `Ok(Run)` with complete reconstructed state
2. Run.history contains all StageAttempts for the run
3. Attempt order is preserved (by attempt number)
4. Artifacts are retrievable by type and stage
5. Returns `Err(OyaDbError::NotFound)` if run does not exist

---

## Invariants

### State Consistency
1. **Run Lifecycle Invariant**: A Run exists in exactly one of [Pending, Running, Waiting, Shipped, Failed, Aborted] states
2. **Attempt Monotonicity**: StageAttempts for a given (run_id, stage) have strictly increasing attempt numbers (1, 2, 3)
3. **Timestamp Ordering**: For any StageAttempt, started_at <= completed_at (if completed)
4. **History Integrity**: Run.history contains at most N attempts per stage where N = max_attempts()

### Referential Integrity
1. **All StageAttempts reference existing RunId**: No orphan attempts
2. **All Artifacts reference existing RunId**: No orphan artifacts
3. **All GateResults reference existing (run_id, gate_name)**: No dangling gate results

### Idempotency
1. **Insert is idempotent**: Inserting the same (run_id, stage, attempt) twice returns Ok but does not duplicate
2. **Update is idempotent**: Updating the same run state multiple times converges to the final state

### Durability
1. **Flush guarantee**: All successful writes are flushed to disk before returning Ok
2. **Atomicity**: Either all fields of a record are written, or none are (Sled ACID)

---

## Error Taxonomy

```rust
pub enum OyaDbError {
    /// Sled database error (disk full, permissions, corruption)
    Database(sled::Error),

    /// Requested record not found (run_id, artifact_id, etc.)
    NotFound(String),

    /// Serialization/deserialization failure (invalid JSON, bincode error)
    Serialization(String),

    /// I/O error (file system, network)
    Io(std::io::Error),

    /// Precondition violation: attempting to create attempt for non-existent run
    RunNotFound(String),

    /// Precondition violation: attempt number exceeds max_attempts() for stage
    AttemptLimitExceeded { stage: String, attempt: u32, max: u32 },

    /// Invariant violation: invalid state transition
    InvalidStateTransition { from: String, to: String },

    /// Contract violation: timestamp ordering (started_at > completed_at)
    InvalidTimestampOrder,

    /// Referential integrity: artifact references non-existent run
    OrphanedArtifact { artifact_id: String, run_id: String },
}
```

---

## Contract Signatures

### Core Persistence API

```rust
/// Persist a new Run to durable storage
///
/// Preconditions:
/// - run.id must be unique ULID
/// - run.bead_id must reference existing bead
/// - run.state must be valid initial state (Pending)
///
/// Postconditions:
/// - Run is stored in 'bead_runs' tree
/// - Write is flushed to disk
///
/// Errors:
/// - Database: Sled error
/// - Serialization: JSON encode failure
pub fn insert_run(&self, run: &Run) -> Result<(), OyaDbError>

/// Retrieve a Run by ID, reconstructing full state
///
/// Preconditions:
/// - run_id must exist in 'bead_runs' tree
///
/// Postconditions:
/// - Returns Ok(Run) with populated history Vec<StageAttempt>
/// - All attempts are ordered by attempt number
///
/// Errors:
/// - NotFound: run_id does not exist
/// - Serialization: JSON decode failure
pub fn get_run(&self, run_id: &RunId) -> Result<Run, OyaDbError>

/// Persist a StageAttempt
///
/// Preconditions:
/// - run_id must exist in 'bead_runs' tree
/// - attempt <= max_attempts() for the stage
/// - started_at <= completed_at (if completed)
///
/// Postconditions:
/// - Attempt stored in 'stage_attempts' tree
/// - Idempotent: duplicate insert does not error
///
/// Errors:
/// - RunNotFound: run_id does not exist
/// - AttemptLimitExceeded: attempt > max_attempts()
/// - InvalidTimestampOrder: completed_at < started_at
pub fn insert_stage_attempt(&self, attempt: &StageAttempt) -> Result<(), OyaDbError>

/// Retrieve all StageAttempts for a Run
///
/// Preconditions:
/// - run_id must exist
///
/// Postconditions:
/// - Returns Vec<StageAttempt> ordered by (stage, attempt)
/// - Empty Vec if no attempts exist
pub fn get_stage_attempts(&self, run_id: &RunId) -> Result<Vec<StageAttempt>, OyaDbError>

/// Persist an Artifact
///
/// Preconditions:
/// - run_id must exist
/// - artifact_type must be valid ArtifactType
/// - location must be valid path/URL
///
/// Postconditions:
/// - Artifact stored in 'artifacts' tree
/// - Checksum validated if present
///
/// Errors:
/// - RunNotFound: run_id does not exist
/// - Serialization: encoding failure
pub fn insert_artifact(&self, artifact: &Artifact) -> Result<(), OyaDbError>

/// Retrieve all Artifacts for a Run, optionally filtered by stage
///
/// Preconditions:
/// - run_id must exist
///
/// Postconditions:
/// - Returns Vec<Artifact> for the run
/// - Filtered by stage if stage_name is Some
pub fn get_artifacts(&self, run_id: &RunId, stage_name: Option<StageName>) -> Result<Vec<Artifact>, OyaDbError>

/// Update Run state (state transition)
///
/// Preconditions:
/// - run_id must exist
/// - Transition must be valid (state machine rules)
///
/// Postconditions:
/// - run.state updated in 'bead_runs'
/// - updated_at set to Utc::now()
/// - Flush to disk
///
/// Errors:
/// - NotFound: run_id does not exist
/// - InvalidStateTransition: transition not allowed
pub fn update_run_state(&self, run_id: &RunId, new_state: &RunState) -> Result<(), OyaDbError>
```

---

## Non-goals

1. **Query APIs**: No complex queries or indexes in this prototype (future work)
2. **Migrations**: No schema migration support (prototype only)
3. **Replication**: No multi-node consistency (single-node Sled)
4. **Backup/Restore**: No snapshot backup (manual file copy for now)
5. **Metrics**: No performance telemetry in prototype (add later)
6. **TTL/Expiration**: No automatic cleanup of old records (manual GC only)

---

## Verification Checklist

- [ ] Every error variant is used at least once in error paths
- [ ] All preconditions have corresponding validation
- [ ] All postconditions are asserted in tests
- [ ] All invariants have invariant-violation tests
- [ ] Idempotency is verified with duplicate insert tests
- [ ] Replayability tested with restart scenarios
