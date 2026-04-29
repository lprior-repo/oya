#![deny(clippy::unwrap_used)]
#![deny(clippy::expect_used)]
#![deny(clippy::panic)]
#![warn(clippy::pedantic)]
#![warn(clippy::nursery)]
#![forbid(unsafe_code)]

//! Restate Journal Viewer
//!
//! Displays journal entries with expandable input/output details

use crate::restate_client::types::{JournalEntry, JournalEntryType};
use dioxus::prelude::*;

#[allow(dead_code)]
const fn entry_type_color(entry_type: &JournalEntryType) -> &'static str {
    match entry_type {
        JournalEntryType::Call | JournalEntryType::OneWayCall => " bg-blue-100 text-blue-800",
        JournalEntryType::Sleep => " bg-purple-100 text-purple-800",
        JournalEntryType::Awakeable => " bg-orange-100 text-orange-800",
        JournalEntryType::GetState
        | JournalEntryType::SetState
        | JournalEntryType::ClearState
        | JournalEntryType::ClearAll => " bg-yellow-100 text-yellow-800",
        JournalEntryType::GetPromise
        | JournalEntryType::PeekPromise
        | JournalEntryType::CompletePromise => " bg-pink-100 text-pink-800",
        JournalEntryType::Custom => " bg-teal-100 text-teal-800",
        JournalEntryType::Unknown(_) => " bg-gray-100 text-gray-800",
    }
}

#[derive(Props, Clone, PartialEq, Eq)]
pub struct RestateJournalViewerProps {
    pub journal: Vec<JournalEntry>,
}

#[component]
pub fn RestateJournalViewer(props: RestateJournalViewerProps) -> Element {
    let mut expanded = use_signal(std::collections::HashSet::<u32>::new);

    rsx! {
        div {
            class: "flex flex-col gap-2",

            for entry in &props.journal {
                div {
                    class: "border border-gray-200 dark:border-gray-700 rounded-lg overflow-hidden",

                    // Header row
                    div {
                        class: {
                            let base = "flex items-center gap-3 p-3 cursor-pointer ";
                            let mut class = String::with_capacity(80);
                            class.push_str(base);
                            if expanded.read().contains(&entry.index) {
                                class.push_str("bg-gray-50 dark:bg-gray-800");
                            } else {
                                class.push_str("hover:bg-gray-50 dark:hover:bg-gray-800");
                            }
                            class
                        },
                        onclick: {
                            let idx = entry.index;
                            move |_| {
                                let mut set = expanded.read().clone();
                                if set.contains(&idx) {
                                    set.remove(&idx);
                                } else {
                                    set.insert(idx);
                                }
                                expanded.set(set);
                            }
                        },

                        // Index
                        span {
                            class: "font-mono text-sm text-gray-500 w-8",
                            {entry.index.to_string()}
                        }

                        // Entry type badge
                        span {
                            class: {
                                let mut class = String::with_capacity(80);
                                class.push_str("px-2 py-0.5 rounded text-xs font-medium ");
                                class.push_str(entry_type_color(&entry.entry_type));
                                class
                            },
                            {entry.raw_entry_type.clone()}
                        }

                        // Name
                        span {
                            class: "flex-1 font-medium",
                            {entry.name.clone().unwrap_or_else(|| "Unknown".to_string())}
                        }

                        // Status
                        span {
                            class: {
                                let mut class = String::with_capacity(64);
                                class.push_str("px-2 py-0.5 rounded text-xs ");
                                if entry.completed {
                                    class.push_str("bg-green-100 text-green-800");
                                } else {
                                    class.push_str("bg-yellow-100 text-yellow-800");
                                }
                                class
                            },
                            if entry.completed { "✓" } else { "○" }
                        }

                        // Expand indicator
                        span {
                            class: "text-gray-400",
                            if expanded.read().contains(&entry.index) { "▼" } else { "▶" }
                        }
                    }

                    // Expanded details
                    if expanded.read().contains(&entry.index) {
                        div {
                            class: "p-3 bg-gray-50 dark:bg-gray-900 border-t border-gray-200 dark:border-gray-700 space-y-3",

                            // Input
                            if let Some(input) = &entry.entry_json {
                                div {
                                    div { class: "text-sm font-medium text-gray-500 mb-1", "Input" }
                                    pre {
                                        class: "text-xs bg-white dark:bg-gray-800 p-2 rounded overflow-x-auto font-mono",
                                        {input.clone()}
                                    }
                                }
                            }

                            // Additional info
                            if let Some(target) = &entry.invoked_target {
                                div {
                                    div { class: "text-sm font-medium text-gray-500 mb-1", "Target" }
                                    div { class: "text-sm font-mono", {target.clone()} }
                                }
                            }

                            if let Some(id) = &entry.invoked_id {
                                div {
                                    div { class: "text-sm font-medium text-gray-500 mb-1", "Invocation ID" }
                                    div { class: "text-sm font-mono", {id.clone()} }
                                }
                            }

                            if let Some(promise) = &entry.promise_name {
                                div {
                                    div { class: "text-sm font-medium text-gray-500 mb-1", "Promise" }
                                    div { class: "text-sm", {promise.clone()} }
                                }
                            }

                            if let Some(wakeup) = entry.sleep_wakeup_at {
                                div {
                                    div { class: "text-sm font-medium text-gray-500 mb-1", "Wakeup At" }
                                    div { class: "text-sm", {format_time(wakeup)} }
                                }
                            }
                        }
                    }
                }
            }
        }
    }
}

#[allow(dead_code)]
fn format_time(ts: i64) -> String {
    chrono::DateTime::from_timestamp_millis(ts)
        .map_or_else(|| ts.to_string(), |dt| dt.format("%Y-%m-%d %H:%M:%S%.3f UTC").to_string())
}
