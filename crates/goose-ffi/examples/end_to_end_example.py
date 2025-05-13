#!/usr/bin/env python3
"""
End-to-end example demonstrating the FFI-friendly agent with frontend tools.

This example shows:
1. Setting up an agent with frontend tools
2. Processing a conversation with tool requests
3. Handling tool approvals and execution
4. Complete conversation flow
"""

import ctypes
import os
import platform
import json
import time
from ctypes import c_char_p, c_bool, c_uint32, c_void_p, Structure, POINTER, c_size_t
from enum import IntEnum

# Import the GooseAgent and ReplyState classes from the main example
# In a real implementation, these would be in a separate module
from goose_agent import GooseAgent, ReplyState, ReplyProcessState, MessageRole

# Calculator tool implementation
class Calculator:
    """A simple calculator tool for demonstration."""
    
    @staticmethod
    def execute(args):
        """Execute the calculator with the given arguments."""
        operation = args["operation"]
        numbers = args["numbers"]
        
        try:
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
                    if n == 0:
                        raise ValueError("Division by zero")
                    result /= n
            else:
                raise ValueError(f"Unknown operation: {operation}")
            
            return {
                "type": "text",
                "text": str(result),
                "annotations": None
            }
        except Exception as e:
            return {
                "type": "text",
                "text": f"Error: {str(e)}",
                "annotations": None
            }

# Weather tool implementation
class WeatherTool:
    """A mock weather tool for demonstration."""
    
    @staticmethod
    def execute(args):
        """Execute the weather tool with the given arguments."""
        location = args.get("location", "Unknown")
        
        # Mock weather data
        weather_data = {
            "New York": {"temp": 72, "condition": "Sunny"},
            "London": {"temp": 65, "condition": "Cloudy"},
            "Tokyo": {"temp": 78, "condition": "Clear"},
            "Paris": {"temp": 68, "condition": "Rainy"},
        }
        
        weather = weather_data.get(location, {"temp": 70, "condition": "Unknown"})
        
        return {
            "type": "text",
            "text": f"Weather in {location}: {weather['temp']}°F, {weather['condition']}",
            "annotations": None
        }

class ConversationManager:
    """Manages a conversation with the Goose agent."""
    
    def __init__(self, agent):
        self.agent = agent
        self.tools = {
            "calculator": Calculator(),
            "weather": WeatherTool(),
        }
        self.conversation_history = []
    
    def process_conversation(self, user_input):
        """Process a single conversation turn."""
        # Create user message
        message = {
            "role": MessageRole.USER,
            "content": [{"type": "text", "text": user_input}]
        }
        
        # Add to conversation history
        self.conversation_history.append(message)
        
        # Create reply state
        reply_state = self.agent.create_reply_state(self.conversation_history)
        
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
                        self.conversation_history.append(message)
                        self._display_message(message)
                    reply_state.advance()
                
                elif state == ReplyProcessState.WAITING_FOR_TOOL_APPROVAL:
                    # Handle tool requests
                    self._handle_tool_requests(reply_state)
                
                elif state == ReplyProcessState.ERROR:
                    print("Error occurred in conversation")
                    break
                
                else:
                    # Continue processing
                    reply_state.advance()
            
        except Exception as e:
            print(f"Error: {e}")
    
    def _display_message(self, message):
        """Display a message to the user."""
        for content in message.get("content", []):
            if content["type"] == "text":
                print(f"Agent: {content['text']}")
            elif content["type"] == "toolRequest":
                tool_call = content["toolCall"]["value"]
                print(f"[Tool Request] {tool_call['name']}: {tool_call['arguments']}")
            elif content["type"] == "toolResponse":
                print(f"[Tool Response] {content['toolResult']}")
    
    def _handle_tool_requests(self, reply_state):
        """Handle tool requests from the agent."""
        tool_requests = reply_state.get_pending_tool_requests()
        
        for request in tool_requests:
            tool_name = request["name"]
            tool_args = request["arguments"]
            
            print(f"\n[Tool Approval] {tool_name} with args: {tool_args}")
            
            # Check if we have this tool
            if tool_name in self.tools:
                # Execute the tool
                tool = self.tools[tool_name]
                result = tool.execute(tool_args)
                
                print(f"[Tool Execution] Result: {result['text']}")
                
                # Approve the tool
                reply_state.approve_tool(request["id"])
                
                # Add tool response to conversation history
                tool_response = {
                    "role": MessageRole.USER,
                    "content": [{
                        "type": "toolResponse",
                        "id": request["id"],
                        "toolResult": {"Ok": [result]}
                    }]
                }
                self.conversation_history.append(tool_response)
            else:
                # Unknown tool - ask for approval
                approve = input(f"Unknown tool '{tool_name}'. Approve? (y/n): ")
                if approve.lower() == 'y':
                    reply_state.approve_tool(request["id"])
                else:
                    reply_state.deny_tool(request["id"])

def main():
    """Main function to run the end-to-end example."""
    print("Goose Agent - End-to-End Example with Frontend Tools")
    print("=" * 50)
    
    # Initialize the agent
    api_key = os.getenv("DATABRICKS_API_KEY")
    host = os.getenv("DATABRICKS_HOST")
    
    try:
        agent = GooseAgent(
            api_key=api_key,
            model_name="claude-3-7-sonnet",
            host=host
        )
        
        # Create conversation manager
        manager = ConversationManager(agent)
        
        # Example conversations
        print("\nExample 1: Calculator Tool")
        print("-" * 30)
        manager.process_conversation("What is 42 + 58?")
        
        print("\nExample 2: Weather Tool")
        print("-" * 30)
        manager.process_conversation("What's the weather like in New York?")
        
        print("\nExample 3: Complex Calculation")
        print("-" * 30)
        manager.process_conversation("Calculate (15 * 3) + (100 / 4)")
        
        # Interactive mode
        print("\nInteractive Mode")
        print("-" * 30)
        print("Type your message (or 'quit' to exit):")
        
        while True:
            user_input = input("\nYou: ")
            if user_input.lower() in ("quit", "exit"):
                break
            
            manager.process_conversation(user_input)
    
    except Exception as e:
        print(f"Error initializing agent: {e}")

if __name__ == "__main__":
    main()