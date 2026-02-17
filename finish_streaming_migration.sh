#!/bin/bash
# Script to finish the streaming migration by removing complete_with_model() from remaining providers
set -e

echo "Streaming Migration - Remaining Work"
echo "===================================="
echo ""

# Check current status
echo "Checking for remaining complete_with_model implementations..."
remaining=$(grep -l "async fn complete_with_model" crates/goose/src/providers/*.rs 2>/dev/null | wc -l)
echo "Found $remaining providers still with complete_with_model()"
echo ""

if [ $remaining -eq 0 ]; then
    echo "✅ All complete_with_model() methods removed!"
    echo ""
    echo "Running compilation check..."
    cargo check --package goose --lib
    exit 0
fi

echo "Remaining providers:"
grep -l "async fn complete_with_model" crates/goose/src/providers/*.rs | while read file; do
    basename "$file"
done
echo ""

echo "STREAMING PROVIDERS (simple deletion):"
echo "- chatgpt_codex, databricks, githubcopilot, openrouter"
echo ""
echo "NON-STREAMING PROVIDERS (need logic inlined into stream()):"
echo "- claude_code, codex, cursor_agent, gemini_cli, litellm"
echo "- sagemaker_tgi, snowflake, venice, testprovider"
echo ""
echo "See STREAMING_MIGRATION_STATUS.md for detailed instructions"
echo ""
echo "Quick test:"
echo "  cargo check --package goose --lib 2>&1 | grep complete_with_model | head -20"
