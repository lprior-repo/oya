#![deny(clippy::unwrap_used)]
#![deny(clippy::expect_used)]
#![deny(clippy::panic)]
#![warn(clippy::pedantic)]
#![forbid(unsafe_code)]

//! Read-only Oya lifecycle status panel.

#[cfg(target_arch = "wasm32")]
use crate::hooks::RestateSyncHandle;
#[cfg(target_arch = "wasm32")]
use crate::restate_client::types::InvocationStatus;
#[cfg(target_arch = "wasm32")]
use crate::restate_client::{LifecycleStatusClient, LifecycleStatusClientConfig};
#[cfg(target_arch = "wasm32")]
use crate::ui::restate::lifecycle_status_model::{
    is_good_status, lifecycle_summary, LifecycleStatusSummary,
};
#[cfg(target_arch = "wasm32")]
use dioxus::prelude::*;
#[cfg(target_arch = "wasm32")]
use oya_contracts::LifecycleGateSnapshot;
#[cfg(target_arch = "wasm32")]
use oya_contracts::{LifecycleStatusSnapshot, LifecycleStepSnapshot};

#[cfg(target_arch = "wasm32")]
#[derive(Props, Clone, PartialEq)]
pub struct LifecycleStatusPanelProps {
    pub handle: RestateSyncHandle,
}

#[cfg(target_arch = "wasm32")]
#[component]
pub fn LifecycleStatusPanel(props: LifecycleStatusPanelProps) -> Element {
    let mut collapsed = use_signal(|| false);
    let mut refresh_tick = use_signal(|| 0_u64);
    let handle = props.handle;
    let selected_run = selected_run_key(handle);

    let lifecycle = use_resource(move || {
        let _tick = *refresh_tick.read();
        let ingress_url = handle.ingress_url.read().clone();
        async move { fetch_lifecycle_snapshot(ingress_url).await }
    });

    rsx! {
        div { class: "border-t border-slate-200 shrink-0",
            button {
                class: "flex w-full items-center justify-between px-3 py-2 hover:bg-slate-50 transition-colors",
                onclick: move |_| {
                    let next = {
                        let current = *collapsed.read();
                        !current
                    };
                    collapsed.set(next);
                },
                div { class: "flex items-center gap-2",
                    span {
                        class: "text-slate-400 transition-transform",
                        style: if *collapsed.read() { "transform: rotate(-90deg);" } else { "" },
                        "v"
                    }
                    span { class: "text-[11px] font-semibold text-slate-600 uppercase tracking-wide", "Oya Lifecycle" }
                    span { class: status_dot_class(&lifecycle) }
                    span { class: "text-[9px] rounded border border-slate-200 bg-slate-50 px-1.5 py-0.5 uppercase tracking-wide text-slate-500", "Read-only" }
                }
                div {
                    onclick: move |evt| evt.stop_propagation(),
                    button {
                        class: "text-[10px] px-2 py-0.5 rounded border font-medium bg-indigo-50 text-indigo-600 border-indigo-200 hover:bg-indigo-100 transition-colors",
                        onclick: move |_| {
                            let next = {
                                let current = *refresh_tick.read();
                                current.saturating_add(1)
                            };
                            refresh_tick.set(next);
                        },
                        "Refresh"
                    }
                }
            }

            if !*collapsed.read() {
                div { class: "px-3 py-1.5 border-b border-slate-100 flex flex-col gap-1",
                    div { class: "text-[9px] font-semibold uppercase tracking-wide text-slate-400", "Ingress status endpoint" }
                    div { class: "text-[10px] text-slate-500 font-mono truncate", "POST {handle.ingress_url.read()}/OyaService/get_lifecycle" }
                }
                div { class: "max-h-[320px] overflow-y-auto px-3 py-2",
                    {render_lifecycle_result(&lifecycle, selected_run.as_deref())}
                }
            }
        }
    }
}

#[cfg(target_arch = "wasm32")]
async fn fetch_lifecycle_snapshot(ingress_url: String) -> Result<LifecycleStatusSnapshot, String> {
    let client =
        LifecycleStatusClient::new(LifecycleStatusClientConfig { ingress_url, timeout_secs: 10 });
    client.get_lifecycle().await.map_err(|error| error.to_string())
}

#[cfg(target_arch = "wasm32")]
fn selected_run_key(handle: RestateSyncHandle) -> Option<String> {
    handle
        .state
        .read()
        .invocations
        .values()
        .filter(|inv| matches!(inv.status, InvocationStatus::Running | InvocationStatus::Ready))
        .find(|inv| inv.target_service_name == "Oya" || inv.target.contains("Oya"))
        .map(|inv| selected_invocation_label(inv.target_service_key.as_deref(), &inv.id))
}

#[cfg(target_arch = "wasm32")]
fn selected_invocation_label(service_key: Option<&str>, invocation_id: &str) -> String {
    service_key
        .filter(|key| !key.trim().is_empty())
        .map_or_else(|| invocation_id.to_owned(), ToOwned::to_owned)
}

#[cfg(target_arch = "wasm32")]
fn status_dot_class(resource: &Resource<Result<LifecycleStatusSnapshot, String>>) -> &'static str {
    match resource.read().as_ref() {
        Some(Ok(snapshot)) if snapshot.success == Some(false) => "w-2 h-2 rounded-full bg-red-500",
        Some(Ok(snapshot)) if snapshot.done => "w-2 h-2 rounded-full bg-emerald-500",
        Some(Ok(_)) => "w-2 h-2 rounded-full bg-blue-500",
        Some(Err(_)) => "w-2 h-2 rounded-full bg-red-400",
        None => "w-2 h-2 rounded-full bg-slate-300",
    }
}

