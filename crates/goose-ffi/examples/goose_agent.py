#!/usr/bin/env python3
"""
Python example for using the Goose FFI interface with the new FFI-friendly agent design.

This example demonstrates how to:
1. Load the Goose FFI library
2. Create an agent with a provider
3. Add a frontend tool extension
4. Use the new ReplyState system to handle conversations
5. Handle tool approvals and responses
"""

import ctypes
import os
import platform
import json
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

# Function signatures
goose.goose_agent_new.argtypes = [POINTER(ProviderConfig)]
goose.goose_agent_new.restype = goose_AgentPtr

goose.goose_agent_free.argtypes = [goose_AgentPtr]
goose.goose_agent_free.restype = None

# FFI Agent functions
goose.goose_ffi_agent_new.argtypes = [goose_AgentPtr]
goose.goose_ffi_agent_new.restype = goose_FFIAgentPtr

goose.goose_ffi_agent_create_reply_state.argtypes = [
    goose_FFIAgentPtr,
    POINTER(MessageFFI),
    c_size_t,
    c_void_p  # session_config (null for now)
]
goose.goose_ffi_agent_create_reply_state.restype = goose_ReplyStatePtr

# ReplyState functions
goose.goose_reply_state_start.argtypes = [goose_ReplyStatePtr]
goose.goose_reply_state_start.restype = POINTER(AsyncResult)

goose.goose_reply_state_advance.argtypes = [goose_ReplyStatePtr]
goose.goose_reply_state_advance.restype = POINTER(AsyncResult)

goose.goose_reply_state_get_state.argtypes = [goose_ReplyStatePtr]
goose.goose_reply_state_get_state.restype = c_uint32

goose.goose_reply_state_get_current_message.argtypes = [goose_ReplyStatePtr]
goose.goose_reply_state_get_current_message.restype = c_char_p

goose.goose_reply_state_get_pending_tool_requests.argtypes = [
    goose_ReplyStatePtr,
    POINTER(c_size_t)
]
goose.goose_reply_state_get_pending_tool_requests.restype = POINTER(PendingToolRequestFFI)

goose.goose_reply_state_approve_tool.argtypes = [goose_ReplyStatePtr, c_char_p]
goose.goose_reply_state_approve_tool.restype = POINTER(AsyncResult)

goose.goose_reply_state_deny_tool.argtypes = [goose_ReplyStatePtr, c_char_p]
goose.goose_reply_state_deny_tool.restype = POINTER(AsyncResult)

goose.goose_reply_state_free.argtypes = [goose_ReplyStatePtr]
goose.goose_reply_state_free.restype = None

# Cleanup functions
goose.goose_free_string.argtypes = [c_void_p]
goose.goose_free_string.restype = None

goose.goose_free_async_result.argtypes = [POINTER(AsyncResult)]
goose.goose_free_async_result.restype = None

class GooseAgent:
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

# Calculator tool implementation
def execute_calculator(args):
    """Execute the calculator tool with the given arguments."""
    operation = args["operation"]
    numbers = args["numbers"]
    
    try:
        result = None
        if operation == "add":
            result = sum(numbers)
        elif operation == "subtract":
            result = numbers[0] - sum(numbers[1:])
        elif operation == "multiply":
            result = 1
            for n in numbers:
                result *= n
        elif operation == "divide":
            result = numbers[0]
            for n in numbers[1:]:
                result /= n
        
        return {"type": "text", "text": str(result)}
    except Exception as e:
        return {"type": "text", "text": f"Error: {str(e)}"}

def main():
    api_key = os.getenv("DATABRICKS_API_KEY")
    host = os.getenv("DATABRICKS_HOST")
    agent = GooseAgent(api_key=api_key, model_name="claude-3-7-sonnet", host=host)
    
    # Add a frontend tool (calculator)
    calculator_tool = {
        "name": "calculator",
        "description": "Perform basic arithmetic calculations",
        "inputSchema": {
            "type": "object",
            "required": ["operation", "numbers"],
            "properties": {
                "operation": {
                    "type": "string",
                    "enum": ["add", "subtract", "multiply", "divide"],
                    "description": "The arithmetic operation to perform",
                },
                "numbers": {
                    "type": "array",
                    "items": {"type": "number"},
                    "description": "List of numbers to operate on in order",
                },
            },
        },
    }
    
    # Note: In a real implementation, you'd need to add the frontend extension
    # through the proper FFI methods. For now, we'll simulate it.
    
    print("Goose Agent with Calculator Tool")
    print("Type a message (or 'quit' to exit):")
    print("Try asking: 'What is 42 + 58?'")
    
    while True:
        user_input = input("> ")
        if user_input.lower() in ("quit", "exit"):
            break
        
        # Create message
        message = {
            "role": MessageRole.USER,
            "content": [{"type": "text", "text": user_input}]
        }
        
        # Create reply state
        reply_state = agent.create_reply_state([message])
        
        try:
            # Start the conversation
            reply_state.start()
            
            # Process the conversation
            while reply_state.get_state() != ReplyProcessState.COMPLETED:
                state = reply_state.get_state()
                
                if state == ReplyProcessState.MESSAGE_YIELDED:
                    # Get and display the message
                    message = reply_state.get_current_message()
                    if message:
                        for content in message.get("content", []):
                            if content["type"] == "text":
                                print(f"Agent: {content['text']}")
                    reply_state.advance()
                
                elif state == ReplyProcessState.WAITING_FOR_TOOL_APPROVAL:
                    # Handle tool requests
                    tool_requests = reply_state.get_pending_tool_requests()
                    for request in tool_requests:
                        print(f"Tool request: {request['name']} with args {request['arguments']}")
                        
                        # For calculator tool, automatically approve and execute
                        if request["name"] == "calculator":
                            result = execute_calculator(request["arguments"])
                            print(f"Calculator result: {result['text']}")
                            reply_state.approve_tool(request["id"])
                        else:
                            # For other tools, ask for approval
                            approve = input(f"Approve tool {request['name']}? (y/n): ")
                            if approve.lower() == 'y':
                                reply_state.approve_tool(request["id"])
                            else:
                                reply_state.deny_tool(request["id"])
                
                elif state == ReplyProcessState.ERROR:
                    print("Error occurred in conversation")
                    break
                
                else:
                    # Continue processing
                    reply_state.advance()
            
            print()  # Add newline for readability
            
        except Exception as e:
            print(f"Error: {e}")

if __name__ == "__main__":
    main()