#![deny(clippy::unwrap_used)]
#![deny(clippy::expect_used)]
#![deny(clippy::panic)]
#![warn(clippy::pedantic)]
#![forbid(unsafe_code)]

use crate::hooks::poll_sleep_ms;
use crate::hooks::RestateSyncHandle;
use crate::restate_client::types::InvocationStatus;
use dioxus::prelude::*;
use serde::Deserialize;
use serde_json::Value;

#[derive(Clone, Debug, Default, Deserialize, PartialEq)]
pub struct OpenCodeTraceSnapshot {
    #[serde(default)]
    pub bead_id: Option<String>,
    pub workflow_key: String,
    #[serde(default)]
    pub active_invocation_id: Option<String>,
    #[serde(default)]
    pub model: Option<String>,
    #[serde(default)]
    pub started_at: Option<String>,
    #[serde(default)]
    pub updated_at: Option<String>,
    #[serde(default)]
    pub finished_at: Option<String>,
    pub status: String,
    #[serde(default)]
    pub current_event: Option<OpenCodeTraceEvent>,
    #[serde(default)]
    pub events: Vec<OpenCodeTraceEvent>,
    #[serde(default)]
    pub tool_call_count: u64,
    #[serde(default)]
    pub text_event_count: u64,
    #[serde(default)]
    pub last_error: Option<String>,
    #[serde(default)]
    pub summary: Option<Value>,
}

#[derive(Clone, Debug, Default, Deserialize, PartialEq)]
pub struct OpenCodeTraceEvent {
    pub sequence: u64,
    pub received_at: String,
    #[serde(default)]
    pub kind: String,
    #[serde(default)]
    pub step: Option<u64>,
    #[serde(default)]
    pub tool: Option<String>,
    #[serde(default)]
    pub description: Option<String>,
    #[serde(default)]
    pub command: Option<String>,
    #[serde(default)]
    pub query: Option<String>,
    #[serde(default)]
    pub text: Option<String>,
    #[serde(default)]
    pub error: Option<String>,
    pub raw: Value,
}

#[component]
pub fn OpenCodeTracePanel(handle: RestateSyncHandle) -> Element {
    let mut collapsed = use_signal(|| true);
    let mut trace_key = use_signal(String::new);
    let mut poll_tick = use_signal(|| 0_u64);
    let enabled = *handle.enabled.read();
    let active_key = active_trace_key(handle);
    let active_key_available = active_key.is_some();
    let active_key_for_button = active_key.clone();
    let trace_handle = handle;

    let trace = use_resource(move || {
        let _tick = *poll_tick.read();
        let url = trace_handle.ingress_url.read().trim_end_matches('/').to_string();
        let key = trace_key.read().trim().to_string();
        async move { fetch_trace(&url, &key).await }
    });

    let poll_handle = handle;
    use_future(move || async move {
        loop {
            poll_sleep_ms(2000).await;
            if *poll_handle.enabled.read() && !trace_key.read().trim().is_empty() {
                let next = (*poll_tick.read()).saturating_add(1);
                poll_tick.set(next);
            }
        }
    });

    rsx! {
        div { class: "border-t border-slate-200 shrink-0",
            button {
                class: "flex w-full items-center justify-between px-3 py-2 hover:bg-slate-50 transition-colors",
                onclick: move |_| {
                    let next = !*collapsed.read();
                    collapsed.set(next);
                },
                div { class: "flex items-center gap-2",
                    span {
                        class: "text-slate-400 transition-transform",
                        style: if *collapsed.read() { "transform: rotate(-90deg);" } else { "" },
                        "v"
                    }
                    span { class: "text-[11px] font-semibold text-slate-600 uppercase tracking-wide", "OpenCode Trace" }
                    if enabled {
                        span { class: "w-2 h-2 rounded-full bg-emerald-500" }
                    } else {
                        span { class: "w-2 h-2 rounded-full bg-slate-300" }
                    }
                }
                span { class: "text-[10px] text-slate-400 font-mono", "{trace_status_label(&trace)}" }
            }

            if !*collapsed.read() {
                div { class: "px-3 py-2 border-b border-slate-100 flex flex-col gap-2",
                    div { class: "flex gap-2 items-end",
                        label { class: "flex flex-col gap-0.5 flex-1",
                            span { class: "text-[9px] font-semibold uppercase tracking-wide text-slate-400", "Workflow or bead key" }
                            input {
                                class: "text-[10px] border border-slate-200 rounded px-1.5 py-1 w-full font-mono bg-white",
                                value: "{trace_key.read()}",
                                placeholder: "workflow key or bead id",
                                oninput: move |event| trace_key.set(event.value()),
                            }
                        }
                        button {
                            class: "text-[10px] px-2 py-1 rounded border border-slate-200 text-slate-600 hover:bg-slate-50",
                            disabled: !active_key_available,
                            onclick: move |_| {
                                if let Some(key) = active_key_for_button.clone() {
                                    trace_key.set(key);
                                }
                            },
                            "Use active"
                        }
                    }
                    div { class: "text-[10px] text-slate-400 font-mono truncate", "POST {handle.ingress_url.read()}/OyaService/get_opencode_trace" }
                }

                div { class: "max-h-[320px] overflow-y-auto px-3 py-2",
                    if trace_key.read().trim().is_empty() {
                        div { class: "text-[11px] text-slate-400 text-center py-4", "Enter a workflow key or use an active invocation." }
                    } else {
                        {render_trace_result(&trace)}
                    }
                }
            }
        }
    }
}

