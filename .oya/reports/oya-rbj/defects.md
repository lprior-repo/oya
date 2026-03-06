# defect evidence

bead: `oya-rbj`

- verdict: fail
- error: terminal Workspace: Jj { args: ["workspace", "add", "/home/lewis/src/oya/oya-oya-rbj", "--name", "oya-oya-rbj"], cwd: None } exited with Some(1): Error: There is no jj repo in "."
Hint: It looks like this is a git repo. You can create a jj repo backed by it by running this:
jj git init
- compensation: mark_bead_blocked target=oya-rbj success=true error=
- compensation: forget_workspace target=oya-oya-rbj success=true error=
