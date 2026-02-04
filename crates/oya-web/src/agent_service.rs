//! Agent service for spawning and tracking workers.

use std::collections::HashMap;
use std::env;
use std::process::Stdio;
use std::sync::Arc;

use async_trait::async_trait;
use orchestrator::agent_swarm::{
    AgentHandle, AgentPool, AgentStateLegacy, AgentSwarmError, HealthConfig, PoolConfig,
};
use tokio::process::Command;
use tokio::sync::Mutex;
use ulid::Ulid;

use crate::agent_repository::{AgentRepository, AgentSnapshot, InMemoryAgentRepository};
use crate::error::AppError;

const DEFAULT_MAX_AGENTS: usize = 100;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AgentRunner {
    Opencode,
    Zjj,
    Custom,
}

#[derive(Debug, Clone)]
pub struct AgentServiceConfig {
    pub max_agents: usize,
    pub runner: AgentRunner,
    pub command: String,
    pub args: Vec<String>,
    pub working_dir: Option<String>,
    pub env: Vec<(String, String)>,
}

impl Default for AgentServiceConfig {
    fn default() -> Self {
        Self {
            max_agents: DEFAULT_MAX_AGENTS,
            runner: AgentRunner::Opencode,
            command: "opencode".to_string(),
            args: Vec::new(),
            working_dir: None,
            env: Vec::new(),
        }
    }
}

impl AgentServiceConfig {
    pub fn from_env() -> Result<Self, AgentServiceError> {
        let max_agents = parse_env_usize("OYA_AGENT_MAX", DEFAULT_MAX_AGENTS)?;
        let runner = parse_runner(env::var("OYA_AGENT_RUNNER").ok())?;
        let command = parse_command(runner)?;
        let args = parse_env_args("OYA_AGENT_ARGS");
        let working_dir = env::var("OYA_AGENT_WORKDIR").ok();
        let env_pairs = parse_env_pairs("OYA_AGENT_ENV");

        Ok(Self {
            max_agents,
            runner,
            command,
            args,
            working_dir,
            env: env_pairs,
        })
    }
}

impl AgentSnapshot {
    fn from_handle(handle: &AgentHandle) -> Self {
        let health_score = health_score_for(handle.state());
        let uptime_secs = handle.uptime().as_secs();

        Self {
            id: handle.id().to_string(),
            status: handle.state().to_string(),
            current_bead: handle.current_bead().map(String::from),
            health_score,
            uptime_secs,
            capabilities: handle.capabilities().to_vec(),
        }
    }
}

#[derive(Debug, Clone)]
pub struct AgentSpawnSummary {
    pub agent_ids: Vec<String>,
    pub total: usize,
}

#[derive(Debug, Clone)]
pub struct AgentScaleSummary {
    pub previous: usize,
    pub total: usize,
    pub spawned: Vec<String>,
    pub terminated: Vec<String>,
}

pub struct AgentService {
    pool: AgentPool,
    launcher: Arc<dyn AgentLauncher>,
    repository: Arc<dyn AgentRepository>,
    processes: Arc<Mutex<HashMap<String, AgentProcessHandle>>>,
    _health_task: tokio::task::JoinHandle<()>,
}

impl AgentService {
    pub fn new(config: AgentServiceConfig) -> Self {
        let launcher = build_launcher(&config);
        let repository = Arc::new(InMemoryAgentRepository::new());
        Self::new_with_launcher_and_repository(config, launcher, repository)
    }

    pub fn new_with_launcher(config: AgentServiceConfig, launcher: Arc<dyn AgentLauncher>) -> Self {
        let repository = Arc::new(InMemoryAgentRepository::new());
        Self::new_with_launcher_and_repository(config, launcher, repository)
    }

    pub fn new_with_repository(
        config: AgentServiceConfig,
        repository: Arc<dyn AgentRepository>,
    ) -> Self {
        let launcher = build_launcher(&config);
        Self::new_with_launcher_and_repository(config, launcher, repository)
    }

