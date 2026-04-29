#![deny(clippy::unwrap_used)]
#![deny(clippy::expect_used)]
#![deny(clippy::panic)]
#![warn(clippy::pedantic)]
#![forbid(unsafe_code)]

//! Collapsible side-panel showing live Restate invocations.
//!
//! - Toggle button enables/disables polling.
//! - Green/red dot shows connection health.
//! - Clicking a row opens `RestateInvocationDetails`.

use crate::hooks::build_restate_config_from_url;
use crate::hooks::RestateSyncHandle;
use crate::restate_client::types::{InvocationStatus, JournalEntry};
use crate::restate_client::RestateClient;
use crate::ui::restate::{RestateInvocationDetails, StateBrowserPanel};
use dioxus::prelude::*;

const fn status_dot_class(connected: bool) -> &'static str {
    if connected {
        "w-2 h-2 rounded-full bg-emerald-500"
    } else {
        "w-2 h-2 rounded-full bg-red-400"
    }
}

const fn invocation_status_label(status: InvocationStatus) -> &'static str {
    match status {
        InvocationStatus::Pending => "pending",
        InvocationStatus::Scheduled => "scheduled",
        InvocationStatus::Ready => "ready",
        InvocationStatus::Running => "running",
        InvocationStatus::Paused => "paused",
        InvocationStatus::BackingOff => "retrying",
        InvocationStatus::Suspended => "suspended",
        InvocationStatus::Completed => "done",
    }
}

const fn status_badge_class(status: InvocationStatus) -> &'static str {
    match status {
        InvocationStatus::Running | InvocationStatus::BackingOff => {
            "text-[10px] font-semibold px-1.5 py-0.5 rounded border bg-blue-50 text-blue-700 border-blue-200"
        }
        InvocationStatus::Completed => {
            "text-[10px] font-semibold px-1.5 py-0.5 rounded border bg-emerald-50 text-emerald-700 border-emerald-200"
        }
        InvocationStatus::Paused => {
            "text-[10px] font-semibold px-1.5 py-0.5 rounded border bg-red-50 text-red-700 border-red-200"
        }
        _ => "text-[10px] font-semibold px-1.5 py-0.5 rounded border bg-slate-50 text-slate-600 border-slate-200",
    }
}

fn truncate_inv_id(id: &str) -> String {
    let clean: String = id.chars().filter(|c| *c != '-').take(10).collect();
    clean
}

