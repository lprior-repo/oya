import re

content = open("src/ui/node.rs").read()

content = content.replace("let (bg_color, text_color, border_color, icon_name, is_spin) =", "let tuple =")
content = content.replace("                                }", "                                }") # Ensure spacing

old_rsx = """                            rsx! {
                                span {
                                    class: "inline-flex items-center gap-1 rounded-full border px-1.5 py-px text-[9px] font-medium leading-none {bg_color} {text_color} {border_color}",
                                    {icon_by_name(icon_name, format!("h-2.5 w-2.5 {}", if is_spin { "animate-spin" } else { "" }))}
                                    "{label}"
                                }
                            }"""

new_rsx = """                            let bg_color = tuple.0;
                            let text_color = tuple.1;
                            let border_color = tuple.2;
                            let icon_name = tuple.3;
                            let is_spin = tuple.4;
                            let icon_class = if is_spin { format!("h-2.5 w-2.5 animate-spin") } else { "h-2.5 w-2.5".to_string() };
                            rsx! {
                                span {
                                    class: "inline-flex items-center gap-1 rounded-full border px-1.5 py-px text-[9px] font-medium leading-none {bg_color} {text_color} {border_color}",
                                    {icon_by_name(icon_name, icon_class)}
                                    "{label}"
                                }
                            }"""

content = content.replace(old_rsx, new_rsx)

with open("src/ui/node.rs", "w") as f:
    f.write(content)
