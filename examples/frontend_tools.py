import asyncio
import json
import os
from typing import List, Dict, Any
import httpx
from datetime import datetime

# Configuration
GOOSE_HOST = "127.0.0.1"
GOOSE_PORT = "3001"
GOOSE_URL = f"http://{GOOSE_HOST}:{GOOSE_PORT}"
SECRET_KEY = "test"  # Default development secret key

# A simple calculator tool definition
CALCULATOR_TOOL = {
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

# Frontend extension configuration
FRONTEND_CONFIG = {
    "name": "pythonclient",
    "type": "frontend",
    "tools": [CALCULATOR_TOOL],
    "instructions": "A calculator extension that can perform basic arithmetic operations.",
}


async def setup_agent() -> None:
    """Initialize the agent with our frontend tool."""
    async with httpx.AsyncClient() as client:
        # First create the agent
        response = await client.post(
            f"{GOOSE_URL}/agent",
            json={"provider": "databricks", "model": "goose"},
            headers={"X-Secret-Key": SECRET_KEY},
        )
        response.raise_for_status()
        print("Successfully created agent")

        # Then add our frontend extension
        response = await client.post(
            f"{GOOSE_URL}/extensions/add",
            json=FRONTEND_CONFIG,
            headers={"X-Secret-Key": SECRET_KEY},
        )
        response.raise_for_status()
        print("Successfully added calculator extension")


def execute_calculator(args: Dict[str, Any]) -> List[Dict[str, Any]]:
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

        # Return properly structured Content::Text variant
        return [
            {
                "type": "text",
                "text": str(result),
                "annotations": None,  # Required field in Rust struct
            }
        ]
    except Exception as e:
        return [
            {
                "type": "text",
                "text": f"Error: {str(e)}",
                "annotations": None,  # Required field in Rust struct
            }
        ]


def submit_tool_result(tool_id: str, result: List[Dict[str, Any]]) -> None:
    """Submit the tool execution result back to Goose.

    The result should be a list of Content variants (Text, Image, or Resource).
    Each Content variant has a type tag and appropriate fields.
    """
    payload = {
        "id": tool_id,
        "result": {
            "Ok": result  # Result enum variant with single key for success case
        },
    }

    with httpx.Client(timeout=2.0) as client:
        response = client.post(
            f"{GOOSE_URL}/tool_result",
            json=payload,
            headers={"X-Secret-Key": SECRET_KEY},
        )
        response.raise_for_status()


async def chat_loop() -> None:
    """Main chat loop that handles the conversation with Goose."""
    session_id = "test-session"

    # Use a client with a longer timeout for streaming
    async with httpx.AsyncClient(timeout=30.0) as client:
        # Get user input
        user_message = input("\nYou: ")
        if user_message.lower() in ["exit", "quit"]:
            return

        # Create the message object
        message = {
            "role": "user",
            "created": int(datetime.now().timestamp()),
            "content": [{"type": "text", "text": user_message}],
        }

        # Send to /reply endpoint
        payload = {
            "messages": [message],
            "session_id": session_id,
            "session_working_dir": os.getcwd(),
        }

        # Process the stream of responses
        async with client.stream(
            "POST",
            f"{GOOSE_URL}/reply",
            json=payload,
            headers={
                "X-Secret-Key": SECRET_KEY,
                "Accept": "text/event-stream",
                "Content-Type": "application/json",
            },
        ) as stream:
            async for line in stream.aiter_lines():
                if not line:
                    continue

                # Handle SSE format
                if line.startswith("data: "):
                    line = line[6:]  # Remove "data: " prefix

                try:
                    payload = json.loads(line)
                except json.JSONDecodeError:
                    print(f"Failed to parse line: {line}")
                    continue

                if payload["type"] == "Finish":
                    break

                message = payload["message"]
                # Handle different message types
                for content in message.get("content", []):
                    if content["type"] == "text":
                        print(f"\nGoose: {content['text']}")
                    elif content["type"] == "frontendToolRequest":
                        # Execute the tool and submit results
                        tool_call = content["toolCall"]["value"]
                        print(f"Calculator: {tool_call}")
                        # Execute the tool
                        result = execute_calculator(tool_call["arguments"])

                        # Submit the result
                        submit_tool_result(content["id"], result)


async def main():
    try:
        # Initialize the agent with our tool
        await setup_agent()

        # Start the chat loop
        await chat_loop()

    except Exception as e:
        print(f"Error: {e}")
        raise  # Re-raise to see full traceback


if __name__ == "__main__":
    asyncio.run(main())
