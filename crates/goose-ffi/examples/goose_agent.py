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
import json
import os
import platform
from ctypes import c_char_p, c_void_p, c_bool, c_uint32, Structure, POINTER, CFUNCTYPE
from enum import IntEnum

# Provider type enum values
class ProviderType(IntEnum):
    DATABRICKS = 0  # Databricks AI provider
from typing import Dict, Any, Optional, Callable

# Determine the platform-specific library name
if platform.system() == "Darwin":
    LIB_NAME = "libgoose_ffi.dylib"
elif platform.system() == "Linux":
    LIB_NAME = "libgoose_ffi.so"
elif platform.system() == "Windows":
    LIB_NAME = "goose_ffi.dll"
else:
    raise RuntimeError(f"Unsupported platform: {platform.system()}")

# Find the library path relative to this script
SCRIPT_DIR = os.path.dirname(os.path.abspath(__file__))
LIB_PATH = os.path.join(SCRIPT_DIR, "../../..", "target", "debug", LIB_NAME)

# Load the FFI library
try:
    goose_lib = ctypes.CDLL(LIB_PATH)
except OSError as e:
    print(f"Error loading library: {e}")
    print(f"Make sure you've built the library with 'cargo build' and the path is correct: {LIB_PATH}")
    exit(1)

# Define C-compatible structures and callbacks
class ProviderConfig(Structure):
    _fields_ = [
        ("provider_type", c_uint32),  # 0 = Databricks (currently the only supported provider)
        ("api_key", c_char_p),
        ("model_name", c_char_p),
        ("host", c_char_p),
    ]

# Extension and tool callback support to be added in future commits

# Set up the function signatures
goose_lib.goose_agent_new.argtypes = [POINTER(ProviderConfig)]
goose_lib.goose_agent_new.restype = c_void_p

goose_lib.goose_agent_free.argtypes = [c_void_p]
goose_lib.goose_agent_free.restype = None

goose_lib.goose_agent_send_message.argtypes = [c_void_p, c_char_p]
goose_lib.goose_agent_send_message.restype = c_char_p

goose_lib.goose_free_string.argtypes = [c_char_p]
goose_lib.goose_free_string.restype = None

class GooseAgent:
    """Python wrapper for Goose Agent."""
    
    def __init__(self, provider_type: int = 0, api_key: Optional[str] = None, 
                 model_name: Optional[str] = None, host: Optional[str] = None):
        """
        Create a new Goose Agent.
        
        Args:
            provider_type: Provider type (0 = Databricks, currently the only supported provider)
            api_key: Provider API key (or None to use environment variables)
            model_name: Model name (or None to use provider default)
            host: Provider host URL (or None to use environment variable)
        """
        # Convert strings to bytes for C compatibility
        api_key_bytes = api_key.encode('utf-8') if api_key else None
        model_name_bytes = model_name.encode('utf-8') if model_name else None
        host_bytes = host.encode('utf-8') if host else None
        
        # Create provider config
        config = ProviderConfig(
            provider_type=provider_type,
            api_key=api_key_bytes,
            model_name=model_name_bytes,
            host=host_bytes
        )
        
        # Create agent
        self.agent_ptr = goose_lib.goose_agent_new(ctypes.byref(config))
        if not self.agent_ptr:
            raise RuntimeError("Failed to create agent")
    
    def __del__(self):
        """Cleanup agent when object is destroyed."""
        if hasattr(self, 'agent_ptr') and self.agent_ptr:
            goose_lib.goose_agent_free(self.agent_ptr)
    
    # Tool support will be added in future commits
    
    def send_message(self, message: str) -> str:
        """
        Send a message to the agent.
        
        Args:
            message: The message to send
        
        Returns:
            The agent's response
        """
        # Send the message
        response_ptr = goose_lib.goose_agent_send_message(
            self.agent_ptr,
            message.encode('utf-8')
        )
        
        if not response_ptr:
            return "Error getting response from agent"
        
        # Read the response string and free it
        response = ctypes.string_at(response_ptr).decode('utf-8')
        goose_lib.goose_free_string(response_ptr)
        
        return response


def main():
    # Get API key and host URL from environment or let the library use them
    api_key = os.environ.get("DATABRICKS_API_KEY")
    host_url = os.environ.get("DATABRICKS_HOST")
    
    # Create agent with Databricks provider
    print("Creating Databricks agent...")
    agent = GooseAgent(
        provider_type=ProviderType.DATABRICKS,  # Use Databricks provider
        api_key=api_key,
        model_name="claude-3-7-sonnet",
        host=host_url
    )
    
    # Interactive loop
    print("Type your message (or 'quit' to exit):")
    while True:
        user_input = input("> ")
        if user_input.lower() in ("quit", "exit"):
            break
        
        response = agent.send_message(user_input)
        print(f"Agent: {response}\n")


if __name__ == "__main__":
    main()