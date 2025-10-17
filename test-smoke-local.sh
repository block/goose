#!/bin/bash

# Local smoke test script - simulates the CI workflow for testing locally
# Run this to validate changes before pushing to CI

set -e

echo "=== Local Smoke Test Script ==="
echo "This script simulates the CI smoke test using cargo run (development version)"
echo ""

# Source hermit environment
source ./bin/activate-hermit

# Set up test environment variables (you'll need to provide your own API keys)
export GOOSE_DISABLE_KEYRING=1
export HOME=/tmp/goose-test-home

# Prompt for provider and model selection
echo "Select provider and model for testing:"
echo "1) anthropic / claude-sonnet-4-5-20250929"
echo "2) openai / gpt-5"
echo "3) custom (you'll enter your own)"
read -p "Enter choice (1-3): " choice

case $choice in
  1)
    export GOOSE_PROVIDER="anthropic"
    export GOOSE_MODEL="claude-sonnet-4-5-20250929"
    if [ -z "$ANTHROPIC_API_KEY" ]; then
      read -p "Enter your Anthropic API key: " ANTHROPIC_API_KEY
      export ANTHROPIC_API_KEY
    fi
    ;;
  2)
    export GOOSE_PROVIDER="openai"
    export GOOSE_MODEL="gpt-5"
    if [ -z "$OPENAI_API_KEY" ]; then
      read -p "Enter your OpenAI API key: " OPENAI_API_KEY
      export OPENAI_API_KEY
    fi
    ;;
  3)
    read -p "Enter provider: " GOOSE_PROVIDER
    read -p "Enter model: " GOOSE_MODEL
    export GOOSE_PROVIDER
    export GOOSE_MODEL
    echo "Make sure you have the appropriate API key environment variable set!"
    ;;
  *)
    echo "Invalid choice"
    exit 1
    ;;
esac

# Ensure the HOME directory structure exists
mkdir -p $HOME/.local/share/goose/sessions
mkdir -p $HOME/.config/goose

# Create a unique test directory
TEST_DIR="/tmp/goose-test-${GOOSE_PROVIDER}-${GOOSE_MODEL}-$(date +%s)"
mkdir -p "$TEST_DIR"

echo ""
echo "=== Test Configuration ==="
echo "Provider: ${GOOSE_PROVIDER}"
echo "Model: ${GOOSE_MODEL}"
echo "Test Directory: $TEST_DIR"
echo "Workspace: $(pwd)"
echo ""

# Create test file in the test directory
cd "$TEST_DIR"
echo "hello" > hello.txt
echo "Created test file: hello.txt"
echo ""

# Get the workspace directory
WORKSPACE_DIR="$(cd "$(dirname "$0")" && pwd)"

echo "=== Running goose with cargo run (development version) ==="
echo "Command: cargo run --manifest-path \"$WORKSPACE_DIR/Cargo.toml\" --bin goose -- run --text \"please list files in the current directory\" --with-builtin developer"
echo ""

# Run goose using cargo run from the current test directory
OUTPUT=$(cargo run --manifest-path "$WORKSPACE_DIR/Cargo.toml" --bin goose -- run --text "please list files in the current directory" --with-builtin developer 2>&1)

echo "=== Output ==="
echo "$OUTPUT"
echo ""

# Check if hello.txt appears in output
if echo "$OUTPUT" | grep -q "hello.txt"; then
  echo "✓ SUCCESS: Test passed - found hello.txt in output"
  echo ""
  echo "The smoke test passed successfully!"
  exit 0
else
  echo "✗ FAILED: Test failed - hello.txt not found in output"
  echo ""
  echo "The smoke test failed. Check the output above for details."
  exit 1
fi
