# Bedrock True Streaming Implementation Plan

## Executive Summary

Replace the current fake streaming implementation with true AWS Bedrock streaming using the `converse_stream()` API. This will provide real-time token-by-token streaming to users.

## Current State Analysis

### What Works
- Fake streaming wrapper around `converse()` API
- Returns results in streaming format using tokio channels
- Basic structure for MessageStream is in place
- Error handling for non-streaming API

### What's Missing
- No use of `converse_stream()` API
- No real-time token streaming
- Users see full response at once, not progressively

## AWS SDK API Understanding

### ConverseStream API Structure

The AWS SDK provides `client.converse_stream()` which returns a stream of `ConverseStreamOutput` events:

```rust
pub enum ConverseStreamOutput {
    ContentBlockDelta(ContentBlockDeltaEvent),     // Text/tool deltas as they arrive
    ContentBlockStart(ContentBlockStartEvent),     // Start of a content block
    ContentBlockStop(ContentBlockStopEvent),       // End of a content block
    MessageStart(MessageStartEvent),               // Start of message
    MessageStop(MessageStopEvent),                 // End of message with stop reason
    Metadata(ConverseStreamMetadataEvent),         // Token usage metadata
    Unknown                                        // Future-proofing
}
```

### Event Flow Pattern

1. **MessageStart** - Signals beginning of response, contains role
2. **ContentBlockStart** - Signals start of content (text or tool use)
   - Contains `content_block_start.start` which is a `ContentBlockStart` enum
   - Can be `Text` or `ToolUse` variant
3. **ContentBlockDelta** (multiple) - Incremental content chunks
   - Contains `delta` which is a `ContentBlockDelta` enum
   - For text: `Text(String)` with partial content
   - For tool use: `ToolUse { name, input }` with partial JSON
4. **ContentBlockStop** - Signals end of content block
5. **MessageStop** - End of message
   - Contains `stop_reason` (EndTurn, ToolUse, MaxTokens, etc.)
6. **Metadata** - Usage statistics
   - Contains token counts: input_tokens, output_tokens

### Key Event Structures

From the SDK documentation:

```rust
// ContentBlockDeltaEvent has a delta field
struct ContentBlockDeltaEvent {
    delta: ContentBlockDelta,  // enum with Text(String) or ToolUse variants
    content_block_index: i32,  // which content block this delta belongs to
}

// ContentBlockStartEvent has start field  
struct ContentBlockStartEvent {
    start: ContentBlockStart,  // enum with Text or ToolUse variants
    content_block_index: i32,
}

// MessageStopEvent has stop_reason
struct MessageStopEvent {
    stop_reason: StopReason,  // EndTurn, ToolUse, MaxTokens, etc.
    // ... other fields
}

// ConverseStreamMetadataEvent has usage
struct ConverseStreamMetadataEvent {
    usage: TokenUsage,
    metrics: ConverseStreamMetrics,
}
```

## Implementation Strategy

### Phase 1: Core Streaming Implementation

#### 1.1 Add New Imports

Add to `crates/goose/src/providers/bedrock.rs`:

```rust
use aws_sdk_bedrockruntime::operation::converse_stream::ConverseStreamError;
use aws_sdk_bedrockruntime::types::{
    ConverseStreamOutput, ContentBlockDelta, ContentBlockDeltaEvent,
    ContentBlockStart, ContentBlockStartEvent, ContentBlockStopEvent,
    MessageStartEvent, MessageStopEvent, ConverseStreamMetadataEvent,
};
use futures::TryStreamExt;
use async_stream::try_stream;
```

#### 1.2 Create Stream Accumulator Structure

Add to `crates/goose/src/providers/formats/bedrock.rs`:

