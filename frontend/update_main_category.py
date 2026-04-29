import re

content = open("src/main.rs").read()

old_badges = """                            let badge_classes = match selected_node.category {
                                oya_frontend::graph::NodeCategory::Trigger => {
                                    "bg-emerald-500/15 text-emerald-300 border-emerald-500/25"
                                }
                                oya_frontend::graph::NodeCategory::Action => {
                                    "bg-indigo-500/15 text-indigo-300 border-indigo-500/25"
                                }
                                oya_frontend::graph::NodeCategory::Logic => {
                                    "bg-amber-500/15 text-amber-300 border-amber-500/25"
                                }
                                oya_frontend::graph::NodeCategory::Output => {
                                    "bg-pink-500/15 text-pink-300 border-pink-500/25"
                                }
                                oya_frontend::graph::NodeCategory::Restate => {
                                    "bg-blue-500/15 text-blue-300 border-blue-500/25"
                                }
                            };"""

new_badges = """                            let badge_classes = match selected_node.category {
                                oya_frontend::graph::NodeCategory::Entry => {
                                    "bg-emerald-500/15 text-emerald-300 border-emerald-500/25"
                                }
                                oya_frontend::graph::NodeCategory::Durable => {
                                    "bg-indigo-500/15 text-indigo-300 border-indigo-500/25"
                                }
                                oya_frontend::graph::NodeCategory::State => {
                                    "bg-orange-500/15 text-orange-300 border-orange-500/25"
                                }
                                oya_frontend::graph::NodeCategory::Flow => {
                                    "bg-amber-500/15 text-amber-300 border-amber-500/25"
                                }
                                oya_frontend::graph::NodeCategory::Timing => {
                                    "bg-pink-500/15 text-pink-300 border-pink-500/25"
                                }
                                oya_frontend::graph::NodeCategory::Signal => {
                                    "bg-blue-500/15 text-blue-300 border-blue-500/25"
                                }
                            };"""

content = content.replace(old_badges, new_badges)

with open("src/main.rs", "w") as f:
    f.write(content)