#[component]
pub fn RestateInvocationsPanel(handle: RestateSyncHandle) -> Element {
    let mut collapsed = use_signal(|| true);
    let mut selected_inv_id: Signal<Option<String>> = use_signal(|| None);
    let mut journal: Signal<Vec<JournalEntry>> = use_signal(Vec::new);
    let mut journal_loading = use_signal(|| false);

    let state = handle.state.read();
    let connected = state.connected;
    let invocations = state.invocations.clone();
    let last_error = state.last_error.clone();
    let enabled = *handle.enabled.read();
    let count = invocations.len();

    // Resolve selected invocation from state reactively
    let _selected_inv = selected_inv_id.read().as_ref().and_then(|id| invocations.get(id).cloned());
    drop(state);

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
                    // Collapse chevron
                    span {
                        class: "text-slate-400 transition-transform",
                        style: if *collapsed.read() { "transform: rotate(-90deg);" } else { "" },
                        "▾"
                    }
                    span { class: "text-[11px] font-semibold text-slate-600 uppercase tracking-wide",
                        "Restate"
                    }
                    // Connection dot
                    if enabled {
                        span { class: status_dot_class(connected) }
                    }
                    if count > 0 {
                        span { class: "text-[10px] text-slate-400", "({count})" }
                    }
                }

                // Enable/disable toggle - stop propagation so it doesn't collapse the panel
                div {
                    onclick: move |evt| { evt.stop_propagation(); },
                    button {
                        class: if enabled {
                            "text-[10px] px-2 py-0.5 rounded border font-medium bg-indigo-50 text-indigo-600 border-indigo-200 hover:bg-indigo-100 transition-colors"
                        } else { "text-[10px] px-2 py-0.5 rounded border font-medium bg-slate-50 text-slate-500 border-slate-200 hover:bg-slate-100 transition-colors" },
                        onclick: move |_| {
                            handle.enabled.set(!enabled);
                        },
                        if enabled { "On" } else { "Off" }
                    }
                }
            }

            // Panel body
            if !*collapsed.read() {
                // URL config row
                div { class: "px-3 py-1.5 border-b border-slate-100 flex gap-2 items-center",
                    div { class: "flex flex-col gap-0.5 flex-1",
                        label { class: "text-[9px] font-semibold uppercase tracking-wide text-slate-400", "Admin" }
                        input {
                            class: "text-[10px] border border-slate-200 rounded px-1.5 py-0.5 w-full font-mono bg-white",
                            value: "{handle.admin_url.read()}",
                            oninput: move |e| handle.admin_url.set(e.value()),
                        }
                    }
                    div { class: "flex flex-col gap-0.5 flex-1",
                        label { class: "text-[9px] font-semibold uppercase tracking-wide text-slate-400", "Ingress" }
                        input {
                            class: "text-[10px] border border-slate-200 rounded px-1.5 py-0.5 w-full font-mono bg-white",
                            value: "{handle.ingress_url.read()}",
                            oninput: move |e| handle.ingress_url.set(e.value()),
                        }
                    }
                }

                div { class: "max-h-[200px] overflow-y-auto",

                    // Error message
                    if let Some(err) = &last_error {
                        div { class: "mx-3 mb-2 rounded bg-red-50 border border-red-200 px-2 py-1 text-[10px] text-red-700",
                            "Connection error: {err}"
                        }
                    }

                    if !enabled {
                        div { class: "px-3 py-3 text-[11px] text-slate-400 text-center",
                            "Enable polling to see live invocations."
                        }
                    } else if invocations.is_empty() {
                        div { class: "px-3 py-3 text-[11px] text-slate-400 text-center",
                            if connected {
                                "No active invocations."
                            } else {
                                "Connecting to Restate…"
                            }
                        }
                    } else {
                        table { class: "w-full border-collapse text-left",
                            thead {
                                tr { class: "bg-slate-50 border-b border-slate-200",
                                    th { class: "text-[10px] font-semibold text-slate-400 uppercase tracking-wide px-3 py-1.5", "ID" }
                                    th { class: "text-[10px] font-semibold text-slate-400 uppercase tracking-wide px-3 py-1.5", "Target" }
                                    th { class: "text-[10px] font-semibold text-slate-400 uppercase tracking-wide px-3 py-1.5", "Status" }
                                }
                            }
                            tbody {
                                {
                                    let mut items: Vec<_> = invocations.values().collect();
                                    items.sort_by_key(|inv| std::cmp::Reverse(inv.modified_at));

                                    rsx! {
                                        for inv in items {
                                            {
                                                let inv_id = inv.id.clone();
                                                let inv_id_short = truncate_inv_id(&inv.id);
                                                let target = inv.target.clone();
                                                let status = inv.status;
                                                let badge_class = status_badge_class(status);
                                                let status_label = invocation_status_label(status);
                                                let is_selected = selected_inv_id.read().as_ref() == Some(&inv_id);

                                                rsx! {
                                                    tr {
                                                        key: "{inv_id}",
                                                        class: if is_selected { "bg-indigo-50/50" } else { "cursor-pointer hover:bg-slate-50 transition-colors" },
                                                        class: "border-b border-slate-100 last:border-b-0",
                                                        onclick: move |_| {
                                                            selected_inv_id.set(Some(inv_id.clone()));
                                                            journal.set(vec![]);
                                                            journal_loading.set(true);
                                                            let id = inv_id.clone();
                                                            let admin_url = handle.admin_url.read().clone();
                                                            spawn(async move {
                                                                let config = build_restate_config_from_url(&admin_url);
                                                                let client = RestateClient::new(config);
                                                                match client.get_journal(&id).await {
                                                                    Ok(entries) => journal.set(entries),
                                                                    Err(_) => journal.set(vec![]),
                                                                }
                                                                journal_loading.set(false);
                                                            });
                                                        },

                                                        td { class: "px-3 py-1.5 font-mono text-[10px] text-slate-600",
                                                            "{inv_id_short}"
                                                        }
                                                        td { class: "px-3 py-1.5 text-[10px] text-slate-600 max-w-[120px] truncate",
                                                            "{target}"
                                                        }
                                                        td { class: "px-3 py-1.5",
                                                            span { class: "{badge_class}", "{status_label}" }
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

        // State Browser — rendered as separate collapsible section
        StateBrowserPanel {
            admin_url: handle.admin_url.read().clone(),
        }

        // Details modal — rendered outside the panel so it floats over everything
        if selected_inv_id.read().is_some() {
            RestateInvocationDetails {
                invocation_id: selected_inv_id,
                handle: handle,
                journal: journal,
                admin_url: handle.admin_url.read().clone(),
                loading: *journal_loading.read(),
                on_close: move |()| {
                    selected_inv_id.set(None);
                    journal.set(vec![]);
                }
            }
        }
    }
}
