---
description: Run goose in a loop with fresh context per iteration and cross-model review
---

# Ralph Loop

The Ralph Loop is an iterative development pattern that keeps goose working on a task until it's genuinely complete. Unlike standard retry mechanisms, each iteration runs with **fresh context** and a **different model reviews the work** before it ships.

This technique is based on [Geoffrey Huntley's "Ralph Wiggum" approach](https://ghuntley.com/ralph/) - put an AI agent in a loop until the job is done, with file I/O as state rather than conversation history.

## Why Ralph Loop?

Standard agent loops have a problem: **context accumulates**. Every failed attempt, every wrong turn stays in the conversation history. By iteration 10, the model is wading through garbage from iterations 1-9.

Ralph Loop solves this with:

| Feature | Standard Retry | Ralph Loop |
|---------|---------------|------------|
| Context per iteration | Accumulated (same session) | Fresh (new session) |
| State persistence | Conversation + files | Files only |
| Failed attempts | Pollute future iterations | Gone |
| Review | Same model | Different model |

The key insight: **file I/O as state, not transcript**. Your files persist between iterations, but the conversation history doesn't.

## How It Works

```
Iteration 1:
  WORK PHASE  → Model A does work, writes to files
  REVIEW PHASE → Model B reviews the work
    → SHIP? Exit successfully ✓
    → REVISE? Write feedback, continue to iteration 2

Iteration 2:
  WORK PHASE  → Model A reads feedback, fixes things (fresh context!)
  REVIEW PHASE → Model B reviews again
    → SHIP? Exit successfully ✓
    → REVISE? Continue...

... repeats until SHIP or max iterations
```

## Setup

Copy and paste this snippet in your terminal to download the Ralph Loop files:

```bash
mkdir -p ~/.config/goose/recipes

curl -sL https://raw.githubusercontent.com/block/goose/main/documentation/src/pages/recipes/data/recipes/ralph-loop.sh -o ~/.config/goose/recipes/ralph-loop.sh
curl -sL https://raw.githubusercontent.com/block/goose/main/documentation/src/pages/recipes/data/recipes/ralph-work.yaml -o ~/.config/goose/recipes/ralph-work.yaml
curl -sL https://raw.githubusercontent.com/block/goose/main/documentation/src/pages/recipes/data/recipes/ralph-review.yaml -o ~/.config/goose/recipes/ralph-review.yaml

chmod +x ~/.config/goose/recipes/ralph-loop.sh
```

This installs three files:

| File | Purpose |
|------|---------|
| `ralph-loop.sh` | Bash wrapper that orchestrates the loop |
| `ralph-work.yaml` | Recipe for the work phase |
| `ralph-review.yaml` | Recipe for the review phase |

## Usage

### Basic Usage

Run with a task string:

```bash
~/.config/goose/recipes/ralph-loop.sh "build a CLI tool that converts markdown to HTML"
```

Or with a task file (better for complex tasks):

```bash
~/.config/goose/recipes/ralph-loop.sh ./my-task.md
```

### Custom Models

Override the default models:

```bash
RALPH_WORKER_MODEL="gpt-4o" \
RALPH_REVIEWER_MODEL="claude-sonnet-4-20250514" \
~/.config/goose/recipes/ralph-loop.sh ./task.md
```

### Environment Variables

| Variable | Default | Description |
|----------|---------|-------------|
| `RALPH_WORKER_MODEL` | Your `GOOSE_MODEL` | Model for work phase |
| `RALPH_REVIEWER_MODEL` | `goose-claude-4-5-sonnet` | Model for review phase |
| `RALPH_MAX_ITERATIONS` | `50` | Safety limit |
| `RALPH_RECIPE_DIR` | `~/.config/goose/recipes` | Where recipes live |

## State Files

All state lives in `.goose/ralph/` in your working directory:

| File | Purpose |
|------|---------|
| `task.md` | The task description |
| `iteration.txt` | Current iteration number |
| `work-summary.txt` | What the worker did this iteration |
| `work-complete.txt` | Exists when worker claims done |
| `review-result.txt` | `SHIP` or `REVISE` |
| `review-feedback.txt` | Feedback for next iteration |
| `.ralph-complete` | Created on successful completion |
| `RALPH-BLOCKED.md` | Created if worker is stuck |

## Starting Over

To reset and start fresh:

```bash
rm -rf .goose/ralph
```

To keep your task but reset progress:

```bash
rm -f .goose/ralph/iteration.txt .goose/ralph/review-*.txt .goose/ralph/work-*.txt
```

## When to Use Ralph Loop

Ralph Loop works best for:

- **Complex, multi-step tasks** that benefit from iteration
- **Tasks with clear completion criteria** (tests pass, builds succeed)
- **Situations where you want quality gates** before shipping

It's overkill for:

- Simple one-shot tasks
- Interactive/exploratory work
- Tasks without verifiable completion criteria

## Comparison with Lead/Worker Mode

| Aspect | Lead/Worker | Ralph Loop |
|--------|-------------|------------|
| Purpose | Cost optimization | Quality gates |
| Context | Same session | Fresh per iteration |
| Model switching | Automatic fallback | Explicit work/review phases |
| Use case | Day-to-day coding | Ship-quality deliverables |

You can combine both: use Lead/Worker for the work phase models, and Ralph Loop for the overall iteration pattern.