#[cfg(target_arch = "wasm32")]
fn render_lifecycle_result(
    resource: &Resource<Result<LifecycleStatusSnapshot, String>>,
    selected_run: Option<&str>,
) -> Element {
    match resource.read().as_ref() {
        Some(Ok(snapshot)) => render_lifecycle_snapshot(snapshot, selected_run),
        Some(Err(error)) => rsx! {
            div { class: "rounded border border-red-200 bg-red-50 px-2 py-2 text-[11px] text-red-700", "Lifecycle status request failed: {error}" }
        },
        None => rsx! {
            div { class: "text-[11px] text-slate-400 text-center py-4", "Loading lifecycle status..." }
        },
    }
}

#[cfg(target_arch = "wasm32")]
fn render_lifecycle_snapshot(
    snapshot: &LifecycleStatusSnapshot,
    selected_run: Option<&str>,
) -> Element {
    let summary = lifecycle_summary(snapshot, selected_run);
    rsx! {
        div { class: "flex flex-col gap-3",
            div { class: "flex items-center justify-between gap-2",
                div { class: "min-w-0",
                    div { class: "text-[9px] uppercase tracking-wide text-slate-400", "Selected run" }
                    div { class: "font-mono text-[11px] text-slate-700 truncate", "{summary.run_label}" }
                }
                span { class: "{summary.badge_class}", "{summary.status_label}" }
            }
            if let Some(message) = &summary.message {
                div { class: "rounded border border-indigo-200 bg-indigo-50 px-2 py-1 text-[10px] text-indigo-800", "{message}" }
            }
            {render_summary_grid(&summary)}
            {render_step_list(&snapshot.steps)}
            {render_gate_list("Service gates", &snapshot.gates)}
            {render_gate_list("Discipline gates", &snapshot.discipline_gates)}
        }
    }
}

#[cfg(target_arch = "wasm32")]
fn render_summary_grid(summary: &LifecycleStatusSummary) -> Element {
    rsx! {
        div { class: "grid grid-cols-1 gap-2 text-[10px]",
            {metric_cell("Steps", &summary.progress_label)}
            {metric_cell("Gates", &summary.gate_label)}
            {metric_cell("Compensation", &summary.compensation_label)}
        }
    }
}

#[cfg(target_arch = "wasm32")]
fn render_step_list(steps: &[LifecycleStepSnapshot]) -> Element {
    rsx! {
        div { class: "flex flex-col gap-1.5",
            div { class: "text-[9px] font-semibold uppercase tracking-wide text-slate-400", "Lifecycle steps" }
            if steps.is_empty() {
                div { class: "text-[11px] text-slate-400 py-2", "No lifecycle steps reported." }
            } else {
                for step in steps.iter().rev().take(8) {
                    {render_step(step)}
                }
            }
        }
    }
}

#[cfg(target_arch = "wasm32")]
fn render_step(step: &LifecycleStepSnapshot) -> Element {
    let badge_class = step_badge_class(&step.status);
    rsx! {
        div { class: "rounded border border-slate-200 bg-white px-2 py-2 text-[10px]",
            div { class: "flex items-center justify-between gap-2",
                span { class: "font-mono text-slate-700 truncate", "{step.step}" }
                span { class: "{badge_class}", "{step.status}" }
            }
            if let Some(message) = &step.message {
                div { class: "mt-1 text-slate-600", "{message}" }
            }
        }
    }
}

#[cfg(target_arch = "wasm32")]
fn render_gate_list(title: &str, gates: &[LifecycleGateSnapshot]) -> Element {
    rsx! {
        if !gates.is_empty() {
            div { class: "flex flex-col gap-1.5",
                div { class: "text-[9px] font-semibold uppercase tracking-wide text-slate-400", "{title}" }
                for gate in gates.iter().take(8) {
                    div { class: "rounded border border-slate-200 bg-slate-50 px-2 py-1 text-[10px] flex items-center justify-between gap-2",
                        span { class: "font-mono text-slate-700 truncate", "{gate.gate_id}" }
                        span { class: "{step_badge_class(&gate.status)}", "{gate.status}" }
                    }
                }
            }
        }
    }
}

#[cfg(target_arch = "wasm32")]
fn metric_cell(label: &str, value: &str) -> Element {
    rsx! {
        div { class: "rounded border border-slate-200 bg-slate-50 px-2 py-1 min-w-0",
            div { class: "text-[9px] uppercase tracking-wide text-slate-400", "{label}" }
            div { class: "font-mono text-slate-700 truncate", "{value}" }
        }
    }
}

#[cfg(target_arch = "wasm32")]
fn step_badge_class(status: &str) -> &'static str {
    if is_good_status(status) {
        "text-[10px] font-semibold px-1.5 py-0.5 rounded border bg-emerald-50 text-emerald-700 border-emerald-200"
    } else if status.eq_ignore_ascii_case("failed") || status.eq_ignore_ascii_case("error") {
        "text-[10px] font-semibold px-1.5 py-0.5 rounded border bg-red-50 text-red-700 border-red-200"
    } else {
        "text-[10px] font-semibold px-1.5 py-0.5 rounded border bg-blue-50 text-blue-700 border-blue-200"
    }
}