```rust
/// Accumulates streaming chunks into a complete message
#[derive(Debug, Default)]
pub struct BedrockStreamAccumulator {
    // Text accumulation per content block
    text_blocks: HashMap<i32, String>,
    
    // Tool use accumulation per content block
    tool_blocks: HashMap<i32, (String, String)>, // (name, partial_json)
    
    // Track current role
    role: Option<Role>,
    
    // Track stop reason
    stop_reason: Option<String>,
    
    // Track usage
    usage: Option<bedrock::TokenUsage>,
}

impl BedrockStreamAccumulator {
    pub fn new() -> Self {
        Self::default()
    }
    
    pub fn handle_event(&mut self, event: ConverseStreamOutput) -> Result<Option<Message>> {
        match event {
            ConverseStreamOutput::MessageStart(msg_start) => {
                self.role = Some(from_bedrock_role(&msg_start.role));
                Ok(None)
            }
            
            ConverseStreamOutput::ContentBlockStart(block_start) => {
                // Initialize block based on type
                match &block_start.start {
                    ContentBlockStart::Text(_) => {
                        self.text_blocks.insert(block_start.content_block_index, String::new());
                    }
                    ContentBlockStart::ToolUse(tool_use) => {
                        self.tool_blocks.insert(
                            block_start.content_block_index,
                            (tool_use.name.clone().unwrap_or_default(), String::new())
                        );
                    }
                    _ => {}
                }
                Ok(None)
            }
            
            ConverseStreamOutput::ContentBlockDelta(delta_event) => {
                match &delta_event.delta {
                    ContentBlockDelta::Text(text) => {
                        // Accumulate text
                        if let Some(block) = self.text_blocks.get_mut(&delta_event.content_block_index) {
                            block.push_str(text);
                        }
                        
                        // Return incremental message with current state
                        self.build_incremental_message()
                    }
                    ContentBlockDelta::ToolUse(tool_delta) => {
                        // Accumulate tool JSON
                        if let Some((_, json)) = self.tool_blocks.get_mut(&delta_event.content_block_index) {
                            json.push_str(&tool_delta.input);
                        }
                        Ok(None) // Don't send until complete
                    }
                    _ => Ok(None)
                }
            }
            
            ConverseStreamOutput::ContentBlockStop(_) => {
                // Could finalize tool blocks here
                Ok(None)
            }
            
            ConverseStreamOutput::MessageStop(msg_stop) => {
                self.stop_reason = msg_stop.stop_reason.map(|r| format!("{:?}", r));
                // Return final message
                self.build_final_message()
            }
            
            ConverseStreamOutput::Metadata(metadata) => {
                self.usage = Some(metadata.usage);
                Ok(None)
            }
            
            _ => Ok(None)
        }
    }
    
    fn build_incremental_message(&self) -> Result<Option<Message>> {
        let role = self.role.clone().unwrap_or(Role::Assistant);
        let mut message = Message::new(role);
        
        // Add accumulated text blocks in order
        let mut indices: Vec<_> = self.text_blocks.keys().cloned().collect();
        indices.sort();
        for idx in indices {
            if let Some(text) = self.text_blocks.get(&idx) {
                if !text.is_empty() {
                    message = message.with_text(text.clone());
                }
            }
        }
        
        Ok(Some(message))
    }
    
    fn build_final_message(&self) -> Result<Option<Message>> {
        let role = self.role.clone().unwrap_or(Role::Assistant);
        let mut message = Message::new(role);
        
        // Add all text blocks
        let mut indices: Vec<_> = self.text_blocks.keys().cloned().collect();
        indices.sort();
        for idx in indices {
            if let Some(text) = self.text_blocks.get(&idx) {
                if !text.is_empty() {
                    message = message.with_text(text.clone());
                }
            }
        }
        
        // Add all tool use blocks
        let mut tool_indices: Vec<_> = self.tool_blocks.keys().cloned().collect();
        tool_indices.sort();
        for idx in tool_indices {
            if let Some((name, json)) = self.tool_blocks.get(&idx) {
                // Parse complete JSON and create tool request
                if let Ok(args) = serde_json::from_str::<serde_json::Value>(json) {
                    let tool_call = CallToolRequestParam {
                        name: name.clone(),
                        arguments: args.as_object()
                            .map(|o| o.clone())
                            .unwrap_or_default(),
                    };
                    message = message.with_tool_request(Ok(tool_call));
                }
            }
        }
        
        Ok(Some(message))
    }
    
    pub fn get_usage(&self) -> Option<Usage> {
        self.usage.as_ref().map(from_bedrock_usage)
    }
}
```

#### 1.3 Replace Fake Streaming with Real Streaming

Modify `stream()` method in `crates/goose/src/providers/bedrock.rs`:

