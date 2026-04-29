import re

content = open("src/graph/mod.rs").read()

old_enum = """pub enum NodeCategory {
    Trigger,
    Action,
    Logic,
    Output,
    Restate,
}"""

new_enum = """pub enum NodeCategory {
    Entry,
    Durable,
    State,
    Flow,
    Timing,
    Signal,
}"""

old_fmt = """        let s = match self {
            Self::Trigger => "trigger",
            Self::Action => "action",
            Self::Logic => "logic",
            Self::Output => "output",
            Self::Restate => "restate",
        };"""

new_fmt = """        let s = match self {
            Self::Entry => "entry",
            Self::Durable => "durable",
            Self::State => "state",
            Self::Flow => "flow",
            Self::Timing => "timing",
            Self::Signal => "signal",
        };"""

content = content.replace(old_enum, new_enum)
content = content.replace(old_fmt, new_fmt)

with open("src/graph/mod.rs", "w") as f:
    f.write(content)
