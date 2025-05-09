# FFI Bindings Update Guide

## Overview

This guide shows how to update the FFI bindings to use the new FFI-friendly agent design.

## Required Changes

### 1. Add New Types to FFI

```rust
// In goose-ffi/src/lib.rs

/// Reply process state for FFI
#[repr(C)]
pub enum ReplyProcessStateFFI {
    Ready = 0,
    WaitingForProvider = 1,
    MessageYielded = 2,
    WaitingForToolApproval = 3,
    ProcessingTools = 4,
    Completed = 5,
    Error = 6,
}

/// Pending tool request for FFI
#[repr(C)]
pub struct PendingToolRequestFFI {
    pub id: *const c_char,
    pub name: *const c_char,
    pub arguments: *const c_char, // JSON string
    pub requires_approval: bool,
}

/// Reply state pointer type
pub type ReplyStatePtr = *mut ReplyState;
```

### 2. Add FFI Functions

```rust
/// Create a new FFI agent
#[no_mangle]
pub unsafe extern "C" fn goose_ffi_agent_new(provider_ptr: ProviderPtr) -> *mut FFIAgent {
    if provider_ptr.is_null() {
        return ptr::null_mut();
    }
    
    let provider = Arc::from_raw(provider_ptr);
    let agent = FFIAgent::new(provider);
    Box::into_raw(Box::new(agent))
}

/// Create a reply state
#[no_mangle]
pub unsafe extern "C" fn goose_ffi_agent_create_reply_state(
    agent_ptr: *mut FFIAgent,
    messages: *const MessageFFI,
    messages_len: usize,
    session_config: *const SessionConfigFFI,
) -> ReplyStatePtr {
    if agent_ptr.is_null() {
        return ptr::null_mut();
    }
    
    let agent = &*agent_ptr;
    
    // Convert FFI messages to Rust messages
    let messages = if messages.is_null() {
        Vec::new()
    } else {
        std::slice::from_raw_parts(messages, messages_len)
            .iter()
            .map(|msg| convert_ffi_message(msg))
            .collect()
    };
    
    // Convert session config
    let session = if session_config.is_null() {
        None
    } else {
        Some(convert_ffi_session_config(&*session_config))
    };
    
    let reply_state = agent.create_reply_state(messages, session);
    Box::into_raw(Box::new(reply_state))
}

/// Start the reply process
#[no_mangle]
pub unsafe extern "C" fn goose_reply_state_start(
    reply_state_ptr: ReplyStatePtr
) -> *mut AsyncResult {
    if reply_state_ptr.is_null() {
        return create_error_result("Reply state pointer is null");
    }
    
    let reply_state = &mut *reply_state_ptr;
    
    get_runtime().block_on(async {
        match reply_state.start().await {
            Ok(_) => create_success_result(),
            Err(e) => create_error_result(&e.to_string()),
        }
    })
}

/// Advance the reply process
#[no_mangle]
pub unsafe extern "C" fn goose_reply_state_advance(
    reply_state_ptr: ReplyStatePtr
) -> *mut AsyncResult {
    if reply_state_ptr.is_null() {
        return create_error_result("Reply state pointer is null");
    }
    
    let reply_state = &mut *reply_state_ptr;
    
    get_runtime().block_on(async {
        match reply_state.advance().await {
            Ok(_) => create_success_result(),
            Err(e) => create_error_result(&e.to_string()),
        }
    })
}

/// Get the current state
#[no_mangle]
pub unsafe extern "C" fn goose_reply_state_get_state(
    reply_state_ptr: ReplyStatePtr
) -> ReplyProcessStateFFI {
    if reply_state_ptr.is_null() {
        return ReplyProcessStateFFI::Error;
    }
    
    let reply_state = &*reply_state_ptr;
    
    match reply_state.get_state() {
        ReplyProcessState::Ready => ReplyProcessStateFFI::Ready,
        ReplyProcessState::WaitingForProvider => ReplyProcessStateFFI::WaitingForProvider,
        ReplyProcessState::MessageYielded => ReplyProcessStateFFI::MessageYielded,
        ReplyProcessState::WaitingForToolApproval => ReplyProcessStateFFI::WaitingForToolApproval,
        ReplyProcessState::ProcessingTools => ReplyProcessStateFFI::ProcessingTools,
        ReplyProcessState::Completed => ReplyProcessStateFFI::Completed,
        ReplyProcessState::Error(_) => ReplyProcessStateFFI::Error,
    }
}

/// Get the current message
#[no_mangle]
pub unsafe extern "C" fn goose_reply_state_get_current_message(
    reply_state_ptr: ReplyStatePtr
) -> *mut MessageFFI {
    if reply_state_ptr.is_null() {
        return ptr::null_mut();
    }
    
    let reply_state = &*reply_state_ptr;
    
    match reply_state.get_current_message() {
        Some(message) => convert_message_to_ffi(message),
        None => ptr::null_mut(),
    }
}

/// Get pending tool requests
#[no_mangle]
pub unsafe extern "C" fn goose_reply_state_get_pending_tool_requests(
    reply_state_ptr: ReplyStatePtr,
    out_len: *mut usize,
) -> *mut PendingToolRequestFFI {
    if reply_state_ptr.is_null() || out_len.is_null() {
        return ptr::null_mut();
    }
    
    let reply_state = &*reply_state_ptr;
    let requests = reply_state.get_pending_tool_requests();
    
    *out_len = requests.len();
    
    if requests.is_empty() {
        return ptr::null_mut();
    }
    
    let ffi_requests: Vec<PendingToolRequestFFI> = requests
        .iter()
        .map(|req| convert_tool_request_to_ffi(req))
        .collect();
    
    let boxed_slice = ffi_requests.into_boxed_slice();
    Box::into_raw(boxed_slice) as *mut PendingToolRequestFFI
}

/// Approve a tool
#[no_mangle]
pub unsafe extern "C" fn goose_reply_state_approve_tool(
    reply_state_ptr: ReplyStatePtr,
    request_id: *const c_char,
) -> *mut AsyncResult {
    if reply_state_ptr.is_null() || request_id.is_null() {
        return create_error_result("Invalid parameters");
    }
    
    let reply_state = &mut *reply_state_ptr;
    let request_id = CStr::from_ptr(request_id).to_string_lossy().to_string();
    
    get_runtime().block_on(async {
        match reply_state.approve_tool(&request_id).await {
            Ok(_) => create_success_result(),
            Err(e) => create_error_result(&e.to_string()),
        }
    })
}

/// Deny a tool
#[no_mangle]
pub unsafe extern "C" fn goose_reply_state_deny_tool(
    reply_state_ptr: ReplyStatePtr,
    request_id: *const c_char,
) -> *mut AsyncResult {
    if reply_state_ptr.is_null() || request_id.is_null() {
        return create_error_result("Invalid parameters");
    }
    
    let reply_state = &mut *reply_state_ptr;
    let request_id = CStr::from_ptr(request_id).to_string_lossy().to_string();
    
    get_runtime().block_on(async {
        match reply_state.deny_tool(&request_id).await {
            Ok(_) => create_success_result(),
            Err(e) => create_error_result(&e.to_string()),
        }
    })
}

/// Free a reply state
#[no_mangle]
pub unsafe extern "C" fn goose_reply_state_free(reply_state_ptr: ReplyStatePtr) {
    if !reply_state_ptr.is_null() {
        let _ = Box::from_raw(reply_state_ptr);
    }
}
```

