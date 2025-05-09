#!/usr/bin/env python3
"""
Test script for the Goose FFI interface with the new FFI-friendly agent design.

This script tests:
1. Basic conversation flow
2. Tool approval workflow
3. Frontend tool execution
4. Error handling
5. Memory management
"""

import ctypes
import os
import platform
import json
import unittest
from ctypes import c_char_p, c_bool, c_uint32, c_void_p, Structure, POINTER, c_size_t
from enum import IntEnum

class ProviderType:
    DATABRICKS = 0

# Platform-specific dynamic lib name
if platform.system() == "Darwin":
    LIB_NAME = "libgoose_ffi.dylib"
elif platform.system() == "Linux":
    LIB_NAME = "libgoose_ffi.so"
elif platform.system() == "Windows":
    LIB_NAME = "goose_ffi.dll"
else:
    raise RuntimeError("Unsupported platform")

# Adjust to your actual build output directory
LIB_PATH = os.path.join(os.path.dirname(__file__), "../../..", "target", "debug", LIB_NAME)

# Load library
goose = ctypes.CDLL(LIB_PATH)

# Forward declarations
class goose_Agent(Structure):
    pass

class goose_FFIAgent(Structure):
    pass

class goose_ReplyState(Structure):
    pass

# Pointer types
goose_AgentPtr = POINTER(goose_Agent)
goose_FFIAgentPtr = POINTER(goose_FFIAgent)
goose_ReplyStatePtr = POINTER(goose_ReplyState)

# Enums
class ReplyProcessState(IntEnum):
    READY = 0
    WAITING_FOR_PROVIDER = 1
    MESSAGE_YIELDED = 2
    WAITING_FOR_TOOL_APPROVAL = 3
    PROCESSING_TOOLS = 4
    COMPLETED = 5
    ERROR = 6

class MessageRole(IntEnum):
    USER = 0
    ASSISTANT = 1
    SYSTEM = 2

# C struct mappings
class ProviderConfig(Structure):
    _fields_ = [
        ("provider_type", c_uint32),
        ("api_key", c_char_p),
        ("model_name", c_char_p),
        ("host", c_char_p),
    ]

class AsyncResult(Structure):
    _fields_ = [
        ("succeeded", c_bool),
        ("error_message", c_char_p),
    ]

class MessageFFI(Structure):
    _fields_ = [
        ("role", c_uint32),
        ("content", c_char_p),
    ]

class PendingToolRequestFFI(Structure):
    _fields_ = [
        ("id", c_char_p),
        ("name", c_char_p),
        ("arguments", c_char_p),  # JSON string
        ("requires_approval", c_bool),
    ]

# Function signatures (same as in the main example)
# ... [Include all the function signatures from the previous file]

class GooseAgent:
    """Wrapper for the Goose FFI agent."""
    def __init__(self, provider_type=ProviderType.DATABRICKS, api_key=None, model_name=None, host=None):
        self.config = ProviderConfig(
            provider_type=provider_type,
            api_key=api_key.encode("utf-8") if api_key else None,
            model_name=model_name.encode("utf-8") if model_name else None,
            host=host.encode("utf-8") if host else None,
        )
        self.agent = goose.goose_agent_new(ctypes.byref(self.config))
        if not self.agent:
            raise RuntimeError("Failed to create Goose agent")
        
        # Create FFI agent wrapper
        self.ffi_agent = goose.goose_ffi_agent_new(self.agent)
        if not self.ffi_agent:
            raise RuntimeError("Failed to create FFI agent")

    def __del__(self):
        if getattr(self, "agent", None):
            goose.goose_agent_free(self.agent)

    def create_reply_state(self, messages):
        """Create a reply state for handling a conversation."""
        # Convert Python messages to FFI messages
        ffi_messages = (MessageFFI * len(messages))()
        for i, msg in enumerate(messages):
            content_json = json.dumps(msg["content"])
            ffi_messages[i].role = msg["role"]
            ffi_messages[i].content = content_json.encode("utf-8")
        
        reply_state_ptr = goose.goose_ffi_agent_create_reply_state(
            self.ffi_agent,
            ffi_messages,
            len(messages),
            None  # No session config for now
        )
        
        if not reply_state_ptr:
            raise RuntimeError("Failed to create reply state")
        
        return ReplyState(reply_state_ptr)

