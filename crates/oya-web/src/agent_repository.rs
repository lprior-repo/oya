//! Agent repository layer for persistence.

use std::collections::HashMap;
use std::sync::Arc;

use async_trait::async_trait;
use tokio::sync::RwLock;

#[derive(Debug, Clone, PartialEq)]
pub struct AgentSnapshot {
    pub id: String,
    pub status: String,
    pub current_bead: Option<String>,
    pub health_score: f64,
    pub uptime_secs: u64,
    pub capabilities: Vec<String>,
}

#[derive(Debug, thiserror::Error)]
pub enum AgentRepositoryError {
    #[error("repository error: {reason}")]
    Repository { reason: String },
}

#[async_trait]
pub trait AgentRepository: Send + Sync {
    async fn upsert(&self, agent: AgentSnapshot) -> Result<(), AgentRepositoryError>;
    async fn remove(&self, agent_id: &str) -> Result<(), AgentRepositoryError>;
    async fn list(&self) -> Result<Vec<AgentSnapshot>, AgentRepositoryError>;
    async fn replace_all(&self, agents: Vec<AgentSnapshot>) -> Result<(), AgentRepositoryError>;
}

#[derive(Debug, Default)]
pub struct InMemoryAgentRepository {
    agents: Arc<RwLock<HashMap<String, AgentSnapshot>>>,
}

impl InMemoryAgentRepository {
    pub fn new() -> Self {
        Self::default()
    }
}

#[async_trait]
impl AgentRepository for InMemoryAgentRepository {
    async fn upsert(&self, agent: AgentSnapshot) -> Result<(), AgentRepositoryError> {
        self.agents.write().await.insert(agent.id.clone(), agent);
        Ok(())
    }

    async fn remove(&self, agent_id: &str) -> Result<(), AgentRepositoryError> {
        self.agents.write().await.remove(agent_id);
        Ok(())
    }

    async fn list(&self) -> Result<Vec<AgentSnapshot>, AgentRepositoryError> {
        let agents = self.agents.read().await;
        Ok(agents.values().cloned().collect())
    }

    async fn replace_all(&self, agents: Vec<AgentSnapshot>) -> Result<(), AgentRepositoryError> {
        let mut store = self.agents.write().await;
        store.clear();
        for agent in agents {
            store.insert(agent.id.clone(), agent);
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_snapshot(id: &str) -> AgentSnapshot {
        AgentSnapshot {
            id: id.to_string(),
            status: "idle".to_string(),
            current_bead: None,
            health_score: 1.0,
            uptime_secs: 10,
            capabilities: vec!["rust".to_string()],
        }
    }

    #[tokio::test]
    async fn test_upsert_and_list() -> Result<(), AgentRepositoryError> {
        let repo = InMemoryAgentRepository::new();
        let agent = sample_snapshot("agent-1");

        repo.upsert(agent.clone()).await?;

        let agents = repo.list().await?;
        assert_eq!(agents.len(), 1);
        assert_eq!(agents[0], agent);
        Ok(())
    }

    #[tokio::test]
    async fn test_replace_all_overwrites() -> Result<(), AgentRepositoryError> {
        let repo = InMemoryAgentRepository::new();
        let first = sample_snapshot("agent-1");
        let second = sample_snapshot("agent-2");

        repo.upsert(first).await?;
        repo.replace_all(vec![second.clone()]).await?;

        let agents = repo.list().await?;
        assert_eq!(agents.len(), 1);
        assert_eq!(agents[0], second);
        Ok(())
    }

    #[tokio::test]
    async fn test_remove_agent() -> Result<(), AgentRepositoryError> {
        let repo = InMemoryAgentRepository::new();
        let agent = sample_snapshot("agent-1");

        repo.upsert(agent).await?;
        repo.remove("agent-1").await?;

        let agents = repo.list().await?;
        assert!(agents.is_empty());
        Ok(())
    }
}
