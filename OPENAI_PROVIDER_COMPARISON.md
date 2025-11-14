# OpenAI Provider Implementation Comparison

## Overview
This document compares the goose OpenAI provider implementation against the official OpenAI Python SDK to identify gaps and prioritize development efforts.

**Last Updated:** 2025-01-13

---

## Architecture Comparison

### Official OpenAI Python SDK
- **Language:** Python
- **HTTP Client:** httpx (with aiohttp option)
- **Code Generation:** Auto-generated from OpenAPI spec using Stainless
- **Lines of Code:** ~66,000 lines across 767 files
- **Type Safety:** Comprehensive Pydantic models for all types
- **Async Support:** Full async/await support with AsyncOpenAI client
- **Resource Pattern:** Hierarchical resource organization (client.chat.completions.create)

### Goose OpenAI Provider
- **Language:** Rust
- **HTTP Client:** reqwest
- **Implementation:** Hand-coded
- **Lines of Code:** ~1,809 lines (openai.rs: 429, formats/openai.rs: 1,380)
- **Type Safety:** Rust types + serde_json::Value for API responses
- **Async Support:** Full async/await with tokio
- **Pattern:** Provider trait implementation

---

## Feature Comparison Matrix

### ✅ Implemented in Goose

| Feature | Status | Notes |
|---------|--------|-------|
| Chat Completions (streaming) | ✅ | Full support with SSE |
| Chat Completions (non-streaming) | ✅ | Complete |
| Tool/Function Calling | ✅ | Full support with proper error handling |
| Multi-tool requests | ✅ | Handles multiple tool calls in streaming |
| Vision (images) | ✅ | Supports image URLs and base64 |
| Embeddings | ✅ | text-embedding-3-small default |
| Model listing | ✅ | fetch_supported_models() |
| Custom headers | ✅ | OPENAI_CUSTOM_HEADERS support |
| Organization/Project headers | ✅ | OPENAI_ORGANIZATION, OPENAI_PROJECT |
| Custom base URL/host | ✅ | For OpenAI-compatible APIs |
| O-series models (o1, o3) | ✅ | Special handling for reasoning_effort, developer role |
| Retry logic | ✅ | Built-in retry with exponential backoff |
| Request logging | ✅ | RequestLog for debugging |
| Timeout configuration | ✅ | OPENAI_TIMEOUT (default: 600s) |
| Azure OpenAI | ✅ | Separate azure.rs provider |
| Error handling | ✅ | Comprehensive ProviderError types |
| Token usage tracking | ✅ | Input/output/total tokens |

### ❌ Missing from Goose

#### High Priority (Core Functionality)
| Feature | Impact | SDK Support |
|---------|--------|-------------|
| **Structured Outputs (JSON Schema)** | 🔴 High | ✅ Full support with response_format |
| **Responses API** | 🔴 High | ✅ New primary API (client.responses.create) |
| **Audio (Whisper)** | 🟡 Medium | ✅ Transcriptions & translations |
| **Text-to-Speech** | 🟡 Medium | ✅ client.audio.speech.create |
| **Batch API** | 🟡 Medium | ✅ client.batches.* |
| **Image Generation (DALL-E)** | 🟡 Medium | ✅ client.images.generate, edit, variations |
| **Video Generation (Sora)** | 🟡 Medium | ✅ client.videos.* (new) |

#### Medium Priority (Advanced Features)
| Feature | Impact | SDK Support |
|---------|--------|-------------|
| **Fine-tuning Management** | 🟡 Medium | ✅ client.fine_tuning.jobs.* |
| **Assistants API (Beta)** | 🟡 Medium | ✅ client.beta.assistants.* |
| **Vector Stores** | 🟡 Medium | ✅ client.beta.vector_stores.* |
| **Threads & Messages** | 🟡 Medium | ✅ client.beta.threads.* |
| **File Management** | 🟡 Medium | ✅ client.files.* |
| **Uploads API** | 🟡 Medium | ✅ client.uploads.* for large files |
| **Moderation API** | 🟢 Low | ✅ client.moderations.create |
| **Realtime API (WebSocket)** | 🟡 Medium | ✅ client.realtime.* |
| **Evals API** | 🟢 Low | ✅ client.evals.* |
| **Containers API** | 🟢 Low | ✅ client.containers.* |

