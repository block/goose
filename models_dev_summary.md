# models.dev API Data Summary

**Downloaded:** 2026-02-02
**Source:** https://models.dev/api.json
**File:** `models_dev_api.json` (980KB)

## Overview

**Total Providers:** 85
**Total Models:** ~2,800+ across all providers

## Provider Structure

Each provider in models.dev has:

```json
{
  "id": "groq",                    // Provider ID (used as key)
  "name": "Groq",                  // Display name
  "npm": "@ai-sdk/groq",           // AI SDK npm package
  "api": "https://api.groq.com",   // Base API URL
  "env": ["GROQ_API_KEY"],         // Environment variables needed
  "doc": "https://...",            // Documentation URL
  "models": {
    "model-id": {
      "id": "...",
      "name": "...",
      "family": "...",
      // ... model details
    }
  }
}
```

## Model Structure

Each model has rich metadata:

```json
{
  "id": "llama-3.1-8b-instant",
  "name": "Llama 3.1 8B Instant",
  "family": "llama",

  // Capabilities
  "attachment": false,             // File/image support
  "reasoning": false,              // Extended thinking
  "tool_call": true,               // Function calling
  "structured_output": true,       // JSON mode
  "temperature": true,             // Temperature parameter support

  // Metadata
  "knowledge": "2023-12",          // Knowledge cutoff
  "release_date": "2024-07-23",
  "last_updated": "2024-07-23",
  "status": "deprecated",          // Optional: deprecated/beta

  // Modalities
  "modalities": {
    "input": ["text"],
    "output": ["text"]
  },

  // Pricing (USD per 1M tokens)
  "cost": {
    "input": 0.05,
    "output": 0.08,
    "cache_read": 0.01,            // Optional
    "cache_write": 0.05            // Optional
  },

  // Limits
  "limit": {
    "context": 131072,
    "output": 131072
  },

  // Open source
  "open_weights": true
}
```

## NPM Package Distribution

**OpenAI-Compatible Dominance:**
- `@ai-sdk/openai-compatible`: **57 providers (67%)**
- All other packages: 28 providers (33%)

**Dedicated Provider Packages:**
- `@ai-sdk/anthropic`: 6 providers (Anthropic, MiniMax variants)
- `@ai-sdk/openai`: 2 providers (OpenAI, Vivgrid)
- `@ai-sdk/google`: 1 provider (Google)
- `@ai-sdk/google-vertex`: 1 provider (Google Vertex)
- `@ai-sdk/google-vertex/anthropic`: 1 provider (Claude on Vertex)
- `@ai-sdk/mistral`: 1 provider (Mistral)
- `@ai-sdk/cohere`: 1 provider (Cohere)
- `@ai-sdk/xai`: 1 provider (xAI)
- `@ai-sdk/groq`: 1 provider (Groq)
- `@ai-sdk/amazon-bedrock`: 1 provider (AWS Bedrock)
- `@ai-sdk/azure`: 2 providers (Azure, Azure Cognitive Services)
- `@ai-sdk/cerebras`: 1 provider (Cerebras)
- `@ai-sdk/deepinfra`: 1 provider (DeepInfra)
- `@ai-sdk/togetherai`: 1 provider (Together AI)
- `@ai-sdk/perplexity`: 1 provider (Perplexity)
- `@ai-sdk/vercel`: 1 provider (Vercel)
- `@ai-sdk/gateway`: 1 provider (Vercel AI Gateway)

**Custom Provider Packages:**
- `venice-ai-sdk-provider`: 1 provider (Venice)
- `workers-ai-provider`: 1 provider (Cloudflare Workers AI)
- `@gitlab/gitlab-ai-provider`: 1 provider (GitLab)
- `@jerome-benoit/sap-ai-provider-v2`: 1 provider (SAP AI Core)

## Major Providers in models.dev

**Large Model Catalogs:**
- `vercel`: 185 models (AI Gateway)
- `openrouter`: 148 models
- `poe`: 115 models
- `azure`: 93 models
- `helicone`: 91 models
- `azure-cognitive-services`: 91 models
- `novita-ai`: 79 models
- `siliconflow`: 74 models
- `nvidia`: 70 models
- `amazon-bedrock`: 67 models

