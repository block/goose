# Canonical Model System

This directory contains the canonical model infrastructure for standardizing model metadata across different providers.

## Overview

The canonical model system provides a unified view of model metadata across different LLM providers. It's based on OpenRouter's model schema and maintained through automated fetching:

1. **Canonical registry** - Models with standardized metadata from OpenRouter
2. **Provider mapping layer** - Maps provider-specific model names to canonical models
3. **Automated updates** - Rust script to re-fetch models from OpenRouter's API

## Components

### `model.rs`

Defines the core data structures based on OpenRouter's schema:

- `CanonicalModel`: Complete model metadata including:
  - IDs: `id` (API identifier), `canonical_slug` (standardized reference)
  - Basic info: name, description, context_length
  - `Architecture`: Modality, input/output types, tokenizer info
  - `Pricing`: Per-token costs (prompt, completion, cache read/write)
  - `TopProvider`: Context limits and moderation status
  - Supported parameters (for tools, temperature, etc.)

### `registry.rs`

Provides model registry management:

- `CanonicalModelRegistry`: In-memory registry of canonical models
- Load/save from/to JSON files
- Query and lookup operations using `canonical_slug`

### `canonical_models.json`

JSON file containing canonical model definitions fetched from OpenRouter. Currently includes models from:
- anthropic
- google
- openai

## Updating Models

To refresh the canonical models from OpenRouter's API:

```bash
cargo run --example build_canonical_models
```

This script:
1. Fetches all models from OpenRouter's `/models` endpoint
2. Transforms the data into our canonical format:
   - Strips version suffixes while preserving model family versions
   - Detects tool support from `supported_parameters`
   - Converts pricing strings to numeric types
3. Filters to only allowed providers (anthropic, google, openai)
4. Generates a new `canonical_models.json` file

The generated files will be written to:
```
crates/goose/src/providers/canonical/data/canonical_models.json
crates/goose/src/providers/canonical/data/report_YYYYMMDD_HHMMSS.json
```

## Usage

### For Provider Implementors

Implement the `map_to_canonical_models()` method in your provider to map provider-specific model IDs to canonical slugs:

```rust
async fn map_to_canonical_models(&self) -> Result<Vec<ModelMapping>, ProviderError> {
    let models = self.fetch_supported_models().await?
        .unwrap_or_default();

    let mut mappings = Vec::new();

    for model in models {
        // Map your provider's model ID to canonical slug
        let canonical_slug = match model.as_str() {
            "claude-3-5-sonnet-20241022" => "anthropic/claude-3-5-sonnet",
            "claude-3-5-sonnet-20250219" => "anthropic/claude-3-7-sonnet-20250219",
            "gpt-4-turbo-2024-04-09" => "openai/gpt-4-turbo",
            "gemini-1.5-pro" => "google/gemini-1-5-pro",
            // ... more mappings
            _ => continue, // Skip unmapped models
        };

        mappings.push(
            ModelMapping::new(model, canonical_slug).verified()
        );
    }

    Ok(mappings)
}
```

### Using Canonical Models

```rust
use goose::providers::canonical::CanonicalModelRegistry;

// Load the registry
let registry = CanonicalModelRegistry::from_file("canonical_models.json")?;

// Look up a model by canonical slug
if let Some(model) = registry.get("anthropic/claude-3-5-sonnet") {
    println!("Model: {}", model.name);
    println!("Context: {} tokens", model.context_length);
    println!("Supports tools: {}", model.supports_tools());
    println!("Supports vision: {}", model.supports_vision());

    if let (Some(prompt_cost), Some(completion_cost)) =
        (model.prompt_cost(), model.completion_cost()) {
        println!("Pricing: ${}/1M prompt, ${}/1M completion",
            prompt_cost * 1_000_000.0,
            completion_cost * 1_000_000.0);
    }
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

### Adding New Providers

To add support for a new provider (e.g., "mistral"):

1. Update the `ALLOWED_PROVIDERS` list in `examples/build_canonical_models.rs`:
   ```rust
   const ALLOWED_PROVIDERS: &[&str] = &["anthropic", "google", "openai", "mistral"];
   ```

2. Re-run the build script:
   ```bash
   cargo run --example build_canonical_models
   ```

## Model Metadata Fields

Fields from OpenRouter's schema:

- **`id`**: OpenRouter's API identifier (e.g., "anthropic/claude-sonnet-4.5")
- **`canonical_slug`**: Standardized reference with version (e.g., "anthropic/claude-4.5-sonnet-20250929")
  - This is our primary key for model lookups
- **`name`**: Human-readable name
- **`created`**: Unix timestamp of model creation
- **`description`**: Detailed model description
- **`context_length`**: Maximum context window in tokens
- **`architecture`**: Modality info, input/output types, tokenizer
- **`pricing`**: Per-token costs in USD (prompt, completion, cache read/write, images)
- **`top_provider`**: Context limits and moderation status from best provider
- **`supported_parameters`**: List of supported parameters (tools, temperature, etc.)

Helper methods on `CanonicalModel`:
- `supports_tools()` - Check if model supports tool/function calling
- `supports_vision()` - Check if model supports image inputs
- `supports_cache()` - Check if model supports prompt caching
- `provider()` - Extract provider name from canonical slug
- `prompt_cost()` / `completion_cost()` - Parse pricing as f64

## Goals

- **Consistency**: Standardized model information across providers
- **Validation**: Ensure all models have proper metadata
- **Tracking**: Monitor when providers add/remove models
- **Selection**: Filter models by capability (e.g., exclude voice models from chat UI)
