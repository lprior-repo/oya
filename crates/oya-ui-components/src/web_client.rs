//! HTTP web client with graceful error handling
//!
//! Provides a type-safe HTTP client for making requests to the oya-web API
//! with comprehensive error handling for network issues, timeouts, and HTTP errors.

use std::collections::BTreeMap;
use std::sync::Arc;
use std::time::Duration;

use reqwest::Client;
use serde::Deserialize;
use thiserror::Error;
use tracing::{debug, error, info, warn};

use crate::correlation::{CorrelationContext, RequestId};

/// HTTP client configuration
#[derive(Debug, Clone)]
pub struct WebClientConfig {
    /// Base URL for API requests
    pub base_url: String,
    /// Request timeout
    pub timeout: Duration,
    /// Maximum number of retries
    pub max_retries: u32,
    /// Retry delay
    pub retry_delay: Duration,
}

impl Default for WebClientConfig {
    fn default() -> Self {
        Self {
            base_url: "http://127.0.0.1:3000".to_string(),
            timeout: Duration::from_secs(30),
            max_retries: 3,
            retry_delay: Duration::from_secs(1),
        }
    }
}

/// HTTP client errors with graceful handling
#[derive(Debug, Error, Clone, PartialEq)]
pub enum WebClientError {
    /// Network connection error
    #[error("Network error: {message}")]
    Network { message: String },

    /// Request timeout
    #[error("Request timeout after {seconds}s")]
    Timeout { seconds: u64 },

    /// HTTP error response (4xx or 5xx)
    #[error("HTTP {status}: {message}")]
    Http { status: u16, message: String },

    /// Invalid response body
    #[error("Invalid response: {message}")]
    InvalidResponse { message: String },

    /// Rate limited (429)
    #[error("Rate limited: retry after {seconds}s")]
    RateLimited { seconds: u64 },

    /// Service unavailable (503)
    #[error("Service unavailable")]
    ServiceUnavailable,

    /// Connection refused
    #[error("Connection refused to {address}")]
    ConnectionRefused { address: String },

    /// DNS resolution failed
    #[error("DNS resolution failed for {host}")]
    DnsFailed { host: String },

    /// SSL/TLS error
    #[error("TLS error: {message}")]
    Tls { message: String },
}

impl WebClientError {
    /// Check if error is retryable
    pub fn is_retryable(&self) -> bool {
        match self {
            Self::Network { .. }
            | Self::Timeout { .. }
            | Self::RateLimited { .. }
            | Self::ServiceUnavailable
            | Self::ConnectionRefused { .. } => true,

            Self::Http { status, .. } => matches!(status, 500..=599 | 429),
            Self::DnsFailed { .. } | Self::Tls { .. } | Self::InvalidResponse { .. } => false,
        }
    }

    /// Get user-friendly error message
    pub fn user_message(&self) -> String {
        match self {
            Self::Network { message } => {
                format!("Network error: Unable to reach server. {}", message)
            }
            Self::Timeout { seconds } => {
                format!(
                    "Request timed out after {} seconds. Please try again.",
                    seconds
                )
            }
            Self::Http { status, message } => {
                if *status >= 500 {
                    format!(
                        "Server error ({}): The server is having problems. {}",
                        status, message
                    )
                } else if *status == 404 {
                    format!(
                        "Not found: The requested resource was not found. {}",
                        message
                    )
                } else if *status == 401 {
                    format!("Unauthorized: Authentication required. {}", message)
                } else if *status == 403 {
                    format!(
                        "Forbidden: You don't have permission to access this resource. {}",
                        message
                    )
                } else if *status == 429 {
                    format!(
                        "Too many requests: Please wait and try again later. {}",
                        message
                    )
                } else {
                    format!("HTTP error ({}): {}", status, message)
                }
            }
            Self::InvalidResponse { message } => {
                format!("Invalid response from server: {}", message)
            }
            Self::RateLimited { seconds } => {
                format!(
                    "Rate limited. Please wait {} seconds and try again.",
                    seconds
                )
            }
            Self::ServiceUnavailable => {
                "Service temporarily unavailable. Please try again later.".to_string()
            }
            Self::ConnectionRefused { address } => {
                format!("Connection refused. Is the server running at {}?", address)
            }
            Self::DnsFailed { host } => {
                format!(
                    "Cannot resolve server address: {}. Check your network connection.",
                    host
                )
            }
            Self::Tls { message } => {
                format!(
                    "Secure connection failed: {}. Check your system clock.",
                    message
                )
            }
        }
    }

