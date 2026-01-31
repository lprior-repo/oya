# Jujutsu: Version Control

Git alternative optimized for stack-based development and instant branching.

## Core Concepts

- **Working Copy** - Your current changes (automatically tracked)
- **Changes** - Immutable commits (can be rearranged)
- **Bookmarks** - Branch pointers
- **Revisions** - Commits (immutable)

## Quick Start

### Status & Diff

```bash
jj status              # Current state
jj diff                # Your changes
jj log                 # Commit history
```

### Making Changes

```bash
# Edit files (automatically tracked)
vim src/lib.rs

# See what changed
jj diff
jj status

# Describe the change
jj describe -m "feat: add validation

- Implement ValidatorBuilder
- Add error types
- Test coverage"

# Start next change
jj new
```

### Remote Operations

```bash
# Fetch latest
jj git fetch --all-remotes

# Push changes
jj git push

# Check if pushed
jj log -r @
```

## Common Commands

### View Information

```bash
jj status                      # Current status
jj diff                        # Changes in working copy
jj diff -r BD-123              # Changes in specific revision
jj log                         # Commit history
jj log -r @                    # Current commit
jj log -r origin/main..@       # Unpushed commits
jj describe -r @               # Current commit message
```

### Managing Changes

```bash
jj describe -m "message"       # Set current commit message
jj describe -e                 # Edit message in editor
jj new                         # Create new change
jj edit -r <revision>          # Edit existing revision
jj squash                      # Squash into parent
jj abandon <revision>          # Discard a commit
```

### Branches (Bookmarks)

```bash
jj bookmark list               # List all bookmarks
jj bookmark set feature/x      # Create/move bookmark
jj bookmark delete feature/x   # Delete bookmark
jj bookmark move --from feature/x --to feature/y  # Rename
```

### Working with Remotes

```bash
jj git fetch                   # Fetch default remote
jj git fetch --all-remotes     # Fetch all remotes
jj git push                    # Push to default remote
jj git push --all              # Push to all remotes
```

### Undoing Changes

```bash
jj undo <revision>             # Undo specific commit
jj restore                     # Restore from parent
jj restore --source <rev>      # Restore specific file
```

### Moving Changes

```bash
jj move <source> <dest>        # Move change to new parent
jj rebase -r <rev> -d <new_parent>  # Rebase change
```

## Workflow

### Single Change

```bash
# Make change
vim src/lib.rs

# Commit
jj describe -m "feat: implement X"

# Push
jj git push
```

### Multiple Changes (Stack)

```bash
# First change
vim src/a.rs
jj describe -m "feat: part 1"

# Create next change
jj new

# Second change
vim src/b.rs
jj describe -m "feat: part 2"

# Create next change
jj new

# Third change (if needed)
vim src/c.rs
jj describe -m "feat: part 3"

# Push all at once
jj git push
```

### Feature Branch

```bash
# Create feature branch
jj bookmark set feature/cool-thing

# Make changes
vim src/lib.rs
jj describe -m "feat: cool thing"
jj new

# More changes on feature branch
vim src/lib.rs
jj describe -m "test: add tests"
jj new

# Switch back to main when done
jj bookmark set main
```

### Reordering Changes

```bash
# If you have changes A, B, C and want B, A, C:
jj log                    # See current order
jj move -r B -d A^        # Move B before A
```

## Conventional Commits

```bash
jj describe -m "feat: add validation

- Implement validator builder
- Add error types
- Add test suite

Closes BD-123"
```

**Format**:
```
<type>: <description>

<body>

<footer>
```

**Types**:
- `feat` - New feature
- `fix` - Bug fix
- `refactor` - Code refactoring
- `chore` - Build, deps, tooling
- `docs` - Documentation
- `test` - Test additions/changes
- `perf` - Performance improvement

## Editing & Squashing

### Edit Commit Message

```bash
jj describe -e  # Opens editor
# Make changes and save
```

### Squash Into Parent

```bash
jj squash  # Squashes current into parent
```

### Squash Multiple

```bash
# If you have A, B, C and want A (B+C):
jj squash -r B    # Squash B into A
jj squash -r C    # Squash C into A
```

## Working with Conflicts

### Automatic Conflict Resolution

```bash
jj git fetch --all-remotes
# jj automatically handles conflicts

# Check status
jj diff  # Shows any remaining conflicts

# Resolve manually
vim conflicted_file.rs

# Commit resolution
jj describe -m "merge: resolve conflicts"
jj git push
```

## Rebasing

### Rebase onto main

```bash
# If main moved and you want to rebase
jj rebase -d main
```

### Rebase range

```bash
# Rebase changes A, B, C onto D
jj rebase -r "A::C" -d D
```

## Integration with Beads

### Link Commits to Issues

Use issue ID in commit message:
```bash
jj describe -m "feat: implement validation

- ...

Closes BD-123"
```

`bv` will correlate commits back to Beads issues.

### Tracking Progress

```bash
# While working on BD-123
jj log              # See your commits
jj git push         # Push progress
bd claim BD-123     # Still claimed in Beads

# When done
jj git push         # Final push
bd complete BD-123  # Close in Beads
```

## Troubleshooting

### "Commit not found"

```bash
jj log  # Find the commit hash
jj edit <hash>  # Use hash directly
```

### "Can't push"

```bash
# Fetch first to get latest
jj git fetch --all-remotes

# Then push
jj git push
```

### "Conflicts after fetch"

```bash
jj status   # Shows conflicts
jj diff     # See conflicted files
# Resolve manually
vim conflicted.rs
jj describe -m "merge: resolved"
```

### "Undo a change"

```bash
jj undo <revision>  # Undo that revision
```

### "Wrong bookmark"

```bash
jj bookmark list                  # See current
jj bookmark set correct-name      # Move current bookmark
jj bookmark delete wrong-name     # Delete wrong one
```

## Advanced

### Moving Commits Between Branches

```bash
jj move -r <commit> -d <new-parent>  # Move commit to new parent
```

### Rewriting History

```bash
# Only do this BEFORE pushing!
jj squash              # Squash into parent
jj abandon <rev>       # Delete revision
jj rebase -r <rev>     # Rebase revision
```

### Iterative Development

```bash
# Make small commits
jj describe -m "wip: work in progress"
jj new

# Make another change
vim file.rs
jj describe -m "wip: more progress"
jj new

# Later, squash into single clean commit
jj squash
jj squash
# Now have one clean commit
```

## Integration with Moon

Commits tracked automatically in Beads history via `bv --robot-history`.

```bash
# Make changes
vim src/lib.rs

# Test
moon run :test

# Commit
jj describe -m "feat: ..."

# Push
jj git push
```

## Philosophy

> "Jujutsu treats commits as immutable, composable units. Rearrange them freely before pushing, then they're locked once on remote."

Benefits:
- ✅ Instant branching (just a bookmark)
- ✅ Reorder commits before pushing
- ✅ Stacking features easily
- ✅ Clean history (no merge commits)
- ✅ Deterministic (no conflicts in most cases)

---

**Next**: [Complete Docs Index](00_START_HERE.md)
