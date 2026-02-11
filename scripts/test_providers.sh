#!/bin/bash
# Test providers with optional code_execution mode
# Usage:
#   ./test_providers.sh              # Normal mode (direct tool calls)
#   ./test_providers.sh --code-exec  # Code execution mode (JS batching)
#
# Environment variables:
#   SKIP_PROVIDERS  Comma-separated list of providers to skip (e.g., "tetrate,xai")
#   SKIP_BUILD      Skip the cargo build step if set

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
  "google:gemini-2.5-flash"
  "google:gemini-3-pro-preview"
  "openrouter:nvidia/nemotron-3-nano-30b-a3b"
  "openai:gpt-3.5-turbo"
)

# Agentic providers handle tools internally and return text results.
# They can't produce the normal tool-call log patterns (e.g. "shell | developer").
AGENTIC_PROVIDERS=("claude-code" "codex" "gemini-cli" "cursor-agent")

if [ -f .env ]; then
  export $(grep -v '^#' .env | xargs)
fi

if [ -z "$SKIP_BUILD" ]; then
  echo "Building goose..."
  cargo build --bin goose
  echo ""
else
  echo "Skipping build (SKIP_BUILD is set)..."
  echo ""
fi

SCRIPT_DIR=$(pwd)

# Create a test file with known content in the current directory
# This cannot be /tmp as some agents cannot work outside the PWD
mkdir -p target
TEST_CONTENT="test-content-abc123"
TEST_FILE="./target/test-content.txt"
echo "$TEST_CONTENT" > "$TEST_FILE"

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
  # - "execute | code_execution" or "get_function_details | code_execution" (fallback format)
  # - "tool call | execute" or "tool calls | execute" (new format with tool_graph)
  SUCCESS_PATTERN="(execute \| code_execution)|(get_function_details \| code_execution)|(tool calls? \| execute)"
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

should_skip_provider() {
  local provider="$1"
  if [ -z "$SKIP_PROVIDERS" ]; then
    return 1
  fi
  IFS=',' read -ra SKIP_LIST <<< "$SKIP_PROVIDERS"
  for skip in "${SKIP_LIST[@]}"; do
    # Trim whitespace
    skip=$(echo "$skip" | xargs)
    if [ "$skip" = "$provider" ]; then
      return 0
    fi
  done
  return 1
}

is_agentic_provider() {
  local provider="$1"
  for agentic in "${AGENTIC_PROVIDERS[@]}"; do
    if [ "$agentic" = "$provider" ]; then
      return 0
    fi
  done
  return 1
}

# Create temp directory for results
RESULTS_DIR=$(mktemp -d)
trap "rm -rf $RESULTS_DIR" EXIT

# Maximum parallel jobs (default: number of CPU cores, or override with MAX_PARALLEL)
MAX_PARALLEL=${MAX_PARALLEL:-$(sysctl -n hw.ncpu 2>/dev/null || nproc 2>/dev/null || echo 8)}
echo "Running tests with up to $MAX_PARALLEL parallel jobs"
echo ""

# Function to run a single test
run_test() {
  local provider="$1"
  local model="$2"
  local result_file="$3"
  local output_file="$4"

  local testdir=$(mktemp -d)

  # Agentic providers use a file-read prompt with known content marker;
  # regular providers use the shell prompt that produces tool-call logs.
  local prompt
  if is_agentic_provider "$provider"; then
    cp "$TEST_FILE" "$testdir/test-content.txt"
    prompt="read ./test-content.txt and output its contents exactly"
  else
    echo "hello" > "$testdir/hello.txt"
    prompt="Immediately use the shell tool to run 'ls'. Do not ask for confirmation."
  fi

  # Run the test and capture output
  (
    export GOOSE_PROVIDER="$provider"
    export GOOSE_MODEL="$model"
    cd "$testdir" && "$SCRIPT_DIR/target/debug/goose" run --text "$prompt" --with-builtin "$BUILTINS" 2>&1
  ) > "$output_file" 2>&1

  # Check result: agentic providers return text containing the test content
  # instead of producing tool-call log patterns
  if is_agentic_provider "$provider"; then
    if grep -qi "$TEST_CONTENT" "$output_file"; then
      echo "success" > "$result_file"
    else
      echo "failure" > "$result_file"
    fi
  elif grep -qE "$SUCCESS_PATTERN" "$output_file"; then
    echo "success" > "$result_file"
  else
    echo "failure" > "$result_file"
  fi

  rm -rf "$testdir"
}

