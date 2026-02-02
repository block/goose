# Provider to AI SDK Package Mapping Analysis

## Summary

This analysis covers **86 providers** from models.dev and their corresponding AI SDK npm packages.

## Key Findings

### 1. Provider-to-SDK Mapping is **NOT** 1-to-1

The mapping is typically 1-to-1 **per provider**, BUT multiple providers map to the same SDK package. Here's the breakdown:

### 2. SDK Package Distribution

| SDK Package | Provider Count | Notes |
|-------------|---------------|-------|
| `@ai-sdk/openai-compatible` | 58 | Most common - generic OpenAI-compatible wrapper |
| `@ai-sdk/anthropic` | 5 | Anthropic Claude API format |
| `@ai-sdk/openai` | 2 | Official OpenAI API |
| `@ai-sdk/google` | 1 | Google Gemini/GenAI |
| `@ai-sdk/google-vertex` | 2 | Google Vertex AI |
| `@ai-sdk/azure` | 2 | Azure OpenAI |
| `@ai-sdk/amazon-bedrock` | 1 | AWS Bedrock |
| `@ai-sdk/xai` | 1 | xAI Grok |
| `@ai-sdk/cohere` | 1 | Cohere |
| `@ai-sdk/mistral` | 1 | Mistral AI |
| `@ai-sdk/groq` | 1 | Groq |
| `@ai-sdk/togetherai` | 1 | Together AI |
| `@ai-sdk/perplexity` | 1 | Perplexity |
| `@ai-sdk/deepinfra` | 1 | DeepInfra |
| `@ai-sdk/cerebras` | 1 | Cerebras |
| `@ai-sdk/vercel` | 1 | Vercel AI |
| `@ai-sdk/gateway` | 1 | Vercel AI Gateway |
| `workers-ai-provider` | 1 | Cloudflare Workers AI |
| `venice-ai-sdk-provider` | 1 | Venice AI (custom) |
| `@jerome-benoit/sap-ai-provider-v2` | 1 | SAP AI Core (custom) |
| `@gitlab/gitlab-ai-provider` | 1 | GitLab AI (custom) |

### 3. The OpenAI-Compatible Dominance

**67% of providers (58/86)** use `@ai-sdk/openai-compatible`, indicating that OpenAI's API format has become the de facto industry standard for LLM APIs.

Providers using OpenAI-compatible include:
- Aggregators: OpenRouter, Helicone, Abacus, Fastrouter, Zenmux, Poe
- Cloud providers: Vultr, Scaleway, OVHCloud, Nebius
- Model hosts: HuggingFace, GitHub Models, Baseten, IO.net
- Regional providers: Alibaba, Moonshot AI, Silicon Flow, ZhipuAI
- Specialized: GitHub Copilot, LM Studio, Nvidia NIM

### 4. Dedicated SDK Packages

Major providers with their own API formats get dedicated packages:
- Anthropic → `@ai-sdk/anthropic`
- Google → `@ai-sdk/google` or `@ai-sdk/google-vertex`
- OpenAI → `@ai-sdk/openai`
- Mistral → `@ai-sdk/mistral`
- Cohere → `@ai-sdk/cohere`
- xAI → `@ai-sdk/xai`

### 5. Interesting Edge Cases

**DeepSeek anomaly**:
- `deepseek` provider uses `@ai-sdk/openai-compatible`
- But there exists a dedicated `@ai-sdk/deepseek` package
- This suggests DeepSeek implements OpenAI-compatible API

**MiniMax uses Anthropic SDK**:
- `minimax`, `minimax-cn`, `minimax-coding-plan`, `kimi-for-coding` all use `@ai-sdk/anthropic`
- Suggests MiniMax implements Anthropic's API format

**Vivgrid uses OpenAI SDK directly**:
- One of only 2 providers using `@ai-sdk/openai` (besides actual OpenAI)
- Suggests full OpenAI API compatibility, not just compatible

**Custom provider packages**:
- Venice, SAP AI Core, GitLab, Cloudflare Workers AI have custom packages
- These are likely maintained by the respective organizations

### 6. Per-Model Differences?

Based on the data, **there are NO per-model differences** in SDK package requirements. Each provider consistently uses the same SDK package across all its models. The mapping is:

```
1 Provider → 1 SDK Package → N Models
```

Not:
```
1 Provider → N Models → Multiple SDK Packages
```

### 7. Implications for Custom Providers

For custom provider support, you need to determine:

1. **Does the provider implement OpenAI-compatible API?**
   - Yes → Use `@ai-sdk/openai-compatible`
   - Examples: Most smaller providers, aggregators, model hosts

2. **Does the provider have a dedicated AI SDK package?**
   - Check if `@ai-sdk/{provider}` exists on npm
   - Examples: Anthropic, Google, Mistral, Cohere, xAI, Groq

3. **Does the provider use a custom SDK package?**
   - Check models.dev or provider docs
   - Examples: Venice, SAP, GitLab, Cloudflare Workers

4. **Is it a major provider not in models.dev?**
   - Likely has dedicated package: `@ai-sdk/{provider}`

### 8. Format Correlation

From Goose's internal format system:

| Goose Format | AI SDK Package(s) |
|--------------|-------------------|
| `openai.rs` | `@ai-sdk/openai`, `@ai-sdk/openai-compatible` |
| `anthropic.rs` | `@ai-sdk/anthropic` |
| `google.rs` | `@ai-sdk/google`, `@ai-sdk/google-vertex` |
| `bedrock.rs` | `@ai-sdk/amazon-bedrock` |
| `databricks.rs` | No AI SDK equivalent |
| `snowflake.rs` | No AI SDK equivalent |
| `gcpvertexai.rs` | `@ai-sdk/google-vertex` |
| `openrouter.rs` | `@ai-sdk/openai-compatible` |
| `openai_responses.rs` | No AI SDK equivalent (yet) |

### 9. Missing from models.dev

These major providers have AI SDK packages but aren't listed in models.dev with npm fields:
- `@ai-sdk/fireworks` (fireworks-ai uses openai-compatible instead)
- `@ai-sdk/replicate`
- `@ai-sdk/baseten` (baseten uses openai-compatible instead)
- `@ai-sdk/huggingface` (huggingface uses openai-compatible instead)

This suggests models.dev may be incomplete or these providers' AI SDK packages aren't their primary integration method.

### 10. Recommendation for Custom Providers

**Decision tree:**

```
Is provider OpenAI API compatible?
├─ Yes → @ai-sdk/openai-compatible
└─ No
   ├─ Is provider Anthropic API compatible? → @ai-sdk/anthropic
   ├─ Is provider Google API compatible? → @ai-sdk/google
   ├─ Does @ai-sdk/{provider} exist on npm? → Use it
   └─ No standard format → Create custom provider or use Goose's format system
```

**For Goose's custom provider feature:**
- Let users specify `ai_sdk_package` field
- Default to `@ai-sdk/openai-compatible` for unknown providers
- Validate package exists on npm
- Map AI SDK package → Goose format module for internal handling
