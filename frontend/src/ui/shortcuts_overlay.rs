//! Keyboard Shortcuts Help Overlay
//!
//! Toggled by pressing '?' key. Displays all available shortcuts in a
//! two-column layout. Dismisses on Escape or clicking outside.

#![deny(clippy::unwrap_used)]
#![deny(clippy::expect_used)]
#![deny(clippy::panic)]

use dioxus::prelude::*;

/// A single keyboard shortcut entry.
struct Shortcut {
    action: &'static str,
    keys: &'static str,
}

/// Category grouping for shortcuts.
struct ShortcutCategory {
    name: &'static str,
    shortcuts: &'static [Shortcut],
}

/// Returns all available keyboard shortcuts grouped by category.
fn shortcut_categories() -> &'static [ShortcutCategory] {
    static CATEGORIES: &[ShortcutCategory] = &[
        ShortcutCategory {
            name: "Editing",
            shortcuts: &[
                Shortcut { action: "Add Node", keys: "K" },
                Shortcut { action: "Delete Selected", keys: "⌫ / Del" },
                Shortcut { action: "Duplicate Node", keys: "Ctrl+D" },
            ],
        },
        ShortcutCategory {
            name: "History",
            shortcuts: &[
                Shortcut { action: "Undo", keys: "Ctrl+Z" },
                Shortcut { action: "Redo", keys: "Ctrl+Shift+Z" },
            ],
        },
        ShortcutCategory {
            name: "File",
            shortcuts: &[Shortcut { action: "Save Workflow", keys: "Ctrl+S" }],
        },
        ShortcutCategory {
            name: "Navigation",
            shortcuts: &[
                Shortcut { action: "Pan Canvas", keys: "Space + Drag" },
                Shortcut { action: "Zoom In/Out", keys: "Scroll / +/-" },
                Shortcut { action: "Fit View", keys: "0" },
                Shortcut { action: "Auto Layout", keys: "Ctrl+L" },
            ],
        },
        ShortcutCategory {
            name: "Selection",
            shortcuts: &[
                Shortcut { action: "Select Next", keys: "Tab" },
                Shortcut { action: "Select Previous", keys: "Shift+Tab" },
            ],
        },
        ShortcutCategory {
            name: "General",
            shortcuts: &[
                Shortcut { action: "Deselect / Close", keys: "Escape" },
                Shortcut { action: "Show Shortcuts", keys: "?" },
            ],
        },
    ];
    CATEGORIES
}

#[component]
pub fn ShortcutsOverlay(on_close: EventHandler<()>) -> Element {
    rsx! {
        div {
            class: "fixed inset-0 z-50 flex items-center justify-center",
            onclick: move |_| on_close.call(()),

            div {
                class: "rounded-xl border border-slate-200 bg-white/95 shadow-2xl shadow-slate-900/20 backdrop-blur-lg",
                onclick: move |evt| evt.stop_propagation(),

                div { class: "flex items-center justify-between border-b border-slate-200 px-6 py-4",
                    h3 { class: "text-[15px] font-semibold text-slate-900", "Keyboard Shortcuts" }
                    button {
                        class: "flex h-7 w-7 items-center justify-center rounded-md text-slate-500 transition-colors hover:bg-slate-100 hover:text-slate-900",
                        r#type: "button",
                        aria_label: "Close shortcuts",
                        onclick: move |_| on_close.call(()),
                        crate::ui::icons::XIcon { class: "h-4 w-4" }
                    }
                }

                div { class: "grid grid-cols-2 gap-x-8 px-6 py-4",
                    for category in shortcut_categories() {
                        div {
                            key: "{category.name}",
                            class: "col-span-2",

                            span { class: "text-[10px] font-semibold uppercase tracking-wider text-slate-400", "{category.name}" }

                            div { class: "mt-1 mb-2",
                                for shortcut in category.shortcuts {
                                    div {
                                        key: "{shortcut.action}",
                                        class: "flex items-center justify-between py-0.5",
                                        span { class: "text-[12px] text-slate-600", "{shortcut.action}" }
                                        kbd { class: "rounded border border-slate-200 bg-slate-50 px-1.5 py-0.5 font-mono text-[10px] text-slate-500", "{shortcut.keys}" }
                                    }
                                }
                            }
                        }
                    }
                }

                div { class: "border-t border-slate-100 px-6 py-3",
                    p { class: "text-[10px] text-slate-400", "Press ? to toggle · Escape to close" }
                }
            }
        }
    }
}
