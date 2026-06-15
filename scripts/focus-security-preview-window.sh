#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
ATTEMPTS="${SECURITY_PREVIEW_FOCUS_ATTEMPTS:-1}"
DELAY_SECONDS="${SECURITY_PREVIEW_FOCUS_DELAY_SECONDS:-1}"
FOCUS_HELPER="$ROOT_DIR/scripts/focus-security-preview-window.js"

if [[ ! -f "$FOCUS_HELPER" ]]; then
  exit 0
fi

for ((attempt = 1; attempt <= ATTEMPTS; attempt += 1)); do
  GOOSE_PREVIEW_REPO_ROOT="${GOOSE_PREVIEW_REPO_ROOT:-$ROOT_DIR}" \
    node "$FOCUS_HELPER" >/dev/null 2>&1 || true

  if [[ "$attempt" -lt "$ATTEMPTS" ]]; then
    sleep "$DELAY_SECONDS"
  fi
done
