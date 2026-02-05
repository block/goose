#!/bin/bash
# Test local inference provider with tool calling
# Usage:
#   ./test_local_inference.sh              # Test all downloaded models
#   ./test_local_inference.sh llama-3.2-1b # Test specific model
#
# Environment variables:
#   SKIP_BUILD      Skip the cargo build step if set

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
DATA_DIR="${HOME}/.local/share/goose"
MODELS_DIR="${DATA_DIR}/models"

# All available local models
ALL_MODELS=(
  "llama-3.2-1b"
  "llama-3.2-3b"
  "hermes-2-pro-7b"
  "mistral-small-22b"
)

# If specific model requested, test only that one
if [ -n "$1" ]; then
  MODELS_TO_TEST=("$1")
else
  # Otherwise, detect which models are downloaded
  MODELS_TO_TEST=()
  for model in "${ALL_MODELS[@]}"; do
    model_file="${MODELS_DIR}/${model}.gguf"
    tokenizer_file="${MODELS_DIR}/${model}_tokenizer.json"
    if [ -f "$model_file" ] && [ -f "$tokenizer_file" ]; then
      MODELS_TO_TEST+=("$model")
    fi
  done
fi

if [ ${#MODELS_TO_TEST[@]} -eq 0 ]; then
  echo "❌ No local models found!"
  echo ""
  echo "To download models:"
  echo "  1. Start the desktop app: just ui-desktop"
  echo "  2. Go to Settings → Models → Local Inference Models"
  echo "  3. Download at least one model"
  echo ""
  echo "Or specify a model to test (will fail if not downloaded):"
  echo "  ./test_local_inference.sh llama-3.2-1b"
  exit 1
fi

echo "Testing local inference provider"
echo "Models to test: ${MODELS_TO_TEST[*]}"
echo ""

RESULTS=()
FAILURES=()

for MODEL in "${MODELS_TO_TEST[@]}"; do
  export GOOSE_PROVIDER="local"
  export GOOSE_MODEL="$MODEL"

  # Check if model files exist
  model_file="${MODELS_DIR}/${MODEL}.gguf"
  tokenizer_file="${MODELS_DIR}/${MODEL}_tokenizer.json"

  if [ ! -f "$model_file" ]; then
    echo "⊘ Skipping ${MODEL}: model file not found at ${model_file}"
    echo "---"
    continue
  fi

  if [ ! -f "$tokenizer_file" ]; then
    echo "⊘ Skipping ${MODEL}: tokenizer file not found at ${tokenizer_file}"
    echo "---"
    continue
  fi

  TESTDIR=$(mktemp -d)
  echo "hello world" > "$TESTDIR/hello.txt"
  echo "test file" > "$TESTDIR/test.txt"

  echo "Model: ${MODEL}"
  echo "Test directory: ${TESTDIR}"
  echo ""

  TMPFILE=$(mktemp)

  # Test tool calling with a simple ls command
  (cd "$TESTDIR" && timeout 120 "$SCRIPT_DIR/target/release/goose" run \
    --text "Use the shell tool to list files in the current directory with 'ls'. Do not ask for confirmation." \
    --with-builtin "developer" 2>&1) | tee "$TMPFILE"

  EXIT_CODE=$?
  echo ""

  # Check for success patterns
  # Look for shell tool being called or actual command execution
  # The output format shows code blocks with ls commands when shell tool is used
  if [ $EXIT_CODE -eq 124 ]; then
    echo "⏱️  TIMEOUT: Test timed out after 120 seconds"
    RESULTS+=("⏱️  ${MODEL} (timeout)")
    FAILURES+=("${MODEL} (timeout)")
  elif grep -qE "(shell \| developer)|(^\`\`\`$)" "$TMPFILE" && grep -q "ls" "$TMPFILE"; then
    echo "✓ SUCCESS: Tool calling works - shell tool called"
    RESULTS+=("✓ ${MODEL}")
  elif grep -qE "error|Error|ERROR|failed|Failed|FAILED" "$TMPFILE"; then
    echo "✗ FAILED: Errors detected in output"
    RESULTS+=("✗ ${MODEL} (error)")
    FAILURES+=("${MODEL} (error)")
  else
    echo "✗ FAILED: No tool calls detected"
    RESULTS+=("✗ ${MODEL} (no tool calls)")
    FAILURES+=("${MODEL} (no tool calls)")
  fi

  rm "$TMPFILE"
  rm -rf "$TESTDIR"
  echo "---"
done

echo ""
echo "=== Test Summary ==="
for result in "${RESULTS[@]}"; do
  echo "$result"
done

if [ ${#FAILURES[@]} -gt 0 ]; then
  echo ""
  echo "Failures (${#FAILURES[@]}):"
  for failure in "${FAILURES[@]}"; do
    echo "  - $failure"
  done
  echo ""
  echo "Some tests failed!"
  exit 1
else
  echo ""
  echo "All tests passed!"
fi
