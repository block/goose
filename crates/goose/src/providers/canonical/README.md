# Canonical Model System

This directory contains the canonical model infrastructure for standardizing model metadata across different providers.

## Overview

The canonical model system solves the problem that different LLM providers return varying levels of information about their models. Some providers only return model names, while others provide rich metadata. This system:

1. **Maintains a canonical registry** of models with standardized metadata
2. **Provides a mapping layer** for providers to map their models to canonical models
3. **Enables model validation** to ensure proper metadata for all supported models

## Components

### `model.rs`

Defines the core data structures:

- `ModelType` enum: chat, voice, embedding, image, other
- `CanonicalModel`: Complete model metadata including:
  - Basic info: name (provider/model-name format), type, context limit
  - Capabilities: streaming, tools
  - Pricing: input/output token costs (in USD)
  - Additional features: cache control

### `registry.rs`

Provides model registry management:

- `CanonicalModelRegistry`: In-memory registry of canonical models
- Load/save from/to JSON files
- Query and lookup operations

### `canonical_models.json`

JSON file containing the canonical model definitions. This is the source of truth for model metadata.

## Usage

### For Provider Implementors

Implement the `map_to_canonical_models()` method in your provider:

```rust
async fn map_to_canonical_models(&self) -> Result<Vec<ModelMapping>, ProviderError> {
    let models = self.fetch_supported_models().await?
        .unwrap_or_default();

    let mut mappings = Vec::new();

    for model in models {
        // Map your provider's model name to canonical name
        let canonical = match model.as_str() {
            "claude-3-5-sonnet-20241022" => "anthropic/claude-3-5-sonnet",
            "gpt-4-turbo-2024-04-09" => "openai/gpt-4-turbo",
            // ... more mappings
            _ => continue, // Skip unmapped models
        };

        mappings.push(
            ModelMapping::new(model, canonical).verified()
        );
    }

    Ok(mappings)
}
```

### Testing Your Mappings

Use the `canonical_model_checker` example to test your mappings:

```bash
cargo run --example canonical_model_checker
```

This will:
- Fetch models from all major providers
- Check which models are mapped to canonical models
- Report unmapped models
- Show canonical models in use
- Compare with previous runs

**Note:** Requires proper provider credentials to be configured.

### Adding New Canonical Models

1. Add the model definition to `canonical_models.json`
2. Include all required metadata:
   - name (provider/model-name format), model_type, context_limit (required)
   - capabilities (streaming, tools, cache_control)
   - pricing information (in USD)
3. Run the test script to verify mappings

## Model Metadata Fields

- `name`: Canonical name for the model using provider/model-name format (e.g., "anthropic/claude-3-5-sonnet", "openai/gpt-4o")
- `model_type`: Type of model (chat, voice, embedding, image, other)
- `context_limit`: Maximum context window in tokens
- `supports_streaming`: Whether model supports streaming responses
- `supports_tools`: Whether model supports tool/function calling (includes MCP)
- `input_token_cost`: Cost per million input tokens in USD (optional)
- `output_token_cost`: Cost per million output tokens in USD (optional)
- `supports_cache_control`: Whether model supports prompt caching

**Note:** Canonical models represent model families without datetime-specific versions. Provider-specific versioned models (e.g., "claude-3-5-sonnet-20241022") should be mapped to their canonical equivalents (e.g., "anthropic/claude-3-5-sonnet").

## Goals

- **Consistency**: Standardized model information across providers
- **Validation**: Ensure all models have proper metadata
- **Tracking**: Monitor when providers add/remove models
- **Selection**: Filter models by capability (e.g., exclude voice models from chat UI)
