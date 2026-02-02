# Custom Provider Flow Proposal: OpenAI-Compatible Providers

## Current State

### What Exists
- **Declarative Providers** (`declarative_providers.rs`): System to define custom providers via JSON
- **Fixed Providers**: Pre-configured providers (DeepSeek, Groq, Mistral, etc.) shipped with Goose
- **Custom Providers**: User-created providers stored in `~/.config/goose/custom_providers/`
- **Canonical Models Registry**: 1000+ models from models.dev with capabilities, pricing, limits
- **Three Engines**: OpenAI, Anthropic, Ollama compatibility layers

### Current Flow (Basic)
```
1. User provides:
   - engine: "openai_compatible" | "anthropic_compatible" | "ollama_compatible"
   - display_name: "My Provider"
   - api_url: "https://api.example.com"
   - api_key: "sk-..."
   - models: ["model-1", "model-2"]

2. System:
   - Generates ID from display_name (e.g., "custom_my_provider")
   - Stores API key in secure storage as {ID}_API_KEY
   - Writes JSON config to custom_providers/{id}.json
   - Registers provider with engine's format handler
```

### Pain Points
❌ Manual model list entry (tedious, error-prone)
❌ No validation that API URL works
❌ No auto-detection of provider capabilities
❌ Can't leverage models.dev data for known providers
❌ No guidance on which engine to use
❌ No model capability information (context limits, pricing, etc.)

---

## Improved Flow: Leveraging models.dev + Canonical Models

### Flow 1: Quick Setup (Known Provider from models.dev)

```
┌─────────────────────────────────────────────┐
│ Step 1: Provider Discovery                  │
│                                              │
│ Select provider setup method:                │
│ ○ Choose from known providers (recommended)  │
│ ○ Manual setup for custom provider          │
└─────────────────────────────────────────────┘
                    ↓
┌─────────────────────────────────────────────┐
│ Step 2: Provider Selection                  │
│                                              │
│ Search known providers:                      │
│ [groq____________]  🔍                       │
│                                              │
│ Results:                                     │
│ ┌───────────────────────────────────────┐  │
│ │ ✓ Groq                                 │  │
│ │   Fast inference with Groq hardware   │  │
│ │   API: api.groq.com                    │  │
│ │   Format: OpenAI Compatible            │  │
│ │   Models: 17 available                 │  │
│ └───────────────────────────────────────┘  │
│                                              │
│ [ Continue with Groq ]                       │
└─────────────────────────────────────────────┘
                    ↓
┌─────────────────────────────────────────────┐
│ Step 3: API Key & Validation                │
│                                              │
│ Provider: Groq                               │
│ Base URL: https://api.groq.com (auto-filled)│
│                                              │
│ API Key:                                     │
│ [sk-......................................]  │
│                                              │
│ Get your API key at:                         │
│ 🔗 https://console.groq.com/keys             │
│                                              │
│ [ Test Connection ]  [ Continue ]            │
└─────────────────────────────────────────────┘
                    ↓
┌─────────────────────────────────────────────┐
│ Step 4: Model Selection (Auto-populated)    │
│                                              │
│ Available models (from models.dev):          │
│                                              │
│ ☑ llama-3.1-8b-instant                      │
│   Context: 131K | Fast | Tool calling       │
│   $0.05/M in | $0.08/M out                  │
│                                              │
│ ☑ mixtral-8x7b-32768                        │
│   Context: 32K | Balanced | Tool calling    │
│   $0.24/M in | $0.24/M out                  │
│                                              │
│ ☐ llama-3.3-70b-versatile (deprecated)      │
│   Context: 131K | High quality              │
│   $0.59/M in | $0.79/M out                  │
│                                              │
│ Select All | Select None                     │
│                                              │
│ [ Add Custom Model ]  [ Continue ]           │
└─────────────────────────────────────────────┘
                    ↓
┌─────────────────────────────────────────────┐
│ Step 5: Confirmation                         │
│                                              │
│ ✓ Provider configured successfully!         │
│                                              │
│ Name: Groq                                   │
│ Format: OpenAI Compatible                    │
│ Models: 17 selected                          │
│                                              │
│ You can now use:                             │
│   --provider groq                            │
│   --model llama-3.1-8b-instant              │
│                                              │
│ [ Done ]  [ Add Another Provider ]           │
└─────────────────────────────────────────────┘
```