    pub fn new_with_launcher_and_repository(
        config: AgentServiceConfig,
        launcher: Arc<dyn AgentLauncher>,
        repository: Arc<dyn AgentRepository>,
    ) -> Self {
        let pool = AgentPool::new(PoolConfig::new(config.max_agents, HealthConfig::default()));
        let processes = Arc::new(Mutex::new(HashMap::new()));
        let health_task = pool.start_health_monitoring();

        Self {
            pool,
            launcher,
            repository,
            processes,
            _health_task: health_task,
        }
    }

    pub async fn list_agents(&self) -> Vec<AgentSnapshot> {
        match self.repository.list().await {
            Ok(agents) if !agents.is_empty() => agents,
            _ => self.sync_repository().await,
        }
    }

    pub async fn spawn_agents(
        &self,
        count: usize,
        capabilities: Vec<String>,
    ) -> Result<AgentSpawnSummary, AgentServiceError> {
        validate_count(count)?;
        let (current, max) = self.capacity_snapshot().await?;
        ensure_capacity(count, current, max)?;

        let mut agent_ids = Vec::with_capacity(count);
        for _ in 0..count {
            let agent_id = self.spawn_one(&capabilities).await?;
            agent_ids.push(agent_id);
        }

        let total = self.pool.len().await;
        Ok(AgentSpawnSummary { agent_ids, total })
    }

    pub async fn scale_to(&self, target: usize) -> Result<AgentScaleSummary, AgentServiceError> {
        let previous = self.pool.len().await;

        if target == previous {
            return Ok(scale_summary(previous, previous, Vec::new(), Vec::new()));
        }

        if target > previous {
            return self.scale_up(previous, target).await;
        }

        self.scale_down(previous, target).await
    }

    async fn spawn_one(&self, capabilities: &[String]) -> Result<String, AgentServiceError> {
        let agent_id = Ulid::new().to_string();
        self.register_and_launch(&agent_id, capabilities).await?;
        self.record_agent_snapshot(&agent_id).await;
        Ok(agent_id)
    }

    async fn register_and_launch(
        &self,
        agent_id: &str,
        capabilities: &[String],
    ) -> Result<(), AgentServiceError> {
        let handle = AgentHandle::new(agent_id).with_capabilities(capabilities.to_vec());
        self.pool
            .register_agent(handle)
            .await
            .map_err(map_pool_error)?;

        match self.launcher.launch(agent_id).await {
            Ok(process) => {
                let mut processes = self.processes.lock().await;
                processes.insert(agent_id.to_string(), process);
                Ok(())
            }
            Err(err) => {
                let _ = self.pool.unregister_agent(agent_id).await;
                Err(err)
            }
        }
    }

    async fn shutdown_idle_agents(&self, count: usize) -> Result<Vec<String>, AgentServiceError> {
        let idle_ids = self.idle_agent_ids(count).await?;
        let mut terminated = Vec::with_capacity(idle_ids.len());

        for agent_id in idle_ids {
            self.shutdown_agent(&agent_id).await?;
            terminated.push(agent_id);
        }

        Ok(terminated)
    }

    async fn scale_up(
        &self,
        previous: usize,
        target: usize,
    ) -> Result<AgentScaleSummary, AgentServiceError> {
        let spawned = self.spawn_agents(target - previous, Vec::new()).await?;
        Ok(scale_summary(
            previous,
            spawned.total,
            spawned.agent_ids,
            Vec::new(),
        ))
    }

    async fn scale_down(
        &self,
        previous: usize,
        target: usize,
    ) -> Result<AgentScaleSummary, AgentServiceError> {
        let to_remove = previous - target;
        let terminated = self.shutdown_idle_agents(to_remove).await?;
        let total = self.pool.len().await;
        Ok(scale_summary(previous, total, Vec::new(), terminated))
    }

    async fn shutdown_agent(&self, agent_id: &str) -> Result<(), AgentServiceError> {
        self.pool
            .shutdown_agent(agent_id)
            .await
            .map_err(map_pool_error)?;
        let process = self.take_process(agent_id).await?;
        process.shutdown(agent_id).await?;
        let _ = self.pool.unregister_agent(agent_id).await;
        let _ = self.repository.remove(agent_id).await;
        Ok(())
    }

