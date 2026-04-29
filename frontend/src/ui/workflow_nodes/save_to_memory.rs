use crate::graph::service_kinds::ServiceKind;
use crate::ui::workflow_nodes::schema::{ObjectKey, SetStateConfig};
use crate::ui::workflow_nodes::shared::{
    input_classes, json_to_display, parse_json_draft, textarea_classes, FormField, FormHint,
    NodeCard,
};
use dioxus::prelude::*;

const FOCUS_RING: &str = "indigo";

#[component]
pub fn SetStateForm(mut config: Signal<SetStateConfig>) -> Element {
    let input_cls = input_classes(FOCUS_RING);
    let textarea_cls = textarea_classes(FOCUS_RING);

    let initial_draft = json_to_display(&config.read().value);
    let mut draft = use_signal(move || initial_draft);
    let mut parse_error = use_signal(|| None::<String>);

    rsx! {
        div {
            class: "space-y-4",
            div {
                class: "bg-indigo-50 border border-indigo-200 rounded-lg p-3 mb-4",
                p {
                    class: "text-sm text-indigo-800",
                    "💾 ",
                    strong { "Set State" },
                    " - Store a value in Restate state."
                }
            }
            FormField {
                label: "State Key",
                input {
                    r#type: "text",
                    class: "{input_cls}",
                    placeholder: "e.g., cart_items",
                    value: "{config.read().key.as_str()}",
                    oninput: move |e| config.write().key = ObjectKey::new(e.value()),
                }
                FormHint { text: "The key to store the value under" }
            }
            FormField {
                label: "Value (JSON)",
                textarea {
                    class: "{textarea_cls}",
                    rows: 4,
                    placeholder: r#"{{"amount": 100, "currency": "USD"}}"#,
                    value: "{draft}",
                    oninput: move |e| {
                        let next_value = e.value();
                        draft.set(next_value.clone());
                        match parse_json_draft(next_value.as_str()) {
                            Ok(value) => {
                                parse_error.set(None);
                                config.write().value = value;
                            }
                            Err(error_text) => parse_error.set(Some(error_text)),
                        }
                    },
                }
                if let Some(error_text) = parse_error.read().as_ref() {
                    p { class: "text-xs text-red-600 mt-1", "{error_text}" }
                }
            }
        }
    }
}

#[component]
pub fn SetStateNodeCard() -> Element {
    rsx! {
        NodeCard {
            icon_bg: "bg-indigo-100",
            icon: "💾",
            title: "Set State",
            subtitle: "Store a value in state",
            service_kind: Some(ServiceKind::Actor),
        }
    }
}

pub use SetStateForm as SaveToMemoryForm;
pub use SetStateNodeCard as SaveToMemoryNodeCard;
