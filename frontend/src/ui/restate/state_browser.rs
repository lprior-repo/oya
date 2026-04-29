#![deny(clippy::unwrap_used)]
#![deny(clippy::expect_used)]
#![deny(clippy::panic)]
#![warn(clippy::pedantic)]
#![warn(clippy::nursery)]
#![forbid(unsafe_code)]
#![allow(dead_code)]

//! Restate State Browser Panel
//!
//! Allows users to inspect Restate Virtual Object / Workflow state.
//! Provides service name and optional key inputs, queries state via
//! [`RestateClient`](crate::restate_client::RestateClient), and displays
//! results in a table with JSON pretty-printing.

use crate::restate_client::types::StateEntry;

// ---------------------------------------------------------------------------
// Pure helper functions (Data → Calc layer)
// ---------------------------------------------------------------------------

/// Format a [`StateEntry`]\'s value for display.
///
/// - If `value_utf8` is valid JSON, returns pretty-printed JSON.
/// - If `value_utf8` is non-JSON text, returns it as-is.
/// - If `value_utf8` is `None` but `value` has bytes, shows byte count.
/// - If both are `None`, shows "(empty)".
pub(crate) fn format_value_display(entry: &StateEntry) -> String {
    entry.value_utf8.as_ref().map_or_else(
        || {
            entry
                .value
                .as_ref()
                .map_or_else(|| "(empty)".to_string(), |bytes| format!("({} bytes)", bytes.len()))
        },
        |utf8| {
            serde_json::from_str::<serde_json::Value>(utf8).map_or_else(
                |_| utf8.clone(),
                |parsed| serde_json::to_string_pretty(&parsed).unwrap_or_else(|_| utf8.clone()),
            )
        },
    )
}

/// Filter state entries by optional service key (client-side filtering).
///
/// When `key` is empty, returns all entries unchanged.
/// When `key` is non-empty, returns only entries whose `service_key` matches.
pub(crate) fn filter_by_service_key(entries: Vec<StateEntry>, key: &str) -> Vec<StateEntry> {
    if key.is_empty() {
        entries
    } else {
        entries.into_iter().filter(|e| e.service_key.as_deref() == Some(key)).collect()
    }
}

// ---------------------------------------------------------------------------
// Load state machine
// ---------------------------------------------------------------------------

/// Represents the current state of the state browser query.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum BrowserLoadState {
    /// Initial state, no query attempted.
    Idle,
    /// Query in flight.
    Loading,
    /// Query succeeded with results.
    Loaded(Vec<StateEntry>),
    /// Query failed.
    Error(String),
}

// ---------------------------------------------------------------------------
// Component (wasm32 only)
// ---------------------------------------------------------------------------

#[cfg(target_arch = "wasm32")]
mod wasm {
    use super::*;
    use crate::hooks::build_restate_config_from_url;
    use crate::restate_client::RestateClient;
    use dioxus::prelude::*;

    #[derive(Props, Clone, PartialEq)]
    pub struct StateBrowserPanelProps {
        pub admin_url: String,
    }

    #[component]
    pub fn StateBrowserPanel(props: StateBrowserPanelProps) -> Element {
        let mut collapsed = use_signal(|| true);
        let mut service_name = use_signal(String::new);
        let mut service_key = use_signal(String::new);
        let mut load_state = use_signal(|| BrowserLoadState::Idle);
        let mut is_loading = use_signal(|| false);

        let admin_url = props.admin_url.clone();

        rsx! {
            div { class: "border-t border-slate-200 shrink-0",

                // Section header
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
                            "State Browser"
                        }
                    }
                }

