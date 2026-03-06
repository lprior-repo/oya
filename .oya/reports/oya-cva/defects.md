# defect evidence

bead: `oya-cva`

- verdict: fail
- error: terminal Workspace: Jj { args: ["workspace", "add", "/home/lewis/src/oya/oya-oya-cva", "--name", "oya-oya-cva"], cwd: None } exited with Some(1): Error: There is no jj repo in "."
Hint: It looks like this is a git repo. You can create a jj repo backed by it by running this:
jj git init
- compensation: mark_bead_blocked target=oya-cva success=true error=
- compensation: forget_workspace target=oya-oya-cva success=true error=
