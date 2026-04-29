use crate::graph::service_kinds::ServiceKind;
use dioxus::prelude::*;

/// Shared CSS class constants used across all workflow node forms and cards.
pub const CARD_CLASSES: &str =
    "flex items-center gap-3 p-3 bg-white border border-gray-200 rounded-lg shadow-sm hover:shadow-md transition-shadow";
pub const LABEL_CLASSES: &str = "block text-sm font-medium text-gray-700 mb-1";
pub const PRESET_BTN_CLASSES: &str = "px-3 py-2 text-sm border rounded-md hover:bg-gray-50";

/// Returns the standard input classes with a per-node focus ring color.
///
/// Every workflow node form uses the same base input styling but with a
/// different Tailwind `focus:ring-{color}-500`. This function avoids
/// duplicating the long base string in each module.
pub fn input_classes(focus_ring_color: &str) -> String {
    format!(
        "w-full px-3 py-2 border border-gray-300 rounded-md focus:outline-none focus:ring-2 focus:ring-{focus_ring_color}-500"
    )
}

/// Returns the standard textarea classes with a per-node focus ring color
/// and monospace font styling for JSON/code editing.
pub fn textarea_classes(focus_ring_color: &str) -> String {
    format!(
        "w-full px-3 py-2 border border-gray-300 rounded-md focus:outline-none focus:ring-2 focus:ring-{focus_ring_color}-500 font-mono text-sm"
    )
}

// ---------------------------------------------------------------------------
// JSON helpers (duplicated across load_from_memory, save_to_memory,
// send_message, service_call, delayed_message)
// ---------------------------------------------------------------------------

/// Pretty-print a `serde_json::Value` for display. Returns an empty string
/// on serialization failure.
pub fn json_to_display(value: &serde_json::Value) -> String {
    serde_json::to_string_pretty(value).unwrap_or_else(|_| String::new())
}

/// Pretty-print an optional `serde_json::Value`. Returns an empty string
/// when the value is `None` or serialization fails.
pub fn optional_json_to_display(value: Option<&serde_json::Value>) -> String {
    value.map_or_else(String::new, json_to_display)
}

/// Parse a string as JSON. Used for live textarea draft validation.
pub fn parse_json_draft(input: &str) -> Result<serde_json::Value, String> {
    serde_json::from_str(input).map_err(|error| format!("Invalid JSON: {error}"))
}

/// Parse a string as optional JSON: empty/whitespace input maps to `Ok(None)`.
pub fn parse_optional_json_draft(input: &str) -> Result<Option<serde_json::Value>, String> {
    let trimmed = input.trim();
    if trimmed.is_empty() {
        Ok(None)
    } else {
        serde_json::from_str(trimmed).map(Some).map_err(|error| format!("Invalid JSON: {error}"))
    }
}

// ---------------------------------------------------------------------------
// ServiceKind badge helpers
// ---------------------------------------------------------------------------

/// Returns the Tailwind CSS classes for a ServiceKind badge.
///
/// - Handler: gray tones ("Stateless")
/// - Actor: blue tones ("Stateful")
/// - Workflow: purple tones ("Durable")
#[must_use]
pub const fn service_kind_badge_classes(kind: ServiceKind) -> &'static str {
    match kind {
        ServiceKind::Handler => "bg-gray-100 text-gray-600",
        ServiceKind::Actor => "bg-blue-100 text-blue-700",
        ServiceKind::Workflow => "bg-purple-100 text-purple-700",
    }
}

/// Returns the human-readable label for a ServiceKind badge.
///
/// - Handler → "Stateless"
/// - Actor → "Stateful"
/// - Workflow → "Durable"
#[must_use]
pub const fn service_kind_label(kind: ServiceKind) -> &'static str {
    match kind {
        ServiceKind::Handler => "Stateless",
        ServiceKind::Actor => "Stateful",
        ServiceKind::Workflow => "Durable",
    }
}

