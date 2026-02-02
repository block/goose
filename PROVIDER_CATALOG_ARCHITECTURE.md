# Provider Catalog Architecture

## Overview

The provider catalog system enables users to quickly set up custom providers by auto-populating provider configurations from curated metadata and canonical models.

## Data Sources

### 1. Canonical Models (`canonical_models.json`)
- **Location**: `crates/goose/src/providers/canonical/data/canonical_models.json`
- **Source**: Generated from models.dev via `build_canonical_models` binary
- **Content**: 472 models across 13 providers
- **Structure**: Flat array of model objects with:
  - `id`: Format `"provider/model"` (e.g., `"anthropic/claude-3-5-sonnet"`)
  - `name`: Human-readable name
  - `capabilities`: tool_call, reasoning, attachment, temperature
  - `limit`: context and output token limits
  - `cost`: pricing per million tokens
  - `modalities`: input/output types (text, image, audio, etc.)

### 2. Provider Metadata (`provider_metadata.json`)
- **Location**: `crates/goose/src/providers/canonical/data/provider_metadata.json`
- **Source**: Manually curated (can be derived from models.dev)
- **Content**: Metadata for 13 providers
- **Structure**: Array of provider objects with:
  - `id`: Provider identifier (matches prefix in canonical models)
  - `display_name`: User-facing name
  - `format`: API format ("openai", "anthropic", "ollama")
  - `api_url`: Base API endpoint URL
  - `doc_url`: Documentation URL
  - `env_var`: Environment variable name for API key
  - `supports_streaming`: Whether provider supports streaming
  - `requires_auth`: Whether provider requires authentication

## Supported Providers

Currently supported providers (from canonical models):
- anthropic (14 models)
- openai (37 models)
- google (26 models)
- google-vertex (20 models)
- openrouter (146 models)
- meta-llama (7 models)
- mistralai (19 models)
- x-ai (14 models)
- deepseek (2 models)
- cohere (7 models)
- azure (87 models)
- amazon-bedrock (67 models)
- venice (26 models)

## Catalog Service (`catalog.rs`)

### Functions

#### `get_providers_by_format(format: ProviderFormat) -> Vec<ProviderCatalogEntry>`
Returns list of providers filtered by API format (OpenAI/Anthropic/Ollama).

**Process**:
1. Load provider metadata
2. Filter by requested format
3. For each provider, count models from canonical registry
4. Skip providers with no models
5. Return sorted list with metadata + model count

#### `get_provider_template(provider_id: &str) -> Option<ProviderTemplate>`
Returns complete provider template for auto-filling custom provider form.

**Process**:
1. Load provider metadata by ID
2. Fetch all models for provider from canonical registry
3. Transform canonical models to template format
4. Return combined provider metadata + models

### Data Flow

```
┌─────────────────────────┐
│  provider_metadata.json │
│  (13 providers)         │
└───────────┬─────────────┘
            │
            │  Load at startup
            ▼
    ┌───────────────┐         ┌──────────────────────┐
    │ PROVIDER_     │         │  canonical_models    │
    │ METADATA      │◀────────│  .json               │
    │ HashMap       │  Join   │  (472 models)        │
    └───────┬───────┘  by ID  └──────────────────────┘
            │
            │  Filter by format
            │  Combine metadata + models
            ▼
    ┌───────────────┐
    │  Catalog API  │
    │  - /provider- │
    │    catalog    │
    │  - /provider- │
    │    catalog/{} │
    └───────────────┘
```

## API Endpoints

### GET `/config/provider-catalog?format=openai`
Returns list of providers filtered by format.

**Response**:
```json
[
  {
    "id": "anthropic",
    "name": "Anthropic",
    "format": "openai",
    "api_url": "https://api.anthropic.com/v1",
    "model_count": 14,
    "doc_url": "https://docs.anthropic.com",
    "env_var": "ANTHROPIC_API_KEY"
  }
]
```

### GET `/config/provider-catalog/{id}`
Returns complete provider template with models.

**Response**:
```json
{
  "id": "anthropic",
  "name": "Anthropic",
  "format": "anthropic",
  "api_url": "https://api.anthropic.com/v1",
  "models": [
    {
      "id": "claude-3-5-sonnet-20241022",
      "name": "Claude Sonnet 3.5 v2",
      "context_limit": 200000,
      "capabilities": {
        "tool_call": true,
        "reasoning": false,
        "attachment": true,
        "temperature": true
      },
      "deprecated": false
    }
  ],
  "supports_streaming": true,
  "env_var": "ANTHROPIC_API_KEY",
  "doc_url": "https://docs.anthropic.com"
}
```

## Adding New Providers

### Option 1: Manual Addition
1. Add provider entry to `provider_metadata.json`
2. Ensure canonical models exist for that provider
3. Rebuild and test

### Option 2: Derive from models.dev
1. Run `cargo run --bin build_canonical_models` to update canonical models
2. Extract provider metadata from models.dev:
   ```bash
   # Fetch models.dev
   curl -o models_dev.json https://models.dev/api.json

   # Extract provider info (example for a provider)
   jq '.groq | {
     id: "groq",
     display_name: .name,
     format: (if .npm | contains("openai") then "openai"
              elif .npm | contains("anthropic") then "anthropic"
              else "openai" end),
     api_url: .api,
     doc_url: .doc,
     env_var: .env[0],
     supports_streaming: true,
     requires_auth: true
   }' models_dev.json
   ```
3. Add to `provider_metadata.json`
4. Verify provider has models in canonical registry

## Benefits of This Approach

1. **Single Source of Truth**: Canonical models come from one place (models.dev)
2. **Small Metadata File**: Provider metadata is lightweight (< 5KB)
3. **Easy Maintenance**: Update canonical models via build script
4. **No External Deps**: All data embedded in binary
5. **Type Safety**: Rust structs ensure data consistency
6. **Curated Experience**: Only well-supported providers are exposed

## Future Enhancements

- Add deprecation tracking to canonical models
- Auto-detect format from canonical model capabilities
- Support for provider-specific authentication flows
- Connection testing before provider save
- Automatic refresh of autogenerated providers when canonical models update