**Major Providers with Dedicated Packages:**
- `anthropic`: 21 models
- `openai`: 40 models
- `google`: 26 models
- `mistral`: 26 models
- `groq`: 17 models
- `cohere`: 7 models

## Provider Categories

### 1. Model Aggregators/Routers
- OpenRouter (148 models)
- Vercel AI Gateway (185 models)
- Poe (115 models)
- Helicone (91 models)
- Abacus (55 models)

### 2. Cloud Provider Model Hosting
- Azure OpenAI (93 models)
- AWS Bedrock (67 models)
- Google Vertex AI (20 models)
- OVHCloud (13 models)
- Scaleway (14 models)

### 3. Model Providers (Original)
- OpenAI (40 models)
- Anthropic (21 models)
- Google (26 models)
- Mistral (26 models)
- xAI (22 models)
- Cohere (7 models)
- DeepSeek (2 models)

### 4. Inference Platforms
- Groq (17 models)
- Together AI (16 models)
- Cerebras (3 models)
- DeepInfra (10 models)
- Fireworks AI (17 models)

### 5. Model Hosts
- HuggingFace (16 models)
- Replicate (via inference endpoint)
- Baseten (6 models)
- Ollama Cloud (29 models)

### 6. Regional/Specialized
- Alibaba (39 models) - China
- Moonshot AI (6 models) - China, Kimi models
- Silicon Flow (74 models) - China
- ZhipuAI (8 models) - China, GLM models
- MiniMax (2 models) - China
- Venice (26 models) - Privacy-focused
- GitHub Copilot (19 models)
- GitHub Models (55 models)

## Capability Distribution

**Tool Calling Support:**
- Most modern models support `tool_call: true`
- Common across GPT-4, Claude 3+, Gemini, Llama 3+, Mistral

**Reasoning Models:**
- OpenAI: o1, o3, o4
- Anthropic: Claude 3.5 Sonnet/Opus (extended thinking)
- DeepSeek: deepseek-reasoner
- Some custom models: gpt-oss-120b

**Multimodal (Vision) Support:**
- `modalities.input: ["text", "image"]`
- GPT-4V, Claude 3+, Gemini, some Qwen models

**Audio/Video:**
- Audio: OpenAI Whisper, some GPT-4o variants
- Video: Amazon Nova, recent Gemini variants

**Structured Output:**
- Many recent models support JSON mode
- `structured_output: true` in models.dev

## Pricing Patterns

**Free Tier Models:**
- Many with `cost: { input: 0, output: 0 }`
- Typically open source models on local/donated infrastructure

**Budget Models ($0.05-0.20/M tokens):**
- Llama 3.1 8B: $0.05/$0.08
- Gemma 3 27B: $0.10/$0.10
- GPT-4o-mini: $0.15/$0.60

**Mid-tier Models ($0.50-3.00/M tokens):**
- Claude 3.5 Sonnet: $3.00/$15.00
- GPT-4o: $2.50/$10.00
- Gemini 2.0 Flash: $0.60/$2.40

**Premium Models ($5.00-15.00/M tokens):**
- Claude Opus 4: $15.00/$75.00
- GPT-4.1: ~$5.00+

**Cache Pricing:**
- Many providers now support cache_read/cache_write
- Typically 90% discount for cache reads
- Anthropic, OpenAI, some others

## What's Useful for Custom Provider Flow

### Auto-Configuration Data
1. **Base URL** - `api` field provides default endpoint
2. **Auth Requirements** - `env` field lists required API keys
3. **Documentation** - `doc` field links to provider docs
4. **API Format** - `npm` field indicates OpenAI/Anthropic/other compatibility

### Model Discovery
1. **Full Model List** - All available models per provider
2. **Context Limits** - `limit.context` for token limits
3. **Capabilities** - tool_call, reasoning, attachment, etc.
4. **Pricing** - input/output costs for budgeting

### Validation
1. **Model Names** - Validate user input against known models
2. **Deprecated Models** - `status: "deprecated"` flag
3. **Model Families** - Group similar models (llama, mistral, etc.)

### Smart Defaults
1. **Engine Detection** - Map npm package → Goose engine
2. **Context Limits** - Default from models.dev, allow override
3. **Capabilities** - Pre-populate supported features