    async fn take_process(&self, agent_id: &str) -> Result<AgentProcessHandle, AgentServiceError> {
        let mut processes = self.processes.lock().await;
        processes
            .remove(agent_id)
            .ok_or_else(|| AgentServiceError::process_missing(agent_id))
    }

    async fn capacity_snapshot(&self) -> Result<(usize, usize), AgentServiceError> {
        let current = self.pool.len().await;
        let max = self.pool.config().max_agents;
        Ok((current, max))
    }

    async fn idle_agent_ids(&self, count: usize) -> Result<Vec<String>, AgentServiceError> {
        let idle_agents: Vec<AgentHandle> = self.pool.get_available_agents().await;
        let available = idle_agents.len();

        if available < count {
            return Err(AgentServiceError::InsufficientIdle {
                requested: count,
                available,
            });
        }

        Ok(idle_agents
            .into_iter()
            .take(count)
            .map(|agent: AgentHandle| agent.id().to_string())
            .collect())
    }

    async fn snapshot_agent(&self, agent_id: &str) -> Option<AgentSnapshot> {
        self.pool
            .get_agent(agent_id)
            .await
            .map(|handle| AgentSnapshot::from_handle(&handle))
    }

    async fn record_agent_snapshot(&self, agent_id: &str) {
        if let Some(snapshot) = self.snapshot_agent(agent_id).await {
            let _ = self.repository.upsert(snapshot).await;
        }
    }

    async fn sync_repository(&self) -> Vec<AgentSnapshot> {
        let agents: Vec<AgentHandle> = self.pool.all_agents().await;
        let snapshots = agents
            .iter()
            .map(AgentSnapshot::from_handle)
            .collect::<Vec<_>>();
        let _ = self.repository.replace_all(snapshots.clone()).await;
        snapshots
    }
}

#[async_trait]
pub trait AgentLauncher: Send + Sync {
    async fn launch(&self, agent_id: &str) -> Result<AgentProcessHandle, AgentServiceError>;
}

pub enum AgentProcessHandle {
    Process(tokio::process::Child),
    Noop,
}

impl AgentProcessHandle {
    async fn shutdown(self, agent_id: &str) -> Result<(), AgentServiceError> {
        match self {
            AgentProcessHandle::Process(mut child) => child
                .kill()
                .await
                .map_err(|e| AgentServiceError::shutdown_failed(agent_id, e.to_string())),
            AgentProcessHandle::Noop => Ok(()),
        }
    }
}

struct ProcessLauncher {
    command: String,
    args: Vec<String>,
    working_dir: Option<String>,
    env: Vec<(String, String)>,
}

impl ProcessLauncher {
    fn new(command: String, args: Vec<String>, config: &AgentServiceConfig) -> Self {
        Self {
            command,
            args,
            working_dir: config.working_dir.clone(),
            env: config.env.clone(),
        }
    }
}

#[async_trait]
impl AgentLauncher for ProcessLauncher {
    async fn launch(&self, agent_id: &str) -> Result<AgentProcessHandle, AgentServiceError> {
        let mut command = Command::new(&self.command);
        command.args(&self.args);
        command.env("OYA_AGENT_ID", agent_id);

        if let Some(dir) = &self.working_dir {
            command.current_dir(dir);
        }

        for (key, value) in &self.env {
            command.env(key, value);
        }

        command.stdin(Stdio::null());
        command.stdout(Stdio::null());
        command.stderr(Stdio::null());

        let child = command
            .spawn()
            .map_err(|e| AgentServiceError::spawn_failed(e.to_string()))?;

        Ok(AgentProcessHandle::Process(child))
    }
}

struct ZjjLauncher {
    command: String,
    args: Vec<String>,
    working_dir: Option<String>,
    env: Vec<(String, String)>,
}

impl ZjjLauncher {
    fn new(config: &AgentServiceConfig) -> Self {
        Self {
            command: config.command.clone(),
            args: config.args.clone(),
            working_dir: config.working_dir.clone(),
            env: config.env.clone(),
        }
    }
}

