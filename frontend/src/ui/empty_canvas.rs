//! Empty Canvas Welcome State
//!
//! Shown when the canvas has zero nodes. Provides quick-start actions
//! for new users. Disappears as soon as any node is added.

#![deny(clippy::unwrap_used)]
#![deny(clippy::expect_used)]
#![deny(clippy::panic)]

use dioxus::prelude::*;

#[component]
pub fn EmptyCanvas(on_add_node: EventHandler<()>, on_import: EventHandler<()>) -> Element {
    rsx! {
        div {
            class: "pointer-events-none absolute inset-0 z-20 flex items-center justify-center",

            div {
                class: "pointer-events-auto flex max-w-sm flex-col items-center gap-6 rounded-2xl border border-slate-200/80 bg-white/80 px-8 py-10 text-center shadow-lg shadow-slate-200/50 backdrop-blur-sm",

                div { class: "flex h-14 w-14 items-center justify-center rounded-2xl border border-cyan-200 bg-cyan-50",
                    svg {
                        class: "h-7 w-7 text-cyan-600",
                        fill: "none",
                        view_box: "0 0 24 24",
                        stroke: "currentColor",
                        stroke_width: "1.5",
                        path { d: "M9.813 15.904L9 18.75l-.813-2.846a4.5 4.5 0 00-3.09-3.09L2.25 12l2.846-.813a4.5 4.5 0 003.09-3.09L9 5.25l.813 2.846a4.5 4.5 0 003.09 3.09L15.75 12l-2.846.813a4.5 4.5 0 00-3.09 3.09zM18.259 8.715L18 9.75l-.259-1.035a3.375 3.375 0 00-2.455-2.456L14.25 6l1.036-.259a3.375 3.375 0 002.455-2.456L18 2.25l.259 1.035a3.375 3.375 0 002.455 2.456L21.75 6l-1.036.259a3.375 3.375 0 00-2.455 2.456zM16.894 20.567L16.5 21.75l-.394-1.183a2.25 2.25 0 00-1.423-1.423L13.5 18.75l1.183-.394a2.25 2.25 0 001.423-1.423l.394-1.183.394 1.183a2.25 2.25 0 001.423 1.423l1.183.394-1.183.394a2.25 2.25 0 00-1.423 1.423z" }
                    }
                }

                div {
                    h2 { class: "text-[18px] font-semibold text-slate-900", "Welcome to Oya" }
                    p { class: "mt-2 text-[13px] leading-relaxed text-slate-500",
                        "Design and execute durable workflows visually. "
                        "Add nodes, connect them, and run — all in your browser."
                    }
                }

                div { class: "flex flex-col gap-2.5 w-full",
                    button {
                        class: "flex h-10 w-full items-center justify-center gap-2 rounded-lg bg-gradient-to-r from-cyan-600 to-teal-600 text-[13px] font-semibold text-white transition-all hover:from-cyan-500 hover:to-teal-500 hover:shadow-lg hover:shadow-cyan-500/25",
                        r#type: "button",
                        onclick: move |_| on_add_node.call(()),
                        "Add your first node"
                    }
                    button {
                        class: "flex h-10 w-full items-center justify-center gap-2 rounded-lg border border-slate-200 bg-white text-[13px] font-medium text-slate-700 transition-all hover:bg-slate-50 hover:text-slate-900",
                        r#type: "button",
                        onclick: move |_| on_import.call(()),
                        svg {
                            class: "h-4 w-4",
                            fill: "none",
                            view_box: "0 0 24 24",
                            stroke: "currentColor",
                            stroke_width: "2",
                            path { d: "M21 15v4a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2v-4" }
                            polyline { points: "17 8 12 3 7 8" }
                            line { x1: "12", y1: "3", x2: "12", y2: "15" }
                        }
                        "Load a workflow"
                    }
                }

                p { class: "text-[11px] text-slate-400",
                    "Press "
                    kbd { class: "rounded border border-slate-200 bg-slate-50 px-1 py-0.5 font-mono text-[10px]", "K" }
                    " to open the node palette · "
                    kbd { class: "rounded border border-slate-200 bg-slate-50 px-1 py-0.5 font-mono text-[10px]", "?" }
                    " for all shortcuts"
                }
            }
        }
    }
}