### 3. Kotlin Bindings

```kotlin
// In Kotlin FFI bindings

enum class ReplyProcessState(val value: Int) {
    READY(0),
    WAITING_FOR_PROVIDER(1),
    MESSAGE_YIELDED(2),
    WAITING_FOR_TOOL_APPROVAL(3),
    PROCESSING_TOOLS(4),
    COMPLETED(5),
    ERROR(6);
    
    companion object {
        fun fromValue(value: Int): ReplyProcessState {
            return values().find { it.value == value } ?: ERROR
        }
    }
}

data class PendingToolRequest(
    val id: String,
    val name: String,
    val arguments: String, // JSON
    val requiresApproval: Boolean
)

class FFIAgent(private val ptr: Long) {
    fun createReplyState(
        messages: List<Message>,
        sessionConfig: SessionConfig? = null
    ): ReplyState {
        val replyStatePtr = goose_ffi_agent_create_reply_state(
            ptr,
            messages.toFFIArray(),
            messages.size,
            sessionConfig?.toFFI()
        )
        return ReplyState(replyStatePtr)
    }
    
    // ... other methods
}

class ReplyState(private val ptr: Long) {
    suspend fun start() {
        val result = goose_reply_state_start(ptr)
        checkResult(result)
    }
    
    suspend fun advance() {
        val result = goose_reply_state_advance(ptr)
        checkResult(result)
    }
    
    fun getState(): ReplyProcessState {
        val stateValue = goose_reply_state_get_state(ptr)
        return ReplyProcessState.fromValue(stateValue)
    }
    
    fun getCurrentMessage(): Message? {
        val messagePtr = goose_reply_state_get_current_message(ptr)
        return if (messagePtr != 0L) {
            Message.fromFFI(messagePtr)
        } else {
            null
        }
    }
    
    fun getPendingToolRequests(): List<PendingToolRequest> {
        val lengthPtr = Memory(8) // For size_t
        val requestsPtr = goose_reply_state_get_pending_tool_requests(ptr, lengthPtr)
        
        if (requestsPtr == 0L) {
            return emptyList()
        }
        
        val length = lengthPtr.getLong(0)
        return (0 until length).map { index ->
            PendingToolRequest.fromFFI(requestsPtr + index * PendingToolRequestFFI.SIZE)
        }
    }
    
    suspend fun approveTool(requestId: String) {
        val result = goose_reply_state_approve_tool(ptr, requestId)
        checkResult(result)
    }
    
    suspend fun denyTool(requestId: String) {
        val result = goose_reply_state_deny_tool(ptr, requestId)
        checkResult(result)
    }
    
    fun free() {
        goose_reply_state_free(ptr)
    }
}
```