    /// Convert from reqwest error
    fn from_reqwest(error: &reqwest::Error) -> Self {
        if error.is_timeout() {
            Self::Timeout {
                seconds: 30, // Default timeout from config
            }
        } else if error.is_connect() {
            if let Some(url) = error.url() {
                Self::ConnectionRefused {
                    address: url.to_string(),
                }
            } else {
                Self::Network {
                    message: "Connection failed".to_string(),
                }
            }
        } else if error.is_request() {
            if let Some(status) = error.status() {
                let status_code = status.as_u16();
                let message = error
                    .to_string()
                    .lines()
                    .next()
                    .unwrap_or("Unknown error")
                    .to_string();

                if status_code == 429 {
                    Self::RateLimited { seconds: 60 }
                } else if status_code == 503 {
                    Self::ServiceUnavailable
                } else {
                    Self::Http {
                        status: status_code,
                        message,
                    }
                }
            } else {
                Self::Network {
                    message: error.to_string(),
                }
            }
        } else if error.is_body() || error.is_decode() {
            Self::InvalidResponse {
                message: error.to_string(),
            }
        } else {
            Self::Network {
                message: error.to_string(),
            }
        }
    }
}

/// HTTP response with metadata
#[derive(Debug, Clone)]
pub struct HttpResponse<T> {
    /// Response body
    pub body: T,
    /// HTTP status code
    pub status: u16,
    /// Response headers
    pub headers: BTreeMap<String, String>,
    /// Request ID for correlation
    pub request_id: RequestId,
    /// Duration of the request in milliseconds
    pub duration_ms: u64,
}

impl<T> HttpResponse<T> {
    /// Create a new HTTP response
    pub fn new(
        body: T,
        status: u16,
        headers: BTreeMap<String, String>,
        request_id: RequestId,
        duration_ms: u64,
    ) -> Self {
        Self {
            body,
            status,
            headers,
            request_id,
            duration_ms,
        }
    }

    /// Map the response body
    pub fn map<U, F>(self, f: F) -> HttpResponse<U>
    where
        F: FnOnce(T) -> U,
    {
        HttpResponse {
            body: f(self.body),
            status: self.status,
            headers: self.headers,
            request_id: self.request_id,
            duration_ms: self.duration_ms,
        }
    }

    /// Check if response is successful (2xx)
    pub fn is_success(&self) -> bool {
        (200..300).contains(&self.status)
    }
}

/// HTTP web client with graceful error handling
#[derive(Debug, Clone)]
pub struct WebClient {
    /// Reqwest client for making HTTP requests
    client: Arc<Client>,
    /// Client configuration
    config: Arc<WebClientConfig>,
}

impl WebClient {
    /// Create a new web client
    ///
    /// # Errors
    ///
    /// Returns an error if the reqwest client cannot be created.
    pub fn new(config: WebClientConfig) -> Result<Self, WebClientError> {
        let client = Client::builder()
            .timeout(config.timeout)
            .build()
            .map_err(|e| WebClientError::Network {
                message: format!("Failed to create HTTP client: {}", e),
            })?;

        Ok(Self {
            client: Arc::new(client),
            config: Arc::new(config),
        })
    }

    /// Create a new web client with default configuration
    ///
    /// # Errors
    ///
    /// Returns an error if the reqwest client cannot be created.
    pub fn with_base_url(base_url: String) -> Result<Self, WebClientError> {
        let config = WebClientConfig {
            base_url,
            ..Default::default()
        };
        Self::new(config)
    }

