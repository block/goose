# OpenAI Responses API vs Chat Completions API: Model Support Analysis

## The Problem

OpenAI recently introduced the **Responses API** as a successor to the **Chat Completions API**. Some newer models ONLY support Responses API, while others support both. However, **models.dev does NOT track which API format(s) each model supports**, making it difficult to determine compatibility.

## Current State in Goose

### Goose's Detection Logic

In `/crates/goose/src/providers/openai.rs:202-204`:

```rust
fn uses_responses_api(model_name: &str) -> bool {
    model_name.starts_with("gpt-5-codex") || model_name.starts_with("gpt-5.1-codex")
}
```

**This is incomplete** - it only catches `gpt-5-codex` and `gpt-5.1-codex`, missing other Responses-only models.

### Goose's Format Support

Goose has **two** OpenAI format modules:
1. **`openai.rs`** - Chat Completions API (legacy/current)
2. **`openai_responses.rs`** - Responses API (new)

Providers that use Responses API:
- **`chatgpt_codex.rs`** - Uses Responses API for gpt-5.1-codex, gpt-5.2-codex models

## Known Model Support

Based on OpenAI documentation and research:

### Responses API ONLY (Confirmed)

1. **gpt-5.2-codex** - Codex/coding-optimized model
   - Only available via Responses API
   - Supports reasoning effort settings (low, medium, high, xhigh)
   - Powers Codex CLI

### Likely Responses API Preferred/Only

2. **o1, o3, o4 models** - Reasoning models
   - Listed in Goose's known models
   - Work better with Responses API (3% improvement on SWE-bench)
   - Unclear if they support Chat Completions at all

3. **gpt-5 models** - Latest generation
   - Used in Responses API examples throughout docs
   - Status unclear for Chat Completions support

### Support Both APIs

4. **gpt-4o, gpt-4o-mini, gpt-4.1, gpt-4.1-mini** - General models
   - Known to work with Chat Completions
   - Likely work with Responses API too
   - Responses API provides benefits (caching, reasoning)

5. **gpt-3.5-turbo, gpt-4-turbo** - Legacy models
   - Work with Chat Completions (primary)
   - Unknown Responses API support

## Responses API Benefits

According to OpenAI documentation:

1. **Better reasoning** - 3% improvement on SWE-bench for reasoning models
2. **Agentic loop** - Model can call multiple tools in one request
3. **40-80% better caching** - Improved cache utilization vs Chat Completions
4. **State management** - `store: true` preserves reasoning/tool context turn-to-turn
5. **Future-proof** - Assistants API being sunset mid-2026, features moving to Responses

## What's Missing from models.dev

models.dev currently has **no fields** to indicate:
- ❌ `api_version` - Which API version (v1/completions, v1/responses)
- ❌ `api_format` - completions vs responses
- ❌ `responses_only` - Boolean flag for Responses-only models
- ❌ `supports_chat_completions` - Boolean flag
- ❌ `supports_responses_api` - Boolean flag

All models share the same field structure:
```json
{
  "id": "...",
  "name": "...",
  "family": "...",
  "attachment": bool,
  "reasoning": bool,
  "tool_call": bool,
  "temperature": bool,
  "modalities": {...},
  "cost": {...},
  "limit": {...}
}
```

## Detection Heuristics

Without explicit API format tracking, we need heuristics:

### Pattern-Based Detection (Goose's Current Approach)

```rust
fn uses_responses_api(model_name: &str) -> bool {
    // Current (incomplete)
    model_name.starts_with("gpt-5-codex")
    || model_name.starts_with("gpt-5.1-codex")

    // Should add:
    || model_name == "gpt-5.2-codex"
    || model_name.starts_with("o1")
    || model_name.starts_with("o3")
    || model_name.starts_with("o4")
    || model_name.starts_with("gpt-5") && !model_name.contains("turbo")
}
```

### Capability-Based Detection

