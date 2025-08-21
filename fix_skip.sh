#!/bin/bash

# List of provider files that need updating
providers=(
    "anthropic.rs"
    "azure.rs"
    "bedrock.rs"
    "claude_code.rs"
    "cursor_agent.rs"
    "databricks.rs"
    "gcpvertexai.rs"
    "gemini_cli.rs"
    "githubcopilot.rs"
    "google.rs"
    "groq.rs"
    "ollama.rs"
    "openrouter.rs"
    "sagemaker_tgi.rs"
    "snowflake.rs"
    "venice.rs"
    "xai.rs"
)

for provider in "${providers[@]}"; do
    file="crates/goose/src/providers/$provider"
    if [ -f "$file" ]; then
        echo "Fixing skip in $file..."
        # Update skip(self, model, ...) to skip(self, model_config, ...)
        sed -i '' 's/skip(self, model,/skip(self, model_config,/g' "$file"
        sed -i '' 's/skip(self, _model,/skip(self, _model_config,/g' "$file"
    fi
done

echo "Done fixing skip parameters"
