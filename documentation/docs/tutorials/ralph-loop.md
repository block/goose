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

## Quick Install

Copy and paste this snippet in your terminal to download the Ralph Loop files:

```bash
mkdir -p ~/.config/goose/recipes

curl -sL https://raw.githubusercontent.com/block/goose/main/documentation/src/pages/recipes/data/recipes/ralph-loop.sh -o ~/.config/goose/recipes/ralph-loop.sh
curl -sL https://raw.githubusercontent.com/block/goose/main/documentation/src/pages/recipes/data/recipes/ralph-work.yaml -o ~/.config/goose/recipes/ralph-work.yaml
curl -sL https://raw.githubusercontent.com/block/goose/main/documentation/src/pages/recipes/data/recipes/ralph-review.yaml -o ~/.config/goose/recipes/ralph-review.yaml

chmod +x ~/.config/goose/recipes/ralph-loop.sh
```

This installs three files into `~/.config/goose/recipes/`:

| File | Purpose |
|------|---------|
| `ralph-loop.sh` | Bash wrapper that orchestrates the loop |
| `ralph-work.yaml` | Recipe for the work phase |
| `ralph-review.yaml` | Recipe for the review phase |

## What's in the Files

<details>
<summary>1. The Bash Wrapper (`ralph-loop.sh`)</summary>

This is the entry point that orchestrates the loop:

```bash
#!/bin/bash
#
# Ralph Wiggum Loop - Multi-Model Edition
#
# Usage: ./ralph-loop.sh "your task description here"
#    or: ./ralph-loop.sh /path/to/task.md
#
# Environment variables:
#   RALPH_WORKER_MODEL    - Model for work phase (default: current GOOSE_MODEL)
#   RALPH_REVIEWER_MODEL  - Model for review phase (default: claude-sonnet-4-20250514)
#   RALPH_MAX_ITERATIONS  - Max iterations (default: 50)
#   RALPH_RECIPE_DIR      - Recipe directory (default: ~/.config/goose/recipes)
#

set -e

INPUT="$1"
MAX_ITERATIONS="${RALPH_MAX_ITERATIONS:-50}"
RECIPE_DIR="${RALPH_RECIPE_DIR:-$HOME/.config/goose/recipes}"
WORKER_MODEL="${RALPH_WORKER_MODEL:-}"
REVIEWER_MODEL="${RALPH_REVIEWER_MODEL:-claude-sonnet-4-20250514}"

RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m'

if [ -z "$INPUT" ]; then
    echo -e "${RED}Error: No task provided${NC}"
    echo "Usage: $0 \"your task description\""
    echo "   or: $0 /path/to/task.md"
    exit 1
fi

STATE_DIR=".goose/ralph"
mkdir -p "$STATE_DIR"

if [ -f "$INPUT" ]; then
    cp "$INPUT" "$STATE_DIR/task.md"
    echo -e "${BLUE}Reading task from file: $INPUT${NC}"
else
    echo "$INPUT" > "$STATE_DIR/task.md"
fi

TASK=$(cat "$STATE_DIR/task.md")

rm -f "$STATE_DIR/review-result.txt"
rm -f "$STATE_DIR/review-feedback.txt"
rm -f "$STATE_DIR/work-complete.txt"
rm -f "$STATE_DIR/work-summary.txt"

echo -e "${BLUE}═══════════════════════════════════════════════════════════════${NC}"
echo -e "${BLUE}  Ralph Wiggum Loop - Multi-Model Edition${NC}"
echo -e "${BLUE}═══════════════════════════════════════════════════════════════${NC}"
echo ""
echo -e "  Task: ${YELLOW}$TASK${NC}"
echo -e "  Worker Model: ${WORKER_MODEL:-default}"
echo -e "  Reviewer Model: $REVIEWER_MODEL"
echo -e "  Max Iterations: $MAX_ITERATIONS"
echo ""

for i in $(seq 1 "$MAX_ITERATIONS"); do
    echo -e "${BLUE}───────────────────────────────────────────────────────────────${NC}"
    echo -e "${BLUE}  Iteration $i / $MAX_ITERATIONS${NC}"
    echo -e "${BLUE}───────────────────────────────────────────────────────────────${NC}"
    
    echo "$i" > "$STATE_DIR/iteration.txt"
    
    echo ""
    echo -e "${YELLOW}▶ WORK PHASE${NC}"
    
    if [ -n "$WORKER_MODEL" ]; then
        GOOSE_MODEL="$WORKER_MODEL" goose run --recipe "$RECIPE_DIR/ralph-work.yaml" || {
            echo -e "${RED}✗ WORK PHASE FAILED${NC}"
            exit 1
        }
    else
        goose run --recipe "$RECIPE_DIR/ralph-work.yaml" || {
            echo -e "${RED}✗ WORK PHASE FAILED${NC}"
            exit 1
        }
    fi
    
    if [ -f "$STATE_DIR/RALPH-BLOCKED.md" ]; then
        echo ""
        echo -e "${RED}✗ BLOCKED${NC}"
        cat "$STATE_DIR/RALPH-BLOCKED.md"
        exit 1
    fi
    
    echo ""
    echo -e "${YELLOW}▶ REVIEW PHASE${NC}"
    
    GOOSE_MODEL="$REVIEWER_MODEL" goose run --recipe "$RECIPE_DIR/ralph-review.yaml" || {
        echo -e "${RED}✗ REVIEW PHASE FAILED${NC}"
        exit 1
    }
    
    if [ -f "$STATE_DIR/review-result.txt" ]; then
        RESULT=$(cat "$STATE_DIR/review-result.txt" | tr -d '[:space:]')
        
        if [ "$RESULT" = "SHIP" ]; then
            echo ""
            echo -e "${GREEN}═══════════════════════════════════════════════════════════════${NC}"
            echo -e "${GREEN}  ✓ SHIPPED after $i iteration(s)${NC}"
            echo -e "${GREEN}═══════════════════════════════════════════════════════════════${NC}"
            echo "COMPLETE: $(date)" > "$STATE_DIR/.ralph-complete"
            exit 0
        else
            echo ""
            echo -e "${YELLOW}↻ REVISE - Feedback for next iteration:${NC}"
            if [ -f "$STATE_DIR/review-feedback.txt" ]; then
                cat "$STATE_DIR/review-feedback.txt"
            fi
        fi
    else
        echo -e "${RED}✗ No review result found${NC}"
        exit 1
    fi
    
    rm -f "$STATE_DIR/work-complete.txt"
    rm -f "$STATE_DIR/review-result.txt"
    echo ""
done

echo -e "${RED}✗ Max iterations ($MAX_ITERATIONS) reached${NC}"
exit 1
```

