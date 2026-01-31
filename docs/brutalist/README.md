Factory

Contract-driven CI/CD pipeline for multi-language projects.

Built in Gleam with jj workspaces for task isolation.

Usage

  gleam run --module=factory -- new -s <slug>
      Create isolated worktree for task

  gleam run --module=factory -- stage -s <slug> --stage <name>
      Run pipeline stage

  gleam run --module=factory -- approve -s <slug>
      Mark task for integration

  gleam run --module=factory -- show -s <slug>
      Display task status

  gleam run --module=factory -- list
      List all tasks

See AGENTS.md for architecture details.
See docs/ARCHITECTURE.md for full documentation.
