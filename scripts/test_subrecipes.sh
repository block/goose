#!/bin/bash
set -e

if [ -f .env ]; then
  export $(grep -v '^#' .env | xargs)
fi

if [ -z "$SKIP_BUILD" ]; then
  echo "Building goose..."
  cargo build --release --bin goose
  echo ""
else
  echo "Skipping build (SKIP_BUILD is set)..."
  echo ""
fi

SCRIPT_DIR=$(pwd)

# Add goose binary to PATH so subagents can find it when spawning
export PATH="$SCRIPT_DIR/target/release:$PATH"

# Set default provider and model if not already set
export GOOSE_PROVIDER="${GOOSE_PROVIDER:-anthropic}"
export GOOSE_MODEL="${GOOSE_MODEL:-claude-sonnet-4-5-20250929}"

echo "Using provider: $GOOSE_PROVIDER"
echo "Using model: $GOOSE_MODEL"
echo ""

TESTDIR=$(mktemp -d)
echo "Created test directory: $TESTDIR"

cp -r "$SCRIPT_DIR/scripts/test-subrecipes-examples/"* "$TESTDIR/"
echo "Copied test recipes from scripts/test-subrecipes-examples"

echo ""
echo "=== Testing Subrecipe Workflow ==="
echo "Recipe: $TESTDIR/travel_planner.yaml"
echo ""

RESULTS=()

check_recipe_output() {
  local tmpfile=$1
  local mode=$2
  
  if grep -q "| subrecipe" "$tmpfile"; then
    echo "✓ SUCCESS: Subrecipe tools invoked"
    RESULTS+=("✓ Subrecipe tool invocation ($mode)")
  else
    echo "✗ FAILED: No evidence of subrecipe tool invocation"
    RESULTS+=("✗ Subrecipe tool invocation ($mode)")
  fi
  
  if grep -q "weather_data" "$tmpfile" && grep -q "activity_suggestions" "$tmpfile"; then
    echo "✓ SUCCESS: Both subrecipes (weather_data, activity_suggestions) found in output"
    RESULTS+=("✓ Both subrecipes present ($mode)")
  else
    echo "✗ FAILED: Not all subrecipes found in output"
    RESULTS+=("✗ Subrecipe names ($mode)")
  fi
  
  if grep -q "| subagent" "$tmpfile"; then
    echo "✓ SUCCESS: Subagent execution detected"
    RESULTS+=("✓ Subagent execution ($mode)")
  else
    echo "✗ FAILED: No evidence of subagent execution"
    RESULTS+=("✗ Subagent execution ($mode)")
  fi
}

echo "Test 1: Running recipe with session..."
TMPFILE=$(mktemp)
if (cd "$TESTDIR" && "$SCRIPT_DIR/target/release/goose" run --recipe travel_planner.yaml 2>&1) | tee "$TMPFILE"; then
  echo "✓ SUCCESS: Recipe completed successfully"
  RESULTS+=("✓ Recipe exit code (with session)")
  check_recipe_output "$TMPFILE" "with session"
else
  echo "✗ FAILED: Recipe execution failed"
  RESULTS+=("✗ Recipe exit code (with session)")
fi
rm "$TMPFILE"
echo ""

echo "Test 2: Running recipe in --no-session mode..."
TMPFILE=$(mktemp)
if (cd "$TESTDIR" && "$SCRIPT_DIR/target/release/goose" run --recipe travel_planner.yaml --no-session 2>&1) | tee "$TMPFILE"; then
  echo "✓ SUCCESS: Recipe completed successfully"
  RESULTS+=("✓ Recipe exit code (--no-session)")
  check_recipe_output "$TMPFILE" "--no-session"
else
  echo "✗ FAILED: Recipe execution failed"
  RESULTS+=("✗ Recipe exit code (--no-session)")
fi
rm "$TMPFILE"
echo ""

echo "Test 3: Running recipe with parallel subrecipes..."
TMPFILE=$(mktemp)
if (cd "$TESTDIR" && "$SCRIPT_DIR/target/release/goose" run --recipe travel_planner_parallel.yaml 2>&1) | tee "$TMPFILE"; then
  echo "✓ SUCCESS: Recipe completed successfully"
  RESULTS+=("✓ Recipe exit code (parallel)")
  check_recipe_output "$TMPFILE" "parallel"
  
  if grep -q "execution_mode: parallel" "$TMPFILE"; then
    echo "✓ SUCCESS: Parallel execution mode detected"
    RESULTS+=("✓ Parallel execution mode")
  else
    echo "✗ FAILED: Parallel execution mode not detected"
    RESULTS+=("✗ Parallel execution mode")
  fi
else
  echo "✗ FAILED: Recipe execution failed"
  RESULTS+=("✗ Recipe exit code (parallel)")
fi
rm "$TMPFILE"
echo ""

rm -rf "$TESTDIR"

echo "=== Test Summary ==="
for result in "${RESULTS[@]}"; do
  echo "$result"
done

if echo "${RESULTS[@]}" | grep -q "✗"; then
  echo ""
  echo "Some tests failed!"
  exit 1
else
  echo ""
  echo "All tests passed!"
fi