**Key Features:**
- ✅ Auto-detect provider from models.dev (86 providers available)
- ✅ Pre-fill base URL, env var names, documentation links
- ✅ Auto-populate models list with metadata (context limits, pricing, capabilities)
- ✅ Detect OpenAI/Anthropic/Ollama format from models.dev npm field
- ✅ Test API key before saving
- ✅ Show model capabilities: tool_call, reasoning, attachment, temperature, modalities

---

### Flow 2: Manual Setup (Unknown Provider)

```
┌─────────────────────────────────────────────┐
│ Step 1: Provider Details                    │
│                                              │
│ Display Name:                                │
│ [My Custom Provider____________]            │
│                                              │
│ Base URL:                                    │
│ [https://api.example.com_______]            │
│                                              │
│ API Format:                                  │
│ ○ OpenAI Compatible (recommended)            │
│ ○ Anthropic Compatible                       │
│ ○ Ollama Compatible                          │
│                                              │
│ [ Auto-detect Format ]  [ Continue ]         │
└─────────────────────────────────────────────┘
                    ↓
┌─────────────────────────────────────────────┐
│ Step 2: Authentication                       │
│                                              │
│ Requires API Key?                            │
│ ● Yes                                        │
│ ○ No (local/proxy)                          │
│                                              │
│ API Key:                                     │
│ [sk-......................................]  │
│                                              │
│ Custom Headers (optional):                   │
│ [ + Add Header ]                             │
│                                              │
│ [ Test Connection ]  [ Continue ]            │
└─────────────────────────────────────────────┘
                    ↓
┌─────────────────────────────────────────────┐
│ Step 3: Models                               │
│                                              │
│ ● Fetch models from API (if supported)      │
│ ○ Enter manually                            │
│                                              │
│ [ Fetch Models ]                             │
│                                              │
│ Discovered models:                           │
│ ☑ model-a (128K context)                    │
│ ☑ model-b (200K context)                    │
│                                              │
│ Or enter manually:                           │
│ [ + Add Model ]                              │
│                                              │
│ [ Continue ]                                 │
└─────────────────────────────────────────────┘
```

**Key Features:**
- ✅ Auto-detect API format by probing endpoints
- ✅ Fetch models list from `/v1/models` if supported
- ✅ Fallback to manual entry
- ✅ Test connection before saving

---

## Backend Implementation

### 1. Provider Discovery Service

```rust
pub struct ProviderDiscoveryService {
    models_dev_cache: HashMap<String, ModelsDevProvider>,
}

pub struct ModelsDevProvider {
    id: String,
    name: String,
    description: Option<String>,
    base_url: String,
    env_vars: Vec<String>,
    npm_package: String,  // e.g., "@ai-sdk/groq"
    doc_url: Option<String>,
    models: Vec<ModelsDevModel>,
}

impl ProviderDiscoveryService {
    // Load models.dev data at startup
    pub fn from_models_dev(json: &str) -> Result<Self>;

    // Search providers by name
    pub fn search(&self, query: &str) -> Vec<&ModelsDevProvider>;

    // Get provider details
    pub fn get(&self, id: &str) -> Option<&ModelsDevProvider>;

    // Convert to DeclarativeProviderConfig
    pub fn to_config(&self, id: &str, api_key: String) -> Result<DeclarativeProviderConfig>;
}
```

### 2. Engine Detection

Map models.dev `npm` field to Goose engine:

```rust
pub fn detect_engine(npm_package: &str) -> ProviderEngine {
    match npm_package {
        "@ai-sdk/openai" | "@ai-sdk/openai-compatible" => ProviderEngine::OpenAI,
        "@ai-sdk/anthropic" => ProviderEngine::Anthropic,
        // Ollama doesn't have npm package, but providers with "ollama" in name
        _ if npm_package.contains("ollama") => ProviderEngine::Ollama,
        _ => ProviderEngine::OpenAI, // Default to most common
    }
}
```

### 3. Model Metadata Enhancement

Enrich `ModelInfo` with canonical model data:

