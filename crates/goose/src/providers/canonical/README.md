# Canonical Model System

Provides a unified view of model metadata (pricing, capabilities, context limits) across different LLM providers. 
Normalizes provider-specific model names (e.g., `claude-3-5-sonnet-20241022`) 
to canonical IDs (e.g., `anthropic/claude-3.5-sonnet`).

## Scripts

### Build Canonical Models
Fetches latest model metadata from OpenRouter and updates the registry:
```bash
cargo run --example build_canonical_models
```
Writes to: `src/providers/canonical/data/canonical_models.json`

### Check Model Mappings
Tests provider model mappings and tracks changes over time:
```bash
cargo run --example canonical_model_checker
```
- Reports unmapped models
- Compares with previous runs (like a lock file)
- Shows changed/added/removed mappings
- Writes to: `src/providers/canonical/data/canonical_mapping_report.json`