#### Low Priority (SDK Features)
| Feature | Impact | SDK Support |
|---------|--------|-------------|
| **Pagination helpers** | 🟢 Low | ✅ SyncPage/AsyncPage |
| **Webhooks** | 🟢 Low | ✅ client.webhooks.* |
| **Raw response access** | 🟢 Low | ✅ with_raw_response() |
| **Response parsing helpers** | 🟢 Low | ✅ lib._parsing module |
| **CLI tool** | 🟢 Low | ✅ openai cli |

### 🔄 Implementation Differences

| Aspect | Goose | OpenAI SDK | Notes |
|--------|-------|------------|-------|
| **Streaming** | Manual SSE parsing | Built-in Stream objects | Both functional |
| **Error handling** | Rust Result types | Python exceptions | Different paradigms |
| **Retries** | with_retry() trait | Built-in retry logic | Both have retry |
| **Type safety** | Rust compile-time | Pydantic runtime | Rust stricter |
| **Config** | Environment vars | Constructor args | Different patterns |
| **Provider abstraction** | Trait system | Not needed | Goose multi-provider |

---

## Key Differences in Chat Completions

### Request Parameters

#### Goose Supports:
- ✅ model, messages, temperature, max_tokens
- ✅ tools (function calling)
- ✅ stream, stream_options
- ✅ O-series: reasoning_effort, max_completion_tokens, developer role
- ✅ Custom: toolshim for models without tool support

#### OpenAI SDK Also Supports:
- ❌ **response_format** (json_object, json_schema, text)
- ❌ **audio** (for multimodal audio input/output)
- ❌ **modalities** (text, audio, vision combinations)
- ❌ **prediction** (for prefilling assistant responses)
- ❌ **metadata** (custom key-value pairs)
- ❌ **store** (for Assistants API)
- ❌ **top_p** (nucleus sampling)
- ❌ **frequency_penalty** / **presence_penalty**
- ❌ **logprobs** (token log probabilities)
- ❌ **top_logprobs**
- ❌ **logit_bias** (token probability modification)
- ❌ **seed** (for deterministic outputs)
- ❌ **service_tier** (default, auto)
- ❌ **user** (end-user identifier)
- ❌ **parallel_tool_calls** (enable/disable)
- ❌ **tool_choice** (auto, required, none, or specific tool)

### Response Handling

#### Goose Supports:
- ✅ Text content
- ✅ Tool calls (with proper streaming)
- ✅ Usage data (tokens)
- ✅ Error content in tool calls
- ✅ Multiple tool calls in one response

#### OpenAI SDK Also Supports:
- ❌ **Audio output** (speech responses)
- ❌ **Refusal** (content policy refusals)
- ❌ **Finish reasons** (stop, length, tool_calls, content_filter, function_call)
- ❌ **Log probabilities** (per token)
- ❌ **System fingerprint** (for reproducibility)

---

## API Coverage by Endpoint

