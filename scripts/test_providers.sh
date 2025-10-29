#!/bin/bash
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

PROVIDERS=(
  "openrouter:google/gemini-2.5-pro:google/gemini-2.5-flash:anthropic/claude-sonnet-4.5:qwen/qwen3-coder"
  "openai:gpt-4o:gpt-4o-mini:gpt-3.5-turbo:gpt-5"
  "anthropic:claude-sonnet-4-5-20250929:claude-opus-4-1-20250805"
  "google:gemini-2.5-pro:gemini-2.5-flash"
  "tetrate:claude-sonnet-4-20250514"
)

# In CI, only run Databricks tests if DATABRICKS_HOST and DATABRICKS_TOKEN are set
# Locally, always run Databricks tests
if [ -n "$CI" ]; then
  if [ -n "$DATABRICKS_HOST" ] && [ -n "$DATABRICKS_TOKEN" ]; then
    echo "✓ Including Databricks tests"
    PROVIDERS+=("databricks:databricks-claude-sonnet-4:gemini-2-5-flash:gpt-4o")
  else
    echo "⚠️  Skipping Databricks tests (DATABRICKS_HOST and DATABRICKS_TOKEN required in CI)"
  fi
else
  echo "✓ Including Databricks tests"
  PROVIDERS+=("databricks:databricks-claude-sonnet-4:gemini-2-5-flash:gpt-4o")
fi

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

    echo "Test 1: Basic tool calling..."
    TMPFILE=$(mktemp)
    (cd "$TESTDIR" && "$SCRIPT_DIR/target/release/goose" run --text "please list files in the current directory" --with-builtin developer,autovisualiser,computercontroller,tutorial  2>&1) | tee "$TMPFILE"
    echo ""
    if grep -q "shell | developer" "$TMPFILE"; then
      echo "✓ SUCCESS: Basic tool calling - developer tool called"
      RESULTS+=("✓ ${PROVIDER}: ${MODEL} - Basic tool calling")
    else
      echo "✗ FAILED: Basic tool calling - no developer tools called"
      RESULTS+=("✗ ${PROVIDER}: ${MODEL} - Basic tool calling")
    fi
    rm "$TMPFILE"

    echo "Test 2: MCP Sampling..."
    TMPFILE=$(mktemp)
    (cd "$TESTDIR" && "$SCRIPT_DIR/target/release/goose" run --text "use the everything extension to call the sampleLLM tool with a prompt asking for a famous quote" --with-extension npx:-y:@modelcontextprotocol/server-everything 2>&1) | tee "$TMPFILE"
    echo ""

    if grep -q "sampleLLM | everything" "$TMPFILE"; then
      echo "✓ SUCCESS: MCP Sampling - sampleLLM tool called"
      RESULTS+=("✓ ${PROVIDER}: ${MODEL} - MCP Sampling (tool called)")
    else
      echo "✗ FAILED: MCP Sampling - sampleLLM tool not called"
      RESULTS+=("✗ ${PROVIDER}: ${MODEL} - MCP Sampling (tool called)")
    fi

    if grep -q "LLM sampling result" "$TMPFILE"; then
      echo "✓ SUCCESS: MCP Sampling - sampling completed with result"
      RESULTS+=("✓ ${PROVIDER}: ${MODEL} - MCP Sampling (result)")
    else
      echo "✗ FAILED: MCP Sampling - no sampling result found"
      RESULTS+=("✗ ${PROVIDER}: ${MODEL} - MCP Sampling (result)")
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
