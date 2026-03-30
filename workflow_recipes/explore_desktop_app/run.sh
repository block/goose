#!/usr/bin/env bash
# Exploratory testing of the Goose desktop app
# Usage:
#   workflow_recipes/explore_desktop_app/run.sh "Test the settings page"
set -euo pipefail

GOAL="${1:?Usage: $0 \"<goal>\"}"
SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"

goose run --recipe "$SCRIPT_DIR/recipe.yaml" --params "goal=$GOAL" --interactive