Models with `reasoning: true` in models.dev might correlate with Responses API preference, but this is **not reliable** since:
- Not all Responses API models have reasoning
- Some reasoning models might support both APIs

### Version-Based Detection

Models released after a certain date (e.g., late 2025) likely prefer/require Responses API, but this requires:
- `release_date` tracking in models.dev (exists!)
- Known cutoff date (unclear)

## Recommendations

### For models.dev

Add API format tracking fields:

```json
{
  "api_formats": ["chat_completions", "responses"],
  "primary_api_format": "responses",
  "responses_only": true
}
```

or simpler:

```json
{
  "api_compatibility": {
    "chat_completions": true,
    "responses": true,
    "recommended": "responses"
  }
}
```

### For Goose

1. **Expand detection logic** - Add more model patterns to `uses_responses_api()`
2. **Add capability flag** - Track `supports_responses_api` in canonical models
3. **Default to Responses** - For unknown models, try Responses API first (it's a superset)
4. **Fallback gracefully** - If Responses fails with 404/unsupported, retry with Chat Completions

### For Custom Providers

When adding custom providers that use OpenAI format, need to specify:
- Which API format they implement (completions vs responses)
- Whether they support both
- Which is preferred

## Decision Tree for API Format Selection

```
Is model gpt-5.2-codex?
├─ Yes → Responses API ONLY
└─ No

Is model o1/o3/o4?
├─ Yes → Prefer Responses API (TBD if Chat Completions works)
└─ No

Is model gpt-5.x (not turbo)?
├─ Yes → Prefer Responses API
└─ No

Is model gpt-4.x, gpt-4o, gpt-3.5-turbo?
├─ Yes → Both work, prefer Responses for caching/reasoning benefits
└─ No

Unknown model → Try Responses first, fallback to Chat Completions
```

## Provider-Specific Notes

### ChatGPT Codex Provider
- Uses Responses API exclusively
- Endpoint: `https://chatgpt.com/backend-api/conversation/agentic/v1/responses`
- Models: gpt-5.2-codex, gpt-5.1-codex, gpt-5.1-codex-mini, gpt-5.1-codex-max

### OpenAI Provider
- Conditionally uses both APIs based on `uses_responses_api()` check
- Responses endpoint: `v1/responses`
- Completions endpoint: `v1/chat/completions`

### GitHub Copilot Provider
- Lists gpt-5 models but uses OpenAI format (likely Chat Completions)
- May need Responses API support added

## Open Questions

1. **Do o1/o3/o4 work with Chat Completions at all?**
   - Documentation unclear
   - Needs testing

2. **Do all gpt-5 variants require Responses?**
   - gpt-5.2-codex: YES (confirmed)
   - gpt-5.1-codex: Likely yes (used with Responses in Goose)
   - gpt-5, gpt-5-mini, gpt-5-codex: Unknown

3. **When did the cutover happen?**
   - Need a clear date/version where new models became Responses-only

4. **Does Azure OpenAI support Responses API?**
   - Azure provider uses `@ai-sdk/azure`
   - Azure docs mention Responses API support
   - Unclear which models

5. **What about third-party providers?**
   - Do OpenRouter, Helicone, etc. support Responses API format?
   - Most likely just proxy Chat Completions

## References

- [OpenAI Responses API Migration Guide](https://platform.openai.com/docs/guides/migrate-to-responses)
- [Responses API vs Chat Completions Comparison](https://platform.openai.com/docs/guides/responses-vs-chat-completions)
- [Using GPT-5.2 Documentation](https://platform.openai.com/docs/guides/latest-model)
- [OpenAI's API Evolution: Responses API vs. Chat Completions API (Medium)](https://medium.com/@praveenkumarsingh/openais-api-evolution-responses-api-vs-chat-completions-api-0463d73ce631)
- [Azure OpenAI Responses API Documentation](https://learn.microsoft.com/en-us/azure/ai-foundry/openai/how-to/responses?view=foundry-classic)
