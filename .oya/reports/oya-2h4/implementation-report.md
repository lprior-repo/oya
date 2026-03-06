# implementation evidence

bead: `oya-2h4`

- effect: `Bd { args: ["update", "oya-2h4", "--status", "in_progress"], cwd: None }` success: true
- effect: `WorkspacePrepare { workspace: WorkspaceName("oya-oya-2h4"), path: "/home/lewis/src/oya-oya-2h4" }` success: true
- effect: `Jj { args: ["workspace", "add", "/home/lewis/src/oya-oya-2h4", "--name", "oya-oya-2h4"], cwd: None }` success: true
- effect: `Opencode { prompt: "Implement bead oya-2h4 in this workspace using functional-rust approach and tests derived from contract. Do not call `oya` or `br`. Use moon/jj/gh as needed. Return one JSON receipt object with required keys: objective, allowed_scope, files_touched, commands, exit_codes, key_stdout_stderr, diff_summary, risks_unknowns, pass_fail_recommendation.", model: "zai-coding-plan/glm-5", cwd: Some("/home/lewis/src/oya-oya-2h4") }` success: true
