#![deny(clippy::unwrap_used)]
#![deny(clippy::expect_used)]
#![deny(clippy::panic)]
#![forbid(unsafe_code)]

use restate_sdk::prelude::HandlerError;

pub async fn forget_workspace_for_targets(targets: Vec<String>) -> Result<String, HandlerError> {
    let mut messages = Vec::new();
    for target in targets {
        messages.push(forget_workspace_for_key(target).await?);
    }
    Ok(messages.join("; "))
}

async fn forget_workspace_for_key(key: String) -> Result<String, HandlerError> {
    let workspace = format!("oya-{key}");
    let output = tokio::process::Command::new("jj")
        .arg("workspace")
        .arg("forget")
        .arg(&workspace)
        .output()
        .await
        .map_err(|error| {
            HandlerError::from(format!("failed to run jj workspace forget: {error}"))
        })?;
    if output.status.success() {
        Ok(format!("workspace cleanup attempted for {workspace}"))
    } else {
        let stderr = String::from_utf8_lossy(&output.stderr).into_owned();
        if stderr.contains("No such workspace") {
            Ok(format!("workspace {workspace} not present"))
        } else {
            Err(HandlerError::from(format!("workspace cleanup failed: {}", stderr.trim())))
        }
    }
}
