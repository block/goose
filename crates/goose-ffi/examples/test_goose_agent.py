#!/usr/bin/env python3
"""
Test script to verify the GooseAgent class works with the new mode parameter.
This test checks both creation and usage of the agent.
"""

import sys
import os

# Add the examples directory to the path
sys.path.insert(0, os.path.dirname(__file__))

from goose_agent import GooseAgent, ProviderType

def test_goose_agent_with_invalid_credentials():
    """Test that GooseAgent fails when trying to use invalid credentials"""
    try:
        # This should succeed in creation
        agent = GooseAgent(
            provider_type=ProviderType.DATABRICKS,
            api_key="invalid_key",
            model_name="claude-3-7-sonnet",
            host="https://invalid.host",
            mode="chat"  # Testing the new mode parameter
        )
        print("✓ GooseAgent constructor succeeded with mode parameter")
        
        # This should fail when trying to use the agent
        try:
            response = agent.send_message("Hello")
            # Check if the response contains an error message
            if "error" in response.lower() and "invalid.host" in response:
                print(f"✓ Agent correctly returned error with invalid credentials: {response[:100]}...")
                return True
            else:
                print(f"✗ Agent unexpectedly succeeded with invalid credentials: {response}")
                return False
        except Exception as e:
            print(f"✓ Agent correctly failed when used with invalid credentials: {e}")
            return True
            
    except Exception as e:
        print(f"✗ Unexpected error during agent creation: {e}")
        return False

def test_goose_agent_with_valid_credentials():
    """Test that GooseAgent works with valid credentials and mode parameter"""
    try:
        api_key = os.getenv("DATABRICKS_API_KEY")
        host = os.getenv("DATABRICKS_HOST")
        
        if not api_key or not host:
            print("⚠️  Skipping valid credentials test - DATABRICKS_API_KEY or DATABRICKS_HOST not set")
            return True
        
        # Create agent with valid credentials and specific mode
        agent = GooseAgent(
            provider_type=ProviderType.DATABRICKS,
            api_key=api_key,
            model_name="claude-3-7-sonnet",
            host=host,
            mode="auto"  # Testing the mode parameter
        )
        print("✓ GooseAgent created with valid credentials and mode parameter")
        
        # Try to use the agent
        response = agent.send_message("Say 'test successful' if you can hear me")
        print(f"✓ Agent responded: {response[:100]}...")  # Print first 100 chars
        return True
        
    except Exception as e:
        print(f"✗ Error with valid credentials: {e}")
        return False

def test_mode_variations():
    """Test different mode values"""
    modes_to_test = ["auto", "chat", None, ""]
    
    for mode in modes_to_test:
        try:
            agent = GooseAgent(
                provider_type=ProviderType.DATABRICKS,
                api_key="test_key",
                model_name="test_model",
                host="test_host",
                mode=mode
            )
            print(f"✓ GooseAgent created with mode='{mode}'")
        except Exception as e:
            print(f"✗ Failed to create agent with mode='{mode}': {e}")
            return False
    
    return True

def main():
    print("Testing GooseAgent class with mode parameter...\n")
    
    tests = [
        test_mode_variations,
        test_goose_agent_with_invalid_credentials,
        test_goose_agent_with_valid_credentials,
    ]
    
    passed = 0
    for test in tests:
        if test():
            passed += 1
        print()
    
    print(f"Tests completed: {passed}/{len(tests)} passed")
    
    if passed == len(tests):
        print("\n✓ All tests passed! The GooseAgent class works correctly with the mode parameter.")
    else:
        print("\n✗ Some tests failed. Please check the output above.")
        exit(1)

if __name__ == "__main__":
    main()