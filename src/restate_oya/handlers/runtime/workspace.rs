#![deny(clippy::unwrap_used)]
#![deny(clippy::expect_used)]
#![deny(clippy::panic)]
#![warn(clippy::pedantic)]
#![forbid(unsafe_code)]

use futures_util::future::try_join_all;
use restate_sdk::prelude::HandlerError;

#[derive(Clone)]
struct WorkspaceName(String);

impl WorkspaceName {
    fn from_key(key: &str) -> Self {
        Self(format!("oya-{key}"))
    }

    fn as_str(&self) -> &str {
        &self.0
    }

    fn into_string(self) -> String {
        self.0
    }
}

pub async fn forget_workspace_for_targets(targets: Vec<String>) -> Result<String, HandlerError> {
    let messages =
        try_join_all(targets.iter().map(String::as_str).map(forget_workspace_for_key)).await?;
    Ok(messages.join("; "))
}

async fn forget_workspace_for_key(key: &str) -> Result<String, HandlerError> {
    let workspace = WorkspaceName::from_key(key);
    let output = tokio::process::Command::new("jj")
        .arg("workspace")
        .arg("forget")
        .arg(workspace.as_str())
        .output()
        .await
        .map_err(|error| {
            HandlerError::from(format!("failed to run jj workspace forget: {error}"))
        })?;
    if output.status.success() {
        Ok(format!("workspace cleanup attempted for {}", workspace.as_str()))
    } else {
        let stderr = String::from_utf8_lossy(&output.stderr).into_owned();
        if stderr.contains("No such workspace") {
            Ok(format!("workspace {} not present", workspace.into_string()))
        } else {
            Err(HandlerError::from(format!("workspace cleanup failed: {}", stderr.trim())))
        }
    }
}