async fn fetch_trace(base_url: &str, key: &str) -> Result<OpenCodeTraceSnapshot, String> {
    if key.is_empty() {
        return Ok(OpenCodeTraceSnapshot::default());
    }

    let url = format!("{base_url}/OyaService/get_opencode_trace");
    let body = serde_json::json!({ "key": key });
    let response = reqwest::Client::new()
        .post(url)
        .header("Content-Type", "application/json")
        .header("Accept", "application/json")
        .json(&body)
        .send()
        .await
        .map_err(|error| error.to_string())?;

    let status = response.status();
    if !status.is_success() {
        let message = match response.text().await {
            Ok(text) => text,
            Err(_) => status.to_string(),
        };
        return Err(format!("HTTP {}: {message}", status.as_u16()));
    }

    response.json::<OpenCodeTraceSnapshot>().await.map_err(|error| error.to_string())
}

fn active_trace_key(handle: RestateSyncHandle) -> Option<String> {
    handle
        .state
        .read()
        .invocations
        .values()
        .filter(|inv| matches!(inv.status, InvocationStatus::Running | InvocationStatus::Ready))
        .find(|inv| inv.target_service_name == "Oya" || inv.target.contains("Oya"))
        .and_then(|inv| inv.target_service_key.clone().or(Some(inv.id.clone())))
}

fn trace_status_label(resource: &Resource<Result<OpenCodeTraceSnapshot, String>>) -> String {
    match resource.read().as_ref() {
        Some(Ok(snapshot)) => snapshot.status.clone(),
        Some(Err(_)) => "error".to_string(),
        None => "loading".to_string(),
    }
}

fn render_trace_result(resource: &Resource<Result<OpenCodeTraceSnapshot, String>>) -> Element {
    match resource.read().as_ref() {
        Some(Ok(snapshot)) => render_trace_snapshot(snapshot),
        Some(Err(error)) => rsx! {
            div { class: "rounded border border-red-200 bg-red-50 px-2 py-2 text-[11px] text-red-700", "Trace request failed: {error}" }
        },
        None => rsx! {
            div { class: "text-[11px] text-slate-400 text-center py-4", "Loading trace..." }
        },
    }
}

fn render_trace_snapshot(snapshot: &OpenCodeTraceSnapshot) -> Element {
    rsx! {
        div { class: "flex flex-col gap-3",
            div { class: "grid grid-cols-2 gap-2 text-[10px]",
                {metric_cell("Status", &snapshot.status)}
                {metric_cell("Model", snapshot.model.as_deref().unwrap_or("unknown"))}
                {metric_cell("Invocation", snapshot.active_invocation_id.as_deref().unwrap_or("none"))}
                {metric_cell("Current", snapshot.current_event.as_ref().map_or("idle", |e| e.kind.as_str()))}
                {metric_cell("Tool calls", &snapshot.tool_call_count.to_string())}
                {metric_cell("Text events", &snapshot.text_event_count.to_string())}
            }

            if let Some(error) = &snapshot.last_error {
                div { class: "rounded border border-red-200 bg-red-50 px-2 py-1 text-[10px] text-red-700", "{error}" }
            }

            if snapshot.events.is_empty() {
                div { class: "text-[11px] text-slate-400 text-center py-4", "No OpenCode events have been persisted yet." }
            } else {
                div { class: "flex flex-col gap-2",
                    for event in snapshot.events.iter().rev().take(40) {
                        {render_trace_event(event)}
                    }
                }
            }

            if let Some(summary) = &snapshot.summary {
                div { class: "rounded border border-emerald-200 bg-emerald-50 px-2 py-2 text-[11px] text-emerald-800 whitespace-pre-wrap",
                    {format!("{}", summary)}
                }
            }
        }
    }
}

fn metric_cell(label: &str, value: &str) -> Element {
    rsx! {
        div { class: "rounded border border-slate-200 bg-slate-50 px-2 py-1 min-w-0",
            div { class: "text-[9px] uppercase tracking-wide text-slate-400", "{label}" }
            div { class: "font-mono text-slate-700 truncate", "{value}" }
        }
    }
}

fn render_trace_event(event: &OpenCodeTraceEvent) -> Element {
    rsx! {
        div { class: "rounded border border-slate-200 bg-white px-2 py-2 text-[10px]",
            div { class: "flex items-center justify-between gap-2 mb-1",
                span { class: "font-mono text-slate-400", "#{event.sequence}" }
                span { class: "rounded bg-slate-100 px-1.5 py-0.5 text-slate-600", "{event.kind}" }
            }
            if let Some(tool) = &event.tool {
                div { class: "font-semibold text-indigo-700", "{tool}" }
            }
            if let Some(description) = &event.description {
                div { class: "text-slate-700", "{description}" }
            }
            if let Some(command) = &event.command {
                pre { class: "mt-1 rounded bg-slate-950 text-slate-100 px-2 py-1 overflow-x-auto", "{command}" }
            }
            if let Some(query) = &event.query {
                div { class: "mt-1 font-mono text-slate-600 break-all", "{query}" }
            }
            if let Some(text) = &event.text {
                div { class: "mt-1 whitespace-pre-wrap text-slate-700", "{text}" }
            }
            if let Some(error) = &event.error {
                div { class: "mt-1 text-red-700", "{error}" }
            }
            pre { class: "mt-1 rounded bg-amber-50 text-amber-900 px-2 py-1 overflow-x-auto", "{event.raw}" }
        }
    }
}
