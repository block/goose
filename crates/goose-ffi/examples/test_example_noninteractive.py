#!/usr/bin/env python3
"""
Non-interactive test of the goose_agent.py example
"""

import os
import sys

# Add the examples directory to the path
sys.path.insert(0, os.path.dirname(__file__))

from goose_agent import GooseAgent

def test_example():
    api_key = os.getenv("DATABRICKS_API_KEY")
    host = os.getenv("DATABRICKS_HOST")
    
    # Create agent with specific mode - demonstrates using agent without global config
    agent = GooseAgent(api_key=api_key, model_name="claude-3-7-sonnet", host=host, mode="auto")
    
    # Send a test message
    reply = agent.send_message("Say 'Hello from Goose FFI!' if you can hear me")
    print(f"Agent: {reply}")
    
    # Verify the response contains expected content
    # The response is JSON, so check if it contains our expected text
    if "Hello from Goose" in reply:
        print("\n✓ Example works correctly!")
        return True
    else:
        print("\n✗ Example did not produce expected response")
        return False

if __name__ == "__main__":
    if test_example():
        exit(0)
    else:
        exit(1)