// ---------------------------------------------------------------------------
// NodeCard component
// ---------------------------------------------------------------------------

/// Props for the shared `NodeCard` component.
///
/// Each workflow node type renders the same card layout: a colored circle
/// containing an emoji icon, a bold title, a muted subtitle, and optionally
/// a ServiceKind badge with capability indicator dots. The only variation is
/// the color, icon, title, subtitle text, and optional service kind.
#[derive(Props, Clone, PartialEq)]
pub struct NodeCardProps {
    pub icon_bg: &'static str,
    pub icon: &'static str,
    pub title: &'static str,
    pub subtitle: &'static str,
    #[props(default)]
    pub service_kind: Option<ServiceKind>,
}

/// A reusable card component for workflow node type selectors.
///
/// All 11 node card components (`HttpTriggerNodeCard`, `ServiceCallNodeCard`,
/// etc.) previously duplicated the same `rsx!` structure with only four
/// varying fields. This component consolidates that pattern.
///
/// When `service_kind` is provided, a small badge and capability dots are
/// rendered alongside the subtitle.
#[component]
pub fn NodeCard(props: NodeCardProps) -> Element {
    let badge_markup = props.service_kind.map(move |kind| {
        let badge_cls = service_kind_badge_classes(kind);
        let label = service_kind_label(kind);
        let has_state = kind.supports_state();
        let has_promises = kind.supports_promises();
        rsx! {
            div {
                class: "flex items-center gap-1.5 mt-0.5",

                span {
                    class: "text-[10px] font-medium px-1.5 py-0.5 rounded {badge_cls}",
                    "{label}"
                }

                // Capability indicator dots
                span {
                    class: "flex items-center gap-1",

                    if has_state {
                        span {
                            class: "inline-block w-1.5 h-1.5 rounded-full bg-amber-400",
                            title: "State access",
                        }
                    } else {
                        span {
                            class: "inline-block w-1.5 h-1.5 rounded-full border border-gray-300",
                            title: "No state",
                        }
                    }

                    if has_promises {
                        span {
                            class: "inline-block w-1.5 h-1.5 rounded-full bg-purple-400",
                            title: "Promises",
                        }
                    } else {
                        span {
                            class: "inline-block w-1.5 h-1.5 rounded-full border border-gray-300",
                            title: "No promises",
                        }
                    }
                }
            }
        }
    });

    rsx! {
        div {
            class: "{CARD_CLASSES}",

            div {
                class: "w-10 h-10 {props.icon_bg} rounded-full flex items-center justify-center",
                span {
                    class: "text-xl",
                    "{props.icon}"
                }
            },

            div {
                class: "flex-1",
                div {
                    class: "flex items-center gap-2",
                    h3 {
                        class: "font-medium text-gray-900",
                        "{props.title}"
                    }
                }
                p {
                    class: "text-sm text-gray-500",
                    "{props.subtitle}"
                }
                {badge_markup}
            }
        }
    }
}

// ---------------------------------------------------------------------------
// Form field helper components
// ---------------------------------------------------------------------------

/// Props for a labelled form field wrapper.
#[derive(Props, Clone, PartialEq)]
pub struct FormFieldProps {
    pub label: &'static str,
    pub children: Element,
}

/// A labelled form field with the standard label styling.
///
/// Wraps any input/select/textarea with a consistent `<label>` element
/// using `LABEL_CLASSES`.
#[component]
pub fn FormField(props: FormFieldProps) -> Element {
    rsx! {
        div {
            class: "form-field",
            label { class: "{LABEL_CLASSES}", "{props.label}" }
            {props.children}
        }
    }
}

/// Props for the form description hint pattern.
#[derive(Props, Clone, PartialEq)]
pub struct FormHintProps {
    pub text: &'static str,
}