```rust
async fn stream(
    &self,
    system: &str,
    messages: &[Message],
    tools: &[Tool],
) -> Result<MessageStream, ProviderError> {
    let (tx, rx) = mpsc::channel::<Result<(Option<Message>, Option<ProviderUsage>), ProviderError>>(100);
    let stream_receiver = ReceiverStream::new(rx);
    
    let client = self.client.clone();
    let model_name = self.model.model_name.clone();
    let system_prompt = system.to_string();
    let messages_clone = messages.to_vec();
    let tools_clone = tools.to_vec();
    
    tokio::spawn(async move {
        let result = Self::converse_stream_internal(
            &client,
            &model_name,
            &system_prompt,
            &messages_clone,
            &tools_clone,
            tx.clone()
        ).await;
        
        if let Err(e) = result {
            let _ = tx.send(Err(e)).await;
        }
    });
    
    Ok(Box::pin(stream_receiver))
}

async fn converse_stream_internal(
    client: &Client,
    model_name: &str,
    system: &str,
    messages: &[Message],
    tools: &[Tool],
    tx: mpsc::Sender<Result<(Option<Message>, Option<ProviderUsage>), ProviderError>>
) -> Result<(), ProviderError> {
    // Build request
    let mut request = client
        .converse_stream()
        .model_id(model_name.to_string());
    
    // Add system prompt if not empty
    if !system.is_empty() {
        request = request.system(bedrock::SystemContentBlock::Text(system.to_string()));
    }
    
    // Add messages
    let bedrock_messages: Vec<bedrock::Message> = messages
        .iter()
        .filter(|m| m.is_agent_visible())
        .map(to_bedrock_message)
        .collect::<Result<_>>()?;
    request = request.set_messages(Some(bedrock_messages));
    
    // Add tool config if tools exist
    if !tools.is_empty() {
        request = request.tool_config(to_bedrock_tool_config(tools)?);
    }
    
    // Send request and get stream
    let mut stream = request
        .send()
        .await
        .map_err(|err| Self::map_stream_error(err))?
        .stream;
    
    // Process stream events
    let mut accumulator = BedrockStreamAccumulator::new();
    
    while let Some(event_result) = stream.next().await {
        match event_result {
            Ok(event) => {
                // Handle event through accumulator
                match accumulator.handle_event(event)? {
                    Some(incremental_msg) => {
                        // Send incremental update
                        tx.send(Ok((Some(incremental_msg), None))).await
                            .map_err(|_| ProviderError::RequestFailed("Channel closed".into()))?;
                    }
                    None => {
                        // No message to send yet (metadata, block starts, etc.)
                    }
                }
            }
            Err(e) => {
                // Handle stream error
                let provider_error = Self::map_stream_error(e);
                tx.send(Err(provider_error)).await
                    .map_err(|_| ProviderError::RequestFailed("Channel closed".into()))?;
                return Err(ProviderError::RequestFailed("Stream error".into()));
            }
        }
    }
    
    // Send final usage if available
    if let Some(usage) = accumulator.get_usage() {
        let provider_usage = ProviderUsage::new(model_name.to_string(), usage);
        tx.send(Ok((None, Some(provider_usage)))).await
            .map_err(|_| ProviderError::RequestFailed("Channel closed".into()))?;
    }
    
    // Send completion signal
    tx.send(Ok((None, None))).await
        .map_err(|_| ProviderError::RequestFailed("Channel closed".into()))?;
    
    Ok(())
}

fn map_stream_error(err: SdkError<ConverseStreamError>) -> ProviderError {
    match err.into_service_error() {
        ConverseStreamError::ThrottlingException(throttle_err) => {
            ProviderError::RateLimitExceeded {
                details: format!("Bedrock streaming throttling: {:?}", throttle_err),
                retry_delay: None,
            }
        }
        ConverseStreamError::AccessDeniedException(err) => {
            ProviderError::Authentication(format!("Bedrock streaming access denied: {:?}", err))
        }
        ConverseStreamError::ValidationException(err) 
            if err.message().unwrap_or_default().contains("too long") =>
        {
            ProviderError::ContextLengthExceeded(format!("Bedrock streaming context exceeded: {:?}", err))
        }
        ConverseStreamError::ModelStreamErrorException(err) => {
            ProviderError::ExecutionError(format!("Bedrock model streaming error: {:?}", err))
        }
        err => ProviderError::ServerError(format!("Bedrock streaming error: {:?}", err)),
    }
}
```

### Phase 2: Helper Functions

Add to `crates/goose/src/providers/formats/bedrock.rs`:

```rust
/// Convert Bedrock ConversationRole to rmcp Role
pub fn from_bedrock_role(role: &bedrock::ConversationRole) -> Role {
    match role {
        bedrock::ConversationRole::User => Role::User,
        bedrock::ConversationRole::Assistant => Role::Assistant,
        _ => Role::Assistant, // Default
    }
}
```

### Phase 3: Testing Strategy

#### Unit Tests

Add to `crates/goose/src/providers/formats/bedrock.rs`:

