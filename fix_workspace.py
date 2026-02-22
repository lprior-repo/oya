import re

with open("src/pipeline/mod.rs", "r") as f:
    content = f.read()

# Remove pub enum WorkspacePreparationPolicy and impl
content = re.sub(r'#\[derive\(Clone, Copy\)\]\n\s*pub enum WorkspacePreparationPolicy \{[\s\S]*?pub fn should_skip\(self\) -> bool \{\n\s*matches!\(self, Self::Skip\)\n\s*\}\n\s*\}\n', '', content)

# Remove workspace_policy field from RuntimeConfig
content = re.sub(r'\s*pub\(super\) workspace_policy: WorkspacePreparationPolicy,\n', '\n', content)

# Remove workspace_policy initialization
content = re.sub(r'\s*workspace_policy: WorkspacePreparationPolicy::from_skip_flag\(true\),\n', '\n', content)

with open("src/pipeline/mod.rs", "w") as f:
    f.write(content)


with open("src/pipeline/executor.rs", "r") as f:
    content = f.read()

# Remove workspace_policy from WorkspacePrepRequest
content = re.sub(r'\s*workspace_policy: config\.workspace_policy,\n', '\n', content)

with open("src/pipeline/executor.rs", "w") as f:
    f.write(content)
