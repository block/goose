#!/bin/bash

# For providers that were creating a temporary model_config and setting model_name
# We need to remove those lines since model_config is now passed directly

providers=(
    "anthropic.rs"
    "azure.rs"
    "claude_code.rs"
    "cursor_agent.rs"
    "databricks.rs"
    "gcpvertexai.rs"
    "gemini_cli.rs"
    "githubcopilot.rs"
    "google.rs"
    "groq.rs"
    "litellm.rs"
    "ollama.rs"
    "openrouter.rs"
    "sagemaker_tgi.rs"
    "snowflake.rs"
    "venice.rs"
    "xai.rs"
    "together.rs"
    "fireworks.rs"
    "octoai.rs"
)

for provider in "${providers[@]}"; do
    file="crates/goose/src/providers/$provider"
    if [ -f "$file" ]; then
        echo "Fixing $file..."
        # Remove the lines that create temporary model_config
        sed -i '' '/let mut model_config = self\.model\.clone();/d' "$file"
        sed -i '' '/model_config\.model_name = model\.to_string();/d' "$file"
        # Change &model_config to model_config in create_request calls
        sed -i '' 's/create_request(&model_config,/create_request(model_config,/g' "$file"
        sed -i '' 's/emit_debug_trace(&model_config,/emit_debug_trace(model_config,/g' "$file"
    fi
done

echo "Done fixing model usage"
