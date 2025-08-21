#!/bin/bash

# List of provider files that need updating
providers=(
    "anthropic.rs"
    "azure.rs"
    "bedrock.rs"
    "claude_code.rs"
    "cursor_agent.rs"
    "databricks.rs"
    "factory.rs"
    "gcpvertexai.rs"
    "gemini_cli.rs"
    "githubcopilot.rs"
    "google.rs"
    "groq.rs"
    "lead_worker.rs"
    "litellm.rs"
    "ollama.rs"
    "openrouter.rs"
    "sagemaker_tgi.rs"
    "snowflake.rs"
    "testprovider.rs"
    "together.rs"
    "venice.rs"
    "xai.rs"
    "fireworks.rs"
    "octoai.rs"
)

for provider in "${providers[@]}"; do
    file="crates/goose/src/providers/$provider"
    if [ -f "$file" ]; then
        echo "Updating $file..."
        # Update the parameter from model: &str to model_config: &ModelConfig
        sed -i '' 's/async fn complete_with_model(/async fn complete_with_model(/g' "$file"
        sed -i '' 's/model: &str,/model_config: \&ModelConfig,/g' "$file"
        sed -i '' 's/_model: &str,/_model_config: \&ModelConfig,/g' "$file"
    fi
done

echo "Done updating provider files"
