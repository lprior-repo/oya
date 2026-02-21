use super::*;
use std::io::Write;

pub(super) async fn run_ops_poller() -> Result<(), DynError> {
    let config = opencode_config()?;
    let interval_ms = poll_interval_ms();
    write_poller_banner(&config, interval_ms)?;
    let client = poller_http_client()?;
    loop {
        write_poll_iteration(&client, &config).await?;
        tokio::time::sleep(std::time::Duration::from_millis(interval_ms)).await;
    }
}

fn poll_interval_ms() -> u64 {
    match std::env::var("OYA_POLL_INTERVAL_MS") {
        Ok(value) => value.parse::<u64>().map_or_else(
            |_| {
                tracing::warn!(
                    "OYA_POLL_INTERVAL_MS='{}' is not a valid number, using default 2000",
                    value
                );
                2000
            },
            clamp_poll_interval,
        ),
        Err(_) => 2000,
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
    writeln!(
        stderr,
        "[oya:ops-poll] source={} interval_ms={}",
        sanitize_url_for_logging(&config.base_url),
        interval_ms
    )
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
    let request = password
        .map_or_else(|| client.get(url), |pwd| client.get(url).basic_auth("opencode", Some(pwd)));

    let response = request.send().await.map_err(|error| {
        OyaError(format!("Request failed for {}: {}", sanitize_url_for_logging(url), error))
    })?;

    let status = response.status();
    let text = response.text().await.map_err(|error| {
        OyaError(format!("Read failed for {}: {}", sanitize_url_for_logging(url), error))
    })?;

    if !status.is_success() {
        return Err(OyaError(format!(
            "Status {} for {}: {}",
            status.as_u16(),
            sanitize_url_for_logging(url),
            truncate_clean(&text, 200)
        )));
    }

    Ok(text)
}
