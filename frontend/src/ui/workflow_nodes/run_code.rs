use crate::graph::service_kinds::ServiceKind;
use crate::ui::workflow_nodes::schema::RunConfig;
use crate::ui::workflow_nodes::shared::{input_classes, FormField, FormHint, NodeCard};
use dioxus::prelude::*;

const FOCUS_RING: &str = "red";

#[component]
pub fn RunForm(mut config: Signal<RunConfig>) -> Element {
    let input_cls = input_classes(FOCUS_RING);

    rsx! {
        div {
            class: "space-y-4",

            div {
                class: "bg-red-50 border border-red-200 rounded-lg p-3 mb-4",
                p {
                    class: "text-sm text-red-800",
                    "⚡ ",
                    strong { "Run" },
                    " - Execute custom code durably."
                }
            }

            FormField {
                label: "Code",
                textarea {
                    class: "{input_cls} font-mono text-sm",
                    rows: 8,
                    placeholder: "// Available: input\nreturn {{ ok: true }};",
                    value: "{config.read().code.as_str()}",
                    oninput: move |e| {
                        config.write().code = crate::ui::workflow_nodes::schema::CodeContent::new(e.value());
                    }
                }
                FormHint { text: "JavaScript or TypeScript code to execute" }
            }

            div {
                class: "bg-yellow-50 p-3 rounded-lg",
                p {
                    class: "text-sm text-yellow-800",
                    "⚠️ Code runs once. On retry, the saved result is used to ensure consistency."
                }
            }
        }
    }
}

#[component]
pub fn RunNodeCard() -> Element {
    rsx! {
        NodeCard {
            icon_bg: "bg-red-100",
            icon: "⚡",
            title: "Run",
            subtitle: "Execute custom logic",
            service_kind: Some(ServiceKind::Handler),
        }
    }
}

pub use RunForm as RunCodeForm;
pub use RunNodeCard as RunCodeNodeCard;
