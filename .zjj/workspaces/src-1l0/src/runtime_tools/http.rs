use super::super::*;
use std::sync::OnceLock;

const OPENCODE_MIN_REQUEST_INTERVAL_MS: u64 = 200;
const OPENCODE_RATE_LIMIT_RETRIES: u32 = 2;

static OPENCODE_RATE_LIMITER: OnceLock<tokio::sync::Mutex<Option<std::time::Instant>>> =
    OnceLock::new();

#[derive(Debug, Clone)]
pub(crate) struct OpenCodeConfig {
    pub(crate) base_url: String,
    pub(crate) password: Option<String>,
}

#[derive(Clone, Copy)]
pub(crate) enum OpenCodeEndpoint {
    SessionStatus,
    Permission,
    Question,
}

impl OpenCodeEndpoint {
    const fn path(self) -> &'static str {
        match self {
            Self::SessionStatus => "session/status",
            Self::Permission => "permission",
            Self::Question => "question",
        }
    }
}

pub(crate) fn opencode_endpoint_url(config: &OpenCodeConfig, endpoint: OpenCodeEndpoint) -> String {
    format!("{}/{}", config.base_url.trim_end_matches('/'), endpoint.path())
}

#[derive(Clone, Copy)]
pub(crate) struct HttpClientSettings {
    pub(crate) timeout_secs: u64,
    pub(crate) connect_timeout_secs: u64,
    pub(crate) pool_max_idle_per_host: usize,
    pub(crate) pool_idle_timeout_secs: u64,
    pub(crate) tcp_keepalive_secs: Option<u64>,
}

pub(crate) fn build_http_client(
    settings: HttpClientSettings,
) -> Result<reqwest::Client, reqwest::Error> {
    let mut builder = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(settings.timeout_secs))
        .connect_timeout(std::time::Duration::from_secs(settings.connect_timeout_secs))
        .pool_max_idle_per_host(settings.pool_max_idle_per_host)
        .pool_idle_timeout(std::time::Duration::from_secs(settings.pool_idle_timeout_secs));

    if let Some(tcp_keepalive_secs) = settings.tcp_keepalive_secs {
        builder = builder.tcp_keepalive(std::time::Duration::from_secs(tcp_keepalive_secs));
    }

    builder.build()
}

pub(crate) fn workflow_http_client_settings() -> HttpClientSettings {
    HttpClientSettings {
        timeout_secs: 30,
        connect_timeout_secs: 10,
        pool_max_idle_per_host: 10,
        pool_idle_timeout_secs: 60,
        tcp_keepalive_secs: Some(60),
    }
}

pub(crate) fn poller_http_client_settings() -> HttpClientSettings {
    HttpClientSettings {
        timeout_secs: 10,
        connect_timeout_secs: 5,
        pool_max_idle_per_host: 5,
        pool_idle_timeout_secs: 30,
        tcp_keepalive_secs: None,
    }
}

pub(crate) fn opencode_http_client_settings(timeout_seconds: u64) -> HttpClientSettings {
    HttpClientSettings {
        timeout_secs: timeout_seconds,
        connect_timeout_secs: 10,
        pool_max_idle_per_host: 10,
        pool_idle_timeout_secs: 60,
        tcp_keepalive_secs: Some(60),
    }
}

pub(crate) async fn enforce_opencode_rate_limit() {
    let limiter = OPENCODE_RATE_LIMITER.get_or_init(|| tokio::sync::Mutex::new(None));
    let min_interval = std::time::Duration::from_millis(OPENCODE_MIN_REQUEST_INTERVAL_MS);

    loop {
        let wait_duration = {
            let mut guard = limiter.lock().await;
            match *guard {
                Some(last) => {
                    let elapsed = last.elapsed();
                    if elapsed >= min_interval {
                        *guard = Some(std::time::Instant::now());
                        None
                    } else {
                        Some(min_interval - elapsed)
                    }
                }
                None => {
                    *guard = Some(std::time::Instant::now());
                    None
                }
            }
        };

        if let Some(duration) = wait_duration {
            tokio::time::sleep(duration).await;
        } else {
            break;
        }
    }
}

