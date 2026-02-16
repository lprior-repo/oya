use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::process::Command;
use thiserror::Error;

#[derive(Debug, Error)]
#[allow(dead_code)]
pub enum OpenCodeError {
    #[error("HTTP error: {0}")]
    HttpError(#[from] reqwest::Error),
    #[error("Parse error: {0}")]
    ParseError(String),
    #[error("Timeout")]
    Timeout,
    #[error("Permission pending: {0}")]
    PermissionPending(String),
    #[error("Question pending: {0}")]
    QuestionPending(String),
    #[error("CLI error: {0}")]
    CliError(String),
}
// ... (rest of the file remains, but I will append the new struct)

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Completion {
    pub status: String,
    pub text: Option<String>,
    pub completed: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionResponse {
    pub id: String,
}

#[allow(dead_code)]
#[derive(Debug, Clone, Serialize)]
struct ModelSpec {
    #[serde(rename = "providerID")]
    provider_id: String,
    #[serde(rename = "modelID")]
    model_id: String,
}

#[allow(dead_code)]
#[derive(Debug, Clone, Serialize)]
struct MessagePart {
    #[serde(rename = "type")]
    part_type: String,
    text: String,
}

#[allow(dead_code)]
#[derive(Debug, Clone, Serialize)]
struct MessageRequest {
    model: ModelSpec,
    agent: String,
    parts: Vec<MessagePart>,
}

#[allow(dead_code)]
#[derive(Clone)]
pub struct OpenCodeClient {
    base_url: String,
    client: Client,
}

#[allow(dead_code)]
impl OpenCodeClient {
    pub fn new(base_url: &str) -> Self {
        Self { base_url: base_url.trim_end_matches('/').to_string(), client: Client::new() }
    }

    pub async fn create_session(&self) -> Result<String, OpenCodeError> {
        let url = format!("{}/session", self.base_url);
        let response: SessionResponse = self
            .client
            .post(&url)
            .send()
            .await?
            .json()
            .await
            .map_err(|e| OpenCodeError::ParseError(e.to_string()))?;
        Ok(response.id)
    }

    pub async fn send_prompt(
        &self,
        session_id: &str,
        model_provider: &str,
        model_id: &str,
        agent: &str,
        prompt: &str,
    ) -> Result<(), OpenCodeError> {
        let url = format!("{}/session/{}/message", self.base_url, session_id);
        let request = MessageRequest {
            model: ModelSpec {
                provider_id: model_provider.to_string(),
                model_id: model_id.to_string(),
            },
            agent: agent.to_string(),
            parts: vec![MessagePart { part_type: "text".to_string(), text: prompt.to_string() }],
        };
        self.client
            .post(&url)
            .json(&request)
            .send()
            .await?
            .error_for_status()
            .map_err(OpenCodeError::from)?;
        Ok(())
    }

    pub async fn poll_completion(&self, session_id: &str) -> Result<Completion, OpenCodeError> {
        let url = format!("{}/session/{}/message?limit=1", self.base_url, session_id);
        let completion: Completion = self
            .client
            .get(&url)
            .send()
            .await?
            .json()
            .await
            .map_err(|e| OpenCodeError::ParseError(e.to_string()))?;
        Ok(completion)
    }

    pub async fn check_permissions(&self) -> Result<Vec<String>, OpenCodeError> {
        let url = format!("{}/permission", self.base_url);
        let permissions: Vec<String> = self
            .client
            .get(&url)
            .send()
            .await?
            .json()
            .await
            .map_err(|e| OpenCodeError::ParseError(e.to_string()))?;
        Ok(permissions)
    }

    pub async fn reply_permission(
        &self,
        permission_id: &str,
        response: &str,
    ) -> Result<(), OpenCodeError> {
        let url = format!("{}/permission/{}/reply", self.base_url, permission_id);
        self.client
            .post(&url)
            .json(&serde_json::json!({ "response": response }))
            .send()
            .await?
            .error_for_status()
            .map_err(OpenCodeError::from)?;
        Ok(())
    }

    pub async fn check_questions(&self) -> Result<Vec<String>, OpenCodeError> {
        let url = format!("{}/question", self.base_url);
        let questions: Vec<String> = self
            .client
            .get(&url)
            .send()
            .await?
            .json()
            .await
            .map_err(|e| OpenCodeError::ParseError(e.to_string()))?;
        Ok(questions)
    }

    pub async fn reply_question(
        &self,
        question_id: &str,
        response: &str,
    ) -> Result<(), OpenCodeError> {
        let url = format!("{}/question/{}/reply", self.base_url, question_id);
        self.client
            .post(&url)
            .json(&serde_json::json!({ "response": response }))
            .send()
            .await?
            .error_for_status()
            .map_err(OpenCodeError::from)?;
        Ok(())
    }

    pub async fn health_check(&self) -> Result<bool, OpenCodeError> {
        let url = format!("{}/global/health", self.base_url);
        let response = self.client.get(&url).send().await?;
        Ok(response.status().is_success())
    }
}

#[allow(dead_code)]
pub struct OpenCodeCliClient {
    binary_path: String,
}

#[allow(dead_code)]
impl OpenCodeCliClient {
    pub fn new() -> Self {
        Self { binary_path: "opencode".to_string() }
    }

    pub async fn run_prompt(
        &self,
        model: &str,
        agent: &str,
        prompt: &str,
    ) -> Result<String, OpenCodeError> {
        let mut cmd = Command::new(&self.binary_path);
        cmd.arg("run")
            .arg("--model")
            .arg(model)
            .arg("--agent")
            .arg(agent)
            .arg("--format")
            .arg("json")
            .arg(prompt);

        let output = tokio::task::spawn_blocking(move || cmd.output())
            .await
            .map_err(|e| OpenCodeError::CliError(e.to_string()))?
            .map_err(|e| OpenCodeError::CliError(e.to_string()))?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(OpenCodeError::CliError(stderr.to_string()));
        }

        let stdout = String::from_utf8_lossy(&output.stdout);
        Ok(stdout.to_string())
    }
}
