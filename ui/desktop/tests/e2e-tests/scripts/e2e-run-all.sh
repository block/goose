#!/usr/bin/env bash
# Run all recorded e2e tests in parallel
# Usage: ./e2e-run-all.sh [--workers N] [--timeout SECONDS]
# Runs all *.batch.json files in recordings/, skipping files with "skip" in the name.
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
PROJECT_DIR="$(cd "$SCRIPT_DIR/../../.." && pwd)"
RECORDINGS_DIR="$SCRIPT_DIR/../recordings"
WORKERS=2
TIMEOUT=120  # seconds per test

# Parse args
while [[ $# -gt 0 ]]; do
  case "$1" in
    --workers) WORKERS="$2"; shift 2 ;;
    --timeout) TIMEOUT="$2"; shift 2 ;;
    *) echo "Unknown option: $1"; exit 1 ;;
  esac
done

# Collect recordings, excluding files with "skip" in the name
RECORDINGS=()
for f in "$RECORDINGS_DIR"/*.batch.json; do
  [[ "$(basename "$f")" == *skip* ]] && continue
  RECORDINGS+=("$f")
done

echo "=== E2E Test Runner ==="
echo "Recordings: ${#RECORDINGS[@]}, Workers: $WORKERS, Timeout: ${TIMEOUT}s"

# Generate API types once
echo ""
echo "Generating API types..."
cd "$PROJECT_DIR"
pnpm run generate-api

# Run a single recording: start app, replay, stop app
# Usage: run_one <recording> <result_dir>
run_one() {
  local RECORDING="$1"
  local RESULT_DIR="$2"
  local TEST_NAME
  TEST_NAME=$(basename "$RECORDING" .batch.json)

  echo "[$TEST_NAME] Starting app..."
  screen -dmS "$TEST_NAME" bash -c "source ~/.zshrc 2>/dev/null && bash '$SCRIPT_DIR/e2e-start.sh' '$TEST_NAME'" 2>/dev/null

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
    echo "[$TEST_NAME] FAIL — app did not start within 30s"
    echo "FAIL" > "$RESULT_DIR/$TEST_NAME"
    return
  fi
  echo "[$TEST_NAME] App ready: port=$CDP_PORT"

  if timeout "$TIMEOUT" bash "$SCRIPT_DIR/replay.sh" "$RECORDING" --connect "$CDP_PORT" --browser-session "$TEST_NAME"; then
    echo "PASS" > "$RESULT_DIR/$TEST_NAME"
    echo "[$TEST_NAME] PASS"
  else
    echo "FAIL" > "$RESULT_DIR/$TEST_NAME"
    echo "[$TEST_NAME] FAIL"
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
echo ""
echo "=== Results ==="
PASSED=0
FAILED=0
for RECORDING in "${RECORDINGS[@]}"; do
  TEST_NAME=$(basename "$RECORDING" .batch.json)
  RESULT=$(cat "$RESULT_DIR/$TEST_NAME" 2>/dev/null || echo "FAIL")
  echo "  $RESULT: $TEST_NAME"
  if [[ "$RESULT" == "PASS" ]]; then
    ((PASSED++))
  else
    ((FAILED++))
  fi
done
echo ""
echo "${#RECORDINGS[@]} tests, $PASSED passed, $FAILED failed"

exit "$FAILED"