#[async_trait]
impl AgentLauncher for ZjjLauncher {
    async fn launch(&self, agent_id: &str) -> Result<AgentProcessHandle, AgentServiceError> {
        let mut command = Command::new(&self.command);
        command.arg("add").arg(agent_id).args(&self.args);
        command.env("OYA_AGENT_ID", agent_id);

        if let Some(dir) = &self.working_dir {
            command.current_dir(dir);
        }

        for (key, value) in &self.env {
            command.env(key, value);
        }

        command.stdin(Stdio::null());
        command.stdout(Stdio::null());
        command.stderr(Stdio::null());

        let child = command
            .spawn()
            .map_err(|e| AgentServiceError::spawn_failed(e.to_string()))?;

        Ok(AgentProcessHandle::Process(child))
    }
}

#[derive(Debug, thiserror::Error)]
pub enum AgentServiceError {
    #[error("invalid agent count: {count}")]
    InvalidCount { count: usize },
    #[error("agent pool capacity exceeded (current {current}, max {max})")]
    CapacityExceeded { current: usize, max: usize },
    #[error("spawn failed: {reason}")]
    SpawnFailed { reason: String },
    #[error("agent not found: {agent_id}")]
    AgentNotFound { agent_id: String },
    #[error("not enough idle agents to scale down (requested {requested}, available {available})")]
    InsufficientIdle { requested: usize, available: usize },
    #[error("agent process missing for {agent_id}")]
    ProcessMissing { agent_id: String },
    #[error("shutdown failed for {agent_id}: {reason}")]
    ShutdownFailed { agent_id: String, reason: String },
    #[error("invalid environment value: {key}={value}")]
    InvalidEnvValue { key: String, value: String },
    #[error("invalid agent runner: {runner}")]
    InvalidRunner { runner: String },
    #[error("missing command for runner {runner}")]
    MissingCommand { runner: String },
}

impl AgentServiceError {
    fn spawn_failed(reason: impl Into<String>) -> Self {
        Self::SpawnFailed {
            reason: reason.into(),
        }
    }

    fn shutdown_failed(agent_id: &str, reason: impl Into<String>) -> Self {
        Self::ShutdownFailed {
            agent_id: agent_id.to_string(),
            reason: reason.into(),
        }
    }

    fn process_missing(agent_id: &str) -> Self {
        Self::ProcessMissing {
            agent_id: agent_id.to_string(),
        }
    }
}

impl From<AgentServiceError> for AppError {
    fn from(err: AgentServiceError) -> Self {
        match err {
            AgentServiceError::InvalidCount { .. } => AppError::BadRequest(err.to_string()),
            AgentServiceError::CapacityExceeded { .. } => AppError::Conflict(err.to_string()),
            AgentServiceError::InsufficientIdle { .. } => AppError::Conflict(err.to_string()),
            AgentServiceError::AgentNotFound { .. } => AppError::NotFound(err.to_string()),
            AgentServiceError::InvalidEnvValue { .. } => AppError::BadRequest(err.to_string()),
            AgentServiceError::InvalidRunner { .. } => AppError::BadRequest(err.to_string()),
            AgentServiceError::MissingCommand { .. } => AppError::BadRequest(err.to_string()),
            _ => AppError::Internal(err.to_string()),
        }
    }
}

fn build_launcher(config: &AgentServiceConfig) -> Arc<dyn AgentLauncher> {
    match config.runner {
        AgentRunner::Opencode => Arc::new(ProcessLauncher::new(
            config.command.clone(),
            config.args.clone(),
            config,
        )),
        AgentRunner::Zjj => Arc::new(ZjjLauncher::new(config)),
        AgentRunner::Custom => Arc::new(ProcessLauncher::new(
            config.command.clone(),
            config.args.clone(),
            config,
        )),
    }
}

fn parse_env_usize(key: &str, default: usize) -> Result<usize, AgentServiceError> {
    match env::var(key) {
        Ok(value) => value
            .parse::<usize>()
            .map_err(|_| AgentServiceError::InvalidEnvValue {
                key: key.to_string(),
                value,
            }),
        Err(_) => Ok(default),
    }
}

fn parse_env_args(key: &str) -> Vec<String> {
    env::var(key)
        .ok()
        .map(|value| {
            value
                .split_whitespace()
                .filter(|part| !part.is_empty())
                .map(|part| part.to_string())
                .collect()
        })
        .unwrap_or_default()
}