    /// Make a GET request with correlation context
    ///
    /// # Errors
    ///
    /// Returns a `WebClientError` if the request fails after all retries.
    pub async fn get<T: for<'de> Deserialize<'de>>(
        &self,
        path: &str,
        context: &CorrelationContext,
    ) -> Result<HttpResponse<T>, WebClientError> {
        let mut retries_left = self.config.max_retries;

        loop {
            let url = format!("{}{}", self.config.base_url, path);
            let start = std::time::Instant::now();

            debug!(
                request_id = %context.request_id().as_str(),
                url = %url,
                retries_left,
                "Making GET request"
            );

            let mut request = self.client.get(&url);

            // Add correlation headers
            request = request.header("X-Request-Id", context.request_id().as_str());
            for (key, value) in context.metadata() {
                request = request.header(format!("X-Ctx-{}", key), value);
            }

            let response = request.send().await;

            let duration_ms = start.elapsed().as_millis() as u64;

            match response {
                Ok(resp) => {
                    let status = resp.status();
                    let status_code = status.as_u16();

                    debug!(
                        request_id = %context.request_id().as_str(),
                        status = %status_code,
                        duration_ms,
                        "Received response"
                    );

                    // Build headers map
                    let headers = resp
                        .headers()
                        .iter()
                        .map(|(k, v)| {
                            (k.as_str().to_string(), v.to_str().unwrap_or("").to_string())
                        })
                        .collect();

                    if !status.is_success() {
                        let error_body = resp
                            .text()
                            .await
                            .unwrap_or_else(|_| "<unreadable>".to_string());

                        let error = if status_code == 429 {
                            WebClientError::RateLimited { seconds: 60 }
                        } else if status_code == 503 {
                            WebClientError::ServiceUnavailable
                        } else {
                            WebClientError::Http {
                                status: status_code,
                                message: error_body.clone(),
                            }
                        };

                        // Log warning for client errors, error for server errors
                        if (400..500).contains(&status_code) {
                            warn!(
                                request_id = %context.request_id().as_str(),
                                status = %status_code,
                                message = %error_body,
                                "HTTP client error"
                            );
                        } else {
                            error!(
                                request_id = %context.request_id().as_str(),
                                status = %status_code,
                                message = %error_body,
                                "HTTP server error"
                            );
                        }

                        // Retry on retryable errors
                        if error.is_retryable() && retries_left > 0 {
                            info!(
                                request_id = %context.request_id().as_str(),
                                retries_left,
                                "Retrying request after error"
                            );
                            retries_left -= 1;
                            tokio::time::sleep(self.config.retry_delay).await;
                            continue;
                        }

                        return Err(error);
                    }

                    match resp.json().await {
                        Ok(body) => {
                            info!(
                                request_id = %context.request_id().as_str(),
                                status = %status_code,
                                duration_ms,
                                "Request successful"
                            );

                            return Ok(HttpResponse::new(
                                body,
                                status_code,
                                headers,
                                context.request_id().clone(),
                                duration_ms,
                            ));
                        }
                        Err(e) => {
                            let error = WebClientError::InvalidResponse {
                                message: format!("Failed to parse response: {}", e),
                            };

                            error!(
                                request_id = %context.request_id().as_str(),
                                error = %error,
                                "Failed to parse response"
                            );

                            return Err(error);
                        }
                    }
                }
                Err(e) => {
                    let error = WebClientError::from_reqwest(&e);

                    warn!(
                        request_id = %context.request_id().as_str(),
                        error = %error,
                        "HTTP request failed"
                    );

                    // Retry on retryable errors
                    if error.is_retryable() && retries_left > 0 {
                        info!(
                            request_id = %context.request_id().as_str(),
                            retries_left,
                            "Retrying request after network error"
                        );
                        retries_left -= 1;
                        tokio::time::sleep(self.config.retry_delay).await;
                        continue;
                    }

                    return Err(error);
                }
            }
        }
    }

