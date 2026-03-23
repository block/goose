#!/usr/bin/env bash
# Stop a goose-under-test instance and clean up
# Usage: ./e2e-stop.sh <test-session-name>
set -euo pipefail

TEST_SESSION_NAME="${1:?Usage: ./e2e-stop.sh <test-session-name>}"
SESSION_DIR="/tmp/goose-e2e/$TEST_SESSION_NAME"

if [[ ! -d "$SESSION_DIR" ]]; then
  echo "Error: session not found: $SESSION_DIR" >&2
  exit 1
fi

CDP_PORT=$(cat "$SESSION_DIR/.port" 2>/dev/null || true)

# Kill everything listening on the CDP port (Electron + helpers)
if [[ -n "$CDP_PORT" ]]; then
  lsof -ti :"$CDP_PORT" 2>/dev/null | xargs kill -9 2>/dev/null || true
  pkill -9 -f "remote-debugging-port=$CDP_PORT" 2>/dev/null || true
fi

# Sweep anything else referencing the session dir
pkill -9 -f "$SESSION_DIR" 2>/dev/null || true

# Clean up
rm -rf "$SESSION_DIR"
echo "Test session $TEST_SESSION_NAME stopped and cleaned up."
