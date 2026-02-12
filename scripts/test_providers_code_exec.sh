#!/bin/bash
# Provider smoke tests - code execution mode (JS batching)
#
# For each provider, asks goose to run 'ls' via shell.
# Verifies the code_execution tool was invoked.
# Agentic providers are skipped (they don't use goose's code_execution system).
#
# Environment variables:
#   SKIP_PROVIDERS  Comma-separated list of providers to skip
#   SKIP_BUILD      Skip the cargo build step if set

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
  # Matches: "execute | code_execution", "get_function_details | code_execution",
  #           "tool call | execute", "tool calls | execute"
  if grep -qE "(execute \| code_execution)|(get_function_details \| code_execution)|(tool calls? \| execute)" "$output_file"; then
    echo "success|code_execution tool called" > "$result_file"
  else
    echo "failure|no code_execution tool calls found" > "$result_file"
  fi

  rm -rf "$testdir"
}

build_test_cases --skip-agentic
run_test_cases run_test
report_results
