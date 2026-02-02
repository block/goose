# Provider Format Compatibility Breakdown

## Anthropic-Compatible Providers (6 total)

Uses `@ai-sdk/anthropic` SDK package:

1. **anthropic** - Official Anthropic provider
   - Models: claude-opus-4-0, claude-3-5-sonnet-20241022, claude-opus-4-1
   - 21 models total

2. **kimi-for-coding** - Moonshot AI coding variant
   - Models: k2p5, kimi-k2-thinking
   - 2 models total

3. **minimax** - MiniMax AI
   - Models: MiniMax-M2, MiniMax-M2.1
   - 2 models total

4. **minimax-cn** - MiniMax China
   - Models: MiniMax-M2.1, MiniMax-M2
   - 2 models total

5. **minimax-coding-plan** - MiniMax coding variant
   - Models: MiniMax-M2, MiniMax-M2.1
   - 2 models total

6. **minimax-cn-coding-plan** - MiniMax China coding variant
   - Models: MiniMax-M2, MiniMax-M2.1
   - 2 models total

**Key Insight**: All MiniMax variants implement Anthropic's API format, suggesting they chose Claude's API as their compatibility standard rather than OpenAI's.

---

## Google Vertex AI Compatible Providers (3 total)

### Using `@ai-sdk/google-vertex`:

1. **google-vertex** - Google Vertex AI
   - Models: gemini-embedding-001, gemini-3-flash-preview, gemini-2.5-flash-preview-05-20
   - 20 models total

2. **google-vertex-anthropic** - Anthropic models on Vertex AI
   - Models: claude-opus-4-5@20251101, claude-3-5-sonnet@20241022, claude-3-5-haiku@20241022
   - 9 models total
   - Note: Uses Google Vertex SDK despite hosting Anthropic models

### Using `@ai-sdk/google`:

3. **google** - Google Generative AI (direct API)
   - Models: gemini-embedding-001, gemini-3-flash-preview, gemini-2.5-flash-image
   - 26 models total

**Key Insight**:
- `@ai-sdk/google` = Direct Google Generative AI API (consumer/simple access)
- `@ai-sdk/google-vertex` = Google Cloud Vertex AI (enterprise/GCP access)
- Even when Vertex hosts Anthropic models, it uses the Vertex SDK (not Anthropic SDK)

---

## Azure OpenAI Compatible Providers (2 total)

Uses `@ai-sdk/azure` SDK package:

1. **azure** - Azure OpenAI Service
   - Models: gpt-4.1-nano, text-embedding-3-small, grok-4-fast-non-reasoning
   - 93 models total

2. **azure-cognitive-services** - Azure Cognitive Services
   - Models: gpt-3.5-turbo-1106, mistral-small-2503, codestral-2501
   - 91 models total

**Key Insight**: Azure has two provider entries, likely representing different deployment/access patterns within Azure's ecosystem.

---

## OpenAI vs OpenAI-Compatible

### Using `@ai-sdk/openai` (2 providers):

1. **openai** - Official OpenAI
   - Models: gpt-4.1-nano, text-embedding-3-small, gpt-4
   - 40 models total

2. **vivgrid** - Vivgrid LLM Gateway
   - Models: gemini-3-flash-preview, gpt-5.2-codex, gpt-5.1-codex
   - 5 models total

### Why Vivgrid uses `@ai-sdk/openai` instead of `@ai-sdk/openai-compatible`:

**Vivgrid** is an LLM gateway service (similar to LiteLLM, Portkey) that:
- Provides a managed routing layer for multiple LLM providers
- Uses the full OpenAI API specification (not just compatible subset)
- Acts as a proxy/gateway for actual OpenAI endpoints plus others
- Allows developers to use "managed" model identifiers instead of binding to specific providers
- Uses OpenAI-format base URL: `https://api.vivgrid.com/v1`

**The distinction:**
- `@ai-sdk/openai` = Full OpenAI API implementation (official or high-fidelity proxy)
- `@ai-sdk/openai-compatible` = Implements OpenAI's API format but with possible limitations/differences

Vivgrid likely uses `@ai-sdk/openai` because:
1. It provides full OpenAI API compatibility (not just partial)
2. It acts as a transparent proxy to actual OpenAI models
3. It supports the complete OpenAI feature set (function calling, vision, etc.)
4. It wants clients to treat it as a drop-in OpenAI replacement

---

## Summary Stats

| Format | Provider Count | Notes |
|--------|---------------|-------|
| OpenAI-Compatible | 58 | Industry standard for custom APIs |
| Anthropic | 6 | Official + MiniMax variants |
| Google Vertex | 2 | Enterprise GCP access |
| Google | 1 | Direct consumer API |
| Azure | 2 | Microsoft cloud variants |
| OpenAI (full) | 2 | Official + high-fidelity gateway |

---

## References

- [Vivgrid LLM Gateway Overview](https://www.vivgrid.com/)
- [Making AI Agent Configurations Stable with an LLM Gateway](https://dev.to/palapalapala/making-ai-agent-configurations-stable-with-an-llm-gateway-2jf1)
- [How to Power Clawdbot with Advanced LLMs Using Vivgrid](https://medium.com/@pala_28493/how-to-power-clawdbot-with-advanced-llms-using-vivgrid-step-by-step-8bc17c6eebfc)