```rust
pub struct EnrichedModelInfo {
    pub name: String,
    pub context_limit: usize,

    // From canonical models
    pub family: Option<String>,
    pub capabilities: ModelCapabilities,
    pub pricing: Option<Pricing>,
    pub modalities: Option<Modalities>,
    pub knowledge_cutoff: Option<String>,
    pub deprecated: bool,
}

pub struct ModelCapabilities {
    pub tool_call: bool,
    pub reasoning: bool,
    pub attachment: bool,
    pub temperature: bool,
}
```

### 4. API Validation Service

```rust
pub struct ApiValidator {
    client: reqwest::Client,
}

impl ApiValidator {
    // Test if API key works
    pub async fn test_connection(
        &self,
        base_url: &str,
        api_key: &str,
        engine: ProviderEngine,
    ) -> Result<ValidationResult>;

    // Try to fetch models list
    pub async fn fetch_models(
        &self,
        base_url: &str,
        api_key: &str,
        engine: ProviderEngine,
    ) -> Result<Vec<String>>;

    // Auto-detect API format
    pub async fn detect_format(&self, base_url: &str) -> Result<ProviderEngine>;
}

pub struct ValidationResult {
    pub success: bool,
    pub message: String,
    pub detected_models: Option<Vec<String>>,
}
```

---

## Data Flow

### Known Provider (from models.dev)

```
User selects "Groq"
         ↓
ProviderDiscoveryService.get("groq")
         ↓
ModelsDevProvider {
    id: "groq",
    npm_package: "@ai-sdk/groq",
    base_url: "https://api.groq.com/openai/v1/chat/completions",
    env_vars: ["GROQ_API_KEY"],
    models: [/* 17 models from models.dev */]
}
         ↓
detect_engine("@ai-sdk/groq") → ProviderEngine::OpenAI
         ↓
For each model, lookup in CanonicalModelRegistry
         ↓
EnrichedModelInfo with capabilities, pricing, limits
         ↓
User enters API key → ApiValidator.test_connection()
         ↓
DeclarativeProviderConfig saved to custom_providers/groq.json
```

### Unknown Provider (manual)

```
User enters base_url + api_key
         ↓
ApiValidator.detect_format(base_url)
         ↓
Try: /v1/chat/completions (OpenAI)
     /v1/messages (Anthropic)
     /api/generate (Ollama)
         ↓
Detect ProviderEngine from response
         ↓
ApiValidator.fetch_models(base_url, api_key)
         ↓
Parse /v1/models response (if available)
         ↓
Create ModelInfo entries with detected context limits
         ↓
DeclarativeProviderConfig saved
```

---

## API Endpoints (for UI)

### GET /api/providers/discover
Returns list of known providers from models.dev

```json
{
  "providers": [
    {
      "id": "groq",
      "name": "Groq",
      "description": "Fast inference with Groq hardware",
      "api_format": "openai_compatible",
      "model_count": 17,
      "doc_url": "https://console.groq.com/docs/models"
    }
  ]
}
```

### GET /api/providers/discover/{id}
Returns full provider details with models

```json
{
  "id": "groq",
  "name": "Groq",
  "base_url": "https://api.groq.com/openai/v1/chat/completions",
  "api_format": "openai_compatible",
  "env_var": "GROQ_API_KEY",
  "models": [
    {
      "name": "llama-3.1-8b-instant",
      "context_limit": 131072,
      "capabilities": {
        "tool_call": true,
        "reasoning": false,
        "attachment": false
      },
      "pricing": {
        "input": 0.05,
        "output": 0.08,
        "currency": "USD"
      }
    }
  ]
}
```

### POST /api/providers/validate
Test API connection

```json
// Request
{
  "base_url": "https://api.groq.com/openai/v1/chat/completions",
  "api_key": "sk-...",
  "engine": "openai_compatible"
}

// Response
{
  "success": true,
  "message": "Connection successful",
  "detected_models": ["llama-3.1-8b-instant", "..."],
  "detected_format": "openai_compatible"
}
```

### POST /api/providers/custom
Create custom provider (existing endpoint, enhanced)

```json
// Request
{
  "source": "models_dev",  // NEW: "models_dev" | "manual"
  "provider_id": "groq",    // NEW: models.dev ID (if source=models_dev)
  "display_name": "Groq",
  "api_key": "sk-...",
  "selected_models": ["llama-3.1-8b-instant"],  // NEW: user selection

  // Manual fields (if source=manual)
  "engine": "openai_compatible",
  "api_url": "https://api.example.com",
  "custom_models": [...]
}
```

