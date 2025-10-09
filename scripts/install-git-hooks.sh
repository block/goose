#!/usr/bin/env bash
# Script to install repository git hooks from .githooks/
set -euo pipefail

HOOK_DIR=".git/hooks"
SRC_DIR=".githooks"

if [[ ! -d "$SRC_DIR" ]]; then
  echo "No $SRC_DIR directory found."
  exit 1
fi

mkdir -p "$HOOK_DIR"

for hook in "prepare-commit-msg" "commit-msg" "pre-commit"; do
  if [[ -f "$SRC_DIR/$hook" ]]; then
    cp "$SRC_DIR/$hook" "$HOOK_DIR/$hook"
    chmod +x "$HOOK_DIR/$hook"
    echo "Installed $hook"
  fi
done

echo "Git hooks installed."
