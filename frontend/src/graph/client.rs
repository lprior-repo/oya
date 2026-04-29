#![deny(clippy::unwrap_used)]
#![deny(clippy::expect_used)]
#![deny(clippy::panic)]
#![warn(clippy::pedantic)]
#![warn(clippy::nursery)]
#![forbid(unsafe_code)]

use reqwest::Client;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone)]
pub struct RestateClient {
    http_client: Client,
    base_url: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct KeyedState {
    pub service: String,
    pub key: String,
    pub value: serde_json::Value,
}

impl RestateClient {
    #[must_use]
    pub fn new(base_url: &str) -> Self {
        Self {
            http_client: Client::new(),
            base_url: base_url.trim_end_matches('/').to_string(),
        }
    }

    #[must_use]
    pub fn base_url(&self) -> &str {
        &self.base_url
    }

    /// Fetches keyed state from the Restate ingress endpoint.
    ///
    /// # Errors
    ///
    /// Returns `RestateClientError::RequestFailed` if the HTTP request fails.
    /// Returns `RestateClientError::ApiError` if the server responds with a non-success status.
    pub async fn get_keyed_state(
        &self,
        service_name: &str,
        key: &str,
    ) -> Result<KeyedState, RestateClientError> {
        let url = format!("{}/{}/{}", self.base_url, service_name, key);
        
        let response = self.http_client
            .get(&url)
            .send()
            .await
            .map_err(RestateClientError::RequestFailed)?;

        if !response.status().is_success() {
            let status = response.status().as_u16();
            let message = match response.text().await {
                Ok(text) => text,
                Err(e) => format!("<failed to read response body: {e}>"),
            };
            return Err(RestateClientError::ApiError { status, message });
        }

        let value: serde_json::Value = response.json().await.map_err(RestateClientError::RequestFailed)?;

        Ok(KeyedState {
            service: service_name.to_string(),
            key: key.to_string(),
            value,
        })
    }
}

#[derive(Debug, thiserror::Error)]
pub enum RestateClientError {
    #[error("Request failed: {0}")]
    RequestFailed(#[from] reqwest::Error),
    
    #[error("API error ({status}): {message}")]
    ApiError { status: u16, message: String },
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used, clippy::panic, clippy::float_cmp)]
mod tests {
    use super::*;
    use crate::canonical_ports::default_ingress_url;

    #[test]
    fn client_creates_with_valid_url() {
        let url = default_ingress_url();
        let client = RestateClient::new(&url);
        assert_eq!(client.base_url(), url);
    }

    #[test]
    fn client_trims_trailing_slash() {
        let url = default_ingress_url();
        let client = RestateClient::new(&format!("{url}/"));
        assert_eq!(client.base_url(), url);
    }

    #[test]
    fn keyed_state_clone_works() {
        let state1 = KeyedState {
            service: "test".to_string(),
            key: "test".to_string(),
            value: serde_json::json!({}),
        };
        let state2 = state1.clone();
        assert_eq!(state1, state2);
    }
}
