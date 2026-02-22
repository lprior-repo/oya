# Codanna Setup (Repo-Scoped)

This repository uses a local Codanna config so it stays isolated from your other repos.

## Config Files

- `.codanna/settings.toml` - repo-specific Codanna settings
- `.codannaignore` - ignore patterns for this repo only
- `.opencode/opencode.json` - OpenCode MCP entry using this repo config

## What Is Indexed

- Code: `src/`, `tests/`
- Docs collection: `docs/`

## Verify Repo Scope

Run:

```bash
codanna --config .codanna/settings.toml --info config
```

Expected:

- `workspace_root = "/home/lewis/src/oya"`
- `indexed_paths` only include OYA paths

## Rebuild Indexes

```bash
codanna --config .codanna/settings.toml index
codanna --config .codanna/settings.toml documents index
```

## MCP Server (OpenCode)

Configured command:

```bash
codanna --config .codanna/settings.toml serve --watch
```

This keeps code intelligence scoped to this repository and prevents cross-repo index bleed.
