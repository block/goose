#!/usr/bin/env bash
# Run all recorded e2e tests in parallel
# Usage: ./e2e-run-all.sh [--workers N] [--timeout SECONDS]
# Runs all *.batch.json files in recordings/, skipping files with "skip" in the name (e.g., settings-dark-mode.skip.batch.json).
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
DESKTOP_DIR="$(cd "$SCRIPT_DIR/../../.." && pwd)"
PROJECT_DIR="$(cd "$DESKTOP_DIR/../.." && pwd)"
RECORDINGS_DIR="$SCRIPT_DIR/../recordings"
RESULTS_DIR="$SCRIPT_DIR/../results"

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
rm -rf "$RESULTS_DIR"
mkdir -p "$RESULTS_DIR/logs" "$RESULTS_DIR/screenshots"

echo "Installing agent-browser..."
cd "$DESKTOP_DIR"
pnpm exec agent-browser install

echo "=== E2E Test Runner ==="
echo "Recordings: ${#RECORDINGS[@]}, Workers: $WORKERS, Timeout: ${TIMEOUT}s"

cleanup_sessions() {
  bash "$SCRIPT_DIR/e2e-stop.sh" 2>/dev/null || true
}

# Wait for the app to write its port file and the port to be listening.
# Usage: wait_for_app <test-name>
# Prints the CDP port on success, returns 1 on timeout.
wait_for_app() {
  local TEST_NAME="$1"
  local PORT_FILE="/tmp/goose-e2e/$TEST_NAME/.port"
  for _ in $(seq 1 30); do
    sleep 1
    if [[ -f "$PORT_FILE" ]]; then
      local PORT
      PORT=$(cat "$PORT_FILE")
      if lsof -i :"$PORT" &>/dev/null; then
        echo "$PORT"
        return 0
      fi
    fi
  done
  return 1
}

# Run a single recording: start app, replay, stop app
# Usage: run_one <recording> <status_dir>
run_one() {
  local RECORDING="$1"
  local STATUS_DIR="$2"
  local TEST_NAME
  TEST_NAME=$(basename "$RECORDING" .batch.json)
  local START_TIME=$SECONDS
  local LOG_FILE="$RESULTS_DIR/logs/$TEST_NAME.log"
  local EXIT_CODE

  echo "[$(date '+%H:%M:%S')] [$TEST_NAME] Starting app..."
  screen -dmS "$TEST_NAME" bash -c "bash '$SCRIPT_DIR/e2e-start.sh' '$TEST_NAME'" 2>/dev/null

  local CDP_PORT
  if ! CDP_PORT=$(wait_for_app "$TEST_NAME"); then
    local DURATION=$(( SECONDS - START_TIME ))
    echo "[$(date '+%H:%M:%S')] [$TEST_NAME] FAIL — app did not start within 30s (${DURATION}s)"
    echo "FAIL ${DURATION}s" > "$STATUS_DIR/$TEST_NAME"
    return
  fi
  echo "[$(date '+%H:%M:%S')] [$TEST_NAME] App ready: port=$CDP_PORT"

  set +e
  timeout "$TIMEOUT" bash "$SCRIPT_DIR/replay.sh" \
    "$RECORDING" \
    --connect "$CDP_PORT" \
    --browser-session "$TEST_NAME" \
    --results-dir "$RESULTS_DIR" \
    2>&1 | tee "$LOG_FILE"
  EXIT_CODE=${PIPESTATUS[0]}
  set -e

  local DURATION=$(( SECONDS - START_TIME ))
  if [[ "$EXIT_CODE" -eq 0 ]]; then
    echo "PASS ${DURATION}s" > "$STATUS_DIR/$TEST_NAME"
    echo "[$(date '+%H:%M:%S')] [$TEST_NAME] PASS (${DURATION}s)"
  elif [[ "$EXIT_CODE" -eq 124 ]]; then
    echo "TIMEOUT ${DURATION}s" > "$STATUS_DIR/$TEST_NAME"
    echo "[$(date '+%H:%M:%S')] [$TEST_NAME] TIMEOUT (${DURATION}s)"
  else
    echo "FAIL ${DURATION}s" > "$STATUS_DIR/$TEST_NAME"
    echo "[$(date '+%H:%M:%S')] [$TEST_NAME] FAIL (${DURATION}s, exit=$EXIT_CODE)"
  fi

  bash "$SCRIPT_DIR/e2e-stop.sh" "$TEST_NAME" 2>/dev/null || true
}

export -f wait_for_app run_one
export SCRIPT_DIR TIMEOUT RESULTS_DIR

# Temp dir for pass/fail status
STATUS_DIR=$(mktemp -d)
cleanup_and_exit() {
  local exit_code="${1:-$?}"
  trap - EXIT INT TERM
  cleanup_sessions
  rm -rf "$STATUS_DIR"
  exit "$exit_code"
}

trap 'cleanup_and_exit $?' EXIT
trap 'echo ""; echo "Interrupted, stopping active E2E sessions..."; cleanup_and_exit 130' INT TERM

# Run recordings in parallel with worker limit
printf '%s\n' "${RECORDINGS[@]}" | xargs -P "$WORKERS" -I {} bash -c "run_one '{}' '$STATUS_DIR'" || true

# Print summary to console and write test-results.json
# Usage: write_results <status_dir> <results_dir> <recordings...>
write_results() {
  local STATUS_DIR="$1"
  local RESULTS_DIR="$2"
  shift 2

  mkdir -p "$RESULTS_DIR"

  local PASSED=0 FAILED=0
  local FAILURES_JSON="[]"

  echo ""
  echo "=== Results ==="
  for RECORDING in "$@"; do
    local TEST_NAME
    TEST_NAME=$(basename "$RECORDING" .batch.json)
    local RAW
    RAW=$(cat "$STATUS_DIR/$TEST_NAME" 2>/dev/null || echo "FAIL 0s")
    local STATUS DURATION
    STATUS=$(echo "$RAW" | awk '{print $1}')
    DURATION=$(echo "$RAW" | awk '{print $2}')
    echo "  $STATUS: $TEST_NAME ($DURATION)"
    if [[ "$STATUS" == "PASS" ]]; then
      PASSED=$((PASSED + 1))
    else
      FAILED=$((FAILED + 1))
      FAILURES_JSON=$(echo "$FAILURES_JSON" | jq \
        --arg test "$TEST_NAME" \
        --arg duration "$DURATION" \
        --arg log "results/logs/$TEST_NAME.log" \
        --arg screenshot "results/screenshots/$TEST_NAME.png" \
        '. + [{"test": $test, "duration": $duration, "log": $log, "screenshot": $screenshot}]')
    fi
  done

  local TOTAL=$#
  echo ""
  echo "$TOTAL tests, $PASSED passed, $FAILED failed (total: ${SECONDS}s)"

  jq -n \
    --arg run_at "$(date -u +%Y-%m-%dT%H:%M:%SZ)" \
    --argjson total "$TOTAL" \
    --argjson passed "$PASSED" \
    --argjson failed "$FAILED" \
    --argjson failures "$FAILURES_JSON" \
    '{run_at: $run_at, summary: {total: $total, passed: $passed, failed: $failed}, failures: $failures}' \
    > "$RESULTS_DIR/test-results.json"
  echo "Results written to $RESULTS_DIR/test-results.json"
}

write_results "$STATUS_DIR" "$RESULTS_DIR" "${RECORDINGS[@]}"

# Exit non-zero if any test failed
FAILED=$(jq '.summary.failed' "$RESULTS_DIR/test-results.json")
if [[ "$FAILED" -gt 0 ]]; then
  exit 1
fi