## Missing from models.dev

**No OpenAI Responses API Indication:**
- No field for Completions vs Responses API support
- Can't tell which models require Responses API
- Would need `api_formats: ["completions", "responses"]`

**No Regional Endpoints:**
- Single URL per provider
- Azure, AWS Bedrock have regional variations not tracked

**No Rate Limits:**
- No requests/minute or RPM information

**No Model Versioning:**
- Some providers have versioned models (gpt-4-0613 vs gpt-4-turbo-2024-04-09)
- models.dev has latest version only

**No Authentication Methods:**
- Only env vars listed
- Doesn't capture OAuth, JWT, custom auth flows

## How to Use in Custom Provider Flow

### Step 1: Load at Build Time
```rust
// Embed models.dev JSON in binary
const MODELS_DEV_DATA: &str = include_str!("models_dev_api.json");

lazy_static! {
    static ref PROVIDER_CATALOG: HashMap<String, ProviderMetadata> = {
        parse_models_dev(MODELS_DEV_DATA).unwrap()
    };
}
```

### Step 2: Provider Discovery
```rust
pub fn search_providers(query: &str) -> Vec<ProviderSummary> {
    PROVIDER_CATALOG
        .values()
        .filter(|p| p.name.contains(query) || p.id.contains(query))
        .map(|p| ProviderSummary {
            id: p.id.clone(),
            name: p.name.clone(),
            model_count: p.models.len(),
            api_format: detect_format(&p.npm),
            doc_url: p.doc.clone(),
        })
        .collect()
}
```

### Step 3: Auto-Configuration
```rust
pub fn create_from_catalog(
    provider_id: &str,
    api_key: String,
    selected_models: Vec<String>,
) -> Result<DeclarativeProviderConfig> {
    let provider = PROVIDER_CATALOG.get(provider_id)?;

    let engine = match provider.npm.as_str() {
        "@ai-sdk/openai" | "@ai-sdk/openai-compatible" => ProviderEngine::OpenAI,
        "@ai-sdk/anthropic" => ProviderEngine::Anthropic,
        _ => ProviderEngine::OpenAI,
    };

    let models = selected_models
        .into_iter()
        .filter_map(|name| provider.models.get(&name))
        .map(|m| ModelInfo::new(
            m.id.clone(),
            m.limit.context,
        ))
        .collect();

    Ok(DeclarativeProviderConfig {
        name: format!("custom_{}", provider_id),
        engine,
        display_name: provider.name.clone(),
        base_url: provider.api.clone(),
        api_key_env: provider.env.first().unwrap_or(&"API_KEY".to_string()).clone(),
        models,
        // ...
    })
}
```

### Step 4: Model Enrichment
```rust
pub fn get_model_metadata(
    provider_id: &str,
    model_id: &str,
) -> Option<ModelMetadata> {
    let provider = PROVIDER_CATALOG.get(provider_id)?;
    let model = provider.models.get(model_id)?;

    Some(ModelMetadata {
        name: model.name.clone(),
        context_limit: model.limit.context,
        capabilities: ModelCapabilities {
            tool_call: model.tool_call,
            reasoning: model.reasoning,
            attachment: model.attachment,
            temperature: model.temperature,
        },
        pricing: model.cost.as_ref().map(|c| Pricing {
            input: c.input,
            output: c.output,
        }),
        modalities: model.modalities.clone(),
        deprecated: model.status.as_ref().map(|s| s == "deprecated").unwrap_or(false),
    })
}
```

## Recommendation

**Use models.dev as the source of truth for:**
- ✅ Known provider list (85 providers)
- ✅ Base URLs and auth requirements
- ✅ Model catalogs (2,800+ models)
- ✅ Model capabilities and pricing
- ✅ API format detection (via npm field)

**Supplement with:**
- ⚠️ Manual Responses API detection (not in models.dev)
- ⚠️ Regional endpoint configuration (for Azure, AWS)
- ⚠️ Custom auth methods (beyond API key)
- ⚠️ Rate limit information (from provider docs)

**Update strategy:**
- Bundle models.dev JSON at build time
- Refresh periodically (weekly/monthly)
- Allow manual override for custom/local providers
- Cache in-memory for fast lookups
