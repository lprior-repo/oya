import re

content = open("src/ui/node.rs").read()

old_border = """    let category_border = match category {
        NodeCategory::Trigger => "border-emerald-500/40",
        NodeCategory::Action => "border-indigo-500/40",
        NodeCategory::Logic => "border-amber-500/40",
        NodeCategory::Output => "border-pink-500/40",
        NodeCategory::Restate => "border-blue-500/40",
    };"""

new_border = """    let category_border = match category {
        NodeCategory::Entry => "border-emerald-500/40",
        NodeCategory::Durable => "border-indigo-500/40",
        NodeCategory::State => "border-orange-500/40",
        NodeCategory::Flow => "border-amber-500/40",
        NodeCategory::Timing => "border-pink-500/40",
        NodeCategory::Signal => "border-blue-500/40",
    };"""

old_bg = """    let category_icon_bg = match category {
        NodeCategory::Trigger => "bg-emerald-500/15 text-emerald-400",
        NodeCategory::Action => "bg-indigo-500/15 text-indigo-500",
        NodeCategory::Logic => "bg-amber-500/15 text-amber-400",
        NodeCategory::Output => "bg-pink-500/15 text-pink-400",
        NodeCategory::Restate => "bg-blue-500/15 text-blue-400",
    };"""

new_bg = """    let category_icon_bg = match category {
        NodeCategory::Entry => "bg-emerald-500/15 text-emerald-400",
        NodeCategory::Durable => "bg-indigo-500/15 text-indigo-500",
        NodeCategory::State => "bg-orange-500/15 text-orange-400",
        NodeCategory::Flow => "bg-amber-500/15 text-amber-400",
        NodeCategory::Timing => "bg-pink-500/15 text-pink-400",
        NodeCategory::Signal => "bg-blue-500/15 text-blue-400",
    };"""

old_accent = """    let category_accent_bar = match category {
        NodeCategory::Trigger => "bg-emerald-500/40",
        NodeCategory::Action => "bg-indigo-500/40",
        NodeCategory::Logic => "bg-amber-500/40",
        NodeCategory::Output => "bg-pink-500/40",
        NodeCategory::Restate => "bg-blue-500/40",
    };"""

new_accent = """    let category_accent_bar = match category {
        NodeCategory::Entry => "bg-emerald-500/40",
        NodeCategory::Durable => "bg-indigo-500/40",
        NodeCategory::State => "bg-orange-500/40",
        NodeCategory::Flow => "bg-amber-500/40",
        NodeCategory::Timing => "bg-pink-500/40",
        NodeCategory::Signal => "bg-blue-500/40",
    };"""

content = content.replace(old_border, new_border)
content = content.replace(old_bg, new_bg)
content = content.replace(old_accent, new_accent)

with open("src/ui/node.rs", "w") as f:
    f.write(content)
