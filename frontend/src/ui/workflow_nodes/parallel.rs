use crate::graph::service_kinds::ServiceKind;
use crate::ui::workflow_nodes::schema::ParallelConfig;
use crate::ui::workflow_nodes::shared::NodeCard;
use dioxus::prelude::*;

#[component]
pub fn ParallelForm(config: Signal<ParallelConfig>) -> Element {
    rsx! {
        div { class: "space-y-4",
            p { class: "text-sm text-slate-600", "Parallel branches configuration" }
        }
    }
}

#[component]
pub fn ParallelNodeCard() -> Element {
    rsx! {
        NodeCard { icon_bg: "bg-indigo-100", icon: "🔀", title: "Parallel", subtitle: "Execute branches in parallel", service_kind: Some(ServiceKind::Handler) }
    }
}