    /// Health check for the web API
    ///
    /// # Errors
    ///
    /// Returns a `WebClientError` if the health check fails.
    pub async fn health_check(&self) -> Result<(), WebClientError> {
        let url = format!("{}/health", self.config.base_url);

        debug!("Health check: {}", url);

        let response = self.client.get(&url).send().await;

        match response {
            Ok(resp) => {
                let status = resp.status();
                if status.is_success() {
                    info!("Health check passed");
                    Ok(())
                } else {
                    warn!(status = %status.as_u16(), "Health check failed");
                    Err(WebClientError::Http {
                        status: status.as_u16(),
                        message: "Health check failed".to_string(),
                    })
                }
            }
            Err(e) => {
                let error = WebClientError::from_reqwest(&e);
                warn!(error = %error, "Health check failed");
                Err(error)
            }
        }
    }
}

#[cfg(test)]
#[allow(clippy::panic)]
mod tests {
    use super::*;

    #[test]
    fn test_web_client_config_default() {
        let config = WebClientConfig::default();

        assert_eq!(config.base_url, "http://127.0.0.1:3000");
        assert_eq!(config.timeout, Duration::from_secs(30));
        assert_eq!(config.max_retries, 3);
        assert_eq!(config.retry_delay, Duration::from_secs(1));
    }

    #[test]
    fn test_web_client_error_network() {
        let error = WebClientError::Network {
            message: "Connection failed".to_string(),
        };

        assert!(error.to_string().contains("Network error"));
        assert!(error.is_retryable());

        let user_msg = error.user_message();
        assert!(user_msg.contains("Network error"));
    }

    #[test]
    fn test_web_client_error_timeout() {
        let error = WebClientError::Timeout { seconds: 30 };

        assert!(error.to_string().contains("timeout"));
        assert!(error.to_string().contains("30"));
        assert!(error.is_retryable());

        let user_msg = error.user_message();
        assert!(user_msg.contains("timed out"));
        assert!(user_msg.contains("30 seconds"));
    }

    #[test]
    fn test_web_client_error_http_404() {
        let error = WebClientError::Http {
            status: 404,
            message: "Not found".to_string(),
        };

        assert!(error.to_string().contains("404"));
        assert!(!error.is_retryable()); // 4xx errors are not retryable

        let user_msg = error.user_message();
        assert!(user_msg.contains("Not found"));
    }

    #[test]
    fn test_web_client_error_http_500() {
        let error = WebClientError::Http {
            status: 500,
            message: "Internal server error".to_string(),
        };

        assert!(error.to_string().contains("500"));
        assert!(error.is_retryable()); // 5xx errors are retryable

        let user_msg = error.user_message();
        assert!(user_msg.contains("Server error"));
    }

    #[test]
    fn test_web_client_error_http_429() {
        let error = WebClientError::Http {
            status: 429,
            message: "Too many requests".to_string(),
        };

        assert!(error.to_string().contains("429"));
        assert!(error.is_retryable()); // 429 is retryable

        let user_msg = error.user_message();
        assert!(user_msg.contains("Too many requests"));
    }

    #[test]
    fn test_web_client_error_rate_limited() {
        let error = WebClientError::RateLimited { seconds: 60 };

        assert!(error.to_string().contains("Rate limited"));
        assert!(error.is_retryable());

        let user_msg = error.user_message();
        assert!(user_msg.contains("60 seconds"));
    }

    #[test]
    fn test_web_client_error_service_unavailable() {
        let error = WebClientError::ServiceUnavailable;

        assert!(error.to_string().contains("Service unavailable"));
        assert!(error.is_retryable());

        let user_msg = error.user_message();
        assert!(user_msg.contains("temporarily unavailable"));
    }

    #[test]
    fn test_web_client_error_connection_refused() {
        let error = WebClientError::ConnectionRefused {
            address: "localhost:8080".to_string(),
        };

        assert!(error.to_string().contains("Connection refused"));
        assert!(error.is_retryable());

        let user_msg = error.user_message();
        assert!(user_msg.contains("localhost:8080"));
    }

    #[test]
    fn test_web_client_error_dns_failed() {
        let error = WebClientError::DnsFailed {
            host: "example.invalid".to_string(),
        };

        assert!(error.to_string().contains("DNS"));
        assert!(!error.is_retryable());

        let user_msg = error.user_message();
        assert!(user_msg.contains("example.invalid"));
    }

