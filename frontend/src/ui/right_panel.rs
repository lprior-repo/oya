#![deny(clippy::unwrap_used)]
#![deny(clippy::expect_used)]
#![deny(clippy::panic)]
#![warn(clippy::pedantic)]
#![forbid(unsafe_code)]

use crate::graph::{NodeId, ValidationResult};
use crate::hooks::use_restate_sync::RestateSyncHandle;
use crate::hooks::use_workflow_state::WorkflowState;
use crate::ui::restate::{
    DeploymentBrowserPanel, LifecycleStatusPanel, OpenCodeTracePanel, PromiseBrowserPanel,
    RestateInvocationsPanel,
};
use crate::ui::{ExecutionHistoryPanel, ExecutionPlanPanel, ValidationPanel};
use dioxus::prelude::*;

#[component]
pub fn RightPanel(
    workflow: WorkflowState,
    validation_result: Memo<ValidationResult>,
    validation_collapsed: Signal<bool>,
    frozen_run_id: Signal<Option<uuid::Uuid>>,
    on_select_node: EventHandler<NodeId>,
    restate: RestateSyncHandle,
) -> Element {
    let plan_collapsed = use_signal(|| false);
    let history_collapsed = use_signal(|| true);
    let history_signal = use_memo(move || workflow.workflow().read().history.clone());

    rsx! {
        div { class: "flex h-full w-[360px] max-w-[360px] min-w-0 shrink-0 flex-col overflow-x-hidden overflow-y-auto border-l border-slate-200",
            ValidationPanel {
                validation_result: ReadSignal::from(validation_result),
                collapsed: validation_collapsed,
                on_select_node: move |node_id| {
                    on_select_node.call(node_id);
                },
            }
            ExecutionPlanPanel {
                on_select_node: move |node_id| {
                    on_select_node.call(node_id);
                },
                collapsed: plan_collapsed,
            }
            ExecutionHistoryPanel {
                history: history_signal,
                on_select_node: move |node_id| {
                    on_select_node.call(node_id);
                },
                collapsed: history_collapsed,
                active_run_id: ReadSignal::from(frozen_run_id),
                on_run_select: move |id| {
                    if let Ok(mut v) = frozen_run_id.try_write() {
                        *v = Some(id);
                    }
                },
                on_exit_frozen: move |()| {
                    if let Ok(mut v) = frozen_run_id.try_write() {
                        *v = None;
                    }
                },
            }
            LifecycleStatusPanel { handle: restate }
            OpenCodeTracePanel { handle: restate }
            RestateInvocationsPanel { handle: restate }
            PromiseBrowserPanel { handle: restate }
            DeploymentBrowserPanel { handle: restate }
        }
    }
}
