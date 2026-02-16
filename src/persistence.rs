use crate::orchestration::{Artifact, Run as BeadRun, GateResult, ShipDecision, StageName as Stage, StageAttempt, StageResult, RunState, BeadId, RunId, AgentState, AgentId, AgentStatus};
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

    pub async fn init_schema(&self) -> Result<(), OyaDbError> {
        Ok(())
    }

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
        let current_stage_str = state.current_stage
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

        self.insert_record("agent_states", state.agent_id.as_str().as_bytes(), &record)
            .await
    }

    #[allow(dead_code)]
    pub async fn get_agent_state(&self, agent_id: &str) -> Result<Option<AgentState>, OyaDbError> {
        let record: Option<AgentStateRecord> = self.get_record("agent_states", agent_id.as_bytes()).await?;

        match record {
            Some(r) => {
                let current_stage: Option<Stage> = r.current_stage
                    .map(|s| serde_json::from_str(&s).map_err(|e| OyaDbError::Serialization(e.to_string())))
                    .transpose()?;
                
                let status = AgentStatus::try_from(r.status.as_str())
                    .map_err(|e| OyaDbError::Serialization(e))?;

                let stage_started_at = r.stage_started_at
                    .map(|t| t.parse().map_err(|e: chrono::ParseError| OyaDbError::Serialization(e.to_string())))
                    .transpose()?;

                let last_update = r.last_update
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

        self.insert_record("bead_runs", run.id.0.as_bytes(), &record)
            .await
    }

    pub async fn update_run_state(
        &self,
        run_id: &str,
        state: &RunState,
    ) -> Result<(), OyaDbError> {
        let run_id_owned = run_id.to_string();
        let state_owned = serde_json::to_string(state)
            .map_err(|e| OyaDbError::Serialization(e.to_string()))?;
        let db = Arc::clone(&self.db);

        tokio::task::spawn_blocking(move || {
            let tree = db.open_tree("bead_runs")?;
            let key = run_id_owned.as_bytes();
            let value = tree
                .get(key)?
                .ok_or_else(|| OyaDbError::NotFound(run_id_owned.clone()))?;

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
            session_id: attempt.session_id.clone().unwrap_or_default(),
            state: serde_json::to_string(&attempt.state)
                .map_err(|e| OyaDbError::Serialization(e.to_string()))?,
            started_at: attempt.started_at.to_rfc3339(),
            completed_at: attempt.completed_at.map(|t| t.to_rfc3339()),
        };

        let key = format!("{}:{}:{:03}", attempt.run_id, stage_str, attempt.attempt);
        self.insert_record("stage_attempts", key.as_bytes(), &record)
            .await
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
            let value = tree
                .get(key.as_bytes())?
                .ok_or_else(|| OyaDbError::NotFound(key.clone()))?;

            let mut record: StageAttemptRecord = serde_json::from_slice(&value)
                .map_err(|e| OyaDbError::Serialization(e.to_string()))?;

            record.state = state_owned;
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
                .map(|f| serde_json::to_string(f).unwrap_or_default()),
            next_stage: result
                .next_stage
                .as_ref()
                .map(|s| serde_json::to_string(s).unwrap_or_default()),
        };

        let key = format!("{}:{}:{:03}", result.run_id, stage_str, result.attempt);
        self.insert_record("stage_results", key.as_bytes(), &record)
            .await
    }

    pub async fn get_stage_results(&self, run_id: &str) -> Result<Vec<StageResult>, OyaDbError> {
        let prefix = format!("{}:", run_id);
        let records: Vec<StageResultRecord> = self
            .get_records_by_prefix("stage_results", prefix.as_bytes())
            .await?;

        let mut stage_results = Vec::new();
        for r in records {
            let stage: Stage = serde_json::from_str(&r.stage)
                .map_err(|e| OyaDbError::Serialization(e.to_string()))?;
            let output = r
                .output
                .map(|s| serde_json::from_str(&s).unwrap_or(serde_json::Value::Null))
                .unwrap_or(serde_json::Value::Null);
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

        stage_results.sort_by_key(|r| r.attempt);
        Ok(stage_results)
    }

    #[allow(dead_code)]
    pub async fn insert_artifact(&self, artifact: &Artifact) -> Result<(), OyaDbError> {
        let record = ArtifactRecord {
            id: artifact.id.clone(),
            run_id: artifact.run_id.clone(),
            artifact_type: artifact.artifact_type.as_str().to_string(),
            location: artifact.location.clone(),
            checksum: artifact.checksum.clone().unwrap_or_default(),
            produced_by_stage: serde_json::to_string(&artifact.produced_by_stage)
                .map_err(|e| OyaDbError::Serialization(e.to_string()))?,
        };

        let key = format!("{}:{}", artifact.run_id, artifact.id);
        self.insert_record("artifacts", key.as_bytes(), &record)
            .await
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
        self.insert_record("gate_results", key.as_bytes(), &record)
            .await
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

        self.insert_record("ship_decisions", decision.run_id.as_bytes(), &record)
            .await
    }
}
