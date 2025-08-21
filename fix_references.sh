#!/bin/bash

# Fix references to model_config in streaming code
providers=(
    "anthropic.rs"
    "databricks.rs"
)

for provider in "${providers[@]}"; do
    file="crates/goose/src/providers/$provider"
    if [ -f "$file" ]; then
        echo "Fixing references in $file..."
        # In streaming code, use self.model instead of model_config
        sed -i '' 's/let model_config = self\.model\.clone();//' "$file"
        sed -i '' 's/emit_debug_trace(model_config,/emit_debug_trace(\&self.model,/g' "$file"
        sed -i '' 's/emit_debug_trace(&model_config,/emit_debug_trace(\&self.model,/g' "$file"
    fi
done

echo "Done fixing references"
