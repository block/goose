#!/bin/bash
#
# Ralph Loop - Multi-Model Edition
#
# Fresh context per iteration + cross-model review
# Based on Geoffrey Huntley's technique
#
# Usage: ./ralph-loop.sh "your task description here"
#    or: ./ralph-loop.sh /path/to/task.md
#
# Environment variables:
#   RALPH_WORKER_MODEL    - Model for work phase (prompts if not set)
#   RALPH_WORKER_PROVIDER - Provider for work phase (prompts if not set)
#   RALPH_REVIEWER_MODEL  - Model for review phase (prompts if not set)
#   RALPH_REVIEWER_PROVIDER - Provider for review phase (prompts if not set)
#   RALPH_MAX_ITERATIONS  - Max iterations (default: 10)
#   RALPH_RECIPE_DIR      - Recipe directory (default: ~/.config/goose/recipes)
#

set -e

INPUT="$1"
RECIPE_DIR="${RALPH_RECIPE_DIR:-$HOME/.config/goose/recipes}"

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

# Function to prompt for settings
prompt_for_settings() {
    local default_model="${GOOSE_MODEL:-}"
    local default_provider="${GOOSE_PROVIDER:-}"
    
    # Worker model
    if [ -n "$default_model" ]; then
        echo -ne "${BLUE}Worker model${NC} [${default_model}]: "
        read -r user_input
        WORKER_MODEL="${user_input:-$default_model}"
    else
        echo -ne "${BLUE}Worker model${NC}: "
        read -r WORKER_MODEL
        if [ -z "$WORKER_MODEL" ]; then
            echo -e "${RED}Error: Worker model is required${NC}"
            exit 1
        fi
    fi
    
    # Worker provider
    if [ -n "$default_provider" ]; then
        echo -ne "${BLUE}Worker provider${NC} [${default_provider}]: "
        read -r user_input
        WORKER_PROVIDER="${user_input:-$default_provider}"
    else
        echo -ne "${BLUE}Worker provider${NC}: "
        read -r WORKER_PROVIDER
        if [ -z "$WORKER_PROVIDER" ]; then
            echo -e "${RED}Error: Worker provider is required${NC}"
            exit 1
        fi
    fi
    
    # Reviewer model
    echo -ne "${BLUE}Reviewer model${NC} (should be different from worker): "
    read -r REVIEWER_MODEL
    if [ -z "$REVIEWER_MODEL" ]; then
        echo -e "${RED}Error: Reviewer model is required${NC}"
        echo "The reviewer should be a different model to provide fresh perspective."
        exit 1
    fi
    
    # Reviewer provider
    echo -ne "${BLUE}Reviewer provider${NC}: "
    read -r REVIEWER_PROVIDER
    if [ -z "$REVIEWER_PROVIDER" ]; then
        echo -e "${RED}Error: Reviewer provider is required${NC}"
        exit 1
    fi
    
    # Same model warning
    if [ "$WORKER_MODEL" = "$REVIEWER_MODEL" ] && [ "$WORKER_PROVIDER" = "$REVIEWER_PROVIDER" ]; then
        echo -e "${YELLOW}Warning: Worker and reviewer are the same model.${NC}"
        echo "For best results, use different models for cross-model review."
        echo -ne "Continue anyway? [y/N]: "
        read -r confirm
        if [ "$confirm" != "y" ] && [ "$confirm" != "Y" ]; then
            exit 1
        fi
    fi
    
    # Max iterations
    echo -ne "${BLUE}Max iterations${NC} [10]: "
    read -r user_input
    MAX_ITERATIONS="${user_input:-10}"
}

# Initialize from environment variables
WORKER_MODEL="${RALPH_WORKER_MODEL:-}"
WORKER_PROVIDER="${RALPH_WORKER_PROVIDER:-}"
REVIEWER_MODEL="${RALPH_REVIEWER_MODEL:-}"
REVIEWER_PROVIDER="${RALPH_REVIEWER_PROVIDER:-}"
MAX_ITERATIONS="${RALPH_MAX_ITERATIONS:-10}"

# If any required setting is missing, prompt for all settings
if [ -z "$WORKER_MODEL" ] || [ -z "$WORKER_PROVIDER" ] || [ -z "$REVIEWER_MODEL" ] || [ -z "$REVIEWER_PROVIDER" ]; then
    prompt_for_settings
fi

# Cost warning and confirmation loop
while true; do
    echo ""
    echo -e "${YELLOW}⚠️  Cost Warning:${NC} This will run up to ${MAX_ITERATIONS} iterations, each using both models."
    echo "    Estimated token usage could be significant depending on your task."
    echo ""
    echo -ne "Continue? [y/N]: "
    read -r confirm
    
    if [ "$confirm" = "y" ] || [ "$confirm" = "Y" ]; then
        break
    else
        echo ""
        prompt_for_settings
    fi
done

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
echo -e "${BLUE}  Ralph Loop - Multi-Model Edition${NC}"
echo -e "${BLUE}═══════════════════════════════════════════════════════════════${NC}"
echo ""
echo -e "  Task: ${YELLOW}$TASK${NC}"
echo -e "  Worker: ${WORKER_MODEL} (${WORKER_PROVIDER})"
echo -e "  Reviewer: ${REVIEWER_MODEL} (${REVIEWER_PROVIDER})"
echo -e "  Max Iterations: $MAX_ITERATIONS"
echo ""

for i in $(seq 1 "$MAX_ITERATIONS"); do
    echo -e "${BLUE}───────────────────────────────────────────────────────────────${NC}"
    echo -e "${BLUE}  Iteration $i / $MAX_ITERATIONS${NC}"
    echo -e "${BLUE}───────────────────────────────────────────────────────────────${NC}"
    
    echo "$i" > "$STATE_DIR/iteration.txt"
    
    echo ""
    echo -e "${YELLOW}▶ WORK PHASE${NC}"
    
    GOOSE_PROVIDER="$WORKER_PROVIDER" GOOSE_MODEL="$WORKER_MODEL" goose run --recipe "$RECIPE_DIR/ralph-work.yaml" || {
        echo -e "${RED}✗ WORK PHASE FAILED${NC}"
        exit 1
    }
    
    if [ -f "$STATE_DIR/RALPH-BLOCKED.md" ]; then
        echo ""
        echo -e "${RED}✗ BLOCKED${NC}"
        cat "$STATE_DIR/RALPH-BLOCKED.md"
        exit 1
    fi
    
    echo ""
    echo -e "${YELLOW}▶ REVIEW PHASE${NC}"
    
    GOOSE_PROVIDER="$REVIEWER_PROVIDER" GOOSE_MODEL="$REVIEWER_MODEL" goose run --recipe "$RECIPE_DIR/ralph-review.yaml" || {
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
