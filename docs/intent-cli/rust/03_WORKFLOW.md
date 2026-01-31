# Daily Workflow: Beads + Jujutsu + Moon

Integration of issue tracking, version control, and build system.

## Full Workflow

### 1. Start Work

```bash
# View available issues
bd list

# Claim issue
bd claim BD-123

# Pull latest
jj git fetch --all-remotes
```

### 2. Make Changes

```bash
# Edit files (automatically tracked by jj)
vim crates/zjj-core/src/lib.rs

# Check status
jj status
jj diff

# Test locally
moon run :test
```

### 3. Commit Changes

```bash
# Describe change (conventional commits)
jj describe -m "feat: add new feature

- Implementation detail 1
- Implementation detail 2

Closes BD-123"

# Start next change
jj new
```

### 4. Push to Remote

```bash
# Fetch latest
jj git fetch --all-remotes

# Push
jj git push

# Verify
jj log -r @
```

### 5. Close Issue

```bash
# Mark complete
bd complete BD-123 --commit-hash <hash>

# Or resolve (pending review)
bd resolve BD-123
```

## Beads (Issue Tracking)

### Creating Issues

```bash
# Feature
bd add --title "Feature: X" --priority high --label feature

# Bug
bd add --title "Bug: X fails on Y" --priority high --label bug \
  --description "Steps: 1. Do X 2. See Y"

# Chore
bd add --title "Chore: refactor X" --label chore
```

### Managing Issues

```bash
bd list                           # Show all open
bd list --filter "assigned:me"    # My issues
bd claim BD-123                   # Start working
bd resolve BD-123                 # Mark ready for review
bd complete BD-123                # Mark done
bd unresolved BD-123              # Reopen
```

### Labels

```
epic       - Large feature
feature    - New functionality
bug        - Something broken
chore      - Maintenance, refactoring
p0, p1, p2 - Priority (0=highest)
```

## Jujutsu (Version Control)

### Status & Diff

```bash
jj status           # Current state
jj diff             # Changes in working copy
jj log              # Commit history
jj log -r @         # Current change
```

### Commits

```bash
# Set commit message
jj describe -m "feat: description"

# View full message
jj describe -r @

# Edit message
jj describe -e

# Start new change
jj new
```

### Conventional Commits

```
feat: New feature
fix: Bug fix
refactor: Code refactoring
chore: Build, dependencies, tooling
docs: Documentation changes
test: Test additions/modifications
perf: Performance improvements
```

### Example Commit

```bash
jj describe -m "feat: add validation builder

- Implement ValidatorBuilder struct
- Add error types for validation
- Add comprehensive tests

Closes BD-42"
```

### Working with Remotes

```bash
jj git fetch --all-remotes        # Fetch latest
jj git push                        # Push changes
jj log -r origin/main..@           # Commits not yet pushed
```

### Editing Commits

```bash
# Edit current change
jj edit

# Edit specific commit
jj edit -r BD-123

# Squash into parent
jj squash
```

## Moon (Build System)

### Before Committing

```bash
# Quick lint
moon run :quick

# If changes to logic
moon run :test
```

### Before Pushing

```bash
# Full validation
moon run :ci

# If all pass
jj git push
```

### Common Issues

```bash
# Fix formatting
cargo fmt

# Re-run tests
moon run :test

# Check lint errors
moon run :quick --log debug
```

## Typical Day

### Morning

```bash
# Check latest
jj git fetch --all-remotes

# See available work
bd list

# Pick an issue
bd claim BD-123
```

### During Work

```bash
# Iterate
vim file.rs
moon run :test
# Fix any issues
vim file.rs
moon run :test
```

### Ready to Commit

```bash
# Final validation
moon run :ci

# Commit with message
jj describe -m "feat: implement feature

- Detail 1
- Detail 2"

# Start next
jj new
```

### End of Day

```bash
# Push all changes
jj git push

# Close completed issues
bd complete BD-123
bd complete BD-124

# Review what you're working on
bd claim --show BD-125
```

## Multi-Issue Workflow

```bash
# Claim first issue
bd claim BD-123

# Make changes, commit
jj describe -m "fix: issue 123"
jj new

# Claim second issue
bd claim BD-124

# Make changes, commit
jj describe -m "feat: issue 124"
jj new

# Push all
jj git push

# Close both
bd complete BD-123
bd complete BD-124
```

## Handling Conflicts

### Update with Latest

```bash
jj git fetch --all-remotes
# jj automatically handles conflicts

# View conflicts
jj diff
```

### Resolving Conflicts

```bash
# Edit conflicted file
vim conflicted_file.rs

# Mark resolved (jj tracks this)
jj diff  # Should show no conflicts

# Commit resolution
jj describe -m "merge: resolve conflicts"
jj git push
```

## Landing (Finishing Session)

```bash
# 1. Run full pipeline
moon run :ci

# 2. File remaining work
bd add --title "Follow-up: X" --label chore

# 3. Commit final changes
jj describe -m "chore: final cleanup"
jj new

# 4. Update Beads
bd complete BD-123
bd complete BD-124

# 5. Push everything
jj git fetch --all-remotes
jj git push

# 6. Verify push
jj log -r @
```

## Common Patterns

### Feature Branch (using jj bookmarks)

```bash
# Create feature bookmark
jj bookmark set feature/cool-thing

# Make changes on current commit
# ... changes ...
jj describe -m "feat: cool thing"
jj new

# Switch back to main
jj bookmark set main

# Later, merge feature
jj bookmark set feature/cool-thing
# ... feature is now on top
```

### Stashing (Temporal Commits)

```bash
# Save work in progress
jj describe -m "wip: work in progress"

# Continue elsewhere
jj new

# Come back to WIP later
jj log
jj edit -r <wip-commit>
```

### Squashing Multiple Commits

```bash
# Make several commits
jj describe -m "feat: part 1"
jj new
jj describe -m "feat: part 2"
jj new

# Squash into parent (now just one commit)
jj squash
```

## Tips & Tricks

### See what changed since last push

```bash
jj log -r origin/main..@
```

### Abandon unwanted changes

```bash
jj abandon <revision>
```

### Revert a change

```bash
jj undo <revision>
```

### Move changes between commits

```bash
jj move <source> <destination>
```

## Troubleshooting

### "Commit not found"

Use `jj log` to find commit hash, then use hash instead of shorthand.

### "Can't push"

```bash
# Fetch first
jj git fetch --all-remotes

# Then push
jj git push
```

### "Changes not tracked"

All file changes are automatically tracked. If not appearing in `jj diff`:
```bash
jj status  # Check status
jj diff    # Show changes
```

### "Wrong commit message"

Edit before pushing:
```bash
jj describe -e  # Opens editor
jj git push     # Push corrected
```

## The Flow

1. **Beads**: Organization (what to work on)
2. **Jujutsu**: Implementation (tracking changes)
3. **Moon**: Validation (building & testing)
4. **Beads**: Closure (marking done)

Everything flows through these tools. Master them and you master ZJJ development.

---

**Next**: [Functional Patterns](04_FUNCTIONAL_PATTERNS.md)
