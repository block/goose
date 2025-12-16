#!/bin/bash
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

JUDGE_PROVIDER=${GOOSE_JUDGE_PROVIDER:-openrouter}
JUDGE_MODEL=${GOOSE_JUDGE_MODEL:-google/gemini-2.5-flash}

PROVIDERS=(
  #"google:gemini-2.5-pro"
  "anthropic:claude-haiku-4-5-20251001"
  #"openrouter:google/gemini-2.5-pro"
  #"openai:gpt-5-mini"
)

# In CI, only run Databricks tests if DATABRICKS_HOST and DATABRICKS_TOKEN are set
# Locally, always run Databricks tests
if [ -n "$CI" ]; then
  if [ -n "$DATABRICKS_HOST" ] && [ -n "$DATABRICKS_TOKEN" ]; then
    echo "✓ Including Databricks tests"
    PROVIDERS+=("databricks:databricks-claude-sonnet-4:gemini-2-5-flash:gpt-4o")
  else
    echo "⚠️  Skipping Databricks tests (DATABRICKS_HOST and DATABRICKS_TOKEN required in CI)"
  fi
else
  echo "✓ Including Databricks tests"
  PROVIDERS+=("databricks:databricks-claude-sonnet-4:gemini-2-5-flash:gpt-4o")
fi

RESULTS=()

for provider_config in "${PROVIDERS[@]}"; do
  IFS=':' read -ra PARTS <<< "$provider_config"
  PROVIDER="${PARTS[0]}"
  for i in $(seq 1 $((${#PARTS[@]} - 1))); do
    MODEL="${PARTS[$i]}"
    export GOOSE_PROVIDER="$PROVIDER"
    export GOOSE_MODEL="$MODEL"
    TESTDIR=$(mktemp -d)
    echo "Provider: ${PROVIDER}"
    echo "Model: ${MODEL}"
    echo ""
    TMPFILE=$(mktemp)
    (cd "$TESTDIR" && "$SCRIPT_DIR/target/release/goose" run --text "Use the sampleLLM tool to ask for a quote from The Great Gatsby" --with-extension "npx -y @modelcontextprotocol/server-everything" 2>&1) | tee "$TMPFILE"
    echo ""
    if grep -q "sampleLLM | " "$TMPFILE"; then

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
      JUDGE_OUT=$(GOOSE_PROVIDER="$JUDGE_PROVIDER" GOOSE_MODEL="$JUDGE_MODEL" \
        "$SCRIPT_DIR/target/release/goose" run --text "$JUDGE_PROMPT" 2>&1)

      if echo "$JUDGE_OUT" | tr -d '\r' | grep -Eq '^[[:space:]]*PASS[[:space:]]*$'; then
        echo "✓ SUCCESS: MCP sampling test passed - confirmed Gatsby related response"
        RESULTS+=("✓ MCP Sampling ${PROVIDER}: ${MODEL}")
      else
        echo "✗ FAILED: MCP sampling test failed - did not confirm Gatsby related response"
        echo "  Judge provider/model: ${JUDGE_PROVIDER}:${JUDGE_MODEL}"
        echo "  Judge output (snippet):"
        echo "$JUDGE_OUT" | tail -n 20
        RESULTS+=("✗ MCP Sampling ${PROVIDER}: ${MODEL}")
      fi
    else
      echo "✗ FAILED: MCP sampling test failed - sampleLLM tool not called"
      RESULTS+=("✗ MCP Sampling ${PROVIDER}: ${MODEL}")
    fi
    rm "$TMPFILE"
    rm -rf "$TESTDIR"
    echo "---"
  done
done

echo ""
echo "=== MCP Sampling Test Summary ==="
for result in "${RESULTS[@]}"; do
  echo "$result"
done

if echo "${RESULTS[@]}" | grep -q "✗"; then
  echo ""
  echo "Some MCP sampling tests failed!"
  exit 1
else
  echo ""
  echo "All MCP sampling tests passed!"
fi