```rust
#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_accumulator_text_streaming() {
        let mut acc = BedrockStreamAccumulator::new();
        
        // Simulate message start
        let msg_start = MessageStartEvent::builder()
            .role(bedrock::ConversationRole::Assistant)
            .build();
        acc.handle_event(ConverseStreamOutput::MessageStart(msg_start)).unwrap();
        
        // Simulate content block start
        let block_start = ContentBlockStartEvent::builder()
            .content_block_index(0)
            .start(ContentBlockStart::Text())
            .build();
        acc.handle_event(ConverseStreamOutput::ContentBlockStart(block_start)).unwrap();
        
        // Simulate text deltas
        let delta1 = ContentBlockDeltaEvent::builder()
            .content_block_index(0)
            .delta(ContentBlockDelta::Text("Hello".to_string()))
            .build();
        let msg1 = acc.handle_event(ConverseStreamOutput::ContentBlockDelta(delta1)).unwrap();
        assert!(msg1.is_some());
        
        let delta2 = ContentBlockDeltaEvent::builder()
            .content_block_index(0)
            .delta(ContentBlockDelta::Text(" World".to_string()))
            .build();
        let msg2 = acc.handle_event(ConverseStreamOutput::ContentBlockDelta(delta2)).unwrap();
        assert!(msg2.is_some());
        
        // Check accumulated text
        let final_msg = msg2.unwrap();
        assert_eq!(final_msg.content[0].as_text().unwrap().text, "Hello World");
    }
    
    #[test]
    fn test_accumulator_tool_streaming() {
        // Similar test for tool use streaming
        todo!("Test tool accumulation")
    }
}
```

#### Integration Tests

Update `crates/goose/tests/providers.rs`:

```rust
#[tokio::test]
#[ignore] // Requires AWS credentials
async fn test_bedrock_real_streaming() -> Result<()> {
    let model_config = ModelConfig::new(BEDROCK_DEFAULT_MODEL)?;
    let provider = BedrockProvider::from_env(model_config).await?;
    
    let messages = vec![Message::user().with_text("Count to 5 slowly")];
    let mut stream = provider.stream("", &messages, &[]).await?;
    
    let mut chunks_received = 0;
    let mut final_text = String::new();
    
    while let Some(result) = stream.next().await {
        match result {
            Ok((Some(msg), _usage)) => {
                chunks_received += 1;
                if let Some(text) = msg.content.first().and_then(|c| c.as_text()) {
                    final_text = text.text.clone();
                    println!("Chunk {}: {}", chunks_received, text.text);
                }
            }
            Ok((None, Some(_usage))) => {
                println!("Received usage info");
            }
            Ok((None, None)) => {
                println!("Stream complete");
                break;
            }
            Err(e) => {
                return Err(anyhow::anyhow!("Stream error: {:?}", e));
            }
        }
    }
    
    assert!(chunks_received > 1, "Should receive multiple streaming chunks");
    assert!(final_text.contains("5"), "Final text should contain the number 5");
    
    Ok(())
}
```

## Dependencies to Add

Check `crates/goose/Cargo.toml` and add if not present:

```toml
async-stream = "0.3"  # For try_stream! macro
```

## Error Handling Considerations

1. **Stream Interruption**: If the stream is interrupted mid-way, we should return the partial message accumulated so far
2. **Malformed Events**: Handle unexpected event orders gracefully
3. **Tool JSON Parsing**: Validate JSON before attempting to parse tool arguments
4. **Channel Errors**: Handle case where receiver is dropped

## Performance Considerations

1. **Memory**: Accumulator holds incremental state - acceptable for typical message sizes
2. **Channel Buffer**: 100-element channel should be sufficient for streaming chunks
3. **Concurrency**: tokio::spawn ensures streaming doesn't block caller

## Rollout Plan

1. **Phase 1**: Implement core streaming with text-only support (1-2 hours)
2. **Phase 2**: Add tool use streaming support (30 mins)
3. **Phase 3**: Add comprehensive tests (1 hour)
4. **Phase 4**: Manual testing with real API (30 mins)
5. **Phase 5**: Documentation updates (30 mins)

Total estimated time: 4-5 hours

## Success Criteria

- ✅ Messages stream token by token in real-time
- ✅ Tool calls are properly assembled from streamed chunks
- ✅ Errors during streaming are handled gracefully
- ✅ Tests validate streaming behavior
- ✅ No regression in non-streaming (complete) mode
- ✅ Usage statistics are properly reported

## Risks and Mitigations

| Risk | Mitigation |
|------|------------|
| AWS SDK API changes | Pin to specific version (=1.120.0) |
| Complex tool JSON streaming | Accumulate full JSON before parsing |
| Event order variations | Defensive programming in accumulator |
| Performance degradation | Benchmark against current implementation |

## Next Steps

1. Create feature branch: `git checkout -b feat/bedrock-true-streaming`
2. Implement BedrockStreamAccumulator in formats/bedrock.rs
3. Update stream() method to use converse_stream()
4. Add helper function from_bedrock_role()
5. Add unit tests for accumulator
6. Test manually with AWS credentials
7. Update integration tests
8. Run full test suite
9. Document changes in TODO-BEDROCK.md
10. Submit PR