    #[test]
    fn test_web_client_error_tls() {
        let error = WebClientError::Tls {
            message: "Certificate expired".to_string(),
        };

        assert!(error.to_string().contains("TLS"));
        assert!(!error.is_retryable());

        let user_msg = error.user_message();
        assert!(user_msg.contains("Secure connection"));
    }

    #[test]
    fn test_web_client_error_invalid_response() {
        let error = WebClientError::InvalidResponse {
            message: "Invalid JSON".to_string(),
        };

        assert!(error.to_string().contains("Invalid response"));
        assert!(!error.is_retryable());

        let user_msg = error.user_message();
        assert!(user_msg.contains("Invalid response"));
    }

    #[test]
    fn test_http_response_creation() {
        let headers = BTreeMap::from_iter(vec![(
            "content-type".to_string(),
            "application/json".to_string(),
        )]);

        let response = HttpResponse::new(
            "test body",
            200,
            headers,
            RequestId::from_string("req-123"),
            150,
        );

        assert_eq!(response.body, "test body");
        assert_eq!(response.status, 200);
        assert_eq!(response.headers.len(), 1);
        assert_eq!(response.request_id.as_str(), "req-123");
        assert_eq!(response.duration_ms, 150);
        assert!(response.is_success());
    }

    #[test]
    fn test_http_response_map() {
        let response = HttpResponse::new(
            "test body",
            200,
            BTreeMap::new(),
            RequestId::from_string("req-123"),
            150,
        );

        let mapped = response.map(|s| s.len());

        assert_eq!(mapped.body, 9);
        assert_eq!(mapped.status, 200);
    }

    #[test]
    fn test_http_response_is_success() {
        let success_2xx = HttpResponse::new("ok", 200, BTreeMap::new(), RequestId::new(), 100);
        assert!(success_2xx.is_success());

        let redirect_3xx =
            HttpResponse::new("redirect", 301, BTreeMap::new(), RequestId::new(), 100);
        assert!(!redirect_3xx.is_success());

        let client_error_4xx =
            HttpResponse::new("not found", 404, BTreeMap::new(), RequestId::new(), 100);
        assert!(!client_error_4xx.is_success());

        let server_error_5xx =
            HttpResponse::new("server error", 500, BTreeMap::new(), RequestId::new(), 100);
        assert!(!server_error_5xx.is_success());
    }

    #[test]
    fn test_web_client_new() {
        let config = WebClientConfig::default();
        let client = WebClient::new(config);

        assert!(client.is_ok());
    }

    #[test]
    fn test_web_client_with_base_url() {
        let client = WebClient::with_base_url("http://localhost:8080".to_string());

        assert!(client.is_ok());
        if let Ok(client) = client {
            assert_eq!(client.config.base_url, "http://localhost:8080");
        }
    }

    #[tokio::test]
    async fn test_web_client_health_check_network_error() -> Result<(), Box<dyn std::error::Error>>
    {
        let client = WebClient::with_base_url("http://invalid.example.test:99999".to_string())?;
        let result = client.health_check().await;

        assert!(result.is_err());

        match result {
            Err(WebClientError::ConnectionRefused { .. }) => {
                // Expected error type
            }
            Err(WebClientError::DnsFailed { .. }) => {
                // Also possible
            }
            other => {
                panic!(
                    "Expected ConnectionRefused or DnsFailed error, got {:?}",
                    other
                );
            }
        }

        Ok(())
    }

    #[test]
    fn test_web_client_error_cloned() {
        let error1 = WebClientError::Timeout { seconds: 30 };
        let error2 = error1.clone();

        assert_eq!(error1, error2);
    }

    #[test]
    fn test_web_client_error_equality() {
        let error1 = WebClientError::Network {
            message: "test".to_string(),
        };
        let error2 = WebClientError::Network {
            message: "test".to_string(),
        };
        let error3 = WebClientError::Network {
            message: "different".to_string(),
        };

        assert_eq!(error1, error2);
        assert_ne!(error1, error3);
    }
}
