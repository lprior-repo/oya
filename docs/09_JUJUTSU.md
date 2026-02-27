# Jujutsu Workspaces

Oya lifecycle uses jj for isolation and landing.

## Typical Sequence

```bash
jj workspace add <workspace-path> --name <workspace>
jj git fetch
jj rebase -d main@origin
jj bookmark set <name> -r @
jj git push --bookmark <name>
jj workspace forget <workspace>
```

For lifecycle automation, workspace naming is derived from bead/run identity and managed by the workflow engine.