/// A muted hint line below a form field.
#[component]
pub fn FormHint(props: FormHintProps) -> Element {
    rsx! {
        p { class: "text-xs text-gray-500 mt-1", "{props.text}" }
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used, clippy::panic, clippy::float_cmp)]
mod tests {
    use super::*;

    #[test]
    fn input_classes_includes_focus_ring_color() {
        let classes = input_classes("blue");
        assert!(
            classes.contains("focus:ring-blue-500"),
            "expected blue focus ring, got: {classes}"
        );
    }

    #[test]
    fn textarea_classes_includes_focus_ring_color_and_mono() {
        let classes = textarea_classes("green");
        assert!(classes.contains("focus:ring-green-500"));
        assert!(classes.contains("font-mono"));
    }

    #[test]
    fn json_to_display_pretty_prints_object() {
        let value = serde_json::json!({"a": 1});
        let display = json_to_display(&value);
        assert!(display.contains('\n'));
    }

    #[test]
    fn optional_json_to_display_none_is_empty() {
        assert_eq!(optional_json_to_display(None), "");
    }

    #[test]
    fn optional_json_to_display_some_is_pretty() {
        let value = serde_json::json!(42);
        let display = optional_json_to_display(Some(&value));
        assert_eq!(display, "42");
    }

    #[test]
    fn parse_json_draft_valid_returns_ok() {
        assert!(parse_json_draft(r#"{"ok":true}"#).is_ok());
    }

    #[test]
    fn parse_json_draft_invalid_returns_err() {
        assert!(parse_json_draft("{not-json}").is_err());
    }

    #[test]
    fn parse_optional_json_draft_empty_is_none() {
        assert_eq!(parse_optional_json_draft("   "), Ok(None));
    }

    #[test]
    fn parse_optional_json_draft_valid_is_some() {
        assert!(matches!(parse_optional_json_draft(r#"{"fallback":1}"#), Ok(Some(_))));
    }

    #[test]
    fn parse_optional_json_draft_invalid_is_err() {
        assert!(parse_optional_json_draft("{not-json}").is_err());
    }

    // ========================================================================
    // ServiceKind badge tests (RED PHASE — functions do not exist yet)
    // ========================================================================

    #[test]
    fn badge_classes_returns_gray_classes_when_handler() {
        let classes = service_kind_badge_classes(crate::graph::service_kinds::ServiceKind::Handler);
        assert!(
            classes.contains("bg-gray"),
            "expected gray background for Handler, got: {classes}"
        );
        assert!(classes.contains("text-gray"), "expected gray text for Handler, got: {classes}");
    }

    #[test]
    fn badge_classes_returns_blue_classes_when_actor() {
        let classes = service_kind_badge_classes(crate::graph::service_kinds::ServiceKind::Actor);
        assert!(classes.contains("bg-blue"), "expected blue background for Actor, got: {classes}");
        assert!(classes.contains("text-blue"), "expected blue text for Actor, got: {classes}");
    }

    #[test]
    fn badge_classes_returns_purple_classes_when_workflow() {
        let classes =
            service_kind_badge_classes(crate::graph::service_kinds::ServiceKind::Workflow);
        assert!(
            classes.contains("bg-purple"),
            "expected purple background for Workflow, got: {classes}"
        );
        assert!(
            classes.contains("text-purple"),
            "expected purple text for Workflow, got: {classes}"
        );
    }

    #[test]
    fn label_returns_stateless_when_handler() {
        let label = service_kind_label(crate::graph::service_kinds::ServiceKind::Handler);
        assert_eq!(label, "Stateless");
    }

    #[test]
    fn label_returns_stateful_when_actor() {
        let label = service_kind_label(crate::graph::service_kinds::ServiceKind::Actor);
        assert_eq!(label, "Stateful");
    }

    #[test]
    fn label_returns_durable_when_workflow() {
        let label = service_kind_label(crate::graph::service_kinds::ServiceKind::Workflow);
        assert_eq!(label, "Durable");
    }
}
