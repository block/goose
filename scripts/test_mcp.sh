#!/bin/bash
set -e

echo "=== FastMCP stderr Regression Test ==="
echo "This script reproduces the bug where FastMCP servers fail to start"
echo "because they write a banner to stderr during initialization."
echo ""

# Build goose if not skipped
if [ -z "$SKIP_BUILD" ]; then
  echo "Building goose..."
  cargo build --release --bin goose
  echo ""
else
  echo "Skipping build (SKIP_BUILD is set)..."
  echo ""
fi

SCRIPT_DIR=$(pwd)
GOOSE_BIN="$SCRIPT_DIR/target/release/goose"

# Set provider/model defaults if not specified
TEST_PROVIDER=${GOOSE_PROVIDER:-anthropic}
TEST_MODEL=${GOOSE_MODEL:-claude-haiku-4-5-20251001}

echo "Provider: ${TEST_PROVIDER}"
echo "Model: ${TEST_MODEL}"
echo ""

TESTDIR=$(mktemp -d)
echo "Test directory: $TESTDIR"
echo ""

# Create a minimal FastMCP server
cat > "$TESTDIR/test_mcp.py" << 'EOF'
from typing import Annotated
from fastmcp import FastMCP

mcp = FastMCP("test_server")

@mcp.tool
def add(
    a: Annotated[float, "First number"],
    b: Annotated[float, "Second number"],
) -> Annotated[float, "Sum of the two numbers"]:
    """Add two numbers."""
    return a + b
EOF

# Create recipe
cat > "$TESTDIR/recipe.yaml" << 'EOF'
title: FastMCP Test
description: Test that FastMCP servers with stderr banners work
prompt: Use the add tool to calculate 42 + 58
extensions:
  - name: test_mcp
    cmd: uv
    args:
      - run
      - --with
      - fastmcp
      - fastmcp
      - run
      - test_mcp.py
    type: stdio
EOF

echo "Running goose with FastMCP server..."
echo "Expected: Tool should be called and return 100"
echo "Actual with bug: 'Failed to start extension' error"
echo ""

TMPFILE=$(mktemp)
(cd "$TESTDIR" && GOOSE_PROVIDER="$TEST_PROVIDER" GOOSE_MODEL="$TEST_MODEL" \
    "$GOOSE_BIN" run --recipe recipe.yaml 2>&1) | tee "$TMPFILE"

echo ""
echo "=== Test Result ==="
if grep -q "add | test_mcp" "$TMPFILE"; then
    if grep -q "100" "$TMPFILE"; then
        echo "✓ SUCCESS: FastMCP server started and tool was called"
        rm "$TMPFILE"
        rm -rf "$TESTDIR"
        exit 0
    fi
fi

if grep -q "Failed to start extension 'test_mcp'" "$TMPFILE"; then
    echo "✗ BUG CONFIRMED: FastMCP server failed to start due to stderr banner"
    echo ""
    echo "The error message shows the server banner was captured as stderr,"
    echo "which caused rmcp to think the process quit before initialization."
    rm "$TMPFILE"
    rm -rf "$TESTDIR"
    exit 1
fi

echo "? UNCLEAR: Test didn't match expected patterns"
rm "$TMPFILE"
rm -rf "$TESTDIR"
exit 1