fn parse_env_pairs(key: &str) -> Vec<(String, String)> {
    env::var(key)
        .ok()
        .map(|value| {
            value
                .split(',')
                .filter_map(|pair| pair.split_once('='))
                .map(|(k, v)| (k.trim().to_string(), v.trim().to_string()))
                .filter(|(k, _)| !k.is_empty())
                .collect()
        })
        .unwrap_or_default()
}

fn parse_runner(value: Option<String>) -> Result<AgentRunner, AgentServiceError> {
    match value.as_deref() {
        None | Some("opencode") => Ok(AgentRunner::Opencode),
        Some("zjj") => Ok(AgentRunner::Zjj),
        Some("custom") => Ok(AgentRunner::Custom),
        Some(other) => Err(AgentServiceError::InvalidRunner {
            runner: other.to_string(),
        }),
    }
}

fn parse_command(runner: AgentRunner) -> Result<String, AgentServiceError> {
    if let Ok(command) = env::var("OYA_AGENT_COMMAND") {
        if !command.trim().is_empty() {
            return Ok(command);
        }
    }

    match runner {
        AgentRunner::Opencode => Ok("opencode".to_string()),
        AgentRunner::Zjj => Ok("zjj".to_string()),
        AgentRunner::Custom => Err(AgentServiceError::MissingCommand {
            runner: "custom".to_string(),
        }),
    }
}

fn validate_count(count: usize) -> Result<(), AgentServiceError> {
    if count == 0 {
        Err(AgentServiceError::InvalidCount { count })
    } else {
        Ok(())
    }
}

fn ensure_capacity(requested: usize, current: usize, max: usize) -> Result<(), AgentServiceError> {
    if requested > max.saturating_sub(current) {
        return Err(AgentServiceError::CapacityExceeded { current, max });
    }

    Ok(())
}

fn scale_summary(
    previous: usize,
    total: usize,
    spawned: Vec<String>,
    terminated: Vec<String>,
) -> AgentScaleSummary {
    AgentScaleSummary {
        previous,
        total,
        spawned,
        terminated,
    }
}

fn map_pool_error(error: AgentSwarmError) -> AgentServiceError {
    match error {
        AgentSwarmError::PoolCapacityExceeded { current, max } => {
            AgentServiceError::CapacityExceeded { current, max }
        }
        AgentSwarmError::AgentNotFound { agent_id } => {
            AgentServiceError::AgentNotFound { agent_id }
        }
        _ => AgentServiceError::SpawnFailed {
            reason: error.to_string(),
        },
    }
}

fn health_score_for(state: AgentStateLegacy) -> f64 {
    match state {
        AgentStateLegacy::Unhealthy | AgentStateLegacy::Terminated => 0.0,
        _ => 1.0,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    struct NoopLauncher;

    #[async_trait]
    impl AgentLauncher for NoopLauncher {
        async fn launch(&self, _agent_id: &str) -> Result<AgentProcessHandle, AgentServiceError> {
            Ok(AgentProcessHandle::Noop)
        }
    }

    fn test_config() -> AgentServiceConfig {
        AgentServiceConfig {
            max_agents: 3,
            runner: AgentRunner::Custom,
            command: "noop".to_string(),
            args: Vec::new(),
            working_dir: None,
            env: Vec::new(),
        }
    }

    #[tokio::test]
    async fn test_spawn_agents_registers_handles() -> Result<(), AgentServiceError> {
        let launcher = Arc::new(NoopLauncher);
        let service = AgentService::new_with_launcher(test_config(), launcher);

        let result = service.spawn_agents(2, Vec::new()).await?;
        assert_eq!(result.agent_ids.len(), 2);
        assert_eq!(result.total, 2);
        Ok(())
    }

    #[tokio::test]
    async fn test_scale_down_terminates_idle_agents() -> Result<(), AgentServiceError> {
        let launcher = Arc::new(NoopLauncher);
        let service = AgentService::new_with_launcher(test_config(), launcher);

        let _ = service.spawn_agents(2, Vec::new()).await?;
        let result = service.scale_to(1).await?;

        assert_eq!(result.terminated.len(), 1);
        assert_eq!(service.pool.len().await, 1);
        Ok(())
    }
}
