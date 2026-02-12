#!/bin/bash
# Provider smoke tests - normal mode (direct tool calls)
#
# For each provider, asks goose to:
#   1. Run 'which ls' with empty PATH (tests PATH propagation via
#      extend_path_with_shell, PR #7161)
#   2. Read a file via text_editor view (tests text_editor, PR #7167)
# Verifies the developer shell tool restores PATH from the user's shell.
#
# Environment variables:
#   SKIP_PROVIDERS  Comma-separated list of providers to skip
#   SKIP_BUILD      Skip the cargo build step if set

LIB_DIR="$(cd "$(dirname "$0")" && pwd)"
source "$LIB_DIR/test_providers_lib.sh"

echo "Mode: normal (direct tool calls)"
echo ""

# --- Setup ---

GOOSE_BIN=$(build_goose)
BUILTINS="developer,autovisualiser,computercontroller,tutorial,todo,extensionmanager"

# Test content for agentic provider verification
mkdir -p target
TEST_CONTENT="test-content-abc123"
TEST_FILE="./target/test-content.txt"
echo "$TEST_CONTENT" > "$TEST_FILE"

# --- Test case ---

run_test() {
  local provider="$1" model="$2" result_file="$3" output_file="$4"
  local testdir=$(mktemp -d)

  local prompt
  if is_agentic_provider "$provider"; then
    cp "$TEST_FILE" "$testdir/test-content.txt"
    prompt="read ./test-content.txt and output its contents exactly"
  else
    echo "$TEST_CONTENT" > "$testdir/hello.txt"
    prompt="read the ./hello.txt"
  fi

  (
    export GOOSE_PROVIDER="$provider"
    export GOOSE_MODEL="$model"
    export PATH=""
    cd "$testdir" && "$GOOSE_BIN" run --text "$prompt" --with-builtin "$BUILTINS" 2>&1
  ) > "$output_file" 2>&1

  # Verify: agentic providers must echo test content,
  # regular providers must have 'which ls' resolve to an actual path
  if is_agentic_provider "$provider"; then
    if grep -qi "$TEST_CONTENT" "$output_file"; then
      echo "success|test content echoed back" > "$result_file"
    else
      echo "failure|test content not found in output" > "$result_file"
    fi
  else
    if grep -qE "/bin/ls|/usr/bin/ls" "$output_file"; then
      echo "success|PATH propagated, which ls resolved" > "$result_file"
    else
      echo "failure|PATH not propagated, which ls did not resolve" > "$result_file"
    fi
  fi

  rm -rf "$testdir"
}

# --- Developer PATH propagation test (PR #7161) ---
# Runs once with a single provider. Empty PATH verifies that
# extend_path_with_shell restores it from the user's shell.

run_developer_path_test() {
  local provider="anthropic"
  local model="claude-sonnet-4-5-20250929"

  echo "Provider: $provider  Model: $model"
  echo ""

  local prompt="use the developer tool to run 'which ls'. don't attempt with other approaches if you don't find it"
  local testdir=$(mktemp -d)
  local output_file="$testdir/output.txt"

  (
    export GOOSE_PROVIDER="$provider"
    export GOOSE_MODEL="$model"
    export PATH=""
    cd "$testdir" && "$GOOSE_BIN" run \
      --text "$prompt" \
      --with-builtin developer 2>&1
  ) > "$output_file" 2>&1

  cat "$output_file"
  echo ""

  if grep -qE "/bin/ls|/usr/bin/ls|aliased to" "$output_file"; then
    echo "✓ PASS: developer shell found 'ls' via PATH from user's shell"
  else
    echo "✗ FAIL: developer shell could not find 'ls' — PATH not restored from user's shell"
    HARD_FAILURES+=("developer-tool-path-test")
  fi
  echo "---"
  echo ""

  rm -rf "$testdir"
}

# --- Run ---

run_developer_path_test
# build_test_cases
# run_test_cases run_test
# report_results
