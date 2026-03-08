#!/bin/bash
set -e

if [ -z "$SKIP_BUILD" ]; then
  echo "Building goose..."
  cargo build --bin goose
  echo ""
else
  echo "Skipping build (SKIP_BUILD is set)..."
  echo ""
fi

SCRIPT_DIR=$(pwd)
GOOSE_BIN="$SCRIPT_DIR/target/debug/goose"

TEST_PROVIDER=${GOOSE_PROVIDER:-anthropic}
TEST_MODEL=${GOOSE_MODEL:-claude-haiku-4-5-20251001}
MCP_SAMPLING_TOOL="trigger-sampling-request"

RESULTS=()

TESTDIR=$(mktemp -d)

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

TMPFILE=$(mktemp)
(cd "$TESTDIR" && GOOSE_PROVIDER="$TEST_PROVIDER" GOOSE_MODEL="$TEST_MODEL" \
    "$GOOSE_BIN" run --recipe recipe.yaml 2>&1) | tee "$TMPFILE"

if grep -q "add | test_mcp" "$TMPFILE" && grep -q "100" "$TMPFILE"; then
    echo "✓ FastMCP stderr test passed"
    RESULTS+=("✓ FastMCP stderr")
else
    echo "✗ FastMCP stderr test failed"
    RESULTS+=("✗ FastMCP stderr")
fi

rm "$TMPFILE"
rm -rf "$TESTDIR"
echo ""

TESTDIR=$(mktemp -d)
TMPFILE=$(mktemp)

(cd "$TESTDIR" && GOOSE_PROVIDER="$TEST_PROVIDER" GOOSE_MODEL="$TEST_MODEL" \
    "$GOOSE_BIN" run --text "Use the sampleLLM tool to ask for a quote from The Great Gatsby" \
    --with-extension "npx -y @modelcontextprotocol/server-everything@2026.1.14" 2>&1) | tee "$TMPFILE"

if grep -q "$MCP_SAMPLING_TOOL | " "$TMPFILE"; then
    JUDGE_PROMPT=$(cat <<EOF
You are a validator. You will be given a transcript of a CLI run that used an MCP tool to initiate MCP sampling.
The MCP server requests a quote from The Great Gatsby from the model via sampling.

Task: Determine whether the transcript shows that the sampling request reached the model and that the output included either:
  • A recognizable quote, paraphrase, or reference from The Great Gatsby, or
  • A clear attempt or explanation from the model about why the quote could not be returned.

If either of these conditions is true, respond PASS.
If there is no evidence that the model attempted or returned a Gatsby-related response, respond FAIL.
If uncertain, lean toward PASS.

Output format: Respond with exactly one word on a single line:
PASS
or
FAIL

Transcript:
----- BEGIN TRANSCRIPT -----
$(cat "$TMPFILE")
----- END TRANSCRIPT -----
EOF
)
    JUDGE_OUT=$(GOOSE_PROVIDER="$TEST_PROVIDER" GOOSE_MODEL="$TEST_MODEL" \
        "$GOOSE_BIN" run --text "$JUDGE_PROMPT" 2>&1)

    if echo "$JUDGE_OUT" | tr -d '\r' | grep -Eq '^[[:space:]]*PASS[[:space:]]*$'; then
        echo "✓ MCP sampling test passed"
        RESULTS+=("✓ MCP sampling")
    else
        echo "✗ MCP sampling test failed"
        RESULTS+=("✗ MCP sampling")
    fi
else
    echo "✗ MCP sampling test failed - $MCP_SAMPLING_TOOL tool not called"
    RESULTS+=("✗ MCP sampling")
fi

rm "$TMPFILE"
rm -rf "$TESTDIR"
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