# Build list of all provider/model combinations
JOBS=()
job_index=0
for provider_config in "${PROVIDERS[@]}"; do
  PROVIDER="${provider_config%% -> *}"
  MODELS_STR="${provider_config#* -> }"

  # Skip provider if it's in SKIP_PROVIDERS
  if should_skip_provider "$PROVIDER"; then
    echo "⊘ Skipping provider: ${PROVIDER} (SKIP_PROVIDERS)"
    continue
  fi

  # Agentic providers don't use goose's code_execution system
  if [ "$CODE_EXEC_MODE" = true ] && is_agentic_provider "$PROVIDER"; then
    echo "⊘ Skipping agentic provider in code_exec mode: ${PROVIDER}"
    continue
  fi

  IFS='|' read -ra MODELS <<< "$MODELS_STR"
  for MODEL in "${MODELS[@]}"; do
    JOBS+=("$PROVIDER|$MODEL|$job_index")
    ((job_index++))
  done
done

total_jobs=${#JOBS[@]}
echo "Starting $total_jobs tests..."
echo ""

# Run first test sequentially if any jobs exist
if [ $total_jobs -gt 0 ]; then
  echo "Running first test sequentially..."
  first_job="${JOBS[0]}"
  IFS='|' read -r provider model idx <<< "$first_job"

  result_file="$RESULTS_DIR/result_$idx"
  output_file="$RESULTS_DIR/output_$idx"
  meta_file="$RESULTS_DIR/meta_$idx"
  echo "$provider|$model" > "$meta_file"

  # Run first test and wait for it to complete
  run_test "$provider" "$model" "$result_file" "$output_file"
  echo "First test completed."
  echo ""
fi

# Run remaining tests in parallel
if [ $total_jobs -gt 1 ]; then
  echo "Running remaining tests in parallel..."
  running_jobs=0
  for ((i=1; i<$total_jobs; i++)); do
    job="${JOBS[$i]}"
    IFS='|' read -r provider model idx <<< "$job"

    result_file="$RESULTS_DIR/result_$idx"
    output_file="$RESULTS_DIR/output_$idx"
    meta_file="$RESULTS_DIR/meta_$idx"
    echo "$provider|$model" > "$meta_file"

    # Run test in background
    run_test "$provider" "$model" "$result_file" "$output_file" &
    ((running_jobs++))

    # Wait if we've hit the parallel limit
    if [ $running_jobs -ge $MAX_PARALLEL ]; then
      wait -n 2>/dev/null || wait
      ((running_jobs--))
    fi
  done

  # Wait for all remaining jobs
  wait
fi

echo ""
echo "=== Test Results ==="
echo ""

# Collect results
RESULTS=()
HARD_FAILURES=()

for job in "${JOBS[@]}"; do
  IFS='|' read -r provider model idx <<< "$job"

  result_file="$RESULTS_DIR/result_$idx"
  output_file="$RESULTS_DIR/output_$idx"

  echo "Provider: $provider"
  echo "Model: $model"
  echo ""
  cat "$output_file"
  echo ""

  if [ -f "$result_file" ] && [ "$(cat "$result_file")" = "success" ]; then
    echo "✓ SUCCESS: Test passed - $SUCCESS_MSG"
    RESULTS+=("✓ ${provider}: ${model}")
  else
    if is_allowed_failure "$provider" "$model"; then
      echo "⚠ FLAKY: Test failed but model is in allowed failures list - $FAILURE_MSG"
      RESULTS+=("⚠ ${provider}: ${model} (flaky)")
    else
      echo "✗ FAILED: Test failed - $FAILURE_MSG"
      RESULTS+=("✗ ${provider}: ${model}")
      HARD_FAILURES+=("${provider}: ${model}")
    fi
  fi
  echo "---"
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