| Endpoint | Goose | OpenAI SDK | Priority |
|----------|-------|------------|----------|
| /chat/completions | ✅ Full | ✅ Full | Core |
| /embeddings | ✅ Basic | ✅ Full | High |
| /audio/transcriptions | ❌ | ✅ | High |
| /audio/translations | ❌ | ✅ | High |
| /audio/speech | ❌ | ✅ | High |
| /images/generations | ❌ | ✅ | Medium |
| /images/edits | ❌ | ✅ | Medium |
| /images/variations | ❌ | ✅ | Medium |
| /videos/* | ❌ | ✅ | Medium |
| /models | ✅ List | ✅ List/Get/Delete | Low |
| /moderations | ❌ | ✅ | Low |
| /fine_tuning/jobs | ❌ | ✅ | Medium |
| /files | ❌ | ✅ | Medium |
| /uploads/* | ❌ | ✅ | Low |
| /batches | ❌ | ✅ | Medium |
| /beta/assistants | ❌ | ✅ | Medium |
| /beta/threads | ❌ | ✅ | Medium |
| /beta/vector_stores | ❌ | ✅ | Medium |
| /realtime/* | ❌ | ✅ | Low |
| /responses/* | ❌ | ✅ | High |
| /evals/* | ❌ | ✅ | Low |
| /containers/* | ❌ | ✅ | Low |

---

## Recommendations: What to Focus On

### 🎯 Immediate Priorities (P0)

1. **Structured Outputs / JSON Schema**
   - **Why:** Critical for reliable tool outputs and structured data extraction
   - **Impact:** Enables schema validation, better reliability
   - **Effort:** Medium - add response_format parameter support
   - **Code:** Add to `create_request()` in formats/openai.rs

2. **Responses API**
   - **Why:** New primary API from OpenAI, replacing chat completions
   - **Impact:** Future-proofing, better developer experience
   - **Effort:** High - new API surface
   - **Code:** New module or extend existing provider

3. **Audio (Whisper) Transcription**
   - **Why:** Core functionality for multimodal applications
   - **Impact:** Enables voice input processing
   - **Effort:** Medium - file upload + API call
   - **Code:** New audio module in provider

4. **Missing Chat Completion Parameters**
   - **Priority:** top_p, frequency_penalty, presence_penalty, seed
   - **Why:** Common parameters for output control
   - **Impact:** Better control over generation
   - **Effort:** Low - just add to payload
   - **Code:** Extend `create_request()` in formats/openai.rs

### 🔄 Short Term (P1)

5. **Image Generation (DALL-E)**
   - **Why:** Popular feature for creative applications
   - **Impact:** Enables image generation workflows
   - **Effort:** Medium - new API endpoint
   - **Code:** New images module

6. **Batch API**
   - **Why:** Cost-effective processing of large workloads
   - **Impact:** Enables efficient bulk processing
   - **Effort:** Medium - async batch handling
   - **Code:** New batches module

7. **Enhanced Embeddings**
   - **Current:** Basic support with text-embedding-3-small
   - **Add:** Model selection, dimensions parameter, encoding_format
   - **Effort:** Low
   - **Code:** Extend embedding.rs

### 📦 Medium Term (P2)

8. **Text-to-Speech**
   - **Why:** Completes audio capabilities
   - **Impact:** Voice output
   - **Effort:** Low - simple API
   - **Code:** Extend audio module

9. **File Management**
   - **Why:** Required for fine-tuning and assistants
   - **Impact:** Enables advanced features
   - **Effort:** Medium
   - **Code:** New files module

10. **Moderation API**
    - **Why:** Content safety
    - **Impact:** Required for production apps
    - **Effort:** Low
    - **Code:** New moderations module

### 🔮 Long Term (P3)

11. **Assistants API (Beta)**
    - **Why:** Stateful conversations with memory
    - **Impact:** Advanced use cases
    - **Effort:** High - complex state management
    - **Code:** New beta/assistants module

12. **Fine-tuning Management**
    - **Why:** Model customization
    - **Impact:** Advanced use cases
    - **Effort:** Medium
    - **Code:** New fine_tuning module

13. **Vector Stores**
    - **Why:** RAG and semantic search
    - **Impact:** Knowledge base applications
    - **Effort:** High
    - **Code:** New vector_stores module

---

## Code Organization Recommendations

### Current Structure
```
crates/goose/src/providers/
├── openai.rs          (429 lines)  - Provider implementation
├── formats/
│   └── openai.rs      (1,380 lines) - Request/response formatting
├── embedding.rs       (24 lines)   - Trait definition
├── api_client.rs      (457 lines)  - HTTP client
└── base.rs            (675 lines)  - Provider trait
```

### Recommended Structure for Growth
```
crates/goose/src/providers/openai/
├── mod.rs                    - Re-exports
├── provider.rs               - Main OpenAiProvider impl
├── client.rs                 - HTTP client wrapper
├── completions/
│   ├── mod.rs
│   ├── chat.rs              - Chat completions
│   ├── streaming.rs         - Streaming logic
│   └── responses.rs         - New Responses API
├── embeddings.rs            - Embeddings API
├── audio/
│   ├── mod.rs
│   ├── transcriptions.rs   - Whisper
│   ├── translations.rs     - Translations
│   └── speech.rs           - TTS
├── images/
│   ├── mod.rs
│   ├── generations.rs
│   ├── edits.rs
│   └── variations.rs
├── batches.rs              - Batch API
├── files.rs                - File management
├── moderations.rs          - Moderation API
├── models.rs               - Model management
├── formats/
│   ├── mod.rs
│   ├── requests.rs         - Request builders
│   ├── responses.rs        - Response parsers
│   └── streaming.rs        - SSE parsing
└── types/
    ├── mod.rs
    ├── completions.rs
    ├── audio.rs
    ├── images.rs
    └── ...
```

---

## Specific Implementation Gaps

### 1. Structured Outputs (JSON Schema)

**Current:** No response_format support
**Needed:**
```rust
// Add to ModelConfig or request payload
pub struct ResponseFormat {
    pub type_: ResponseFormatType,
    pub json_schema: Option<JsonSchema>,
}

pub enum ResponseFormatType {
    Text,
    JsonObject,
    JsonSchema,
}
```

**Usage in SDK:**
```python
response = client.chat.completions.create(
    model="gpt-4o",
    messages=[{"role": "user", "content": "Extract: John is 30"}],
    response_format={
        "type": "json_schema",
        "json_schema": {
            "name": "person",
            "strict": True,
            "schema": {
                "type": "object",
                "properties": {
                    "name": {"type": "string"},
                    "age": {"type": "integer"}
                },
                "required": ["name", "age"]
            }
        }
    }
)
```

### 2. Missing Parameters

Add to `create_request()`:
```rust
// Currently missing:
pub struct ModelConfig {
    // ... existing fields ...
    pub top_p: Option<f32>,
    pub frequency_penalty: Option<f32>,
    pub presence_penalty: Option<f32>,
    pub seed: Option<i32>,
    pub logit_bias: Option<HashMap<String, f32>>,
    pub logprobs: Option<bool>,
    pub top_logprobs: Option<i32>,
    pub service_tier: Option<String>,
    pub user: Option<String>,
}
```

### 3. Tool Choice Control

**Current:** Tools are either provided or not
**Needed:**
```rust
pub enum ToolChoice {
    Auto,           // Let model decide
    Required,       // Must call a tool
    None,           // Don't call tools
    Specific(String), // Call specific tool
}
```

---

## Testing Gaps

### Current Testing
- ✅ Basic request/response parsing
- ✅ Tool call parsing
- ✅ Streaming multi-tool
- ✅ O-series model handling
- ✅ Error handling

### Missing Tests
- ❌ Structured outputs validation
- ❌ Audio parameter handling
- ❌ All chat completion parameters
- ❌ Moderation API
- ❌ Batch API
- ❌ Image generation
- ❌ File upload
- ❌ Retry behavior verification
- ❌ Rate limit handling
- ❌ Timeout behavior

---

## Performance Considerations

### Goose Advantages
- 🚀 Rust performance (memory safety, zero-cost abstractions)
- 🚀 Compiled binary (faster startup)
- 🚀 No GIL issues (true parallelism)
- 🚀 Lower memory footprint

### SDK Advantages
- 📦 Auto-generated (always up-to-date with API)
- 📦 Comprehensive type hints
- 📦 More helper utilities
- 📦 Larger ecosystem integration

---

## Migration Path for Users

If implementing parity, consider:

1. **Backward Compatibility:** Keep existing API stable
2. **Gradual Addition:** Add new features as optional
3. **Feature Flags:** Use Cargo features for optional endpoints
4. **Documentation:** Clear examples for each new feature
5. **Testing:** Comprehensive integration tests against OpenAI API

---

## Summary Statistics

| Metric | Goose | OpenAI SDK |
|--------|-------|------------|
| **Total Files** | 2 main | 767 |
| **Lines of Code** | ~1,809 | ~66,153 |
| **API Endpoints** | 2 | ~20+ |
| **Chat Params** | ~8 | ~30+ |
| **Response Types** | 3 | 15+ |
| **Test Coverage** | ~10 tests | ~100+ tests |
| **Feature Completeness** | ~30% | 100% |

---

## Conclusion

**Current State:** Goose has excellent coverage of core chat completion functionality with streaming, tool calling, and O-series model support. The implementation is solid and performant.

**Main Gaps:** Missing structured outputs (critical), Responses API (new primary API), audio APIs (whisper/TTS), and many advanced parameters.

**Recommended Focus:**
1. ⚡ **P0:** Structured outputs (JSON Schema) - critical for reliability
2. ⚡ **P0:** Responses API - future-proofing
3. ⚡ **P0:** Missing chat parameters (top_p, penalties, seed) - common needs
4. 🔄 **P1:** Audio transcription (Whisper) - multimodal applications
5. 🔄 **P1:** Image generation (DALL-E) - creative applications
6. 🔄 **P1:** Batch API - cost optimization

The goose implementation is well-architected for a provider abstraction layer. Adding full OpenAI parity would significantly expand the codebase but provide comprehensive API coverage. Consider prioritizing based on actual user needs rather than 100% API parity.
