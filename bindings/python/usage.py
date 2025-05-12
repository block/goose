import asyncio
import os
import time
from goose_llm import (
    Message, MessageContent, TextContent, ToolRequest, ToolResponse,
    Role, ModelConfig, ToolApprovalMode,
    create_tool_config, ExtensionConfig,
    generate_session_name, generate_tooltip,
    create_completion_request, completion
)

async def main():
    now = int(time.time())

    # 1) User sends a plain-text prompt
    messages = [
        Message(
            role=Role.USER,
            created=now,
            content=[MessageContent.TEXT(TextContent(text="What is 7 x 6?"))]
        ),

        # 2) Assistant makes a tool request
        Message(
            role=Role.ASSISTANT,
            created=now + 2,
            content=[MessageContent.TOOL_REQ(ToolRequest(
                id="calc1",
                tool_call="""
                    {
                      "status": "success",
                      "value": {
                        "name": "calculator_extension__toolname",
                        "arguments": {
                          "operation": "multiply",
                          "numbers": [7, 6]
                        },
                        "needsApproval": false
                      }
                    }
                """
            ))]
        ),

        # 3) User sends tool result
        Message(
            role=Role.USER,
            created=now + 3,
            content=[MessageContent.TOOL_RESP(ToolResponse(
                id="calc1",
                tool_result="""
                    {
                      "status": "success",
                      "value": [
                        {"type": "text", "text": "42"}
                      ]
                    }
                """
            ))]
        )
    ]

    provider_name = "databricks"
    provider_config = f'''{{
        "host": "{os.environ.get("DATABRICKS_HOST")}",
        "token": "{os.environ.get("DATABRICKS_TOKEN")}"
    }}'''

    print(f"Provider Name: {provider_name}")
    print(f"Provider Config: {provider_config}")

    session_name = await generate_session_name(provider_name, provider_config, messages)
    print(f"\nSession Name: {session_name}")

    tooltip = await generate_tooltip(provider_name, provider_config, messages)
    print(f"\nTooltip: {tooltip}")

    model_config = ModelConfig(
        model_name="goose-gpt-4-1",
        max_tokens=500,
        temperature=0.1,
        context_limit=4096,
    )

    calculator_tool = create_tool_config(
        name="calculator",
        description="Perform basic arithmetic operations",
        input_schema="""
            {
                "type": "object",
                "required": ["operation", "numbers"],
                "properties": {
                    "operation": {
                        "type": "string",
                        "enum": ["add", "subtract", "multiply", "divide"],
                        "description": "The arithmetic operation to perform"
                    },
                    "numbers": {
                        "type": "array",
                        "items": { "type": "number" },
                        "description": "List of numbers to operate on in order"
                    }
                }
            }
        """,
        approval_mode=ToolApprovalMode.AUTO
    )

    calculator_extension = ExtensionConfig(
        name="calculator_extension",
        instructions="This extension provides a calculator tool.",
        tools=[calculator_tool]
    )

    system_preamble = "You are a helpful assistant."
    extensions = [calculator_extension]

    req = create_completion_request(
        provider_name,
        provider_config,
        model_config,
        system_preamble,
        messages,
        extensions
    )

    resp = await completion(req)
    print(f"\nCompletion Response:\n{resp.message}")
    print(f"Msg content: {resp.message.content[0][0]}")


if __name__ == "__main__":
    asyncio.run(main())