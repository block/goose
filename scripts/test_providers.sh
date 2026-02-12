#!/bin/bash

LIB_DIR="$(cd "$(dirname "$0")" && pwd)"
source "$LIB_DIR/test_providers_lib.sh"

echo "Mode: normal (direct tool calls)"
echo ""

# --- Setup ---
GOOSE_BIN=$(build_goose)
BUILTINS="developer"

mkdir -p target
TEST_CONTENT="test-content-abc123"
TEST_FILE="./target/test-content.txt"
echo "$TEST_CONTENT" > "$TEST_FILE"

run_test() {
  local provider="$1" model="$2" result_file="$3" output_file="$4"
  local testdir=$(mktemp -d)

  local prompt
  if is_agentic_provider "$provider"; then
    cp "$TEST_FILE" "$testdir/test-content.txt"
    prompt="read ./test-content.txt and output its contents exactly"
  else
    echo "$TEST_CONTENT" > "$testdir/hello.txt"
    prompt="Use the text_editor view command to read ./hello.txt, then output its contents in UPPERCASE don't use any other tool in Developer"
  fi

  (
    export GOOSE_PROVIDER="$provider"
    export GOOSE_MODEL="$model"
    export PATH=""
    cd "$testdir" && "$GOOSE_BIN" run --text "$prompt" --with-builtin "$BUILTINS" 2>&1
  ) > "$output_file" 2>&1

  if is_agentic_provider "$provider"; then
    if grep -qi "$TEST_CONTENT" "$output_file"; then
      echo "success|test content found by model" > "$result_file"
    else
      echo "failure|test content not found by model" > "$result_file"
    fi
  else
    if grep -q "TEST-CONTENT-ABC123" "$output_file"; then
      echo "success|model read and uppercased file content" > "$result_file"
    else
      echo "failure|model did not return uppercased file content" > "$result_file"
    fi
  fi

  rm -rf "$testdir"
}

build_test_cases
run_test_cases run_test
report_results