</details>

<details>
<summary>2. Work Phase Recipe (`ralph-work.yaml`)</summary>

This recipe runs one iteration of work:

```yaml
version: 1.0.0
title: Ralph Work Phase
description: Single iteration of work - fresh context each time

instructions: |
  You are in a RALPH WIGGUM LOOP - one iteration of work.
  
  Your work persists through FILES ONLY. You will NOT remember previous iterations.
  
  STATE FILES (in .goose/ralph/):
  - task.md = The task you need to accomplish (READ THIS FIRST)
  - iteration.txt = Current iteration number
  - review-feedback.txt = Feedback from last review (if any)
  - work-complete.txt = Create when task is DONE (reviewer will verify)
  
  FIRST: Check your state
  1. cat .goose/ralph/task.md (YOUR TASK)
  2. cat .goose/ralph/iteration.txt 2>/dev/null || echo "1"
  3. cat .goose/ralph/review-feedback.txt 2>/dev/null
  4. ls -la to see existing work
  
  THEN: Make progress
  - If review-feedback.txt exists, ADDRESS THAT FEEDBACK FIRST
  - Read existing code/files before modifying
  - Make meaningful incremental progress
  - Run tests/verification if applicable
  
  FINALLY: Signal status
  - If task is complete: echo "done" > .goose/ralph/work-complete.txt
  - Always write a summary: echo "what I did" > .goose/ralph/work-summary.txt

prompt: |
  ## Ralph Work Phase
  
  Read your task from: .goose/ralph/task.md
  
  1. Read the task: `cat .goose/ralph/task.md`
  2. Check iteration: `cat .goose/ralph/iteration.txt 2>/dev/null || echo "1"`
  3. Check for review feedback: `cat .goose/ralph/review-feedback.txt 2>/dev/null`
  4. List existing files: `ls -la`
  5. Do the work (address feedback if any, otherwise make progress)
  6. Write summary: `echo "summary" > .goose/ralph/work-summary.txt`
  7. If complete: `echo "done" > .goose/ralph/work-complete.txt`

extensions:
  - type: builtin
    name: developer
    timeout: 600
```

</details>

<details>
<summary>3. Review Phase Recipe (`ralph-review.yaml`)</summary>

This recipe reviews the work with a different model:

```yaml
version: 1.0.0
title: Ralph Review Phase
description: Cross-model review of work - returns SHIP or REVISE

instructions: |
  You are a CODE REVIEWER in a Ralph Wiggum loop.
  
  Your job: Review the work done and decide SHIP or REVISE.
  
  You are a DIFFERENT MODEL than the worker. Your fresh perspective catches mistakes.
  
  STATE FILES (in .goose/ralph/):
  - task.md = The original task (READ THIS FIRST)
  - work-summary.txt = What the worker claims to have done
  - work-complete.txt = Exists if worker claims task is complete
  
  REVIEW CRITERIA:
  1. Does the code/work actually accomplish the task?
  2. Does it run without errors?
  3. Is it reasonably complete, not half-done?
  4. Are there obvious bugs or issues?
  
  BE STRICT but FAIR:
  - Don't nitpick style if functionality is correct
  - DO reject incomplete work
  - DO reject code that doesn't run
  - DO reject if tests fail
  
  OUTPUT:
  If approved: echo "SHIP" > .goose/ralph/review-result.txt
  If needs work: 
    echo "REVISE" > .goose/ralph/review-result.txt
    echo "specific feedback" > .goose/ralph/review-feedback.txt

prompt: |
  ## Ralph Review Phase
  
  1. Read the task: `cat .goose/ralph/task.md`
  2. Read work summary: `cat .goose/ralph/work-summary.txt`
  3. Check if complete: `cat .goose/ralph/work-complete.txt 2>/dev/null`
  4. Examine the actual files created/modified
  5. Run verification (tests, build, etc.)
  6. Decide: SHIP or REVISE
  
  If SHIP: `echo "SHIP" > .goose/ralph/review-result.txt`
  If REVISE: 
    `echo "REVISE" > .goose/ralph/review-result.txt`
    `echo "specific feedback" > .goose/ralph/review-feedback.txt`

extensions:
  - type: builtin
    name: developer
    timeout: 300
```

</details>

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
