//! Restate SQL query client for fetching invocation data.

#![deny(clippy::unwrap_used)]
#![deny(clippy::expect_used)]
#![deny(clippy::panic)]
#![forbid(unsafe_code)]

use super::types::{RestateInvocationRow, RestateQueryResponse};
use anyhow::Result;
use reqwest::Client;
use serde_json::json;

const DEFAULT_RESTATE_ADMIN_URL: &str = "http://127.0.0.1:9070";

/// Resolve Restate admin URL from environment or default.
fn resolve_restate_admin_url() -> String {
    std::env::var("OYA_RESTATE_ADMIN_URL")
        .map_or_else(|_| DEFAULT_RESTATE_ADMIN_URL.to_string(), std::convert::identity)
}

/// Fetch OyaOrchestrator invocations from Restate.
pub async fn fetch_invocations(client: &Client, limit: usize) -> Result<Vec<RestateInvocationRow>> {
    let url = resolve_restate_admin_url();

    let query = format!(
        "SELECT target_service_key, status, completion_result, completion_failure, modified_at \
         FROM sys_invocation \
         WHERE target_service_name = 'OyaOrchestrator' \
         ORDER BY modified_at DESC \
         LIMIT {};",
        limit
    );

    let response = client
        .post(format!("{}/query", url))
        .json(&json!({ "query": query }))
        .send()
        .await?
        .json::<RestateQueryResponse>()
        .await?;

    Ok(response.rows)
}