pub(crate) fn build_blocking_http_client(
    settings: HttpClientSettings,
) -> Result<reqwest::blocking::Client, reqwest::Error> {
    let mut builder = reqwest::blocking::Client::builder()
        .timeout(std::time::Duration::from_secs(settings.timeout_secs))
        .connect_timeout(std::time::Duration::from_secs(settings.connect_timeout_secs))
        .pool_max_idle_per_host(settings.pool_max_idle_per_host)
        .pool_idle_timeout(std::time::Duration::from_secs(settings.pool_idle_timeout_secs));

    if let Some(tcp_keepalive_secs) = settings.tcp_keepalive_secs {
        builder = builder.tcp_keepalive(std::time::Duration::from_secs(tcp_keepalive_secs));
    }

    builder.build()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_blocking_http_client_builds_with_all_settings() {
        let settings = opencode_http_client_settings(300);
        let _client =
            build_blocking_http_client(settings).expect("Failed to build blocking client");
    }

    #[test]
    fn test_should_retry_rate_limited_true_for_429() {
        assert!(should_retry_rate_limited(reqwest::StatusCode::TOO_MANY_REQUESTS, ""));
    }

    #[test]
    fn test_should_retry_rate_limited_true_for_message() {
        assert!(should_retry_rate_limited(reqwest::StatusCode::BAD_REQUEST, "Rate limit exceeded"));
    }

    #[test]
    fn test_should_retry_rate_limited_false_for_other_status() {
        assert!(!should_retry_rate_limited(reqwest::StatusCode::BAD_REQUEST, "invalid input"));
    }
}

pub(crate) fn opencode_config() -> Result<OpenCodeConfig, OyaError> {
    let base_url = std::env::var("OYA_OPENCODE_BASE_URL")
        .ok()
        .filter(|value| !value.trim().is_empty())
        .unwrap_or_else(|| "http://127.0.0.1:4097".to_string());

    if !is_valid_http_url(base_url.as_str()) {
        return Err(OyaError(format!("Invalid OYA_OPENCODE_BASE_URL '{}'", base_url)));
    }

    let password =
        std::env::var("OYA_OPENCODE_PASSWORD").ok().filter(|value| !value.trim().is_empty());

    Ok(OpenCodeConfig { base_url, password })
}

pub(crate) async fn fetch_opencode_text(
    config: &OpenCodeConfig,
    path: &str,
    timeout_seconds: u64,
) -> Result<String, OyaError> {
    let client = build_http_client(opencode_http_client_settings(timeout_seconds))
        .map_err(|error| OyaError(format!("OpenCode HTTP client build failed: {}", error)))?;

    let url = format!("{}{}", config.base_url.trim_end_matches('/'), path);
    let request = config.password.as_ref().map_or_else(
        || client.get(url.clone()),
        |password| client.get(url.clone()).basic_auth("opencode", Some(password)),
    );

    fetch_opencode_text_with_backoff(request, path).await
}

async fn fetch_opencode_text_with_backoff(
    request: reqwest::RequestBuilder,
    path: &str,
) -> Result<String, OyaError> {
    let mut attempt = 0;
    loop {
        enforce_opencode_rate_limit().await;
        let response = request
            .try_clone()
            .ok_or_else(|| OyaError("failed to clone OpenCode request".to_string()))?
            .send()
            .await
            .map_err(|error| {
                OyaError(format!("OpenCode request failed for {}: {}", path, error))
            })?;

        let status = response.status();
        let text = response.text().await.map_err(|error| {
            OyaError(format!("OpenCode response read failed for {}: {}", path, error))
        })?;

        if status.is_success() {
            return Ok(text);
        }

        if should_retry_rate_limited(status, text.as_str()) && attempt < OPENCODE_RATE_LIMIT_RETRIES
        {
            tokio::time::sleep(rate_limit_backoff(attempt)).await;
            attempt += 1;
            continue;
        }

        return Err(OyaError(format!(
            "OpenCode request failed for {} with status {}: {}",
            path,
            status.as_u16(),
            truncate_clean(text.as_str(), 4000)
        )));
    }
}

fn should_retry_rate_limited(status: reqwest::StatusCode, body: &str) -> bool {
    status.as_u16() == 429 || body.to_ascii_lowercase().contains("rate limit")
}

fn rate_limit_backoff(attempt: u32) -> std::time::Duration {
    let millis = 200_u64.saturating_mul(2_u64.saturating_pow(attempt));
    std::time::Duration::from_millis(millis)
}
