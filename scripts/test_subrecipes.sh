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
# Use fast model for CI to speed up tests
export GOOSE_PROVIDER="${GOOSE_PROVIDER:-anthropic}"
export GOOSE_MODEL="${GOOSE_MODEL:-claude-3-5-haiku-20241022}"

echo "Using provider: $GOOSE_PROVIDER"
echo "Using model: $GOOSE_MODEL"
echo ""

TESTDIR=$(mktemp -d)
echo "Created test directory: $TESTDIR"

cp -r "$SCRIPT_DIR/scripts/test-subrecipes-examples/"* "$TESTDIR/"
echo "Copied test recipes from scripts/test-subrecipes-examples"

echo ""
echo "=== Testing Subrecipe Workflow ==="
echo "Recipe: $TESTDIR/project_analyzer.yaml"
echo ""

# Create sample code files for analysis
echo "Creating sample code files for testing..."
cat > "$TESTDIR/sample.rs" << 'EOF'
// TODO: Add error handling
fn calculate(x: i32, y: i32) -> i32 {
    x + y
}

#[test]
fn test_calculate() {
    assert_eq!(calculate(2, 2), 4);
}
EOF

cat > "$TESTDIR/sample.py" << 'EOF'
# FIXME: Optimize this function
def process_data(items):
    """Process a list of items"""
    return [item * 2 for item in items]

def test_process_data():
    assert process_data([1, 2, 3]) == [2, 4, 6]
EOF

cat > "$TESTDIR/README.md" << 'EOF'
# Sample Project
This is a test project for analyzing code patterns.
## TODO
- Add more tests
EOF
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
  
  if grep -q "file_stats" "$tmpfile" && grep -q "code_patterns" "$tmpfile"; then
    echo "✓ SUCCESS: Both subrecipes (file_stats, code_patterns) found in output"
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

echo "Running recipe with parallel subrecipes..."
TMPFILE=$(mktemp)
if (cd "$TESTDIR" && "$SCRIPT_DIR/target/release/goose" run --recipe project_analyzer_parallel.yaml --no-session 2>&1) | tee "$TMPFILE"; then
  echo "✓ SUCCESS: Recipe completed successfully"
  RESULTS+=("✓ Recipe exit code")
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
  RESULTS+=("✗ Recipe exit code")
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
