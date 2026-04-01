#!/bin/bash
# Debug E2E test failures
# Usage:
#   workflow_recipes/debug_e2e_failures/run.sh
#   workflow_recipes/debug_e2e_failures/run.sh --test-name recipe-from-session
#   workflow_recipes/debug_e2e_failures/run.sh --results-url https://...
#   workflow_recipes/debug_e2e_failures/run.sh --test-name settings-dark-mode --results-url https://...
set -e

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"

PARAMS=()
while [[ $# -gt 0 ]]; do
  case "$1" in
    --test-name) PARAMS+=(--params "test_name=$2"); shift 2 ;;
    --results-url) PARAMS+=(--params "results_url=$2"); shift 2 ;;
    *) echo "Usage: $0 [--test-name <name>] [--results-url <url>]"; exit 1 ;;
  esac
done

goose run --recipe "$SCRIPT_DIR/recipe.yaml" "${PARAMS[@]}" --interactive
