use super::super::*;

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
}

pub(crate) fn opencode_config() -> Result<OpenCodeConfig, OyaError> {
    let base_url = std::env::var("OYA_OPENCODE_BASE_URL")
        .ok()
        .filter(|value| !value.trim().is_empty())
        .unwrap_or_else(|| "http://127.0.0.1:4097".to_string());

    if !is_valid_http_url(base_url.as_str()) {
        return Err(OyaError(format!(
            "Invalid OYA_OPENCODE_BASE_URL '{}'",
            sanitize_url_for_logging(&base_url)
        )));
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

    let response = request
        .send()
        .await
        .map_err(|error| OyaError(format!("OpenCode request failed for {}: {}", path, error)))?;
    let status = response.status();
    let text = response.text().await.map_err(|error| {
        OyaError(format!("OpenCode response read failed for {}: {}", path, error))
    })?;

    if !status.is_success() {
        return Err(OyaError(format!(
            "OpenCode request failed for {} with status {}: {}",
            path,
            status.as_u16(),
            truncate_clean(text.as_str(), 4000)
        )));
    }

    Ok(text)
}
