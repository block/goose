#!/bin/bash
if [ -f .env ]; then
  export $(grep -v '^#' .env | xargs)
fi

echo "Building goose..."
cargo build --release --bin goose
echo ""

SCRIPT_DIR=$(pwd)

PROVIDERS=(
  "openai:gpt-4o:gpt-4o-mini:gpt-3.5-turbo"
  "anthropic:claude-sonnet-4-0:claude-3-7-sonnet-latest"
  "google:gemini-2.5-pro:gemini-2.5-pro:gemini-2.5-flash"
  "databricks:databricks-claude-sonnet-4:gemini-2-5-flash:databricks-meta-llama-3-1-405b-instruct"
)

RESULTS=()

for provider_config in "${PROVIDERS[@]}"; do
  IFS=':' read -ra PARTS <<< "$provider_config"
  PROVIDER="${PARTS[0]}"
  for i in $(seq 1 $((${#PARTS[@]} - 1))); do
    MODEL="${PARTS[$i]}"
    export GOOSE_PROVIDER="$PROVIDER"
    export GOOSE_MODEL="$MODEL"
    TESTDIR=$(mktemp -d)
    echo "hello" > "$TESTDIR/hello.txt"
    echo "Provider: ${PROVIDER}"
    echo "Model: ${MODEL}"
    echo ""
    TMPFILE=$(mktemp)
    (cd "$TESTDIR" && "$SCRIPT_DIR/target/release/goose" run --text "please list files in the current directory" --with-builtin developer 2>&1) | tee "$TMPFILE"
    echo ""
    if grep -q "shell | developer" "$TMPFILE"; then
      echo "✓ SUCCESS: Test passed - developer tool called"
      RESULTS+=("✓ ${PROVIDER}/${MODEL}")
    else
      echo "✗ FAILED: Test failed - no developer tools called"
      RESULTS+=("✗ ${PROVIDER}/${MODEL}")
    fi
    rm "$TMPFILE"
    rm -rf "$TESTDIR"
    echo "---"
  done
done
echo ""
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