---

## UI Components

### 1. Provider Search/Browse
- Searchable list of 86 providers from models.dev
- Filter by API format (OpenAI, Anthropic, Ollama)
- Show provider cards with description, model count, documentation link

### 2. Provider Configuration Form
- Conditional fields based on source (models_dev vs manual)
- Auto-populate from models.dev when available
- Real-time validation with loading states
- Connection test before saving

### 3. Model Selection Grid
- Checkboxes for each model
- Model cards showing:
  - Name + context limit
  - Capabilities (icons for tool_call, reasoning, attachment)
  - Pricing (if available)
  - Deprecated badge
- "Select All" / "Select None" toggles
- Search/filter models

### 4. Connection Tester
- Visual feedback (spinner → success/error)
- Show detected models on success
- Clear error messages with suggestions

---

## Benefits

### For Known Providers (models.dev)
✅ **2-minute setup**: Select provider → Enter API key → Done
✅ **Zero manual entry**: Models, URLs, capabilities auto-populated
✅ **Validated config**: Test connection before saving
✅ **Rich metadata**: See pricing, capabilities, context limits
✅ **Stay updated**: Models list stays current with models.dev

### For Unknown Providers
✅ **Auto-detection**: Probe API to detect format
✅ **Model discovery**: Fetch available models from API
✅ **Flexible**: Support any OpenAI/Anthropic/Ollama-compatible API
✅ **Manual fallback**: Can still enter everything manually

### For All Providers
✅ **Canonical models integration**: Leverage 1000+ models metadata
✅ **Smart defaults**: Context limits, pricing, capabilities
✅ **Validation**: Test before saving
✅ **Extensible**: Easy to add new engines (e.g., Google Vertex, Azure)

---

## Implementation Phases

### Phase 1: Core Infrastructure (Backend)
1. ProviderDiscoveryService with models.dev parser
2. Engine detection from npm package field
3. ApiValidator with connection testing
4. Enhance DeclarativeProviderConfig with source field

### Phase 2: Model Metadata Enhancement
1. Integrate CanonicalModelRegistry lookup
2. EnrichedModelInfo with capabilities/pricing
3. Model status tracking (deprecated, beta)
4. Format detection (Completions vs Responses API)

### Phase 3: API Endpoints
1. `/api/providers/discover` - List providers
2. `/api/providers/discover/{id}` - Provider details
3. `/api/providers/validate` - Test connection
4. Enhanced `/api/providers/custom` - Create with source

### Phase 4: UI Components
1. Provider search/browse interface
2. Enhanced configuration form
3. Model selection grid with metadata
4. Connection tester component

### Phase 5: Advanced Features
1. Auto-fetch models from provider API
2. Format auto-detection (OpenAI vs Responses API)
3. Bulk import from config file
4. Provider templates/presets

---

## Open Questions

1. **Sync with models.dev**: How often to refresh? Bundle at build vs fetch at runtime?
2. **Custom model overrides**: Allow users to edit context limits for known models?
3. **Multi-region support**: Some providers (Azure, AWS) have regional endpoints
4. **Authentication methods**: Support OAuth, JWT, custom headers beyond API key?
5. **Model aliases**: Handle provider-specific model naming (e.g., "gpt-4" vs "gpt-4-0613")?
6. **Responses API detection**: Auto-detect which models need Responses vs Completions API?

---

## Recommendation: Start with Phase 1

**Minimum Viable Flow:**
1. Bundle models.dev JSON at build time
2. Provider search by name
3. Auto-populate base_url, models from models.dev
4. Simple connection test
5. Save as DeclarativeProviderConfig

**User Experience:**
```
goose provider add

? Select provider:
  > Groq (17 models, OpenAI compatible)
    DeepSeek (2 models, OpenAI compatible)
    Mistral (26 models, OpenAI compatible)
    [Enter custom provider]

? API Key: sk-...

✓ Testing connection... Success!
✓ Found 17 models

? Select models:
  ☑ llama-3.1-8b-instant (131K context)
  ☑ mixtral-8x7b-32768 (32K context)
  ☐ llama-3.3-70b-versatile (deprecated)

✓ Provider 'groq' configured!

Usage:
  goose --provider groq --model llama-3.1-8b-instant
```

This provides immediate value while being extensible for future phases.
