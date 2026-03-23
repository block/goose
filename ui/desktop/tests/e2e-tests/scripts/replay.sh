#!/usr/bin/env bash
# Replay an agent-browser batch recording
# Usage: ./replay.sh <recording.batch.json> [--connect <port>] [--browser-session <name>] [--screenshot-on-fail]
set -uo pipefail

RECORDING="${1:?Usage: ./replay.sh <recording.batch.json> [--connect <port>] [--browser-session <name>]}"
CONNECT_PORT=""
SESSION_NAME=""
SCREENSHOT_ON_FAIL=false

# Resolve project root for $PROJECT_DIR substitution in recordings
SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
DESKTOP_DIR="$(cd "$SCRIPT_DIR/../../.." && pwd)"
PROJECT_DIR="${PROJECT_DIR:-$(cd "$DESKTOP_DIR/../.." && pwd)}"

# Activate hermit to get pnpm, node, etc. on PATH
source "$PROJECT_DIR/bin/activate-hermit"

# Must run from ui/desktop for pnpm exec agent-browser
cd "$DESKTOP_DIR"

shift
while [[ $# -gt 0 ]]; do
  case "$1" in
    --connect) CONNECT_PORT="$2"; shift 2 ;;
    --browser-session) SESSION_NAME="$2"; shift 2 ;;
    --screenshot-on-fail) SCREENSHOT_ON_FAIL=true; shift ;;
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

# Build global args that go before each command
GLOBAL_ARGS=("--session" "$SESSION_NAME")

# Per-command timeout in seconds (shell timeout as primary, --timeout as fallback)
CMD_TIMEOUT="${AGENT_BROWSER_CMD_TIMEOUT:-10}"
CMD_TIMEOUT_MS=$((CMD_TIMEOUT * 1000))

# Connect if port specified
if [[ -n "$CONNECT_PORT" ]]; then
  echo "Connecting to CDP port $CONNECT_PORT..."
  pnpm exec agent-browser "${GLOBAL_ARGS[@]}" connect "$CONNECT_PORT"
fi

# Read JSON array and execute each command
TOTAL=$(jq length "$RECORDING")
echo "Replaying $TOTAL commands from $RECORDING"
[[ -n "$SESSION_NAME" ]] && echo "Using session: $SESSION_NAME"

for i in $(seq 0 $((TOTAL - 1))); do
  # Extract command args as a bash array
  ARGS=()
  CMD_LENGTH=$(jq -r ".[$i] | length" "$RECORDING")
  for j in $(seq 0 $((CMD_LENGTH - 1))); do
    ARG=$(jq -r ".[$i][$j]" "$RECORDING")
    # Substitute $PROJECT_DIR with the actual project root
    ARG="${ARG//\$PROJECT_DIR/$PROJECT_DIR}"
    ARGS+=("$ARG")
  done

  STEP=$((i + 1))
  echo "[$STEP/$TOTAL] agent-browser ${GLOBAL_ARGS[*]} ${ARGS[*]} --timeout $CMD_TIMEOUT_MS"
  if ! timeout "$CMD_TIMEOUT" pnpm exec agent-browser "${GLOBAL_ARGS[@]}" "${ARGS[@]}" "--timeout" "$CMD_TIMEOUT_MS"; then
    echo ""
    echo "FAILED at step $STEP/$TOTAL: ${ARGS[*]}"
    if [[ "$SCREENSHOT_ON_FAIL" == "true" ]]; then
      SCREENSHOT_DIR="$SCRIPT_DIR/../screenshots"
      mkdir -p "$SCREENSHOT_DIR"
      SCREENSHOT_PATH="$SCREENSHOT_DIR/${SESSION_NAME}.png"
      echo "Capturing failure screenshot → $SCREENSHOT_PATH"
      pnpm exec agent-browser "${GLOBAL_ARGS[@]}" screenshot "$SCREENSHOT_PATH" 2>/dev/null || echo "Screenshot capture failed"
    fi
    exit 1
  fi
done

echo "Replay complete: $TOTAL commands passed"

# Close the agent-browser session to release the CDP connection
pnpm exec agent-browser "${GLOBAL_ARGS[@]}" close 2>/dev/null || true
