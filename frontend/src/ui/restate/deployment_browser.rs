#![deny(clippy::unwrap_used)]
#![deny(clippy::expect_used)]
#![deny(clippy::panic)]
#![warn(clippy::pedantic)]
#![forbid(unsafe_code)]

//! Deployment Browser Panel — inspect Restate deployments.
//!
//! Collapsible panel showing all registered deployments with endpoint, type badge
//! (HTTP/Lambda), and relative age. Auto-refreshes on a configurable interval.

use crate::hooks::{build_restate_config_from_url, poll_sleep_ms, RestateSyncHandle};
use crate::restate_client::types::{DeploymentInfo, DeploymentType};
use crate::restate_client::RestateClient;
use dioxus::prelude::*;

/// Format a millisecond epoch timestamp as a relative age string.
/// Returns human-readable durations like "3s ago", "5m ago", "2h ago", "7d ago".
fn format_age(created_at_ms: i64) -> String {
    let now_ms = chrono::Utc::now().timestamp_millis();
    let delta_ms = now_ms - created_at_ms;

    if delta_ms < 0 {
        return "just now".to_string();
    }

    let secs = delta_ms / 1000;

    if secs < 60 {
        format!("{secs}s ago")
    } else if secs < 3600 {
        let mins = secs / 60;
        format!("{mins}m ago")
    } else if secs < 86_400 {
        let hrs = secs / 3600;
        format!("{hrs}h ago")
    } else {
        let days = secs / 86_400;
        format!("{days}d ago")
    }
}

/// Return the Tailwind badge class for a deployment type.
const fn deployment_type_badge_class(ty: DeploymentType) -> &'static str {
    match ty {
        DeploymentType::Http => {
            "text-[10px] font-semibold px-1.5 py-0.5 rounded border bg-blue-50 text-blue-700 border-blue-200"
        }
        DeploymentType::Lambda => {
            "text-[10px] font-semibold px-1.5 py-0.5 rounded border bg-purple-50 text-purple-700 border-purple-200"
        }
        DeploymentType::Unknown => {
            "text-[10px] font-semibold px-1.5 py-0.5 rounded border bg-slate-50 text-slate-600 border-slate-200"
        }
    }
}

/// Display label for deployment type.
const fn deployment_type_label(ty: DeploymentType) -> &'static str {
    match ty {
        DeploymentType::Http => "HTTP",
        DeploymentType::Lambda => "Lambda",
        DeploymentType::Unknown => "Unknown",
    }
}

#[derive(Props, Clone, PartialEq)]
pub struct DeploymentBrowserPanelProps {
    pub handle: RestateSyncHandle,
}