class ReplyState:
    """Wrapper for the reply state."""
    def __init__(self, ptr):
        self.ptr = ptr
    
    def __del__(self):
        if getattr(self, "ptr", None):
            goose.goose_reply_state_free(self.ptr)
    
    def start(self):
        """Start the reply process."""
        result = goose.goose_reply_state_start(self.ptr)
        self._check_result(result)
    
    def advance(self):
        """Advance to the next state."""
        result = goose.goose_reply_state_advance(self.ptr)
        self._check_result(result)
    
    def get_state(self):
        """Get the current state."""
        state_value = goose.goose_reply_state_get_state(self.ptr)
        return ReplyProcessState(state_value)
    
    def get_current_message(self):
        """Get the current message if available."""
        message_ptr = goose.goose_reply_state_get_current_message(self.ptr)
        if not message_ptr:
            return None
        
        message_json = ctypes.string_at(message_ptr).decode("utf-8")
        goose.goose_free_string(message_ptr)
        return json.loads(message_json)
    
    def get_pending_tool_requests(self):
        """Get pending tool requests."""
        length = c_size_t()
        requests_ptr = goose.goose_reply_state_get_pending_tool_requests(
            self.ptr,
            ctypes.byref(length)
        )
        
        if not requests_ptr:
            return []
        
        requests = []
        for i in range(length.value):
            request = requests_ptr[i]
            requests.append({
                "id": ctypes.string_at(request.id).decode("utf-8"),
                "name": ctypes.string_at(request.name).decode("utf-8"),
                "arguments": json.loads(ctypes.string_at(request.arguments).decode("utf-8")),
                "requires_approval": request.requires_approval
            })
        
        return requests
    
    def approve_tool(self, request_id):
        """Approve a tool request."""
        result = goose.goose_reply_state_approve_tool(
            self.ptr,
            request_id.encode("utf-8")
        )
        self._check_result(result)
    
    def deny_tool(self, request_id):
        """Deny a tool request."""
        result = goose.goose_reply_state_deny_tool(
            self.ptr,
            request_id.encode("utf-8")
        )
        self._check_result(result)
    
    def _check_result(self, result):
        """Check an AsyncResult and raise if there's an error."""
        if not result:
            raise RuntimeError("Null result returned")
        
        if not result.contents.succeeded:
            error_msg = ctypes.string_at(result.contents.error_message).decode("utf-8")
            goose.goose_free_async_result(result)
            raise RuntimeError(f"Operation failed: {error_msg}")
        
        goose.goose_free_async_result(result)

class TestGooseFFI(unittest.TestCase):
    def setUp(self):
        """Set up test environment."""
        self.api_key = os.getenv("DATABRICKS_API_KEY")
        self.host = os.getenv("DATABRICKS_HOST")
        self.agent = GooseAgent(
            api_key=self.api_key,
            model_name="claude-3-7-sonnet",
            host=self.host
        )
    
    def test_basic_conversation(self):
        """Test a basic conversation flow."""
        message = {
            "role": MessageRole.USER,
            "content": [{"type": "text", "text": "Hello, how are you?"}]
        }
        
        reply_state = self.agent.create_reply_state([message])
        
        # Start the conversation
        reply_state.start()
        
        # Process until completion
        responses = []
        while reply_state.get_state() != ReplyProcessState.COMPLETED:
            state = reply_state.get_state()
            
            if state == ReplyProcessState.MESSAGE_YIELDED:
                message = reply_state.get_current_message()
                if message:
                    responses.append(message)
                reply_state.advance()
            else:
                reply_state.advance()
        
        # Verify we got at least one response
        self.assertGreater(len(responses), 0)
        self.assertIn("content", responses[0])
    
    def test_tool_approval_flow(self):
        """Test the tool approval workflow."""
        # Create a message that will trigger a tool request
        message = {
            "role": MessageRole.USER,
            "content": [{"type": "text", "text": "What is 42 + 58?"}]
        }
        
        reply_state = self.agent.create_reply_state([message])
        reply_state.start()
        
        tool_requests_found = False
        
        while reply_state.get_state() != ReplyProcessState.COMPLETED:
            state = reply_state.get_state()
            
            if state == ReplyProcessState.WAITING_FOR_TOOL_APPROVAL:
                tool_requests = reply_state.get_pending_tool_requests()
                self.assertGreater(len(tool_requests), 0)
                
                # Approve all tool requests
                for request in tool_requests:
                    reply_state.approve_tool(request["id"])
                
                tool_requests_found = True
            
            elif state == ReplyProcessState.MESSAGE_YIELDED:
                reply_state.advance()
            
            else:
                reply_state.advance()
        
        # Verify we found tool requests
        self.assertTrue(tool_requests_found)
    
    def test_error_handling(self):
        """Test error handling in the FFI layer."""
        # Test with invalid message format
        with self.assertRaises(Exception):
            invalid_message = {"invalid": "format"}
            self.agent.create_reply_state([invalid_message])
    
    def test_memory_management(self):
        """Test that memory is properly managed."""
        # Create and destroy multiple reply states
        for _ in range(10):
            message = {
                "role": MessageRole.USER,
                "content": [{"type": "text", "text": "Test message"}]
            }
            
            reply_state = self.agent.create_reply_state([message])
            reply_state.start()
            
            # Advance a few times
            for _ in range(3):
                if reply_state.get_state() != ReplyProcessState.COMPLETED:
                    reply_state.advance()
            
            # Let the reply state be garbage collected
            del reply_state
    
    def test_frontend_tool_simulation(self):
        """Test frontend tool handling (simulated)."""
        # Create a message that would trigger a calculator tool
        message = {
            "role": MessageRole.USER,
            "content": [{"type": "text", "text": "Calculate 15 * 3"}]
        }
        
        reply_state = self.agent.create_reply_state([message])
        reply_state.start()
        
        while reply_state.get_state() != ReplyProcessState.COMPLETED:
            state = reply_state.get_state()
            
            if state == ReplyProcessState.WAITING_FOR_TOOL_APPROVAL:
                tool_requests = reply_state.get_pending_tool_requests()
                
                for request in tool_requests:
                    # Simulate frontend tool execution
                    if request["name"] == "calculator":
                        args = request["arguments"]
                        # Simulate calculator execution
                        result = 15 * 3
                        print(f"Calculator result: {result}")
                    
                    reply_state.approve_tool(request["id"])
            
            elif state == ReplyProcessState.MESSAGE_YIELDED:
                message = reply_state.get_current_message()
                if message:
                    print(f"Response: {message}")
                reply_state.advance()
            
            else:
                reply_state.advance()

if __name__ == "__main__":
    unittest.main()