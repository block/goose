#!/bin/bash

LIB_DIR="$(cd "$(dirname "$0")" && pwd)"
source "$LIB_DIR/test_providers_lib.sh"

echo "Mode: normal (direct tool calls)"
echo ""

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
    # Write two files with unique random tokens. Validation checks that the shell
    # tool was used and that both tokens appear in the output, proving the model
    # actually read the files (random tokens can't be guessed or hallucinated).
    local token_a="smoke-alpha-$RANDOM"
    local token_b="smoke-bravo-$RANDOM"
    echo "$token_a" > "$testdir/part-a.txt"
    echo "$token_b" > "$testdir/part-b.txt"
    # Store tokens so validation can check them
    echo "$token_a" > "$testdir/.token_a"
    echo "$token_b" > "$testdir/.token_b"
    prompt="Use the shell tool to cat ./part-a.txt and ./part-b.txt, then reply with ONLY the contents of both files, one per line, nothing else."
  fi

  (
    export GOOSE_PROVIDER="$provider"
    export GOOSE_MODEL="$model"
    cd "$testdir" && "$GOOSE_BIN" run --text "$prompt" --with-builtin "$BUILTINS" 2>&1
  ) > "$output_file" 2>&1

  if is_agentic_provider "$provider"; then
    if grep -qi "$TEST_CONTENT" "$output_file"; then
      echo "success|test content found by model" > "$result_file"
    else
      echo "failure|test content not found by model" > "$result_file"
    fi
  else
    local token_a token_b
    token_a=$(cat "$testdir/.token_a")
    token_b=$(cat "$testdir/.token_b")
    if ! grep -qE "(shell \| developer)|(▸.*shell)" "$output_file"; then
      echo "failure|model did not use shell tool" > "$result_file"
    elif ! grep -q "$token_a" "$output_file"; then
      echo "failure|model did not return contents of part-a.txt ($token_a)" > "$result_file"
    elif ! grep -q "$token_b" "$output_file"; then
      echo "failure|model did not return contents of part-b.txt ($token_b)" > "$result_file"
    else
      echo "success|model read and returned both file contents" > "$result_file"
    fi
  fi

  rm -rf "$testdir"
}

build_test_cases
run_test_cases run_test
report_results
