use crate::lifecycle::types::Model;
use restate_sdk::prelude::{HandlerError, TerminalError};
use serde_json::Value;
use tokio::process::Command;

#[derive(Debug, Clone)]
pub struct Prompt(String);

impl Prompt {
    pub fn parse(raw: String) -> Result<Self, TerminalError> {
        let normalized = raw.trim().to_owned();
        if normalized.is_empty() {
            return Err(TerminalError::new("prompt cannot be empty"));
        }
        Ok(Self(normalized))
    }

    pub fn into_inner(self) -> String {
        self.0
    }
}

pub fn pipeline_prompt(bead_id: &str, bead_state: Value) -> Result<Prompt, TerminalError> {
    let state_json = serde_json::to_string_pretty(&bead_state)
        .map_err(|error| TerminalError::new(format!("invalid bead_state json: {error}")))?;
    Prompt::parse(format!(
        "Implement bead {bead_id} using this state from Restate.\n\nBead State:\n{state_json}\n\nSteps: 1) implement requested changes in repo, 2) run moon run :check, 3) summarize files changed and test result."
    ))
}

#[must_use]
pub fn model_or_default(value: Option<String>) -> Model {
    value.and_then(|m| Model::parse(&m).ok()).unwrap_or_else(Model::default_model)
}

pub async fn run_opencode(prompt: Prompt, model: Model) -> Result<String, HandlerError> {
    let output = Command::new("opencode")
        .arg("run")
        .arg("--format")
        .arg("json")
        .arg("--model")
        .arg(model.as_str())
        .arg(prompt.into_inner())
        .output()
        .await
        .map_err(|error| HandlerError::from(format!("failed to run opencode: {error}")))?;
    parse_output(output)
}

fn parse_output(output: std::process::Output) -> Result<String, HandlerError> {
    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr).into_owned();
        let message = format!("opencode failed: {}", stderr.trim());
        return Err(TerminalError::new(message).into());
    }
    String::from_utf8(output.stdout).map_err(|error| {
        TerminalError::new(format!("opencode output was not UTF-8: {error}")).into()
    })
}

pub async fn cancel_invocation(invocation_id: String) -> Result<(), HandlerError> {
    let output = Command::new("restate")
        .arg("invocations")
        .arg("cancel")
        .arg(&invocation_id)
        .output()
        .await
        .map_err(|error| HandlerError::from(format!("failed to invoke restate CLI: {error}")))?;
    if output.status.success() {
        Ok(())
    } else {
        let stderr = String::from_utf8_lossy(&output.stderr).into_owned();
        Err(TerminalError::new(format!(
            "restate cancel failed for {invocation_id}: {}",
            stderr.trim()
        ))
        .into())
    }
}

pub async fn cancel_invocation_query(query: String) -> Result<String, HandlerError> {
    let output = Command::new("restate")
        .arg("invocations")
        .arg("cancel")
        .arg(&query)
        .arg("--kill")
        .arg("-y")
        .output()
        .await
        .map_err(|error| HandlerError::from(format!("failed to invoke restate CLI: {error}")))?;
    let stdout = String::from_utf8_lossy(&output.stdout).into_owned();
    if output.status.success() {
        Ok(format!("cancelled workflow query {query}"))
    } else {
        let stderr = String::from_utf8_lossy(&output.stderr).into_owned();
        if stderr.contains("No invocations found for query")
            || stdout.contains("No invocations found for query")
        {
            Ok(format!("no running workflow invocations for {query}"))
        } else {
            Err(TerminalError::new(format!(
                "restate cancel failed for query {query}: {}",
                stderr.trim()
            ))
            .into())
        }
    }
}
