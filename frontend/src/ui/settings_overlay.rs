#![deny(clippy::unwrap_used)]
#![deny(clippy::expect_used)]
#![deny(clippy::panic)]
#![warn(clippy::pedantic)]
#![forbid(unsafe_code)]

use crate::hooks::use_ui_panels::UiPanels;
use dioxus::prelude::*;

#[component]
pub fn SettingsOverlay(panels: UiPanels) -> Element {
    if !*panels.settings_open().read() {
        return rsx! {};
    }

    rsx! {
        div { class: "absolute right-4 top-14 z-40 w-[280px] rounded-lg border border-slate-700 bg-slate-900/95 p-3 shadow-2xl shadow-slate-950/70 backdrop-blur",
            div { class: "mb-2 flex items-center justify-between",
                h4 { class: "text-[12px] font-semibold text-slate-100", "Workflow Settings" }
                button {
                    class: "flex h-6 w-6 items-center justify-center rounded-md text-slate-500 transition-colors hover:bg-slate-800 hover:text-slate-100",
                    onclick: move |_| panels.close_settings(),
                    crate::ui::icons::XIcon { class: "h-3.5 w-3.5" }
                }
            }
            p { class: "mb-3 text-[11px] leading-relaxed text-slate-400", "Use Save to export the current workflow as JSON. Undo and Redo track recent graph edits." }
            div { class: "flex items-center gap-2",
                button {
                    class: "flex h-8 flex-1 items-center justify-center rounded-md border border-slate-700 text-[12px] text-slate-300 transition-colors hover:bg-slate-800 hover:text-slate-100",
                    onclick: move |_| panels.close_settings(),
                    "Close"
                }
            }
        }
    }
}
