use crate::graph::service_kinds::ServiceKind;
use crate::ui::workflow_nodes::schema::WorkflowCallConfig;
use crate::ui::workflow_nodes::shared::{
    input_classes, json_to_display, parse_json_draft, textarea_classes, FormField, FormHint,
    NodeCard,
};
use dioxus::prelude::*;

const FOCUS_RING: &str = "violet";

#[component]
pub fn WorkflowCallForm(mut config: Signal<WorkflowCallConfig>) -> Element {
    let input_cls = input_classes(FOCUS_RING);
    let textarea_cls = textarea_classes(FOCUS_RING);

    let pretty_input = json_to_display(&serde_json::json!({}));
    let mut json_draft = use_signal(|| pretty_input.clone());
    let mut json_error = use_signal(|| Option::<String>::None);
    let mut last_synced_input = use_signal(|| pretty_input.clone());

    use_effect(move || {
        let latest = json_to_display(&serde_json::Value::Null);
        if latest != last_synced_input.read().clone() {
            last_synced_input.set(latest.clone());
            json_draft.set(latest);
            json_error.set(None);
        }
    });

    let workflow_name_value =
        config.read().workflow_name.as_ref().map(|n| n.as_str().to_string()).unwrap_or_default();

    rsx! {
        div {
            class: "space-y-4",

            div {
                class: "bg-violet-50 border border-violet-200 rounded-lg p-3 mb-4",
                p {
                    class: "text-sm text-violet-800",
                    "🔄 ",
                    strong { "Workflow Call" },
                    " - Call another Restate workflow and wait for completion."
                }
            }

            FormField {
                label: "Workflow Name",
                input {
                    r#type: "text",
                    class: "{input_cls}",
                    placeholder: "e.g., order_processing",
                    value: "{workflow_name_value}",
                    oninput: move |e| {
                        let value = e.value();
                        config.write().workflow_name = if value.trim().is_empty() {
                            None
                        } else {
                            Some(value.into())
                        };
                    }
                }
                FormHint { text: "The name of the workflow to call" }
            }

            FormField {
                label: "Input (JSON)",
                textarea {
                    class: "{textarea_cls}",
                    rows: 3,
                    value: "{json_draft}",
                    oninput: move |e| {
                        let draft = e.value();
                        json_draft.set(draft.clone());

                        match parse_json_draft(&draft) {
                            Ok(_value) => {
                                last_synced_input.set(draft);
                                json_error.set(None);
                            }
                            Err(error) => {
                                json_error.set(Some(error));
                            }
                        }
                    },
                }
                if let Some(error) = json_error() {
                    p {
                        class: "text-xs text-red-600 mt-1",
                        "{error}"
                    }
                } else {
                    FormHint { text: "Input data to pass to the workflow" }
                }
            }
        }
    }
}

#[component]
pub fn WorkflowCallNodeCard() -> Element {
    rsx! {
        NodeCard {
            icon_bg: "bg-violet-100",
            icon: "🔄",
            title: "Workflow Call",
            subtitle: "Call another workflow",
            service_kind: Some(ServiceKind::Workflow),
        }
    }
}
