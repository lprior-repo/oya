use chrono::Utc;
use std::fs::OpenOptions;
use std::io::Write;
use std::path::Path;

use super::model::{
    MetricsData, MetricsStore, QualityGateIteration, QualityGateSession, ScenarioValidationMetrics,
    SessionId, SessionStatus, SpecId, SpecValidationMetrics, SpecVersion,
    SuggestionDecisionMetrics,
};

impl MetricsStore {
    /// Creates a new `MetricsStore` backed by the given directory.
    ///
    /// If the directory cannot be created, logs the error and continues.
    /// If persisted data cannot be loaded, starts with default empty data.
    #[must_use]
    pub fn new(base_path: &Path) -> Self {
        let data_path = base_path.join("quality-metrics");

        match std::fs::create_dir_all(&data_path) {
            Ok(()) => {}
            Err(e) => {
                eprintln!("Warning: could not create metrics directory {data_path:?}: {e}");
            }
        }

        let data: MetricsData = match Self::load_data(&data_path) {
            Ok(loaded) => loaded,
            Err(e) => {
                eprintln!("Warning: could not load metrics data: {e}");
                MetricsData::default()
            }
        };

        Self { base_path: data_path, data: std::sync::Arc::new(std::sync::RwLock::new(data)) }
    }

    fn load_data(path: &Path) -> Result<MetricsData, Box<dyn std::error::Error>> {
        let metrics_file = path.join("metrics.json");
        if metrics_file.exists() {
            let content = std::fs::read_to_string(&metrics_file)?;
            Ok(serde_json::from_str(&content)?)
        } else {
            Ok(MetricsData::default())
        }
    }

    /// Save data to disk.
    ///
    /// # Errors
    /// Returns an error if the lock cannot be acquired or writing fails.
    pub fn save_data(&self) -> Result<(), Box<dyn std::error::Error>> {
        let json = {
            let data = self.data.read().map_err(|e| format!("Failed to acquire lock: {e}"))?;
            serde_json::to_string_pretty(&*data)?
        };
        let metrics_file = self.base_path.join("metrics.json");

        let mut file =
            OpenOptions::new().write(true).create(true).truncate(true).open(&metrics_file)?;

        file.write_all(json.as_bytes())?;

        Ok(())
    }

    /// Record spec validation metrics.
    ///
    /// # Errors
    /// Returns an error if saving fails.
    pub fn record_spec_validation(
        &self,
        metrics: SpecValidationMetrics,
    ) -> Result<(), Box<dyn std::error::Error>> {
        {
            let mut data = self.data.write().map_err(|e| format!("Failed to acquire lock: {e}"))?;
            data.spec_validations.push(metrics);
        }
        self.save_data()
    }

    /// Record scenario validation metrics.
    ///
    /// # Errors
    /// Returns an error if saving fails.
    pub fn record_scenario_validation(
        &self,
        metrics: ScenarioValidationMetrics,
    ) -> Result<(), Box<dyn std::error::Error>> {
        {
            let mut data = self.data.write().map_err(|e| format!("Failed to acquire lock: {e}"))?;
            data.scenario_validations.push(metrics);
        }
        self.save_data()
    }

    /// Record extension suggestion acceptance/rejection metrics.
    ///
    /// # Errors
    /// Returns an error if saving fails.
    pub fn record_suggestion_decision(
        &self,
        metrics: SuggestionDecisionMetrics,
    ) -> Result<(), Box<dyn std::error::Error>> {
        {
            let mut data = self.data.write().map_err(|e| format!("Failed to acquire lock: {e}"))?;
            data.suggestion_decisions.push(metrics);
        }
        self.save_data()
    }

    /// Start a new quality gate session.
    ///
    /// # Errors
    /// Returns an error if saving fails.
    pub fn start_session(
        &self,
        spec_id: &str,
        spec_version: &str,
    ) -> Result<String, Box<dyn std::error::Error>> {
        let session_id = SessionId::new();
        let session_id_str = session_id.to_string();
        let timestamp = Utc::now();

        {
            let mut data = self.data.write().map_err(|e| format!("Failed to acquire lock: {e}"))?;

            let session = QualityGateSession {
                session_id,
                spec_id: SpecId::new(spec_id).map_err(|e| format!("Invalid spec_id: {e}"))?,
                spec_version: SpecVersion::new(spec_version)
                    .map_err(|e| format!("Invalid spec_version: {e}"))?,
                started_at: timestamp,
                completed_at: None,
                iterations: Vec::new(),
                total_duration_ms: 0,
                status: SessionStatus::InProgress,
                escalated: false,
            };

            data.sessions.push(session);
        }
        self.save_data()?;

        Ok(session_id_str)
    }

    /// Record a quality gate iteration.
    ///
    /// # Errors
    /// Returns an error if saving fails.
    pub fn record_iteration(
        &self,
        session_id: &str,
        iteration: QualityGateIteration,
    ) -> Result<(), Box<dyn std::error::Error>> {
        {
            let mut data = self.data.write().map_err(|e| format!("Failed to acquire lock: {e}"))?;

            if let Some(session) =
                data.sessions.iter_mut().find(|s| s.session_id.as_str() == session_id)
            {
                let passed = iteration.overall_passed;
                session.iterations.push(iteration);

                let now = Utc::now();
                if passed {
                    session.status = SessionStatus::Passed;
                    session.completed_at = Some(now);
                } else if session.iterations.len() >= 5 {
                    session.status = SessionStatus::Failed;
                    session.completed_at = Some(now);
                    session.escalated = true;
                }
            }
        }
        self.save_data()
    }

    #[must_use]
    pub fn get_session(&self, session_id: &str) -> Option<QualityGateSession> {
        let data = self.data.read().ok()?;
        data.sessions.iter().find(|s| s.session_id.as_str() == session_id).cloned()
    }
}
