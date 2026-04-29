import re

content = open("src/ui/sidebar.rs").read()

old_templates = r"const NODE_TEMPLATES.*?\];"

new_templates = """const NODE_TEMPLATES: [NodeTemplate; 24] = [
    NodeTemplate { node_type: "http-handler", label: "HTTP Handler", description: "Handle HTTP/gRPC invocation", icon: "globe", category: "entry" },
    NodeTemplate { node_type: "kafka-handler", label: "Kafka Consumer", description: "Consume events from Kafka topic", icon: "kafka", category: "entry" },
    NodeTemplate { node_type: "cron-trigger", label: "Cron Trigger", description: "Schedule periodic execution", icon: "clock", category: "entry" },
    NodeTemplate { node_type: "workflow-submit", label: "Workflow Submit", description: "Submit workflow with key", icon: "play-circle", category: "entry" },
    
    NodeTemplate { node_type: "run", label: "Durable Step", description: "ctx.run() - persisted side effect", icon: "shield", category: "durable" },
    NodeTemplate { node_type: "service-call", label: "Service Call", description: "Request-response to service", icon: "arrow-right", category: "durable" },
    NodeTemplate { node_type: "object-call", label: "Object Call", description: "Call virtual object handler", icon: "box", category: "durable" },
    NodeTemplate { node_type: "workflow-call", label: "Workflow Call", description: "Submit or attach to workflow", icon: "workflow", category: "durable" },
    NodeTemplate { node_type: "send-message", label: "Send Message", description: "Fire-and-forget one-way call", icon: "send", category: "durable" },
    NodeTemplate { node_type: "delayed-send", label: "Delayed Message", description: "Schedule future handler call", icon: "clock-send", category: "durable" },
    
    NodeTemplate { node_type: "get-state", label: "Get State", description: "ctx.get() - read persisted state", icon: "download", category: "state" },
    NodeTemplate { node_type: "set-state", label: "Set State", description: "ctx.set() - write persisted state", icon: "upload", category: "state" },
    NodeTemplate { node_type: "clear-state", label: "Clear State", description: "ctx.clear() / clearAll()", icon: "eraser", category: "state" },
    
    NodeTemplate { node_type: "condition", label: "If / Else", description: "Conditional branching", icon: "git-branch", category: "flow" },
    NodeTemplate { node_type: "switch", label: "Switch", description: "Multi-path routing", icon: "git-fork", category: "flow" },
    NodeTemplate { node_type: "loop", label: "Loop / Iterate", description: "Iterate over collection", icon: "repeat", category: "flow" },
    NodeTemplate { node_type: "parallel", label: "Parallel", description: "Promise.all() concurrent steps", icon: "layers", category: "flow" },
    NodeTemplate { node_type: "compensate", label: "Compensate", description: "Saga compensation / rollback", icon: "undo", category: "flow" },
    
    NodeTemplate { node_type: "sleep", label: "Sleep / Timer", description: "ctx.sleep() - durable pause", icon: "timer", category: "timing" },
    NodeTemplate { node_type: "timeout", label: "Timeout", description: "orTimeout() - deadline guard", icon: "alarm", category: "timing" },
    
    NodeTemplate { node_type: "durable-promise", label: "Durable Promise", description: "ctx.promise() - await external", icon: "sparkles", category: "signal" },
    NodeTemplate { node_type: "awakeable", label: "Awakeable", description: "Pause for external completion", icon: "bell", category: "signal" },
    NodeTemplate { node_type: "resolve-promise", label: "Resolve Promise", description: "Resolve a durable promise", icon: "check-circle", category: "signal" },
    NodeTemplate { node_type: "signal-handler", label: "Signal Handler", description: "Shared handler for signals", icon: "radio", category: "signal" },
];"""

content = re.sub(old_templates, new_templates, content, flags=re.DOTALL)

old_cat_match = """                    let category_label = match cat {
                        "trigger" => "Triggers",
                        "action" => "Actions",
                        "logic" => "Logic & Flow",
                        "output" => "Outputs",
                        "restate" => "Restate Workflows",
                        _ => cat,
                    };"""

new_cat_match = """                    let category_label = match cat {
                        "entry" => "Entry Points",
                        "durable" => "Durable Steps",
                        "state" => "State",
                        "flow" => "Control Flow",
                        "timing" => "Timing & Events",
                        "signal" => "Signals & Promises",
                        _ => cat,
                    };"""

content = content.replace(old_cat_match, new_cat_match)

old_order = """        let categories = ["trigger", "action", "logic", "output", "restate"];"""
new_order = """        let categories = ["entry", "durable", "state", "flow", "timing", "signal"];"""

content = content.replace(old_order, new_order)

old_border_bg = """                                    let (border_color, text_color, bg_color) = match template.category {
                                        "trigger" => ("border-emerald-500/20", "text-emerald-400", "bg-emerald-500/10"),
                                        "action" => ("border-indigo-500/20", "text-indigo-400", "bg-indigo-500/10"),
                                        "logic" => ("border-amber-500/20", "text-amber-400", "bg-amber-500/10"),
                                        "output" => ("border-pink-500/20", "text-pink-400", "bg-pink-500/10"),
                                        "restate" => ("border-blue-500/20", "text-blue-400", "bg-blue-500/10"),
                                        _ => ("border-slate-700", "text-slate-400", "bg-slate-800"),
                                    };"""

new_border_bg = """                                    let (border_color, text_color, bg_color) = match template.category {
                                        "entry" => ("border-emerald-500/20", "text-emerald-400", "bg-emerald-500/10"),
                                        "durable" => ("border-indigo-500/20", "text-indigo-400", "bg-indigo-500/10"),
                                        "state" => ("border-orange-500/20", "text-orange-400", "bg-orange-500/10"),
                                        "flow" => ("border-amber-500/20", "text-amber-400", "bg-amber-500/10"),
                                        "timing" => ("border-pink-500/20", "text-pink-400", "bg-pink-500/10"),
                                        "signal" => ("border-blue-500/20", "text-blue-400", "bg-blue-500/10"),
                                        _ => ("border-slate-700", "text-slate-400", "bg-slate-800"),
                                    };"""

content = content.replace(old_border_bg, new_border_bg)

with open("src/ui/sidebar.rs", "w") as f:
    f.write(content)
