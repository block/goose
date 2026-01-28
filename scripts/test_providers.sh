#!/bin/bash
# Test providers with optional code_execution mode
# Usage:
#   ./test_providers.sh              # Normal mode (direct tool calls)
#   ./test_providers.sh --code-exec  # Code execution mode (JS batching)

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
# Base providers that are always tested (with appropriate env vars)
PROVIDERS=(
  "openrouter -> google/gemini-2.5-pro|anthropic/claude-sonnet-4.5|qwen/qwen3-coder:exacto|z-ai/glm-4.6:exacto|nvidia/nemotron-3-nano-30b-a3b"
  "xai -> grok-3"
  "openai -> gpt-4o|gpt-4o-mini|gpt-3.5-turbo|gpt-5"
  "anthropic -> claude-sonnet-4-5-20250929|claude-opus-4-1-20250805"
  "google -> gemini-2.5-pro|gemini-2.5-flash|gemini-3-pro-preview|gemini-3-flash-preview"
  "tetrate -> claude-sonnet-4-20250514"
)

# Conditionally add providers based on environment variables

# Databricks: requires DATABRICKS_HOST and DATABRICKS_TOKEN
if [ -n "$DATABRICKS_HOST" ] && [ -n "$DATABRICKS_TOKEN" ]; then
  echo "✓ Including Databricks tests"
  PROVIDERS+=("databricks -> databricks-claude-sonnet-4|gemini-2-5-flash|gpt-4o")
else
  echo "⚠️  Skipping Databricks tests (DATABRICKS_HOST and DATABRICKS_TOKEN required)"
fi

# Azure OpenAI: requires AZURE_OPENAI_ENDPOINT and AZURE_OPENAI_DEPLOYMENT_NAME
if [ -n "$AZURE_OPENAI_ENDPOINT" ] && [ -n "$AZURE_OPENAI_DEPLOYMENT_NAME" ]; then
  echo "✓ Including Azure OpenAI tests"
  PROVIDERS+=("azure_openai -> ${AZURE_OPENAI_DEPLOYMENT_NAME}")
else
  echo "⚠️  Skipping Azure OpenAI tests (AZURE_OPENAI_ENDPOINT and AZURE_OPENAI_DEPLOYMENT_NAME required)"
fi

# AWS Bedrock: requires AWS credentials (profile or keys) and AWS_REGION
if [ -n "$AWS_REGION" ] && { [ -n "$AWS_PROFILE" ] || [ -n "$AWS_ACCESS_KEY_ID" ]; }; then
  echo "✓ Including AWS Bedrock tests"
  PROVIDERS+=("aws_bedrock -> us.anthropic.claude-sonnet-4-5-20250929-v1:0")
else
  echo "⚠️  Skipping AWS Bedrock tests (AWS_REGION and AWS_PROFILE or AWS credentials required)"
fi

# GCP Vertex AI: requires GCP_PROJECT_ID
if [ -n "$GCP_PROJECT_ID" ]; then
  echo "✓ Including GCP Vertex AI tests"
  PROVIDERS+=("gcp_vertex_ai -> gemini-2.5-pro")
else
  echo "⚠️  Skipping GCP Vertex AI tests (GCP_PROJECT_ID required)"
fi

# Snowflake: requires SNOWFLAKE_HOST and SNOWFLAKE_TOKEN
if [ -n "$SNOWFLAKE_HOST" ] && [ -n "$SNOWFLAKE_TOKEN" ]; then
  echo "✓ Including Snowflake tests"
  PROVIDERS+=("snowflake -> claude-sonnet-4-5")
else
  echo "⚠️  Skipping Snowflake tests (SNOWFLAKE_HOST and SNOWFLAKE_TOKEN required)"
fi

# Venice: requires VENICE_API_KEY
if [ -n "$VENICE_API_KEY" ]; then
  echo "✓ Including Venice tests"
  PROVIDERS+=("venice -> llama-3.3-70b")
else
  echo "⚠️  Skipping Venice tests (VENICE_API_KEY required)"
fi

# LiteLLM: requires LITELLM_API_KEY (and optionally LITELLM_HOST)
if [ -n "$LITELLM_API_KEY" ]; then
  echo "✓ Including LiteLLM tests"
  PROVIDERS+=("litellm -> gpt-4o-mini")
