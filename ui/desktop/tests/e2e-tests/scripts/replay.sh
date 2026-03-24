#!/usr/bin/env bash
# Replay an agent-browser batch recording
# Usage: ./replay.sh <recording.batch.json> [--connect <port>] [--browser-session <name>] [--results-dir <dir>] [--record]
set -euo pipefail

RECORDING="${1:?Usage: ./replay.sh <recording.batch.json> [--connect <port>] [--browser-session <name>] [--results-dir <dir>]}"
CONNECT_PORT=""
SESSION_NAME=""
RESULTS_DIR=""
RECORD=false

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
    --record) RECORD=true; shift ;;
    *) echo "Unknown option: $1"; exit 1 ;;
  esac
done

if [[ ! -f "$RECORDING" ]]; then
  echo "Error: Recording file not found: $RECORDING"
  exit 1
fi

if [[ -z "$SESSION_NAME" ]]; then
  SESSION_NAME=$(basename "$RECORDING" .batch.json)
fi

GLOBAL_ARGS=("--session" "$SESSION_NAME")

DEFAULT_TIMEOUT_MS="${AGENT_BROWSER_DEFAULT_TIMEOUT:-10000}"

ts() { date "+%H:%M:%S"; }

cleanup() {
  if [[ "$RECORD" == true ]]; then
    pnpm exec agent-browser "${GLOBAL_ARGS[@]}" record stop 2>/dev/null || true
  fi
  pnpm exec agent-browser "${GLOBAL_ARGS[@]}" close 2>/dev/null || true
}
trap cleanup EXIT

if [[ -n "$CONNECT_PORT" ]]; then
  MAX_CONNECT_RETRIES=10
  CONNECT_RETRY_DELAY=2
  for attempt in $(seq 1 "$MAX_CONNECT_RETRIES"); do
    echo "[$(ts)] Connecting to CDP port $CONNECT_PORT (attempt $attempt/$MAX_CONNECT_RETRIES)..."
    if pnpm exec agent-browser "${GLOBAL_ARGS[@]}" connect "$CONNECT_PORT" 2>&1; then
      break
    fi
    if [[ "$attempt" -eq "$MAX_CONNECT_RETRIES" ]]; then
      echo "[$(ts)] Failed to connect after $MAX_CONNECT_RETRIES attempts"
      exit 1
    fi
    echo "[$(ts)] Connect failed, retrying in ${CONNECT_RETRY_DELAY}s..."
    sleep "$CONNECT_RETRY_DELAY"
  done
fi

if [[ "$RECORD" == true && -n "$RESULTS_DIR" ]]; then
  VIDEO_DIR="$RESULTS_DIR/videos"
  mkdir -p "$VIDEO_DIR"
  VIDEO_PATH="$VIDEO_DIR/${SESSION_NAME}.webm"
  echo "[$(ts)] Recording video → $VIDEO_PATH"
  pnpm exec agent-browser "${GLOBAL_ARGS[@]}" record restart "$VIDEO_PATH" 2>/dev/null || echo "Video recording failed to start"
fi

TOTAL=$(jq length "$RECORDING")
echo "[$(ts)] Replaying $TOTAL commands from $RECORDING"
echo "[$(ts)] Using session: $SESSION_NAME"

for i in $(seq 0 $((TOTAL - 1))); do
  ARGS=()
  CMD_LENGTH=$(jq -r ".[$i] | length" "$RECORDING")
  for j in $(seq 0 $((CMD_LENGTH - 1))); do
    ARG=$(jq -r ".[$i][$j]" "$RECORDING")
    ARG="${ARG//\$PROJECT_DIR/$PROJECT_DIR}"
    ARGS+=("$ARG")
  done

  TIMEOUT_MS="$DEFAULT_TIMEOUT_MS"
  for k in $(seq 0 $((${#ARGS[@]} - 1))); do
    if [[ "${ARGS[$k]}" == "--timeout" && $((k + 1)) -lt ${#ARGS[@]} ]]; then
      TIMEOUT_MS="${ARGS[$((k + 1))]}"
      break
    fi
  done
  CMD_TIMEOUT=$(( TIMEOUT_MS / 1000 + 1 ))

  STEP=$((i + 1))
  echo "[$(ts)] [$STEP/$TOTAL] agent-browser ${GLOBAL_ARGS[*]} ${ARGS[*]}"
  if ! timeout "$CMD_TIMEOUT" pnpm exec agent-browser "${GLOBAL_ARGS[@]}" "${ARGS[@]}"; then
    echo ""
    echo "[$(ts)] FAILED at step $STEP/$TOTAL: ${ARGS[*]}"
    if [[ -n "$RESULTS_DIR" ]]; then
      SCREENSHOT_DIR="$RESULTS_DIR/screenshots"
      mkdir -p "$SCREENSHOT_DIR"
      echo "Capturing failure screenshot → $SCREENSHOT_DIR/${SESSION_NAME}.png"
      pnpm exec agent-browser "${GLOBAL_ARGS[@]}" screenshot "$SCREENSHOT_DIR/${SESSION_NAME}.png" 2>/dev/null || echo "Screenshot capture failed"
    fi
    exit 1
  fi
done

echo "[$(ts)] Replay complete: $TOTAL commands passed"
