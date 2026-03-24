#!/usr/bin/env bash
# Run all recorded e2e tests in parallel
# Usage: ./e2e-run-all.sh [--workers N] [--timeout SECONDS]
# Runs all *.batch.json files in recordings/, skipping files with "skip" in the name (e.g., settings-dark-mode.skip.batch.json).
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
DESKTOP_DIR="$(cd "$SCRIPT_DIR/../../.." && pwd)"
PROJECT_DIR="$(cd "$DESKTOP_DIR/../.." && pwd)"
RECORDINGS_DIR="$SCRIPT_DIR/../recordings"

source "$PROJECT_DIR/bin/activate-hermit"
WORKERS=4
TIMEOUT=120  # seconds per test
FILTER=""

# Parse args
while [[ $# -gt 0 ]]; do
  case "$1" in
    --workers) WORKERS="$2"; shift 2 ;;
    --timeout) TIMEOUT="$2"; shift 2 ;;
    --only) FILTER="$2"; shift 2 ;;
    *) echo "Unknown option: $1"; exit 1 ;;
  esac
done

# Collect recordings, excluding *.skip.batch.json files
RECORDINGS=()
for f in "$RECORDINGS_DIR"/*.batch.json; do
  [[ "$(basename "$f")" == *.skip.batch.json ]] && continue
  [[ -n "$FILTER" && "$(basename "$f")" != $FILTER*.batch.json ]] && continue
  RECORDINGS+=("$f")
done

if [[ ${#RECORDINGS[@]} -eq 0 ]]; then
  echo "No recordings matched${FILTER:+ (filter: $FILTER)}"
  exit 0
fi

# Clean up previous test sessions
pkill -9 -f "/tmp/goose-e2e" 2>/dev/null || true
rm -rf /tmp/goose-e2e

export AGENT_BROWSER_DEFAULT_TIMEOUT=10000

echo "Installing agent-browser..."
cd "$DESKTOP_DIR"
pnpm exec agent-browser install

echo "=== E2E Test Runner ==="
echo "Recordings: ${#RECORDINGS[@]}, Workers: $WORKERS, Timeout: ${TIMEOUT}s"

# Run a single recording: start app, replay, stop app
# Usage: run_one <recording> <result_dir>
run_one() {
  set -o pipefail
  local RECORDING="$1"
  local RESULT_DIR="$2"
  local TEST_NAME
  TEST_NAME=$(basename "$RECORDING" .batch.json)
  local START_TIME=$SECONDS

  echo "[$TEST_NAME] Starting app..."
  screen -dmS "$TEST_NAME" bash -c "bash '$SCRIPT_DIR/e2e-start.sh' '$TEST_NAME'" 2>/dev/null

  # Wait for the app to write its port file
  local CDP_PORT=""
  for _ in $(seq 1 30); do
    sleep 1
    if [[ -f "/tmp/goose-e2e/$TEST_NAME/.port" ]]; then
      CDP_PORT=$(cat "/tmp/goose-e2e/$TEST_NAME/.port")
      if lsof -i :"$CDP_PORT" &>/dev/null; then
        break
      fi
      CDP_PORT=""
    fi
  done

  if [[ -z "$CDP_PORT" ]]; then
    local DURATION=$(( SECONDS - START_TIME ))
    echo "[$TEST_NAME] FAIL — app did not start within 30s (${DURATION}s)"
    echo "FAIL ${DURATION}s" > "$RESULT_DIR/$TEST_NAME"
    return
  fi
  echo "[$TEST_NAME] App ready: port=$CDP_PORT"

  local LOG_DIR="$SCRIPT_DIR/../logs"
  mkdir -p "$LOG_DIR"
  local LOG_FILE="$LOG_DIR/$TEST_NAME.log"

  if timeout "$TIMEOUT" bash "$SCRIPT_DIR/replay.sh" "$RECORDING" --connect "$CDP_PORT" --browser-session "$TEST_NAME" --screenshot-on-fail 2>&1 | tee "$LOG_FILE"; then
    local DURATION=$(( SECONDS - START_TIME ))
    echo "PASS ${DURATION}s" > "$RESULT_DIR/$TEST_NAME"
    echo "[$TEST_NAME] PASS (${DURATION}s)"
  else
    local DURATION=$(( SECONDS - START_TIME ))
    echo "FAIL ${DURATION}s" > "$RESULT_DIR/$TEST_NAME"
    echo "[$TEST_NAME] FAIL (${DURATION}s)"
  fi

  bash "$SCRIPT_DIR/e2e-stop.sh" "$TEST_NAME" 2>/dev/null || true
}

export -f run_one
export SCRIPT_DIR TIMEOUT

# Temp dir for results
RESULT_DIR=$(mktemp -d)
trap 'rm -rf "$RESULT_DIR"' EXIT

# Run recordings in parallel with worker limit
printf '%s\n' "${RECORDINGS[@]}" | xargs -P "$WORKERS" -I {} bash -c "run_one '{}' '$RESULT_DIR'"

# Summary
TOTAL_TIME=$SECONDS
echo ""
echo "=== Results ==="
PASSED=0
FAILED=0
for RECORDING in "${RECORDINGS[@]}"; do
  TEST_NAME=$(basename "$RECORDING" .batch.json)
  RAW=$(cat "$RESULT_DIR/$TEST_NAME" 2>/dev/null || echo "FAIL 0s")
  STATUS=$(echo "$RAW" | awk '{print $1}')
  DURATION=$(echo "$RAW" | awk '{print $2}')
  echo "  $STATUS: $TEST_NAME ($DURATION)"
  if [[ "$STATUS" == "PASS" ]]; then
    ((PASSED++))
  else
    ((FAILED++))
  fi
done
echo ""
echo "${#RECORDINGS[@]} tests, $PASSED passed, $FAILED failed (total: ${TOTAL_TIME}s)"

exit "$FAILED"
