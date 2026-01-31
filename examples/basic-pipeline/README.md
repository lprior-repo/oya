# Basic Pipeline Example

Minimal example showing Factory pipeline usage.

## Setup

```bash
# From repo root, initialize jj
jj git init

# Create a new task
factory new -s my-feature

# This creates:
# - .factory-workspaces/my-feature-<id>/  (isolated workspace)
# - .factory/my-feature                    (symlink)
```

## Running Pipeline Stages

```bash
# Build/compile
factory stage -s my-feature --stage implement

# Run tests
factory stage -s my-feature --stage unit-test

# Full pipeline (implement through lint)
factory stage -s my-feature --from implement --to lint

# Check status
factory show -s my-feature --detailed
```

## Pipeline Flow

```
implement -> unit-test -> coverage -> lint -> static -> integration -> security -> review -> accept
```

Each stage validates code quality. Failures trigger retries based on stage config.

## Example Session

```bash
$ factory new -s add-login
Created task add-login in .factory-workspaces/add-login-abc123

$ factory stage -s add-login --stage implement
Running implement stage...
Stage passed.

$ factory stage -s add-login --stage unit-test
Running unit-test stage...
Stage passed.

$ factory show -s add-login
Task: add-login
Status: in_progress
Stages completed: implement, unit-test
```

## Approval

```bash
# Mark ready for integration
factory approve -s add-login

# Task moves to merged state after CI passes
```
