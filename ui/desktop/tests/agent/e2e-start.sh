#!/usr/bin/env bash
# Start a fresh goose-under-test instance for e2e testing
# Usage: ./e2e-start.sh
# Prints session ID and CDP port on success
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
PROJECT_DIR="$(cd "$SCRIPT_DIR/../.." && pwd)"
FIXTURES_DIR="$SCRIPT_DIR/fixtures"
BASE_DIR="/tmp/goose-e2e"

SESSION_ID=$(date +"%y%m%d-%H%M%S")
SESSION_DIR="$BASE_DIR/$SESSION_ID"

# Pick a random available port in range 9300-9399
pick_port() {
  for _ in $(seq 1 20); do
    PORT=$((9300 + RANDOM % 100))
    if ! lsof -i :"$PORT" &>/dev/null; then
      echo "$PORT"
      return 0
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

# Generate API types
cd "$PROJECT_DIR"
pnpm run generate-api

echo ""
echo "Session: $SESSION_ID"
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
