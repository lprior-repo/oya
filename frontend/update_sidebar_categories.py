import re

content = open("src/ui/sidebar.rs").read()

old_ico_bg = """fn category_icon_bg(category: &str) -> &'static str {
    match category {
        "trigger" => "bg-emerald-500/10 text-emerald-300 border-emerald-500/20",
        "action" => "bg-indigo-500/10 text-indigo-300 border-indigo-500/20",
        "logic" => "bg-amber-500/10 text-amber-300 border-amber-500/20",
        "output" => "bg-pink-500/10 text-pink-300 border-pink-500/20",
        "restate" => "bg-blue-500/10 text-blue-300 border-blue-500/20",
        _ => "bg-slate-500/10 text-slate-300 border-slate-500/20",
    }
}"""

new_ico_bg = """fn category_icon_bg(category: &str) -> &'static str {
    match category {
        "entry" => "bg-emerald-500/10 text-emerald-300 border-emerald-500/20",
        "durable" => "bg-indigo-500/10 text-indigo-300 border-indigo-500/20",
        "state" => "bg-orange-500/10 text-orange-300 border-orange-500/20",
        "flow" => "bg-amber-500/10 text-amber-300 border-amber-500/20",
        "timing" => "bg-pink-500/10 text-pink-300 border-pink-500/20",
        "signal" => "bg-blue-500/10 text-blue-300 border-blue-500/20",
        _ => "bg-slate-500/10 text-slate-300 border-slate-500/20",
    }
}"""

old_cat_label = """fn category_label(category: &str) -> &'static str {
    match category {
        "trigger" => "Triggers",
        "action" => "Actions",
        "logic" => "Logic",
        "output" => "Outputs",
        "restate" => "Restate SDK",
        _ => "Other",
    }
}"""

new_cat_label = """fn category_label(category: &str) -> &'static str {
    match category {
        "entry" => "Entry Points",
        "durable" => "Durable Steps",
        "state" => "State",
        "flow" => "Control Flow",
        "timing" => "Timing & Events",
        "signal" => "Signals & Promises",
        _ => "Other",
    }
}"""

old_loop_array = """for category in ["restate", "trigger", "action", "logic", "output"] {"""
new_loop_array = """for category in ["entry", "durable", "state", "flow", "timing", "signal"] {"""

content = content.replace(old_ico_bg, new_ico_bg)
content = content.replace(old_cat_label, new_cat_label)
content = content.replace(old_loop_array, new_loop_array)

with open("src/ui/sidebar.rs", "w") as f:
    f.write(content)
