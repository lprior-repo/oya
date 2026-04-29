import re

content = open("src/ui/node.rs").read()

old_status = """                // Status indicator
                div { class: "ml-auto shrink-0",
                    if node.executing {
                        LoaderIcon { class: "h-3.5 w-3.5 animate-spin text-indigo-500" }
                    } else if node.error.is_some() {
                        AlertCircleIcon { class: "h-3.5 w-3.5 text-red-500" }
                    } else if node.last_output.is_some() {
                        CheckCircleIcon { class: "h-3.5 w-3.5 text-emerald-500" }
                    } else if !node.config.is_null() {
                        div { class: "h-1.5 w-1.5 rounded-full bg-emerald-500" }
                    }
                }
            }"""

new_status = """                // Status indicator
                div { class: "ml-auto shrink-0",
                    if let Some(status) = node.config.get("status").and_then(|s| s.as_str()) {
                        if status != "pending" {
                            let (bg_color, text_color, border_color, icon_name, is_spin) = match status {
                                "running" => ("bg-indigo-500/15", "text-indigo-400", "border-indigo-500/30", "loader", true),
                                "suspended" => ("bg-pink-500/15", "text-pink-400", "border-pink-500/30", "pause", false),
                                "completed" => ("bg-emerald-500/15", "text-emerald-400", "border-emerald-500/30", "check-circle", false),
                                "failed" => ("bg-red-500/15", "text-red-400", "border-red-500/30", "alert-circle", false),
                                "retrying" => ("bg-amber-500/15", "text-amber-400", "border-amber-500/30", "refresh", true),
                                _ => ("bg-slate-500/15", "text-slate-400", "border-slate-500/30", "help-circle", false),
                            };
                            let label = match status {
                                "running" => "Running",
                                "suspended" => "Suspended",
                                "completed" => "Done",
                                "failed" => "Failed",
                                "retrying" => "Retrying",
                                _ => status,
                            };
                            rsx! {
                                span {
                                    class: "inline-flex items-center gap-1 rounded-full border px-1.5 py-px text-[9px] font-medium leading-none {bg_color} {text_color} {border_color}",
                                    {icon_by_name(icon_name, format!("h-2.5 w-2.5 {}", if is_spin { "animate-spin" } else { "" }))}
                                    "{label}"
                                }
                            }
                        } else if node.config.get("configured").and_then(|c| c.as_bool()).unwrap_or(false) {
                            rsx! { div { class: "h-1.5 w-1.5 rounded-full bg-emerald-500" } }
                        } else {
                            rsx! { div {} }
                        }
                    } else if node.config.get("configured").and_then(|c| c.as_bool()).unwrap_or(false) {
                        rsx! { div { class: "h-1.5 w-1.5 rounded-full bg-emerald-500" } }
                    } else {
                        rsx! { div {} }
                    }
                }
            }

            // Journal row
            if node.config.get("journalIndex").is_some() || node.config.get("retryCount").and_then(|c| c.as_u64()).unwrap_or(0) > 0 {
                div { class: "flex items-center gap-2 px-3 pb-2 text-[9px] font-mono text-slate-400/70",
                    if let Some(idx) = node.config.get("journalIndex").and_then(|i| i.as_u64()) {
                        span { class: "rounded bg-slate-800/60 px-1 py-px", "journal #{idx}" }
                    }
                    if let Some(retries) = node.config.get("retryCount").and_then(|i| i.as_u64()) {
                        if retries > 0 {
                            span { class: "rounded bg-red-500/10 text-red-400/70 px-1 py-px", "{retries} retries" }
                        }
                    }
                    if let Some(key) = node.config.get("idempotencyKey").and_then(|i| i.as_str()) {
                        span { class: "rounded bg-slate-800/60 px-1 py-px truncate max-w-[80px]", title: "{key}", "key: {key}" }
                    }
                }
            }"""

content = content.replace(old_status, new_status)

with open("src/ui/node.rs", "w") as f:
    f.write(content)
