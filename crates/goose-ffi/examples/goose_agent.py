#!/usr/bin/env python3
"""
Python example for using the Goose FFI interface.

This example demonstrates how to:
1. Load the Goose FFI library
2. Create an agent with a provider
3. Add a tool extension
4. Send messages to the agent
5. Handle tool calls and responses
"""

import ctypes
import os
import platform
from ctypes import c_char_p, c_bool, c_uint32, c_void_p, Structure, POINTER

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

# Forward declaration for goose_Agent
class goose_Agent(Structure):
    pass

# Agent pointer type
goose_AgentPtr = POINTER(goose_Agent)

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

# Function signatures
goose.goose_agent_new.argtypes = [POINTER(ProviderConfig)]
goose.goose_agent_new.restype = goose_AgentPtr

goose.goose_agent_free.argtypes = [goose_AgentPtr]
goose.goose_agent_free.restype = None

goose.goose_agent_send_message.argtypes = [goose_AgentPtr, c_char_p]
goose.goose_agent_send_message.restype = c_void_p

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

    def __del__(self):
        if getattr(self, "agent", None):
            goose.goose_agent_free(self.agent)

    def send_message(self, message: str) -> str:
        msg = message.encode("utf-8")
        response_ptr = goose.goose_agent_send_message(self.agent, msg)
        if not response_ptr:
            return "Error or NULL response from agent"
        response = ctypes.string_at(response_ptr).decode("utf-8")
        # Free the string using the proper C function provided by the library
        # This correctly releases memory allocated by the Rust side
        goose.goose_free_string(response_ptr)
        return response

def main():
    api_key = os.getenv("DATABRICKS_API_KEY")
    host = os.getenv("DATABRICKS_HOST")
    agent = GooseAgent(api_key=api_key, model_name="claude-3-7-sonnet", host=host)

    print("Type a message (or 'quit' to exit):")
    while True:
        user_input = input("> ")
        if user_input.lower() in ("quit", "exit"):
            break
        reply = agent.send_message(user_input)
        print(f"Agent: {reply}\n")

if __name__ == "__main__":
    main()
