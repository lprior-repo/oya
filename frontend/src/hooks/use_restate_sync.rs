#![deny(clippy::unwrap_used)]
#![deny(clippy::expect_used)]
#![deny(clippy::panic)]

//! Hook for polling Restate's introspection API and surfacing live invocation state.
//!
//! Usage:
//! ```rust
//! let restate = use_restate_sync();
//! // restate.state.read().invocations → Vec<Invocation>
//! // restate.enabled → toggle polling on/off
//! // restate.admin_url → configurable admin URL (default: http://localhost:9070)
//! // restate.ingress_url → configurable ingress URL (default: http://localhost:909)
//! ```

use crate::canonical_ports::{default_admin_url, default_ingress_url, ADMIN_PORT};
use crate::restate_client::types::Invocation;
use crate::restate_client::{RestateClient, RestateClientConfig};
use crate::restate_sync::poller::{InvocationEvent, InvocationPoller, PollResult};
use dioxus::prelude::*;
use im::HashMap;
use std::sync::Arc;

/// Live state surfaced from the Restate introspection poll.
#[derive(Clone, Debug, Default)]
pub struct RestateState {
    /// Latest snapshot of all invocations, indexed by ID.
    pub invocations: HashMap<String, Invocation>,
    /// Whether the last poll succeeded.
    pub connected: bool,
    /// Last error message if the connection failed.
    pub last_error: Option<String>,
    /// Last time the state was updated (timestamp).
    pub last_updated: i64,
}

/// Handle returned by `use_restate_sync`.
#[derive(Clone, Copy, PartialEq)]
pub struct RestateSyncHandle {
    /// Read-only view of the latest Restate state.
    pub state: ReadSignal<RestateState>,
    /// Toggle to start/stop polling. Write `true` to enable, `false` to pause.
    pub enabled: Signal<bool>,
    /// Admin API base URL (default: <http://localhost:9070>). Changing this restarts the client.
    pub admin_url: Signal<String>,
    /// Ingress base URL (default: <http://localhost:909>). Used when running workflows.
    pub ingress_url: Signal<String>,
    /// Polling interval in milliseconds (default: 2000ms).
    pub poll_interval_ms: Signal<u32>,
}

pub fn provide_restate_sync_context() -> RestateSyncHandle {
    let state = use_signal(RestateState::default);
    let enabled = use_signal(|| false);
    let admin_url = use_signal(default_admin_url);
    let ingress_url = use_signal(default_ingress_url);
    let poll_interval_ms = use_signal(|| 2000u32);

    use_future(move || polling_loop(enabled, admin_url, state, poll_interval_ms));

    provide_context(sync_handle(state, enabled, admin_url, ingress_url, poll_interval_ms))
}

fn sync_handle(
    state: Signal<RestateState>,
    enabled: Signal<bool>,
    admin_url: Signal<String>,
    ingress_url: Signal<String>,
    poll_interval_ms: Signal<u32>,
) -> RestateSyncHandle {
    RestateSyncHandle { state: state.into(), enabled, admin_url, ingress_url, poll_interval_ms }
}

async fn polling_loop(
    enabled: Signal<bool>,
    admin_url: Signal<String>,
    mut state: Signal<RestateState>,
    poll_interval_ms: Signal<u32>,
) {
    let mut last_admin_url = String::new();
    let mut poller: Option<InvocationPoller> = None;

    loop {
        poll_cycle(enabled, admin_url, &mut state, &mut last_admin_url, &mut poller).await;
        poll_sleep_ms(*poll_interval_ms.read()).await;
    }
}

async fn poll_cycle(
    enabled: Signal<bool>,
    admin_url: Signal<String>,
    state: &mut Signal<RestateState>,
    last_admin_url: &mut String,
    poller: &mut Option<InvocationPoller>,
) {
    if *enabled.read() {
        refresh_poller(admin_url.read().as_str(), last_admin_url, poller);
        if let Some(ref mut p) = poller {
            poll_once(state, p).await;
        }
    } else {
        reset_poller(last_admin_url, poller);
    }
}

fn refresh_poller(
    current_admin_url: &str,
    last_admin_url: &mut String,
    poller: &mut Option<InvocationPoller>,
) {
    if current_admin_url == last_admin_url {
        return;
    }

    let config = build_restate_config_from_url(current_admin_url);
    let client = Arc::new(RestateClient::new(config));
    *poller = Some(InvocationPoller::new(client));
    *last_admin_url = current_admin_url.to_string();
}

