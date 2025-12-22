# Amazon Bedrock Provider Improvements

## Recent Improvements (Completed)

### Phase 1: Core Functionality - COMPLETED

1. **Streaming Support** - DONE (Commit 7ab3395b65)
   - Implemented streaming API using stream() method
   - Returns MessageStream using tokio channels
   - Currently uses non-streaming API under the hood but returns results in streaming format
   - Test coverage added in test_bedrock_supports_streaming
   
2. **Credential Handling** - IMPROVED (Commit f6a2e2ec83, 743dc2ac97)
   - Enhanced AWS credential loading with better config handling
   - Fixed credential refresh issues
   - Support for AWS SSO profiles
   - Support for AWS_PROFILE and AWS_REGION configuration
   - Improved error messages for authentication failures

3. **Tool Calling** - FIXED (Commits 645f00e7b9, 47d5c96ee8)
   - Fixed tool input schema handling
   - Fixed tool call error response handling
   - Proper support for tool use blocks in Bedrock format

4. **Error Handling** - ENHANCED
   - Specific error types for throttling, authentication, validation
   - Context length exceeded detection
   - Model error exception handling
   - Improved logging (Commit 9b4c8726f1)

5. **Retry Logic** - IMPLEMENTED (Commit 627696a68a)
   - Configurable retry parameters (max retries, intervals, backoff)
   - Environment variable configuration support
   - Default values: 6 retries, 2s initial, 2x backoff, 120s max

6. **Multi-modal Support** - ADDED (Commit 36b517379b)
   - Image content support
   - Document processing with conversion to text for unsupported formats
   - Proper MIME type handling

7. **Model Roster** - UPDATED
   - Latest Claude 4 models included
   - Default model: us.anthropic.claude-sonnet-4-5-20250929-v1:0
   - Known models list maintained with latest versions

8. **Provider Metadata** - IMPLEMENTED
   - Runtime access to provider name (Commit 4d8c91efbd)
   - Token usage tracking including cache tokens (Commit 37e1bb1d37)
   - Provider usage statistics

## Current Limitations

### 1. Streaming Implementation
- **Issue**: Current streaming uses non-streaming Bedrock API
- **Details**: The stream() method wraps the converse() API call and returns results in streaming format, but does not use Bedrocks native streaming API (converse_stream())
- **Impact**: No real-time token streaming, users do not see partial responses

### 2. Model Coverage
- Limited to Anthropic Claude models on Bedrock
- No support for other Bedrock model families (Amazon Titan, Cohere, AI21, Meta Llama, Mistral)
- No model capability detection (which models support images, tools, etc.)

### 3. Advanced Bedrock Features
- No integration with Bedrock Knowledge Bases
- No integration with Bedrock Agents
- No support for Bedrock Guardrails
- No support for Bedrock Model Evaluation

## Implementation Roadmap

### Phase 2: True Streaming Support (PRIORITY)

**Goal**: Implement real Bedrock streaming using converse_stream() API

1. **Use Bedrocks ConverseStream API**
   - Replace converse() with converse_stream() in stream() method
   - Handle ConverseStreamOutput events properly
   - Process chunks as they arrive: ContentBlockStart, ContentBlockDelta, ContentBlockStop
   - Handle MessageStart, MessageStop events
   - Accumulate tool use blocks properly during streaming

2. **Incremental Message Building**
   - Create message builder that accumulates chunks
   - Support partial text updates
   - Support tool call streaming (accumulate tool use blocks)
   - Send incremental updates through the MessageStream

3. **Error Handling for Streaming**
   - Handle stream interruptions gracefully
   - Detect and report throttling in streams
   - Handle model errors mid-stream

4. **Testing**
   - Add integration test with actual streaming
   - Test tool calling during streaming
   - Test error recovery
   - Mock streaming responses for unit tests

### Phase 3: Model Expansion

1. **Add More Bedrock Models**
   - Amazon Titan Text models
   - Cohere Command models
   - AI21 Jurassic models
   - Meta Llama models
   - Mistral models

2. **Model Capability Detection**
   - Create capability matrix (tools, vision, streaming)
   - Auto-detect based on model ID
   - Validate features before use

3. **Cross-Region Model Support**
   - Support different model IDs per region
   - Handle region-specific model availability

### Phase 4: Advanced Features

1. **Bedrock Guardrails Integration**
   - Add guardrail configuration support
   - Handle guardrail intervention responses
   - Configure content filtering policies

2. **Bedrock Knowledge Base Integration**
   - Create tool/MCP server for querying Knowledge Bases
   - Support RAG workflows
   - Citation tracking

3. **Bedrock Agents Integration**
   - Support calling pre-configured Bedrock Agents
   - Session management for agent conversations
   - Agent orchestration

### Phase 5: Developer Experience

1. **Enhanced Configuration**
   - Support for custom endpoints (VPC endpoints)
   - Better credential chain debugging
   - Configuration validation before API calls

2. **Testing Infrastructure**
   - Comprehensive mock Bedrock client
   - Deterministic streaming behavior for tests
   - Performance benchmarks

3. **Documentation**
   - Comprehensive setup guide
   - Common use cases and examples
   - Troubleshooting guide
   - Configuration reference

## Next Immediate Steps

### Priority 1: Implement True Streaming

This is the most impactful improvement and addresses the main limitation.

**Tasks**:
1. Study aws_sdk_bedrockruntime::operation::converse_stream API
2. Implement converse_stream_internal() helper method
3. Replace fake streaming with real streaming in stream() method
4. Handle all event types: MessageStart, ContentBlockStart, ContentBlockDelta, ContentBlockStop, MessageStop, Metadata
5. Build incremental messages from chunks
6. Update tests to validate real streaming behavior

**Files to modify**:
- crates/goose/src/providers/bedrock.rs - main implementation
- crates/goose/src/providers/formats/bedrock.rs - add streaming event converters
- crates/goose/tests/providers.rs - enhance streaming tests

**Acceptance Criteria**:
- Messages stream token by token in real-time
- Tool calls are properly assembled from streamed chunks
- Errors during streaming are handled gracefully
- Tests validate streaming behavior (even with mocks)
- No regression in non-streaming mode

### Priority 2: Add More Model Support

Expand beyond Anthropic Claude to other Bedrock model families.

### Priority 3: Guardrails Support

Add content filtering and safety features.

## Notes

- All credential handling improvements are complete
- Retry logic is fully configurable
- Tool calling works reliably
- Image support is functional
- The main gap is true streaming implementation
