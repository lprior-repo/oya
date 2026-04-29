import re

content = open("src/graph/layout.rs").read()

content = content.replace("which should not happen for f32 partial_cmp.", "which should not happen for f32 `partial_cmp`.")

with open("src/graph/layout.rs", "w") as f:
    f.write(content)
