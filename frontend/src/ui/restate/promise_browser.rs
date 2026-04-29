#![deny(clippy::unwrap_used)]
#![deny(clippy::expect_used)]
#![deny(clippy::panic)]
#![warn(clippy::pedantic)]
#![forbid(unsafe_code)]

//! Promise Browser Panel — inspect Restate workflow promises.
//!
//! Provides service name + key text inputs and a "Load Promises" button.
//! Displays a table of promises with: name, completed status, result value/failure.

use crate::hooks::{build_restate_config_from_url, RestateSyncHandle};
use crate::restate_client::types::PromiseInfo;
use crate::restate_client::RestateClient;
use dioxus::prelude::*;

/// Format bytes as a lossy UTF-8 string, falling back to hex representation.
fn format_bytes(bytes: &[u8]) -> String {
    match std::str::from_utf8(bytes) {
        Ok(s) => s.to_string(),
        Err(_) => bytes.iter().map(|b| format!("{b:02x}")).collect(),
    }
}

#[derive(Props, Clone, PartialEq)]
pub struct PromiseBrowserPanelProps {
    pub handle: RestateSyncHandle,
}

#[component]
pub fn PromiseBrowserPanel(props: PromiseBrowserPanelProps) -> Element {
    let mut collapsed = use_signal(|| true);
    let mut service_name = use_signal(String::new);
    let mut service_key = use_signal(String::new);
    let mut promises: Signal<Vec<PromiseInfo>> = use_signal(Vec::new);
    let mut loading = use_signal(|| false);
    let mut error_msg: Signal<Option<String>> = use_signal(|| None);
    let mut loaded = use_signal(|| false);

    rsx! {
        div { class: "border-t border-slate-200 shrink-0",

            // Panel header
            button {
                class: "flex w-full items-center justify-between px-3 py-2 hover:bg-slate-50 transition-colors",
                onclick: move |_| {
                    let current = *collapsed.read();
                    collapsed.set(!current);
                },

                div { class: "flex items-center gap-2",
                    span {
                        class: "text-slate-400 transition-transform",
                        style: if *collapsed.read() { "transform: rotate(-90deg);" } else { "" },
                        "▾"
                    }
                    span { class: "text-[11px] font-semibold text-slate-600 uppercase tracking-wide",
                        "Promises"
                    }
                    if !promises.read().is_empty() {
                        span { class: "text-[10px] text-slate-400",
                            "({promises.read().len()})"
                        }
                    }
                }
            }

            // Panel body
            if !*collapsed.read() {
                // Input row
                div { class: "px-3 py-1.5 border-b border-slate-100 flex gap-1.5 items-end",
                    div { class: "flex flex-col gap-0.5 flex-1",
                        label { class: "text-[9px] font-semibold uppercase tracking-wide text-slate-400", "Service" }
                        input {
                            class: "text-[10px] border border-slate-200 rounded px-1.5 py-0.5 w-full font-mono bg-white",
                            r#type: "text",
                            placeholder: "WorkflowService",
                            value: "{service_name.read()}",
                            oninput: move |e| service_name.set(e.value()),
                        }
                    }
                    div { class: "flex flex-col gap-0.5 flex-1",
                        label { class: "text-[9px] font-semibold uppercase tracking-wide text-slate-400", "Key" }
                        input {
                            class: "text-[10px] border border-slate-200 rounded px-1.5 py-0.5 w-full font-mono bg-white",
                            r#type: "text",
                            placeholder: "order-123",
                            value: "{service_key.read()}",
                            oninput: move |e| service_key.set(e.value()),
                        }
                    }
                    button {
                        class: if *loading.read() {
                            "text-[10px] px-2.5 py-1 rounded border font-medium bg-slate-100 text-slate-400 border-slate-200 cursor-not-allowed"
                        } else {
                            "text-[10px] px-2.5 py-1 rounded border font-medium bg-indigo-50 text-indigo-600 border-indigo-200 hover:bg-indigo-100 transition-colors"
                        },
                        disabled: *loading.read(),
                        onclick: move |_| {
                            let name = service_name.read().trim().to_string();
                            let key = service_key.read().trim().to_string();

                            if name.is_empty() || key.is_empty() {
                                error_msg.set(Some("Enter both service name and key".to_string()));
                                return;
                            }

                            loading.set(true);
                            error_msg.set(None);
                            loaded.set(false);
                            promises.set(vec![]);

                            let admin_url = props.handle.admin_url.read().clone();
                            spawn(async move {
                                let config = build_restate_config_from_url(&admin_url);
                                let client = RestateClient::new(config);
                                match client.get_promises(&name, &key).await {
                                    Ok(result) => {
                                        promises.set(result);
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
                        if *loading.read() { "Loading…" } else { "Load" }
                    }
                }

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
                            "Loading promises…"
                        }
                    } else if !*loaded.read() {
                        // Initial state
                        div { class: "px-3 py-3 text-[11px] text-slate-400 text-center",
                            "Enter service name and key to load promises."
                        }
                    } else if promises.read().is_empty() {
                        // Empty result state
                        div { class: "px-3 py-3 text-[11px] text-slate-400 text-center",
                            "No promises found for this service/key."
                        }
                    } else {
                        // Promise table
                        table { class: "w-full border-collapse text-left",
                            thead {
                                tr { class: "bg-slate-50 border-b border-slate-200",
                                    th { class: "text-[10px] font-semibold text-slate-400 uppercase tracking-wide px-3 py-1.5", "Name" }
                                    th { class: "text-[10px] font-semibold text-slate-400 uppercase tracking-wide px-3 py-1.5 w-16", "Done" }
                                    th { class: "text-[10px] font-semibold text-slate-400 uppercase tracking-wide px-3 py-1.5", "Result" }
                                }
                            }
                            tbody {
                                for promise in promises.read().iter() {
                                    {
                                        let name = promise.key.clone();
                                        let done = promise.completed;
                                        let result_text = if promise.completed {
                                            match (&promise.completion_success_value, &promise.completion_failure) {
                                                (Some(bytes), _) => format_bytes(bytes),
                                                (None, Some(failure)) => failure.clone(),
                                                (None, None) => "—".to_string(),
                                            }
                                        } else {
                                            String::new()
                                        };

                                        rsx! {
                                            tr {
                                                key: "{name}",
                                                class: "border-b border-slate-100 last:border-b-0 hover:bg-slate-50 transition-colors",

                                                td { class: "px-3 py-1.5 font-mono text-[10px] text-slate-600 max-w-[100px] truncate",
                                                    "{name}"
                                                }
                                                td { class: "px-3 py-1.5 text-center",
                                                    if done {
                                                        span { class: "text-emerald-600 text-[11px]", "✓" }
                                                    } else {
                                                        span { class: "text-slate-300 text-[11px]", "✗" }
                                                    }
                                                }
                                                td { class: "px-3 py-1.5 text-[10px] text-slate-600 max-w-[140px] truncate",
                                                    if done {
                                                        "{result_text}"
                                                    } else {
                                                        span { class: "text-slate-400 italic", "pending" }
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
}
