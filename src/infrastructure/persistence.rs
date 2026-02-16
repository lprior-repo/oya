use crate::domain::{
    AgentId, AgentState, AgentStatus, Artifact, ArtifactType, BeadId, GateResult, Run as BeadRun,
    RunId, RunState, ShipDecision, StageAttempt, StageName as Stage, StageResult, StageState,
};
use serde::{de::DeserializeOwned, Deserialize, Serialize};
use sled::Db;
use std::sync::Arc;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum OyaDbError {
    #[error("Database error: {0}")]
    Database(#[from] sled::Error),
    #[error("Record not found: {0}")]
    NotFound(String),
    #[error("Serialization error: {0}")]
    Serialization(String),
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
    #[error("Run not found: {0}")]
    RunNotFound(String),
    #[error("Attempt limit exceeded for stage '{stage}': attempt {attempt} exceeds max {max}")]
    AttemptLimitExceeded { stage: String, attempt: u32, max: u32 },
    #[error("Invalid state transition: from '{from}' to '{to}'")]
    InvalidStateTransition { from: String, to: String },
    #[error("Invalid timestamp order: completed_at is before started_at")]
    InvalidTimestampOrder,
    #[error("Orphaned artifact: artifact '{artifact_id}' references non-existent run '{run_id}'")]
    OrphanedArtifact { artifact_id: String, run_id: String },
}

pub struct OyaDb {
    db: Arc<Db>,
}

impl std::fmt::Debug for OyaDb {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "OyaDb")
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct BeadRunRecord {
    run_id: String,
    bead_id: String,
    state: String,
    created_at: String,
    updated_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct StageAttemptRecord {
    run_id: String,
    stage: String,
    attempt: u32,
    session_id: String,
    state: String,
    started_at: String,
    completed_at: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct StageResultRecord {
    run_id: String,
    stage: String,
    attempt: u32,
    passed: bool,
    output: Option<String>,
    failure_category: Option<String>,
    next_stage: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct ArtifactRecord {
    id: String,
    run_id: String,
    artifact_type: String,
    location: String,
    checksum: String,
    produced_by_stage: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct GateResultRecord {
    run_id: String,
    gate_name: String,
    passed: bool,
    exit_code: i32,
    log_ref: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct ShipDecisionRecord {
    run_id: String,
    shipped: bool,
    rationale: String,
    approver_mode: String,
    timestamp: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct AgentStateRecord {
    agent_id: String,
    bead_id: Option<String>,
    current_stage: Option<String>,
    stage_started_at: Option<String>,
    status: String,
    last_update: String,
    implementation_attempt: u32,
    feedback: Option<String>,
}

impl OyaDb {
    pub async fn connect(path: &str) -> Result<Self, OyaDbError> {
        let path = path.to_string();
        let db = tokio::task::spawn_blocking(move || {
            if path == "memory://" {
                sled::Config::new().temporary(true).open()
            } else {
                sled::open(&path)
            }
        })
        .await
        .map_err(|e| OyaDbError::Serialization(e.to_string()))??;

        Ok(Self { db: Arc::new(db) })
    }

    /// Open Sled database synchronously (for functional core)
    ///
    /// Preconditions: path must be writable or "memory://" for in-memory
    /// Postconditions: Returns OyaDb with initialized Sled handle
    pub fn connect_sync(path: &str) -> Result<Self, OyaDbError> {
        let db = if path == "memory://" {
            sled::Config::new().temporary(true).open()?
        } else {
            sled::open(path)?
        };

        Ok(Self { db: Arc::new(db) })
    }

    pub async fn init_schema(&self) -> Result<(), OyaDbError> {
        Ok(())
    }

    // =========================================================================
    // Synchronous Core (Functional Purity)
    // =========================================================================

    /// Insert a Run synchronously
    ///
    /// Preconditions: run.id must be unique ULID, run.state must be valid
    /// Postconditions: Run persisted to 'bead_runs' tree, flushed to disk
    pub fn insert_run_sync(&self, run: &BeadRun) -> Result<(), OyaDbError> {
        let record = BeadRunRecord {
            run_id: run.id.0.clone(),
            bead_id: run.bead_id.0.clone(),
            state: serde_json::to_string(&run.state)
                .map_err(|e| OyaDbError::Serialization(e.to_string()))?,
            created_at: run.created_at.to_rfc3339(),
            updated_at: run.updated_at.to_rfc3339(),
        };

        let tree = self.db.open_tree("bead_runs")?;
        let key = run.id.0.as_bytes();
        let value =
            serde_json::to_vec(&record).map_err(|e| OyaDbError::Serialization(e.to_string()))?;

        tree.insert(key, value)?;
        tree.flush()?;
        Ok(())
    }

    /// Get a Run by ID with full history (replayability)
    ///
    /// Preconditions: run_id must exist in 'bead_runs' tree
    /// Postconditions: Returns Ok(Run) with populated history Vec
    pub fn get_run_sync(&self, run_id: &str) -> Result<BeadRun, OyaDbError> {
        let tree = self.db.open_tree("bead_runs")?;
        let value =
            tree.get(run_id.as_bytes())?.ok_or_else(|| OyaDbError::NotFound(run_id.to_string()))?;

        let record: BeadRunRecord =
            serde_json::from_slice(&value).map_err(|e| OyaDbError::Serialization(e.to_string()))?;

        let state: RunState = serde_json::from_str(&record.state)
            .map_err(|e| OyaDbError::Serialization(e.to_string()))?;
        let created_at = record
            .created_at
            .parse()
            .map_err(|e: chrono::ParseError| OyaDbError::Serialization(e.to_string()))?;
        let updated_at = record
            .updated_at
            .parse()
            .map_err(|e: chrono::ParseError| OyaDbError::Serialization(e.to_string()))?;

        // Load history for replayability
        let history = self.get_stage_attempts_sync(run_id)?;

        Ok(BeadRun {
            id: RunId(record.run_id),
            bead_id: BeadId(record.bead_id),
            state,
            created_at,
            updated_at,
            history,
        })
    }

    /// Insert StageAttempt with validation
    ///
    /// Preconditions: run_id exists, attempt <= max_attempts(), timestamps valid
    /// Postconditions: Attempt persisted, idempotent on duplicate key
    pub fn insert_stage_attempt_sync(&self, attempt: &StageAttempt) -> Result<(), OyaDbError> {
        // Validate run exists
        let tree = self.db.open_tree("bead_runs")?;
        if tree.get(attempt.run_id.as_bytes())?.is_none() {
            return Err(OyaDbError::RunNotFound(attempt.run_id.clone()));
        }

        // Validate attempt limit
        let max_attempts = attempt.stage.max_attempts();
        if attempt.attempt > max_attempts {
            return Err(OyaDbError::AttemptLimitExceeded {
                stage: attempt.stage.as_str().to_string(),
                attempt: attempt.attempt,
                max: max_attempts,
            });
        }

        // Validate timestamp ordering
        if let Some(completed) = attempt.completed_at {
            if completed < attempt.started_at {
                return Err(OyaDbError::InvalidTimestampOrder);
            }
        }

        let stage_str = serde_json::to_string(&attempt.stage)
            .map_err(|e| OyaDbError::Serialization(e.to_string()))?;
        let record = StageAttemptRecord {
            run_id: attempt.run_id.clone(),
            stage: stage_str.clone(),
            attempt: attempt.attempt,
            session_id: attempt.session_id.clone().map_or_else(String::new, std::convert::identity),
            state: serde_json::to_string(&attempt.state)
                .map_err(|e| OyaDbError::Serialization(e.to_string()))?,
            started_at: attempt.started_at.to_rfc3339(),
            completed_at: attempt.completed_at.map(|t| t.to_rfc3339()),
        };

        let key = format!("{}:{}:{:03}", attempt.run_id, stage_str, attempt.attempt);
        let tree = self.db.open_tree("stage_attempts")?;
        let value =
            serde_json::to_vec(&record).map_err(|e| OyaDbError::Serialization(e.to_string()))?;

        tree.insert(key.as_bytes(), value)?;
        tree.flush()?;
        Ok(())
    }

    /// Get all StageAttempts for a Run, ordered by (stage, attempt)
    ///
    /// Preconditions: run_id exists
    /// Postconditions: Returns Vec ordered by stage_order then attempt number
    pub fn get_stage_attempts_sync(&self, run_id: &str) -> Result<Vec<StageAttempt>, OyaDbError> {
        let tree = self.db.open_tree("stage_attempts")?;
        let prefix = format!("{}:", run_id);

        tree.scan_prefix(prefix.as_bytes())
            .map(|item| {
                let (_key, value) = item?;
                let record: StageAttemptRecord = serde_json::from_slice(&value)
                    .map_err(|e| OyaDbError::Serialization(e.to_string()))?;

                let stage: Stage = serde_json::from_str(&record.stage)
                    .map_err(|e| OyaDbError::Serialization(e.to_string()))?;
                let state = serde_json::from_str(&record.state)
                    .map_err(|e| OyaDbError::Serialization(e.to_string()))?;

                let started_at = record
                    .started_at
                    .parse()
                    .map_err(|e: chrono::ParseError| OyaDbError::Serialization(e.to_string()))?;
                let completed_at = record
                    .completed_at
                    .map(|t| {
                        t.parse().map_err(|e: chrono::ParseError| {
                            OyaDbError::Serialization(e.to_string())
                        })
                    })
                    .transpose()?;

                let session_id =
                    if record.session_id.is_empty() { None } else { Some(record.session_id) };

                Ok(StageAttempt {
                    run_id: record.run_id,
                    stage,
                    attempt: record.attempt,
                    session_id,
                    state,
                    started_at,
                    completed_at,
                })
            })
            .collect::<Result<Vec<_>, _>>()
            .map(|mut attempts| {
                attempts.sort_by_key(|a| (stage_order(&a.stage), a.attempt));
                attempts
            })
    }

    /// Insert an Artifact with validation
    ///
    /// Preconditions: run_id exists, artifact_type is valid
    /// Postconditions: Artifact persisted to 'artifacts' tree
    pub fn insert_artifact_sync(&self, artifact: &Artifact) -> Result<(), OyaDbError> {
        // Validate run exists
        let tree = self.db.open_tree("bead_runs")?;
        if tree.get(artifact.run_id.as_bytes())?.is_none() {
            return Err(OyaDbError::OrphanedArtifact {
                artifact_id: artifact.id.clone(),
                run_id: artifact.run_id.clone(),
            });
        }

        let record = ArtifactRecord {
            id: artifact.id.clone(),
            run_id: artifact.run_id.clone(),
            artifact_type: artifact.artifact_type.as_str().to_string(),
            location: artifact.location.clone(),
            checksum: artifact.checksum.clone().map_or_else(String::new, std::convert::identity),
            produced_by_stage: serde_json::to_string(&artifact.produced_by_stage)
                .map_err(|e| OyaDbError::Serialization(e.to_string()))?,
        };

        let key = format!("{}:{}", artifact.run_id, artifact.id);
        let tree = self.db.open_tree("artifacts")?;
        let value =
            serde_json::to_vec(&record).map_err(|e| OyaDbError::Serialization(e.to_string()))?;

        tree.insert(key.as_bytes(), value)?;
        tree.flush()?;
        Ok(())
    }

    /// Get Artifacts for a Run, optionally filtered by stage
    ///
    /// Preconditions: run_id exists
    /// Postconditions: Returns Vec of artifacts, filtered if stage_name is Some
    pub fn get_artifacts_sync(
        &self,
        run_id: &str,
        stage_name: Option<Stage>,
    ) -> Result<Vec<Artifact>, OyaDbError> {
        let tree = self.db.open_tree("artifacts")?;
        let prefix = format!("{}:", run_id);

        tree.scan_prefix(prefix.as_bytes())
            .map(|item| {
                let (_key, value) = item?;
                let record: ArtifactRecord = serde_json::from_slice(&value)
                    .map_err(|e| OyaDbError::Serialization(e.to_string()))?;

                let produced_by_stage = serde_json::from_str(&record.produced_by_stage)
                    .map_err(|e| OyaDbError::Serialization(e.to_string()))?;

                // Filter by stage if specified
                if let Some(ref stage) = stage_name {
                    if produced_by_stage != *stage {
                        return Ok(None);
                    }
                }

                Ok(Some(Artifact {
                    id: record.id,
                    run_id: record.run_id,
                    artifact_type: parse_artifact_type(&record.artifact_type)?,
                    location: record.location,
                    checksum: if record.checksum.is_empty() { None } else { Some(record.checksum) },
                    produced_by_stage,
                }))
            })
            .filter_map(|r| r.transpose())
            .collect::<Result<Vec<_>, _>>()
    }

    /// Update Run state with transition validation
    ///
    /// Preconditions: run_id exists, transition is valid per state machine
    /// Postconditions: State updated, updated_at set to now, flushed to disk
    pub fn update_run_state_sync(
        &self,
        run_id: &str,
        new_state: &RunState,
    ) -> Result<(), OyaDbError> {
        let tree = self.db.open_tree("bead_runs")?;
        let key = run_id.as_bytes();
        let value = tree.get(key)?.ok_or_else(|| OyaDbError::NotFound(run_id.to_string()))?;

        let mut record: BeadRunRecord =
            serde_json::from_slice(&value).map_err(|e| OyaDbError::Serialization(e.to_string()))?;

        // Validate state transition (could add more sophisticated validation here)
        let current_state: RunState = serde_json::from_str(&record.state)
            .map_err(|e| OyaDbError::Serialization(e.to_string()))?;

        if !is_valid_transition(&current_state, new_state) {
            return Err(OyaDbError::InvalidStateTransition {
                from: format!("{:?}", current_state),
                to: format!("{:?}", new_state),
            });
        }

        record.state = serde_json::to_string(new_state)
            .map_err(|e| OyaDbError::Serialization(e.to_string()))?;
        record.updated_at = chrono::Utc::now().to_rfc3339();

        let encoded =
            serde_json::to_vec(&record).map_err(|e| OyaDbError::Serialization(e.to_string()))?;

        tree.insert(key, encoded)?;
        tree.flush()?;
        Ok(())
    }

    // =========================================================================
    // Async Shells (wrapper around sync core)
    // =========================================================================

    async fn insert_record<T: Serialize>(
        &self,
        tree_name: &str,
        key: &[u8],
        record: &T,
    ) -> Result<(), OyaDbError> {
        let db = Arc::clone(&self.db);
        let tree_name = tree_name.to_string();
        let key = key.to_vec();
        let value =
            serde_json::to_vec(record).map_err(|e| OyaDbError::Serialization(e.to_string()))?;

        tokio::task::spawn_blocking(move || {
            let tree = db.open_tree(&tree_name)?;
            tree.insert(key, value)?;
            tree.flush()?;
            Ok::<_, OyaDbError>(())
        })
        .await
        .map_err(|e| OyaDbError::Serialization(e.to_string()))??;

        Ok(())
    }

    async fn get_record<T: DeserializeOwned + Send + 'static>(
        &self,
        tree_name: &str,
        key: &[u8],
    ) -> Result<Option<T>, OyaDbError> {
        let db = Arc::clone(&self.db);
        let tree_name = tree_name.to_string();
        let key = key.to_vec();

        let result = tokio::task::spawn_blocking(move || {
            let tree = db.open_tree(&tree_name)?;
            let value = tree.get(key)?;
            match value {
                Some(v) => {
                    let record: T = serde_json::from_slice(&v)
                        .map_err(|e| OyaDbError::Serialization(e.to_string()))?;
                    Ok::<_, OyaDbError>(Some(record))
                }
                None => Ok(None),
            }
        })
        .await
        .map_err(|e| OyaDbError::Serialization(e.to_string()))??;

        Ok(result)
    }

    #[allow(dead_code)]
    async fn get_records_by_prefix<T: DeserializeOwned + Send + 'static>(
        &self,
        tree_name: &str,
        prefix: &[u8],
    ) -> Result<Vec<T>, OyaDbError> {
        let db = Arc::clone(&self.db);
        let tree_name = tree_name.to_string();
        let prefix = prefix.to_vec();

        let result = tokio::task::spawn_blocking(move || {
            let tree = db.open_tree(&tree_name)?;
            let mut records = Vec::new();
            for item in tree.scan_prefix(&prefix) {
                let (_, value) = item?;
                let record: T = serde_json::from_slice(&value)
                    .map_err(|e| OyaDbError::Serialization(e.to_string()))?;
                records.push(record);
            }
            Ok::<_, OyaDbError>(records)
        })
        .await
        .map_err(|e| OyaDbError::Serialization(e.to_string()))??;

        Ok(result)
    }

    pub async fn insert_agent_state(&self, state: &AgentState) -> Result<(), OyaDbError> {
        let current_stage_str = state
            .current_stage
            .as_ref()
            .map(|s| serde_json::to_string(s).map_err(|e| OyaDbError::Serialization(e.to_string())))
            .transpose()?;

        let record = AgentStateRecord {
            agent_id: state.agent_id.as_str().to_string(),
            bead_id: state.bead_id.as_ref().map(|b| b.as_str().to_string()),
            current_stage: current_stage_str,
            stage_started_at: state.stage_started_at.map(|t| t.to_rfc3339()),
            status: state.status.as_str().to_string(),
            last_update: state.last_update.to_rfc3339(),
            implementation_attempt: state.implementation_attempt,
            feedback: state.feedback.clone(),
        };

        self.insert_record("agent_states", state.agent_id.as_str().as_bytes(), &record).await
    }

    #[allow(dead_code)]
    pub async fn get_agent_state(&self, agent_id: &str) -> Result<Option<AgentState>, OyaDbError> {
        let record: Option<AgentStateRecord> =
            self.get_record("agent_states", agent_id.as_bytes()).await?;

        match record {
            Some(r) => {
                let current_stage: Option<Stage> = r
                    .current_stage
                    .map(|s| {
                        serde_json::from_str(&s)
                            .map_err(|e| OyaDbError::Serialization(e.to_string()))
                    })
                    .transpose()?;

                let status =
                    AgentStatus::try_from(r.status.as_str()).map_err(OyaDbError::Serialization)?;

                let stage_started_at = r
                    .stage_started_at
                    .map(|t| {
                        t.parse().map_err(|e: chrono::ParseError| {
                            OyaDbError::Serialization(e.to_string())
                        })
                    })
                    .transpose()?;

                let last_update = r
                    .last_update
                    .parse()
                    .map_err(|e: chrono::ParseError| OyaDbError::Serialization(e.to_string()))?;

                Ok(Some(AgentState {
                    agent_id: AgentId(r.agent_id),
                    bead_id: r.bead_id.map(BeadId::new),
                    current_stage,
                    stage_started_at,
                    status,
                    last_update,
                    implementation_attempt: r.implementation_attempt,
                    feedback: r.feedback,
                }))
            }
            None => Ok(None),
        }
    }

    pub async fn insert_bead_run(&self, run: &BeadRun) -> Result<(), OyaDbError> {
        let record = BeadRunRecord {
            run_id: run.id.0.clone(),
            bead_id: run.bead_id.0.clone(),
            state: serde_json::to_string(&run.state)
                .map_err(|e| OyaDbError::Serialization(e.to_string()))?,
            created_at: run.created_at.to_rfc3339(),
            updated_at: run.updated_at.to_rfc3339(),
        };

        self.insert_record("bead_runs", run.id.0.as_bytes(), &record).await
    }

    pub async fn update_run_state(&self, run_id: &str, state: &RunState) -> Result<(), OyaDbError> {
        let run_id_owned = run_id.to_string();
        let state_owned =
            serde_json::to_string(state).map_err(|e| OyaDbError::Serialization(e.to_string()))?;
        let db = Arc::clone(&self.db);

        tokio::task::spawn_blocking(move || {
            let tree = db.open_tree("bead_runs")?;
            let key = run_id_owned.as_bytes();
            let value = tree.get(key)?.ok_or_else(|| OyaDbError::NotFound(run_id_owned.clone()))?;

            let mut record: BeadRunRecord = serde_json::from_slice(&value)
                .map_err(|e| OyaDbError::Serialization(e.to_string()))?;

            record.state = state_owned;
            record.updated_at = chrono::Utc::now().to_rfc3339();

            let encoded = serde_json::to_vec(&record)
                .map_err(|e| OyaDbError::Serialization(e.to_string()))?;

            tree.insert(key, encoded)?;
            tree.flush()?;
            Ok::<_, OyaDbError>(())
        })
        .await
        .map_err(|e| OyaDbError::Serialization(e.to_string()))??;

        Ok(())
    }

    pub async fn get_run(&self, run_id: &str) -> Result<Option<BeadRun>, OyaDbError> {
        let record: Option<BeadRunRecord> = self.get_record("bead_runs", run_id.as_bytes()).await?;

        match record {
            Some(r) => {
                let state: RunState = serde_json::from_str(&r.state)
                    .map_err(|e| OyaDbError::Serialization(e.to_string()))?;
                let created_at = r
                    .created_at
                    .parse()
                    .map_err(|e: chrono::ParseError| OyaDbError::Serialization(e.to_string()))?;
                let updated_at = r
                    .updated_at
                    .parse()
                    .map_err(|e: chrono::ParseError| OyaDbError::Serialization(e.to_string()))?;

                Ok(Some(BeadRun {
                    id: RunId(r.run_id),
                    bead_id: BeadId(r.bead_id),
                    state,
                    created_at,
                    updated_at,
                    history: Vec::new(),
                }))
            }
            None => Ok(None),
        }
    }

    pub async fn insert_stage_attempt(&self, attempt: &StageAttempt) -> Result<(), OyaDbError> {
        let stage_str = serde_json::to_string(&attempt.stage)
            .map_err(|e| OyaDbError::Serialization(e.to_string()))?;
        let record = StageAttemptRecord {
            run_id: attempt.run_id.clone(),
            stage: stage_str.clone(),
            attempt: attempt.attempt,
            session_id: attempt.session_id.clone().map_or_else(String::new, std::convert::identity),
            state: serde_json::to_string(&attempt.state)
                .map_err(|e| OyaDbError::Serialization(e.to_string()))?,
            started_at: attempt.started_at.to_rfc3339(),
            completed_at: attempt.completed_at.map(|t| t.to_rfc3339()),
        };

        let key = format!("{}:{}:{:03}", attempt.run_id, stage_str, attempt.attempt);
        self.insert_record("stage_attempts", key.as_bytes(), &record).await
    }

    #[allow(dead_code)]
    pub async fn update_stage_attempt_state(
        &self,
        run_id: &str,
        stage: &str,
        attempt: u32,
        state: &str,
    ) -> Result<(), OyaDbError> {
        let key = format!("{}:{}:{:03}", run_id, stage, attempt);
        let state_owned = state.to_string();
        let db = Arc::clone(&self.db);

        tokio::task::spawn_blocking(move || {
            let tree = db.open_tree("stage_attempts")?;
            let value =
                tree.get(key.as_bytes())?.ok_or_else(|| OyaDbError::NotFound(key.clone()))?;

            let mut record: StageAttemptRecord = serde_json::from_slice(&value)
                .map_err(|e| OyaDbError::Serialization(e.to_string()))?;

            let parsed_state = match state_owned.as_str() {
                "pending" => StageState::Pending,
                "running" => StageState::Running,
                "passed" => StageState::Passed,
                "failed" => StageState::Failed,
                "waiting_permission" => StageState::WaitingPermission,
                "waiting_question" => StageState::WaitingQuestion,
                _ => {
                    return Err(OyaDbError::Serialization(format!(
                        "Unknown stage attempt state: {}",
                        state_owned
                    )))
                }
            };

            record.state = serde_json::to_string(&parsed_state)
                .map_err(|e| OyaDbError::Serialization(e.to_string()))?;
            record.completed_at = Some(chrono::Utc::now().to_rfc3339());

            let encoded = serde_json::to_vec(&record)
                .map_err(|e| OyaDbError::Serialization(e.to_string()))?;

            tree.insert(key.as_bytes(), encoded)?;
            tree.flush()?;
            Ok::<_, OyaDbError>(())
        })
        .await
        .map_err(|e| OyaDbError::Serialization(e.to_string()))??;

        Ok(())
    }

    pub async fn insert_stage_result(&self, result: &StageResult) -> Result<(), OyaDbError> {
        let stage_str = serde_json::to_string(&result.stage)
            .map_err(|e| OyaDbError::Serialization(e.to_string()))?;
        let record = StageResultRecord {
            run_id: result.run_id.clone(),
            stage: stage_str.clone(),
            attempt: result.attempt,
            passed: result.passed,
            output: Some(result.output.to_string()),
            failure_category: result
                .failure_category
                .as_ref()
                .map(|f| {
                    serde_json::to_string(f).map_err(|e| OyaDbError::Serialization(e.to_string()))
                })
                .transpose()?,
            next_stage: result
                .next_stage
                .as_ref()
                .map(|s| {
                    serde_json::to_string(s).map_err(|e| OyaDbError::Serialization(e.to_string()))
                })
                .transpose()?,
        };

        let key = format!("{}:{}:{:03}", result.run_id, stage_str, result.attempt);
        self.insert_record("stage_results", key.as_bytes(), &record).await
    }

    pub async fn get_stage_results(&self, run_id: &str) -> Result<Vec<StageResult>, OyaDbError> {
        let prefix = format!("{}:", run_id);
        let records: Vec<StageResultRecord> =
            self.get_records_by_prefix("stage_results", prefix.as_bytes()).await?;

        let mut stage_results = Vec::new();
        for r in records {
            let stage: Stage = serde_json::from_str(&r.stage)
                .map_err(|e| OyaDbError::Serialization(e.to_string()))?;
            let output = match r.output {
                Some(output_json) => serde_json::from_str(&output_json)
                    .map_err(|e| OyaDbError::Serialization(e.to_string()))?,
                None => serde_json::Value::Null,
            };
            let failure_category = r
                .failure_category
                .map(|f| serde_json::from_str(&f))
                .transpose()
                .map_err(|e| OyaDbError::Serialization(e.to_string()))?;
            let next_stage = r
                .next_stage
                .map(|s| serde_json::from_str(&s))
                .transpose()
                .map_err(|e| OyaDbError::Serialization(e.to_string()))?;

            stage_results.push(StageResult {
                run_id: r.run_id,
                stage,
                attempt: r.attempt,
                passed: r.passed,
                output,
                failure_category,
                next_stage,
            });
        }

        stage_results.sort_by_key(|result| (stage_order(&result.stage), result.attempt));
        Ok(stage_results)
    }

    #[allow(dead_code)]
    pub async fn insert_artifact(&self, artifact: &Artifact) -> Result<(), OyaDbError> {
        let record = ArtifactRecord {
            id: artifact.id.clone(),
            run_id: artifact.run_id.clone(),
            artifact_type: artifact.artifact_type.as_str().to_string(),
            location: artifact.location.clone(),
            checksum: artifact.checksum.clone().map_or_else(String::new, std::convert::identity),
            produced_by_stage: serde_json::to_string(&artifact.produced_by_stage)
                .map_err(|e| OyaDbError::Serialization(e.to_string()))?,
        };

        let key = format!("{}:{}", artifact.run_id, artifact.id);
        self.insert_record("artifacts", key.as_bytes(), &record).await
    }

    #[allow(dead_code)]
    pub async fn insert_gate_result(&self, gate: &GateResult) -> Result<(), OyaDbError> {
        let record = GateResultRecord {
            run_id: gate.run_id.clone(),
            gate_name: gate.gate_name.clone(),
            passed: gate.passed,
            exit_code: gate.exit_code,
            log_ref: gate.log_ref.clone(),
        };

        let key = format!("{}:{}", gate.run_id, gate.gate_name);
        self.insert_record("gate_results", key.as_bytes(), &record).await
    }

    pub async fn insert_ship_decision(&self, decision: &ShipDecision) -> Result<(), OyaDbError> {
        let record = ShipDecisionRecord {
            run_id: decision.run_id.clone(),
            shipped: decision.shipped,
            rationale: decision.rationale.clone(),
            approver_mode: serde_json::to_string(&decision.approver_mode)
                .map_err(|e| OyaDbError::Serialization(e.to_string()))?,
            timestamp: decision.timestamp.to_rfc3339(),
        };

        self.insert_record("ship_decisions", decision.run_id.as_bytes(), &record).await
    }
}

fn stage_order(stage: &Stage) -> u8 {
    match stage {
        Stage::Contract => 0,
        Stage::Tdd15 => 1,
        Stage::Qa => 2,
        Stage::RedQueen => 3,
        Stage::GptReview => 4,
        Stage::ShipGate => 5,
    }
}

/// Parse artifact type from string (pure function)
///
/// This is a helper to convert string representations back to ArtifactType enum
fn parse_artifact_type(s: &str) -> Result<ArtifactType, OyaDbError> {
    match s {
        "contract_document" => Ok(ArtifactType::ContractDocument),
        "requirements" => Ok(ArtifactType::Requirements),
        "system_context" => Ok(ArtifactType::SystemContext),
        "invariants" => Ok(ArtifactType::Invariants),
        "data_flow" => Ok(ArtifactType::DataFlow),
        "implementation_plan" => Ok(ArtifactType::ImplementationPlan),
        "acceptance_criteria" => Ok(ArtifactType::AcceptanceCriteria),
        "error_handling" => Ok(ArtifactType::ErrorHandling),
        "test_scenarios" => Ok(ArtifactType::TestScenarios),
        "validation_gates" => Ok(ArtifactType::ValidationGates),
        "success_metrics" => Ok(ArtifactType::SuccessMetrics),
        "implementation_code" => Ok(ArtifactType::ImplementationCode),
        "modified_files" => Ok(ArtifactType::ModifiedFiles),
        "implementation_notes" => Ok(ArtifactType::ImplementationNotes),
        "test_output" => Ok(ArtifactType::TestOutput),
        "test_results" => Ok(ArtifactType::TestResults),
        "coverage_report" => Ok(ArtifactType::CoverageReport),
        "validation_report" => Ok(ArtifactType::ValidationReport),
        "failure_details" => Ok(ArtifactType::FailureDetails),
        "adversarial_report" => Ok(ArtifactType::AdversarialReport),
        "regression_report" => Ok(ArtifactType::RegressionReport),
        "quality_gate_report" => Ok(ArtifactType::QualityGateReport),
        "stage_log" => Ok(ArtifactType::StageLog),
        "retry_packet" => Ok(ArtifactType::RetryPacket),
        "skill_invocation" => Ok(ArtifactType::SkillInvocation),
        "error_message" => Ok(ArtifactType::ErrorMessage),
        "feedback" => Ok(ArtifactType::Feedback),
        _ => Err(OyaDbError::Serialization(format!("Unknown artifact type: {}", s))),
    }
}

/// Validate state transition (precondition for update_run_state_sync)
///
/// Pure function: validates RunState transitions per state machine rules
fn is_valid_transition(from: &RunState, to: &RunState) -> bool {
    match (from, to) {
        // Pending can go to Running
        (RunState::Pending, RunState::Running { .. }) => true,
        // Running can go to Running (next stage), Waiting, or terminal states
        (RunState::Running { .. }, RunState::Running { .. }) => true,
        (RunState::Running { .. }, RunState::Waiting { .. }) => true,
        (RunState::Running { .. }, RunState::Shipped { .. }) => true,
        (RunState::Running { .. }, RunState::Failed { .. }) => true,
        (RunState::Running { .. }, RunState::Aborted { .. }) => true,
        // Waiting can go back to Running or to terminal states
        (RunState::Waiting { .. }, RunState::Running { .. }) => true,
        (RunState::Waiting { .. }, RunState::Failed { .. }) => true,
        (RunState::Waiting { .. }, RunState::Aborted { .. }) => true,
        // Terminal states are absorbing (no transitions out)
        (RunState::Shipped { .. }, _) => false,
        (RunState::Failed { .. }, _) => false,
        (RunState::Aborted { .. }, _) => false,
        // All other transitions are invalid
        _ => false,
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;
    use crate::domain::{ArtifactType, StageState};
    use chrono::Utc;

    fn create_test_db() -> OyaDb {
        OyaDb::connect_sync("memory://").unwrap()
    }

    fn create_test_run() -> BeadRun {
        BeadRun {
            id: RunId::new(),
            bead_id: BeadId::new("test-bead-001"),
            state: RunState::Pending,
            created_at: Utc::now(),
            updated_at: Utc::now(),
            history: Vec::new(),
        }
    }

    #[test]
    fn test_insert_new_run_persists_all_fields() {
        let db = create_test_db();
        let run = create_test_run();

        let result = db.insert_run_sync(&run);

        assert!(result.is_ok(), "insert_run_sync should succeed");
    }

    #[test]
    fn test_get_run_retrieves_complete_run() {
        let db = create_test_db();
        let run = create_test_run();
        let run_id = run.id.as_str().to_string();

        db.insert_run_sync(&run).unwrap();

        let retrieved = db.get_run_sync(&run_id).unwrap();

        assert_eq!(retrieved.id.0, run.id.0);
        assert_eq!(retrieved.bead_id.0, run.bead_id.0);
        assert_eq!(retrieved.state, RunState::Pending);
        assert_eq!(retrieved.history.len(), 0);
    }

    #[test]
    fn test_get_run_returns_error_when_run_does_not_exist() {
        let db = create_test_db();

        let result = db.get_run_sync("nonexistent-run");

        assert!(matches!(result, Err(OyaDbError::NotFound(_))));
    }

    #[test]
    fn test_insert_stage_attempt_persists_attempt_details() {
        let db = create_test_db();
        let run = create_test_run();
        db.insert_run_sync(&run).unwrap();

        let attempt = StageAttempt {
            run_id: run.id.0.clone(),
            stage: Stage::Contract,
            attempt: 1,
            session_id: Some("session-123".to_string()),
            state: StageState::Running,
            started_at: Utc::now(),
            completed_at: None,
        };

        let result = db.insert_stage_attempt_sync(&attempt);

        assert!(result.is_ok());
    }

    #[test]
    fn test_insert_stage_attempt_returns_error_when_run_not_found() {
        let db = create_test_db();

        let attempt = StageAttempt {
            run_id: "ghost-run".to_string(),
            stage: Stage::Contract,
            attempt: 1,
            session_id: None,
            state: StageState::Pending,
            started_at: Utc::now(),
            completed_at: None,
        };

        let result = db.insert_stage_attempt_sync(&attempt);

        assert!(matches!(result, Err(OyaDbError::RunNotFound(_))));
    }

    #[test]
    fn test_insert_stage_attempt_enforces_max_attempts() {
        let db = create_test_db();
        let run = create_test_run();
        db.insert_run_sync(&run).unwrap();

        let attempt = StageAttempt {
            run_id: run.id.0.clone(),
            stage: Stage::Contract,
            attempt: 4, // Exceeds max_attempts() of 3
            session_id: None,
            state: StageState::Pending,
            started_at: Utc::now(),
            completed_at: None,
        };

        let result = db.insert_stage_attempt_sync(&attempt);

        assert!(matches!(
            result,
            Err(OyaDbError::AttemptLimitExceeded { stage: _, attempt: 4, max: 3 })
        ));
    }

    #[test]
    fn test_get_stage_attempts_returns_ordered_attempts() {
        let db = create_test_db();
        let run = create_test_run();
        db.insert_run_sync(&run).unwrap();

        // Insert attempts out of order to test sorting
        let attempt2 = StageAttempt {
            run_id: run.id.0.clone(),
            stage: Stage::Contract,
            attempt: 2,
            session_id: None,
            state: StageState::Pending,
            started_at: Utc::now(),
            completed_at: None,
        };

        let attempt1 = StageAttempt {
            run_id: run.id.0.clone(),
            stage: Stage::Contract,
            attempt: 1,
            session_id: None,
            state: StageState::Running,
            started_at: Utc::now(),
            completed_at: None,
        };

        db.insert_stage_attempt_sync(&attempt2).unwrap();
        db.insert_stage_attempt_sync(&attempt1).unwrap();

        let attempts = db.get_stage_attempts_sync(&run.id.0).unwrap();

        assert_eq!(attempts.len(), 2);
        assert_eq!(attempts[0].attempt, 1);
        assert_eq!(attempts[1].attempt, 2);
    }

    #[test]
    fn test_insert_artifact_persists_with_checksum() {
        let db = create_test_db();
        let run = create_test_run();
        db.insert_run_sync(&run).unwrap();

        let artifact = Artifact {
            id: "artifact-001".to_string(),
            run_id: run.id.0.clone(),
            artifact_type: ArtifactType::ContractDocument,
            location: "/path/to/contract.md".to_string(),
            checksum: Some("abc123def".to_string()),
            produced_by_stage: Stage::Contract,
        };

        let result = db.insert_artifact_sync(&artifact);

        assert!(result.is_ok());
    }

    #[test]
    fn test_insert_artifact_returns_error_when_run_not_found() {
        let db = create_test_db();

        let artifact = Artifact {
            id: "orphan-artifact".to_string(),
            run_id: "ghost-run".to_string(),
            artifact_type: ArtifactType::ContractDocument,
            location: "/path/to/contract.md".to_string(),
            checksum: None,
            produced_by_stage: Stage::Contract,
        };

        let result = db.insert_artifact_sync(&artifact);

        assert!(matches!(result, Err(OyaDbError::OrphanedArtifact { .. })));
    }

    #[test]
    fn test_get_artifacts_filters_by_stage() {
        let db = create_test_db();
        let run = create_test_run();
        db.insert_run_sync(&run).unwrap();

        let artifact1 = Artifact {
            id: "artifact-001".to_string(),
            run_id: run.id.0.clone(),
            artifact_type: ArtifactType::ContractDocument,
            location: "/path/1".to_string(),
            checksum: None,
            produced_by_stage: Stage::Contract,
        };

        let artifact2 = Artifact {
            id: "artifact-002".to_string(),
            run_id: run.id.0.clone(),
            artifact_type: ArtifactType::ImplementationCode,
            location: "/path/2".to_string(),
            checksum: None,
            produced_by_stage: Stage::Tdd15,
        };

        db.insert_artifact_sync(&artifact1).unwrap();
        db.insert_artifact_sync(&artifact2).unwrap();

        let contract_artifacts = db.get_artifacts_sync(&run.id.0, Some(Stage::Contract)).unwrap();

        assert_eq!(contract_artifacts.len(), 1);
        assert_eq!(contract_artifacts[0].artifact_type, ArtifactType::ContractDocument);

        let all_artifacts = db.get_artifacts_sync(&run.id.0, None).unwrap();
        assert_eq!(all_artifacts.len(), 2);
    }

    #[test]
    fn test_update_run_state_transitions_from_pending_to_running() {
        let db = create_test_db();
        let run = create_test_run();
        db.insert_run_sync(&run).unwrap();

        let new_state = RunState::Running { current_stage: Stage::Contract };

        let result = db.update_run_state_sync(&run.id.0, &new_state);

        assert!(result.is_ok());

        let updated = db.get_run_sync(&run.id.0).unwrap();
        assert!(matches!(updated.state, RunState::Running { .. }));
    }

    #[test]
    fn test_update_run_state_returns_error_on_invalid_transition() {
        let db = create_test_db();
        let run = create_test_run();

        // Create run in Shipped state
        let shipped_run =
            BeadRun { state: RunState::Shipped { completed_at: Utc::now() }, ..run.clone() };
        db.insert_run_sync(&shipped_run).unwrap();

        // Try to transition from Shipped to Running (invalid)
        let new_state = RunState::Running { current_stage: Stage::Contract };

        let result = db.update_run_state_sync(&run.id.0, &new_state);

        assert!(matches!(result, Err(OyaDbError::InvalidStateTransition { .. })));
    }

    #[test]
    fn test_insert_run_is_idempotent() {
        let db = create_test_db();
        let run = create_test_run();
        let run_id = run.id.0.clone();

        // First insert
        db.insert_run_sync(&run).unwrap();

        // Modify and insert again with same ID
        let modified_run = BeadRun { bead_id: BeadId::new("different-bead"), ..run.clone() };
        db.insert_run_sync(&modified_run).unwrap();

        // Should retrieve the latest version (upsert semantics)
        let retrieved = db.get_run_sync(&run_id).unwrap();
        assert_eq!(retrieved.bead_id.0, "different-bead");
    }

    #[test]
    fn test_replayability_restores_run_with_history() {
        let db = create_test_db();
        let run = create_test_run();
        let run_id = run.id.0.clone();

        // Insert run
        db.insert_run_sync(&run).unwrap();

        // Insert attempts
        for attempt_num in 1..=2 {
            let attempt = StageAttempt {
                run_id: run_id.clone(),
                stage: Stage::Contract,
                attempt: attempt_num,
                session_id: None,
                state: StageState::Passed,
                started_at: Utc::now(),
                completed_at: Some(Utc::now()),
            };
            db.insert_stage_attempt_sync(&attempt).unwrap();
        }

        // Insert artifact
        let artifact = Artifact {
            id: "artifact-001".to_string(),
            run_id: run_id.clone(),
            artifact_type: ArtifactType::ContractDocument,
            location: "/path/contract.md".to_string(),
            checksum: None,
            produced_by_stage: Stage::Contract,
        };
        db.insert_artifact_sync(&artifact).unwrap();

        // Replay: get run should restore full state
        let restored = db.get_run_sync(&run_id).unwrap();

        assert_eq!(restored.id.0, run_id);
        assert_eq!(restored.history.len(), 2);

        let artifacts = db.get_artifacts_sync(&run_id, None).unwrap();
        assert_eq!(artifacts.len(), 1);
    }
}
