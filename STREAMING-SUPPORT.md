# Streaming Support Implementation Plan for Goose

## Overview
Currently, Goose waits until the full response is complete before showing it to users. This plan outlines how to add streaming support to display partial responses in real-time, starting with the CLI and extending to the UI.

## Current State Analysis

### What's Already Implemented
1. **Provider Level**: Streaming infrastructure exists (`stream()` method in Provider trait, MessageStream type)
2. **Server/UI**: Full streaming support via Server-Sent Events (SSE)
3. **Agent Level**: `stream_response_from_provider()` method handles streaming from providers

### What's Missing
1. **CLI**: No streaming support - waits for complete responses before rendering
2. **Partial Message Rendering**: CLI can't display incomplete messages
3. **Progress Indicators**: "Thinking..." spinner hides actual streaming content

## Implementation Plan

### Phase 1: CLI Streaming Support

#### 1.1 Modify CLI Session Handler
- Update `process_agent_response()` in `crates/goose-cli/src/session/mod.rs`
- Add streaming message handler similar to UI's approach
- Handle partial text content rendering

#### 1.2 Create Streaming Renderer
- Add new module `crates/goose-cli/src/session/streaming.rs`
- Implement partial message accumulation and rendering
- Handle ANSI escape sequences for updating content in-place

#### 1.3 Update Output Module
- Modify `render_message()` to support partial content
- Add `render_streaming_text()` function for incremental updates
- Ensure markdown rendering works with partial content

### Phase 2: Message Content Handling

#### 2.1 Partial Message State
- Track message assembly state in CLI session
- Buffer partial text content until complete
- Handle tool calls (only render when complete)

#### 2.2 Stream Processing
- Process `AgentEvent::Message` events incrementally
- Distinguish between text updates and complete messages
- Maintain message integrity for session storage

### Phase 3: UI/UX Improvements

#### 3.1 Progress Indicators
- Replace blocking "Thinking..." with streaming-aware indicators
- Show partial content instead of hiding behind spinner
- Add character/word streaming animation

#### 3.2 Error Handling
- Handle stream interruptions gracefully
- Ensure partial messages are cleaned up on errors
- Maintain session consistency

### Phase 4: Testing & Polish

#### 4.1 Test Coverage
- Unit tests for streaming renderer
- Integration tests with mock providers
- Manual testing with various providers

#### 4.2 Performance
- Optimize rendering frequency
- Minimize terminal flicker
- Handle slow streams gracefully

## Technical Details

### Key Changes Required

1. **Session Module** (`crates/goose-cli/src/session/mod.rs`):
   - Refactor `process_agent_response()` to handle streaming
   - Add message accumulation logic
   - Update event processing loop

2. **Output Module** (`crates/goose-cli/src/session/output.rs`):
   - Add streaming text renderer
   - Support partial markdown rendering
   - Handle cursor positioning for updates

3. **Message Handling**:
   - Track partial vs complete messages
   - Ensure tool calls are atomic
   - Preserve message metadata

### Implementation Order
1. Start with basic text streaming
2. Add proper cursor control and updates
3. Handle edge cases (tools, errors, interrupts)
4. Polish UI/UX
5. Add comprehensive tests

## Success Criteria
- CLI shows response text as it arrives from the LLM
- No regression in functionality
- Smooth, flicker-free updates
- Proper handling of interruptions
- Consistent session state

## File Structure
```
STREAMING-SUPPORT.md (this plan)
crates/goose-cli/src/session/
  ├── mod.rs (modify)
  ├── output.rs (modify)
  └── streaming.rs (new)
```

## Implementation Status
- [x] Phase 1: CLI Streaming Support (Basic infrastructure)
  - [x] Created streaming renderer module (`streaming.rs`)
  - [x] Updated session handler to detect streaming messages
  - [x] Added streaming renderer to Session struct
  - [x] Implemented message accumulation logic
  - [x] Code compiles successfully
  - [ ] Need to modify agent to yield partial messages
- [x] Phase 2: Message Content Handling  
  - [x] Track partial vs complete messages
  - [x] Handle message IDs (including optional IDs)
  - [x] Buffer text content until complete
- [ ] Phase 3: UI/UX Improvements
- [ ] Phase 4: Testing & Polish

## Current Challenges

### Agent Architecture
The current agent implementation (`reply_internal` in `agent.rs`) yields complete messages via `AgentEvent::Message`. This causes the following issues:

1. **Multiple Message Problem**: The agent sends each text chunk as a separate complete message rather than updates to a single message
2. **Message ID Inconsistency**: Each chunk may have a different or missing message ID, preventing proper streaming detection
3. **Newline Issues**: Each "complete" message adds spacing, resulting in broken output with extra newlines between chunks

### Current Behavior
When the agent streams a response, it appears to:
1. Receive streaming data from the provider
2. Buffer small chunks of text
3. Yield each chunk as a complete `AgentEvent::Message`
4. Each chunk is treated as a new message by the CLI

This results in output like:
```
Hello! I'm Goose,

 an AI assistant created by Block. I'm here to help you with a

 wide variety of tasks.
```

Instead of:
```
Hello! I'm Goose, an AI assistant created by Block. I'm here to help you with a wide variety of tasks.
```

### Provider Integration
While providers support streaming (e.g., Anthropic has a `stream()` method), the agent currently processes the stream in a way that breaks it into multiple messages. This needs to be changed to support true incremental updates.

### Next Steps for True Streaming
1. **Modify agent's `reply_internal`** to:
   - Yield partial `AgentEvent::Message` events with consistent message IDs
   - Mark messages as "partial" vs "complete"
   - Accumulate text content properly at the agent level

2. **Update `stream_response_from_provider`** to:
   - Yield incremental text updates as they arrive
   - Maintain message structure and metadata
   - Handle buffering more intelligently

3. **Enhance Message Structure**:
   - Add a "streaming" or "partial" flag to messages
   - Ensure consistent message IDs throughout streaming
   - Add proper message completion signals

4. **Handle Edge Cases**:
   - Tool calls must remain atomic (only yield when complete)
   - Error handling during streaming
   - Proper cleanup on interruption

### Workarounds Attempted
1. Created streaming renderer that tracks active messages
2. Attempted to detect streaming by checking for active message IDs
3. Adjusted newline handling to minimize extra spacing
4. Added debug logging to understand message flow

These workarounds help but don't solve the fundamental issue that the agent is sending multiple complete messages instead of streaming updates to a single message.