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
  - Basic info: name, type, context limit
  - Capabilities: streaming, tools, vision, computer use
  - Pricing: input/output token costs
  - Additional features: cache control, custom metadata

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
            "claude-3-5-sonnet-20241022" => "claude-3-5-sonnet-20241022",
            "gpt-4-turbo-2024-04-09" => "gpt-4-turbo",
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
   - name, model_type, context_limit (required)
   - capabilities (streaming, tools, vision, etc.)
   - pricing information
3. Run the test script to verify mappings

## Model Metadata Fields

- `name`: Canonical name for the model (e.g., "claude-3-5-sonnet-20241022")
- `model_type`: Type of model (chat, voice, embedding, image, other)
- `context_limit`: Maximum context window in tokens
- `supports_streaming`: Whether model supports streaming responses
- `supports_tools`: Whether model supports tool/function calling
- `supports_vision`: Whether model supports image inputs
- `supports_computer_use`: Whether model supports computer use/MCP
- `input_token_cost`: Cost per million input tokens (optional)
- `output_token_cost`: Cost per million output tokens (optional)
- `currency`: Currency for pricing (default: "USD")
- `supports_cache_control`: Whether model supports prompt caching
- `metadata`: Additional custom metadata as key-value pairs

## Goals

- **Consistency**: Standardized model information across providers
- **Validation**: Ensure all models have proper metadata
- **Tracking**: Monitor when providers add/remove models
- **Selection**: Filter models by capability (e.g., exclude voice models from chat UI)
