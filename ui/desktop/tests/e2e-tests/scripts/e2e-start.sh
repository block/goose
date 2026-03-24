#!/usr/bin/env bash
# Start a fresh goose-under-test instance for e2e testing
# Usage: ./e2e-start.sh [test-session-name]
# Prints session name and CDP port on success
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
DESKTOP_DIR="$(cd "$SCRIPT_DIR/../../.." && pwd)"
PROJECT_DIR="$(cd "$DESKTOP_DIR/../.." && pwd)"
FIXTURES_DIR="$SCRIPT_DIR/../fixtures"
BASE_DIR="/tmp/goose-e2e"

source "$PROJECT_DIR/bin/activate-hermit"

cd "$DESKTOP_DIR"

TEST_SESSION_NAME="${1:-$(date +"%y%m%d-%H%M%S")}"
SESSION_DIR="$BASE_DIR/$TEST_SESSION_NAME"

# Pick an available port in range 9300-9399, using a lock file to prevent
# parallel instances from selecting the same port (TOCTOU race condition).
LOCK_DIR="$BASE_DIR/.port-locks"
mkdir -p "$LOCK_DIR"

pick_port() {
  for _ in $(seq 1 100); do
    PORT=$((9300 + RANDOM % 100))
    LOCK_FILE="$LOCK_DIR/$PORT"
    # Atomically create lock file — fails if another process already claimed this port
    if (set -C; echo $$ > "$LOCK_FILE") 2>/dev/null; then
      if ! lsof -i :"$PORT" &>/dev/null; then
        echo "$PORT"
        return 0
      fi
      rm -f "$LOCK_FILE"
    fi
  done
  echo "Error: could not find available port in 9300-9399" >&2
  return 1
}

CDP_PORT=$(pick_port)

# Create clean session directory
rm -rf "$SESSION_DIR"
mkdir -p "$SESSION_DIR/root"
mkdir -p "$SESSION_DIR/workspace"

cp "$FIXTURES_DIR/e2e-goosehints" "$SESSION_DIR/workspace/.goosehints"
# Write port to file
echo "$CDP_PORT" > "$SESSION_DIR/.port"

echo ""
echo "Test session name: $TEST_SESSION_NAME"
echo "CDP port: $CDP_PORT"
echo "Session dir: $SESSION_DIR"
echo ""

# Start the app in foreground (Ctrl+C to stop)
export GOOSE_ALLOWLIST_BYPASS=true
export GOOSE_DISABLE_KEYRING=1
export GOOSE_PATH_ROOT="$SESSION_DIR/root"
export GOOSE_WORKING_DIR="$SESSION_DIR/workspace"
export GOOSE_PROVIDER=anthropic
export GOOSE_MODEL=claude-haiku-4-5-20251001
export ANTHROPIC_API_KEY="${ANTHROPIC_API_KEY:?ANTHROPIC_API_KEY must be set}"
export GOOSE_TELEMETRY_ENABLED=false
export ENABLE_PLAYWRIGHT=true
export PLAYWRIGHT_DEBUG_PORT="$CDP_PORT"
exec pnpm exec electron-forge start
