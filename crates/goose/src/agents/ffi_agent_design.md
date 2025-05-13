# FFI-Friendly Agent Design

## Overview

The FFI-friendly agent design addresses the challenges of using Rust's async streams across FFI boundaries. The original `reply` method returns a `BoxStream` which yields messages asynchronously, making it difficult to use from languages like Kotlin.

## Key Components

### 1. ReplyState

A stateful object that manages the conversation flow:

```rust
pub struct ReplyState {
    pub state: ReplyProcessState,
    pub current_message: Option<Message>,
    pub pending_tool_requests: Vec<PendingToolRequest>,
    pub messages: Vec<Message>,
    pub session: Option<SessionConfig>,
    agent: Arc<Agent>,
}
```

### 2. ReplyProcessState

An enum that tracks the current state of the conversation:

```rust
pub enum ReplyProcessState {
    Ready,
    WaitingForProvider,
    MessageYielded,
    WaitingForToolApproval,
    ProcessingTools,
    Completed,
    Error(String),
}
```

### 3. FFIAgent

A wrapper around the Agent that provides FFI-friendly methods:

```rust
pub struct FFIAgent {
    agent: Arc<Agent>,
}
```

## Usage Pattern

### Kotlin Side

```kotlin
// Create agent
val agent = FFIAgent(provider)

// Create reply state
val replyState = agent.createReplyState(messages, sessionConfig)

// Start the conversation
replyState.start()

// Main loop
while (replyState.getState() != ReplyProcessState.Completed) {
    when (replyState.getState()) {
        ReplyProcessState.MessageYielded -> {
            val message = replyState.getCurrentMessage()
            // Process and display message
            replyState.advance()
        }
        
        ReplyProcessState.WaitingForToolApproval -> {
            val toolRequests = replyState.getPendingToolRequests()
            // Show tool approval UI
            // For each approved tool:
            replyState.approveTool(toolId)
            // For each denied tool:
            replyState.denyTool(toolId)
        }
        
        ReplyProcessState.Error -> {
            // Handle error
            break
        }
        
        else -> {
            // Continue processing
            replyState.advance()
        }
    }
}
```

## Benefits

1. **No Async Streams**: Replaces async streams with a state machine pattern
2. **Clear State Management**: Explicit states make it easy to understand what's happening
3. **Tool Approval Flow**: Handles tool approval in a synchronous manner
4. **Error Handling**: Errors are captured in the state machine
5. **FFI-Friendly**: All methods are synchronous and use simple types

## Implementation Details

1. **State Machine**: The `ReplyState` acts as a state machine that advances through different states
2. **Message Processing**: Messages are processed one at a time instead of streaming
3. **Tool Handling**: Tool requests are extracted and stored for approval
4. **Memory Management**: Uses `Arc` for shared ownership across FFI boundaries

## Testing

The implementation includes comprehensive tests:
- Basic flow testing
- Tool request extraction
- Tool approval flow

All tests pass successfully, confirming the design works as intended.

## Future Improvements

1. Add support for streaming responses (partial messages)
2. Implement timeout handling
3. Add cancellation support
4. Optimize memory usage for large conversations