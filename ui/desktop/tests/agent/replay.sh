#!/usr/bin/env bash
# Replay an agent-browser batch recording
# Usage: ./replay.sh <recording.batch.json> [--connect <port>] [--browser-session <name>]
set -euo pipefail

RECORDING="${1:?Usage: ./replay.sh <recording.batch.json> [--connect <port>] [--browser-session <name>]}"
CONNECT_PORT=""
SESSION_NAME=""

shift
while [[ $# -gt 0 ]]; do
  case "$1" in
    --connect) CONNECT_PORT="$2"; shift 2 ;;
    --browser-session) SESSION_NAME="$2"; shift 2 ;;
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
    ARGS+=("$ARG")
  done

  STEP=$((i + 1))
  echo "[$STEP/$TOTAL] agent-browser ${GLOBAL_ARGS[*]} ${ARGS[*]}"
  pnpm exec agent-browser "${GLOBAL_ARGS[@]}" "${ARGS[@]}"
done

echo "Replay complete: $TOTAL commands passed"