## Usage Example

```kotlin
class GooseService {
    suspend fun handleRequest(request: ChatRequest): ChatResponse {
        val agent = FFIAgent(providerPtr)
        val replyState = agent.createReplyState(request.messages, request.sessionConfig)
        
        try {
            replyState.start()
            
            val responses = mutableListOf<Message>()
            
            while (replyState.getState() != ReplyProcessState.COMPLETED) {
                when (replyState.getState()) {
                    ReplyProcessState.MESSAGE_YIELDED -> {
                        val message = replyState.getCurrentMessage()
                        if (message != null) {
                            responses.add(message)
                        }
                        replyState.advance()
                    }
                    
                    ReplyProcessState.WAITING_FOR_TOOL_APPROVAL -> {
                        val toolRequests = replyState.getPendingToolRequests()
                        
                        // Process tool approvals (could be async with user input)
                        for (toolRequest in toolRequests) {
                            if (shouldApprove(toolRequest)) {
                                replyState.approveTool(toolRequest.id)
                            } else {
                                replyState.denyTool(toolRequest.id)
                            }
                        }
                    }
                    
                    ReplyProcessState.ERROR -> {
                        throw RuntimeException("Error in reply process")
                    }
                    
                    else -> {
                        replyState.advance()
                    }
                }
            }
            
            return ChatResponse(responses)
        } finally {
            replyState.free()
        }
    }
}
```

## Benefits

1. **Synchronous Interface**: All FFI functions are synchronous
2. **State Machine**: Clear state transitions
3. **Memory Safety**: Proper memory management with free functions
4. **Error Handling**: Errors are captured and returned properly
5. **Tool Approval**: Explicit tool approval flow

## Testing

1. Test basic conversation flow
2. Test tool approval/denial
3. Test error handling
4. Test memory management (no leaks)
5. Test concurrent access