                // Section body
                if !*collapsed.read() {
                    div { class: "px-3 py-2 space-y-2",

                        // Input row
                        div { class: "flex gap-2 items-center",
                            input {
                                class: "text-[10px] border border-slate-200 rounded px-1.5 py-0.5 flex-1 font-mono bg-white",
                                placeholder: "Service name",
                                value: "{service_name}",
                                oninput: move |e| service_name.set(e.value()),
                            }
                            input {
                                class: "text-[10px] border border-slate-200 rounded px-1.5 py-0.5 flex-1 font-mono bg-white",
                                placeholder: "Key (optional)",
                                value: "{service_key}",
                                oninput: move |e| service_key.set(e.value()),
                            }
                        }

                        // Load button
                        {
                            let name_empty = service_name.read().is_empty();
                            let loading_now = *is_loading.read();
                            let btn_class = if name_empty || loading_now {
                                "text-[10px] px-3 py-1 rounded border font-medium bg-slate-50 text-slate-400 border-slate-200 cursor-not-allowed"
                            } else {
                                "text-[10px] px-3 py-1 rounded border font-medium bg-indigo-50 text-indigo-600 border-indigo-200 hover:bg-indigo-100 transition-colors cursor-pointer"
                            };

                            rsx! {
                                button {
                                    class: "{btn_class}",
                                    disabled: name_empty || loading_now,
                                    onclick: move |_| {
                                        if !*is_loading.read() && !service_name.read().is_empty() {
                                            is_loading.set(true);
                                            load_state.set(BrowserLoadState::Loading);
                                            let name = service_name.read().clone();
                                            let key = service_key.read().clone();
                                            let url = admin_url.clone();
                                            spawn(async move {
                                                let config = build_restate_config_from_url(&url);
                                                let client = RestateClient::new(config);
                                                let result = client.get_service_state(&name).await;
                                                match result {
                                                    Ok(entries) => {
                                                        let filtered = filter_by_service_key(entries, &key);
                                                        load_state.set(BrowserLoadState::Loaded(filtered));
                                                    }
                                                    Err(e) => {
                                                        load_state.set(BrowserLoadState::Error(e.to_string()));
                                                    }
                                                }
                                                is_loading.set(false);
                                            });
                                        }
                                    },
                                    if *is_loading.read() { "Loading…" } else { "Load State" }
                                }
                            }
                        }

                        // Results
                        {
                            let state = load_state.read();
                            let state_clone = state.clone();
                            drop(state);

                            match state_clone {
                                BrowserLoadState::Idle => rsx! { div {} },
                                BrowserLoadState::Loading => rsx! {
                                    div { class: "text-[10px] text-slate-400 text-center py-2",
                                        "Loading state…"
                                    }
                                },
                                BrowserLoadState::Loaded(entries) => {
                                    if entries.is_empty() {
                                        rsx! {
                                            div { class: "text-[10px] text-slate-400 text-center py-2",
                                                "No state found."
                                            }
                                        }
                                    } else {
                                        let count = entries.len();
                                        rsx! {
                                            div {
                                                div { class: "text-[10px] text-slate-400 mb-1",
                                                    "{count} state entries"
                                                }
                                                table { class: "w-full border-collapse text-left",
                                                    thead {
                                                        tr { class: "bg-slate-50 border-b border-slate-200",
                                                            th { class: "text-[10px] font-semibold text-slate-400 uppercase tracking-wide px-2 py-1", "Key" }
                                                            th { class: "text-[10px] font-semibold text-slate-400 uppercase tracking-wide px-2 py-1", "Service Key" }
                                                            th { class: "text-[10px] font-semibold text-slate-400 uppercase tracking-wide px-2 py-1", "Value" }
                                                        }
                                                    }
                                                    tbody {
                                                        for entry in &entries {
                                                            {
                                                                let display_val = format_value_display(entry);
                                                                let key_display = entry.key.clone();
                                                                let svc_key = entry.service_key.clone().unwrap_or_else(|| "—".to_string());
                                                                rsx! {
                                                                    tr { class: "border-b border-slate-100",
                                                                        td { class: "px-2 py-1 font-mono text-[10px] text-slate-600",
                                                                            "{key_display}"
                                                                        }
                                                                        td { class: "px-2 py-1 text-[10px] text-slate-500",
                                                                            "{svc_key}"
                                                                        }
                                                                        td { class: "px-2 py-1 text-[10px] text-slate-600 max-w-[200px]",
                                                                            pre { class: "whitespace-pre-wrap break-all font-mono text-[9px]",
                                                                                "{display_val}"
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
                                BrowserLoadState::Error(msg) => rsx! {
                                    div { class: "rounded bg-red-50 border border-red-200 px-2 py-1 text-[10px] text-red-700",
                                        "{msg}"
                                    }
                                },
                            }
                        }
                    }
                }
            }
        }
    }
}

#[cfg(target_arch = "wasm32")]
pub use wasm::StateBrowserPanel;

// ---------------------------------------------------------------------------
// Tests (native + wasm32)
// ---------------------------------------------------------------------------

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used, clippy::panic, clippy::float_cmp)]
mod tests {
    use super::*;

    fn make_entry(
        service_name: &str,
        service_key: Option<&str>,
        key: &str,
        value_utf8: Option<&str>,
        value: Option<Vec<u8>>,
    ) -> StateEntry {
        StateEntry {
            service_name: service_name.to_string(),
            service_key: service_key.map(|s| s.to_string()),
            key: key.to_string(),
            value_utf8: value_utf8.map(|s| s.to_string()),
            value,
        }
    }

    // -----------------------------------------------------------------------
    // U1: Pretty-prints valid JSON object
    // -----------------------------------------------------------------------
    #[test]
    fn format_value_pretty_prints_json_object() {
        let entry = make_entry("Svc", None, "k", Some("{\"key\":\"value\"}"), None);
        let result = format_value_display(&entry);
        assert!(result.contains('\n'), "should be pretty-printed: {result}");
        assert!(result.contains("\"key\""), "should contain key: {result}");
        assert!(result.contains("\"value\""), "should contain value: {result}");
    }

    // -----------------------------------------------------------------------
    // U2: Pretty-prints valid JSON array
    // -----------------------------------------------------------------------
    #[test]
    fn format_value_pretty_prints_json_array() {
        let entry = make_entry("Svc", None, "k", Some("[1,2,3]"), None);
        let result = format_value_display(&entry);
        assert!(result.contains('\n'), "should be pretty-printed: {result}");
        assert!(result.contains('1'), "should contain array elements: {result}");
    }

    // -----------------------------------------------------------------------
    // U3: Returns raw string for non-JSON value_utf8
    // -----------------------------------------------------------------------
    #[test]
    fn format_value_returns_raw_non_json() {
        let entry = make_entry("Svc", None, "k", Some("hello world"), None);
        let result = format_value_display(&entry);
        assert_eq!(result, "hello world");
    }

    // -----------------------------------------------------------------------
    // U4: Shows byte count for binary value
    // -----------------------------------------------------------------------
    #[test]
    fn format_value_shows_byte_count() {
        let entry = make_entry("Svc", None, "k", None, Some(vec![1, 2, 3, 4, 5]));
        let result = format_value_display(&entry);
        assert_eq!(result, "(5 bytes)");
    }

    // -----------------------------------------------------------------------
    // U5: Shows "(empty)" when no value at all
    // -----------------------------------------------------------------------
    #[test]
    fn format_value_shows_empty() {
        let entry = make_entry("Svc", None, "k", None, None);
        let result = format_value_display(&entry);
        assert_eq!(result, "(empty)");
    }

    // -----------------------------------------------------------------------
    // U6: Shows "(0 bytes)" for empty binary
    // -----------------------------------------------------------------------
    #[test]
    fn format_value_shows_zero_bytes() {
        let entry = make_entry("Svc", None, "k", None, Some(vec![]));
        let result = format_value_display(&entry);
        assert_eq!(result, "(0 bytes)");
    }

    // -----------------------------------------------------------------------
    // U7: Returns all entries when key is empty
    // -----------------------------------------------------------------------
    #[test]
    fn filter_returns_all_when_key_empty() {
        let entries = vec![
            make_entry("Svc", Some("a"), "k1", None, None),
            make_entry("Svc", Some("b"), "k2", None, None),
        ];
        let result = filter_by_service_key(entries, "");
        assert_eq!(result.len(), 2);
    }

    // -----------------------------------------------------------------------
    // U8: Filters to matching service_key
    // -----------------------------------------------------------------------
    #[test]
    fn filter_matches_service_key() {
        let entries = vec![
            make_entry("Svc", Some("user-1"), "k1", None, None),
            make_entry("Svc", Some("user-2"), "k2", None, None),
            make_entry("Svc", Some("user-1"), "k3", None, None),
        ];
        let result = filter_by_service_key(entries, "user-1");
        assert_eq!(result.len(), 2);
        assert!(result.iter().all(|e| e.service_key.as_deref() == Some("user-1")));
    }

    // -----------------------------------------------------------------------
    // U9: Returns empty when no entries match key
    // -----------------------------------------------------------------------
    #[test]
    fn filter_returns_empty_no_match() {
        let entries = vec![make_entry("Svc", Some("other"), "k1", None, None)];
        let result = filter_by_service_key(entries, "user-1");
        assert!(result.is_empty());
    }

    // -----------------------------------------------------------------------
    // U10: Props equality works
    // -----------------------------------------------------------------------
    #[test]
    fn load_state_error_preserves_message() {
        let state = BrowserLoadState::Error("test error".to_string());
        if let BrowserLoadState::Error(msg) = state {
            assert_eq!(msg, "test error");
        } else {
            panic!("Expected Error variant");
        }
    }

    // -----------------------------------------------------------------------
    // U11: BrowserLoadState Idle default
    // -----------------------------------------------------------------------
    #[test]
    fn load_state_idle_is_default() {
        let state = BrowserLoadState::Idle;
        assert_eq!(state, BrowserLoadState::Idle);
    }

    // -----------------------------------------------------------------------
    // U12: BrowserLoadState Loaded preserves entries
    // -----------------------------------------------------------------------
    #[test]
    fn load_state_loaded_preserves_entries() {
        let entries = vec![make_entry("Svc", None, "k1", Some("v1"), None)];
        let state = BrowserLoadState::Loaded(entries.clone());
        if let BrowserLoadState::Loaded(result) = state {
            assert_eq!(result.len(), 1);
            assert_eq!(result[0].key, "k1");
        } else {
            panic!("Expected Loaded variant");
        }
    }

    // -----------------------------------------------------------------------
    // U13: filter_by_service_key with None service_key entries
    // -----------------------------------------------------------------------
    #[test]
    fn filter_excludes_none_service_key_when_key_set() {
        let entries = vec![
            make_entry("Svc", None, "k1", None, None),
            make_entry("Svc", Some("user-1"), "k2", None, None),
        ];
        let result = filter_by_service_key(entries, "user-1");
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].key, "k2");
    }

    // -----------------------------------------------------------------------
    // U14: format_value_display handles nested JSON
    // -----------------------------------------------------------------------
    #[test]
    fn format_value_handles_nested_json() {
        let entry = make_entry("Svc", None, "k", Some("{\"outer\":{\"inner\":42}}"), None);
        let result = format_value_display(&entry);
        assert!(result.contains("\"outer\""), "should contain outer key: {result}");
        assert!(result.contains("\"inner\""), "should contain inner key: {result}");
        assert!(result.contains('\n'), "should be pretty-printed: {result}");
    }
}
