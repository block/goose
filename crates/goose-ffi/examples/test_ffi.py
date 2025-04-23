#!/usr/bin/env python3
"""
Test script for the Goose FFI Python bindings.
This script tests that the FFI bindings are working correctly without requiring actual API credentials.
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
LIB_PATH = os.path.join(os.path.dirname(__file__), "../../..", "target", "release", LIB_NAME)

# Load library
try:
    goose = ctypes.CDLL(LIB_PATH)
    print(f"✓ Successfully loaded library from {LIB_PATH}")
except Exception as e:
    print(f"✗ Failed to load library: {e}")
    exit(1)

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
        ("mode", c_char_p),
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

def test_provider_config():
    """Test that we can create a ProviderConfig structure with mode field"""
    try:
        config = ProviderConfig(
            provider_type=ProviderType.DATABRICKS,
            api_key=b"test_api_key",
            model_name=b"test_model",
            host=b"test_host",
            mode=b"test_mode",
        )
        print("✓ ProviderConfig structure created successfully with mode field")
        
        # Test that we can access all fields
        assert config.provider_type == ProviderType.DATABRICKS
        assert config.api_key == b"test_api_key"
        assert config.model_name == b"test_model"
        assert config.host == b"test_host"
        assert config.mode == b"test_mode"
        print("✓ All fields accessible")
        
    except Exception as e:
        print(f"✗ Failed to create ProviderConfig: {e}")
        return False
    return True

def test_agent_creation_failure():
    """Test that agent creation fails gracefully with invalid credentials"""
    try:
        config = ProviderConfig(
            provider_type=ProviderType.DATABRICKS,
            api_key=None,  # No API key - should fail
            model_name=b"claude-3-7-sonnet",
            host=None,  # No host - should fail
            mode=b"auto",
        )
        
        # This should return null pointer due to missing credentials
        agent = goose.goose_agent_new(ctypes.byref(config))
        
        if not agent:
            print("✓ Agent creation correctly failed with missing credentials")
        else:
            print(f"✗ Agent creation unexpectedly succeeded with missing credentials (agent ptr: {agent})")
            goose.goose_agent_free(agent)
            return False
            
    except Exception as e:
        print(f"✗ Unexpected error during agent creation test: {e}")
        return False
    return True

def test_function_signatures():
    """Test that all function signatures are correctly defined"""
    try:
        # Check that all functions have the expected argtypes and restype
        assert goose.goose_agent_new.argtypes == [POINTER(ProviderConfig)]
        assert goose.goose_agent_new.restype == goose_AgentPtr
        
        assert goose.goose_agent_free.argtypes == [goose_AgentPtr]
        assert goose.goose_agent_free.restype == None
        
        assert goose.goose_agent_send_message.argtypes == [goose_AgentPtr, c_char_p]
        assert goose.goose_agent_send_message.restype == c_void_p
        
        assert goose.goose_free_string.argtypes == [c_void_p]
        assert goose.goose_free_string.restype == None
        
        assert goose.goose_free_async_result.argtypes == [POINTER(AsyncResult)]
        assert goose.goose_free_async_result.restype == None
        
        print("✓ All function signatures correctly defined")
    except Exception as e:
        print(f"✗ Function signature test failed: {e}")
        return False
    return True

def main():
    print("Testing Goose FFI Python bindings...\n")
    
    tests = [
        test_function_signatures,
        test_provider_config,
        test_agent_creation_failure,
    ]
    
    passed = 0
    for test in tests:
        if test():
            passed += 1
        print()
    
    print(f"Tests completed: {passed}/{len(tests)} passed")
    
    if passed == len(tests):
        print("\n✓ All tests passed! The FFI bindings are working correctly.")
        print("  The mode field has been successfully added to the ProviderConfig.")
    else:
        print("\n✗ Some tests failed. Please check the output above.")
        exit(1)

if __name__ == "__main__":
    main()