fn reset_poller(last_admin_url: &mut String, poller: &mut Option<InvocationPoller>) {
    *poller = None;
    last_admin_url.clear();
}

async fn poll_once(state: &mut Signal<RestateState>, poller: &mut InvocationPoller) {
    match poller.poll().await {
        Ok(result) => apply_poll_result(&mut state.write(), &result, poller),
        Err(err) => apply_poll_error(&mut state.write(), err.to_string()),
    }
}

fn apply_poll_error(state: &mut RestateState, error: String) {
    state.connected = false;
    state.last_error = Some(error);
}

fn apply_poll_result(state: &mut RestateState, result: &PollResult, poller: &InvocationPoller) {
    state.connected = true;
    state.last_error = None;
    state.last_updated = result.timestamp;

    let needs_refresh = apply_delta_events(state, result);
    if needs_refresh || result.events.is_empty() {
        state.invocations =
            poller.state().invocations().into_iter().map(|inv| (inv.id.clone(), inv)).collect();
    }
}

fn apply_delta_events(state: &mut RestateState, result: &PollResult) -> bool {
    result.events.iter().fold(false, |needs_refresh, event| match event {
        InvocationEvent::StatusChanged { invocation_id, new_status, .. } => {
            if let Some(inv) = state.invocations.get_mut(invocation_id) {
                inv.status = (*new_status).into();
            }
            needs_refresh
        }
        InvocationEvent::Completed { .. }
        | InvocationEvent::Failed { .. }
        | InvocationEvent::New { .. } => true,
    })
}

#[must_use]
pub fn use_restate_sync() -> RestateSyncHandle {
    use_context::<RestateSyncHandle>()
}

/// Parse a URL like "http://host:port" into a `RestateClientConfig`.
/// Falls back to defaults if parsing fails.
#[must_use]
pub fn build_restate_config_from_url(url: &str) -> RestateClientConfig {
    let url = url.trim_end_matches('/');
    // Strip scheme.
    let without_scheme = match url.strip_prefix("http://").or_else(|| url.strip_prefix("https://"))
    {
        Some(stripped) => stripped,
        None => url,
    };

    let (host, port) = if let Some(colon) = without_scheme.rfind(':') {
        let h = &without_scheme[..colon];
        let p = without_scheme[colon + 1..].parse::<u16>().ok();
        (h.to_string(), p)
    } else {
        (without_scheme.to_string(), None)
    };

    RestateClientConfig {
        host: if host.is_empty() { "localhost".to_string() } else { host },
        port: port.unwrap_or(ADMIN_PORT),
        timeout_secs: 10,
    }
}

/// Target-specific sleep: real timer in WASM, tokio on native.
pub async fn poll_sleep_ms(ms: u32) {
    #[cfg(target_arch = "wasm32")]
    {
        gloo_timers::future::TimeoutFuture::new(ms).await;
    }
    #[cfg(not(target_arch = "wasm32"))]
    {
        tokio::time::sleep(std::time::Duration::from_millis(u64::from(ms))).await;
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used, clippy::panic, clippy::float_cmp)]
mod tests {
    use super::build_restate_config_from_url;

    #[test]
    fn given_default_url_when_parsing_then_localhost_9070_is_used() {
        let config = build_restate_config_from_url("http://localhost:9070");
        assert_eq!(config.host, "localhost");
        assert_eq!(config.port, 9070);
    }

    #[test]
    fn given_custom_host_and_port_when_parsing_then_both_are_captured() {
        let config = build_restate_config_from_url("http://192.168.1.100:9999");
        assert_eq!(config.host, "192.168.1.100");
        assert_eq!(config.port, 9999);
    }

    #[test]
    fn given_url_without_port_when_parsing_then_default_port_is_used() {
        let config = build_restate_config_from_url("http://myhost");
        assert_eq!(config.host, "myhost");
        assert_eq!(config.port, 9070);
    }

    #[test]
    fn given_url_with_trailing_slash_when_parsing_then_slash_is_stripped() {
        let config = build_restate_config_from_url("http://localhost:9070/");
        assert_eq!(config.host, "localhost");
        assert_eq!(config.port, 9070);
    }

    #[test]
    fn given_empty_url_when_parsing_then_defaults_are_used() {
        let config = build_restate_config_from_url("");
        assert_eq!(config.host, "localhost");
        assert_eq!(config.port, 9070);
    }
}
