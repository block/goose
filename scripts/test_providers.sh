#!/bin/bash
# Test providers with optional code_execution mode
# Usage:
#   ./test_providers.sh              # Normal mode (direct tool calls)
#   ./test_providers.sh --code-exec  # Code execution mode (JS batching)
#
# Environment variables:
#   SKIP_PROVIDERS  Comma-separated list of providers to skip (e.g., "tetrate,xai")
#   SKIP_BUILD      Skip the cargo build step if set

CODE_EXEC_MODE=false
for arg in "$@"; do
  case $arg in
    --code-exec)
      CODE_EXEC_MODE=true
      ;;
  esac
done

# Flaky models that are allowed to fail without failing the entire test run.
# These are typically preview/experimental models with inconsistent tool-calling behavior.
# Failures are still reported but don't block PRs.
ALLOWED_FAILURES=(
  "google:gemini-3-pro-preview"
  "openrouter:nvidia/nemotron-3-nano-30b-a3b"
)

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

# Format: "provider -> model1|model2|model3"
PROVIDERS=(
  "openrouter -> google/gemini-2.5-pro|anthropic/claude-sonnet-4.5|qwen/qwen3-coder:exacto|z-ai/glm-4.6:exacto|nvidia/nemotron-3-nano-30b-a3b"
  "xai -> grok-3"
  "openai -> gpt-4o|gpt-4o-mini|gpt-3.5-turbo|gpt-5"
  "anthropic -> claude-sonnet-4-5-20250929|claude-opus-4-1-20250805"
  "google -> gemini-2.5-pro|gemini-2.5-flash|gemini-3-pro-preview|gemini-3-flash-preview"
  "tetrate -> claude-sonnet-4-20250514"
)

# In CI, only run Databricks tests if DATABRICKS_HOST and DATABRICKS_TOKEN are set
# Locally, always run Databricks tests
if [ -n "$CI" ]; then
  if [ -n "$DATABRICKS_HOST" ] && [ -n "$DATABRICKS_TOKEN" ]; then
    echo "✓ Including Databricks tests"
    PROVIDERS+=("databricks -> databricks-claude-sonnet-4|gemini-2-5-flash|gpt-4o")
  else
    echo "⚠️  Skipping Databricks tests (DATABRICKS_HOST and DATABRICKS_TOKEN required in CI)"
  fi
else
  echo "✓ Including Databricks tests"
  PROVIDERS+=("databricks -> databricks-claude-sonnet-4|gemini-2-5-flash|gpt-4o")
fi

# Configure mode-specific settings
if [ "$CODE_EXEC_MODE" = true ]; then
  echo "Mode: code_execution (JS batching)"
  BUILTINS="developer,code_execution"
  # Match code_execution tool usage:
  # - "execute | code_execution" or "get_function_details | code_execution" (fallback format)
  # - "tool call | execute" or "tool calls | execute" (new format with tool_graph)
  SUCCESS_PATTERN="(execute \| code_execution)|(get_function_details \| code_execution)|(tool calls? \| execute)"
  SUCCESS_MSG="code_execution tool called"
  FAILURE_MSG="no code_execution tools called"
else
  echo "Mode: normal (direct tool calls)"
  BUILTINS="developer,autovisualiser,computercontroller,tutorial,todo,extensionmanager"
  SUCCESS_PATTERN="shell \| developer"
  SUCCESS_MSG="developer tool called"
  FAILURE_MSG="no developer tools called"
fi
echo ""

is_allowed_failure() {
  local provider="$1"
  local model="$2"
  local key="${provider}:${model}"
  for allowed in "${ALLOWED_FAILURES[@]}"; do
    if [ "$allowed" = "$key" ]; then
      return 0
    fi
  done
  return 1
}

should_skip_provider() {
  local provider="$1"
  if [ -z "$SKIP_PROVIDERS" ]; then
    return 1
  fi
  IFS=',' read -ra SKIP_LIST <<< "$SKIP_PROVIDERS"
  for skip in "${SKIP_LIST[@]}"; do
    # Trim whitespace
    skip=$(echo "$skip" | xargs)
    if [ "$skip" = "$provider" ]; then
      return 0
    fi
  done
  return 1
}

RESULTS=()
HARD_FAILURES=()

for provider_config in "${PROVIDERS[@]}"; do
  # Split on " -> " to get provider and models
  PROVIDER="${provider_config%% -> *}"
  MODELS_STR="${provider_config#* -> }"

  # Skip provider if it's in SKIP_PROVIDERS
  if should_skip_provider "$PROVIDER"; then
    echo "⊘ Skipping provider: ${PROVIDER} (SKIP_PROVIDERS)"
    echo "---"
    continue
  fi

  # Split models on "|"
  IFS='|' read -ra MODELS <<< "$MODELS_STR"
  for MODEL in "${MODELS[@]}"; do
    export GOOSE_PROVIDER="$PROVIDER"
    export GOOSE_MODEL="$MODEL"
    TESTDIR=$(mktemp -d)
    echo "hello" > "$TESTDIR/hello.txt"
    echo "Provider: ${PROVIDER}"
    echo "Model: ${MODEL}"
    echo ""
    TMPFILE=$(mktemp)
    (cd "$TESTDIR" && "$SCRIPT_DIR/target/release/goose" run --text "Immediately use the shell tool to run 'ls'. Do not ask for confirmation." --with-builtin "$BUILTINS" 2>&1) | tee "$TMPFILE"
    echo ""
    if grep -qE "$SUCCESS_PATTERN" "$TMPFILE"; then
      echo "✓ SUCCESS: Test passed - $SUCCESS_MSG"
      RESULTS+=("✓ ${PROVIDER}: ${MODEL}")
    else
      if is_allowed_failure "$PROVIDER" "$MODEL"; then
        echo "⚠ FLAKY: Test failed but model is in allowed failures list - $FAILURE_MSG"
        RESULTS+=("⚠ ${PROVIDER}: ${MODEL} (flaky)")
      else
        echo "✗ FAILED: Test failed - $FAILURE_MSG"
        RESULTS+=("✗ ${PROVIDER}: ${MODEL}")
        HARD_FAILURES+=("${PROVIDER}: ${MODEL}")
      fi
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

if [ ${#HARD_FAILURES[@]} -gt 0 ]; then
  echo ""
  echo "Hard failures (${#HARD_FAILURES[@]}):"
  for failure in "${HARD_FAILURES[@]}"; do
    echo "  - $failure"
  done
  echo ""
  echo "Some tests failed!"
  exit 1
else
  if echo "${RESULTS[@]}" | grep -q "⚠"; then
    echo ""
    echo "All required tests passed! (some flaky tests failed but are allowed)"
  else
    echo ""
    echo "All tests passed!"
  fi
fi