#[component]
pub fn DeploymentBrowserPanel(props: DeploymentBrowserPanelProps) -> Element {
    let mut collapsed = use_signal(|| true);
    let mut deployments: Signal<Vec<DeploymentInfo>> = use_signal(Vec::new);
    let mut loading = use_signal(|| false);
    let mut error_msg: Signal<Option<String>> = use_signal(|| None);
    let mut loaded = use_signal(|| false);

    // Auto-refresh future — runs while panel is expanded
    use_future(move || async move {
        loop {
            if !*collapsed.read() {
                let admin_url = props.handle.admin_url.read().clone();
                let config = build_restate_config_from_url(&admin_url);
                let client = RestateClient::new(config);

                let result = client.list_deployments().await;
                match result {
                    Ok(data) => {
                        deployments.set(data);
                        loaded.set(true);
                    }
                    Err(err) => {
                        error_msg.set(Some(err.to_string()));
                        loaded.set(true);
                    }
                }
            }

            let interval = *props.handle.poll_interval_ms.read();
            poll_sleep_ms(interval).await;
        }
    });

    let count = deployments.read().len();

    rsx! {
        div { class: "border-t border-slate-200 shrink-0",

            // Panel header
            button {
                class: "flex w-full items-center justify-between px-3 py-2 hover:bg-slate-50 transition-colors",
                onclick: move |_| {
                    let current = *collapsed.read();
                    collapsed.set(!current);

                    // Auto-load on first expand
                    if current && !*loaded.read() {
                        let admin_url = props.handle.admin_url.read().clone();
                        spawn(async move {
                            let config = build_restate_config_from_url(&admin_url);
                            let client = RestateClient::new(config);
                            match client.list_deployments().await {
                                Ok(data) => {
                                    deployments.set(data);
                                    loading.set(false);
                                    loaded.set(true);
                                }
                                Err(err) => {
                                    error_msg.set(Some(err.to_string()));
                                    loading.set(false);
                                    loaded.set(true);
                                }
                            }
                        });
                        loading.set(true);
                    }
                },

                div { class: "flex items-center gap-2",
                    span {
                        class: "text-slate-400 transition-transform",
                        style: if *collapsed.read() { "transform: rotate(-90deg);" } else { "" },
                        "▾"
                    }
                    span { class: "text-[11px] font-semibold text-slate-600 uppercase tracking-wide",
                        "Deployments"
                    }
                    if count > 0 {
                        span { class: "text-[10px] text-slate-400",
                            "({count})"
                        }
                    }
                }

                // Refresh button (visible when panel is open)
                if !*collapsed.read() {
                    button {
                        class: if *loading.read() {
                            "text-[10px] px-2 py-0.5 rounded border font-medium bg-slate-100 text-slate-400 border-slate-200 cursor-not-allowed"
                        } else {
                            "text-[10px] px-2 py-0.5 rounded border font-medium bg-indigo-50 text-indigo-600 border-indigo-200 hover:bg-indigo-100 transition-colors"
                        },
                        disabled: *loading.read(),
                        onclick: move |evt| {
                            evt.stop_propagation();
                            let admin_url = props.handle.admin_url.read().clone();
                            loading.set(true);
                            error_msg.set(None);
                            spawn(async move {
                                let config = build_restate_config_from_url(&admin_url);
                                let client = RestateClient::new(config);
                                match client.list_deployments().await {
                                    Ok(data) => {
                                        deployments.set(data);
                                        loading.set(false);
                                        loaded.set(true);
                                    }
                                    Err(err) => {
                                        error_msg.set(Some(err.to_string()));
                                        loading.set(false);
                                        loaded.set(true);
                                    }
                                }
                            });
                        },
                        if *loading.read() { "Loading…" } else { "Refresh" }
                    }
                }
            }

            // Panel body
            if !*collapsed.read() {
                div { class: "max-h-[200px] overflow-y-auto",

                    // Error banner
                    if let Some(err) = error_msg.read().as_ref() {
                        div { class: "mx-3 my-2 rounded bg-red-50 border border-red-200 px-2 py-1 text-[10px] text-red-700",
                            "{err}"
                        }
                    }

                    // Loading state
                    if *loading.read() {
                        div { class: "px-3 py-3 text-[11px] text-slate-400 text-center",
                            "Loading deployments…"
                        }
                    } else if !*loaded.read() {
                        // Initial state
                        div { class: "px-3 py-3 text-[11px] text-slate-400 text-center",
                            "Expand to load deployments."
                        }
                    } else if deployments.read().is_empty() {
                        // Empty result state
                        div { class: "px-3 py-3 text-[11px] text-slate-400 text-center",
                            "No deployments found."
                        }
                    } else {
                        // Deployment table
                        table { class: "w-full border-collapse text-left",
                            thead {
                                tr { class: "bg-slate-50 border-b border-slate-200",
                                    th { class: "text-[10px] font-semibold text-slate-400 uppercase tracking-wide px-3 py-1.5", "Endpoint" }
                                    th { class: "text-[10px] font-semibold text-slate-400 uppercase tracking-wide px-3 py-1.5 w-20", "Type" }
                                    th { class: "text-[10px] font-semibold text-slate-400 uppercase tracking-wide px-3 py-1.5 w-20", "Age" }
                                }
                            }
                            tbody {
                                for dep in deployments.read().iter() {
                                    {
                                        let endpoint = dep.endpoint.clone();
                                        let ty = dep.ty;
                                        let badge_class = deployment_type_badge_class(ty);
                                        let type_label = deployment_type_label(ty);
                                        let age = format_age(dep.created_at);
                                        let row_key = dep.id.clone();

                                        rsx! {
                                            tr {
                                                key: "{row_key}",
                                                class: "border-b border-slate-100 last:border-b-0 hover:bg-slate-50 transition-colors",

                                                td { class: "px-3 py-1.5 font-mono text-[10px] text-slate-600 max-w-[160px] truncate",
                                                    "{endpoint}"
                                                }
                                                td { class: "px-3 py-1.5",
                                                    span { class: "{badge_class}", "{type_label}" }
                                                }
                                                td { class: "px-3 py-1.5 text-[10px] text-slate-500",
                                                    "{age}"
                                                }
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used, clippy::panic, clippy::float_cmp)]
mod tests {
    use super::*;
    use crate::restate_client::types::DeploymentType;

    #[test]
    fn format_age_recent_seconds() {
        let now_ms = chrono::Utc::now().timestamp_millis();
        let thirty_sec_ago = now_ms - 30_000;
        let result = format_age(thirty_sec_ago);
        assert!(result.contains("s ago"), "Expected seconds format, got: {result}");
    }

    #[test]
    fn format_age_hours() {
        let now_ms = chrono::Utc::now().timestamp_millis();
        let three_hours_ago = now_ms - (3 * 3600 * 1000);
        let result = format_age(three_hours_ago);
        assert!(result.contains("h ago"), "Expected hours format, got: {result}");
    }

    #[test]
    fn format_age_zero_timestamp() {
        let result = format_age(0);
        // Should not panic — any output is acceptable for epoch 0
        assert!(!result.is_empty(), "format_age should return non-empty string");
    }

    #[test]
    fn deployment_type_badge_class_http_is_blue() {
        let class = deployment_type_badge_class(DeploymentType::Http);
        assert!(class.contains("blue"), "HTTP badge should contain 'blue', got: {class}");
    }

    #[test]
    fn deployment_type_badge_class_lambda_is_purple() {
        let class = deployment_type_badge_class(DeploymentType::Lambda);
        assert!(class.contains("purple"), "Lambda badge should contain 'purple', got: {class}");
    }
}
