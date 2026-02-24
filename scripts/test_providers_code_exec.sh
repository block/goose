#!/bin/bash
# Provider smoke tests - code execution mode (JS batching)

LIB_DIR="$(cd "$(dirname "$0")" && pwd)"
source "$LIB_DIR/test_providers_lib.sh"

echo "Mode: code_execution (JS batching)"
echo ""

# --- Setup ---

GOOSE_BIN=$(build_goose)
BUILTINS="developer,code_execution"

# --- Test case ---

run_test() {
  local provider="$1" model="$2" result_file="$3" output_file="$4"
  local testdir=$(mktemp -d)

  echo "hello" > "$testdir/hello.txt"
  local prompt="Run 'ls' to list files in the current directory."

  # Run goose
  (
    export GOOSE_PROVIDER="$provider"
    export GOOSE_MODEL="$model"
    cd "$testdir" && "$GOOSE_BIN" run --text "$prompt" --with-builtin "$BUILTINS" 2>&1
  ) > "$output_file" 2>&1

  # Verify: code_execution tool must be called
  if grep -qE "(execute \| code_execution)|(get_function_details \| code_execution)|(▸.*execute)" "$output_file"; then
    echo "success|code_execution tool called" > "$result_file"
  else
    echo "failure|no code_execution tool calls found" > "$result_file"
  fi

  rm -rf "$testdir"
}

build_test_cases --skip-agentic
run_test_cases run_test
report_results
