use crate::graph::service_kinds::ServiceKind;
use crate::ui::workflow_nodes::schema::SleepConfig;
use crate::ui::workflow_nodes::shared::{input_classes, FormField, NodeCard, PRESET_BTN_CLASSES};
use dioxus::prelude::*;

const FOCUS_RING: &str = "gray";

#[component]
pub fn SleepForm(mut config: Signal<SleepConfig>) -> Element {
    let mut duration_error = use_signal(|| Option::<String>::None);
    let input_cls = input_classes(FOCUS_RING);

    rsx! {
        div {
            class: "space-y-4",

            div {
                class: "bg-gray-50 border border-gray-200 rounded-lg p-3 mb-4",
                p {
                    class: "text-sm text-gray-700",
                    "⏱️ ",
                    strong { "Sleep" },
                    " - Pause the workflow for a while."
                }
            }

            FormField {
                label: "How long to sleep?",
                div {
                    class: "grid grid-cols-3 gap-2 mb-3",
                    role: "group",
                    aria_label: "Duration presets",
                    button {
                        class: "{PRESET_BTN_CLASSES}",
                        onclick: move |_| {
                            config.write().duration_ms = 1000;
                        },
                        "1 second"
                    }
                    button {
                        class: "{PRESET_BTN_CLASSES}",
                        onclick: move |_| {
                            config.write().duration_ms = 5000;
                        },
                        "5 seconds"
                    }
                    button {
                        class: "{PRESET_BTN_CLASSES}",
                        onclick: move |_| {
                            config.write().duration_ms = 30000;
                        },
                        "30 seconds"
                    }
                }
                div {
                    class: "grid grid-cols-3 gap-2 mb-2",
                    role: "group",
                    aria_label: "Duration presets",
                    button {
                        class: "{PRESET_BTN_CLASSES}",
                        onclick: move |_| {
                            config.write().duration_ms = 60_000;
                        },
                        "1 minute"
                    }
                    button {
                        class: "{PRESET_BTN_CLASSES}",
                        onclick: move |_| {
                            config.write().duration_ms = 300_000;
                        },
                        "5 minutes"
                    }
                    button {
                        class: "{PRESET_BTN_CLASSES}",
                        onclick: move |_| {
                            config.write().duration_ms = 3_600_000;
                        },
                        "1 hour"
                    }
                }
                div {
                    class: "flex items-center gap-2",
                    input {
                        r#type: "number",
                        min: "1",
                        class: "{input_cls}",
                        placeholder: "milliseconds",
                        value: "{config.read().duration_ms}",
                        oninput: move |e| {
                            let value = e.value();
                            if value.trim().is_empty() {
                                duration_error.set(None);
                            } else if let Ok(v) = value.parse::<u64>() {
                                if v > 0 {
                                    config.write().duration_ms = v;
                                    duration_error.set(None);
                                } else {
                                    duration_error.set(Some("Duration must be greater than 0 ms".to_string()));
                                }
                            }
                        }
                    }
                    span {
                        class: "text-sm text-gray-500",
                        "ms"
                    }
                }
            }

                if let Some(error) = duration_error() {
                    p {
                        class: "text-xs text-red-600 mt-1",
                        "{error}"
                    }
                } else {
                    p {
                        class: "text-xs text-gray-500 mt-1",
                        "Duration must be greater than 0 ms"
                    }
                }

                div {
                    class: "bg-yellow-50 p-3 rounded-lg",
                    p {
                        class: "text-sm text-yellow-800",
                        "⚠️ The workflow will be paused. For Virtual Objects, this blocks other calls to the same object."
                    }
                }
        }
    }
}

#[component]
pub fn SleepNodeCard() -> Element {
    rsx! {
        NodeCard {
            icon_bg: "bg-gray-200",
            icon: "⏱️",
            title: "Sleep",
            subtitle: "Wait for a period of time",
            service_kind: Some(ServiceKind::Handler),
        }
    }
}

pub use SleepForm as DelayForm;
pub use SleepNodeCard as DelayNodeCard;
