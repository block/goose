#!/usr/bin/env bash
# Stop a goose-under-test instance and clean up
# Usage: ./e2e-stop.sh [test-session-name]
set -euo pipefail

BASE_DIR="/tmp/goose-e2e"
SESSIONS_DIR="$BASE_DIR/sessions"

stop_session() {
  local TEST_SESSION_NAME="$1"
  local SESSION_DIR="$SESSIONS_DIR/$TEST_SESSION_NAME"

  if [[ ! -d "$SESSION_DIR" ]]; then
    echo "Error: session not found: $SESSION_DIR" >&2
    return 1
  fi

  # Kill the screen session (takes Electron + goosed with it)
  screen -S "$TEST_SESSION_NAME" -X quit 2>/dev/null || true

  local CDP_PORT
  CDP_PORT=$(cat "$SESSION_DIR/.port" 2>/dev/null || true)

  if [[ -n "$CDP_PORT" ]]; then
    lsof -ti :"$CDP_PORT" 2>/dev/null | xargs kill -9 2>/dev/null || true
    pkill -9 -f "remote-debugging-port=$CDP_PORT" 2>/dev/null || true
    # Release port lock
    rm -f "$BASE_DIR/.port-locks/$CDP_PORT"
  fi

  pkill -9 -f "$SESSION_DIR" 2>/dev/null || true

  rm -rf "$SESSION_DIR"
  echo "Test session $TEST_SESSION_NAME stopped and cleaned up."
}

if [[ $# -gt 0 ]]; then
  stop_session "$1"
  exit 0
fi

if [[ -d "$SESSIONS_DIR" ]]; then
  for session_dir in "$SESSIONS_DIR"/*; do
  [[ -d "$session_dir" ]] || continue
  stop_session "$(basename "$session_dir")" || true
done

pkill -9 -f "$BASE_DIR" 2>/dev/null || true
pkill -9 -f 'agent-browser-chrome' 2>/dev/null || true
pkill -9 -f 'agent-browser-darwin' 2>/dev/null || true
pkill -9 -f 'agent-browser-linux' 2>/dev/null || true
