# Migration to Canonical Models-Based Catalog

## Overview

The provider catalog system has been migrated from using models.dev directly to deriving all data from canonical models locally.

## What Changed

### Before (models.dev direct)
- Embedded entire models.dev JSON (980KB) in binary
- Parsed models.dev structure at runtime
- Provider catalog derived from models.dev

### After (canonical models)
- Use canonical models as single source of truth
- Small provider metadata file (< 5KB)
- Catalog combines metadata + canonical models
- No dependency on models.dev at runtime

## Files Changed

### New Files
1. **`crates/goose/src/providers/canonical/data/provider_metadata.json`**
   - Contains provider-level metadata for 13 providers
   - Fields: id, display_name, format, api_url, doc_url, env_var, supports_streaming, requires_auth
   - Manually curated but can be derived from models.dev

2. **`scripts/add_provider_metadata.js`**
   - Helper script to extract provider metadata from models.dev
   - Usage: `node scripts/add_provider_metadata.js <provider_id>`
   - Generates JSON for adding to provider_metadata.json

3. **`PROVIDER_CATALOG_ARCHITECTURE.md`**
   - Complete documentation of catalog system
   - Data sources, API endpoints, adding new providers

### Modified Files
1. **`crates/goose/src/providers/catalog.rs`**
   - Removed models.dev JSON embedding
   - Now loads from provider_metadata.json + canonical models
   - `get_provider_models()` - fetches models for provider from canonical registry
   - `get_providers_by_format()` - combines metadata + model counts
   - `get_provider_template()` - combines metadata + full model list

### Deleted Files
1. **`models_dev_api.json`** (980KB) - No longer needed

## Data Flow

### Old Flow
```
models_dev_api.json (980KB)
  ↓ embedded in binary
  ↓ parsed at runtime
PROVIDER_CATALOG HashMap
  ↓
Catalog API
```

### New Flow
```
provider_metadata.json (5KB)  +  canonical_models.json (existing)
  ↓                                ↓
PROVIDER_METADATA HashMap     CanonicalModelRegistry
  ↓                                ↓
  └────────────┬───────────────────┘
               ↓
         Catalog API
         (combines metadata + models by provider ID)
```

## Benefits

1. **Smaller Binary**: Removed 980KB of embedded JSON
2. **Single Source of Truth**: All models come from canonical models
3. **Easier Maintenance**: Provider metadata is small and curated
4. **Consistency**: Same model data used for catalog and runtime lookups
5. **Flexibility**: Easy to add new providers via metadata file

## Provider Coverage

All 13 providers in canonical models are now available:
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

## Adding New Providers

### Method 1: Via Script
```bash
# Generate metadata for a new provider
node scripts/add_provider_metadata.js groq

# Copy output to provider_metadata.json
# Ensure provider exists in canonical models
cargo run --bin build_canonical_models

# Rebuild
cargo build
```

### Method 2: Manual
1. Add entry to `provider_metadata.json`:
   ```json
   {
     "id": "groq",
     "display_name": "Groq",
     "format": "openai",
     "api_url": "https://api.groq.com/openai/v1",
     "doc_url": "https://groq.com/docs",
     "env_var": "GROQ_API_KEY",
     "supports_streaming": true,
     "requires_auth": true
   }
   ```
2. Ensure canonical models exist for provider
3. Rebuild project

## Testing

```bash
# Run catalog tests
cargo test --package goose --lib providers::catalog::tests

# Build server
cargo build --package goose-server --bin goosed

# Test API endpoint
curl http://localhost:3000/config/provider-catalog?format=openai
curl http://localhost:3000/config/provider-catalog/anthropic
```

## Breaking Changes

None - API endpoints remain the same, just data source changed.

## Performance Impact

- **Binary Size**: -980KB (removed models_dev_api.json)
- **Startup Time**: No measurable difference (both use `include_str!`)
- **Runtime**: Slightly faster (fewer hops, simpler data structure)

## Future Work

- Add deprecation field to canonical models
- Auto-sync provider metadata from models.dev
- Support for provider aliases (e.g., "llama" → "meta-llama")
- Rich provider descriptions from models.dev
