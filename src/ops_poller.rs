use super::*;
use std::io::Write;

pub(super) async fn run_ops_poller() -> Result<(), DynError> {
    let config = opencode_config()?;
    let interval_ms = poll_interval_ms()?;
    write_poller_banner(&config, interval_ms)?;
    let client = poller_http_client()?;
    loop {
        write_poll_iteration(&client, &config).await?;
        tokio::time::sleep(std::time::Duration::from_millis(interval_ms)).await;
    }
}

fn poll_interval_ms() -> Result<u64, OyaError> {
    match std::env::var("OYA_POLL_INTERVAL_MS") {
        Ok(value) => value.parse::<u64>().map(clamp_poll_interval).map_err(|error| {
            OyaError(format!(
                "Invalid OYA_POLL_INTERVAL_MS='{}': {} (expected integer milliseconds)",
                value, error
            ))
        }),
        Err(_) => Ok(2000),
    }
}

fn clamp_poll_interval(parsed: u64) -> u64 {
    let clamped = parsed.clamp(500, 30000);
    if clamped != parsed {
        tracing::warn!("OYA_POLL_INTERVAL_MS={} out of range, clamped to {}", parsed, clamped);
    }
    clamped
}

fn write_poller_banner(config: &OpenCodeConfig, interval_ms: u64) -> Result<(), OyaError> {
    let mut stderr = std::io::stderr().lock();
    writeln!(stderr, "[oya:ops-poll] source={} interval_ms={}", config.base_url, interval_ms)
        .map_err(|error| OyaError(format!("Failed to write poller banner: {}", error)))?;
    writeln!(stderr, "[oya:ops-poll] columns: ts | busy | perm | quest | event_preview")
        .map_err(|error| OyaError(format!("Failed to write poller banner: {}", error)))?;
    Ok(())
}

fn poller_http_client() -> Result<reqwest::Client, reqwest::Error> {
    build_http_client(poller_http_client_settings())
}

async fn write_poll_iteration(
    client: &reqwest::Client,
    config: &OpenCodeConfig,
) -> Result<(), OyaError> {
    match poll_opencode_status(client, config).await {
        Ok(status_line) => {
            let mut stdout = std::io::stdout().lock();
            writeln!(stdout, "{}", status_line)
                .map_err(|error| OyaError(format!("Failed to write poll status line: {}", error)))
        }
        Err(error) => {
            let mut stderr = std::io::stderr().lock();
            writeln!(stderr, "[oya:ops-poll] error: {}", error)
                .map_err(|io_error| OyaError(format!("Failed to write poll error: {}", io_error)))
        }
    }
}

async fn poll_opencode_status(
    client: &reqwest::Client,
    config: &OpenCodeConfig,
) -> Result<String, OyaError> {
    let status_url = opencode_endpoint_url(config, OpenCodeEndpoint::SessionStatus);
    let perm_url = opencode_endpoint_url(config, OpenCodeEndpoint::Permission);
    let question_url = opencode_endpoint_url(config, OpenCodeEndpoint::Question);

    let status_raw =
        fetch_text_with_client(client, &status_url, config.password.as_deref()).await?;
    let perm_raw = fetch_text_with_client(client, &perm_url, config.password.as_deref()).await?;
    let question_raw =
        fetch_text_with_client(client, &question_url, config.password.as_deref()).await?;

    let snapshot = build_opencode_poll_snapshot(&status_raw, &perm_raw, &question_raw)
        .map_err(|error| OyaError(format!("Parse failed: {}", error)))?;

    let busy_preview = if snapshot.busy_sessions.is_empty() {
        "-".to_string()
    } else if snapshot.busy_sessions.len() <= 3 {
        snapshot.busy_sessions.join(",")
    } else {
        format!("{},...+{}", snapshot.busy_sessions[0], snapshot.busy_sessions.len() - 1)
    };

    let ts = chrono::Utc::now().format("%H:%M:%S%.3f");
    Ok(format!(
        "{} | {} | {} | {}",
        ts, busy_preview, snapshot.pending_permissions, snapshot.pending_questions
    ))
}

async fn fetch_text_with_client(
    client: &reqwest::Client,
    url: &str,
    password: Option<&str>,
) -> Result<String, OyaError> {
    let mut attempt = 0;
    loop {
        enforce_opencode_rate_limit().await;
        let request = password.map_or_else(
            || client.get(url),
            |pwd| client.get(url).basic_auth("opencode", Some(pwd)),
        );

        let response = request
            .send()
            .await
            .map_err(|error| OyaError(format!("Request failed for {}: {}", url, error)))?;

        let status = response.status();
        let text = response
            .text()
            .await
            .map_err(|error| OyaError(format!("Read failed for {}: {}", url, error)))?;

        if status.is_success() {
            return Ok(text);
        }

        if should_retry_rate_limit(status, &text) && attempt < 2 {
            let wait = std::time::Duration::from_millis(200_u64.saturating_mul(1_u64 << attempt));
            tokio::time::sleep(wait).await;
            attempt += 1;
            continue;
        }

        return Err(OyaError(format!(
            "Status {} for {}: {}",
            status.as_u16(),
            url,
            truncate_clean(&text, 200)
        )));
    }
}

fn should_retry_rate_limit(status: reqwest::StatusCode, text: &str) -> bool {
    status.as_u16() == 429 || text.to_ascii_lowercase().contains("rate limit")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn poll_interval_rejects_invalid_env_value() {
        std::env::set_var("OYA_POLL_INTERVAL_MS", "abc");
        let result = poll_interval_ms();
        std::env::remove_var("OYA_POLL_INTERVAL_MS");
        assert!(result.is_err());
    }

    #[test]
    fn poll_interval_defaults_when_env_missing() {
        std::env::remove_var("OYA_POLL_INTERVAL_MS");
        let result = poll_interval_ms();
        assert!(matches!(result, Ok(2000)));
    }
}
