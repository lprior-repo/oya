use crate::graph::service_kinds::ServiceKind;
use crate::ui::workflow_nodes::schema::{CronExpression, CronTriggerConfig};
use crate::ui::workflow_nodes::shared::{
    input_classes, FormField, FormHint, NodeCard, PRESET_BTN_CLASSES,
};
use dioxus::prelude::*;

const FOCUS_RING: &str = "blue";

#[component]
pub fn CronTriggerForm(mut config: Signal<CronTriggerConfig>) -> Element {
    let input_cls = input_classes(FOCUS_RING);

    rsx! {
        div {
            class: "space-y-4",

            div {
                class: "bg-blue-50 border border-blue-200 rounded-lg p-3 mb-4",
                p {
                    class: "text-sm text-blue-800",
                    "🕐 ",
                    strong { "Starts here" },
                    " - This workflow runs on a schedule."
                }
            }

            FormField {
                label: "When should this run?",
                div {
                    class: "grid grid-cols-3 gap-2 mb-2",
                    role: "group",
                    aria_label: "Schedule presets",
                    button {
                        class: "{PRESET_BTN_CLASSES}",
                        onclick: move |_| {
                            config.write().schedule = CronExpression::new("0 * * * *");
                        },
                        "Every hour"
                    }
                    button {
                        class: "{PRESET_BTN_CLASSES}",
                        onclick: move |_| {
                            config.write().schedule = CronExpression::new("0 0 * * *");
                        },
                        "Daily"
                    }
                    button {
                        class: "{PRESET_BTN_CLASSES}",
                        onclick: move |_| {
                            config.write().schedule = CronExpression::new("0 0 * * 0");
                        },
                        "Weekly"
                    }
                }
                input {
                    r#type: "text",
                    class: "{input_cls}",
                    placeholder: "0 * * * *",
                    value: "{config.read().schedule.as_str()}",
                    oninput: move |e| {
                        config.write().schedule = CronExpression::new(e.value());
                    }
                }
                FormHint { text: "Cron expression: minute hour day month weekday" }
            }
        }
    }
}

#[component]
pub fn CronTriggerNodeCard() -> Element {
    rsx! {
        NodeCard {
            icon_bg: "bg-purple-100",
            icon: "🕐",
            title: "Cron Trigger",
            subtitle: "Runs automatically on a schedule",
            service_kind: Some(ServiceKind::Handler),
        }
    }
}
