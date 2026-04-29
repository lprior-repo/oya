import re

content = open("src/graph/mod.rs").read()

old_func = """    fn get_node_metadata(node_type: &str) -> (NodeCategory, String, String) {"""
new_func = """    #[allow(clippy::too_many_lines)]
    fn get_node_metadata(node_type: &str) -> (NodeCategory, String, String) {"""

content = content.replace(old_func, new_func)

with open("src/graph/mod.rs", "w") as f:
    f.write(content)