else
  echo "⚠️  Skipping LiteLLM tests (LITELLM_API_KEY required)"
fi

# Ollama: requires OLLAMA_HOST (or uses default localhost:11434)
if [ -n "$OLLAMA_HOST" ] || command -v ollama &> /dev/null; then
  echo "✓ Including Ollama tests"
  PROVIDERS+=("ollama -> qwen3")
else
  echo "⚠️  Skipping Ollama tests (OLLAMA_HOST required or ollama must be installed)"
fi

# SageMaker TGI: requires AWS credentials and SAGEMAKER_ENDPOINT_NAME
if [ -n "$SAGEMAKER_ENDPOINT_NAME" ] && [ -n "$AWS_REGION" ]; then
  echo "✓ Including SageMaker TGI tests"
  PROVIDERS+=("sagemaker_tgi -> sagemaker-tgi-endpoint")
else
  echo "⚠️  Skipping SageMaker TGI tests (SAGEMAKER_ENDPOINT_NAME and AWS_REGION required)"
fi

# GitHub Copilot: requires OAuth setup (check for cached token)
if [ -n "$GITHUB_COPILOT_TOKEN" ] || [ -f "$HOME/.config/goose/github_copilot_token.json" ]; then
  echo "✓ Including GitHub Copilot tests"
  PROVIDERS+=("github_copilot -> gpt-4.1")
else
  echo "⚠️  Skipping GitHub Copilot tests (OAuth setup required - run 'goose configure' first)"
fi

# ChatGPT Codex: requires OAuth setup
if [ -n "$CHATGPT_CODEX_TOKEN" ] || [ -f "$HOME/.config/goose/chatgpt_codex_token.json" ]; then
  echo "✓ Including ChatGPT Codex tests"
  PROVIDERS+=("chatgpt_codex -> gpt-5.1-codex")
else
  echo "⚠️  Skipping ChatGPT Codex tests (OAuth setup required - run 'goose configure' first)"
fi

# CLI-based providers (require the CLI tool to be installed)

# Claude Code CLI: requires 'claude' CLI tool
if command -v claude &> /dev/null; then
  echo "✓ Including Claude Code CLI tests"
  PROVIDERS+=("claude-code -> claude-sonnet-4-20250514")
else
  echo "⚠️  Skipping Claude Code CLI tests ('claude' CLI tool required)"
fi

# Codex CLI: requires 'codex' CLI tool
if command -v codex &> /dev/null; then
  echo "✓ Including Codex CLI tests"
  PROVIDERS+=("codex -> gpt-5.2-codex")
else
  echo "⚠️  Skipping Codex CLI tests ('codex' CLI tool required)"
fi

# Gemini CLI: requires 'gemini' CLI tool
if command -v gemini &> /dev/null; then
  echo "✓ Including Gemini CLI tests"
  PROVIDERS+=("gemini-cli -> gemini-2.5-pro")
else
  echo "⚠️  Skipping Gemini CLI tests ('gemini' CLI tool required)"
fi

# Cursor Agent: requires 'cursor-agent' CLI tool
if command -v cursor-agent &> /dev/null; then
  echo "✓ Including Cursor Agent tests"
  PROVIDERS+=("cursor-agent -> auto")
else
  echo "⚠️  Skipping Cursor Agent tests ('cursor-agent' CLI tool required)"
fi

echo ""

# Configure mode-specific settings
if [ "$CODE_EXEC_MODE" = true ]; then
  echo "Mode: code_execution (JS batching)"
  BUILTINS="developer,code_execution"
  # Match code_execution tool usage:
  # - "execute_code | code_execution" or "read_module | code_execution" (fallback format)
  # - "tool call | execute_code" or "tool calls | execute_code" (new format with tool_graph)
  SUCCESS_PATTERN="(execute_code \| code_execution)|(read_module \| code_execution)|(tool calls? \| execute_code)"
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

RESULTS=()
HARD_FAILURES=()

for provider_config in "${PROVIDERS[@]}"; do
  # Split on " -> " to get provider and models
  PROVIDER="${provider_config%% -> *}"
  MODELS_STR="${provider_config#* -> }"
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
