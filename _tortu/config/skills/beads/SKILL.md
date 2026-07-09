---
name: beads
description: Use beads (bd) for persistent task tracking in coding projects. A git-backed issue tracker designed for AI agents with dependency graphs, hierarchical tasks, and multi-agent coordination.
author: goose
version: "1.0"
tags:
  - development
  - productivity
  - task-management
  - workflow
---

This skill teaches effective use of **beads** (`bd`), a distributed git-backed issue tracker designed for AI agents. Use beads to track tasks, manage dependencies, and coordinate work across sessions.

## Getting Started

First, check if beads is available and initialized:

```bash
# Check if bd is installed
bd version

# Check if current project has beads initialized
bd status
```

**If `bd` is not installed**, ask the user which installation method they prefer:
- **npm:** `npm install -g @beads/bd`
- **Homebrew:** `brew install beads` (macOS)
- **Go:** `go install github.com/steveyegge/beads/cmd/bd@latest`

Do not install without user confirmation, as these are global system packages.

**If not initialized in the project**, run:
```bash
bd init
```

For personal use on shared projects (won't commit to repo):
```bash
bd init --stealth
```

For contributors on forked repos (routes to separate planning repo):
```bash
bd init --contributor
```

## Essential Commands

| Command | Purpose |
|---------|---------|
| `bd ready` | List tasks with no open blockers (what to work on next) |
| `bd create "Title" -p 1` | Create a task (priority 0-3, lower = higher priority) |
| `bd show <id>` | View task details and dependencies |
| `bd list` | List all open issues |
| `bd close <id>` | Mark task as complete |
| `bd update <id> --status in_progress` | Update task status |
| `bd dep add <child> <parent>` | Create dependency (child blocked by parent) |
| `bd sync` | Force immediate sync to git |

**Always use `--json` flag** for machine-readable output when parsing results.

## Task Hierarchy

Beads supports hierarchical IDs for organizing work:

- `bd-a3f8` — Epic (large feature)
- `bd-a3f8.1` — Task under epic
- `bd-a3f8.1.1` — Sub-task

Create hierarchical tasks:
```bash
bd create "Epic: User Authentication" -t epic -p 1
bd create "Implement login flow" -p 1 --parent bd-a3f8
```

## Session Workflow

### Starting a Session

```bash
# See what's ready to work on
bd ready --json

# Pick a task and mark it in progress
bd update <id> --status in_progress

# View full details
bd show <id> --json
```

### During Work

```bash
# Create new tasks as you discover them
bd create "Fix edge case in validation" -p 2

# Add dependencies
bd dep add <new-task> <blocking-task>

# Update task with notes
bd update <id> --notes "Found issue with timezone handling"
```

### Ending a Session ("Land the Plane")

When finishing work, complete ALL these steps:

```bash
# 1. File issues for remaining work
bd create "TODO: Add integration tests" -p 2

# 2. Close completed tasks
bd close <id> --reason "Completed"

# 3. Sync and push (MANDATORY)
git pull --rebase
bd sync
git push

# 4. Verify push succeeded
git status  # Must show "up to date with origin"

# 5. Identify next task for follow-up
bd ready --json
```

**CRITICAL**: Always push before ending a session. Unpushed work causes coordination problems in multi-agent workflows.

## Important Rules

### DO use `bd update` with flags
```bash
bd update <id> --description "new description"
bd update <id> --title "new title"  
bd update <id> --design "design notes"
bd update <id> --notes "additional notes"
bd update <id> --acceptance "acceptance criteria"
bd update <id> --status in_progress
```

### DO NOT use `bd edit`
The `edit` command opens an interactive editor (`$EDITOR`) which AI agents cannot use. Always use `bd update` with flags instead.

### DO sync before ending sessions
```bash
bd sync  # Forces immediate export, commit, and push
```

Without `bd sync`, changes sit in a 30-second debounce window and may not be committed.

### DO include issue IDs in commit messages
```bash
git commit -m "Fix auth validation bug (bd-abc)"
```

This enables `bd doctor` to detect orphaned issues.

## Dependency Types

```bash
# Hard blocker - child cannot start until parent is done
bd dep add <child> <parent> --type blocks

# Soft link - related but not blocking  
bd dep add <issue1> <issue2> --type related

# Parent-child - hierarchical relationship
bd dep add <child> <parent> --type parent-child
```

## Handling Merge Conflicts

If conflicts occur in `.beads/issues.jsonl`:

```bash
# Accept remote version
git checkout --theirs .beads/issues.jsonl

# Re-import to database
bd import -i .beads/issues.jsonl

# Continue with your work
```

## Git Hooks (Recommended)

Install hooks for automatic sync:

```bash
bd hooks install
```

This prevents stale JSONL problems by auto-syncing on commit, merge, push, and checkout.

## MCP Server Alternative

For tighter integration, beads also offers an MCP server (`beads-mcp`) that provides tools directly to your agent. Install via:

```bash
pip install beads-mcp
```

The CLI (`bd`) and MCP server work with the same underlying database.

## Quick Reference

```bash
# What should I work on?
bd ready

# Create a task
bd create "Fix bug in login" -p 1

# Start working
bd update bd-xyz --status in_progress

# Done working
bd close bd-xyz --reason "Completed"
bd sync
git push
```
