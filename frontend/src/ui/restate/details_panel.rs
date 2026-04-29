#![deny(clippy::unwrap_used)]
#![deny(clippy::expect_used)]
#![deny(clippy::panic)]
#![warn(clippy::pedantic)]
#![warn(clippy::nursery)]
#![forbid(unsafe_code)]

//! Restate Invocation Details Panel
//!
//! Shows detailed information about a Restate invocation including:
//! - Invocation ID, workflow name, status, timing
//! - Action buttons (Cancel, Kill, Pause, Resume, Purge)
//! - Confirmation dialogs for destructive actions (Kill, Purge)
//! - Inline feedback for action results
//! - Journal entries list

use crate::hooks::build_restate_config_from_url;
use crate::hooks::use_restate_sync::{poll_sleep_ms, use_restate_sync};
use crate::restate_client::types::{Invocation, InvocationAction, InvocationStatus, JournalEntry};
use crate::restate_client::RestateClient;
use crate::ui::restate::invocation_actions::{
    action_label, is_action_available, is_destructive, ActionFeedback, ConfirmDialog,
};
use crate::ui::restate::RestateJournalViewer;
use dioxus::prelude::*;

const fn status_to_ui_string(status: InvocationStatus) -> &'static str {
    match status {
        InvocationStatus::Pending => "pending",
        InvocationStatus::Scheduled => "scheduled",
        InvocationStatus::Ready => "ready",
        InvocationStatus::Running => "running",
        InvocationStatus::Paused => "paused",
        InvocationStatus::BackingOff => "backing-off",
        InvocationStatus::Suspended => "suspended",
        InvocationStatus::Completed => "completed",
    }
}

const fn action_button_class(action: InvocationAction) -> &'static str {
    match action {
        InvocationAction::Cancel => {
            "text-[10px] px-2 py-0.5 rounded border font-medium \
             bg-yellow-50 text-yellow-700 border-yellow-200 \
             hover:bg-yellow-100 transition-colors"
        }
        InvocationAction::Kill => {
            "text-[10px] px-2 py-0.5 rounded border font-medium \
             bg-red-50 text-red-700 border-red-200 \
             hover:bg-red-100 transition-colors"
        }
        InvocationAction::Pause => {
            "text-[10px] px-2 py-0.5 rounded border font-medium \
             bg-orange-50 text-orange-700 border-orange-200 \
             hover:bg-orange-100 transition-colors"
        }
        InvocationAction::Resume => {
            "text-[10px] px-2 py-0.5 rounded border font-medium \
             bg-emerald-50 text-emerald-700 border-emerald-200 \
             hover:bg-emerald-100 transition-colors"
        }
        InvocationAction::Purge => {
            "text-[10px] px-2 py-0.5 rounded border font-medium \
             bg-red-50 text-red-700 border-red-200 \
             hover:bg-red-100 transition-colors"
        }
    }
}

const fn disabled_class() -> &'static str {
    "opacity-50 cursor-not-allowed"
}

#[derive(Props, Clone, PartialEq)]
pub struct RestateInvocationDetailsProps {
    pub invocation_id: Signal<Option<String>>,
    pub handle: crate::hooks::RestateSyncHandle,
    pub journal: Signal<Vec<JournalEntry>>,
    pub admin_url: String,
    #[props(default)]
    pub loading: bool,
    pub on_close: EventHandler<()>,
}

