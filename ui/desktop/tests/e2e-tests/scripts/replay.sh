#!/usr/bin/env bash
# Replay an agent-browser batch recording
# Usage: ./replay.sh <recording.batch.json> [--connect <port>] [--browser-session <name>] [--results-dir <dir>]
set -uo pipefail

RECORDING="${1:?Usage: ./replay.sh <recording.batch.json> [--connect <port>] [--browser-session <name>] [--results-dir <dir>]}"
CONNECT_PORT=""
SESSION_NAME=""
RESULTS_DIR=""

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
DESKTOP_DIR="$(cd "$SCRIPT_DIR/../../.." && pwd)"
PROJECT_DIR="${PROJECT_DIR:-$(cd "$DESKTOP_DIR/../.." && pwd)}"

source "$PROJECT_DIR/bin/activate-hermit"

cd "$DESKTOP_DIR"

shift
while [[ $# -gt 0 ]]; do
  case "$1" in
    --connect) CONNECT_PORT="$2"; shift 2 ;;
    --browser-session) SESSION_NAME="$2"; shift 2 ;;
    --results-dir) RESULTS_DIR="$2"; shift 2 ;;
    *) echo "Unknown option: $1"; exit 1 ;;
  esac
done

if [[ ! -f "$RECORDING" ]]; then
  echo "Error: Recording file not found: $RECORDING"
  exit 1
fi

# Default session name from recording filename (e.g., settings-dark-mode.batch.json -> settings-dark-mode)
if [[ -z "$SESSION_NAME" ]]; then
  SESSION_NAME=$(basename "$RECORDING" .batch.json)
fi

# Set up logging: tee all output to both console and log file
if [[ -n "$RESULTS_DIR" ]]; then
  mkdir -p "$RESULTS_DIR/logs"
  LOG_FILE="$RESULTS_DIR/logs/$SESSION_NAME.log"
  exec > >(tee "$LOG_FILE") 2>&1
  trap 'wait' EXIT
fi

# Build global args that go before each command
GLOBAL_ARGS=("--session" "$SESSION_NAME")

# Per-command timeout in seconds (shell timeout as primary, --timeout as fallback)
export AGENT_BROWSER_DEFAULT_TIMEOUT="${AGENT_BROWSER_DEFAULT_TIMEOUT:-10000}"
CMD_TIMEOUT=$(( AGENT_BROWSER_DEFAULT_TIMEOUT / 1000 + 1 ))

# Connect if port specified, with retries to wait for the renderer to be ready.
# The CDP port may be listening before the Electron BrowserWindow is fully
# initialized, causing "Target.createTarget: Not supported" errors.
if [[ -n "$CONNECT_PORT" ]]; then
  MAX_CONNECT_RETRIES=10
  CONNECT_RETRY_DELAY=2
  for attempt in $(seq 1 "$MAX_CONNECT_RETRIES"); do
    echo "Connecting to CDP port $CONNECT_PORT (attempt $attempt/$MAX_CONNECT_RETRIES)..."
    if pnpm exec agent-browser "${GLOBAL_ARGS[@]}" connect "$CONNECT_PORT" 2>&1; then
      break
    fi
    if [[ "$attempt" -eq "$MAX_CONNECT_RETRIES" ]]; then
      echo "Failed to connect after $MAX_CONNECT_RETRIES attempts"
      exit 1
    fi
    echo "Connect failed, retrying in ${CONNECT_RETRY_DELAY}s..."
    sleep "$CONNECT_RETRY_DELAY"
  done
fi

TOTAL=$(jq length "$RECORDING")
echo "Replaying $TOTAL commands from $RECORDING"
echo "Using session: $SESSION_NAME"

for i in $(seq 0 $((TOTAL - 1))); do
  ARGS=()
  CMD_LENGTH=$(jq -r ".[$i] | length" "$RECORDING")
  for j in $(seq 0 $((CMD_LENGTH - 1))); do
    ARG=$(jq -r ".[$i][$j]" "$RECORDING")
    ARG="${ARG//\$PROJECT_DIR/$PROJECT_DIR}"
    ARGS+=("$ARG")
  done

  STEP=$((i + 1))
  echo "[$STEP/$TOTAL] agent-browser ${GLOBAL_ARGS[*]} ${ARGS[*]}"
  if ! timeout "$CMD_TIMEOUT" pnpm exec agent-browser "${GLOBAL_ARGS[@]}" "${ARGS[@]}"; then
    echo ""
    echo "FAILED at step $STEP/$TOTAL: ${ARGS[*]}"
    if [[ -n "$RESULTS_DIR" ]]; then
      SCREENSHOT_DIR="$RESULTS_DIR/screenshots"
      mkdir -p "$SCREENSHOT_DIR"
      SCREENSHOT_PATH="$SCREENSHOT_DIR/${SESSION_NAME}.png"
      echo "Capturing failure screenshot → $SCREENSHOT_PATH"
      pnpm exec agent-browser "${GLOBAL_ARGS[@]}" screenshot "$SCREENSHOT_PATH" 2>/dev/null || echo "Screenshot capture failed"
    fi
    exit 1
  fi
done

echo "Replay complete: $TOTAL commands passed"

pnpm exec agent-browser "${GLOBAL_ARGS[@]}" close 2>/dev/null || true