#[component]
pub fn RestateInvocationDetails(props: RestateInvocationDetailsProps) -> Element {
    let restate = use_restate_sync();
    let invocations = restate.state.read().invocations.clone();
    let inv = props.invocation_id.read().as_ref().and_then(|id| invocations.get(id));
    let _is_active = inv.map_or(false, |i| i.status.is_active());

    let mut journal = props.journal;
    let status_str = inv.map_or("unknown", |i| status_to_ui_string(i.status));

    // Action feedback state
    let mut feedback: Signal<ActionFeedback> = use_signal(ActionFeedback::default);
    let mut confirm_dialog: Signal<ConfirmDialog> = use_signal(ConfirmDialog::default);

    // Auto-clear feedback after 3 seconds
    let feedback_val = (*feedback.read()).clone();
    if matches!(feedback_val, ActionFeedback::Success { .. } | ActionFeedback::Error { .. }) {
        let current_feedback = feedback_val.clone();
        spawn(async move {
            poll_sleep_ms(3000).await;
            // Only clear if feedback hasn't changed (user didn't fire another action)
            if *feedback.read() == current_feedback {
                feedback.set(ActionFeedback::Idle);
            }
        });
    }

    // Poll for journal updates if active
    let inv_id = props.invocation_id.read().clone();
    let admin_url = props.admin_url.clone();

    use_future(move || {
        let id = inv_id.clone();
        let url = admin_url.clone();
        async move {
            if id.is_some() {
                loop {
                    poll_sleep_ms(2000).await;

                    let restate = crate::hooks::use_restate_sync();
                    let invocations = restate.state.read().invocations.clone();
                    if let Some(inv) = id.as_ref().and_then(|id2| invocations.get(id2)) {
                        if !inv.status.is_active() {
                            break;
                        }
                    } else {
                        break;
                    }

                    let config = crate::hooks::build_restate_config_from_url(&url);
                    let client = crate::restate_client::RestateClient::new(config);
                    if let Some(ref inv_id) = id {
                        if let Ok(entries) = client.get_journal(inv_id).await {
                            if entries.len() != journal.read().len() {
                                journal.set(entries);
                            }
                        }
                    }
                }
            }
        }
    });

    let inv_status = inv.map_or(InvocationStatus::Pending, |i| i.status);
    let is_loading = matches!(*feedback.read(), ActionFeedback::Loading);
    let current_admin_url = props.admin_url.clone();
    let current_inv_id = props.invocation_id.read().clone().unwrap_or_default();

    rsx! {
        div {
            class: "fixed inset-0 bg-black/50 flex items-center justify-center z-50",
            onclick: move |_| props.on_close.call(()),

            div {
                class: "bg-white dark:bg-gray-900 rounded-lg shadow-xl max-w-4xl w-full max-h-[80vh] overflow-hidden",
                onclick: |_| {},

                // Header
                div {
                    class: "flex items-center justify-between p-4 border-b border-gray-200 dark:border-gray-700",

                    h2 {
                        class: "text-lg font-semibold",
                        "Restate Invocation Details"
                    }

                    // Action buttons
                    div {
                        class: "flex items-center gap-1.5",

                        for action in [
                            InvocationAction::Cancel,
                            InvocationAction::Kill,
                            InvocationAction::Pause,
                            InvocationAction::Resume,
                            InvocationAction::Purge,
                        ] {
                            if is_action_available(action, inv_status) {
                                {
                                    let btn_class = action_button_class(action);
                                    let label = action_label(action);
                                    let disabled_class_str = if is_loading {
                                        disabled_class()
                                    } else {
                                        ""
                                    };
                                    let needs_confirm = is_destructive(action);
                                    let btn_action = action;
                                    let btn_inv_id = current_inv_id.clone();
                                    let btn_admin_url = current_admin_url.clone();

                                    rsx! {
                                        button {
                                            class: "{btn_class} {disabled_class_str}",
                                            disabled: is_loading,
                                            onclick: move |_| {
                                                if is_loading {
                                                    return;
                                                }
                                                if needs_confirm {
                                                    match btn_action {
                                                        InvocationAction::Kill => {
                                                            confirm_dialog.set(ConfirmDialog::Kill);
                                                        }
                                                        InvocationAction::Purge => {
                                                            confirm_dialog.set(ConfirmDialog::Purge);
                                                        }
                                                        _ => {}
                                                    }
                                                } else {
                                                    fire_action(
                                                        btn_action,
                                                        &btn_inv_id,
                                                        &btn_admin_url,
                                                        feedback,
                                                    );
                                                }
                                            },
                                            {label}
                                        }
                                    }
                                }
                            }
                        }
                    }

                    button {
                        class: "p-2 hover:bg-gray-100 dark:hover:bg-gray-800 rounded",
                        onclick: move |_| props.on_close.call(()),
                        "✕"
                    }
                }

                // Feedback banner
                { render_feedback_banner(feedback) }

                // Content
                div {
                    class: "p-4 overflow-y-auto max-h-[calc(80vh-120px)]",

                    if let Some(inv) = inv {
                        // Invocation Info
                        div {
                            class: "grid grid-cols-2 gap-4 mb-6",

                            div {
                                class: "space-y-2",
                                div { class: "text-sm text-gray-500", "Invocation ID" }
                                div { class: "font-mono text-sm break-all", {inv.id.clone()} }
                            }

                            div {
                                class: "space-y-2",
                                div { class: "text-sm text-gray-500", "Workflow" }
                                div { class: "font-medium", {inv.target.clone()} }
                            }

                            div {
                                class: "space-y-2",
                                div { class: "text-sm text-gray-500", "Status" }
                                div {
                                    class: {
                                        let mut class = String::with_capacity(64);
                                        class.push_str("px-2 py-1 rounded text-sm ");
                                        match inv.status {
                                            InvocationStatus::Completed => class.push_str("bg-green-100 text-green-800"),
                                            InvocationStatus::Running => class.push_str("bg-blue-100 text-blue-800"),
                                            InvocationStatus::Paused | InvocationStatus::BackingOff => class.push_str("bg-red-100 text-red-800"),
                                            _ => class.push_str("bg-gray-100 text-gray-800"),
                                        }
                                        class
                                    },
                                    {status_str}
                                }
                            }

                            div {
                                class: "space-y-2",
                                div { class: "text-sm text-gray-500", "Started" }
                                div { class: "text-sm", {format_time(inv.created_at)} }
                            }

                            if let Some(finished) = inv.completed_at {
                                div {
                                    class: "space-y-2",
                                    div { class: "text-sm text-gray-500", "Finished" }
                                    div { class: "text-sm", {format_time(finished)} }
                                }
                            }

                            div {
                                class: "space-y-2",
                                div { class: "text-sm text-gray-500", "Journal Size" }
                                div { class: "text-sm", {inv.journal_size.to_string()} }
                            }

                            div {
                                class: "space-y-2",
                                div { class: "text-sm text-gray-500", "Trace ID" }
                                div { class: "font-mono text-sm break-all", {inv.trace_id.clone().unwrap_or_else(|| "none".to_string())} }
                            }

                            div {
                                class: "space-y-2",
                                div { class: "text-sm text-gray-500", "Caller" }
                                div { class: "text-sm", {caller_label(inv)} }
                            }

                            if let Some(code) = &inv.last_failure_error_code {
                                div {
                                    class: "space-y-2",
                                    div { class: "text-sm text-gray-500", "Failure Code" }
                                    div { class: "font-mono text-sm text-red-700", {code.clone()} }
                                }
                            }

                            if let Some(message) = &inv.last_failure {
                                div {
                                    class: "col-span-2 space-y-2",
                                    div { class: "text-sm text-gray-500", "Failure Message" }
                                    div { class: "rounded bg-red-50 border border-red-200 px-2 py-1 text-sm text-red-700 whitespace-pre-wrap", {message.clone()} }
                                }
                            }
                        }
                    } else {
                        div {
                            class: "p-4 text-center text-gray-500",
                            "Loading invocation..."
                        }
                    }

                    // Journal Entries
                    div {
                        class: "mt-6",
                        h3 {
                            class: "text-md font-semibold mb-3",
                            "Journal Entries"
                        }

                        if props.loading {
                            div {
                                class: "text-gray-500 text-sm",
                                "Loading journal\u{2026}"
                            }
                        } else if journal.read().is_empty() {
                            div {
                                class: "text-gray-500 text-sm",
                                "No journal entries"
                            }
                        } else {
                            RestateJournalViewer { journal: journal.read().clone() }
                        }
                    }
                }
            }
        }

        // Confirmation dialog overlay
        {
            let dialog = *confirm_dialog.read();
            match dialog {
                ConfirmDialog::None => rsx! {},
                ConfirmDialog::Kill | ConfirmDialog::Purge => {
                    let action = match dialog {
                        ConfirmDialog::Kill => InvocationAction::Kill,
                        ConfirmDialog::Purge => InvocationAction::Purge,
                        ConfirmDialog::None => unreachable!(),
                    };
                    let label = action_label(action);
                    let confirm_inv_id = current_inv_id.clone();
                    let confirm_admin_url = current_admin_url.clone();

                    rsx! {
                        div {
                            class: "fixed inset-0 bg-black/60 flex items-center justify-center z-[60]",
                            onclick: move |_| {
                                confirm_dialog.set(ConfirmDialog::None);
                            },

                            div {
                                class: "bg-white dark:bg-gray-900 rounded-lg shadow-xl max-w-sm w-full p-6",
                                onclick: |_| {},

                                h3 {
                                    class: "text-lg font-semibold mb-2",
                                    "Confirm {label}"
                                }

                                p {
                                    class: "text-sm text-gray-600 dark:text-gray-400 mb-1",
                                    "Are you sure you want to {label} this invocation?"
                                }

                                p {
                                    class: "font-mono text-xs text-gray-400 mb-4 truncate",
                                    "{confirm_inv_id}"
                                }

                                div {
                                    class: "flex justify-end gap-2",

                                    button {
                                        class: "text-sm px-3 py-1.5 rounded border border-gray-300 \
                                               hover:bg-gray-100 dark:hover:bg-gray-800 transition-colors",
                                        onclick: move |_| {
                                            confirm_dialog.set(ConfirmDialog::None);
                                        },
                                        "Cancel"
                                    }

                                    button {
                                        class: "text-sm px-3 py-1.5 rounded border font-medium \
                                               bg-red-50 text-red-700 border-red-200 \
                                               hover:bg-red-100 transition-colors",
                                        onclick: move |_| {
                                            confirm_dialog.set(ConfirmDialog::None);
                                            fire_action(
                                                action,
                                                &confirm_inv_id,
                                                &confirm_admin_url,
                                                feedback,
                                            );
                                        },
                                        {label}
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

fn render_feedback_banner(feedback: Signal<ActionFeedback>) -> Element {
    let fb = (*feedback.read()).clone();
    match fb {
        ActionFeedback::Idle => rsx! {},
        ActionFeedback::Loading => rsx! {
            div {
                class: "px-4 py-2 border-b border-gray-200 dark:border-gray-700 \
                       bg-blue-50 text-blue-700 text-[11px]",
                "Sending action\u{2026}"
            }
        },
        ActionFeedback::Success { action } => {
            let label = action_label(action);
            rsx! {
                div {
                    class: "px-4 py-2 border-b border-gray-200 dark:border-gray-700 \
                           bg-emerald-50 text-emerald-700 text-[11px]",
                    "\u{2713} {label} succeeded"
                }
            }
        }
        ActionFeedback::Error { action, message } => {
            let label = action_label(action);
            rsx! {
                div {
                    class: "px-4 py-2 border-b border-gray-200 dark:border-gray-700 \
                           bg-red-50 text-red-700 text-[11px]",
                    "\u{2717} {label} failed: {message}"
                }
            }
        }
    }
}

fn fire_action(
    action: InvocationAction,
    inv_id: &str,
    admin_url: &str,
    mut feedback: Signal<ActionFeedback>,
) {
    feedback.set(ActionFeedback::Loading);
    let id = inv_id.to_string();
    let url = admin_url.to_string();

    spawn(async move {
        let config = build_restate_config_from_url(&url);
        let client = RestateClient::new(config);
        let result = match action {
            InvocationAction::Cancel => client.cancel_invocation(&id).await,
            InvocationAction::Kill => client.kill_invocation(&id).await,
            InvocationAction::Pause => client.pause_invocation(&id).await,
            InvocationAction::Resume => client.resume_invocation(&id).await,
            InvocationAction::Purge => client.purge_invocation(&id).await,
        };
        match result {
            Ok(_) => {
                feedback.set(ActionFeedback::Success { action });
            }
            Err(e) => {
                feedback.set(ActionFeedback::Error { action, message: e.to_string() });
            }
        }
    });
}

fn caller_label(invocation: &Invocation) -> String {
    let caller = invocation
        .invoked_by_service_name
        .as_deref()
        .or(invocation.invoked_by_id.as_deref())
        .unwrap_or("external");
    format!("{:?}: {caller}", invocation.invoked_by)
}

fn format_time(ts: i64) -> String {
    chrono::DateTime::from_timestamp_millis(ts)
        .map_or_else(|| ts.to_string(), |dt| dt.format("%Y-%m-%d %H:%M:%S UTC").to_string())
}
